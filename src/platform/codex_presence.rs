use super::OwnedHandle;
use std::{
    collections::HashMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use windows::{
    Win32::{
        Foundation::FILETIME,
        System::{Diagnostics::ToolHelp::*, Threading::*},
    },
    core::PWSTR,
};

fn image(pid: u32) -> Option<String> {
    let handle =
        OwnedHandle(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?);
    let mut buffer = vec![0u16; 32768];
    let mut length = buffer.len() as u32;
    unsafe {
        QueryFullProcessImageNameW(
            handle.0,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        )
    }
    .ok()?;
    Some(String::from_utf16_lossy(&buffer[..length as usize]).to_lowercase())
}

pub fn desktop_started() -> Option<SystemTime> {
    let snapshot = OwnedHandle(unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }.ok()?);
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut processes = HashMap::new();
    unsafe { Process32FirstW(snapshot.0, &mut entry) }.ok()?;
    loop {
        let end = entry
            .szExeFile
            .iter()
            .position(|c| *c == 0)
            .unwrap_or(entry.szExeFile.len());
        processes.insert(
            entry.th32ProcessID,
            (
                entry.th32ParentProcessID,
                String::from_utf16_lossy(&entry.szExeFile[..end]).to_lowercase(),
            ),
        );
        if unsafe { Process32NextW(snapshot.0, &mut entry) }.is_err() {
            break;
        }
    }
    let mut earliest = None;
    for (&pid, &(parent, ref name)) in &processes {
        if name != "codex.exe" {
            continue;
        }
        let Some((_, parent_name)) = processes.get(&parent) else {
            continue;
        };
        if !matches!(parent_name.as_str(), "chatgpt.exe" | "codex.exe") {
            continue;
        }
        let Some(parent_image) = image(parent) else {
            continue;
        };
        if !(parent_image.contains("\\openai.codex_")
            || parent_image.contains("\\openai\\codex\\") && !parent_image.contains("\\bin\\"))
        {
            continue;
        }
        let Ok(handle) = (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) })
        else {
            continue;
        };
        let handle = OwnedHandle(handle);
        let (mut created, mut exited, mut kernel, mut user) = (
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
        );
        if unsafe { GetProcessTimes(handle.0, &mut created, &mut exited, &mut kernel, &mut user) }
            .is_err()
        {
            continue;
        }
        let ticks = (u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime);
        if let Some(unix_ticks) = ticks.checked_sub(116_444_736_000_000_000) {
            let at = UNIX_EPOCH + Duration::from_nanos(unix_ticks.saturating_mul(100));
            earliest = Some(earliest.map_or(at, |old: SystemTime| old.min(at)));
        }
    }
    earliest
}
