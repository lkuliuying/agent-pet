use crate::{
    bridge::CodexEvent,
    notices::Notice,
    sessions::{self, Session, SessionStatus, SessionView},
};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs::{self, File},
    io::{BufReader, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, SyncSender, TrySendError},
    },
    time::{Duration, Instant, SystemTime},
};

const FRAME_LIMIT: usize = 1024 * 1024;
const INITIAL_TAIL: u64 = 4 * 1024 * 1024;
const MAX_FILES: usize = 1024;

pub struct SessionUpdate {
    pub rows: Vec<SessionView>,
    pub notices: Vec<Notice>,
    pub status: String,
    pub hook_status: Option<String>,
}

pub fn codex_home() -> Option<PathBuf> {
    std::env::var_os("CODEX_HOME")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(|p| PathBuf::from(p).join(".codex")))
        .filter(|p| p.is_absolute())
}

struct Log {
    path: PathBuf,
    offset: u64,
    buffer: Vec<u8>,
    discard_line: bool,
    session: Session,
    last_growth: Instant,
    incompatible: bool,
    large_pending: Option<(u64, u64)>,
}

impl Log {
    fn open(path: PathBuf, live: bool, notices: &mut Vec<Notice>) -> std::io::Result<Self> {
        let mut file = File::open(&path)?;
        let mut head = Vec::new();
        (&mut file)
            .take(FRAME_LIMIT as u64)
            .read_to_end(&mut head)?;
        let end = head
            .iter()
            .position(|b| *b == b'\n')
            .ok_or_else(|| std::io::Error::other("缺少元数据帧"))?;
        let mut session = Session::default();
        let record = sessions::parse_record(&head[..end]).map_err(std::io::Error::other)?;
        if record.kind != "session_meta"
            || record.payload.originator == "Codex Desktop"
                && record.payload.source.as_str() != Some("vscode")
                && record.payload.source.get("subagent").is_none()
        {
            return Err(std::io::Error::other("Desktop 元数据版本不兼容"));
        }
        session.record(record, false);
        let length = file.metadata()?.len();
        let offset = length.saturating_sub(INITIAL_TAIL);
        let mut log = Self {
            path,
            offset,
            buffer: Vec::new(),
            discard_line: offset > 0,
            session,
            last_growth: Instant::now(),
            incompatible: false,
            large_pending: None,
        };
        // 初始化最多读取尾部四 MiB；无法重建的旧状态保持待确认，不读取完整对话历史。
        for _ in 0..5 {
            if !log.read(live, notices)? {
                break;
            }
        }
        if offset > 0 {
            log.session.check_reconstruction();
        }
        Ok(log)
    }

    fn read(&mut self, live: bool, notices: &mut Vec<Notice>) -> std::io::Result<bool> {
        let mut file = File::open(&self.path)?;
        let length = file.metadata()?.len();
        if length < self.offset {
            self.offset = 0;
            self.buffer.clear();
            self.discard_line = false;
            self.session = Session::default();
            self.large_pending = None;
        }
        if let Some((start, previous_length)) = self.large_pending {
            if previous_length == length {
                return Ok(false);
            }
            return self.read_large(start, length, live, notices);
        }
        if length == self.offset {
            return Ok(false);
        }
        file.seek(SeekFrom::Start(self.offset))?;
        let mut bytes = Vec::new();
        file.take(FRAME_LIMIT as u64).read_to_end(&mut bytes)?;
        self.offset += bytes.len() as u64;
        self.last_growth = Instant::now();
        if self.discard_line {
            let Some(end) = bytes.iter().position(|b| *b == b'\n') else {
                return Ok(true);
            };
            bytes.drain(..=end);
            self.discard_line = false;
        }
        self.buffer.extend(bytes);
        while let Some(end) = self.buffer.iter().position(|b| *b == b'\n') {
            if end > FRAME_LIMIT {
                let start = self.offset - self.buffer.len() as u64;
                self.buffer.clear();
                return self.read_large(start, length, live, notices);
            }
            let line: Vec<_> = self.buffer.drain(..=end).collect();
            if line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            match sessions::parse_record(&line) {
                Ok(record) => {
                    if let Some(notice) = self.session.record(record, live)
                        && notices.len() < 64
                    {
                        notices.push(notice);
                    }
                }
                Err(_) => {
                    self.incompatible = true;
                    self.session.mark_unconfirmed();
                }
            }
        }
        if self.buffer.len() > FRAME_LIMIT {
            let start = self.offset - self.buffer.len() as u64;
            self.buffer.clear();
            return self.read_large(start, length, live, notices);
        }
        Ok(true)
    }

    fn read_large(
        &mut self,
        start: u64,
        length: u64,
        live: bool,
        notices: &mut Vec<Notice>,
    ) -> std::io::Result<bool> {
        // 图片等正文可能很大；流式跳过非状态字段，避免把整个记录缓存在内存。
        let mut file = File::open(&self.path)?;
        file.seek(SeekFrom::Start(start))?;
        let mut stream =
            serde_json::Deserializer::from_reader(BufReader::new(file.take(64 * 1024 * 1024)))
                .into_iter::<sessions::Record>();
        match stream.next() {
            Some(Ok(record)) => {
                self.offset = start + stream.byte_offset() as u64;
                self.large_pending = None;
                if let Some(notice) = self.session.record(record, live)
                    && notices.len() < 64
                {
                    notices.push(notice);
                }
            }
            Some(Err(error)) if error.is_eof() && length - start < 64 * 1024 * 1024 => {
                self.large_pending = Some((start, length));
            }
            _ => {
                self.large_pending = None;
                self.discard_line = true;
                self.incompatible = true;
                self.session.mark_unconfirmed();
            }
        }
        Ok(true)
    }
}

fn discover(directory: &Path, since: SystemTime) -> std::io::Result<Vec<PathBuf>> {
    let mut directories = vec![(directory.to_path_buf(), 0)];
    let mut candidates = Vec::new();
    while let Some((dir, depth)) = directories.pop() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            use std::os::windows::fs::MetadataExt;
            let metadata = entry.metadata()?;
            // 不跟随链接或目录联接，读取范围必须留在本机 sessions 树内。
            if metadata.file_attributes() & 0x400 != 0 {
                continue;
            }
            if kind.is_dir() && depth < 3 {
                directories.push((entry.path(), depth + 1));
            }
            if kind.is_file() && entry.path().extension().is_some_and(|s| s == "jsonl") {
                let modified = metadata.modified()?;
                if modified >= since {
                    candidates.push((modified, entry.path()));
                }
            }
        }
    }
    candidates.sort_by_key(|entry| std::cmp::Reverse(entry.0));
    Ok(candidates.into_iter().map(|(_, p)| p).collect())
}

#[derive(Deserialize)]
struct Title {
    id: String,
    thread_name: String,
}
#[derive(Default)]
struct Titles {
    offset: u64,
    buffer: Vec<u8>,
    names: BTreeMap<String, String>,
    discard_line: bool,
}
impl Titles {
    fn read(&mut self, root: &Path) -> std::io::Result<()> {
        let mut file = File::open(root.join("session_index.jsonl"))?;
        let length = file.metadata()?.len();
        if length < self.offset {
            *self = Self::default();
        }
        if self.offset == 0 && length > INITIAL_TAIL {
            self.offset = length - INITIAL_TAIL;
            self.discard_line = true;
        }
        file.seek(SeekFrom::Start(self.offset))?;
        let mut bytes = Vec::new();
        file.take(INITIAL_TAIL).read_to_end(&mut bytes)?;
        self.offset += bytes.len() as u64;
        self.buffer.extend(bytes);
        while let Some(end) = self.buffer.iter().position(|b| *b == b'\n') {
            let line: Vec<_> = self.buffer.drain(..=end).collect();
            if self.discard_line {
                self.discard_line = false;
                continue;
            }
            if line.len() <= 16384
                && let Ok(title) = serde_json::from_slice::<Title>(&line)
            {
                self.names.insert(
                    sessions::clean_label(&title.id, 128),
                    sessions::clean_label(&title.thread_name, 160),
                );
                if self.names.len() > 4096 {
                    self.names.pop_first();
                }
            }
        }
        if self.buffer.len() > 16384 {
            self.buffer.clear();
            self.discard_line = true;
        }
        Ok(())
    }
}

pub fn run(stop: Arc<AtomicBool>, hooks: Receiver<CodexEvent>, updates: SyncSender<SessionUpdate>) {
    let Some(root) = codex_home() else {
        let _ = updates.try_send(SessionUpdate {
            rows: vec![],
            notices: vec![],
            status: "无法定位 Codex 配置目录。".into(),
            hook_status: None,
        });
        return;
    };
    let mut logs: BTreeMap<PathBuf, Log> = BTreeMap::new();
    let mut pending_hooks = VecDeque::new();
    let mut next_discovery = Instant::now();
    let mut next_tick = Instant::now();
    let mut titles = Titles::default();
    let mut ignored_paths = BTreeSet::new();
    let mut health = String::new();
    let mut desktop_epoch = super::codex_presence::desktop_started();
    let mut first = true;
    let mut changes = super::session_changes::Changes::open(&root.join("sessions")).ok();
    let mut changed_paths = BTreeSet::new();
    let mut rescan = true;
    while !stop.load(Ordering::Relaxed) {
        let mut notices = Vec::new();
        let mut hook_status = None;
        let mut changed = false;
        if let Some(watcher) = &mut changes {
            match watcher.poll() {
                Ok(Some(paths)) => changed_paths.extend(paths),
                Ok(None) => {}
                Err(_) => {
                    changes = None;
                    rescan = true;
                }
            }
        }
        if changed_paths.len() > 4096 {
            changed_paths.clear();
            rescan = true;
        }
        while let Ok(event) = hooks.try_recv() {
            if event.simulated {
                hook_status = Some("已收到管道模拟事件，不能据此判断真实接入。".into());
                notices.push(event.notice());
            } else {
                if pending_hooks.len() >= 128 {
                    pending_hooks.pop_front();
                }
                pending_hooks.push_back((Instant::now(), event));
            }
            changed = true;
        }
        if Instant::now() >= next_discovery {
            let previous_epoch = desktop_epoch;
            desktop_epoch = super::codex_presence::desktop_started();
            if desktop_epoch != previous_epoch && !first {
                for log in logs.values_mut() {
                    log.session.mark_unconfirmed();
                }
            }
            let since =
                desktop_epoch.unwrap_or_else(|| SystemTime::now() - Duration::from_secs(7 * 86400));
            if changes.is_none() {
                changes = super::session_changes::Changes::open(&root.join("sessions")).ok();
                rescan = true;
            }
            let discovered = if rescan {
                changed_paths.clear();
                discover(&root.join("sessions"), since)
            } else {
                Ok(std::mem::take(&mut changed_paths).into_iter().collect())
            };
            rescan = changes.is_none();
            match discovered {
                Ok(paths) => {
                    health = if paths.len() > MAX_FILES {
                        format!("会话超过读取上限，仅跟踪最近 {MAX_FILES} 个；其余状态未确认。")
                    } else if desktop_epoch.is_none() {
                        "尚未确认 Desktop 运行，保留最后状态供核对。".into()
                    } else {
                        "本机 Desktop 状态 · 每秒读取变化".into()
                    };
                    for path in paths.into_iter().take(MAX_FILES) {
                        if !logs.contains_key(&path)
                            && !ignored_paths.contains(&path)
                            && path.is_file()
                        {
                            let mut log = match Log::open(path.clone(), !first, &mut notices) {
                                Ok(log) => log,
                                Err(_) => {
                                    health =
                                        "部分记录暂时不可读或格式不兼容；等待后续变化重新检测。"
                                            .into();
                                    continue;
                                }
                            };
                            if desktop_epoch.is_none() {
                                log.session.mark_unconfirmed();
                            }
                            if logs.len() >= MAX_FILES {
                                if let Some(old) = logs
                                    .iter()
                                    .find(|(_, v)| v.session.view().is_none())
                                    .map(|(p, _)| p.clone())
                                {
                                    logs.remove(&old);
                                } else {
                                    break;
                                }
                            }
                            if log.session.desktop {
                                logs.insert(path, log);
                            } else {
                                ignored_paths.insert(path);
                                if ignored_paths.len() > MAX_FILES {
                                    ignored_paths.pop_first();
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    health = "无法读取本机会话状态，请检查 Codex 配置目录和访问权限。".into();
                    for log in logs.values_mut() {
                        log.session.mark_unconfirmed();
                    }
                }
            }
            if titles.read(&root).is_err() {
                health.push_str("；名称索引不可用，使用项目和短标识");
            }
            next_discovery = Instant::now() + Duration::from_secs(5);
            first = false;
            changed = true;
        }
        if Instant::now() >= next_tick {
            for log in logs.values_mut() {
                match log.read(true, &mut notices) {
                    Ok(_) => {}
                    Err(_) => {
                        log.session.mark_unconfirmed();
                    }
                }
                if log.last_growth.elapsed() > Duration::from_secs(120) || desktop_epoch.is_none() {
                    log.session.mark_unconfirmed();
                }
                if let Some(title) = titles.names.get(&log.session.id) {
                    log.session.title = title.clone();
                }
                if log.incompatible {
                    health = "部分状态记录格式不兼容或过大，相关会话状态待确认。".into();
                }
            }
            next_tick = Instant::now() + Duration::from_secs(1);
            changed = true;
        }
        pending_hooks.retain(|(at, event)| {
            if let Some(log) = logs
                .values_mut()
                .find(|log| log.session.id == event.session_id)
            {
                if let Some(notice) = log.session.hook(event) {
                    notices.push(notice);
                }
                hook_status = Some(format!("已收到 Desktop hooks 状态事件：{:?}。", event.kind));
                changed = true;
                false
            } else {
                at.elapsed() < Duration::from_secs(10)
            }
        });
        if changed {
            let mut rows: Vec<_> = logs.values().filter_map(|log| log.session.view()).collect();
            rows.sort_by_key(|row| {
                (
                    match row.status {
                        SessionStatus::Waiting => 0,
                        SessionStatus::Approval => 1,
                        SessionStatus::Running => 2,
                        _ => 3,
                    },
                    row.title.clone(),
                )
            });
            let mut update = SessionUpdate {
                rows,
                notices,
                status: health.clone(),
                hook_status,
            };
            while !stop.load(Ordering::Relaxed) {
                match updates.try_send(update) {
                    Ok(()) => break,
                    Err(TrySendError::Disconnected(_)) => return,
                    Err(TrySendError::Full(value)) => update = value,
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "需要本机正在运行的 Codex Desktop，仅执行只读状态检测"]
    fn real_desktop_metadata_read_only() {
        assert!(
            super::super::codex_presence::desktop_started().is_some(),
            "无法确认本机 Desktop 进程"
        );
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let (_tx, hooks) = std::sync::mpsc::sync_channel(1);
        let (tx, rx) = std::sync::mpsc::sync_channel(4);
        let worker = std::thread::spawn(move || run(worker_stop, hooks, tx));
        let result = rx.recv_timeout(Duration::from_secs(15));
        stop.store(true, Ordering::Relaxed);
        worker.join().unwrap();
        let update = result.expect("读取真实本地状态超时");
        println!(
            "Desktop 已识别；当前列表 {} 项；{}",
            update.rows.len(),
            update.status
        );
        assert!(!update.status.contains("无法读取"));
        assert!(!update.status.contains("不兼容"));
        assert!(update.notices.is_empty(), "启动重建不重播历史消息");
    }
    #[test]
    fn partial_lines_and_truncation_are_recovered_without_replay() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(
            format!("pet-session-{}-{:?}", std::process::id(), Instant::now())
                .replace([':', ' ', '{', '}'], ""),
        );
        fs::create_dir(&dir).unwrap();
        let path = dir.join("log.jsonl");
        let meta = "{\"type\":\"session_meta\",\"payload\":{\"id\":\"s\",\"originator\":\"Codex Desktop\",\"source\":\"vscode\"}}\n";
        fs::write(&path, meta).unwrap();
        let mut log = Log::open(path.clone(), false, &mut vec![]).unwrap();
        let mut notices = vec![];
        let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(
            b"{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_started\",\"turn_id\":\"t\"}}",
        )
        .unwrap();
        log.read(true, &mut notices).unwrap();
        assert!(log.session.view().is_none());
        file.write_all(b"\n").unwrap();
        log.read(true, &mut notices).unwrap();
        assert_eq!(log.session.status(), SessionStatus::Running);
        drop(file);
        fs::write(&path, meta).unwrap();
        log.read(true, &mut notices).unwrap();
        assert!(log.session.view().is_none());
        fs::remove_file(path).unwrap();
        fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn large_private_bodies_are_streamed_and_subagents_excluded() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/session-fixtures")
            .join(format!(
                "{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("main.jsonl");
        let meta = "{\"type\":\"session_meta\",\"payload\":{\"id\":\"s\",\"originator\":\"Codex Desktop\",\"source\":\"vscode\"}}\n";
        let big = serde_json::json!({"type":"response_item","payload":{"type":"message","id":null,"content":"private".repeat(200_000)}});
        fs::write(&path, format!("{meta}{big}\n{{\"type\":\"event_msg\",\"payload\":{{\"type\":\"task_started\",\"turn_id\":\"t\"}}}}\n")).unwrap();
        let log = Log::open(path, false, &mut vec![]).unwrap();
        assert!(!log.incompatible);
        assert_eq!(log.session.status(), SessionStatus::Running);
        let path = dir.join("child.jsonl");
        fs::write(&path, "{\"type\":\"session_meta\",\"payload\":{\"id\":\"child\",\"originator\":\"Codex Desktop\",\"source\":{\"subagent\":{\"parent_thread_id\":\"s\"}}}}\n").unwrap();
        assert!(!Log::open(path, false, &mut vec![]).unwrap().session.desktop);
    }
}
