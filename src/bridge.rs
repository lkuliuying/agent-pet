use crate::notices::Notice;
use serde::{Deserialize, Serialize};
use std::io::Read;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CodexKind {
    Stop,
    PermissionRequest,
    Interrupt,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodexEvent {
    pub version: u8,
    pub kind: CodexKind,
    pub session_id: String,
    pub turn_id: String,
    pub simulated: bool,
}
#[derive(Deserialize)]
struct HookInput {
    hook_event_name: CodexKind,
    session_id: String,
    turn_id: String,
}
impl CodexEvent {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("不支持的事件版本".into());
        }
        for id in [&self.session_id, &self.turn_id] {
            if id.is_empty()
                || id.len() > 128
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_:".contains(&b))
            {
                return Err("事件标识无效".into());
            }
        }
        Ok(())
    }
    pub fn notice(&self) -> Notice {
        Notice {
            key: format!(
                "codex:{:?}:{}:{}:{}",
                self.kind, self.session_id, self.turn_id, self.simulated
            ),
            text: match self.kind {
                CodexKind::Stop => "Codex 这一轮回复结束了。",
                CodexKind::PermissionRequest => "Codex 发起了审批请求。",
                CodexKind::Interrupt => "这一轮已中断。",
            }
            .into(),
            simulated: self.simulated,
            important: matches!(self.kind, CodexKind::PermissionRequest),
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
        version: 1,
        kind: input.hook_event_name,
        session_id: input.session_id,
        turn_id: input.turn_id,
        simulated: false,
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
