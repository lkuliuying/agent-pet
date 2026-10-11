use super::{OwnedHandle, wide};
use crate::bridge::{CodexEvent, parse_wire};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::SyncSender,
    },
    time::{Duration, Instant},
};
use windows::{
    Win32::{
        Foundation::*,
        Security::{Authorization::*, *},
        Storage::FileSystem::*,
        System::{IO::*, Pipes::*, Threading::*},
    },
    core::{PCWSTR, PWSTR},
};

pub fn user_sid() -> windows::core::Result<String> {
    unsafe {
        let mut raw = HANDLE::default();
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut raw)?;
        let token = OwnedHandle(raw);
        let mut size = 0;
        let _ = GetTokenInformation(token.0, TokenUser, None, 0, &mut size);
        if size == 0 {
            return Err(windows::core::Error::from_thread());
        }
        let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
        GetTokenInformation(
            token.0,
            TokenUser,
            Some(buffer.as_mut_ptr().cast()),
            size,
            &mut size,
        )?;
        let info = &*(buffer.as_ptr().cast::<TOKEN_USER>());
        let mut sid = PWSTR::null();
        ConvertSidToStringSidW(info.User.Sid, &mut sid)?;
        let value = sid.to_string();
        let _ = LocalFree(Some(HLOCAL(sid.0.cast())));
        Ok(value?)
    }
}
fn pipe_name() -> windows::core::Result<Vec<u16>> {
    Ok(wide(&format!(r"\\.\pipe\FireflyPet-v1-{}", user_sid()?)))
}
pub fn single_instance() -> windows::core::Result<Option<OwnedHandle>> {
    unsafe {
        let name = wide(&format!("Local\\FireflyPet-{}", user_sid()?));
        let handle = OwnedHandle(CreateMutexW(None, false, PCWSTR(name.as_ptr()))?);
        if GetLastError() == ERROR_ALREADY_EXISTS {
            Ok(None)
        } else {
            Ok(Some(handle))
        }
    }
}
pub fn send(event: &CodexEvent) -> Result<(), String> {
    let name = pipe_name().map_err(|_| "无法确定本地用户".to_string())?;
    send_to(&name, event)
}
fn send_to(name: &[u16], event: &CodexEvent) -> Result<(), String> {
    event.validate()?;
    let bytes = serde_json::to_vec(event).map_err(|_| "事件编码失败".to_string())?;
    send_bytes_to(name, &bytes)
}
fn send_bytes_to(name: &[u16], bytes: &[u8]) -> Result<(), String> {
    unsafe {
        if !WaitNamedPipeW(PCWSTR(name.as_ptr()), 150).as_bool() {
            return Err(format!(
                "桌宠未运行或通道忙：{}",
                windows::core::Error::from_thread()
            ));
        }
        let handle = OwnedHandle(
            CreateFileW(
                PCWSTR(name.as_ptr()),
                GENERIC_WRITE.0,
                FILE_SHARE_MODE(0),
                None,
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED,
                None,
            )
            .map_err(|e| format!("无法连接桌宠：{e}"))?,
        );
        let signal = OwnedHandle(
            CreateEventW(None, true, false, None).map_err(|_| "无法建立发送事件".to_string())?,
        );
        let mut overlapped = OVERLAPPED {
            hEvent: signal.0,
            ..Default::default()
        };
        let mut written = 0;
        let result = WriteFile(
            handle.0,
            Some(bytes),
            Some(&mut written),
            Some(&mut overlapped),
        );
        if result.is_err() {
            if GetLastError() != ERROR_IO_PENDING {
                return Err("事件发送失败".into());
            }
            if WaitForSingleObject(signal.0, 200) != WAIT_OBJECT_0 {
                let _ = CancelIoEx(handle.0, Some(&overlapped));
                let _ = GetOverlappedResult(handle.0, &overlapped, &mut written, true);
                return Err("发送超时".into());
            }
            GetOverlappedResult(handle.0, &overlapped, &mut written, false)
                .map_err(|_| "事件发送失败".to_string())?;
        }
        if written as usize != bytes.len() {
            return Err("事件未完整发送".into());
        }
    }
    Ok(())
}
pub fn listen(stop: Arc<AtomicBool>, tx: SyncSender<CodexEvent>) -> Result<(), String> {
    let name = pipe_name().map_err(|e| e.to_string())?;
    listen_on(&name, stop, tx)
}
fn listen_on(
    name: &[u16],
    stop: Arc<AtomicBool>,
    tx: SyncSender<CodexEvent>,
) -> Result<(), String> {
    let descriptor = wide(&format!(
        "D:P(A;;GA;;;{})",
        user_sid().map_err(|e| e.to_string())?
    ));
    unsafe {
        let mut sd = PSECURITY_DESCRIPTOR::default();
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            PCWSTR(descriptor.as_ptr()),
            1,
            &mut sd,
            None,
        )
        .map_err(|e| e.to_string())?;
        let sa = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: sd.0,
            bInheritHandle: false.into(),
        };
        let handle = OwnedHandle(CreateNamedPipeW(
            PCWSTR(name.as_ptr()),
            PIPE_ACCESS_INBOUND | FILE_FLAG_FIRST_PIPE_INSTANCE | FILE_FLAG_OVERLAPPED,
            PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            1,
            0,
            4096,
            0,
            Some(&sa),
        ));
        let create_error = windows::core::Error::from_thread();
        let _ = LocalFree(Some(HLOCAL(sd.0)));
        if handle.0.is_invalid() {
            return Err(format!("无法建立 Codex 本地通道：{create_error}"));
        }
        let signal = OwnedHandle(CreateEventW(None, true, false, None).map_err(|e| e.to_string())?);
        while !stop.load(Ordering::Relaxed) {
            ResetEvent(signal.0).map_err(|e| e.to_string())?;
            let mut operation = OVERLAPPED {
                hEvent: signal.0,
                ..Default::default()
            };
            let mut transferred = 0;
            let result = ConnectNamedPipe(handle.0, Some(&mut operation));
            let error = GetLastError();
            let connected = result.is_ok()
                || error == ERROR_PIPE_CONNECTED
                || (error == ERROR_IO_PENDING
                    && finish_io(handle.0, &operation, &mut transferred, &stop, None));
            if connected && !stop.load(Ordering::Relaxed) {
                ResetEvent(signal.0).map_err(|e| e.to_string())?;
                let mut operation = OVERLAPPED {
                    hEvent: signal.0,
                    ..Default::default()
                };
                let mut buffer = [0u8; 2049];
                let result = ReadFile(
                    handle.0,
                    Some(&mut buffer),
                    Some(&mut transferred),
                    Some(&mut operation),
                );
                let error = GetLastError();
                let complete = result.is_ok()
                    || (error == ERROR_IO_PENDING
                        && finish_io(
                            handle.0,
                            &operation,
                            &mut transferred,
                            &stop,
                            Some(Instant::now() + Duration::from_millis(300)),
                        ));
                if complete
                    && transferred > 0
                    && let Ok(event) = parse_wire(&buffer[..transferred as usize])
                {
                    let _ = tx.try_send(event);
                }
            }
            let _ = DisconnectNamedPipe(handle.0);
            if !connected {
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
    Ok(())
}

fn finish_io(
    handle: HANDLE,
    operation: &OVERLAPPED,
    transferred: &mut u32,
    stop: &AtomicBool,
    deadline: Option<Instant>,
) -> bool {
    unsafe {
        loop {
            if stop.load(Ordering::Relaxed) || deadline.is_some_and(|end| Instant::now() >= end) {
                // 取消后等待内核释放 OVERLAPPED 和缓冲区，避免退出时悬空引用。
                let _ = CancelIoEx(handle, Some(operation));
                let _ = GetOverlappedResult(handle, operation, transferred, true);
                return false;
            }
            match WaitForSingleObject(operation.hEvent, 25) {
                WAIT_OBJECT_0 => {
                    return GetOverlappedResult(handle, operation, transferred, false).is_ok();
                }
                WAIT_TIMEOUT => (),
                _ => {
                    let _ = CancelIoEx(handle, Some(operation));
                    let _ = GetOverlappedResult(handle, operation, transferred, true);
                    return false;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_pipe_transports_and_reconnects() {
        let name = wide(&format!(
            r"\\.\pipe\FireflyPet-test-{}-{}",
            user_sid().unwrap(),
            std::process::id()
        ));
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, rx) = std::sync::mpsc::sync_channel(8);
        let worker_stop = stop.clone();
        let worker_name = name.clone();
        let worker = std::thread::spawn(move || listen_on(&worker_name, worker_stop, tx));
        let result = std::panic::catch_unwind(|| {
            for index in 0..3 {
                let event = CodexEvent {
                    version: 1,
                    kind: crate::bridge::CodexKind::Stop,
                    session_id: "test-session".into(),
                    turn_id: format!("turn-{index}"),
                    simulated: true,
                };
                let deadline = Instant::now() + Duration::from_secs(2);
                loop {
                    let error = match send_to(&name, &event) {
                        Ok(()) => break,
                        Err(error) => error,
                    };
                    assert!(Instant::now() < deadline, "命名管道未准备好：{error}");
                    std::thread::sleep(Duration::from_millis(20));
                }
                let received = rx.recv_timeout(Duration::from_secs(2)).unwrap();
                assert_eq!(received.turn_id, event.turn_id);
                assert!(received.simulated);
                if index == 0 {
                    send_bytes_to(&name, b"{malformed").unwrap();
                    send_bytes_to(&name, &vec![b'x'; 2049]).unwrap();
                    assert!(rx.recv_timeout(Duration::from_millis(100)).is_err());
                    unsafe {
                        assert!(WaitNamedPipeW(PCWSTR(name.as_ptr()), 150).as_bool());
                        let idle = OwnedHandle(
                            CreateFileW(
                                PCWSTR(name.as_ptr()),
                                GENERIC_WRITE.0,
                                FILE_SHARE_MODE(0),
                                None,
                                OPEN_EXISTING,
                                FILE_FLAG_OVERLAPPED,
                                None,
                            )
                            .unwrap(),
                        );
                        std::thread::sleep(Duration::from_millis(400));
                        drop(idle);
                    }
                }
            }
        });
        stop.store(true, Ordering::Relaxed);
        worker.join().unwrap().unwrap();
        assert!(
            send_to(
                &name,
                &CodexEvent {
                    version: 1,
                    kind: crate::bridge::CodexKind::Stop,
                    session_id: "test".into(),
                    turn_id: "gone".into(),
                    simulated: true
                }
            )
            .is_err()
        );
        if let Err(e) = result {
            std::panic::resume_unwind(e);
        }
    }
}
