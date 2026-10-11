use super::{AlertValues, ResourceValues};
use pet_desktop::resource_alerts::{AlertRule, ResourceAlerts};

fn rule_values(rule: AlertRule) -> AlertValues {
    AlertValues {
        threshold: i32::from(rule.threshold),
        seconds: rule.sustained_seconds as i32,
        minutes: rule.interval_minutes as i32,
    }
}

pub fn values(rules: &ResourceAlerts) -> ResourceValues {
    ResourceValues {
        gpu: rules.gpu_enabled,
        cpu_rule: rule_values(rules.cpu),
        memory_rule: rule_values(rules.memory),
        gpu_rule: rule_values(rules.gpu),
    }
}

fn rule(value: AlertValues) -> Result<AlertRule, String> {
    let invalid = || "高占用提醒数值无效，请检查阈值、持续时间和间隔。".to_owned();
    let rule = AlertRule {
        threshold: value.threshold.try_into().map_err(|_| invalid())?,
        sustained_seconds: value.seconds.try_into().map_err(|_| invalid())?,
        interval_minutes: value.minutes.try_into().map_err(|_| invalid())?,
    };
    rule.validate()?;
    Ok(rule)
}

pub fn config(value: ResourceValues) -> Result<ResourceAlerts, String> {
    Ok(ResourceAlerts {
        cpu: rule(value.cpu_rule)?,
        memory: rule(value.memory_rule)?,
        gpu: rule(value.gpu_rule)?,
        gpu_enabled: value.gpu,
    })
}
