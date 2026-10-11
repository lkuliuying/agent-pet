use windows::{
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        UI::{Input::KeyboardAndMouse::*, Shell::*, WindowsAndMessaging::*},
    },
    core::{PCWSTR, w},
};

pub fn alert(text: &str) {
    let text = super::wide(text);
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(text.as_ptr()),
            w!("流萤桌宠"),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}
pub fn configure_pet(hwnd: HWND) {
    unsafe {
        let style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        SetWindowLongW(
            hwnd,
            GWL_EXSTYLE,
            ((style | WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0) & !WS_EX_APPWINDOW.0) as i32,
        );
    }
}
pub fn configure_menu(hwnd: HWND) {
    unsafe {
        let style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        SetWindowLongW(
            hwnd,
            GWL_EXSTYLE,
            ((style | WS_EX_TOOLWINDOW.0) & !(WS_EX_APPWINDOW.0 | WS_EX_NOACTIVATE.0)) as i32,
        );
    }
}
pub fn pointer_press() -> Option<(bool, (i32, i32))> {
    unsafe {
        let mut point = POINT::default();
        GetCursorPos(&mut point).ok()?;
        let pressed = [VK_LBUTTON, VK_RBUTTON, VK_MBUTTON]
            .iter()
            .any(|key| GetAsyncKeyState(key.0 as i32) < 0);
        Some((pressed, (point.x, point.y)))
    }
}
pub fn escape_pressed() -> bool {
    unsafe { GetAsyncKeyState(VK_ESCAPE.0 as i32) < 0 }
}

fn panel_in_area(pet: RECT, width: i32, height: i32, area: RECT) -> (i32, i32) {
    let x = pet.left + (pet.right - pet.left - width) / 2;
    let above = pet.top.saturating_sub(height + 8);
    let y = if above >= area.top {
        above
    } else {
        pet.bottom.saturating_add(8)
    };
    fit_to_work_area((x, y), width, height, area)
}

pub fn panel_position(pet: (i32, i32, i32, i32), width: i32, height: i32) -> (i32, i32) {
    let rect = RECT {
        left: pet.0,
        top: pet.1,
        right: pet.0.saturating_add(pet.2),
        bottom: pet.1.saturating_add(pet.3),
    };
    work_area(pet.0 + pet.2 / 2, pet.1 + pet.3 / 2)
        .map(|area| panel_in_area(rect, width, height, area))
        .unwrap_or((pet.0, pet.1.saturating_sub(height)))
}
fn work_area(x: i32, y: i32) -> Option<RECT> {
    unsafe {
        let monitor = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        GetMonitorInfoW(monitor, &mut info)
            .as_bool()
            .then_some(info.rcWork)
    }
}
fn fit_to_work_area(position: (i32, i32), width: i32, height: i32, area: RECT) -> (i32, i32) {
    (
        position
            .0
            .clamp(area.left, area.right.saturating_sub(width).max(area.left)),
        position
            .1
            .clamp(area.top, area.bottom.saturating_sub(height).max(area.top)),
    )
}
fn popup_in_area(pointer: (i32, i32), width: i32, height: i32, area: RECT) -> (i32, i32) {
    let x = if pointer.0.saturating_add(width) > area.right {
        pointer.0.saturating_sub(width)
    } else {
        pointer.0
    };
    let y = if pointer.1.saturating_add(height) > area.bottom {
        pointer.1.saturating_sub(height)
    } else {
        pointer.1
    };
    fit_to_work_area((x, y), width, height, area)
}
pub fn popup_position(pointer: (i32, i32), width: i32, height: i32) -> (i32, i32) {
    work_area(pointer.0, pointer.1)
        .map(|area| popup_in_area(pointer, width, height, area))
        .unwrap_or(pointer)
}
pub fn reset_position(current: (i32, i32), width: i32, height: i32) -> (i32, i32) {
    work_area(current.0, current.1)
        .map(|area| {
            fit_to_work_area(
                (area.right - width - 32, area.bottom - height - 24),
                width,
                height,
                area,
            )
        })
        .unwrap_or_else(|| initial_position(width, height))
}
pub fn clamp_position(x: i32, y: i32, width: i32, height: i32) -> (i32, i32) {
    work_area(x, y)
        .map(|area| fit_to_work_area((x, y), width, height, area))
        .unwrap_or((x, y))
}
pub fn initial_position(width: i32, height: i32) -> (i32, i32) {
    unsafe {
        let mut rect = RECT::default();
        if SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some((&mut rect as *mut RECT).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .is_ok()
        {
            (rect.right - width - 32, rect.bottom - height - 24)
        } else {
            (32, 32)
        }
    }
}
pub fn fullscreen_or_busy() -> bool {
    unsafe {
        if let Ok(s) = SHQueryUserNotificationState()
            && [
                QUNS_RUNNING_D3D_FULL_SCREEN,
                QUNS_PRESENTATION_MODE,
                QUNS_NOT_PRESENT,
            ]
            .contains(&s)
        {
            return true;
        }
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() || hwnd == GetShellWindow() || hwnd == GetDesktopWindow() {
            return false;
        }
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == std::process::id() {
            return false;
        }
        let mut class = [0u16; 128];
        let count = GetClassNameW(hwnd, &mut class);
        let name = String::from_utf16_lossy(&class[..count.max(0) as usize]);
        if ["WorkerW", "Progman"].contains(&name.as_str()) {
            return false;
        }
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_err() {
            return false;
        }
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        GetMonitorInfoW(monitor, &mut info).as_bool()
            && rect.left <= info.rcMonitor.left
            && rect.top <= info.rcMonitor.top
            && rect.right >= info.rcMonitor.right
            && rect.bottom >= info.rcMonitor.bottom
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_panel_prefers_above_and_fits_negative_monitor_edges() {
        let area = RECT {
            left: -1920,
            top: -200,
            right: 0,
            bottom: 840,
        };
        let pet = RECT {
            left: -290,
            top: 525,
            right: -20,
            bottom: 840,
        };
        assert_eq!(panel_in_area(pet, 320, 416, area), (-320, 101));
        let top = RECT {
            left: -1920,
            top: -200,
            right: -1650,
            bottom: 115,
        };
        assert_eq!(panel_in_area(top, 320, 416, area), (-1920, 123));
    }

    #[test]
    fn popup_flips_at_work_area_edges_and_handles_negative_monitors() {
        let primary = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1040,
        };
        assert_eq!(popup_in_area((1900, 1020), 248, 300, primary), (1652, 720));
        assert_eq!(popup_in_area((0, 0), 248, 300, primary), (0, 0));
        let left = RECT {
            left: -1920,
            top: -200,
            right: 0,
            bottom: 840,
        };
        assert_eq!(popup_in_area((-10, 830), 372, 450, left), (-382, 380));
        assert_eq!(popup_in_area((-1920, -200), 372, 450, left), (-1920, -200));
        assert_eq!(popup_in_area((-10, 830), 2500, 1600, left), (-1920, -200));
    }
}
