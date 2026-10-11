use crate::config::Config;
use crate::resource_alerts::AlertRule;
use crate::voice::VoiceCue;
use std::{
    collections::{HashMap, VecDeque},
    time::{Duration, Instant},
};

#[derive(Debug, Clone)]
pub struct Notice {
    pub key: String,
    pub text: String,
    pub simulated: bool,
    pub important: bool,
    pub voice: Option<VoiceCue>,
}
impl Notice {
    pub fn system(key: &str, text: &str) -> Self {
        Self {
            key: key.into(),
            text: text.into(),
            simulated: false,
            important: false,
            voice: VoiceCue::system(key),
        }
    }
}
#[derive(Debug, Default)]
pub struct Announcer {
    queue: VecDeque<(Notice, Instant)>,
    seen: HashMap<String, Instant>,
}
impl Announcer {
    pub fn push(&mut self, notice: Notice, now: Instant) {
        self.seen
            .retain(|_, at| now.saturating_duration_since(*at) < Duration::from_secs(300));
        if self.seen.contains_key(&notice.key) {
            return;
        }
        if self.seen.len() >= 512
            && let Some(oldest) = self
                .seen
                .iter()
                .min_by_key(|(_, at)| **at)
                .map(|(key, _)| key.clone())
        {
            self.seen.remove(&oldest);
        }
        self.seen.insert(notice.key.clone(), now);
        if self.queue.len() >= 12 {
            self.queue.pop_back();
        }
        if notice.important {
            self.queue.push_front((notice, now));
        } else {
            self.queue.push_back((notice, now));
        }
    }
    pub fn next(&mut self, now: Instant, suppressed: bool) -> Option<Notice> {
        if suppressed {
            self.queue.clear();
            return None;
        }
        while let Some((notice, at)) = self.queue.pop_front() {
            if now.saturating_duration_since(at) < Duration::from_secs(45) {
                return Some(notice);
            }
        }
        None
    }
    pub fn clear(&mut self) {
        self.queue.clear();
    }
    pub fn remove_disabled(&mut self, config: &Config) {
        self.queue.retain(
            |(notice, _)| match notice.key.split(':').next().unwrap_or("") {
                "cpu" => config.cpu,
                "memory" => config.memory,
                "gpu" => config.resource_alerts.gpu_enabled,
                "battery10" | "battery20" => config.battery,
                "power-on" | "power-off" => config.power,
                "network-on" | "network-off" => config.network,
                "codex" => config.codex,
                _ => true,
            },
        );
    }
    pub fn remove_changed_resources(&mut self, before: &Config, after: &Config) {
        for (key, changed) in [
            (
                "cpu",
                before.cpu != after.cpu || before.resource_alerts.cpu != after.resource_alerts.cpu,
            ),
            (
                "memory",
                before.memory != after.memory
                    || before.resource_alerts.memory != after.resource_alerts.memory,
            ),
            (
                "gpu",
                before.resource_alerts.gpu_enabled != after.resource_alerts.gpu_enabled
                    || before.resource_alerts.gpu != after.resource_alerts.gpu,
            ),
        ] {
            if changed {
                self.queue
                    .retain(|(notice, _)| notice.key.split(':').next() != Some(key));
            }
        }
    }
}
#[derive(Debug, Clone, Default)]
pub struct SystemSample {
    pub cpu: Option<f64>,
    pub memory: Option<f64>,
    pub gpu: Option<f64>,
    pub battery: Option<u8>,
    pub charging: Option<bool>,
    pub online: Option<bool>,
}
#[derive(Default)]
struct Sustained {
    since: Option<Instant>,
    last: Option<Instant>,
    rule: Option<(bool, AlertRule)>,
}
impl Sustained {
    fn configure(&mut self, enabled: bool, rule: AlertRule) {
        if self.rule != Some((enabled, rule)) {
            self.since = None;
            self.last = None;
            self.rule = Some((enabled, rule));
        }
    }
    fn check(&mut self, value: Option<f64>, enabled: bool, rule: AlertRule, now: Instant) -> bool {
        self.configure(enabled, rule);
        if !enabled
            || !value.is_some_and(|v| {
                v.is_finite() && (0.0..=100.0).contains(&v) && v >= f64::from(rule.threshold)
            })
        {
            self.since = None;
            return false;
        }
        let since = *self.since.get_or_insert(now);
        if now.saturating_duration_since(since)
            >= Duration::from_secs(u64::from(rule.sustained_seconds))
            && self.last.is_none_or(|at| {
                now.saturating_duration_since(at)
                    >= Duration::from_secs(u64::from(rule.interval_minutes) * 60)
            })
        {
            self.last = Some(now);
            return true;
        }
        false
    }
}
#[derive(Default)]
pub struct SystemRules {
    cpu: Sustained,
    memory: Sustained,
    gpu: Sustained,
    sequence: u64,
    last_power: Option<bool>,
    network: Option<bool>,
    candidate: Option<(bool, Instant)>,
    battery20: bool,
    battery10: bool,
}
impl SystemRules {
    pub fn configure(&mut self, config: &Config) {
        self.cpu.configure(config.cpu, config.resource_alerts.cpu);
        self.memory
            .configure(config.memory, config.resource_alerts.memory);
        self.gpu.configure(
            config.resource_alerts.gpu_enabled,
            config.resource_alerts.gpu,
        );
    }
    pub fn update(&mut self, s: &SystemSample, c: &Config, now: Instant) -> Vec<Notice> {
        let mut out = Vec::new();
        for (key, label, value, enabled, rule, tracker) in [
            (
                "cpu",
                "CPU",
                s.cpu,
                c.cpu,
                c.resource_alerts.cpu,
                &mut self.cpu,
            ),
            (
                "memory",
                "内存",
                s.memory,
                c.memory,
                c.resource_alerts.memory,
                &mut self.memory,
            ),
            (
                "gpu",
                "GPU",
                s.gpu,
                c.resource_alerts.gpu_enabled,
                c.resource_alerts.gpu,
                &mut self.gpu,
            ),
        ] {
            if tracker.check(value, enabled, rule, now) {
                self.sequence = self.sequence.wrapping_add(1);
                let mut notice = Notice::system(
                    key,
                    &format!(
                        "{label} 持续高占用，当前 {:.0}%（阈值 {}%）。",
                        value.unwrap_or_default(),
                        rule.threshold
                    ),
                );
                // 资源规则已负责冷却，每次触发独立标识，不能被通用五分钟去重压住。
                notice.key = format!("{key}:{}", self.sequence);
                out.push(notice);
            }
        }
        if let Some(power) = s.charging {
            if c.power && self.last_power.is_some_and(|old| old != power) {
                out.push(Notice::system(
                    if power { "power-on" } else { "power-off" },
                    if power {
                        "电源已接好，安心工作吧。"
                    } else {
                        "现在使用电池供电啦。"
                    },
                ));
            }
            self.last_power = Some(power);
            if power {
                self.battery20 = false;
                self.battery10 = false;
            }
        }
        if let Some(percent) = s.battery {
            if percent > 20 {
                self.battery20 = false;
                self.battery10 = false;
            }
            if c.battery && s.charging == Some(false) {
                if percent <= 10 && !self.battery10 {
                    self.battery10 = true;
                    self.battery20 = true;
                    let mut n = Notice::system("battery10", "电量只剩 10% 或更少，记得接上电源。");
                    n.important = true;
                    out.push(n);
                } else if percent <= 20 && !self.battery20 {
                    self.battery20 = true;
                    out.push(Notice::system(
                        "battery20",
                        "电量只剩 20% 或更少，准备充电吧。",
                    ));
                }
            }
        }
        if let Some(online) = s.online {
            match self.network {
                None => self.network = Some(online),
                Some(old) if old != online => {
                    let (value, at) = self.candidate.get_or_insert((online, now));
                    if *value != online {
                        *value = online;
                        *at = now;
                    }
                    if now.saturating_duration_since(*at) >= Duration::from_secs(5) {
                        self.network = Some(online);
                        self.candidate = None;
                        if c.network {
                            out.push(Notice::system(
                                if online { "network-on" } else { "network-off" },
                                if online {
                                    "互联网连接恢复啦。"
                                } else {
                                    "暂时未连接到互联网，检查一下网络吧。"
                                },
                            ));
                        }
                    }
                }
                _ => self.candidate = None,
            }
        } else {
            self.candidate = None;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saving_and_restoring_a_rule_between_samples_restarts_timing() {
        let mut rules = SystemRules::default();
        let mut config = Config::default();
        let sample = SystemSample {
            cpu: Some(95.),
            ..Default::default()
        };
        let start = Instant::now();
        assert!(rules.update(&sample, &config, start).is_empty());
        config.resource_alerts.cpu.threshold = 99;
        rules.configure(&config);
        config.resource_alerts.cpu.threshold = 90;
        rules.configure(&config);
        assert!(
            rules
                .update(&sample, &config, start + Duration::from_secs(30))
                .is_empty()
        );
        assert_eq!(
            rules
                .update(&sample, &config, start + Duration::from_secs(60))
                .len(),
            1
        );
    }
    #[test]
    fn configurable_rules_reset_and_allow_short_independent_cooldowns() {
        let mut rules = SystemRules::default();
        let mut queue = Announcer::default();
        let mut config = Config::default();
        config.resource_alerts.cpu = AlertRule {
            threshold: 80,
            sustained_seconds: 5,
            interval_minutes: 1,
        };
        config.resource_alerts.gpu = AlertRule {
            threshold: 95,
            sustained_seconds: 10,
            interval_minutes: 2,
        };
        let now = Instant::now();
        let mut sample = SystemSample {
            cpu: Some(80.),
            gpu: Some(95.),
            ..Default::default()
        };
        assert!(rules.update(&sample, &config, now).is_empty());
        for at in [5, 10, 65] {
            let notices = rules.update(&sample, &config, now + Duration::from_secs(at));
            assert_eq!(notices.len(), 1);
            assert_eq!(
                notices[0].voice,
                Some(if at == 10 {
                    VoiceCue::Gpu
                } else {
                    VoiceCue::Cpu
                })
            );
            queue.push(notices[0].clone(), now + Duration::from_secs(at));
            assert!(
                queue.next(now + Duration::from_secs(at), false).is_some(),
                "一分钟提醒不能被固定五分钟去重吞掉"
            );
        }
        let before = config.clone();
        config.resource_alerts.cpu.threshold = 81;
        queue.push(Notice::system("cpu:old", "旧 CPU 提醒"), now);
        queue.push(Notice::system("gpu:old", "GPU 提醒"), now);
        queue.remove_changed_resources(&before, &config);
        assert_eq!(queue.next(now, false).unwrap().key, "gpu:old");
        assert!(queue.next(now, false).is_none());
        sample.cpu = Some(82.);
        sample.gpu = None;
        assert!(
            rules
                .update(&sample, &config, now + Duration::from_secs(70))
                .is_empty()
        );
        assert_eq!(
            rules
                .update(&sample, &config, now + Duration::from_secs(75))
                .len(),
            1
        );
        config.cpu = false;
        config.resource_alerts.gpu_enabled = false;
        sample.gpu = Some(100.);
        assert!(
            rules
                .update(&sample, &config, now + Duration::from_secs(200))
                .is_empty()
        );
        for value in [None, Some(f64::NAN), Some(-1.), Some(101.), Some(79.)] {
            let mut sustained = Sustained::default();
            assert!(!sustained.check(Some(90.), true, config.resource_alerts.cpu, now));
            assert!(!sustained.check(
                value,
                true,
                config.resource_alerts.cpu,
                now + Duration::from_secs(5)
            ));
            assert!(!sustained.check(
                Some(90.),
                true,
                config.resource_alerts.cpu,
                now + Duration::from_secs(6)
            ));
        }
    }
    #[test]
    fn resource_threshold_cooldown_and_unknown() {
        let mut r = SystemRules::default();
        let c = Config::default();
        let t = Instant::now();
        let sample = SystemSample {
            cpu: Some(91.),
            ..Default::default()
        };
        assert!(r.update(&sample, &c, t).is_empty());
        assert!(
            r.update(&sample, &c, t + Duration::from_secs(29))
                .is_empty()
        );
        assert_eq!(r.update(&sample, &c, t + Duration::from_secs(30)).len(), 1);
        assert!(
            r.update(&sample, &c, t + Duration::from_secs(329))
                .is_empty()
        );
        assert_eq!(r.update(&sample, &c, t + Duration::from_secs(330)).len(), 1);
        assert!(
            r.update(&SystemSample::default(), &c, t + Duration::from_secs(400))
                .is_empty()
        );
        assert!(
            r.update(&sample, &c, t + Duration::from_secs(700))
                .is_empty()
        );
    }
    #[test]
    fn battery_and_network_transitions() {
        let mut r = SystemRules::default();
        let c = Config::default();
        let t = Instant::now();
        let s = SystemSample {
            battery: Some(9),
            charging: Some(false),
            online: Some(true),
            ..Default::default()
        };
        assert_eq!(r.update(&s, &c, t).len(), 1);
        assert!(r.update(&s, &c, t).is_empty());
        let off = SystemSample {
            online: Some(false),
            ..s
        };
        assert!(r.update(&off, &c, t + Duration::from_secs(1)).is_empty());
        assert_eq!(r.update(&off, &c, t + Duration::from_secs(6)).len(), 1);
    }
    #[test]
    fn dedup_expiry_and_quiet() {
        let mut a = Announcer::default();
        let t = Instant::now();
        a.push(Notice::system("x", "x"), t);
        a.push(Notice::system("x", "x"), t);
        assert!(a.next(t, false).is_some());
        assert!(a.next(t, false).is_none());
        a.push(Notice::system("y", "y"), t);
        assert!(a.next(t, true).is_none());
        a.push(Notice::system("z", "z"), t);
        assert!(a.next(t + Duration::from_secs(46), false).is_none());
    }
    #[test]
    fn disabled_rules_and_exact_thresholds() {
        let mut r = SystemRules::default();
        let t = Instant::now();
        let c = Config::default();
        let mut s = SystemSample {
            cpu: Some(90.),
            memory: Some(f64::NAN),
            battery: Some(20),
            charging: Some(false),
            ..Default::default()
        };
        assert_eq!(r.update(&s, &c, t)[0].key, "battery20");
        assert!(
            r.update(&s, &c, t + Duration::from_secs(60))[0]
                .key
                .starts_with("cpu:")
        );
        s.battery = Some(10);
        assert_eq!(
            r.update(&s, &c, t + Duration::from_secs(65))[0].key,
            "battery10"
        );
        let disabled = Config {
            battery: false,
            power: false,
            network: false,
            cpu: false,
            memory: false,
            ..Config::default()
        };
        s.charging = Some(true);
        s.cpu = Some(100.);
        s.memory = Some(99.);
        assert!(
            r.update(&s, &disabled, t + Duration::from_secs(70))
                .is_empty()
        );
        assert!(
            r.update(&s, &disabled, t + Duration::from_secs(110))
                .is_empty()
        );
    }
    #[test]
    fn disabled_categories_are_removed_from_pending_queue() {
        let mut a = Announcer::default();
        let t = Instant::now();
        for key in [
            "cpu",
            "memory",
            "battery20",
            "power-on",
            "network-off",
            "codex:Stop:session:turn:false",
            "platform-error",
        ] {
            a.push(Notice::system(key, key), t);
        }
        a.remove_disabled(&Config {
            cpu: false,
            memory: false,
            battery: false,
            power: false,
            network: false,
            codex: false,
            ..Config::default()
        });
        assert_eq!(a.next(t, false).unwrap().key, "platform-error");
        assert!(a.next(t, false).is_none());
    }
    #[test]
    fn burst_is_bounded_and_important_notice_goes_first() {
        let mut a = Announcer::default();
        let t = Instant::now();
        for index in 0..600 {
            a.push(Notice::system(&format!("n{index}"), "x"), t);
        }
        assert_eq!(a.queue.len(), 12);
        assert_eq!(a.seen.len(), 512);
        let mut urgent = Notice::system("urgent", "电量提醒");
        urgent.important = true;
        a.push(urgent, t);
        assert_eq!(a.next(t, false).unwrap().key, "urgent");
        a.push(
            Notice::system("urgent", "电量提醒"),
            t + Duration::from_secs(301),
        );
        assert_eq!(
            a.next(t + Duration::from_secs(301), false).unwrap().key,
            "urgent"
        );
    }
}
