use crate::config::Config;
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
}
impl Notice {
    pub fn system(key: &str, text: &str) -> Self {
        Self {
            key: key.into(),
            text: text.into(),
            simulated: false,
            important: false,
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
        self.queue.retain(|(notice, _)| match notice.key.as_str() {
            "cpu" => config.cpu,
            "memory" => config.memory,
            "battery10" | "battery20" => config.battery,
            "power-on" | "power-off" => config.power,
            "network-on" | "network-off" => config.network,
            key if key.starts_with("codex:") => config.codex,
            _ => true,
        });
    }
}
#[derive(Debug, Clone, Default)]
pub struct SystemSample {
    pub cpu: Option<f64>,
    pub memory: Option<f64>,
    pub battery: Option<u8>,
    pub charging: Option<bool>,
    pub online: Option<bool>,
}
#[derive(Default)]
struct Sustained {
    since: Option<Instant>,
    last: Option<Instant>,
}
impl Sustained {
    fn check(&mut self, value: Option<f64>, enabled: bool, now: Instant) -> bool {
        if !enabled || !value.is_some_and(|v| v.is_finite() && v > 90.) {
            self.since = None;
            return false;
        }
        let since = *self.since.get_or_insert(now);
        if now.saturating_duration_since(since) >= Duration::from_secs(30)
            && self
                .last
                .is_none_or(|at| now.saturating_duration_since(at) >= Duration::from_secs(300))
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
    last_power: Option<bool>,
    network: Option<bool>,
    candidate: Option<(bool, Instant)>,
    battery20: bool,
    battery10: bool,
}
impl SystemRules {
    pub fn update(&mut self, s: &SystemSample, c: &Config, now: Instant) -> Vec<Notice> {
        let mut out = Vec::new();
        if self.cpu.check(s.cpu, c.cpu, now) {
            out.push(Notice::system(
                "cpu",
                "电脑有点忙，处理器已经持续高负载了。",
            ));
        }
        if self.memory.check(s.memory, c.memory, now) {
            out.push(Notice::system(
                "memory",
                "内存快装满啦，可以关掉暂时不用的程序。",
            ));
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
        assert!(r.update(&s, &c, t + Duration::from_secs(60)).is_empty());
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
