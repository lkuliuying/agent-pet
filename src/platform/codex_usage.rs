use super::OwnedHandle;
use crate::usage::{self, QuotaSnapshot, UsageError};
use serde_json::{Value, json};
use std::{
    ffi::OsStr,
    io::{Read, Write},
    os::windows::{io::AsRawHandle, process::CommandExt},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::AtomicBool,
        mpsc::{Receiver, SyncSender},
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
pub const QUERY_TIMEOUT: Duration = Duration::from_secs(10);

pub enum UsageUpdate {
    Refreshing,
    Reset,
    Complete(Result<QuotaSnapshot, UsageError>),
}

pub fn find_codex() -> Result<PathBuf, UsageError> {
    let paths = std::env::var_os("PATH").unwrap_or_default();
    let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    find_codex_in(&paths, local.as_deref())
}

fn find_codex_in(paths: &OsStr, local: Option<&Path>) -> Result<PathBuf, UsageError> {
    if let Some(path) = std::env::split_paths(paths)
        .filter(|p| p.is_absolute())
        .map(|p| p.join("codex.exe"))
        .filter(|p| p.is_file())
        .find_map(|p| p.canonicalize().ok())
    {
        return Ok(path);
    }
    // Desktop 的运行目录随更新变化；只检查其直接版本子目录，跳过备份和残留空目录。
    let root = local
        .filter(|p| p.is_absolute())
        .ok_or(UsageError::MissingCodex)?
        .join("OpenAI/Codex/bin");
    let entries = std::fs::read_dir(root).map_err(|_| UsageError::MissingCodex)?;
    let mut candidates = Vec::new();
    for entry in entries.take(256).flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if name.len() != 16 || !name.bytes().all(|c| c.is_ascii_hexdigit()) {
            continue;
        }
        let path = entry.path().join("codex.exe");
        if let Ok(metadata) = path.metadata()
            && metadata.is_file()
            && let Ok(path) = path.canonicalize()
        {
            candidates.push((metadata.modified().unwrap_or(std::time::UNIX_EPOCH), path));
        }
    }
    candidates.sort_by(|a, b| b.cmp(a));
    candidates
        .into_iter()
        .next()
        .map(|(_, path)| path)
        .ok_or(UsageError::MissingCodex)
}

pub(crate) struct QueryProcess {
    child: Child,
    job: OwnedHandle,
    buffer: Vec<u8>,
}

impl QueryProcess {
    pub(crate) fn capture(command: &mut Command, stop: &AtomicBool) -> Result<Value, String> {
        let mut process = Self::spawn(command).map_err(|e| e.message().to_string())?;
        process.child.stdin.take();
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut output = Vec::new();
        loop {
            if stop.load(std::sync::atomic::Ordering::Relaxed) {
                return Err("操作已取消".into());
            }
            if Instant::now() >= deadline {
                return Err("CLI 操作超过 30 秒".into());
            }
            let bytes = available_bytes(process.child.stdout.as_mut().ok_or("CLI 输出不可用")?)
                .map_err(|_| "读取 CLI 输出失败")?;
            let drained = bytes.is_empty();
            output.extend(bytes);
            if output.len() > 4 * 1024 * 1024 {
                return Err("CLI 响应过大，版本可能不兼容".into());
            }
            if let Some(stderr) = process.child.stderr.as_mut() {
                available_bytes(stderr).map_err(|_| "读取 CLI 错误通道失败")?;
            }
            if let Some(status) = process
                .child
                .try_wait()
                .map_err(|_| "读取 CLI 退出状态失败")?
                && drained
            {
                if !status.success() {
                    return Err(format!(
                        "CLI 退出码 {}，请检查目录权限或 Codex CLI 版本",
                        status.code().unwrap_or(-1)
                    ));
                }
                return serde_json::from_slice(&output).map_err(|_| "CLI 响应格式不兼容".into());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    pub(crate) fn spawn(command: &mut Command) -> Result<Self, UsageError> {
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
        })
    }

    pub(crate) fn send(&mut self, value: Value) -> Result<(), UsageError> {
        let input = self.child.stdin.as_mut().ok_or(UsageError::Process)?;
        let mut bytes = serde_json::to_vec(&value).map_err(|_| UsageError::Protocol)?;
        bytes.push(b'\n');
        input
            .write_all(&bytes)
            .and_then(|_| input.flush())
            .map_err(|_| UsageError::Process)
    }

    pub(crate) fn receive(
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
            if let Some(value) = self.poll()? {
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
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    pub(crate) fn poll(&mut self) -> Result<Option<Value>, UsageError> {
        // 每次仅排空有限数据；限制未完成帧，不对常驻连接累计字节数设上限。
        if let Some(stderr) = self.child.stderr.as_mut() {
            let _ = available_bytes(stderr)?;
        }
        if !self.buffer.contains(&b'\n')
            && let Some(stdout) = self.child.stdout.as_mut()
        {
            self.buffer.extend(available_bytes(stdout)?);
        }
        if let Some(end) = self.buffer.iter().position(|b| *b == b'\n') {
            if end > MAX_FRAME {
                return Err(UsageError::Protocol);
            }
            let line: Vec<_> = self.buffer.drain(..=end).collect();
            if line.iter().all(u8::is_ascii_whitespace) {
                return Ok(None);
            }
            return serde_json::from_slice(&line)
                .map(Some)
                .map_err(|_| UsageError::Protocol);
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
        Ok(None)
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

pub fn run(stop: Arc<AtomicBool>, requests: Receiver<()>, results: SyncSender<UsageUpdate>) {
    super::quota_service::run(stop, requests, results);
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
    fn desktop_discovery_works_without_path_and_skips_backups() {
        let root = std::env::temp_dir().join(format!(
            "firefly-discovery-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let bin = root.join("OpenAI/Codex/bin");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir_all(bin.join("ffffffffffffffff.bak")).unwrap();
        std::fs::write(bin.join("ffffffffffffffff.bak/codex.exe"), b"backup").unwrap();
        std::fs::create_dir_all(bin.join("1111111111111111")).unwrap();
        assert_eq!(
            find_codex_in(OsStr::new(""), Some(&root)),
            Err(UsageError::MissingCodex)
        );
        let executable = bin.join("2222222222222222/codex.exe");
        std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
        std::fs::write(&executable, b"fixture").unwrap();
        assert_eq!(
            find_codex_in(OsStr::new("relative;missing"), Some(&root)).unwrap(),
            executable.canonicalize().unwrap()
        );
        let older = bin.join("aaaaaaaaaaaaaaaa/codex.exe");
        std::fs::create_dir_all(older.parent().unwrap()).unwrap();
        std::fs::write(&older, b"old version").unwrap();
        std::fs::File::options()
            .write(true)
            .open(&older)
            .unwrap()
            .set_times(
                std::fs::FileTimes::new()
                    .set_modified(std::time::UNIX_EPOCH + Duration::from_secs(1_000_000)),
            )
            .unwrap();
        assert_eq!(
            find_codex_in(OsStr::new(""), Some(&root)).unwrap(),
            executable.canonicalize().unwrap()
        );
        let cli = root.join("独立 CLI 空格目录");
        std::fs::create_dir(&cli).unwrap();
        std::fs::write(cli.join("codex.exe"), b"cli").unwrap();
        assert_eq!(
            find_codex_in(&std::env::join_paths([&cli]).unwrap(), Some(&root)).unwrap(),
            cli.join("codex.exe").canonicalize().unwrap()
        );
        assert_eq!(
            find_codex_in(OsStr::new(""), Some(Path::new("relative"))),
            Err(UsageError::MissingCodex)
        );
        assert_eq!(
            find_codex_in(OsStr::new(""), None),
            Err(UsageError::MissingCodex)
        );
        assert!(
            root.canonicalize()
                .unwrap()
                .starts_with(std::env::temp_dir().canonicalize().unwrap())
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "需要本机 Codex Desktop 登录和网络，仅手动运行"]
    fn real_desktop_without_path_read_only() {
        let local = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap());
        let path = find_codex_in(OsStr::new(""), Some(&local)).unwrap();
        let mut command = Command::new(&path);
        command
            .args(["app-server", "--listen", "stdio://"])
            .current_dir(path.parent().unwrap());
        let snapshot = query_process(&mut command, QUERY_TIMEOUT, || false).unwrap();
        assert!(!snapshot.windows.is_empty());
        println!(
            "无 Codex PATH 时，Desktop 自动发现及真实只读额度查询成功，返回 {} 个额度窗口；未输出账号或原始响应。",
            snapshot.windows.len()
        );
    }

    #[test]
    fn reset_times_validate_overflow_and_use_local_calendar() {
        let formatted = local_time(1_900_000_000).unwrap();
        assert_eq!(formatted.len(), 16);
        assert_eq!(&formatted[..4], "2030");
        assert!(local_time(i64::MAX).is_none());
        assert!(local_time(i64::MIN).is_none());
    }
}
