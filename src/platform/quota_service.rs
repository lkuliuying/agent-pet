use super::codex_usage::{QUERY_TIMEOUT, QueryProcess, UsageUpdate, find_codex};
use crate::usage::{self, QuotaSnapshot, UsageError};
use serde_json::{Value, json};
use std::{
    hash::{Hash, Hasher},
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, SyncSender, TrySendError},
    },
    time::{Duration, Instant},
};

fn publish(tx: &SyncSender<UsageUpdate>, mut event: UsageUpdate, stop: &AtomicBool) -> bool {
    while !stop.load(Ordering::Relaxed) {
        match tx.try_send(event) {
            Ok(()) => return true,
            Err(TrySendError::Disconnected(_)) => return false,
            Err(TrySendError::Full(value)) => event = value,
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{OnceLock, atomic::AtomicUsize, mpsc};

    fn fixture() -> &'static std::path::Path {
        static BINARY: OnceLock<std::path::PathBuf> = OnceLock::new();
        BINARY
            .get_or_init(|| {
                let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("target/quota-service-fixtures")
                    .join(std::process::id().to_string());
                std::fs::create_dir_all(&dir).unwrap();
                let binary = dir.join("fixture.exe");
                assert!(
                    Command::new("rustc")
                        .args(["--edition", "2024"])
                        .arg(
                            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                                .join("tests/support/usage_fixture.rs")
                        )
                        .arg("-o")
                        .arg(&binary)
                        .status()
                        .unwrap()
                        .success()
                );
                binary
            })
            .as_path()
    }

    fn scenario(
        mode: &'static str,
        count: usize,
        expected_error: Option<UsageError>,
        reconnect: bool,
    ) {
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let (request, requests) = mpsc::sync_channel(1);
        let (results, receive) = mpsc::sync_channel(4);
        let spawned = Arc::new(AtomicUsize::new(0));
        let spawn_count = spawned.clone();
        let path = fixture().to_path_buf();
        let pid_file = path.parent().unwrap().join(format!("{mode}.pid"));
        let worker_pid = pid_file.clone();
        let worker = std::thread::spawn(move || {
            run_with(
                worker_stop,
                requests,
                results,
                move || {
                    let n = spawn_count.fetch_add(1, Ordering::Relaxed);
                    let mut command = Command::new(&path);
                    command
                        .env(
                            "FIREFLY_FIXTURE_MODE",
                            if reconnect && n > 0 {
                                "persistent"
                            } else {
                                mode
                            },
                        )
                        .env("FIREFLY_FIXTURE_PID_FILE", &worker_pid);
                    Ok(command)
                },
                if mode == "persistent-long" {
                    Duration::from_secs(60)
                } else {
                    Duration::from_millis(150)
                },
                Duration::from_secs(8),
            )
        });
        let deadline = Instant::now() + Duration::from_secs(25);
        let mut snapshots = Vec::new();
        let mut error = None;
        let mut reset = false;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            while Instant::now() < deadline
                && (snapshots.len() < count || expected_error.is_some())
                && error.is_none()
            {
                if let Ok(update) = receive.recv_timeout(Duration::from_millis(100)) {
                    match update {
                        UsageUpdate::Refreshing => {
                            let _ = request.try_send(());
                        }
                        UsageUpdate::Reset => reset = true,
                        UsageUpdate::Complete(Ok(value)) => snapshots.push(value),
                        UsageUpdate::Complete(Err(value)) => error = Some(value),
                    }
                }
            }
            assert_eq!(error, expected_error);
            assert!(snapshots.len() >= count, "后台轮询或通知没有按期到达");
            if mode != "persistent-auth" {
                assert!(
                    snapshots.iter().any(|s| s.windows.len() == 3
                        && s.windows
                            .iter()
                            .any(|w| w.bucket_id == "codex:primary" && w.remaining == Some(70.))),
                    "通知更新优先于在途完整响应，同时保留其他额度窗口"
                );
            }
            assert_eq!(reset, reconnect);
            assert_eq!(
                spawned.load(Ordering::Relaxed),
                if reconnect { 2 } else { 1 }
            );
        }));
        stop.store(true, Ordering::Relaxed);
        worker.join().unwrap();
        let pid: u32 = std::fs::read_to_string(&pid_file).unwrap().parse().unwrap();
        use windows::Win32::System::Threading::*;
        if let Ok(handle) = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) } {
            let handle = super::super::OwnedHandle(handle);
            let mut code = 0;
            unsafe { GetExitCodeProcess(handle.0, &mut code) }.unwrap();
            assert_ne!(code, 259, "退出必须回收自己的查询进程");
        }
        std::fs::remove_file(pid_file).unwrap();
        if let Err(error) = result {
            std::panic::resume_unwind(error);
        }
    }

    #[test]
    fn background_polling_coalesces_and_reuses_process() {
        scenario("persistent", 6, None, false);
    }
    #[test]
    fn notifications_can_exceed_four_mib_over_connection_lifetime() {
        scenario("persistent-long", 33, None, false);
    }
    #[test]
    fn account_change_discards_old_responses_and_reconnects() {
        scenario("persistent-change", 3, None, true);
    }
    #[test]
    fn authentication_failure_is_reported_after_a_success() {
        scenario("persistent-auth", 3, Some(UsageError::NotLoggedIn), false);
    }
}

fn command() -> Result<Command, UsageError> {
    let path = find_codex()?;
    let mut command = Command::new(&path);
    command.args(["app-server", "--listen", "stdio://"]);
    if let Some(parent) = path.parent() {
        command.current_dir(parent);
    }
    Ok(command)
}

fn connect(
    mut command: Command,
    stop: &AtomicBool,
    deadline: Instant,
) -> Result<QueryProcess, UsageError> {
    let mut process = QueryProcess::spawn(&mut command)?;
    let cancelled = || stop.load(Ordering::Relaxed);
    process.send(json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"firefly_pet","version":env!("CARGO_PKG_VERSION")}}}))?;
    process.receive(1, deadline, &cancelled)?;
    process.send(json!({"method":"initialized","params":{}}))?;
    Ok(process)
}

fn identity(value: &Value) -> u64 {
    // 仅在内存比较账号身份摘要，原始账号字段不进入 UI、日志或配置。
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    for key in ["type", "email", "id", "organizationId"] {
        value["account"][key].as_str().hash(&mut hash);
    }
    hash.finish()
}

#[derive(Clone, Copy)]
enum Pending {
    Account(u64, Instant),
    Limits(u64, Instant, u64),
}
impl Pending {
    fn id(self) -> u64 {
        match self {
            Self::Account(id, _) | Self::Limits(id, _, _) => id,
        }
    }
    fn deadline(self) -> Instant {
        match self {
            Self::Account(_, at) | Self::Limits(_, at, _) => at,
        }
    }
}

pub fn run(stop: Arc<AtomicBool>, requests: Receiver<()>, results: SyncSender<UsageUpdate>) {
    run_with(
        stop,
        requests,
        results,
        command,
        Duration::from_secs(15),
        QUERY_TIMEOUT,
    );
}

fn run_with(
    stop: Arc<AtomicBool>,
    requests: Receiver<()>,
    results: SyncSender<UsageUpdate>,
    factory: impl Fn() -> Result<Command, UsageError>,
    interval: Duration,
    timeout: Duration,
) {
    let mut process: Option<QueryProcess> = None;
    let mut pending: Option<Pending> = None;
    let mut next = Instant::now();
    let mut sequence = 1u64;
    let mut failures = 0u32;
    let mut account = None;
    let mut snapshot: Option<QuotaSnapshot> = None;
    let mut revision = 0u64;
    let mut recent_updates: Option<QuotaSnapshot> = None;
    while !stop.load(Ordering::Relaxed) {
        let manual = requests.try_recv().is_ok();
        while requests.try_recv().is_ok() {}
        if manual && pending.is_none() {
            next = Instant::now();
        }
        let outcome = (|| -> Result<(), UsageError> {
            if pending.is_some_and(|p| Instant::now() >= p.deadline()) {
                return Err(UsageError::Timeout);
            }
            if pending.is_none() && Instant::now() >= next {
                if !publish(&results, UsageUpdate::Refreshing, &stop) {
                    return Err(UsageError::Cancelled);
                }
                let deadline = Instant::now() + timeout;
                if process.is_none() {
                    process = Some(connect(factory()?, &stop, deadline)?);
                }
                sequence += 1;
                process.as_mut().unwrap().send(
                    json!({"id":sequence,"method":"account/read","params":{"refreshToken":false}}),
                )?;
                pending = Some(Pending::Account(sequence, deadline));
            }
            let Some(pipe) = &mut process else {
                return Ok(());
            };
            let Some(value) = pipe.poll()? else {
                return Ok(());
            };
            if let Some(method) = value.get("method").and_then(Value::as_str) {
                if value.get("id").is_some() {
                    return Err(UsageError::Protocol);
                }
                match method {
                    "account/updated" if account.is_some() => {
                        snapshot = None;
                        account = None;
                        publish(&results, UsageUpdate::Reset, &stop);
                        // 切换账号时重建只读连接，旧连接的在途响应不能污染新账号。
                        return Err(UsageError::Cancelled);
                    }
                    "account/rateLimits/updated" if account.is_some() => {
                        let update = usage::parse_limits(&value["params"])?;
                        snapshot
                            .get_or_insert_with(|| QuotaSnapshot { windows: vec![] })
                            .merge(update.clone())?;
                        recent_updates
                            .get_or_insert_with(|| QuotaSnapshot { windows: vec![] })
                            .merge(update)?;
                        revision += 1;
                        publish(
                            &results,
                            UsageUpdate::Complete(Ok(snapshot.clone().unwrap())),
                            &stop,
                        );
                    }
                    _ => {}
                }
                return Ok(());
            }
            let Some(waiting) = pending else {
                return Ok(());
            };
            if value.get("id").and_then(Value::as_u64) != Some(waiting.id()) {
                return Ok(());
            }
            if let Some(error) = value.get("error") {
                return Err(usage::rpc_error(error));
            }
            let result = value.get("result").ok_or(UsageError::Protocol)?;
            match waiting {
                Pending::Account(_, deadline) => {
                    usage::check_account(result)?;
                    let current = identity(result);
                    if account.is_some_and(|old| old != current) {
                        snapshot = None;
                        publish(&results, UsageUpdate::Reset, &stop);
                    }
                    account = Some(current);
                    sequence += 1;
                    pipe.send(json!({"id":sequence,"method":"account/rateLimits/read"}))?;
                    recent_updates = None;
                    pending = Some(Pending::Limits(sequence, deadline, revision));
                }
                Pending::Limits(_, _, at_revision) => {
                    let full = usage::parse_limits(result)?;
                    if at_revision == revision {
                        snapshot = Some(full);
                    } else {
                        // 完整查询发出后收到的通知更新优先，同时补齐通知未覆盖的额度桶。
                        let newest = recent_updates.take();
                        snapshot = Some(full);
                        if let Some(newest) = newest {
                            snapshot.as_mut().unwrap().merge(newest)?;
                        }
                    }
                    pending = None;
                    failures = 0;
                    next = Instant::now() + interval;
                    publish(
                        &results,
                        UsageUpdate::Complete(Ok(snapshot.clone().unwrap())),
                        &stop,
                    );
                }
            }
            Ok(())
        })();
        if let Err(error) = outcome {
            process = None;
            pending = None;
            if matches!(
                error,
                UsageError::NotLoggedIn | UsageError::UnsupportedAccount
            ) {
                snapshot = None;
                account = None;
            }
            if error == UsageError::Cancelled {
                next = Instant::now() + Duration::from_secs(1);
            } else {
                failures = failures.saturating_add(1);
                next = Instant::now() + Duration::from_secs((1u64 << failures.min(6)).min(60));
                if !publish(&results, UsageUpdate::Complete(Err(error)), &stop) {
                    break;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}
