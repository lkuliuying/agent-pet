use pet_desktop::{
    assets::{self, Sprite},
    gaze::Pose,
    gaze_render::GazeRenderer,
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
#[path = "../src/app/expression_settings.rs"]
mod expression_settings;
#[path = "../src/app/resource_settings.rs"]
mod resource_settings;
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
    let (requests_tx, requests_rx) = std::sync::mpsc::sync_channel(1);
    let (results_tx, results_rx) = std::sync::mpsc::sync_channel(2);
    let binding = usage_panel::UsagePanel::new(&settings, requests_tx, results_rx)?;
    assert!(!binding.is_visible(), "生产状态面板默认隐藏");
    binding.metrics(Some(18.), Some(42.), Some(12.));
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
    let mut gaze_renderer =
        GazeRenderer::new(&character, pet.get_character_atlas(), pet.get_gaze_atlas())?;
    expression_settings::bind_preview(&settings, &character, &mut gaze_renderer)?;
    pet.set_face_image(gaze_renderer.render(0, Pose::default())?);
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
    panel.set_gpu_text("12%".into());
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
        expressions: expression_settings::values(&Default::default()),
        resources: resource_settings::values(&Default::default()),
        ..Default::default()
    };
    settings.set_draft(initial.clone());
    settings.set_applied(initial);
    settings.set_voice_status("没有可用音频输出设备，连接设备后可重新试听。".into());
    settings.set_battery_status("当前电量：无电池或不可用".into());
    let session_rows: Vec<_> = (0..12)
        .map(|index| SessionRow {
            title: format!("并行项目 {index} · 很长的中文会话名称用于验证省略和滚动行为").into(),
            project: "F:\\示例工作区\\中文 空格目录\\项目".into(),
            status: if index < 2 { "待回答" } else { "运行中" }.into(),
            attention: index < 2,
        })
        .collect();
    let sessions: slint::ModelRc<SessionRow> = Rc::new(slint::VecModel::from(session_rows)).into();
    panel.set_sessions(sessions.clone());
    settings.set_sessions(sessions);
    panel.set_session_status("模拟列表，仅用于布局验证".into());
    settings.set_session_status("模拟列表，仅用于布局验证".into());
    settings.set_integration_status(
        "需要信任：请在 Codex 中审阅并信任流萤 hooks，然后重新检测。".into(),
    );
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
    let gaze_only = std::env::args().nth(2).as_deref() == Some("--gaze");
    let monitoring_only = std::env::args().nth(2).as_deref() == Some("--monitoring");
    if monitoring_only {
        pet.show()?;
    }
    let mut step = if gaze_only {
        58
    } else if monitoring_only {
        78
    } else {
        0
    };
    let mut capture = false;
    let mut scroll_pending = false;
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
                    pet.set_face_image(gaze_renderer.render(face, Pose::default())?);
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
                    panel.set_gpu_text(if step == 56 { "未知" } else { "12%" }.into());
                    panel.set_quota_text(match step { 53 => "查询中", 54 => "未连接", 56 => "未知", 57 => "0%", _ => "75%" }.into());
                    panel.set_details_open(step >= 53);
                    panel.set_busy(step == 53);
                    panel.set_status(match step {
                        53 => "正在查询本机 Codex 账号额度…",
                        54 => pet_desktop::usage::UsageError::MissingCodex.message(),
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
                    let logical = panel.window().size().to_logical(panel.window().scale_factor());
                    assert!((logical.width - 352.).abs() < 1., "状态条宽度应保持紧凑");
                    if step < 53 { assert!((logical.height - 32.).abs() < 1., "默认只显示一行小框"); }
                    snapshot(panel.window(), &directory.join(format!("status-panel-{step}.bmp")))?;
                }
            } else if !gaze_only && step < 62 {
                if !capture {
                    panel.set_details_open(false);
                    panel.set_cpu_text(if step == 61 { "未知" } else { "100%" }.into());
                    panel.set_memory_text(if step == 61 { "未知" } else { "100%" }.into());
                    panel.set_gpu_text(if step == 61 { "未知" } else { "100%" }.into());
                    panel.set_quota_text(match step { 58 => "未连接", 59 => "未登录", 60 => "连接失败", _ => "未知" }.into());
                    panel.show()?;
                } else {
                    snapshot(panel.window(), &directory.join(format!("compact-status-{step}.bmp")))?;
                }
            } else if !gaze_only && step < 78 {
                let index = step - 62;
                let small = if index < 12 { index >= 6 } else { index % 2 == 1 };
                if !capture {
                    panel.hide()?;
                    let size = if small { (680., 440.) } else { (860., 600.) };
                    settings.window().set_size(LogicalSize::new(size.0, size.1));
                    settings.set_page(0);
                    settings.set_page(if index < 14 { 1 } else { 5 });
                    settings.set_preview_index((index % 6) as i32);
                    settings.show()?;
                } else if (index == 12 || index == 13) && !scroll_pending {
                    // 缩放窗口后先等待布局完成，再滚动，避免使用上一帧的可滚动范围。
                    settings.window().dispatch_event(slint::platform::WindowEvent::PointerScrolled {
                        position: slint::LogicalPosition::new(500., 250.),
                        delta_x: 0.,
                        delta_y: -4000.,
                    });
                    scroll_pending = true;
                    capture = false;
                } else {
                    assert!(settings.get_save_visible() || settings.get_page() == 5);
                    snapshot(settings.window(), &directory.join(format!("expressions-{index}-{}.bmp", if small { "small" } else { "normal" })))?;
                    scroll_pending = false;
                }
            } else if !gaze_only && step < 86 {
                let index = step - 78;
                let page = if index < 4 { 2 } else { 4 };
                let small = index % 4 >= 2;
                let scrolled = index % 2 == 1;
                if !capture {
                    panel.hide()?;
                    let size = if small { (680., 440.) } else { (860., 600.) };
                    settings.window().set_size(LogicalSize::new(size.0, size.1));
                    settings.set_page(0);
                    settings.set_page(page);
                    settings.show()?;
                } else if !scroll_pending {
                    settings.window().dispatch_event(slint::platform::WindowEvent::PointerScrolled {
                        position: slint::LogicalPosition::new(500., 250.), delta_x: 0., delta_y: if scrolled { -6000. } else { 6000. },
                    });
                    scroll_pending = true;
                    capture = false;
                } else {
                    assert!(settings.get_save_visible(), "滚动不能隐藏统一保存栏");
                    snapshot(settings.window(), &directory.join(format!("monitoring-page{page}-small{small}-bottom{scrolled}.bmp")))?;
                    scroll_pending = false;
                }
            } else if !gaze_only && step == 86 {
                if !capture {
                    settings.hide()?;
                    panel.set_details_open(true);
                    panel.show()?;
                } else if !scroll_pending {
                    panel.window().dispatch_event(slint::platform::WindowEvent::PointerScrolled {
                        position: slint::LogicalPosition::new(180., 240.), delta_x: 0., delta_y: -6000.,
                    });
                    scroll_pending = true;
                    capture = false;
                } else {
                    snapshot(panel.window(), &directory.join("monitoring-panel-bottom.bmp"))?;
                    panel.set_details_open(false);
                    scroll_pending = false;
                }
            } else if gaze_only && step < 58 + 324 {
                let index = step - 58;
                let direction = index % 9;
                let face = (index / 9) % 6;
                let left = (index / 54) % 2 == 1;
                let factor = [0.6, 0.9, 1.6][index / 108];
                let x = (direction % 3) as f32 - 1.;
                let y = (direction / 3) as f32 - 1.;
                if !capture {
                    pet.set_factor(factor);
                    pet.set_facing_left(left);
                    pet.set_face(sprite(character.faces[face]));
                    pet.set_face_image(gaze_renderer.render(face, Pose { facing_left: left, x: if left { -x } else { x }, y })?);
                    pet.set_left_tap(direction % 2 == 0);
                    pet.set_right_tap(direction % 2 == 1);
                    assert_eq!(pet.get_near_tap(), if left { pet.get_right_tap() } else { pet.get_left_tap() });
                    assert_eq!(pet.get_far_tap(), if left { pet.get_left_tap() } else { pet.get_right_tap() });
                    pet.set_breath(assets::BREATH_AMPLITUDE);
                    pet.set_key_label(if direction == 4 { "Ctrl + Alt + Shift + Win + Print Screen" } else { "Ctrl + C" }.into());
                    pet.set_preview_background(if face % 2 == 0 { slint::Color::from_rgb_u8(245, 248, 247) } else { slint::Color::from_rgb_u8(39, 58, 54) });
                    pet.window().set_size(LogicalSize::new(300. * factor, 350. * factor));
                    pet.show()?;
                } else {
                    snapshot(pet.window(), &directory.join(format!("gaze-{}-left{left}-face{face}-dir{direction}.bmp", (factor * 100.) as i32)))?;
                }
            } else {
                if !gaze_only {
                    let position = slint::LogicalPosition::new(280., 16.);
                    for expected in [true, false] {
                        panel.window().dispatch_event(slint::platform::WindowEvent::PointerPressed { position, button: slint::platform::PointerEventButton::Left });
                        panel.window().dispatch_event(slint::platform::WindowEvent::PointerReleased { position, button: slint::platform::PointerEventButton::Left });
                        assert_eq!(panel.get_details_open(), expected, "Codex 小框应切换额度详情");
                    }
                }
                panel.hide()?;
                menu.show()?;
                menu.invoke_focus_menu();
                menu.window().dispatch_event(slint::platform::WindowEvent::KeyPressed { text: slint::platform::Key::Escape.into() });
                assert!(!menu.window().is_visible(), "Esc 应关闭快捷菜单");
                settings.show()?;
                let mut draft = settings.get_draft();
                draft.scale_percent = 125.;
                draft.volume = 35;
                draft.expressions.click = 0;
                draft.expressions.input_duration_ms = 2300;
                draft.resources.cpu_rule.threshold = 75;
                draft.resources.gpu_rule.minutes = 1;
                assert_eq!(resource_settings::config(draft.resources.clone())?.cpu.threshold, 75);
                settings.set_draft(draft.clone());
                for page in 0..6 { settings.set_page(page); assert_eq!(settings.get_draft(), draft); }
                assert!(settings.get_save_visible(), "关于页也应能保存其他模块的修改");
                settings.show()?;
                assert_eq!(settings.get_draft(), draft);
                assert!(settings.get_dirty());
                let face_before_preview = pet.get_face();
                let applied_before_preview = settings.get_applied();
                for index in 0..6 {
                    settings.set_preview_index(index);
                    assert_eq!(settings.get_draft(), draft, "页内预览不能修改草稿");
                    assert_eq!(settings.get_applied(), applied_before_preview, "页内预览不能应用设置");
                    assert_eq!(pet.get_face(), face_before_preview, "页内预览不能改变桌面角色");
                }
                settings.invoke_reset_expressions();
                let restored = settings.get_draft();
                assert_eq!(restored.scale_percent, 125.);
                assert_eq!(restored.volume, 35);
                assert_eq!(expression_settings::config(restored.expressions.clone())?, Default::default());
                assert_eq!(settings.get_applied(), applied_before_preview, "恢复默认只修改草稿");
                assert!(settings.get_dirty(), "其他页面的修改必须保留");
                settings.set_applied(restored);
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
                assert!(requests_rx.try_recv().is_err(), "打开面板不额外触发后台查询");
                binding.refresh();
                requests_rx.try_recv().expect("手动触发一次额度查询");
                binding.refresh();
                assert!(requests_rx.try_recv().is_err(), "在途刷新必须合并");
                results_tx.try_send(pet_desktop::platform::codex_usage::UsageUpdate::Complete(
                    Ok(pet_desktop::usage::QuotaSnapshot { windows: vec![pet_desktop::usage::QuotaWindow {
                        bucket_id: "codex".into(),
                        title: "Codex · 5 小时额度".into(), remaining: Some(75.), resets_at: Some(1_900_000_000),
                    }] }),
                ))?;
                binding.tick(&pet);
                assert!(!settings.get_quota_busy());
                assert!(settings.get_quota_status().contains("75%"), "面板与设置共享真实绑定结果");
                pet.window().dispatch_event(slint::platform::WindowEvent::PointerPressed { position, button: slint::platform::PointerEventButton::Left });
                pet.window().dispatch_event(slint::platform::WindowEvent::PointerReleased { position, button: slint::platform::PointerEventButton::Left });
                assert!(!binding.is_visible(), "再次左键收起信息面板");
                let pats = pat_count.get();
                binding.toggle(&pet);
                binding.refresh();
                requests_rx.try_recv().expect("手动刷新可再次发出");
                pet.window().dispatch_event(slint::platform::WindowEvent::PointerPressed { position, button: slint::platform::PointerEventButton::Left });
                let moved = slint::LogicalPosition::new(145., 180.);
                pet.window().dispatch_event(slint::platform::WindowEvent::PointerMoved { position: moved });
                assert!(pet.get_drag_active(), "拖动期间必须锁定转向");
                pet.window().dispatch_event(slint::platform::WindowEvent::PointerReleased { position: moved, button: slint::platform::PointerEventButton::Left });
                assert!(!pet.get_drag_active(), "释放后必须恢复转向，不能依赖残留 dragging 标记");
                assert!(!binding.is_visible(), "拖动收起信息面板");
                assert!(settings.get_quota_busy(), "拖动关闭面板不取消后台查询");
                results_tx.try_send(pet_desktop::platform::codex_usage::UsageUpdate::Complete(Ok(pet_desktop::usage::QuotaSnapshot { windows: vec![pet_desktop::usage::QuotaWindow {
                    bucket_id: "codex".into(), title: "Codex".into(), remaining: Some(64.), resets_at: None,
                }] })))?;
                binding.tick(&pet);
                assert!(settings.get_quota_status().contains("64%"), "关闭面板后仍更新共享额度");
                binding.sessions(vec![], "当前无运行会话".into());
                assert_eq!(settings.get_session_status(), "当前无运行会话");
                assert_eq!(pat_count.get(), pats, "拖动不得触发点击切换");
                println!("已保存 {} 张原生渲染截图；表情预览隔离、六模块草稿、高占用规则、会话列表滚动、面板关闭后后台额度更新、右键分流、Esc、左键开关和拖动分流检查通过。模拟检查不等于真实输入或多屏 DPI 验收。", if gaze_only { 324 } else if monitoring_only { 9 } else { 87 });
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
