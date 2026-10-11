use pet_desktop::{
    assets::{self, Sprite},
    ui_state,
};
use slint::{ComponentHandle, LogicalSize, Timer, TimerMode};
use std::{
    cell::{Cell, RefCell},
    fs::File,
    io::{BufWriter, Write},
    path::Path,
    rc::Rc,
    time::Duration,
};

slint::include_modules!();
#[path = "../src/app/usage_panel.rs"]
mod usage_panel;

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

fn snapshot(window: &slint::Window, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // 只截取本示例的渲染缓冲区，不采集桌面或其他应用；BMP 无需新增依赖。
    let pixels = window.take_snapshot()?;
    let (width, height) = (pixels.width(), pixels.height());
    let bytes = width
        .checked_mul(height)
        .and_then(|n| n.checked_mul(4))
        .ok_or("截图尺寸溢出")?;
    let mut file = BufWriter::new(File::create_new(path)?);
    file.write_all(b"BM")?;
    file.write_all(&(54 + bytes).to_le_bytes())?;
    file.write_all(&[0; 4])?;
    file.write_all(&54u32.to_le_bytes())?;
    file.write_all(&40u32.to_le_bytes())?;
    file.write_all(&(width as i32).to_le_bytes())?;
    file.write_all(&(-(height as i32)).to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&32u16.to_le_bytes())?;
    file.write_all(&[0; 24])?;
    for pixel in pixels.as_slice() {
        file.write_all(&[pixel.b, pixel.g, pixel.r, 255])?;
    }
    file.flush()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::args()
        .nth(1)
        .ok_or("请提供一个新的截图输出目录")?;
    let directory = Path::new(&directory).to_path_buf();
    std::fs::create_dir(&directory)?;
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("femtovg-wgpu".into())
        .select()?;
    let pet = PetWindow::new()?;
    let settings = SettingsWindow::new()?;
    let menu = QuickMenuWindow::new()?;
    let panel = StatusPanelWindow::new()?;
    let generation = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let (requests_tx, requests_rx) = std::sync::mpsc::sync_channel(1);
    let (results_tx, results_rx) = std::sync::mpsc::sync_channel(2);
    let binding =
        usage_panel::UsagePanel::new(&settings, generation.clone(), requests_tx, results_rx)?;
    assert!(!binding.is_visible(), "生产状态面板默认隐藏");
    binding.metrics(Some(18.), Some(42.));
    assert!(!panel.window().is_visible(), "状态面板默认隐藏");
    let weak = panel.as_weak();
    panel.on_dismiss(move || {
        if let Some(panel) = weak.upgrade() {
            panel.hide().unwrap();
        }
    });
    let production = binding.clone();
    pet.on_drag_request(move |_, _| {
        production.close();
    });
    let character = assets::firefly()?;
    pet.set_body(sprite(character.body));
    pet.set_left_hand(sprite(character.left_hand));
    pet.set_right_hand(sprite(character.right_hand));
    pet.set_keyboard(sprite(character.keyboard));
    pet.set_label_y(character.anchors.label_y);
    pet.set_left_tap_dy(character.anchors.left_tap_dy);
    pet.set_right_tap_dy(character.anchors.right_tap_dy);
    pet.set_headroom(ui_state::HEADROOM);
    pet.set_key_label("Ctrl + Alt + Shift + Win + Print Screen".into());
    pet.set_key_opacity(1.);
    panel.set_cpu_text("18%".into());
    panel.set_memory_text("42%".into());
    pet.set_bubble("Codex 这一轮回复结束了。".into());
    pet.set_simulation(true);
    pet.set_preview_background(slint::Color::from_rgb_u8(245, 248, 247));
    let initial = SettingsValues {
        scale_percent: 90.,
        volume: 70,
        quiet_fullscreen: true,
        battery: true,
        power: true,
        network: true,
        cpu: true,
        memory: true,
        codex: true,
        voice_index: -1,
        ..Default::default()
    };
    settings.set_draft(initial.clone());
    settings.set_applied(initial);
    settings.set_voice_status("没有可用音色，文字提醒仍正常显示。".into());
    settings.set_battery_status("当前电量：无电池或不可用".into());
    let wm = menu.as_weak();
    menu.on_dismiss(move || {
        if let Some(m) = wm.upgrade() {
            m.hide().unwrap();
        }
    });
    let context_count = Rc::new(Cell::new(0));
    let count = context_count.clone();
    let production = binding.clone();
    pet.on_context_request(move |_, _| {
        count.set(count.get() + 1);
        production.close();
    });
    let pat_count = Rc::new(Cell::new(0));
    let count = pat_count.clone();
    let production = binding.clone();
    let wp = pet.as_weak();
    pet.on_pat(move || {
        count.set(count.get() + 1);
        if let Some(pet) = wp.upgrade() {
            production.toggle(&pet);
        }
    });
    let output_error = Rc::new(RefCell::new(None::<String>));
    let error = output_error.clone();
    let mut step = 0;
    let mut capture = false;
    let timer = Timer::default();
    timer.start(TimerMode::Repeated, Duration::from_millis(220), move || {
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            if step < 36 {
                let factor = [0.6, 0.9, 1.6][step / 12];
                let face = (step % 12) / 2;
                let tap = step % 2 == 1;
                if !capture {
                    pet.set_factor(factor);
                    pet.set_face(sprite(character.faces[face]));
                    pet.set_left_tap(tap);
                    pet.set_right_tap(tap);
                    pet.set_breath(if tap { assets::BREATH_AMPLITUDE } else { -assets::BREATH_AMPLITUDE });
                    pet.window().set_size(LogicalSize::new(300. * factor, 350. * factor));
                    pet.show()?;
                } else {
                    snapshot(pet.window(), &directory.join(format!("pet-{}-face{face}-tap{tap}.bmp", (factor * 100.) as i32)))?;
                }
            } else if step < 46 {
                let page = (step - 36) % 5;
                if !capture {
                    settings.set_page(page as i32);
                    let size = if step < 41 { (860., 600.) } else { (680., 440.) };
                    settings.window().set_size(LogicalSize::new(size.0, size.1));
                    settings.show()?;
                } else {
                    snapshot(settings.window(), &directory.join(format!("settings-{}-page{page}.bmp", if step < 41 { "normal" } else { "small" })))?;
                }
            } else if step < 48 {
                settings.hide()?;
                if !capture {
                    menu.set_paused(step == 47);
                    menu.set_quiet(step == 47);
                    menu.set_passthrough(step == 47);
                    menu.show()?;
                    menu.invoke_focus_menu();
                } else {
                    snapshot(menu.window(), &directory.join(format!("menu-{}.bmp", step == 47)))?;
                }
            } else if step < 50 {
                if !capture {
                    menu.hide()?;
                    pet.set_factor(0.9);
                    pet.window().set_size(LogicalSize::new(270., 315.));
                    pet.set_key_label("Ctrl + C".into());
                    panel.set_cpu_text("未知".into());
                    panel.set_memory_text("未知".into());
                    pet.set_preview_background(if step == 48 { slint::Color::from_rgb_u8(245, 248, 247) } else { slint::Color::from_rgb_u8(39, 58, 54) });
                } else {
                    snapshot(pet.window(), &directory.join(format!("pet-detail-{}.bmp", step == 48)))?;
                }
            } else if step < 58 {
                if !capture {
                    let factor = [0.6, 0.9, 1.6][(step - 50).min(2)];
                    pet.set_factor(factor);
                    pet.window().set_size(LogicalSize::new(300. * factor, 350. * factor));
                    panel.set_cpu_text(if step == 56 { "未知" } else { "18%" }.into());
                    panel.set_memory_text(if step == 56 { "未知" } else { "42%" }.into());
                    panel.set_busy(step == 53);
                    panel.set_status(match step {
                        53 => "正在查询本机 Codex 账号额度…",
                        54 => "未找到 codex.exe，请将本机 Codex CLI 加入 PATH 后重试。",
                        _ => "模拟额度，仅用于布局预览",
                    }.into());
                    panel.set_updated(if step == 53 || step == 54 { "" } else { "更新于 2026-10-11 09:30（本地时间）" }.into());
                    let rows = if step == 53 || step == 54 { vec![] } else {
                        (0..if step == 55 { 8 } else { 2 }).map(|index| QuotaRow {
                            title: if step == 55 { format!("自定义超长额度名称与模型类别 {index} · 15 分钟额度") }
                                else if index == 0 { "Codex · 5 小时额度".into() } else { "Codex · 周额度".into() }.into(),
                            remaining: match step { 56 => "未知", 57 => "0%", _ => "75%" }.into(),
                            reset: if step == 56 { "重置时间未知" } else { "重置于 2026-10-16 09:30" }.into(),
                        }).collect()
                    };
                    panel.set_rows(Rc::new(slint::VecModel::from(rows)).into());
                    panel.show()?;
                } else {
                    snapshot(panel.window(), &directory.join(format!("status-panel-{step}.bmp")))?;
                }
            } else {
                panel.hide()?;
                menu.show()?;
                menu.invoke_focus_menu();
                menu.window().dispatch_event(slint::platform::WindowEvent::KeyPressed { text: slint::platform::Key::Escape.into() });
                assert!(!menu.window().is_visible(), "Esc 应关闭快捷菜单");
                settings.show()?;
                let mut draft = settings.get_draft();
                draft.scale_percent = 125.;
                draft.volume = 35;
                settings.set_draft(draft.clone());
                for page in 0..5 { settings.set_page(page); assert_eq!(settings.get_draft(), draft); }
                assert!(settings.get_save_visible(), "关于页也应能保存其他模块的修改");
                settings.show()?;
                assert_eq!(settings.get_draft(), draft);
                assert!(settings.get_dirty());
                settings.set_applied(draft);
                assert!(!settings.get_dirty());
                assert!(!settings.get_save_visible(), "关于页无修改时不显示保存按钮");
                let position = slint::LogicalPosition::new(120., 160.);
                pet.window().dispatch_event(slint::platform::WindowEvent::PointerPressed { position, button: slint::platform::PointerEventButton::Right });
                pet.window().dispatch_event(slint::platform::WindowEvent::PointerReleased { position, button: slint::platform::PointerEventButton::Right });
                assert_eq!(context_count.get(), 1);
                assert_eq!(pat_count.get(), 0, "右键不得触发抚摸或拖动");
                pet.window().dispatch_event(slint::platform::WindowEvent::PointerPressed { position, button: slint::platform::PointerEventButton::Left });
                pet.window().dispatch_event(slint::platform::WindowEvent::PointerReleased { position, button: slint::platform::PointerEventButton::Left });
                assert!(binding.is_visible(), "左键打开信息面板");
                requests_rx.try_recv().expect("展开触发一次额度查询");
                binding.refresh();
                assert!(requests_rx.try_recv().is_err(), "在途刷新必须合并");
                let request_id = generation.load(std::sync::atomic::Ordering::Acquire);
                results_tx.try_send(pet_desktop::platform::codex_usage::QueryResult {
                    id: request_id,
                    result: Ok(pet_desktop::usage::QuotaSnapshot { windows: vec![pet_desktop::usage::QuotaWindow {
                        title: "Codex · 5 小时额度".into(), remaining: Some(75.), resets_at: Some(1_900_000_000),
                    }] }),
                })?;
                binding.tick(&pet);
                assert!(!settings.get_quota_busy());
                assert!(settings.get_quota_status().contains("75%"), "面板与设置共享真实绑定结果");
                pet.window().dispatch_event(slint::platform::WindowEvent::PointerPressed { position, button: slint::platform::PointerEventButton::Left });
                pet.window().dispatch_event(slint::platform::WindowEvent::PointerReleased { position, button: slint::platform::PointerEventButton::Left });
                assert!(!binding.is_visible(), "再次左键收起信息面板");
                let pats = pat_count.get();
                binding.toggle(&pet);
                assert!(generation.load(std::sync::atomic::Ordering::Acquire) > 0);
                pet.window().dispatch_event(slint::platform::WindowEvent::PointerPressed { position, button: slint::platform::PointerEventButton::Left });
                let moved = slint::LogicalPosition::new(145., 180.);
                pet.window().dispatch_event(slint::platform::WindowEvent::PointerMoved { position: moved });
                pet.window().dispatch_event(slint::platform::WindowEvent::PointerReleased { position: moved, button: slint::platform::PointerEventButton::Left });
                assert!(!binding.is_visible(), "拖动收起信息面板");
                assert_eq!(generation.load(std::sync::atomic::Ordering::Acquire), 0, "拖动收起时取消查询");
                assert_eq!(pat_count.get(), pats, "拖动不得触发点击切换");
                println!("已保存 58 张原生渲染截图；模块草稿、关于页保存按钮、右键分流、Esc、左键开关和拖动分流检查通过。模拟检查不等于真实输入或多屏 DPI 验收。");
                slint::quit_event_loop()?;
            }
            if capture { step += 1; }
            capture = !capture;
            Ok(())
        })();
        if let Err(e) = result { *error.borrow_mut() = Some(e.to_string()); let _ = slint::quit_event_loop(); }
    });
    slint::run_event_loop()?;
    if let Some(error) = output_error.borrow_mut().take() {
        return Err(error.into());
    }
    Ok(())
}
