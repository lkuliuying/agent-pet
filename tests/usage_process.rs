#![cfg(windows)]

use pet_desktop::platform::OwnedHandle;
use pet_desktop::{
    platform::codex_usage::{self, query_process},
    usage::UsageError,
};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
use windows::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn fixture_binary() -> &'static std::path::Path {
    static BINARY: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    BINARY
        .get_or_init(|| {
            let output = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target/usage-fixtures")
                .join(format!(
                    "{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                ));
            fs::create_dir_all(&output).unwrap();
            let binary = output.join("usage-fixture.exe");
            let status = Command::new("rustc")
                .args(["--edition", "2024", "--crate-name", "usage_fixture"])
                .arg(
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("tests/support/usage_fixture.rs"),
                )
                .arg("-o")
                .arg(&binary)
                .status()
                .unwrap();
            assert!(status.success(), "无法编译只使用标准库的进程夹具");
            binary
        })
        .as_path()
}

struct Fixture {
    directory: PathBuf,
    command: Command,
}

impl Fixture {
    fn new(mode: &str) -> Self {
        let id = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let directory =
            std::env::temp_dir().join(format!("firefly-quota-fixture-{}-{id}", std::process::id()));
        fs::create_dir(&directory).unwrap();
        let mut command = Command::new(fixture_binary());
        command
            .env("FIREFLY_FIXTURE_MODE", mode)
            .env("FIREFLY_FIXTURE_PID_FILE", directory.join("parent.pid"))
            .env(
                "FIREFLY_FIXTURE_CHILD_PID_FILE",
                directory.join("child.pid"),
            );
        Self { directory, command }
    }

    fn assert_reaped(&self) {
        for file in ["parent.pid", "child.pid"] {
            let path = self.directory.join(file);
            if let Ok(text) = fs::read_to_string(&path) {
                let pid = text.trim().parse::<u32>().unwrap();
                let until = Instant::now() + Duration::from_secs(2);
                loop {
                    let running =
                        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }
                            .ok()
                            .map(OwnedHandle)
                            .is_some_and(|handle| {
                                let mut code = 0;
                                unsafe { GetExitCodeProcess(handle.0, &mut code) }.is_ok()
                                    && code == 259
                            });
                    if !running {
                        break;
                    }
                    assert!(Instant::now() < until, "查询夹具进程未回收");
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for name in ["parent.pid", "child.pid"] {
            let _ = fs::remove_file(self.directory.join(name));
        }
        let _ = fs::remove_dir(&self.directory);
    }
}

#[test]
fn account_protocol_order_and_owned_process_tree_are_verified() {
    for mode in ["success", "descendant"] {
        let mut fixture = Fixture::new(mode);
        let result = query_process(&mut fixture.command, Duration::from_secs(8), || false).unwrap();
        assert_eq!(result.windows.len(), 2);
        assert_eq!(result.windows[0].remaining, Some(75.));
        assert_eq!(result.windows[1].remaining, Some(58.));
        assert!(fixture.directory.join("parent.pid").is_file());
        if mode == "descendant" {
            assert!(fixture.directory.join("child.pid").is_file());
        }
        fixture.assert_reaped();
    }
}

#[test]
fn query_failures_are_explicit_and_reap_the_child() {
    for (mode, error) in [
        ("signed-out", UsageError::NotLoggedIn),
        ("api-key", UsageError::UnsupportedAccount),
        ("network", UsageError::Network),
        ("invalid", UsageError::Protocol),
        ("oversize", UsageError::Protocol),
        ("exit", UsageError::Process),
    ] {
        let mut fixture = Fixture::new(mode);
        assert_eq!(
            query_process(&mut fixture.command, Duration::from_secs(8), || false),
            Err(error),
            "模式 {mode}"
        );
        fixture.assert_reaped();
    }
}

#[test]
fn timeout_and_cancellation_are_bounded() {
    let mut timeout = Fixture::new("timeout");
    let started = Instant::now();
    assert_eq!(
        query_process(&mut timeout.command, Duration::from_millis(1500), || false),
        Err(UsageError::Timeout)
    );
    assert!(started.elapsed() < Duration::from_secs(4));
    timeout.assert_reaped();
    let mut cancelled = Fixture::new("timeout");
    let pid_file = cancelled.directory.join("parent.pid");
    let started = Instant::now();
    assert_eq!(
        query_process(&mut cancelled.command, Duration::from_secs(8), || pid_file
            .is_file()),
        Err(UsageError::Cancelled)
    );
    assert!(started.elapsed() < Duration::from_secs(4));
    cancelled.assert_reaped();
}

#[test]
#[ignore = "需要本机 Codex 登录和网络，仅手动运行"]
fn real_codex_quota_read_only() {
    let snapshot = codex_usage::query(|| false).unwrap();
    assert!(!snapshot.windows.is_empty());
    println!(
        "本机只读额度查询成功，返回 {} 个额度窗口；未输出账号或原始响应。",
        snapshot.windows.len()
    );
}
