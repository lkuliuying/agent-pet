use super::codex_usage::{QueryProcess, find_codex};
use serde_json::{Value, json};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, SyncSender},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const PLUGIN: &str = "firefly-pet@firefly-local";
const MARKET: &str = "firefly-local";
const FILES: &[(&str, &str)] = &[
    (
        ".agents/plugins/marketplace.json",
        include_str!("../../codex-marketplace/.agents/plugins/marketplace.json"),
    ),
    (
        "plugins/firefly-pet/.codex-plugin/plugin.json",
        include_str!("../../codex-marketplace/plugins/firefly-pet/.codex-plugin/plugin.json"),
    ),
    (
        "plugins/firefly-pet/hooks/hooks.json",
        include_str!("../../codex-marketplace/plugins/firefly-pet/hooks/hooks.json"),
    ),
    (
        "plugins/firefly-pet/hooks/notify.cmd",
        include_str!("../../codex-marketplace/plugins/firefly-pet/hooks/notify.cmd"),
    ),
];

pub enum Request {
    Check,
    Install,
}
pub struct Update {
    pub status: String,
    pub busy: bool,
}

fn cli(exe: &Path, args: &[&std::ffi::OsStr], stop: &AtomicBool) -> Result<Value, String> {
    let mut command = Command::new(exe);
    command.args(args);
    if let Some(parent) = exe.parent() {
        command.current_dir(parent);
    }
    QueryProcess::capture(&mut command, stop)
}

fn call(exe: &Path, args: &[&str], stop: &AtomicBool) -> Result<Value, String> {
    cli(
        exe,
        &args.iter().map(std::ffi::OsStr::new).collect::<Vec<_>>(),
        stop,
    )
}

fn same_file(a: &Path, b: &Path) -> std::io::Result<bool> {
    let mut a = File::open(a)?;
    let mut b = File::open(b)?;
    if a.metadata()?.len() != b.metadata()?.len() {
        return Ok(false);
    }
    let (mut left, mut right) = ([0; 65536], [0; 65536]);
    loop {
        let count = a.read(&mut left)?;
        b.read_exact(&mut right[..count])?;
        if left[..count] != right[..count] {
            return Ok(false);
        }
        if count == 0 {
            return Ok(true);
        }
    }
}

fn valid_plugin(root: &Path, current_exe: &Path) -> bool {
    root.is_absolute()
        && FILES.iter().skip(1).all(|(path, content)| {
            fs::read(root.join(path.strip_prefix("plugins/firefly-pet/").unwrap()))
                .ok()
                .as_deref()
                == Some(content.as_bytes())
        })
        && same_file(&root.join("PetDesktop.exe"), current_exe).unwrap_or(false)
}

fn deploy(directory: &Path, current_exe: &Path) -> Result<PathBuf, String> {
    let base = directory.join("codex-bridge");
    fs::create_dir_all(&base).map_err(|_| "创建稳定联动目录失败")?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "系统时间无效")?
        .as_nanos();
    let root = base.join(format!("bridge-{stamp}"));
    fs::create_dir(&root).map_err(|_| "创建联动版本目录失败")?;
    for (path, content) in FILES {
        let target = root.join(path);
        fs::create_dir_all(target.parent().unwrap()).map_err(|_| "创建插件资源目录失败")?;
        let mut file = File::create_new(target).map_err(|_| "写入插件资源失败")?;
        file.write_all(content.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|_| "保存插件资源失败")?;
    }
    let plugin = root.join("plugins/firefly-pet");
    fs::copy(current_exe, plugin.join("PetDesktop.exe")).map_err(|_| "复制桥接程序失败")?;
    if !valid_plugin(&plugin, current_exe) {
        return Err("桥接资源完整性校验失败".into());
    }
    root.canonicalize()
        .map_err(|_| "无法定位稳定联动目录".into())
}

fn marketplace(value: &Value) -> Result<Option<PathBuf>, String> {
    let entries = value["marketplaces"]
        .as_array()
        .ok_or("marketplace 响应格式不兼容")?;
    entries
        .iter()
        .find(|v| v["name"].as_str() == Some(MARKET))
        .map(|v| {
            v["root"]
                .as_str()
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .ok_or_else(|| "本地 marketplace 路径无效".into())
        })
        .transpose()
}

fn installed(value: &Value) -> Result<Option<&Value>, String> {
    let entries = value["installed"].as_array().ok_or("插件列表格式不兼容")?;
    Ok(entries
        .iter()
        .find(|v| v["pluginId"].as_str() == Some(PLUGIN)))
}

fn hook_state(value: &Value, current_exe: &Path) -> Result<String, String> {
    let entries = value["data"].as_array().ok_or("hooks/list 版本不兼容")?;
    let mut hooks = Vec::new();
    for entry in entries {
        hooks.extend(
            entry["hooks"]
                .as_array()
                .ok_or("hooks/list 条目不兼容")?
                .iter()
                .filter(|h| h["pluginId"].as_str() == Some(PLUGIN)),
        );
    }
    if hooks.is_empty() {
        return Ok("未启用：CLI 未加载流萤 hooks，请在 Codex 插件设置检查启用状态。".into());
    }
    for hook in &hooks {
        let path = hook["sourcePath"]
            .as_str()
            .map(PathBuf::from)
            .ok_or("hooks 来源路径缺失")?;
        let root = path
            .parent()
            .and_then(Path::parent)
            .ok_or("hooks 来源路径无效")?;
        if !valid_plugin(root, current_exe) {
            return Ok(
                "版本不兼容：已加载的桥接程序或 hooks 与当前桌宠不一致，请安装／更新联动。".into(),
            );
        }
    }
    let expected = [
        "sessionStart",
        "userPromptSubmit",
        "preToolUse",
        "postToolUse",
        "permissionRequest",
        "stop",
        "interrupt",
        "sessionEnd",
    ];
    if expected
        .iter()
        .any(|event| !hooks.iter().any(|h| h["eventName"].as_str() == Some(event)))
    {
        return Ok("版本不兼容：缺少必需的 hooks 事件，请更新联动。".into());
    }
    if hooks
        .iter()
        .any(|h| matches!(h["trustStatus"].as_str(), Some("untrusted" | "modified")))
    {
        return Ok("需要信任：请在 Codex 中审阅并信任流萤 hooks，然后重新检测。".into());
    }
    if hooks
        .iter()
        .any(|h| !matches!(h["trustStatus"].as_str(), Some("trusted" | "managed")))
    {
        return Err("hooks 信任状态版本不兼容".into());
    }
    if hooks.iter().any(|h| h["enabled"].as_bool() != Some(true)) {
        return Ok("未启用：请在 Codex 中启用流萤插件和 hooks。".into());
    }
    Ok("等待事件：CLI 已加载并信任联动；Desktop 投递以收到真实事件为准。".into())
}

fn check(exe: &Path, current_exe: &Path, stop: &AtomicBool) -> Result<String, String> {
    let list = call(
        exe,
        &["plugin", "list", "--marketplace", MARKET, "--json"],
        stop,
    )?;
    let Some(plugin) = installed(&list)? else {
        return Ok("未安装：点击“安装／更新联动”接入权限和生命周期提醒。".into());
    };
    if plugin["enabled"].as_bool() != Some(true) {
        return Ok("未启用：请在 Codex 中启用流萤插件。".into());
    }
    let mut command = Command::new(exe);
    command.args(["app-server", "--listen", "stdio://"]);
    if let Some(parent) = exe.parent() {
        command.current_dir(parent);
    }
    let mut process = QueryProcess::spawn(&mut command).map_err(|e| e.message().to_string())?;
    let deadline = Instant::now() + Duration::from_secs(10);
    let cancelled = || stop.load(Ordering::Relaxed);
    process.send(json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"firefly_integration","version":env!("CARGO_PKG_VERSION")},"capabilities":{"experimentalApi":true}}})).map_err(|e| e.message())?;
    process
        .receive(1, deadline, &cancelled)
        .map_err(|e| e.message())?;
    process
        .send(json!({"method":"initialized","params":{}}))
        .map_err(|e| e.message())?;
    process
        .send(json!({"id":2,"method":"hooks/list","params":{"cwds":[]}}))
        .map_err(|e| e.message())?;
    let hooks = process
        .receive(2, deadline, &cancelled)
        .map_err(|e| format!("hooks/list 检测失败或版本不兼容：{}", e.message()))?;
    hook_state(&hooks, current_exe)
}

fn install(
    exe: &Path,
    current_exe: &Path,
    directory: &Path,
    stop: &AtomicBool,
) -> Result<(), String> {
    let old = marketplace(&call(
        exe,
        &["plugin", "marketplace", "list", "--json"],
        stop,
    )?)?;
    let previous = call(
        exe,
        &["plugin", "list", "--marketplace", MARKET, "--json"],
        stop,
    )?;
    let was_installed = installed(&previous)?.is_some();
    let root = if let Some(old) = old.as_ref().filter(|p| {
        valid_plugin(&p.join("plugins/firefly-pet"), current_exe)
            && p.starts_with(directory.join("codex-bridge"))
    }) {
        old.clone()
    } else {
        deploy(directory, current_exe)?
    };
    let changed = old.as_ref() != Some(&root);
    let stage = (|| {
        if changed {
            if old.is_some() {
                call(
                    exe,
                    &["plugin", "marketplace", "remove", MARKET, "--json"],
                    stop,
                )
                .map_err(|e| format!("切换旧联动市场失败：{e}"))?;
            }
            cli(
                exe,
                &[
                    "plugin".as_ref(),
                    "marketplace".as_ref(),
                    "add".as_ref(),
                    root.as_os_str(),
                    "--json".as_ref(),
                ],
                stop,
            )
            .map_err(|e| format!("登记新联动市场失败：{e}"))?;
        }
        let result = call(exe, &["plugin", "add", PLUGIN, "--json"], stop)
            .map_err(|e| format!("安装插件失败：{e}"))?;
        let path = result["installedPath"]
            .as_str()
            .map(PathBuf::from)
            .ok_or("安装结果缺少 installedPath，版本不兼容")?;
        if !valid_plugin(&path, current_exe) {
            return Err("安装后校验失败：实际桥接内容与当前桌宠不一致".into());
        }
        Ok(())
    })();
    if let Err(error) = stage {
        if changed && !stop.load(Ordering::Relaxed) {
            let rollback = (|| {
                let current = marketplace(&call(
                    exe,
                    &["plugin", "marketplace", "list", "--json"],
                    stop,
                )?)?;
                if current.is_some() {
                    call(
                        exe,
                        &["plugin", "marketplace", "remove", MARKET, "--json"],
                        stop,
                    )?;
                }
                if let Some(old) = old {
                    cli(
                        exe,
                        &[
                            "plugin".as_ref(),
                            "marketplace".as_ref(),
                            "add".as_ref(),
                            old.as_os_str(),
                            "--json".as_ref(),
                        ],
                        stop,
                    )?;
                    if was_installed {
                        call(exe, &["plugin", "add", PLUGIN, "--json"], stop)?;
                    }
                }
                Ok::<_, String>(())
            })();
            return Err(format!(
                "{error}；{}",
                if rollback.is_ok() {
                    "已恢复原市场登记"
                } else {
                    "恢复原市场登记失败，请在 Codex 中核对 firefly-local"
                }
            ));
        }
        return Err(error);
    }
    Ok(())
}

pub fn run(
    stop: Arc<AtomicBool>,
    directory: PathBuf,
    requests: Receiver<Request>,
    updates: SyncSender<Update>,
) {
    let mut first = true;
    while !stop.load(Ordering::Relaxed) {
        let request = if first {
            first = false;
            Request::Check
        } else {
            match requests.recv_timeout(Duration::from_millis(100)) {
                Ok(request) => request,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(_) => break,
            }
        };
        let _ = updates.try_send(Update {
            status: "正在检查联动资源和 Codex CLI…".into(),
            busy: true,
        });
        let result = (|| {
            let exe = find_codex().map_err(|e| e.message().to_string())?;
            let current = std::env::current_exe().map_err(|_| "无法定位桌宠程序")?;
            if matches!(request, Request::Install) {
                install(&exe, &current, &directory, &stop)?;
            }
            check(&exe, &current, &stop)
        })();
        let update = Update {
            status: result.unwrap_or_else(|e| format!("接入失败：{e}")),
            busy: false,
        };
        // UI 每帧排空；结果通道容量足以容纳本次开始与完成状态。
        let mut update = update;
        while !stop.load(Ordering::Relaxed) {
            match updates.try_send(update) {
                Ok(()) => break,
                Err(std::sync::mpsc::TrySendError::Disconnected(_)) => return,
                Err(std::sync::mpsc::TrySendError::Full(value)) => update = value,
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unrelated_marketplaces_and_plugins_are_never_selected() {
        assert!(
            marketplace(&json!({"marketplaces":[{"name":"other","root":"invalid"}]}))
                .unwrap()
                .is_none()
        );
        assert!(
            installed(&json!({"installed":[{"pluginId":"other@firefly-local"}]}))
                .unwrap()
                .is_none()
        );
        assert!(marketplace(&json!({"marketplaces":[{"name":MARKET,"root":"relative"}]})).is_err());
        assert!(installed(&json!({})).is_err());
    }

    #[test]
    fn installation_and_rollback_preserve_other_hooks_and_validate_actual_files() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/integration-fixtures")
            .join(format!(
                "中文 空格-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        fs::create_dir_all(&root).unwrap();
        let exe = root.join("cli.exe");
        assert!(
            Command::new("rustc")
                .args(["--edition", "2024"])
                .arg(
                    Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("tests/support/integration_fixture.rs")
                )
                .arg("-o")
                .arg(&exe)
                .status()
                .unwrap()
                .success()
        );
        let current = root.join("pet.exe");
        fs::write(&current, b"fixture bridge").unwrap();
        let original = root.join("older market");
        fs::create_dir(&original).unwrap();
        fs::write(root.join("market.txt"), original.to_str().unwrap()).unwrap();
        fs::write(root.join("installed"), b"yes").unwrap();
        fs::write(
            root.join("hooks.json"),
            b"other app hook - preserve exactly",
        )
        .unwrap();
        let stop = AtomicBool::new(false);
        fs::write(root.join("fail-once"), b"yes").unwrap();
        assert!(
            install(&exe, &current, &root, &stop)
                .unwrap_err()
                .contains("已恢复原市场登记")
        );
        assert_eq!(
            fs::read_to_string(root.join("market.txt")).unwrap(),
            original.to_str().unwrap()
        );
        install(&exe, &current, &root, &stop).unwrap();
        let installed_root = PathBuf::from(fs::read_to_string(root.join("market.txt")).unwrap())
            .join("plugins/firefly-pet");
        assert!(valid_plugin(&installed_root, &current));
        assert_eq!(
            fs::read(root.join("hooks.json")).unwrap(),
            b"other app hook - preserve exactly"
        );
        let hook_path = installed_root.join("hooks/hooks.json");
        // hooks/list 使用 camelCase，区别于 hooks.json 的事件键。
        let hooks: Vec<_> = ["sessionStart", "userPromptSubmit", "preToolUse", "postToolUse", "permissionRequest", "stop", "interrupt", "sessionEnd"].into_iter().map(|name| json!({"pluginId":PLUGIN,"eventName":name,"sourcePath":hook_path,"enabled":true,"trustStatus":"untrusted"})).collect();
        let mut data = json!({"data":[{"hooks":hooks}]});
        assert!(hook_state(&data, &current).unwrap().starts_with("需要信任"));
        for hook in data["data"][0]["hooks"].as_array_mut().unwrap() {
            hook["trustStatus"] = json!("trusted");
        }
        assert!(hook_state(&data, &current).unwrap().starts_with("等待事件"));
        data["data"][0]["hooks"][0]["enabled"] = json!(false);
        assert!(hook_state(&data, &current).unwrap().starts_with("未启用"));
        fs::write(installed_root.join("PetDesktop.exe"), b"old version").unwrap();
        assert!(
            hook_state(&data, &current)
                .unwrap()
                .starts_with("版本不兼容")
        );
    }
}
