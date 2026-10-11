use serde_json::Value;
use std::time::{Duration, Instant, SystemTime};

#[derive(Clone, Debug, PartialEq)]
pub struct QuotaWindow {
    pub title: String,
    pub remaining: Option<f64>,
    pub resets_at: Option<i64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuotaSnapshot {
    pub windows: Vec<QuotaWindow>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsageError {
    MissingCodex,
    NotLoggedIn,
    UnsupportedAccount,
    Timeout,
    Network,
    Protocol,
    Process,
    Cancelled,
}

impl UsageError {
    pub fn message(self) -> &'static str {
        match self {
            Self::MissingCodex => "未找到 codex.exe，请将本机 Codex CLI 加入 PATH 后重试。",
            Self::NotLoggedIn => "Codex 尚未登录或登录已失效，请在 Codex 中登录后重试。",
            Self::UnsupportedAccount => "当前 Codex 登录环境不提供 ChatGPT 订阅额度。",
            Self::Timeout => "额度查询超时，请稍后重试。",
            Self::Network => "无法连接额度服务，请检查网络后重试。",
            Self::Protocol => "Codex 额度响应不可用或版本不兼容，请稍后重试。",
            Self::Process => "无法完成本机 Codex 查询，请检查 CLI 是否可正常运行。",
            Self::Cancelled => "查询已取消，展开面板或点击刷新可重新查询。",
        }
    }
}

pub fn rpc_error(error: &Value) -> UsageError {
    // 只用上游文案分类，不将可能包含账号或路径的原始错误输出到界面或日志。
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_lowercase();
    if [
        "401",
        "unauthorized",
        "not logged",
        "not authenticated",
        "authentication required",
        "refresh token",
        "login",
    ]
    .iter()
    .any(|s| message.contains(s))
    {
        UsageError::NotLoggedIn
    } else if [
        "not supported",
        "only available",
        "chatgpt auth",
        "chatgpt account",
        "api key",
    ]
    .iter()
    .any(|s| message.contains(s))
    {
        UsageError::UnsupportedAccount
    } else if ["timeout", "timed out"].iter().any(|s| message.contains(s)) {
        UsageError::Timeout
    } else if [
        "network",
        "connect",
        "dns",
        "request failed",
        "error sending request",
        "offline",
    ]
    .iter()
    .any(|s| message.contains(s))
    {
        UsageError::Network
    } else {
        UsageError::Protocol
    }
}

pub fn check_account(result: &Value) -> Result<(), UsageError> {
    match result.get("account") {
        Some(Value::Null) => Err(UsageError::NotLoggedIn),
        Some(account) => match account.get("type").and_then(Value::as_str) {
            Some("chatgpt" | "chatgptAuthTokens") => Ok(()),
            Some(_) => Err(UsageError::UnsupportedAccount),
            None => Err(UsageError::Protocol),
        },
        None => Err(UsageError::Protocol),
    }
}

fn period(minutes: Option<u64>, fallback: &str) -> String {
    match minutes {
        Some(10080) => "周额度".into(),
        Some(n) if n > 0 && n.is_multiple_of(1440) => format!("{} 天额度", n / 1440),
        Some(n) if n > 0 && n.is_multiple_of(60) => format!("{} 小时额度", n / 60),
        Some(n) if n > 0 => format!("{n} 分钟额度"),
        _ => fallback.into(),
    }
}

pub fn parse_limits(result: &Value) -> Result<QuotaSnapshot, UsageError> {
    let mut windows = Vec::new();
    let buckets: Vec<(&str, &Value)> = match result.get("rateLimitsByLimitId") {
        Some(Value::Object(map)) if !map.is_empty() => {
            if map.len() > 32 {
                return Err(UsageError::Protocol);
            }
            map.iter()
                .map(|(key, value)| (key.as_str(), value))
                .collect()
        }
        Some(Value::Null) | None | Some(Value::Object(_)) => {
            vec![(
                "Codex",
                result.get("rateLimits").ok_or(UsageError::Protocol)?,
            )]
        }
        _ => return Err(UsageError::Protocol),
    };
    for (id, bucket) in buckets {
        if !bucket.is_object() {
            return Err(UsageError::Protocol);
        }
        let name = bucket
            .get("limitName")
            .and_then(Value::as_str)
            .filter(|v| !v.trim().is_empty())
            .unwrap_or(if id == "codex" { "Codex" } else { id });
        let name: String = name.chars().filter(|c| !c.is_control()).take(160).collect();
        for (key, fallback) in [("primary", "主要额度"), ("secondary", "次要额度")] {
            let Some(window) = bucket.get(key).filter(|v| !v.is_null()) else {
                continue;
            };
            if !window.is_object() {
                return Err(UsageError::Protocol);
            }
            let duration = window.get("windowDurationMins").and_then(Value::as_u64);
            windows.push(QuotaWindow {
                title: format!("{} · {}", name, period(duration, fallback)),
                remaining: window
                    .get("usedPercent")
                    .and_then(Value::as_f64)
                    .filter(|v| v.is_finite())
                    .map(|v| (100. - v).clamp(0., 100.)),
                resets_at: window
                    .get("resetsAt")
                    .and_then(Value::as_i64)
                    .filter(|v| *v > 0),
            });
        }
    }
    if windows.is_empty() {
        return Err(UsageError::Protocol);
    }
    Ok(QuotaSnapshot { windows })
}

#[derive(Default)]
pub struct UsageState {
    sequence: u64,
    pub in_flight: Option<u64>,
    pub snapshot: Option<QuotaSnapshot>,
    pub error: Option<UsageError>,
    pub updated_at: Option<SystemTime>,
    last_request: Option<Instant>,
}

impl UsageState {
    pub fn request(&mut self, now: Instant) -> Option<u64> {
        if self.in_flight.is_some() {
            return None;
        }
        self.sequence = self.sequence.wrapping_add(1).max(1);
        self.in_flight = Some(self.sequence);
        self.last_request = Some(now);
        self.snapshot = None;
        self.error = None;
        self.updated_at = None;
        Some(self.sequence)
    }

    pub fn cancel(&mut self) {
        if self.in_flight.take().is_some() {
            self.error = Some(UsageError::Cancelled);
        }
    }

    pub fn complete(
        &mut self,
        id: u64,
        result: Result<QuotaSnapshot, UsageError>,
        now: SystemTime,
    ) -> bool {
        if self.in_flight != Some(id) {
            return false;
        }
        self.in_flight = None;
        match result {
            Ok(snapshot) => {
                self.snapshot = Some(snapshot);
                self.updated_at = Some(now);
                self.error = None;
            }
            Err(error) => {
                self.snapshot = None;
                self.updated_at = None;
                self.error = Some(error);
            }
        }
        true
    }

    pub fn refresh_due(&self, visible: bool, now: Instant) -> bool {
        visible
            && self.in_flight.is_none()
            && self
                .last_request
                .is_some_and(|last| now.saturating_duration_since(last) >= Duration::from_secs(60))
    }

    pub fn status(&self) -> &'static str {
        if self.in_flight.is_some() {
            "正在查询本机 Codex 账号额度…"
        } else if let Some(error) = self.error {
            error.message()
        } else if self.snapshot.is_some() {
            "来自本机 Codex 登录环境"
        } else {
            "点击角色或“刷新额度”读取账号额度。"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn multiple_buckets_take_precedence_and_periods_are_not_hardcoded() {
        let snapshot = parse_limits(&json!({"rateLimits": {"primary":{"usedPercent":99}},"rateLimitsByLimitId":{
            "codex":{"primary":{"usedPercent":25,"windowDurationMins":300,"resetsAt":1900000000},"secondary":{"usedPercent":0,"windowDurationMins":10080}},
            "other":{"limitName":"其他额度","primary":{"usedPercent":100,"windowDurationMins":15}}
        }})).unwrap();
        assert_eq!(snapshot.windows.len(), 3);
        assert_eq!(snapshot.windows[0].remaining, Some(75.));
        assert_eq!(snapshot.windows[0].resets_at, Some(1900000000));
        assert!(snapshot.windows[0].title.contains("5 小时"));
        assert!(snapshot.windows[1].title.contains("周额度"));
        assert_eq!(snapshot.windows[1].remaining, Some(100.));
        assert!(snapshot.windows[2].title.contains("15 分钟"));
        assert_eq!(snapshot.windows[2].remaining, Some(0.));
    }

    #[test]
    fn legacy_missing_and_invalid_fields_remain_explicit() {
        let snapshot = parse_limits(&json!({"rateLimitsByLimitId":{},"rateLimits":{"primary":{},"secondary":{"usedPercent":"invalid","resetsAt":-1}}})).unwrap();
        assert_eq!(snapshot.windows[0].remaining, None);
        assert!(snapshot.windows[0].title.contains("主要额度"));
        assert_eq!(snapshot.windows[1].resets_at, None);
        for value in [
            json!({}),
            json!({"rateLimits":null}),
            json!({"rateLimits":{}}),
            json!({"rateLimitsByLimitId":[]}),
            json!({"rateLimits":{"primary":7}}),
        ] {
            assert_eq!(parse_limits(&value), Err(UsageError::Protocol));
        }
        for (used, expected) in [(-10., 100.), (150., 0.)] {
            let value =
                parse_limits(&json!({"rateLimits":{"primary":{"usedPercent":used}}})).unwrap();
            assert_eq!(value.windows[0].remaining, Some(expected));
        }
    }

    #[test]
    fn requests_coalesce_cancel_and_ignore_outdated_results() {
        let now = Instant::now();
        let mut state = UsageState::default();
        assert!(!state.refresh_due(false, now));
        let old = state.request(now).unwrap();
        assert!(state.request(now).is_none());
        state.cancel();
        let current = state.request(now).unwrap();
        assert!(!state.complete(old, Err(UsageError::Timeout), SystemTime::now()));
        let snapshot = QuotaSnapshot { windows: vec![] };
        assert!(state.complete(current, Ok(snapshot), SystemTime::now()));
        assert!(!state.refresh_due(false, now + Duration::from_secs(60)));
        assert!(state.refresh_due(true, now + Duration::from_secs(60)));
        let next = state.request(now).unwrap();
        assert!(state.snapshot.is_none());
        assert!(state.complete(next, Err(UsageError::Network), SystemTime::now()));
        assert!(state.snapshot.is_none());
        assert!(state.updated_at.is_none());
    }

    #[test]
    fn accounts_and_errors_do_not_expose_raw_responses() {
        assert_eq!(
            check_account(&json!({"account":null})),
            Err(UsageError::NotLoggedIn)
        );
        assert_eq!(
            check_account(&json!({"account":{"type":"apiKey"}})),
            Err(UsageError::UnsupportedAccount)
        );
        assert!(check_account(&json!({"account":{"type":"chatgpt"}})).is_ok());
        assert_eq!(
            rpc_error(&json!({"message":"error sending request: fixture-private-value"})),
            UsageError::Network
        );
        assert!(
            !UsageError::Network
                .message()
                .contains("fixture-private-value")
        );
    }
}
