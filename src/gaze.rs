use std::time::Duration;

pub const SAMPLE_INTERVAL: Duration = Duration::from_millis(32);
const TURN_DELAY: Duration = Duration::from_millis(100);
const HALF_STAGE: f32 = 150.;
const DEADBAND: f32 = 16.;

#[derive(Clone, Copy)]
pub struct Geometry {
    pub origin: (i32, i32),
    pub dpi: f32,
    pub scale: f32,
    pub headroom: f32,
    pub breath: f32,
    pub eye_center: [f32; 2],
}

impl Geometry {
    fn stage_point(self, pointer: (i32, i32)) -> Option<[f32; 2]> {
        if ![
            self.dpi,
            self.scale,
            self.headroom,
            self.breath,
            self.eye_center[0],
            self.eye_center[1],
        ]
        .into_iter()
        .all(f32::is_finite)
            || self.dpi <= 0.
            || self.scale <= 0.
        {
            return None;
        }
        // 先提升到浮点再相减，兼容负坐标显示器并避免整数溢出。
        let factor = f64::from(self.dpi) * f64::from(self.scale);
        let x = ((f64::from(pointer.0) - f64::from(self.origin.0)) / factor) as f32;
        let y = ((f64::from(pointer.1) - f64::from(self.origin.1)) / factor) as f32 - self.headroom;
        (x.is_finite() && y.is_finite()).then_some([x, y])
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pose {
    pub facing_left: bool,
    // 偏移使用镜像前的素材坐标；数值限制在单位圆内。
    pub x: f32,
    pub y: f32,
}

#[derive(Default)]
pub struct Tracker {
    facing_left: bool,
    candidate: Option<(bool, Duration)>,
    last_tick: Option<Duration>,
    screen_offset: [f32; 2],
}

impl Tracker {
    pub fn update(
        &mut self,
        pointer: Option<(i32, i32)>,
        geometry: Geometry,
        dragging: bool,
        now: Duration,
    ) -> Pose {
        let point = pointer.and_then(|p| geometry.stage_point(p));
        let desired = point.and_then(|[x, _]| {
            if x < HALF_STAGE - DEADBAND {
                Some(true)
            } else if x > HALF_STAGE + DEADBAND {
                Some(false)
            } else {
                None
            }
        });
        if let Some(next) = desired.filter(|next| !dragging && *next != self.facing_left) {
            match self.candidate {
                Some((candidate, since)) if candidate == next => {
                    if now.saturating_sub(since) >= TURN_DELAY {
                        self.facing_left = next;
                        self.candidate = None;
                    }
                }
                _ => self.candidate = Some((next, now)),
            }
        } else {
            self.candidate = None;
        }
        let target = point.map_or([0., 0.], |[x, y]| {
            let center_x = if self.facing_left {
                300. - geometry.eye_center[0]
            } else {
                geometry.eye_center[0]
            };
            let dx = x - center_x;
            let dy = y - geometry.eye_center[1] - geometry.breath;
            let distance = dx.hypot(dy).max(120.);
            [dx / distance, dy / distance]
        });
        let elapsed = self
            .last_tick
            .map(|last| now.saturating_sub(last))
            .unwrap_or(SAMPLE_INTERVAL)
            .as_secs_f32()
            .min(0.25);
        self.last_tick = Some(now);
        let alpha = 1. - (-elapsed / 0.080).exp();
        for (offset, target) in self.screen_offset.iter_mut().zip(target) {
            *offset += (target - *offset) * alpha;
            if (target - *offset).abs() < 0.001 {
                *offset = target;
            }
        }
        Pose {
            facing_left: self.facing_left,
            x: self.screen_offset[0] * if self.facing_left { -1. } else { 1. },
            y: self.screen_offset[1],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geometry() -> Geometry {
        Geometry {
            origin: (0, 0),
            dpi: 1.,
            scale: 1.,
            headroom: 30.,
            breath: 0.,
            eye_center: [140., 170.],
        }
    }
    fn tick(t: &mut Tracker, x: i32, ms: u64, dragging: bool) -> Pose {
        t.update(
            Some((x, 200)),
            geometry(),
            dragging,
            Duration::from_millis(ms),
        )
    }

    #[test]
    fn turns_only_after_continuous_opposite_side_delay() {
        let mut t = Tracker::default();
        assert!(!tick(&mut t, 0, 0, false).facing_left);
        assert!(!tick(&mut t, 0, 99, false).facing_left);
        assert!(tick(&mut t, 0, 100, false).facing_left);
        for (ms, x) in [(132, 134), (164, 150), (196, 166), (228, 135)] {
            assert!(tick(&mut t, x, ms, false).facing_left);
        }
        assert!(tick(&mut t, 300, 260, false).facing_left);
        assert!(tick(&mut t, 150, 350, false).facing_left);
        assert!(tick(&mut t, 300, 400, false).facing_left);
        assert!(!tick(&mut t, 300, 500, false).facing_left);
    }

    #[test]
    fn dragging_resets_candidate_and_release_restarts_delay() {
        let mut t = Tracker::default();
        tick(&mut t, 0, 0, false);
        assert!(!tick(&mut t, 0, 200, true).facing_left);
        assert!(!tick(&mut t, 0, 300, false).facing_left);
        assert!(tick(&mut t, 0, 400, false).facing_left);
    }

    #[test]
    fn mirrored_gaze_keeps_screen_direction_and_unit_bounds() {
        let mut t = Tracker::default();
        let before = tick(&mut t, -10000, 0, false);
        let after = tick(&mut t, -10000, 100, false);
        assert!(before.x < 0. && after.x > 0.);
        for i in 0..100 {
            let p = t.update(
                Some((i32::MIN, i32::MAX)),
                geometry(),
                false,
                Duration::from_millis(132 + i * 32),
            );
            assert!(p.x.is_finite() && p.y.is_finite());
            assert!(p.x.hypot(p.y) <= 1.00001);
        }
    }

    #[test]
    fn cursor_loss_recenters_and_keeps_facing() {
        let mut t = Tracker::default();
        tick(&mut t, 0, 0, false);
        let previous = tick(&mut t, 0, 100, false);
        let first = t.update(None, geometry(), false, Duration::from_millis(132));
        assert!(first.facing_left && first.x < previous.x && first.x > 0.);
        let mut pose = first;
        for i in 1..30 {
            pose = t.update(None, geometry(), false, Duration::from_millis(132 + i * 32));
        }
        assert_eq!(
            pose,
            Pose {
                facing_left: true,
                x: 0.,
                y: 0.
            }
        );
        assert!(!tick(&mut t, 300, 1200, false).x.is_nan());
        assert!(!tick(&mut t, 300, 1300, false).facing_left);
    }

    #[test]
    fn coordinates_account_for_scale_dpi_headroom_and_negative_monitors() {
        for dpi in [1., 1.5, 2.] {
            for scale in [0.6, 0.9, 1.6] {
                let g = Geometry {
                    origin: (-1920, -200),
                    dpi,
                    scale,
                    ..geometry()
                };
                let pointer = (
                    -1920 + (220. * dpi * scale).round() as i32,
                    -200 + (100. * dpi * scale).round() as i32,
                );
                let point = g.stage_point(pointer).unwrap();
                assert!((point[0] - 220.).abs() < 1.);
                assert!((point[1] - 70.).abs() < 1.);
            }
        }
        for dpi in [0., -1., f32::NAN, f32::INFINITY] {
            assert!(Geometry { dpi, ..geometry() }.stage_point((0, 0)).is_none());
        }
    }

    #[test]
    fn fixed_center_prevents_feedback_when_face_changes_side() {
        let mut t = Tracker::default();
        tick(&mut t, 130, 0, false);
        assert!(tick(&mut t, 130, 100, false).facing_left);
        for i in 1..20 {
            assert!(tick(&mut t, 150, 100 + i * 32, false).facing_left);
        }
    }
}
