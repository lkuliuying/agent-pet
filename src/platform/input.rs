use crate::keyboard::{KeyboardState, scan_code};
use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use windows::{
    Win32::{
        Foundation::*,
        System::{LibraryLoader::GetModuleHandleW, RemoteDesktop::*},
        UI::{
            Input::{KeyboardAndMouse::*, *},
            WindowsAndMessaging::*,
        },
    },
    core::w,
};

struct Context {
    state: Arc<Mutex<KeyboardState>>,
    virtual_keys: RefCell<HashMap<u16, u16>>,
}
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        if message == WM_NCCREATE {
            let create = &*(lparam.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        }
        let context = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Context;
        if !context.is_null() {
            let context = &*context;
            match message {
                WM_INPUT => {
                    let mut raw = RAWINPUT::default();
                    let mut size = std::mem::size_of::<RAWINPUT>() as u32;
                    let received = GetRawInputData(
                        HRAWINPUT(lparam.0 as *mut _),
                        RID_INPUT,
                        Some((&mut raw as *mut RAWINPUT).cast()),
                        &mut size,
                        std::mem::size_of::<RAWINPUTHEADER>() as u32,
                    );
                    if received
                        >= std::mem::size_of::<RAWINPUTHEADER>() as u32
                            + std::mem::size_of::<RAWKEYBOARD>() as u32
                        && received != u32::MAX
                        && raw.header.dwType == RIM_TYPEKEYBOARD.0
                    {
                        let kb = raw.data.keyboard;
                        if let Some(code) = scan_code(kb.MakeCode, kb.Flags, kb.VKey)
                            && let Ok(mut state) = context.state.lock()
                        {
                            state.transition(
                                raw.header.hDevice.0 as usize,
                                code,
                                kb.Flags & RI_KEY_BREAK as u16 == 0,
                                Instant::now(),
                            );
                            // 数字键盘关闭 NumLock 时采用事件中的真实 VK，避免把长按误判为已松开。
                            let vk = if [VK_SHIFT.0, VK_CONTROL.0, VK_MENU.0].contains(&kb.VKey) {
                                MapVirtualKeyW(
                                    if kb.Flags & 2 != 0 {
                                        kb.MakeCode as u32 | 0xe000
                                    } else {
                                        kb.MakeCode as u32
                                    },
                                    MAPVK_VSC_TO_VK_EX,
                                ) as u16
                            } else {
                                kb.VKey
                            };
                            context.virtual_keys.borrow_mut().insert(code, vk);
                        }
                    }
                }
                WM_INPUT_DEVICE_CHANGE if wparam.0 == GIDC_REMOVAL as usize => {
                    if let Ok(mut state) = context.state.lock() {
                        state.remove_device(lparam.0 as usize);
                    }
                }
                WM_WTSSESSION_CHANGE | WM_POWERBROADCAST => {
                    if let Ok(mut state) = context.state.lock() {
                        state.clear();
                    }
                    context.virtual_keys.borrow_mut().clear();
                }
                _ => {}
            }
        }
        DefWindowProcW(hwnd, message, wparam, lparam)
    }
}
pub fn run(stop: Arc<AtomicBool>, state: Arc<Mutex<KeyboardState>>) -> Result<(), String> {
    unsafe {
        let instance = GetModuleHandleW(None).map_err(|e| e.to_string())?;
        let class = w!("FireflyPetInput");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance.into(),
            lpszClassName: class,
            ..Default::default()
        };
        if RegisterClassW(&wc) == 0 {
            return Err("键盘窗口注册失败".into());
        }
        let context = Box::new(Context {
            state: state.clone(),
            virtual_keys: RefCell::new(HashMap::new()),
        });
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class,
            w!("FireflyPetInput"),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance.into()),
            Some((&*context as *const Context).cast()),
        )
        .map_err(|e| e.to_string())?;
        let devices = [RAWINPUTDEVICE {
            usUsagePage: 1,
            usUsage: 6,
            dwFlags: RIDEV_INPUTSINK | RIDEV_DEVNOTIFY,
            hwndTarget: hwnd,
        }];
        if let Err(e) =
            RegisterRawInputDevices(&devices, std::mem::size_of::<RAWINPUTDEVICE>() as u32)
        {
            let _ = DestroyWindow(hwnd);
            return Err(e.to_string());
        }
        let registered_session =
            WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION).is_ok();
        let mut last_reconcile = Instant::now();
        while !stop.load(Ordering::Relaxed) {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            if last_reconcile.elapsed() >= Duration::from_millis(500) {
                if let Ok(mut state) = state.lock() {
                    // 只核对当前按下的键，恢复遗漏的松键事件，不扫描或记录输入文本。
                    for code in state.pressed() {
                        let vk = context
                            .virtual_keys
                            .borrow()
                            .get(&code)
                            .copied()
                            .unwrap_or(0);
                        if vk == 0 || GetAsyncKeyState(vk as i32) >= 0 {
                            state.release(code);
                        }
                    }
                }
                last_reconcile = Instant::now();
            }
            std::thread::sleep(Duration::from_millis(8));
        }
        let remove = [RAWINPUTDEVICE {
            usUsagePage: 1,
            usUsage: 6,
            dwFlags: RIDEV_REMOVE,
            hwndTarget: HWND::default(),
        }];
        let _ = RegisterRawInputDevices(&remove, std::mem::size_of::<RAWINPUTDEVICE>() as u32);
        if registered_session {
            let _ = WTSUnRegisterSessionNotification(hwnd);
        }
        let _ = DestroyWindow(hwnd);
        let _ = UnregisterClassW(class, Some(instance.into()));
    }
    Ok(())
}
