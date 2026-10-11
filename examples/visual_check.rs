use pet_desktop::assets::{self, Sprite};
use slint::{ComponentHandle, PhysicalPosition, Timer, TimerMode};
use std::time::{Duration, Instant};

slint::include_modules!();

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let label = args.first().map(String::as_str).unwrap_or("Ctrl + C");
    let face = args
        .get(1)
        .map(|n| n.parse::<usize>())
        .transpose()?
        .unwrap_or(0);
    let factor = args
        .get(2)
        .map(|n| n.parse::<f32>())
        .transpose()?
        .unwrap_or(0.9);
    let mut animate = false;
    let mut background = None;
    for arg in args.iter().skip(3) {
        match arg.as_str() {
            "--animate" if !animate => animate = true,
            "--background=light" if background.is_none() => background = Some(true),
            "--background=dark" if background.is_none() => background = Some(false),
            _ => {
                return Err(
                    "参数为：标签 表情索引 缩放 [--animate] [--background=light|dark]".into(),
                );
            }
        }
    }
    if face >= 6 || !factor.is_finite() || !(0.6..=1.6).contains(&factor) {
        return Err("表情索引须为 0–5，缩放须为 0.6–1.6".into());
    }
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("femtovg-wgpu".into())
        .select()?;
    let c = assets::firefly()?;
    let pet = PetWindow::new()?;
    let panel = StatusPanelWindow::new()?;
    panel.set_cpu_text("18%".into());
    panel.set_memory_text("42%".into());
    panel.set_status("模拟额度，仅用于布局预览".into());
    panel.set_updated("更新于 2026-10-11 09:30（模拟）".into());
    panel.set_rows(
        std::rc::Rc::new(slint::VecModel::from(vec![
            QuotaRow {
                title: "Codex · 5 小时额度".into(),
                remaining: "75%".into(),
                reset: "重置于 2026-10-11 14:30".into(),
            },
            QuotaRow {
                title: "Codex · 周额度".into(),
                remaining: "58%".into(),
                reset: "重置于 2026-10-16 09:30".into(),
            },
        ]))
        .into(),
    );
    let weak = panel.as_weak();
    panel.on_dismiss(move || {
        if let Some(panel) = weak.upgrade() {
            let _ = panel.hide();
        }
    });
    let weak = panel.as_weak();
    pet.on_pat(move || {
        if let Some(panel) = weak.upgrade() {
            if panel.window().is_visible() {
                let _ = panel.hide();
            } else {
                panel.window().set_position(PhysicalPosition::new(880, 100));
                let _ = panel.show();
            }
        }
    });
    if let Some(light) = background {
        // 底色只用于开发预览，避免透明截图包含后方其他窗口的内容。
        let color = if light { (245, 248, 247) } else { (39, 58, 54) };
        pet.set_preview_background(slint::Color::from_rgb_u8(color.0, color.1, color.2));
    }
    pet.set_factor(factor);
    pet.set_body(sprite(c.body));
    pet.set_face(sprite(c.faces[face]));
    pet.set_left_hand(sprite(c.left_hand));
    pet.set_right_hand(sprite(c.right_hand));
    pet.set_keyboard(sprite(c.keyboard));
    pet.set_label_y(c.anchors.label_y);
    pet.set_left_tap_dy(c.anchors.left_tap_dy);
    pet.set_right_tap_dy(c.anchors.right_tap_dy);
    pet.set_key_label(label.into());
    pet.set_key_opacity(1.);
    pet.set_simulation(true);
    pet.set_bubble("Codex 这一轮回复结束了。".into());
    pet.show()?;
    pet.window().set_position(PhysicalPosition::new(600, 300));
    // 演示仅驱动展示属性，不启动输入监听、托盘、管道或配置写入。
    let animation = Timer::default();
    if animate {
        let weak = pet.as_weak();
        let started = Instant::now();
        animation.start(TimerMode::Repeated, Duration::from_millis(16), move || {
            let Some(pet) = weak.upgrade() else { return };
            let elapsed = started.elapsed();
            let step = ((elapsed.as_secs() / 2) as usize + face) % 6;
            let labels = [
                "A",
                "Space",
                "Ctrl + C",
                "Num Enter",
                "Ctrl + Alt + Shift + Win + Print Screen",
                "",
            ];
            let pulse = elapsed.as_millis() % 1600;
            pet.set_face(sprite(c.faces[step]));
            pet.set_key_label(labels[step].into());
            pet.set_left_tap(pulse < 110 || (800..910).contains(&pulse));
            pet.set_right_tap((400..510).contains(&pulse) || (800..910).contains(&pulse));
            let phase = (elapsed.as_millis() / 80) as f32 * 0.08;
            pet.set_breath((phase * 1.4).sin() * assets::BREATH_AMPLITUDE);
            pet.set_bubble(if step.is_multiple_of(2) {
                "Codex 这一轮回复结束了。".into()
            } else {
                "".into()
            });
        });
    }
    let timer = Timer::default();
    timer.start(TimerMode::SingleShot, Duration::from_secs(180), || {
        let _ = slint::quit_event_loop();
    });
    slint::run_event_loop()?;
    Ok(())
}
