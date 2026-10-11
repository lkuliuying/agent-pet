use super::OwnedHandle;
use crate::usage::{self, QuotaSnapshot, UsageError};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    os::windows::{io::AsRawHandle, process::CommandExt},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{Receiver, RecvTimeoutError, SyncSender, TrySendError},
    },
    time::{Duration, Instant},
};
use windows::{
    Win32::{
        Foundation::{FILETIME, HANDLE, SYSTEMTIME},
        System::{
            JobObjects::*,
            Pipes::PeekNamedPipe,
            Time::{FileTimeToSystemTime, SystemTimeToTzSpecificLocalTime},
        },
    },
    core::PCWSTR,
};

const MAX_FRAME: usize = 1024 * 1024;
const MAX_OUTPUT: usize = 4 * MAX_FRAME;
pub const QUERY_TIMEOUT: Duration = Duration::from_secs(10);

pub struct QueryResult {
    pub id: u64,
    pub result: Result<QuotaSnapshot, UsageError>,
}

pub fn find_codex() -> Result<PathBuf, UsageError> {
    let paths = std::env::var_os("PATH").ok_or(UsageError::MissingCodex)?;
    std::env::split_paths(&paths)
        .filter(|p| p.is_absolute())
        .map(|p| p.join("codex.exe"))
        .filter(|p| p.is_file())
        .find_map(|p| p.canonicalize().ok())
        .ok_or(UsageError::MissingCodex)
}

struct QueryProcess {
    child: Child,
    job: OwnedHandle,
    buffer: Vec<u8>,
    stdout_bytes: usize,
    stderr_bytes: usize,
}

impl QueryProcess {
    fn spawn(command: &mut Command) -> Result<Self, UsageError> {
        let job = OwnedHandle(
            unsafe { CreateJobObjectW(None, PCWSTR::null()) }.map_err(|_| UsageError::Process)?,
        );
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        unsafe {
            SetInformationJobObject(
                job.0,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&limits) as u32,
            )
        }
        .map_err(|_| UsageError::Process)?;
        let mut child = command
            .creation_flags(0x08000000)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| UsageError::Process)?;
        // 作业只包含本次创建的查询进程及其后代，不按名称结束用户的 Codex。
        if unsafe { AssignProcessToJobObject(job.0, HANDLE(child.as_raw_handle())) }.is_err() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(UsageError::Process);
        }
        Ok(Self {
            child,
            job,
            buffer: Vec::new(),
            stdout_bytes: 0,
            stderr_bytes: 0,
        })
    }

    fn send(&mut self, value: Value) -> Result<(), UsageError> {
        let input = self.child.stdin.as_mut().ok_or(UsageError::Process)?;
        let mut bytes = serde_json::to_vec(&value).map_err(|_| UsageError::Protocol)?;
        bytes.push(b'\n');
        input
            .write_all(&bytes)
            .and_then(|_| input.flush())
            .map_err(|_| UsageError::Process)
    }

    fn receive(
        &mut self,
        id: u64,
        deadline: Instant,
        cancelled: &impl Fn() -> bool,
    ) -> Result<Value, UsageError> {
        loop {
            if cancelled() {
                return Err(UsageError::Cancelled);
            }
            if Instant::now() >= deadline {
                return Err(UsageError::Timeout);
            }
            if let Some(stderr) = self.child.stderr.as_mut() {
                let bytes = available_bytes(stderr)?;
                self.stderr_bytes += bytes.len();
                if self.stderr_bytes > MAX_OUTPUT {
                    return Err(UsageError::Process);
                }
            }
            if let Some(stdout) = self.child.stdout.as_mut() {
                let bytes = available_bytes(stdout)?;
                self.stdout_bytes += bytes.len();
                if self.stdout_bytes > MAX_OUTPUT {
                    return Err(UsageError::Protocol);
                }
                self.buffer.extend_from_slice(&bytes);
            }
            while let Some(end) = self.buffer.iter().position(|b| *b == b'\n') {
                if end > MAX_FRAME {
                    return Err(UsageError::Protocol);
                }
                let line: Vec<u8> = self.buffer.drain(..=end).collect();
                if line.iter().all(u8::is_ascii_whitespace) {
                    continue;
                }
                let value: Value =
                    serde_json::from_slice(&line).map_err(|_| UsageError::Protocol)?;
                if value.get("id").and_then(Value::as_u64) == Some(id)
                    && value.get("method").is_none()
                {
                    if let Some(error) = value.get("error") {
                        return Err(usage::rpc_error(error));
                    }
                    return value.get("result").cloned().ok_or(UsageError::Protocol);
                }
                if value.get("method").and_then(Value::as_str).is_none() {
                    return Err(UsageError::Protocol);
                }
                // 此连接不提供令牌或审批能力；遇到额外请求直接结束查询，防止阻塞写入。
                if value.get("id").is_some() {
                    return Err(UsageError::Protocol);
                }
            }
            if self.buffer.len() > MAX_FRAME {
                return Err(UsageError::Protocol);
            }
            if self
                .child
                .try_wait()
                .map_err(|_| UsageError::Process)?
                .is_some()
            {
                return Err(UsageError::Process);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for QueryProcess {
    fn drop(&mut self) {
        self.child.stdin.take();
        // 查询结束后回收整个自建作业，避免 CLI 辅助进程继承管道而阻止退出。
        unsafe {
            let _ = TerminateJobObject(self.job.0, 0);
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn available_bytes(pipe: &mut (impl AsRawHandle + Read)) -> Result<Vec<u8>, UsageError> {
    let mut available = 0;
    if let Err(error) = unsafe {
        PeekNamedPipe(
            HANDLE(pipe.as_raw_handle()),
            None,
            0,
            None,
            Some(&mut available),
            None,
        )
    } {
        // 对端退出时已读完的匿名管道会报告断开，由主循环处理退出状态。
        if error.code() == windows::Win32::Foundation::ERROR_BROKEN_PIPE.to_hresult() {
            return Ok(Vec::new());
        }
        return Err(UsageError::Process);
    }
    let mut bytes = vec![0; (available as usize).min(8192)];
    if !bytes.is_empty() {
        let read = pipe.read(&mut bytes).map_err(|_| UsageError::Process)?;
        bytes.truncate(read);
    }
    Ok(bytes)
}

pub fn query_process(
    command: &mut Command,
    timeout: Duration,
    cancelled: impl Fn() -> bool,
) -> Result<QuotaSnapshot, UsageError> {
    if cancelled() {
        return Err(UsageError::Cancelled);
    }
    let deadline = Instant::now() + timeout;
    let mut process = QueryProcess::spawn(command)?;
    process.send(json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"firefly_pet","title":"Firefly Pet","version":env!("CARGO_PKG_VERSION")}}}))?;
    process.receive(1, deadline, &cancelled)?;
    process.send(json!({"method":"initialized","params":{}}))?;
    process.send(json!({"id":2,"method":"account/read","params":{"refreshToken":false}}))?;
    usage::check_account(&process.receive(2, deadline, &cancelled)?)?;
    process.send(json!({"id":3,"method":"account/rateLimits/read"}))?;
    usage::parse_limits(&process.receive(3, deadline, &cancelled)?)
}

pub fn query(cancelled: impl Fn() -> bool) -> Result<QuotaSnapshot, UsageError> {
    let path = find_codex()?;
    let mut command = Command::new(&path);
    command.args(["app-server", "--listen", "stdio://"]);
    // 不从桌宠所在项目加载工作区配置；账号仍由 Codex 自己的登录环境决定。
    if let Some(parent) = path.parent() {
        command.current_dir(parent);
    }
    query_process(&mut command, QUERY_TIMEOUT, cancelled)
}

pub fn run(
    stop: Arc<AtomicBool>,
    generation: Arc<AtomicU64>,
    requests: Receiver<()>,
    results: SyncSender<QueryResult>,
) {
    while !stop.load(Ordering::Relaxed) {
        match requests.recv_timeout(Duration::from_millis(100)) {
            Ok(()) => {}
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        }
        let id = generation.load(Ordering::Acquire);
        if id == 0 {
            continue;
        }
        let cancelled = || stop.load(Ordering::Relaxed) || generation.load(Ordering::Acquire) != id;
        let mut event = QueryResult {
            id,
            result: query(cancelled),
        };
        while !cancelled() {
            match results.try_send(event) {
                Ok(()) | Err(TrySendError::Disconnected(_)) => break,
                Err(TrySendError::Full(pending)) => {
                    event = pending;
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
        }
    }
}

pub fn local_time(unix_seconds: i64) -> Option<String> {
    let ticks = unix_seconds
        .checked_add(11_644_473_600)?
        .checked_mul(10_000_000)?;
    let ticks = u64::try_from(ticks).ok()?;
    let filetime = FILETIME {
        dwLowDateTime: ticks as u32,
        dwHighDateTime: (ticks >> 32) as u32,
    };
    let mut utc = SYSTEMTIME::default();
    let mut local = SYSTEMTIME::default();
    unsafe {
        FileTimeToSystemTime(&filetime, &mut utc).ok()?;
        SystemTimeToTzSpecificLocalTime(None, &utc, &mut local).ok()?;
    }
    if !(1601..=9999).contains(&local.wYear) {
        return None;
    }
    Some(format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        local.wYear, local.wMonth, local.wDay, local.wHour, local.wMinute
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_times_validate_overflow_and_use_local_calendar() {
        let formatted = local_time(1_900_000_000).unwrap();
        assert_eq!(formatted.len(), 16);
        assert_eq!(&formatted[..4], "2030");
        assert!(local_time(i64::MAX).is_none());
        assert!(local_time(i64::MIN).is_none());
    }
}
