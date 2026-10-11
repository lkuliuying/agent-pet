#![cfg(windows)]

use pet_desktop::{
    bridge::{CodexEvent, CodexKind},
    platform::ipc,
};
use std::{
    io::Write,
    os::windows::process::CommandExt,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

struct Receiver {
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<Result<(), String>>>,
    events: mpsc::Receiver<CodexEvent>,
}
impl Receiver {
    fn start() -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, events) = mpsc::sync_channel(8);
        let worker_stop = stop.clone();
        let receiver = Self {
            stop,
            worker: Some(std::thread::spawn(move || ipc::listen(worker_stop, tx))),
            events,
        };
        let ready = CodexEvent {
            version: 1,
            kind: CodexKind::Stop,
            session_id: "fixture-ready".into(),
            turn_id: "ready".into(),
            simulated: true,
            ..Default::default()
        };
        ipc::send(&ready).expect("接收端应在发送期限内就绪");
        assert!(
            receiver
                .events
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
                .simulated
        );
        receiver
    }
}
impl Drop for Receiver {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let result = self.worker.take().unwrap().join();
        if !std::thread::panicking() {
            result.unwrap().unwrap();
        }
    }
}

#[test]
fn hook_process_delivers_minimal_events_and_remains_neutral() {
    // 占用生产单实例锁，防止夹具进入用户桌宠或被误当作真实会话证据。
    let _instance = ipc::single_instance()
        .unwrap()
        .expect("请先退出桌宠再运行 hook 进程测试");
    let receiver = Receiver::start();
    let inputs = [
        r#"{"hook_event_name":"Stop","session_id":"fixture-process","turn_id":"test-stop","last_assistant_message":"discard this body"}"#,
        r#"{"hook_event_name":"PermissionRequest","session_id":"fixture-process","turn_id":"test-approval","tool_input":{"command":"not executed"}}"#,
        r#"{"hook_event_name":"Interrupt","session_id":"fixture-process","turn_id":"test-interrupt"}"#,
        r#"{"hook_event_name":"SessionStart","session_id":"fixture-process"}"#,
        r#"{"hook_event_name":"UserPromptSubmit","session_id":"fixture-process","turn_id":"test-start","prompt":"discard"}"#,
        r#"{"hook_event_name":"PreToolUse","session_id":"fixture-process","turn_id":"test-tool","tool_use_id":"call","tool_name":"exec"}"#,
        r#"{"hook_event_name":"PostToolUse","session_id":"fixture-process","turn_id":"test-tool","tool_use_id":"call","tool_name":"exec","tool_response":"discard"}"#,
        r#"{"hook_event_name":"SessionEnd","session_id":"fixture-process"}"#,
    ];
    for (input, kind) in inputs.iter().zip([
        CodexKind::Stop,
        CodexKind::PermissionRequest,
        CodexKind::Interrupt,
        CodexKind::SessionStart,
        CodexKind::UserPromptSubmit,
        CodexKind::PreToolUse,
        CodexKind::PostToolUse,
        CodexKind::SessionEnd,
    ]) {
        run_hook(input);
        let event = receiver
            .events
            .recv_timeout(Duration::from_secs(2))
            .expect("中立退出不等于投递成功，必须收到对应事件");
        assert_eq!(event.kind, kind);
        assert_eq!(event.session_id, "fixture-process");
        assert!(!event.simulated);
        let wire = serde_json::to_string(&event).unwrap();
        assert!(!wire.contains("discard"));
        assert!(!wire.contains("command"));
    }
    for input in [
        "{broken",
        "{}",
        r#"{"hook_event_name":"Stop","session_id":"fixture-process"}"#,
    ] {
        run_hook(input);
        assert!(
            receiver
                .events
                .recv_timeout(Duration::from_millis(60))
                .is_err()
        );
    }
    drop(receiver);
    for input in inputs {
        run_hook(input);
    }
}

fn run_hook(input: &str) {
    let start = Instant::now();
    let mut child = Command::new(env!("CARGO_BIN_EXE_PetDesktop"))
        .arg("--codex-hook")
        .creation_flags(0x08000000)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    while child.try_wait().unwrap().is_none() {
        if start.elapsed() > Duration::from_secs(3) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("hook 未在限定时间内退出");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "{}");
    assert!(output.stderr.is_empty());
}
