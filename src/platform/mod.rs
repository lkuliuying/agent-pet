pub mod codex_usage;
pub mod desktop;
pub mod input;
pub mod ipc;
pub mod system;
pub mod voice;
use windows::Win32::Foundation::{CloseHandle, HANDLE};

pub struct OwnedHandle(pub HANDLE);
impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
}
pub fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}
