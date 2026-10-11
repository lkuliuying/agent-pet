use crate::notices::Notice;
use crate::voice::VoiceCue;
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_APPROVAL_NOTICE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CodexKind {
    #[default]
    Stop,
    PermissionRequest,
    Interrupt,
    SessionStart,
    UserPromptSubmit,
    PreToolUse,
    PostToolUse,
    SessionEnd,
}
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodexEvent {
    pub version: u8,
    pub kind: CodexKind,
    pub session_id: String,
    pub turn_id: String,
    pub simulated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emitted_at_ms: Option<u64>,
}
#[derive(Deserialize)]
struct HookInput {
    hook_event_name: CodexKind,
    session_id: String,
    #[serde(default)]
    turn_id: String,
    #[serde(default)]
    tool_use_id: Option<String>,
    #[serde(default)]
    tool_name: Option<String>,
}
impl CodexEvent {
    pub fn validate(&self) -> Result<(), String> {
        if !matches!(self.version, 1 | 2) {
            return Err("不支持的事件版本".into());
        }
        for (index, id) in [&self.session_id, &self.turn_id].into_iter().enumerate() {
            if id.is_empty()
                && index == 1
                && self.version == 2
                && matches!(self.kind, CodexKind::SessionStart | CodexKind::SessionEnd)
            {
                continue;
            }
            if id.is_empty()
                || id.len() > 128
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_:".contains(&b))
            {
                return Err("事件标识无效".into());
            }
        }
        for value in [&self.request_id, &self.tool_name].into_iter().flatten() {
            if value.is_empty()
                || value.len() > 160
                || !value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_:./".contains(&b))
            {
                return Err("工具标识无效".into());
            }
        }
        Ok(())
    }
    pub fn notice(&self) -> Notice {
        let mut key = format!(
            "codex:{:?}:{}:{}:{}",
            self.kind, self.session_id, self.turn_id, self.simulated
        );
        if self.kind == CodexKind::PermissionRequest {
            // 有请求标识时去重，缺失时按本次接收区分，保留同轮的多次请求。
            if let Some(id) = &self.request_id {
                key.push_str(&format!(":{id}"));
            } else {
                key.push_str(&format!(
                    ":{}",
                    NEXT_APPROVAL_NOTICE.fetch_add(1, Ordering::Relaxed)
                ));
            }
        }
        Notice {
            key,
            text: match self.kind {
                CodexKind::Stop => "Codex 这一轮回复结束了。",
                CodexKind::PermissionRequest => "Codex 发起了权限请求。",
                CodexKind::Interrupt => "这一轮已中断。",
                _ => "Codex 状态已更新。",
            }
            .into(),
            simulated: self.simulated,
            important: matches!(self.kind, CodexKind::PermissionRequest),
            voice: match (self.kind, self.simulated) {
                (CodexKind::Stop, false) => Some(VoiceCue::CodexStop),
                (CodexKind::PermissionRequest, false) => Some(VoiceCue::CodexPermission),
                (CodexKind::Interrupt, false) => Some(VoiceCue::CodexInterrupt),
                (CodexKind::Stop, true) => Some(VoiceCue::DemoCodexStop),
                (CodexKind::PermissionRequest, true) => Some(VoiceCue::DemoCodexPermission),
                (CodexKind::Interrupt, true) => Some(VoiceCue::DemoCodexInterrupt),
                _ => None,
            },
        }
    }
}
pub fn parse_hook(reader: impl Read) -> Result<CodexEvent, String> {
    let mut bytes = Vec::new();
    reader
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "无法读取事件".to_string())?;
    if bytes.len() > 1024 * 1024 {
        return Err("事件超出大小限制".into());
    }
    // 反序列化时跳过正文，仅允许最小状态字段进入进程间通道。
    let input: HookInput =
        serde_json::from_slice(&bytes).map_err(|_| "事件格式无效".to_string())?;
    let event = CodexEvent {
        version: 2,
        kind: input.hook_event_name,
        session_id: input.session_id,
        turn_id: input.turn_id,
        simulated: false,
        request_id: input.tool_use_id,
        tool_name: input.tool_name,
        emitted_at_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map(|v| v.as_millis() as u64),
    };
    event.validate()?;
    Ok(event)
}
pub fn parse_wire(bytes: &[u8]) -> Result<CodexEvent, String> {
    if bytes.len() > 2048 {
        return Err("消息过长".into());
    }
    let event: CodexEvent = serde_json::from_slice(bytes).map_err(|_| "消息无效".to_string())?;
    event.validate()?;
    Ok(event)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lifecycle_v2_and_legacy_v1_are_compatible_and_private_fields_are_dropped() {
        for kind in ["SessionStart", "SessionEnd"] {
            let event = parse_hook(
                format!("{{\"hook_event_name\":\"{kind}\",\"session_id\":\"s\"}}").as_bytes(),
            )
            .unwrap();
            assert!(event.turn_id.is_empty());
            assert_eq!(event.version, 2);
        }
        let event = parse_hook(br#"{"hook_event_name":"PreToolUse","session_id":"s","turn_id":"t","tool_use_id":"call","tool_name":"exec_command","tool_input":{"command":"private"},"credential":"private"}"#.as_slice()).unwrap();
        assert_eq!(event.request_id.as_deref(), Some("call"));
        assert!(!serde_json::to_string(&event).unwrap().contains("private"));
        assert!(
            parse_wire(
                br#"{"version":1,"kind":"Stop","session_id":"s","turn_id":"t","simulated":false}"#
            )
            .is_ok()
        );
        assert!(
            parse_wire(
                br#"{"version":3,"kind":"Stop","session_id":"s","turn_id":"t","simulated":false}"#
            )
            .is_err()
        );
    }
    #[test]
    fn separate_approvals_in_one_turn_are_not_deduplicated() {
        let event = parse_hook(
            br#"{"hook_event_name":"PermissionRequest","session_id":"s1","turn_id":"t1"}"#
                .as_slice(),
        )
        .unwrap();
        let mut announcer = crate::notices::Announcer::default();
        let now = std::time::Instant::now();
        announcer.push(event.notice(), now);
        announcer.push(event.notice(), now);
        assert!(announcer.next(now, false).unwrap().important);
        assert!(announcer.next(now, false).unwrap().important);
        assert!(announcer.next(now, false).is_none());
        for kind in [CodexKind::Stop, CodexKind::Interrupt] {
            let event = CodexEvent {
                kind,
                ..event.clone()
            };
            announcer.push(event.notice(), now);
            announcer.push(event.notice(), now);
            assert!(announcer.next(now, false).is_some());
            assert!(announcer.next(now, false).is_none());
        }
    }
    #[test]
    fn input_is_reduced_and_semantics_preserved() {
        let e=parse_hook(br#"{"hook_event_name":"Stop","session_id":"session-1","turn_id":"turn-1","prompt":"private body","last_assistant_message":"private answer"}"#.as_slice()).unwrap();
        let wire = serde_json::to_string(&e).unwrap();
        assert!(!wire.contains("private"));
        assert!(e.notice().text.contains("回复结束"));
        assert!(parse_wire(wire.as_bytes()).is_ok());
    }
    #[test]
    fn invalid_payloads_are_rejected() {
        for input in [
            "{}",
            "null",
            "[]",
            "{broken",
            r#"{"hook_event_name":"Fake","session_id":"a","turn_id":"b"}"#,
            r#"{"hook_event_name":"Stop","session_id":"../x","turn_id":"b"}"#,
        ] {
            assert!(parse_hook(input.as_bytes()).is_err());
        }
        assert!(parse_hook(vec![b' '; 1024 * 1024 + 1].as_slice()).is_err());
        assert!(parse_wire(&[b' '; 2049]).is_err());
    }
}
