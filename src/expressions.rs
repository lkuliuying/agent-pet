use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpressionId {
    Idle,
    Blink,
    Focused,
    Happy,
    Sleepy,
    Surprised,
}

impl ExpressionId {
    pub const ALL: [Self; 6] = [
        Self::Idle,
        Self::Blink,
        Self::Focused,
        Self::Happy,
        Self::Sleepy,
        Self::Surprised,
    ];

    pub const fn index(self) -> usize {
        match self {
            Self::Idle => 0,
            Self::Blink => 1,
            Self::Focused => 2,
            Self::Happy => 3,
            Self::Sleepy => 4,
            Self::Surprised => 5,
        }
    }

    pub fn from_index(index: i32) -> Result<Self, String> {
        usize::try_from(index)
            .ok()
            .and_then(|i| Self::ALL.get(i).copied())
            .ok_or_else(|| "请选择有效的角色表情".into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ExpressionConfig {
    pub idle: ExpressionId,
    pub blink: Option<ExpressionId>,
    pub input: Option<ExpressionId>,
    pub click: Option<ExpressionId>,
    pub sleepy: Option<ExpressionId>,
    pub drag: Option<ExpressionId>,
    pub notice: Option<ExpressionId>,
    pub blink_interval_ms: u32,
    pub blink_duration_ms: u32,
    pub input_duration_ms: u32,
    pub click_duration_ms: u32,
    pub sleepy_after_ms: u32,
    pub drag_duration_ms: u32,
}

impl Default for ExpressionConfig {
    fn default() -> Self {
        Self {
            idle: ExpressionId::Idle,
            blink: Some(ExpressionId::Blink),
            input: Some(ExpressionId::Focused),
            click: Some(ExpressionId::Happy),
            sleepy: Some(ExpressionId::Sleepy),
            drag: Some(ExpressionId::Surprised),
            notice: Some(ExpressionId::Surprised),
            blink_interval_ms: 4800,
            blink_duration_ms: 150,
            input_duration_ms: 1200,
            click_duration_ms: 2000,
            sleepy_after_ms: 90_000,
            drag_duration_ms: 2000,
        }
    }
}

impl ExpressionConfig {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }

    pub fn validate(&self) -> Result<(), String> {
        for (label, value) in [
            ("输入表情", self.input_duration_ms),
            ("点击表情", self.click_duration_ms),
            ("拖动表情", self.drag_duration_ms),
        ] {
            if !(100..=10_000).contains(&value) {
                return Err(format!("{label}持续时间必须在 0.1 到 10 秒之间"));
            }
        }
        if !(15_000..=3_600_000).contains(&self.sleepy_after_ms) {
            return Err("困倦等待时间必须在 15 到 3600 秒之间".into());
        }
        if !(1000..=30_000).contains(&self.blink_interval_ms) {
            return Err("眨眼间隔必须在 1 到 30 秒之间".into());
        }
        if !(50..=1000).contains(&self.blink_duration_ms) {
            return Err("眨眼持续时间必须在 50 到 1000 毫秒之间".into());
        }
        if self.blink_duration_ms >= self.blink_interval_ms {
            return Err("眨眼持续时间必须小于眨眼间隔".into());
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct ExpressionState {
    clicked_at: Option<Instant>,
    dragged_at: Option<Instant>,
}

pub struct ExpressionContext {
    pub elapsed: Duration,
    pub input_idle: Option<Duration>,
    pub notice_active: bool,
}

impl ExpressionState {
    pub fn click(&mut self, now: Instant) {
        self.clicked_at = Some(now);
    }

    pub fn drag(&mut self, now: Instant) {
        self.dragged_at = Some(now);
    }

    pub fn clear_interactions(&mut self) {
        self.clicked_at = None;
        self.dragged_at = None;
    }

    pub fn select(
        &self,
        config: &ExpressionConfig,
        now: Instant,
        context: ExpressionContext,
    ) -> ExpressionId {
        let active = |at: Option<Instant>, duration_ms| {
            at.is_some_and(|at| {
                now.saturating_duration_since(at) < Duration::from_millis(duration_ms as u64)
            })
        };
        // 按场景确定优先级，再取映射表情，不能用最终图片编号判断是否待机。
        if let Some(face) = config.drag
            && active(self.dragged_at, config.drag_duration_ms)
        {
            return face;
        }
        if let Some(face) = config.click
            && active(self.clicked_at, config.click_duration_ms)
        {
            return face;
        }
        if let Some(face) = config.notice
            && context.notice_active
        {
            return face;
        }
        if let Some(idle) = context.input_idle {
            if let Some(face) = config.input
                && idle < Duration::from_millis(config.input_duration_ms as u64)
            {
                return face;
            }
            if let Some(face) = config.sleepy
                && idle > Duration::from_millis(config.sleepy_after_ms as u64)
            {
                return face;
            }
        }
        if let Some(face) = config.blink
            && context
                .elapsed
                .as_millis()
                .checked_rem(config.blink_interval_ms as u128)
                .is_some_and(|phase| phase < config.blink_duration_ms as u128)
        {
            return face;
        }
        config.idle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(elapsed_ms: u64, input_idle_ms: u64, notice: bool) -> ExpressionContext {
        ExpressionContext {
            elapsed: Duration::from_millis(elapsed_ms),
            input_idle: Some(Duration::from_millis(input_idle_ms)),
            notice_active: notice,
        }
    }

    #[test]
    fn defaults_keep_startup_and_exact_time_boundaries() {
        let config = ExpressionConfig::default();
        let state = ExpressionState::default();
        for (ms, expected) in [
            (0, ExpressionId::Focused),
            (1199, ExpressionId::Focused),
            (1200, ExpressionId::Idle),
            (4799, ExpressionId::Idle),
            (4800, ExpressionId::Blink),
            (4949, ExpressionId::Blink),
            (4950, ExpressionId::Idle),
            (90_000, ExpressionId::Idle),
            (90_001, ExpressionId::Sleepy),
        ] {
            assert_eq!(
                state.select(&config, Instant::now(), context(ms, ms, false)),
                expected,
                "{ms}"
            );
        }
    }

    #[test]
    fn priority_disabled_scenes_and_repeated_interactions() {
        let t = Instant::now();
        let mut state = ExpressionState::default();
        let mut config = ExpressionConfig {
            drag: Some(ExpressionId::Sleepy),
            notice: Some(ExpressionId::Blink),
            ..ExpressionConfig::default()
        };
        state.click(t);
        state.drag(t);
        assert_eq!(
            state.select(&config, t, context(5000, 0, true)),
            ExpressionId::Sleepy
        );
        config.drag = None;
        assert_eq!(
            state.select(&config, t, context(5000, 0, true)),
            ExpressionId::Happy
        );
        config.click = None;
        assert_eq!(
            state.select(&config, t, context(5000, 0, true)),
            ExpressionId::Blink
        );
        config.notice = None;
        assert_eq!(
            state.select(&config, t, context(5000, 0, true)),
            ExpressionId::Focused
        );
        config.input = None;
        assert_eq!(
            state.select(&config, t, context(5000, 0, true)),
            ExpressionId::Idle
        );
        config.sleepy = None;
        config.blink = None;
        assert_eq!(
            state.select(&config, t, context(4800, 100_000, true)),
            ExpressionId::Idle
        );

        let config = ExpressionConfig::default();
        state.clear_interactions();
        state.click(t);
        state.click(t + Duration::from_millis(1500));
        assert_eq!(
            state.select(
                &config,
                t + Duration::from_millis(3499),
                context(5000, 5000, false)
            ),
            ExpressionId::Happy
        );
        assert_eq!(
            state.select(
                &config,
                t + Duration::from_millis(3500),
                context(5000, 5000, false)
            ),
            ExpressionId::Idle
        );
        state.drag(t + Duration::from_millis(4000));
        state.drag(t + Duration::from_millis(5500));
        assert_eq!(
            state.select(
                &config,
                t + Duration::from_millis(7499),
                context(5000, 5000, false)
            ),
            ExpressionId::Surprised
        );
        assert_eq!(
            state.select(
                &config,
                t + Duration::from_millis(7500),
                context(5000, 5000, false)
            ),
            ExpressionId::Idle
        );
    }

    #[test]
    fn mapping_to_idle_does_not_enable_blinks_in_an_active_scene() {
        let config = ExpressionConfig {
            idle: ExpressionId::Happy,
            input: Some(ExpressionId::Idle),
            click: Some(ExpressionId::Idle),
            ..ExpressionConfig::default()
        };
        let t = Instant::now();
        let mut state = ExpressionState::default();
        assert_eq!(
            state.select(&config, t, context(4800, 0, false)),
            ExpressionId::Idle
        );
        state.click(t);
        assert_eq!(
            state.select(&config, t, context(4800, 5000, false)),
            ExpressionId::Idle
        );
        state.clear_interactions();
        assert_eq!(
            state.select(&config, t, context(4800, 5000, false)),
            ExpressionId::Blink
        );
        assert_eq!(
            state.select(&config, t, context(5000, 5000, false)),
            ExpressionId::Happy
        );
    }

    #[test]
    fn custom_durations_control_boundaries() {
        let config = ExpressionConfig {
            input_duration_ms: 100,
            click_duration_ms: 500,
            drag_duration_ms: 300,
            sleepy_after_ms: 15_000,
            blink_interval_ms: 1000,
            blink_duration_ms: 50,
            ..ExpressionConfig::default()
        };
        assert!(config.validate().is_ok());
        let t = Instant::now();
        let mut state = ExpressionState::default();
        for (ms, expected) in [
            (99, ExpressionId::Focused),
            (100, ExpressionId::Idle),
            (1000, ExpressionId::Blink),
            (1049, ExpressionId::Blink),
            (1050, ExpressionId::Idle),
            (15_001, ExpressionId::Sleepy),
        ] {
            assert_eq!(state.select(&config, t, context(ms, ms, false)), expected);
        }
        state.click(t);
        state.drag(t);
        assert_eq!(
            state.select(
                &config,
                t + Duration::from_millis(299),
                context(600, 600, false)
            ),
            ExpressionId::Surprised
        );
        assert_eq!(
            state.select(
                &config,
                t + Duration::from_millis(300),
                context(600, 600, false)
            ),
            ExpressionId::Happy
        );
        assert_eq!(
            state.select(
                &config,
                t + Duration::from_millis(500),
                context(600, 600, false)
            ),
            ExpressionId::Idle
        );
    }

    #[test]
    fn names_and_configuration_limits_are_validated() {
        for (index, id) in ExpressionId::ALL.into_iter().enumerate() {
            assert_eq!(ExpressionId::from_index(index as i32).unwrap(), id);
            assert_eq!(id.index(), index);
        }
        assert!(ExpressionId::from_index(-1).is_err());
        assert!(ExpressionId::from_index(6).is_err());
        assert!(serde_json::from_str::<ExpressionConfig>(r#"{"idle":"missing"}"#).is_err());
        assert!(serde_json::from_str::<ExpressionConfig>(r#"{"idle":null}"#).is_err());
        assert!(serde_json::from_str::<ExpressionConfig>(r#"{"mystery":true}"#).is_err());
        assert!(serde_json::from_str::<ExpressionConfig>(r#"{"click_duration_ms":-1}"#).is_err());
        assert!(serde_json::from_str::<ExpressionConfig>(r#"{"click_duration_ms":0.5}"#).is_err());
        for field in [
            "input_duration_ms",
            "click_duration_ms",
            "drag_duration_ms",
            "sleepy_after_ms",
            "blink_interval_ms",
            "blink_duration_ms",
        ] {
            for value in [0, u32::MAX] {
                let mut json = serde_json::to_value(ExpressionConfig::default()).unwrap();
                json[field] = value.into();
                assert!(
                    serde_json::from_value::<ExpressionConfig>(json)
                        .unwrap()
                        .validate()
                        .is_err(),
                    "{field}={value}"
                );
            }
        }
        let equal = ExpressionConfig {
            blink_interval_ms: 1000,
            blink_duration_ms: 1000,
            ..ExpressionConfig::default()
        };
        assert!(equal.validate().is_err());
        let maximum = ExpressionConfig {
            blink_interval_ms: 30_000,
            blink_duration_ms: 1000,
            input_duration_ms: 10_000,
            click_duration_ms: 10_000,
            drag_duration_ms: 10_000,
            sleepy_after_ms: 3_600_000,
            ..ExpressionConfig::default()
        };
        assert!(maximum.validate().is_ok());
    }
}
