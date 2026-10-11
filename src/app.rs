use pet_desktop::{
    assets::{self, Sprite},
    bridge::{CodexEvent, CodexKind},
    config::{self, Config},
    keyboard::KeyboardState,
    notices::{Announcer, Notice, SystemRules},
    platform::{
        self,
        voice::{VoiceCommand, VoiceEvent},
    },
    ui_state::{self, QuickAction},
};
use slint::winit_030::{
    WinitWindowAccessor,
    winit::{
        platform::windows::WindowAttributesExtWindows,
        raw_window_handle::{HasWindowHandle, RawWindowHandle},
    },
};
use slint::{ComponentHandle, PhysicalPosition, PhysicalSize, Timer, TimerMode, VecModel};
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
use windows::Win32::Foundation::HWND;
slint::include_modules!();
mod usage_panel;
use usage_panel::UsagePanel;

struct Workers {
    stop: Arc<AtomicBool>,
    handles: Vec<std::thread::JoinHandle<()>>,
}
impl Workers {
    fn new() -> Self {
        Self {
            stop: Arc::new(AtomicBool::new(false)),
            handles: Vec::new(),
        }
    }
    fn spawn(&mut self, name: &str, f: impl FnOnce() + Send + 'static) -> std::io::Result<()> {
        self.handles
            .push(std::thread::Builder::new().name(name.into()).spawn(f)?);
        Ok(())
    }
}
impl Drop for Workers {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for handle in self.handles.drain(..) {
            let _ = handle.join();
        }
    }
}
struct State {
    config: Config,
    directory: PathBuf,
    announcer: Announcer,
    rules: SystemRules,
    bubble_until: Option<Instant>,
    happy_until: Option<Instant>,
    surprised_until: Option<Instant>,
    voices: Vec<(String, String)>,
    position_dirty: Option<Instant>,
    last_position: Option<(i32, i32)>,
}
fn sprite(s: Sprite) -> SpriteSpec {
    SpriteSpec {
        sx: s.sx,
        sy: s.sy,
        sw: s.sw,
        sh: s.sh,
        px: s.x,
        py: s.y,
        w: s.width,
        h: s.height,
    }
}
fn values(c: &Config, voices: &[(String, String)]) -> SettingsValues {
    SettingsValues {
        scale_percent: c.scale * 100.,
        paused: c.paused,
        passthrough: c.passthrough,
        quiet: c.quiet,
        quiet_fullscreen: c.quiet_fullscreen,
        voice: c.voice,
        volume: c.volume as i32,
        battery: c.battery,
        power: c.power,
        network: c.network,
        cpu: c.cpu,
        memory: c.memory,
        codex: c.codex,
        voice_index: voices
            .iter()
            .position(|(id, _)| id == &c.voice_id)
            .map(|index| index as i32)
            .unwrap_or(if c.voice_id.is_empty() && !voices.is_empty() {
                0
            } else {
                -1
            }),
    }
}
fn draft_config(draft: SettingsValues, current: &Config, voices: &[(String, String)]) -> Config {
    let mut c = current.clone();
    c.scale = draft.scale_percent / 100.;
    c.paused = draft.paused;
    c.passthrough = draft.passthrough;
    c.quiet = draft.quiet;
    c.quiet_fullscreen = draft.quiet_fullscreen;
    c.voice = draft.voice;
    c.volume = draft.volume.clamp(0, 100) as u16;
    if draft.voice_index >= 0
        && let Some(voice) = voices.get(draft.voice_index as usize)
    {
        c.voice_id.clone_from(&voice.0);
    }
    c.battery = draft.battery;
    c.power = draft.power;
    c.network = draft.network;
    c.cpu = draft.cpu;
    c.memory = draft.memory;
    c.codex = draft.codex;
    c
}
fn populate(settings: &SettingsWindow, c: &Config, voices: &[(String, String)]) {
    let v = values(c, voices);
    settings.set_draft(v.clone());
    settings.set_applied(v);
}
fn apply_visual(pet: &PetWindow, tray: &PetTray, menu: &QuickMenuWindow, c: &Config) {
    pet.set_factor(c.scale);
    pet.set_headroom(ui_state::HEADROOM);
    tray.set_paused(c.paused);
    tray.set_quiet(c.quiet);
    tray.set_passthrough(c.passthrough);
    menu.set_paused(c.paused);
    menu.set_quiet(c.quiet);
    menu.set_passthrough(c.passthrough);
    pet.window().set_size(slint::LogicalSize::new(
        ui_state::PET_WIDTH * c.scale,
        (ui_state::PET_HEIGHT + ui_state::HEADROOM) * c.scale,
    ));
    apply_native_window(pet, c);
}
fn apply_native_window(pet: &PetWindow, c: &Config) -> bool {
    pet.window()
        .with_winit_window(|w| {
            let _ = w.set_cursor_hittest(!c.passthrough);
            // Winit 切换穿透会重写扩展样式，随后重新确保桌宠不激活、不出现在任务栏。
            if let Ok(handle) = w.window_handle()
                && let RawWindowHandle::Win32(h) = handle.as_raw()
            {
                platform::desktop::configure_pet(HWND(h.hwnd.get() as *mut _));
            }
        })
        .is_some()
}
fn place(pet: &PetWindow, position: Option<(i32, i32)>) {
    let PhysicalSize { width, height } = pet.window().size();
    let (x, y) = position
        .map(|p| ui_state::window_position(p, headroom(pet)))
        .unwrap_or_else(|| platform::desktop::initial_position(width as i32, height as i32));
    let (x, y) = platform::desktop::clamp_position(x, y, width as i32, height as i32);
    pet.window().set_position(PhysicalPosition::new(x, y));
}
fn headroom(pet: &PetWindow) -> i32 {
    ui_state::headroom_pixels(pet.get_factor(), pet.window().scale_factor() as f64)
}
fn reset_position(pet: &PetWindow) -> (i32, i32) {
    let pos = pet.window().position();
    let size = pet.window().size();
    let position =
        platform::desktop::reset_position((pos.x, pos.y), size.width as i32, size.height as i32);
    ui_state::saved_position(position, headroom(pet))
}
fn display(settings: &SettingsWindow, state: &State) {
    if !settings.window().is_visible() {
        populate(settings, &state.config, &state.voices);
        settings.set_page(0);
    }
    match settings.show() {
        Ok(()) => {
            settings.window().with_winit_window(|w| w.focus_window());
        }
        Err(e) => platform::desktop::alert(&format!("无法打开设置：{e}")),
    }
}
pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut directory = config::default_directory()?;
    let mut exit_after = None;
    for arg in args {
        if let Some(path) = arg.strip_prefix("--data-dir=") {
            directory = PathBuf::from(path);
        } else if let Some(seconds) = arg.strip_prefix("--exit-after=") {
            exit_after = Some(Duration::from_secs(seconds.parse::<u64>()?.clamp(1, 3600)));
        } else {
            return Err(format!("不支持的参数：{arg}").into());
        }
    }
    let Some(_instance) = platform::ipc::single_instance()? else {
        platform::desktop::alert("流萤已经在陪你啦，请从系统托盘打开设置。");
        return Ok(());
    };
    let character = assets::firefly()?;
    let (config, warning) = config::load(&directory);
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("femtovg-wgpu".into())
        .with_winit_window_attributes_hook(|attrs| {
            let tool = attrs.title == "流萤桌宠 · 快捷操作" || attrs.title == "流萤桌宠 · 当前状态";
            attrs
                .with_no_redirection_bitmap(true)
                .with_active(false)
                .with_skip_taskbar(tool)
        })
        .select()?;
    let pet = PetWindow::new()?;
    let tray = PetTray::new()?;
    let settings = SettingsWindow::new()?;
    let menu = QuickMenuWindow::new()?;
    let menu_pointer = Rc::new(Cell::new(None::<(i32, i32)>));
    let menu_dismiss_armed = Rc::new(Cell::new(false));
    settings.set_status_error(warning.is_some());
    settings.set_status(
        warning
            .unwrap_or_else(|| "设置保存在本机，点击“保存并应用”生效。".into())
            .into(),
    );
    pet.set_body(sprite(character.body));
    pet.set_left_hand(sprite(character.left_hand));
    pet.set_right_hand(sprite(character.right_hand));
    pet.set_face(sprite(character.faces[0]));
    pet.set_keyboard(sprite(character.keyboard));
    pet.set_label_y(character.anchors.label_y);
    pet.set_left_tap_dy(character.anchors.left_tap_dy);
    pet.set_right_tap_dy(character.anchors.right_tap_dy);
    let keyboard = Arc::new(Mutex::new(KeyboardState::default()));
    keyboard
        .lock()
        .map_err(|_| "键盘状态不可用")?
        .pause(config.paused);
    let state = Rc::new(RefCell::new(State {
        config,
        directory,
        announcer: Announcer::default(),
        rules: SystemRules::default(),
        bubble_until: None,
        happy_until: None,
        surprised_until: None,
        voices: Vec::new(),
        position_dirty: None,
        last_position: None,
    }));
    let mut workers = Workers::new();
    let usage_generation = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let (usage_tx, usage_rx) = mpsc::sync_channel(1);
    let (usage_result_tx, usage_result_rx) = mpsc::sync_channel(2);
    let usage_panel = UsagePanel::new(
        &settings,
        usage_generation.clone(),
        usage_tx,
        usage_result_rx,
    )?;
    {
        let stop = workers.stop.clone();
        workers.spawn("pet-quota", move || {
            platform::codex_usage::run(stop, usage_generation, usage_rx, usage_result_tx)
        })?;
    }
    let (errors_tx, errors_rx) = mpsc::sync_channel::<String>(16);
    let (system_tx, system_rx) = mpsc::sync_channel(2);
    let (codex_tx, codex_rx) = mpsc::sync_channel(32);
    let (voice_tx, voice_rx) = mpsc::sync_channel(8);
    let (voice_event_tx, voice_event_rx) = mpsc::sync_channel(8);
    {
        let stop = workers.stop.clone();
        let keyboard = keyboard.clone();
        let errors = errors_tx.clone();
        workers.spawn("pet-input", move || {
            if let Err(e) = platform::input::run(stop, keyboard) {
                let _ = errors.try_send(format!("键盘同步不可用：{e}"));
            }
        })?;
    }
    {
        let stop = workers.stop.clone();
        workers.spawn("pet-system", move || platform::system::run(stop, system_tx))?;
    }
    {
        let stop = workers.stop.clone();
        let errors = errors_tx.clone();
        workers.spawn("pet-codex", move || {
            if let Err(e) = platform::ipc::listen(stop, codex_tx) {
                let _ = errors.try_send(e);
            }
        })?;
    }
    {
        let stop = workers.stop.clone();
        workers.spawn("pet-voice", move || {
            platform::voice::run(stop, voice_rx, voice_event_tx)
        })?;
    }
    populate(&settings, &state.borrow().config, &[]);
    apply_visual(&pet, &tray, &menu, &state.borrow().config);
    pet.show()?;
    place(&pet, state.borrow().config.position);
    {
        let state = state.clone();
        let panel = usage_panel.clone();
        let wp = pet.as_weak();
        pet.on_pat(move || {
            state.borrow_mut().happy_until = Some(Instant::now() + Duration::from_secs(2));
            if let Some(pet) = wp.upgrade() {
                panel.toggle(&pet);
            }
        });
    }
    {
        let state = state.clone();
        let weak = pet.as_weak();
        let panel = usage_panel.clone();
        pet.on_drag_request(move |dx, dy| {
            panel.close();
            state.borrow_mut().surprised_until = Some(Instant::now() + Duration::from_secs(2));
            if let Some(pet) = weak.upgrade() {
                pet.window().with_winit_window(|w| {
                    // 按当前屏幕 DPI 移动无激活窗口，避免系统非客户区拖动抢焦点。
                    if let Ok(pos) = w.outer_position() {
                        let dpi = w.scale_factor();
                        w.set_outer_position(slint::winit_030::winit::dpi::PhysicalPosition::new(
                            (pos.x as f64 + dx as f64 * dpi).round() as i32,
                            (pos.y as f64 + dy as f64 * dpi).round() as i32,
                        ));
                    }
                });
            }
        });
    }
    {
        let wm = menu.as_weak();
        menu.on_dismiss(move || {
            if let Some(m) = wm.upgrade() {
                let _ = m.hide();
            }
        });
    }
    menu.window().on_winit_window_event(|window, event| {
        use slint::winit_030::{EventResult, winit::event::{ElementState, WindowEvent}};
        let dismiss = matches!(event, WindowEvent::Focused(false))
            || matches!(event, WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed && event.logical_key == slint::winit_030::winit::keyboard::Key::Named(slint::winit_030::winit::keyboard::NamedKey::Escape));
        if dismiss { let _ = window.hide(); }
        EventResult::Propagate
    });
    {
        let wm = menu.as_weak();
        pet.on_dismiss_menu(move || {
            if let Some(m) = wm.upgrade() {
                let _ = m.hide();
            }
        });
    }
    {
        let wp = pet.as_weak();
        let wm = menu.as_weak();
        let pointer = menu_pointer.clone();
        let armed = menu_dismiss_armed.clone();
        let panel = usage_panel.clone();
        pet.on_context_request(move |x, y| {
            panel.close();
            let (Some(p), Some(m)) = (wp.upgrade(), wm.upgrade()) else {
                return;
            };
            let pos = p.window().position();
            let dpi = p.window().scale_factor();
            let at = (
                pos.x + (x * dpi).round() as i32,
                pos.y + (y * dpi).round() as i32,
            );
            pointer.set(Some(at));
            armed.set(false);
            // 先按角色所在屏幕的 DPI 放置，原生菜单创建后再按实际尺寸校正。
            let target =
                platform::desktop::popup_position(at, (248. * dpi) as i32, (300. * dpi) as i32);
            m.window()
                .set_position(PhysicalPosition::new(target.0, target.1));
            if let Err(e) = m.show() {
                platform::desktop::alert(&format!("无法打开快捷菜单：{e}"));
                return;
            }
            m.invoke_focus_menu();
            m.window().with_winit_window(|w| {
                if let Ok(handle) = w.window_handle()
                    && let RawWindowHandle::Win32(h) = handle.as_raw()
                {
                    platform::desktop::configure_menu(HWND(h.hwnd.get() as *mut _));
                }
                w.focus_window();
            });
        });
    }
    {
        let state = state.clone();
        let weak = settings.as_weak();
        let panel = usage_panel.clone();
        tray.on_settings_request(move || {
            panel.close();
            if let Some(s) = weak.upgrade() {
                display(&s, &state.borrow());
            }
        });
    }
    let quick_action: Rc<dyn Fn(QuickAction)> = {
        let state = state.clone();
        let wp = pet.as_weak();
        let wt = tray.as_weak();
        let ws = settings.as_weak();
        let wm = menu.as_weak();
        let keyboard = keyboard.clone();
        let voice = voice_tx.clone();
        let panel = usage_panel.clone();
        Rc::new(move |action: QuickAction| {
            if let (Some(p), Some(t), Some(s), Some(m)) =
                (wp.upgrade(), wt.upgrade(), ws.upgrade(), wm.upgrade())
            {
                let mut state = state.borrow_mut();
                let mut draft = draft_config(s.get_draft(), &state.config, &state.voices);
                let candidate = action.candidate(&state.config);
                let directory = state.directory.clone();
                if let Err(e) = ui_state::commit(&mut state.config, candidate, &directory) {
                    let message = format!("保存失败，原设置仍有效：{e}");
                    s.set_status(message.clone().into());
                    s.set_status_error(true);
                    drop(state);
                    platform::desktop::alert(&message);
                    return;
                }
                action.merge_into_draft(&mut draft, &state.config);
                s.set_draft(values(&draft, &state.voices));
                s.set_applied(values(&state.config, &state.voices));
                s.set_status("快捷操作已保存，其他修改仍保留。".into());
                s.set_status_error(false);
                if let Ok(mut kb) = keyboard.lock() {
                    kb.pause(state.config.paused);
                }
                if state.config.quiet {
                    state.announcer.clear();
                    state.bubble_until = None;
                    p.set_bubble("".into());
                    let _ = voice.try_send(VoiceCommand::Stop);
                }
                apply_visual(&p, &t, &m, &state.config);
                if state.config.passthrough || matches!(action, QuickAction::ResetPosition(_)) {
                    panel.close();
                }
                if matches!(action, QuickAction::Restore | QuickAction::ResetPosition(_)) {
                    if let Err(e) = p.show() {
                        s.set_status(format!("无法显示桌宠：{e}").into());
                        s.set_status_error(true);
                    }
                    place(&p, state.config.position);
                }
            }
        })
    };
    {
        let action = quick_action.clone();
        tray.on_restore_pet(move || action(QuickAction::Restore));
        let action = quick_action.clone();
        tray.on_toggle_pause(move || action(QuickAction::TogglePause));
        let action = quick_action.clone();
        tray.on_toggle_quiet(move || action(QuickAction::ToggleQuiet));
        let action = quick_action.clone();
        tray.on_toggle_passthrough(move || action(QuickAction::TogglePassthrough));
    }
    for is_tray in [false, true] {
        let action = quick_action.clone();
        let wp = pet.as_weak();
        let reset = move || {
            if let Some(p) = wp.upgrade() {
                action(QuickAction::ResetPosition(reset_position(&p)));
            }
        };
        if is_tray {
            tray.on_reset_position(reset);
        } else {
            settings.on_reset_position(reset);
        }
    }
    {
        let action = quick_action.clone();
        let wm = menu.as_weak();
        let wp = pet.as_weak();
        let ws = settings.as_weak();
        let state = state.clone();
        let panel = usage_panel.clone();
        menu.on_action(move |selected| {
            if let Some(m) = wm.upgrade() {
                let _ = m.hide();
            }
            match selected {
                MenuAction::Pause => action(QuickAction::TogglePause),
                MenuAction::Quiet => action(QuickAction::ToggleQuiet),
                MenuAction::Passthrough => action(QuickAction::TogglePassthrough),
                MenuAction::ResetPosition => {
                    if let Some(p) = wp.upgrade() {
                        action(QuickAction::ResetPosition(reset_position(&p)));
                    }
                }
                MenuAction::Settings => {
                    panel.close();
                    if let Some(s) = ws.upgrade() {
                        display(&s, &state.borrow());
                    }
                }
                MenuAction::Quit => {
                    let _ = slint::quit_event_loop();
                }
            }
        });
    }
    tray.on_quit_request(|| {
        let _ = slint::quit_event_loop();
    });
    {
        let state = state.clone();
        let ws = settings.as_weak();
        let wp = pet.as_weak();
        let wt = tray.as_weak();
        let wm = menu.as_weak();
        let keyboard = keyboard.clone();
        let voice = voice_tx.clone();
        let panel = usage_panel.clone();
        settings.on_apply(move || {
            let (Some(s), Some(p), Some(t), Some(m)) =
                (ws.upgrade(), wp.upgrade(), wt.upgrade(), wm.upgrade())
            else {
                return;
            };
            let mut state = state.borrow_mut();
            let c = draft_config(s.get_draft(), &state.config, &state.voices);
            let directory = state.directory.clone();
            if let Err(e) = ui_state::commit(&mut state.config, c.clone(), &directory) {
                s.set_status(format!("保存失败，原设置仍有效：{e}").into());
                s.set_status_error(true);
                return;
            }
            if let Ok(mut kb) = keyboard.lock() {
                kb.pause(c.paused);
            }
            if !c.voice || c.quiet {
                let _ = voice.try_send(VoiceCommand::Stop);
            }
            if c.quiet {
                state.announcer.clear();
                state.bubble_until = None;
                p.set_bubble("".into());
            }
            state.announcer.remove_disabled(&c);
            apply_visual(&p, &t, &m, &state.config);
            if c.passthrough {
                panel.close();
            }
            place(&p, state.config.position);
            populate(&s, &state.config, &state.voices);
            s.set_status("已保存并应用。".into());
            s.set_status_error(false);
        });
    }
    {
        let state = state.clone();
        let ws = settings.as_weak();
        let tx = voice_tx.clone();
        settings.on_preview_voice(move || {
            if let Some(s) = ws.upgrade() {
                let state = state.borrow();
                let id = state
                    .voices
                    .get(s.get_draft().voice_index.max(0) as usize)
                    .map(|v| v.0.clone())
                    .unwrap_or_default();
                if state.voices.is_empty() {
                    s.set_voice_status("没有可用音色，无法试听。".into());
                    return;
                }
                if tx
                    .try_send(VoiceCommand::Say {
                        text: "你好，我是流萤。今天也一起加油吧。".into(),
                        voice_id: id,
                        volume: s.get_draft().volume.clamp(0, 100) as u16,
                    })
                    .is_err()
                {
                    s.set_voice_status("语音通道暂时不可用。".into());
                }
            }
        });
    }
    {
        let state = state.clone();
        settings.on_test_codex(move || {
            let event = CodexEvent {
                version: 1,
                kind: CodexKind::Stop,
                session_id: "settings-demo".into(),
                turn_id: format!(
                    "{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis()
                ),
                simulated: true,
            };
            state
                .borrow_mut()
                .announcer
                .push(event.notice(), Instant::now());
        });
    }
    let started = Instant::now();
    let mut last_busy_check = Instant::now() - Duration::from_secs(2);
    let mut busy = false;
    let mut was_suppressed = false;
    let mut placed = false;
    let mut native_configured = false;
    let mut last_window_size = None;
    let mut last_menu_size = None;
    let shutdown_state = state.clone();
    let wp = pet.as_weak();
    let ws = settings.as_weak();
    let wm = menu.as_weak();
    let timer = Timer::default();
    timer.start(TimerMode::Repeated, Duration::from_millis(16), move || {
        let (Some(p), Some(s)) = (wp.upgrade(), ws.upgrade()) else {
            return;
        };
        let now = Instant::now();
        usage_panel.tick(&p);
        if let Some(m) = wm.upgrade() {
            if m.window().is_visible() {
                let size = m.window().size();
                // 无激活窗口被点击时不一定产生失焦；只在菜单打开期间补充检查外部点击。
                // 等唤出菜单的右键先松开，避免边缘翻转后把该次按下当成外部点击。
                if let Some((pressed, point)) = platform::desktop::pointer_press() {
                    if !pressed {
                        menu_dismiss_armed.set(true);
                    }
                    let pos = m.window().position();
                    let inside = point.0 >= pos.x
                        && point.1 >= pos.y
                        && point.0 < pos.x + size.width as i32
                        && point.1 < pos.y + size.height as i32;
                    if pressed && menu_dismiss_armed.get() && !inside {
                        let _ = m.hide();
                    }
                }
                let geometry = (size.width, size.height, menu_pointer.get());
                if last_menu_size != Some(geometry)
                    && let Some(at) = menu_pointer.get()
                {
                    let pos = platform::desktop::popup_position(
                        at,
                        size.width as i32,
                        size.height as i32,
                    );
                    m.window().set_position(PhysicalPosition::new(pos.0, pos.1));
                    last_menu_size = Some(geometry);
                }
            } else {
                last_menu_size = None;
            }
        }
        if exit_after.is_some_and(|limit| started.elapsed() >= limit) {
            let _ = slint::quit_event_loop();
            return;
        }
        let mut state = state.borrow_mut();
        // 原生窗口在事件循环启动后才存在，首次成功访问时恢复穿透和无激活样式。
        if !native_configured {
            native_configured = apply_native_window(&p, &state.config);
        }
        // 等首帧确定显示器 DPI 后再定位，防止逻辑尺寸被当作物理尺寸导致越界。
        if !placed && started.elapsed() > Duration::from_millis(150) {
            place(&p, state.config.position);
            placed = true;
        }
        let size = p.window().size();
        if placed && last_window_size.is_some_and(|old| old != (size.width, size.height)) {
            place(&p, state.config.position);
        }
        last_window_size = Some((size.width, size.height));
        if last_busy_check.elapsed() >= Duration::from_secs(1) {
            busy = platform::desktop::fullscreen_or_busy();
            last_busy_check = now;
        }
        let suppressed = state.config.quiet || (state.config.quiet_fullscreen && busy);
        if suppressed && !was_suppressed {
            let _ = voice_tx.try_send(VoiceCommand::Stop);
            p.set_bubble("".into());
            state.bubble_until = None;
            state.announcer.clear();
        }
        was_suppressed = suppressed;
        while let Ok(error) = errors_rx.try_recv() {
            s.set_status(error.clone().into());
            s.set_status_error(true);
            state
                .announcer
                .push(Notice::system("platform-error", &error), now);
        }
        while let Ok(event) = voice_event_rx.try_recv() {
            match event {
                VoiceEvent::Ready(voices) => {
                    state.voices = voices;
                    s.set_voices(
                        Rc::new(VecModel::from(
                            state
                                .voices
                                .iter()
                                .map(|(_, name)| name.clone().into())
                                .collect::<Vec<_>>(),
                        ))
                        .into(),
                    );
                    s.set_voice_status(
                        if state.voices.is_empty() {
                            "没有可用音色，文字提醒仍正常显示。"
                        } else {
                            "使用 Windows 已安装音色；语音默认关闭。"
                        }
                        .into(),
                    );
                    let applied = values(&state.config, &state.voices);
                    let mut draft = s.get_draft();
                    draft.voice_index = applied.voice_index;
                    s.set_draft(draft);
                    s.set_applied(applied);
                }
                VoiceEvent::Error(error) => s.set_voice_status(error.into()),
            }
        }
        while let Ok(sample) = system_rx.try_recv() {
            usage_panel.metrics(sample.cpu, sample.memory);
            s.set_battery_status(
                format!(
                    "当前电量：{}",
                    sample
                        .battery
                        .map(|b| format!("{b}%"))
                        .unwrap_or_else(|| "无电池或不可用".into())
                )
                .into(),
            );
            let config = state.config.clone();
            for notice in state.rules.update(&sample, &config, now) {
                state.announcer.push(notice, now);
            }
        }
        while let Ok(event) = codex_rx.try_recv() {
            if event.simulated {
                s.set_codex_status("已收到管道模拟事件，真实接入尚需在 Codex 中验证。".into());
            } else {
                s.set_codex_status(
                    format!(
                        "已收到 hooks 状态事件：{:?}。来源验收请在 Codex 中触发。",
                        event.kind
                    )
                    .into(),
                );
            }
            if state.config.codex {
                state.announcer.push(event.notice(), now);
            }
        }
        if state.bubble_until.is_some_and(|end| now >= end) {
            p.set_bubble("".into());
            state.bubble_until = None;
        }
        if state.bubble_until.is_none()
            && let Some(notice) = state.announcer.next(now, suppressed)
        {
            p.set_bubble(notice.text.clone().into());
            p.set_simulation(notice.simulated);
            state.bubble_until = Some(now + Duration::from_secs(7));
            if state.config.voice && !state.voices.is_empty() {
                let text = if notice.simulated {
                    format!("这是一条演示提醒。{}", notice.text)
                } else {
                    notice.text
                };
                if voice_tx
                    .try_send(VoiceCommand::Say {
                        text,
                        voice_id: state.config.voice_id.clone(),
                        volume: state.config.volume,
                    })
                    .is_err()
                {
                    s.set_voice_status("语音通道暂时不可用，已保留文字提醒。".into());
                }
            }
        }
        let mut face = 0;
        if let Ok(mut kb) = keyboard.lock() {
            let (label, opacity) = kb.hint(now);
            p.set_key_label(label.into());
            p.set_key_opacity(opacity);
            p.set_left_tap(kb.left_until.is_some_and(|end| now < end));
            p.set_right_tap(kb.right_until.is_some_and(|end| now < end));
            let idle = kb
                .last_input
                .map(|t| now.saturating_duration_since(t))
                .unwrap_or(started.elapsed());
            if idle < Duration::from_millis(1200) {
                face = 2;
            } else if idle > Duration::from_secs(90) {
                face = 4;
            }
        }
        if state.bubble_until.is_some() {
            face = 5;
        }
        if state.happy_until.is_some_and(|end| now < end) {
            face = 3;
        }
        if state.surprised_until.is_some_and(|end| now < end) {
            face = 5;
        }
        if face == 0 && started.elapsed().as_millis() % 4800 < 150 {
            face = 1;
        }
        p.set_face(sprite(character.faces[face]));
        let phase = (started.elapsed().as_millis() / 80) as f32 * 0.08;
        p.set_breath((phase * 1.4).sin() * assets::BREATH_AMPLITUDE);
        if placed
            && let Some(pos) = p
                .window()
                .with_winit_window(|w| w.outer_position().ok())
                .flatten()
        {
            let pos = ui_state::saved_position((pos.x, pos.y), headroom(&p));
            if state.last_position != Some(pos) {
                state.last_position = Some(pos);
                state.config.position = Some(pos);
                state.position_dirty = Some(now);
            }
        }
        if state
            .position_dirty
            .is_some_and(|at| now.saturating_duration_since(at) >= Duration::from_secs(1))
        {
            if let Err(e) = config::save(&state.directory, &state.config) {
                s.set_status(format!("窗口位置保存失败：{e}").into());
                s.set_status_error(true);
            }
            state.position_dirty = None;
        }
    });
    let event_loop_result = slint::run_event_loop();
    drop(timer);
    drop(workers);
    event_loop_result?;
    let state = shutdown_state.borrow();
    if state.position_dirty.is_some() {
        config::save(&state.directory, &state.config)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_roundtrip_preserves_v1_position_and_unavailable_voice() {
        let c = Config {
            tkl: true,
            position: Some((-1200, 300)),
            voice_id: "unavailable-system-voice".into(),
            ..Config::default()
        };
        let voices = vec![("another-voice".into(), "其他音色".into())];
        for available in [&[][..], voices.as_slice()] {
            let mut draft = values(&c, available);
            assert_eq!(draft.voice_index, -1);
            assert_eq!(draft_config(draft.clone(), &c, available), c);
            draft.volume = 30;
            draft.network = false;
            let saved = draft_config(draft, &c, available);
            assert_eq!(saved.voice_id, c.voice_id);
            assert_eq!(saved.position, c.position);
            assert_eq!(saved.version, 1);
            assert!(saved.tkl);
            assert_eq!(saved.volume, 30);
            assert!(!saved.network);
        }
    }
}
