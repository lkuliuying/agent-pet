use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    pub scale: f32,
    pub tkl: bool,
    pub position: Option<(i32, i32)>,
    pub paused: bool,
    pub passthrough: bool,
    pub quiet: bool,
    pub quiet_fullscreen: bool,
    pub voice: bool,
    pub voice_id: String,
    pub volume: u16,
    pub battery: bool,
    pub power: bool,
    pub network: bool,
    pub cpu: bool,
    pub memory: bool,
    pub codex: bool,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            scale: 0.9,
            tkl: false,
            position: None,
            paused: false,
            passthrough: false,
            quiet: false,
            quiet_fullscreen: true,
            voice: false,
            voice_id: String::new(),
            volume: 70,
            battery: true,
            power: true,
            network: true,
            cpu: true,
            memory: true,
            codex: true,
        }
    }
}
impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("不支持的配置版本".into());
        }
        if !self.scale.is_finite() || !(0.6..=1.6).contains(&self.scale) {
            return Err("缩放必须在 60% 到 160% 之间".into());
        }
        if self.volume > 100 {
            return Err("音量必须在 0 到 100 之间".into());
        }
        if self.voice_id.len() > 4096 {
            return Err("语音标识过长".into());
        }
        Ok(())
    }
}
pub fn default_directory() -> io::Result<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(|p| PathBuf::from(p).join("FireflyPet"))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "找不到 LOCALAPPDATA"))
}
pub fn load(directory: &Path) -> (Config, Option<String>) {
    let path = directory.join("settings.json");
    match read_settings(&path) {
        Ok(c) => (c, None),
        Err(e) if e.kind() == io::ErrorKind::NotFound => (Config::default(), None),
        Err(e) if e.kind() != io::ErrorKind::InvalidData => (
            Config::default(),
            Some(format!("配置无法读取，原文件保留：{e}")),
        ),
        Err(reason) => {
            // 仅在恢复备份成功后才允许后续写入覆盖损坏配置。
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let backup = directory.join(format!("settings.invalid-{stamp}.json"));
            match fs::copy(&path, &backup) {
                Ok(_) => (
                    Config::default(),
                    Some(format!("{reason}，原配置已备份，当前使用默认设置。")),
                ),
                Err(e) => (
                    Config::default(),
                    Some(format!("{reason}；原文件保留，备份失败：{e}")),
                ),
            }
        }
    }
}
fn read_settings(path: &Path) -> io::Result<Config> {
    use io::Read;
    let mut bytes = Vec::new();
    fs::File::open(path)?.take(65537).read_to_end(&mut bytes)?;
    if bytes.len() > 65536 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "配置超过 64 KiB",
        ));
    }
    let config: Config = serde_json::from_slice(&bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "配置格式无效"))?;
    config
        .validate()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(config)
}
pub fn save(directory: &Path, config: &Config) -> io::Result<()> {
    config.validate().map_err(io::Error::other)?;
    fs::create_dir_all(directory)?;
    let target = directory.join("settings.json");
    if target.exists()
        && let Err(error) = read_settings(&target)
    {
        if error.kind() != io::ErrorKind::InvalidData {
            return Err(error);
        }
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        fs::copy(
            &target,
            directory.join(format!("settings.invalid-{stamp}.json")),
        )?;
    }
    let temp = directory.join(format!("settings.{}.tmp", std::process::id()));
    let result = (|| {
        use io::Write;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temp)?;
        file.write_all(&serde_json::to_vec_pretty(config)?)?;
        file.sync_all()?;
        drop(file);
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows::{
                Win32::Storage::FileSystem::{
                    MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
                },
                core::PCWSTR,
            };
            let from: Vec<u16> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
            let to: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
            unsafe {
                MoveFileExW(
                    PCWSTR(from.as_ptr()),
                    PCWSTR(to.as_ptr()),
                    MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
                )
                .map_err(io::Error::other)?;
            }
        }
        #[cfg(not(windows))]
        fs::rename(&temp, &target)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validation() {
        assert!(Config::default().validate().is_ok());
        assert!(
            Config {
                scale: f32::NAN,
                ..Config::default()
            }
            .validate()
            .is_err()
        );
        assert!(
            Config {
                version: 2,
                ..Config::default()
            }
            .validate()
            .is_err()
        );
        assert!(serde_json::from_str::<Config>(r#"{"mystery":true}"#).is_err());
    }
    #[test]
    fn legacy_layout_preserves_other_preferences() {
        for tkl in [false, true] {
            let original = Config {
                tkl,
                position: Some((-300, 80)),
                scale: 1.2,
                voice: true,
                voice_id: "test-voice".into(),
                volume: 35,
                quiet: true,
                network: false,
                ..Config::default()
            };
            let json = serde_json::to_string(&original).unwrap();
            let restored: Config = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, original);
            assert!(restored.validate().is_ok());
        }
    }
    #[test]
    fn atomic_roundtrip_and_recovery() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/config-tests")
            .join(format!(
                "pet-config-test-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        let cfg = Config {
            tkl: true,
            ..Config::default()
        };
        save(&dir, &cfg).unwrap();
        assert_eq!(load(&dir).0, cfg);
        fs::write(dir.join("settings.json"), b"{broken").unwrap();
        let (recovered, warning) = load(&dir);
        assert!(warning.is_some());
        assert_eq!(recovered, Config::default());
        assert!(fs::read_dir(&dir).unwrap().any(|e| {
            e.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("settings.invalid")
        }));
        save(&dir, &recovered).unwrap();
        assert!(load(&dir).1.is_none());
        let before = fs::read(dir.join("settings.json")).unwrap();
        assert!(save(&dir, &Config { volume: 101, ..cfg }).is_err());
        assert_eq!(fs::read(dir.join("settings.json")).unwrap(), before);
        fs::write(dir.join("settings.json"), vec![b' '; 65537]).unwrap();
        assert!(load(&dir).1.unwrap().contains("64 KiB"));
        fs::remove_dir_all(dir).unwrap();
    }
}
