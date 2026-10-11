#![cfg(windows)]

use std::{
    io::Write,
    os::windows::process::CommandExt,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[test]
fn hook_process_is_neutral_and_finishes_promptly() {
    for input in [
        r#"{"hook_event_name":"Stop","session_id":"fixture-process","turn_id":"test-stop","last_assistant_message":"discard this body"}"#,
        r#"{"hook_event_name":"PermissionRequest","session_id":"fixture-process","turn_id":"test-approval","tool_input":{"command":"not executed"}}"#,
        r#"{"hook_event_name":"Interrupt","session_id":"fixture-process","turn_id":"test-interrupt"}"#,
        "{broken",
    ] {
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
}
