use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AlertRule {
    pub threshold: u8,
    pub sustained_seconds: u32,
    pub interval_minutes: u32,
}

impl Default for AlertRule {
    fn default() -> Self {
        Self {
            threshold: 90,
            sustained_seconds: 30,
            interval_minutes: 5,
        }
    }
}

impl AlertRule {
    pub fn validate(self) -> Result<(), String> {
        if !(1..=100).contains(&self.threshold) {
            return Err("提醒阈值必须在 1% 到 100% 之间。".into());
        }
        if !(5..=3600).contains(&self.sustained_seconds)
            || !self.sustained_seconds.is_multiple_of(5)
        {
            return Err("持续时间必须为 5–3600 秒，且是 5 秒的整数倍。".into());
        }
        if !(1..=60).contains(&self.interval_minutes) {
            return Err("重复提醒间隔必须在 1 到 60 分钟之间。".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ResourceAlerts {
    pub cpu: AlertRule,
    pub memory: AlertRule,
    pub gpu: AlertRule,
    pub gpu_enabled: bool,
}

impl Default for ResourceAlerts {
    fn default() -> Self {
        Self {
            cpu: AlertRule::default(),
            memory: AlertRule::default(),
            gpu: AlertRule::default(),
            gpu_enabled: true,
        }
    }
}

impl ResourceAlerts {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
    pub fn validate(&self) -> Result<(), String> {
        self.cpu.validate()?;
        self.memory.validate()?;
        self.gpu.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranges_and_defaults_are_explicit() {
        assert!(ResourceAlerts::default().validate().is_ok());
        for rule in [
            AlertRule {
                threshold: 0,
                ..AlertRule::default()
            },
            AlertRule {
                threshold: 101,
                ..AlertRule::default()
            },
            AlertRule {
                sustained_seconds: 6,
                ..AlertRule::default()
            },
            AlertRule {
                interval_minutes: 0,
                ..AlertRule::default()
            },
            AlertRule {
                interval_minutes: 61,
                ..AlertRule::default()
            },
        ] {
            assert!(rule.validate().is_err());
        }
        assert!(
            AlertRule {
                threshold: 100,
                sustained_seconds: 3600,
                interval_minutes: 60
            }
            .validate()
            .is_ok()
        );
    }
}
