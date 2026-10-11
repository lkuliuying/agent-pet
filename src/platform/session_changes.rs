use super::{OwnedHandle, wide};
use std::path::{Component, Path, PathBuf};
use windows::{
    Win32::{
        Foundation::*,
        Storage::FileSystem::*,
        System::{IO::*, Threading::*},
    },
    core::PCWSTR,
};

pub(super) struct Changes {
    directory: OwnedHandle,
    event: OwnedHandle,
    overlapped: Box<OVERLAPPED>,
    buffer: Vec<u32>,
    root: PathBuf,
    pending: bool,
}

impl Changes {
    pub fn open(root: &Path) -> windows::core::Result<Self> {
        let name = wide(&root.to_string_lossy());
        let directory = OwnedHandle(unsafe {
            CreateFileW(
                PCWSTR(name.as_ptr()),
                FILE_LIST_DIRECTORY.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OVERLAPPED,
                None,
            )?
        });
        let event = OwnedHandle(unsafe { CreateEventW(None, true, false, None)? });
        let overlapped = Box::new(OVERLAPPED {
            hEvent: event.0,
            ..Default::default()
        });
        let mut this = Self {
            directory,
            event,
            overlapped,
            buffer: vec![0; 8192],
            root: root.into(),
            pending: false,
        };
        this.arm()?;
        Ok(this)
    }
    fn arm(&mut self) -> windows::core::Result<()> {
        unsafe {
            ResetEvent(self.event.0)?;
            ReadDirectoryChangesW(
                self.directory.0,
                self.buffer.as_mut_ptr().cast(),
                (self.buffer.len() * 4) as u32,
                true,
                FILE_NOTIFY_CHANGE_FILE_NAME
                    | FILE_NOTIFY_CHANGE_DIR_NAME
                    | FILE_NOTIFY_CHANGE_LAST_WRITE
                    | FILE_NOTIFY_CHANGE_SIZE,
                None,
                Some(&mut *self.overlapped),
                None,
            )?;
        }
        self.pending = true;
        Ok(())
    }
    pub fn poll(&mut self) -> windows::core::Result<Option<Vec<PathBuf>>> {
        if unsafe { WaitForSingleObject(self.event.0, 0) } == WAIT_TIMEOUT {
            return Ok(None);
        }
        let mut length = 0;
        unsafe {
            GetOverlappedResult(self.directory.0, &*self.overlapped, &mut length, false)?;
        }
        self.pending = false;
        if length as usize > self.buffer.len() * 4 {
            return Err(windows::core::Error::from(ERROR_NOTIFY_ENUM_DIR));
        }
        let bytes = unsafe {
            std::slice::from_raw_parts(self.buffer.as_ptr().cast::<u8>(), length as usize)
        };
        let paths = changed_paths(bytes, &self.root)
            .ok_or_else(|| windows::core::Error::from(ERROR_NOTIFY_ENUM_DIR))?;
        self.arm()?;
        Ok(Some(paths))
    }
}

fn changed_paths(bytes: &[u8], root: &Path) -> Option<Vec<PathBuf>> {
    if bytes.is_empty() {
        return None;
    }
    let mut at = 0;
    let mut paths = Vec::new();
    loop {
        let header = bytes.get(at..at + 12)?;
        let next = u32::from_le_bytes(header[..4].try_into().ok()?) as usize;
        let length = u32::from_le_bytes(header[8..12].try_into().ok()?) as usize;
        if !length.is_multiple_of(2) {
            return None;
        }
        let raw: Vec<_> = bytes
            .get(at + 12..at + 12 + length)?
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        let name = PathBuf::from(String::from_utf16(&raw).ok()?);
        if name.components().all(|c| matches!(c, Component::Normal(_)))
            && name.extension().is_some_and(|e| e == "jsonl")
        {
            let path = root.join(name);
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
        if next == 0 {
            return Some(paths);
        }
        if next < 12 + length {
            return None;
        }
        at = at.checked_add(next)?;
    }
}

impl Drop for Changes {
    fn drop(&mut self) {
        if !self.pending {
            return;
        }
        // 等待本次重叠读取取消完成，确保内核不再访问随对象释放的缓冲区。
        unsafe {
            let _ = CancelIoEx(self.directory.0, Some(&*self.overlapped));
            let mut length = 0;
            let _ = GetOverlappedResult(self.directory.0, &*self.overlapped, &mut length, true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn notification_names_cannot_escape_the_sessions_directory() {
        for (name, count) in [
            ("2026\\10\\log.jsonl", 1),
            ("..\\auth.jsonl", 0),
            ("C:\\outside.jsonl", 0),
        ] {
            let raw: Vec<_> = name.encode_utf16().flat_map(u16::to_le_bytes).collect();
            let mut bytes = vec![0; 8];
            bytes.extend((raw.len() as u32).to_le_bytes());
            bytes.extend(raw);
            assert_eq!(
                changed_paths(&bytes, Path::new("C:/sessions"))
                    .unwrap()
                    .len(),
                count
            );
        }
        assert!(changed_paths(&[], Path::new("C:/sessions")).is_none());
    }
}
