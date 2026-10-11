use crate::config::{self, Config};
use std::{io, path::Path};

pub const PET_WIDTH: f32 = 300.;
pub const PET_HEIGHT: f32 = 320.;
pub const HEADROOM: f32 = 30.;

#[derive(Default)]
pub struct PanelDismissal {
    armed: bool,
    escape_down: bool,
}

impl PanelDismissal {
    pub fn opened(escape_down: bool) -> Self {
        Self {
            armed: false,
            escape_down,
        }
    }

    pub fn observe(
        &mut self,
        pressed: bool,
        inside_panel: bool,
        inside_pet: bool,
        escape_down: bool,
    ) -> bool {
        let escape = escape_down && !self.escape_down;
        self.escape_down = escape_down;
        if !pressed {
            self.armed = true;
        }
        // 角色上的按下交给松开后的切换回调，避免先关闭再被同一次点击打开。
        escape || (self.armed && pressed && !inside_panel && !inside_pet)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum QuickAction {
    TogglePause,
    ToggleQuiet,
    TogglePassthrough,
    ResetPosition((i32, i32)),
    Restore,
}

impl QuickAction {
    pub fn candidate(self, current: &Config) -> Config {
        let mut candidate = current.clone();
        match self {
            Self::TogglePause => candidate.paused = !candidate.paused,
            Self::ToggleQuiet => candidate.quiet = !candidate.quiet,
            Self::TogglePassthrough => candidate.passthrough = !candidate.passthrough,
            Self::ResetPosition(position) => candidate.position = Some(position),
            Self::Restore => candidate.passthrough = false,
        }
        candidate
    }

    pub fn merge_into_draft(self, draft: &mut Config, applied: &Config) {
        // 快捷操作只回填自己修改的字段，其他页面尚未保存的草稿必须保留。
        match self {
            Self::TogglePause => draft.paused = applied.paused,
            Self::ToggleQuiet => draft.quiet = applied.quiet,
            Self::TogglePassthrough | Self::Restore => draft.passthrough = applied.passthrough,
            Self::ResetPosition(_) => draft.position = applied.position,
        }
    }
}

pub fn commit(current: &mut Config, candidate: Config, directory: &Path) -> io::Result<()> {
    config::save(directory, &candidate)?;
    *current = candidate;
    Ok(())
}

pub fn metric_text(value: Option<f64>) -> String {
    value
        .filter(|v| v.is_finite() && (0.0..=100.0).contains(v))
        .map(|v| format!("{v:.0}%"))
        .unwrap_or_else(|| "未知".into())
}

pub fn headroom_pixels(scale: f32, dpi: f64) -> i32 {
    (HEADROOM as f64 * scale as f64 * dpi).round() as i32
}

pub fn window_position(anchor: (i32, i32), headroom: i32) -> (i32, i32) {
    (anchor.0, anchor.1.saturating_sub(headroom))
}

pub fn saved_position(window: (i32, i32), headroom: i32) -> (i32, i32) {
    // 配置 v1 的位置仍指向原 300×320 舞台，旧版本读取时也不会发生累积偏移。
    (window.0, window.1.saturating_add(headroom))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_panel_dismissal_does_not_turn_a_toggle_into_a_reopen() {
        let mut state = PanelDismissal::opened(false);
        assert!(!state.observe(true, false, false, false));
        assert!(!state.observe(false, false, true, false));
        assert!(!state.observe(true, false, true, false));
        assert!(!state.observe(true, true, false, false));
        assert!(state.observe(true, false, false, false));
        assert!(state.observe(false, false, false, true));
        let mut held_escape = PanelDismissal::opened(true);
        assert!(!held_escape.observe(false, false, false, true));
        assert!(!held_escape.observe(false, false, false, false));
        assert!(held_escape.observe(false, false, false, true));
    }

    #[test]
    fn quick_actions_preserve_unrelated_drafts_and_legacy_preferences() {
        let original = Config {
            tkl: true,
            position: Some((-900, 240)),
            ..Config::default()
        };
        for action in [
            QuickAction::TogglePause,
            QuickAction::ToggleQuiet,
            QuickAction::TogglePassthrough,
            QuickAction::Restore,
            QuickAction::ResetPosition((-200, 50)),
        ] {
            let mut draft = original.clone();
            draft.scale = 1.25;
            draft.volume = 35;
            draft.voice_id = "unsaved-voice".into();
            draft.network = false;
            let applied = action.candidate(&original);
            action.merge_into_draft(&mut draft, &applied);
            assert_eq!(draft.scale, 1.25);
            assert_eq!(draft.volume, 35);
            assert_eq!(draft.voice_id, "unsaved-voice");
            assert!(!draft.network);
            assert!(applied.tkl);
            assert_eq!(applied.volume, original.volume);
            let toggled_back = action.candidate(&applied);
            match action {
                QuickAction::TogglePause => {
                    assert!(draft.paused);
                    assert!(!toggled_back.paused);
                }
                QuickAction::ToggleQuiet => {
                    assert!(draft.quiet);
                    assert!(!toggled_back.quiet);
                }
                QuickAction::TogglePassthrough => {
                    assert!(draft.passthrough);
                    assert!(!toggled_back.passthrough);
                }
                QuickAction::Restore => assert!(!draft.passthrough),
                QuickAction::ResetPosition(p) => assert_eq!(draft.position, Some(p)),
            }
        }
    }

    #[test]
    fn failed_save_does_not_change_running_configuration() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/ui-state-tests");
        std::fs::create_dir_all(&dir).unwrap();
        let blocker = dir.join(format!("not-a-directory-{}", std::process::id()));
        std::fs::write(&blocker, b"preserve").unwrap();
        let mut current = Config::default();
        let before = current.clone();
        let candidate = QuickAction::ToggleQuiet.candidate(&current);
        assert!(commit(&mut current, candidate, &blocker).is_err());
        assert_eq!(current, before);
        assert_eq!(std::fs::read(&blocker).unwrap(), b"preserve");
        std::fs::remove_file(blocker).unwrap();
    }

    #[test]
    fn legacy_positions_roundtrip_across_pet_sizes_and_dpi() {
        for scale in [0.6, 0.9, 1.6] {
            for dpi in [1., 1.5, 2.] {
                let offset = headroom_pixels(scale, dpi);
                for anchor in [(-1400, -200), (0, 0), (2000, 900)] {
                    let position = window_position(anchor, offset);
                    assert_eq!(saved_position(position, offset), anchor);
                    assert_eq!(position.1 + offset, anchor.1);
                }
            }
        }
    }

    #[test]
    fn unknown_and_invalid_metrics_are_not_shown_as_zero() {
        assert_eq!(metric_text(Some(18.2)), "18%");
        assert_eq!(metric_text(Some(99.7)), "100%");
        assert_eq!(metric_text(Some(0.)), "0%");
        for value in [
            None,
            Some(f64::NAN),
            Some(f64::INFINITY),
            Some(-1.),
            Some(101.),
        ] {
            assert_eq!(metric_text(value), "未知");
        }
    }
}
