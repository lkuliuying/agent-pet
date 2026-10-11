use crate::{
    bridge::{CodexEvent, CodexKind},
    notices::Notice,
    voice::VoiceCue,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

fn optional_text<'de, D: serde::Deserializer<'de>>(value: D) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(value)?.unwrap_or_default())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    Running,
    Waiting,
    Approval,
    Unconfirmed,
    Finished,
    Interrupted,
}
impl SessionStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Running => "运行中",
            Self::Waiting => "待回答",
            Self::Approval => "权限请求",
            Self::Unconfirmed => "状态待确认",
            Self::Finished => "已结束",
            Self::Interrupted => "已中断",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionView {
    pub id: String,
    pub title: String,
    pub project: String,
    pub status: SessionStatus,
}

#[derive(Default, Deserialize)]
pub struct Record {
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub payload: Payload,
}

// 只反序列化白名单字段；正文、参数、输出和身份信息由 serde 跳过。
#[derive(Default, Deserialize)]
pub struct Payload {
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default, deserialize_with = "optional_text")]
    pub id: String,
    #[serde(default, deserialize_with = "optional_text")]
    pub turn_id: String,
    #[serde(default, deserialize_with = "optional_text")]
    pub originator: String,
    #[serde(default, deserialize_with = "optional_text")]
    pub cwd: String,
    #[serde(default, deserialize_with = "optional_text")]
    pub name: String,
    #[serde(default, deserialize_with = "optional_text")]
    pub call_id: String,
    #[serde(default)]
    pub source: serde_json::Value,
    pub item: Option<SafeItem>,
}

#[derive(Default, Deserialize)]
pub struct SafeItem {
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default, deserialize_with = "optional_text")]
    pub id: String,
    pub questions: Option<Vec<serde::de::IgnoredAny>>,
    #[serde(default)]
    pub content: Vec<SafeContent>,
}

#[derive(Default, Deserialize)]
pub struct SafeContent {
    pub questions: Option<Vec<serde::de::IgnoredAny>>,
}

pub fn parse_record(bytes: &[u8]) -> Result<Record, &'static str> {
    if bytes.len() > 1024 * 1024 {
        return Err("状态记录超出大小限制");
    }
    serde_json::from_slice(bytes).map_err(|_| "状态记录格式不兼容")
}

pub fn clean_label(value: &str, maximum: usize) -> String {
    value
        .chars()
        .filter(|c| !c.is_control())
        .take(maximum)
        .collect()
}

#[derive(Default)]
pub struct Session {
    pub id: String,
    pub project: String,
    pub title: String,
    pub desktop: bool,
    pub turn: String,
    running: bool,
    interrupted: bool,
    uncertain: bool,
    ended_turns: BTreeSet<String>,
    sync_questions: BTreeSet<String>,
    async_questions: BTreeSet<String>,
    approvals: BTreeSet<String>,
    pending_tools: BTreeMap<String, String>,
    latest_hook_ms: u64,
}

impl Session {
    pub fn check_reconstruction(&mut self) {
        if self.desktop && self.turn.is_empty() && self.ended_turns.is_empty() {
            self.uncertain = true;
        }
    }
    pub fn status(&self) -> SessionStatus {
        if self.uncertain {
            SessionStatus::Unconfirmed
        } else if !self.sync_questions.is_empty() || !self.async_questions.is_empty() {
            SessionStatus::Waiting
        } else if !self.approvals.is_empty() {
            SessionStatus::Approval
        } else if self.running {
            SessionStatus::Running
        } else if self.interrupted {
            SessionStatus::Interrupted
        } else {
            SessionStatus::Finished
        }
    }

    pub fn view(&self) -> Option<SessionView> {
        if !self.desktop
            || matches!(
                self.status(),
                SessionStatus::Finished | SessionStatus::Interrupted
            )
        {
            return None;
        }
        let title = if self.title.is_empty() {
            let project = self
                .project
                .rsplit(['/', '\\'])
                .next()
                .filter(|v| !v.is_empty())
                .unwrap_or("Codex 会话");
            format!(
                "{project} · {}",
                self.id.chars().take(8).collect::<String>()
            )
        } else {
            self.title.clone()
        };
        Some(SessionView {
            id: self.id.clone(),
            title,
            project: self.project.clone(),
            status: self.status(),
        })
    }

    pub fn mark_unconfirmed(&mut self) {
        if self.view().is_some() {
            self.uncertain = true;
        }
    }

    fn start(&mut self, turn: &str) {
        if turn.is_empty() || self.ended_turns.contains(turn) {
            return;
        }
        if self.turn != turn {
            self.sync_questions.clear();
            self.approvals.clear();
            self.pending_tools.clear();
            self.turn = turn.into();
        }
        self.running = true;
        self.interrupted = false;
        self.uncertain = false;
    }

    fn finish(&mut self, turn: &str, interrupted: bool) {
        if !turn.is_empty() {
            self.ended_turns.insert(turn.into());
            if self.ended_turns.len() > 128 {
                self.ended_turns.pop_first();
            }
        }
        if !self.turn.is_empty() && turn != self.turn {
            return;
        }
        self.running = false;
        self.interrupted = interrupted;
        self.sync_questions.clear();
        self.approvals.clear();
        self.pending_tools.clear();
        self.uncertain = false;
        if interrupted {
            self.async_questions.clear();
        }
    }

    fn question(&mut self, id: &str, asynchronous: bool, live: bool) -> Option<Notice> {
        if id.is_empty() || id.len() > 160 {
            return None;
        }
        self.uncertain = false;
        let pending = if asynchronous {
            &mut self.async_questions
        } else {
            &mut self.sync_questions
        };
        if pending.len() >= 64 || !pending.insert(id.into()) || !live {
            return None;
        }
        Some(Notice {
            key: format!("codex:question:{}:{id}", self.id),
            text: format!(
                "{}：Codex 正在等待你的回答。",
                self.view()
                    .map(|v| v.title)
                    .unwrap_or_else(|| "当前会话".into())
            ),
            simulated: false,
            important: true,
            voice: Some(VoiceCue::CodexQuestion),
        })
    }

    pub fn record(&mut self, record: Record, live: bool) -> Option<Notice> {
        let p = record.payload;
        if record.kind == "session_meta" {
            self.desktop = p.originator == "Codex Desktop" && p.source.as_str() == Some("vscode");
            self.id = clean_label(&p.id, 128);
            self.project = clean_label(&p.cwd, 1024);
            return None;
        }
        if !self.desktop {
            return None;
        }
        match record.kind.as_str() {
            "event_msg" => match p.kind.as_str() {
                "task_started" => self.start(&p.turn_id),
                "task_complete" => self.finish(&p.turn_id, false),
                "turn_aborted" => self.finish(&p.turn_id, true),
                "user_message" => {
                    self.async_questions.clear();
                }
                "item_completed" => {
                    if self.ended_turns.contains(&p.turn_id) {
                        return None;
                    }
                    if live {
                        self.start(&p.turn_id);
                    }
                    if let Some(item) = p.item {
                        if matches!(item.kind.as_str(), "UserMessage" | "userMessage") {
                            self.async_questions.clear();
                        } else if matches!(item.kind.as_str(), "AgentMessage" | "agentMessage")
                            && (item.questions.is_some_and(|v| !v.is_empty())
                                || item
                                    .content
                                    .iter()
                                    .any(|c| c.questions.as_ref().is_some_and(|v| !v.is_empty())))
                            && self.async_questions.is_empty()
                        {
                            return self.question(&item.id, true, live);
                        }
                    }
                }
                _ => {}
            },
            "response_item" => match p.kind.as_str() {
                "function_call" if !self.ended_turns.contains(&self.turn) => {
                    let name = p.name.rsplit('.').next().unwrap_or(&p.name);
                    if name == "request_user_input" {
                        return self.question(&p.call_id, false, live);
                    }
                    if name == "request_user_input_async" && self.async_questions.is_empty() {
                        return self.question(&p.call_id, true, live);
                    }
                }
                "function_call_output" => {
                    self.sync_questions.remove(&p.call_id);
                    self.approvals.remove(&p.call_id);
                }
                _ => {}
            },
            _ => {}
        }
        None
    }

    pub fn hook(&mut self, event: &CodexEvent) -> Option<Notice> {
        if !self.desktop || event.simulated || event.session_id != self.id {
            return None;
        }
        if let Some(at) = event.emitted_at_ms {
            if at < self.latest_hook_ms {
                return None;
            }
            self.latest_hook_ms = at;
        }
        if self.ended_turns.contains(&event.turn_id)
            && !matches!(event.kind, CodexKind::Stop | CodexKind::Interrupt)
        {
            return None;
        }
        match event.kind {
            CodexKind::UserPromptSubmit => {
                self.async_questions.clear();
                self.start(&event.turn_id);
            }
            CodexKind::PreToolUse => {
                self.start(&event.turn_id);
                if let (Some(id), Some(name)) = (&event.request_id, &event.tool_name)
                    && self.pending_tools.len() < 128
                {
                    self.pending_tools.insert(id.clone(), name.clone());
                }
            }
            CodexKind::PermissionRequest => {
                self.start(&event.turn_id);
                let candidates: Vec<_> = self
                    .pending_tools
                    .iter()
                    .filter(|(_, name)| Some(*name) == event.tool_name.as_ref())
                    .map(|(id, _)| id.clone())
                    .collect();
                let mut notice_event = event.clone();
                if notice_event.request_id.is_none() && candidates.len() == 1 {
                    notice_event.request_id = candidates.first().cloned();
                }
                if let Some(id) = &notice_event.request_id {
                    self.approvals.insert(id.clone());
                } else {
                    self.uncertain = true;
                }
                let mut notice = event.notice();
                if let Some(view) = self.view() {
                    notice.text = format!("{}：{}", view.title, notice.text);
                }
                return Some(notice);
            }
            CodexKind::PostToolUse => {
                if let Some(id) = &event.request_id {
                    self.pending_tools.remove(id);
                    self.approvals.remove(id);
                }
            }
            CodexKind::Stop | CodexKind::Interrupt => {
                self.finish(&event.turn_id, event.kind == CodexKind::Interrupt);
                return Some(event.notice());
            }
            CodexKind::SessionEnd => {
                self.running = false;
                self.async_questions.clear();
                self.sync_questions.clear();
                self.approvals.clear();
                self.uncertain = false;
            }
            CodexKind::SessionStart => {}
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn session() -> Session {
        let mut s = Session::default();
        s.record(parse_record(br#"{"type":"session_meta","payload":{"id":"s","originator":"Codex Desktop","source":"vscode","cwd":"F:/work","base_instructions":"private"}}"#).unwrap(), false);
        s
    }
    fn event(s: &mut Session, value: serde_json::Value) -> Option<Notice> {
        s.record(
            parse_record(&serde_json::to_vec(&value).unwrap()).unwrap(),
            true,
        )
    }
    #[test]
    fn questions_resolve_independently_and_async_ack_does_not_resolve() {
        let mut s = session();
        event(
            &mut s,
            serde_json::json!({"type":"event_msg","payload":{"type":"task_started","turn_id":"t"}}),
        );
        for (name, id) in [
            ("request_user_input", "one"),
            ("request_user_input_async", "two"),
        ] {
            assert!(event(&mut s, serde_json::json!({"type":"response_item","payload":{"type":"function_call","name":name,"call_id":id,"arguments":"private"}})).is_some());
            event(
                &mut s,
                serde_json::json!({"type":"response_item","payload":{"type":"function_call_output","call_id":id,"output":"private"}}),
            );
        }
        assert_eq!(s.status(), SessionStatus::Waiting);
        event(
            &mut s,
            serde_json::json!({"type":"event_msg","payload":{"type":"task_complete","turn_id":"t"}}),
        );
        assert_eq!(s.status(), SessionStatus::Waiting);
        event(
            &mut s,
            serde_json::json!({"type":"event_msg","payload":{"type":"task_started","turn_id":"next"}}),
        );
        event(
            &mut s,
            serde_json::json!({"type":"event_msg","payload":{"type":"item_completed","turn_id":"next","item":{"type":"UserMessage","content":[{"text":"private"}]}}}),
        );
        assert_eq!(s.status(), SessionStatus::Running);
    }
    #[test]
    fn completed_turn_cannot_be_resurrected_and_cli_is_excluded() {
        let mut s = session();
        s.start("t");
        s.finish("t", true);
        s.start("t");
        assert!(s.view().is_none());
        s.start("new");
        s.mark_unconfirmed();
        assert_eq!(s.status(), SessionStatus::Unconfirmed);
        s.record(parse_record(br#"{"type":"session_meta","payload":{"id":"cli","originator":"codex_cli_rs","source":"cli"}}"#).unwrap(), false);
        assert!(s.view().is_none());
    }

    #[test]
    fn structured_async_questions_need_real_input_and_plain_text_is_ignored() {
        let mut s = session();
        s.start("t");
        assert!(event(&mut s, serde_json::json!({"type":"event_msg","payload":{"type":"item_completed","turn_id":"t","item":{"type":"AgentMessage","id":"message","content":[{"text":"只是问句？"}]}}})).is_none());
        assert_eq!(s.status(), SessionStatus::Running);
        let item = serde_json::json!({"type":"event_msg","payload":{"type":"item_completed","turn_id":"t","item":{"type":"AgentMessage","id":"ask","questions":[{"title":"private","options":["secret"]}],"content":[]}}});
        let notice = event(&mut s, item.clone()).unwrap();
        assert!(!notice.text.contains("private"));
        assert!(event(&mut s, item).is_none());
        s.finish("t", false);
        assert_eq!(s.status(), SessionStatus::Waiting);
        event(
            &mut s,
            serde_json::json!({"type":"event_msg","payload":{"type":"user_message","message":"secret"}}),
        );
        assert!(s.view().is_none());
    }

    #[test]
    fn permissions_keep_separate_missing_ids_and_reconcile_tool_completion() {
        let mut s = session();
        s.start("t");
        let mut hook = CodexEvent {
            version: 2,
            session_id: "s".into(),
            turn_id: "t".into(),
            tool_name: Some("exec_command".into()),
            request_id: Some("call".into()),
            kind: CodexKind::PreToolUse,
            ..Default::default()
        };
        s.hook(&hook);
        hook.kind = CodexKind::PermissionRequest;
        hook.request_id = None;
        let first = s.hook(&hook).unwrap();
        let second = s.hook(&hook).unwrap();
        assert_ne!(first.key, second.key);
        assert_eq!(s.status(), SessionStatus::Approval);
        hook.request_id = Some("call".into());
        assert_eq!(s.hook(&hook).unwrap().key, s.hook(&hook).unwrap().key);
        hook.kind = CodexKind::PostToolUse;
        s.hook(&hook);
        assert_eq!(s.status(), SessionStatus::Running);
        hook.kind = CodexKind::PermissionRequest;
        hook.request_id = None;
        s.hook(&hook);
        assert_eq!(s.status(), SessionStatus::Unconfirmed);
        hook.kind = CodexKind::Interrupt;
        s.hook(&hook);
        assert!(s.view().is_none());
        hook.kind = CodexKind::PreToolUse;
        assert!(s.hook(&hook).is_none());
        assert!(s.view().is_none());
    }

    #[test]
    fn parallel_sessions_and_out_of_order_turns_are_isolated() {
        let mut a = session();
        let mut b = session();
        b.id = "b".into();
        a.start("old");
        a.start("new");
        a.finish("old", false);
        assert_eq!(a.status(), SessionStatus::Running);
        b.start("b-turn");
        b.question("q", false, false);
        assert_eq!(b.status(), SessionStatus::Waiting);
        a.finish("new", false);
        assert!(a.view().is_none());
        assert_eq!(b.status(), SessionStatus::Waiting);
        b.mark_unconfirmed();
        assert_eq!(b.status(), SessionStatus::Unconfirmed);
        assert!(parse_record(b"null").is_err());
        assert!(parse_record(b"{\"payload\":null}").is_err());
        assert!(parse_record(&vec![b' '; 1024 * 1024 + 1]).is_err());
    }
}
