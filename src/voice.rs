use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, SyncSender},
    },
    time::{Duration, Instant},
};

macro_rules! voice_cues {
    ($($variant:ident => $file:literal),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum VoiceCue { $($variant),+ }
        impl VoiceCue {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];
            pub fn id(self) -> &'static str {
                match self { $(Self::$variant => $file),+ }
            }
            pub fn wav(self) -> &'static [u8] {
                match self {
                    $(Self::$variant => include_bytes!(concat!("../assets/voice/", $file, ".wav"))),+
                }
            }
        }
    };
}
voice_cues! {
    Cpu => "cpu", Memory => "memory", Gpu => "gpu", CodexQuestion => "codex-question", PowerOn => "power-on", PowerOff => "power-off",
    Battery10 => "battery10", Battery20 => "battery20", NetworkOn => "network-on",
    NetworkOff => "network-off", CodexStop => "codex-stop", CodexPermission => "codex-permission",
    CodexInterrupt => "codex-interrupt", DemoCodexStop => "demo-codex-stop",
    DemoCodexPermission => "demo-codex-permission", DemoCodexInterrupt => "demo-codex-interrupt",
    Preview => "preview", Error => "error",
}

impl VoiceCue {
    pub fn system(key: &str) -> Option<Self> {
        Some(match key {
            "cpu" => Self::Cpu,
            "memory" => Self::Memory,
            "gpu" => Self::Gpu,
            "power-on" => Self::PowerOn,
            "power-off" => Self::PowerOff,
            "battery10" => Self::Battery10,
            "battery20" => Self::Battery20,
            "network-on" => Self::NetworkOn,
            "network-off" => Self::NetworkOff,
            "platform-error" => Self::Error,
            _ => return None,
        })
    }
}

#[derive(Debug)]
pub struct Pcm<'a> {
    pub sample_rate: u32,
    pub data: &'a [u8],
}
impl<'a> Pcm<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, String> {
        let invalid = || "内置语音 WAV 格式无效。".to_owned();
        if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return Err(invalid());
        }
        let declared = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
        if declared.checked_add(8) != Some(bytes.len()) {
            return Err(invalid());
        }
        let mut offset = 12usize;
        let mut sample_rate = None;
        let mut data = None;
        while offset < bytes.len() {
            let header = bytes.get(offset..offset + 8).ok_or_else(invalid)?;
            let len = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
            offset += 8;
            let end = offset.checked_add(len).ok_or_else(invalid)?;
            let chunk = bytes.get(offset..end).ok_or_else(invalid)?;
            match &header[..4] {
                b"fmt " => {
                    if sample_rate.is_some() || !matches!(len, 16 | 18) {
                        return Err(invalid());
                    }
                    let rate = u32::from_le_bytes(chunk[4..8].try_into().unwrap());
                    let byte_rate = u32::from_le_bytes(chunk[8..12].try_into().unwrap());
                    if chunk[..4] != [1, 0, 1, 0]
                        || chunk[12..16] != [2, 0, 16, 0]
                        || !(8000..=48000).contains(&rate)
                        || byte_rate != rate * 2
                        || (len == 18 && chunk[16..18] != [0, 0])
                    {
                        return Err(invalid());
                    }
                    sample_rate = Some(rate);
                }
                b"data" => {
                    if data.is_some() || chunk.is_empty() || !len.is_multiple_of(2) {
                        return Err(invalid());
                    }
                    data = Some(chunk);
                }
                _ => {}
            }
            offset = end.checked_add(len % 2).ok_or_else(invalid)?;
            if offset > bytes.len() {
                return Err(invalid());
            }
        }
        let sample_rate = sample_rate.ok_or_else(invalid)?;
        let data = data.ok_or_else(invalid)?;
        if data.len() > sample_rate as usize * 2 * 7 {
            return Err("内置语音超过 7 秒提醒周期。".into());
        }
        Ok(Self { sample_rate, data })
    }

    pub fn samples(&self, volume: u16) -> Vec<i16> {
        self.data
            .chunks_exact(2)
            .map(|v| {
                let sample = i16::from_le_bytes([v[0], v[1]]) as i32;
                (sample * i32::from(volume.min(100)) / 100) as i16
            })
            .collect()
    }
}

pub struct PlayRequest {
    pub cue: VoiceCue,
    pub volume: u16,
    generation: u64,
    created: Instant,
}
#[derive(Clone)]
pub struct VoiceSender {
    tx: SyncSender<PlayRequest>,
    generation: Arc<AtomicU64>,
}
pub struct VoiceReceiver {
    rx: Receiver<PlayRequest>,
    generation: Arc<AtomicU64>,
}
pub fn channel() -> (VoiceSender, VoiceReceiver) {
    let (tx, rx) = mpsc::sync_channel(8);
    let generation = Arc::new(AtomicU64::new(0));
    (
        VoiceSender {
            tx,
            generation: generation.clone(),
        },
        VoiceReceiver { rx, generation },
    )
}
impl VoiceSender {
    pub fn play(&self, cue: VoiceCue, volume: u16) -> Result<(), String> {
        self.tx
            .try_send(PlayRequest {
                cue,
                volume: volume.min(100),
                generation: self.generation.load(Ordering::SeqCst),
                created: Instant::now(),
            })
            .map_err(|_| "语音通道暂时不可用，文字提醒仍可使用。".into())
    }
    pub fn stop(&self) {
        // 取消不经过有界队列，队列满时也能让当前播放和旧请求失效。
        self.generation.fetch_add(1, Ordering::SeqCst);
    }
}
impl VoiceReceiver {
    pub fn cancelled(&self, request: &PlayRequest) -> bool {
        request.generation != self.generation.load(Ordering::SeqCst)
    }
    pub fn next(&self) -> Result<PlayRequest, RecvTimeoutError> {
        loop {
            let request = self.rx.recv_timeout(Duration::from_millis(20))?;
            if !self.cancelled(&request) && request.created.elapsed() < Duration::from_secs(7) {
                return Ok(request);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_notification_uses_its_matching_recording() {
        use crate::{
            bridge::{CodexEvent, CodexKind},
            config::Config,
            notices::{Notice, SystemRules, SystemSample},
        };
        let lines: Vec<serde_json::Value> =
            serde_json::from_str(include_str!("../assets/voice/lines.json")).unwrap();
        assert_eq!(lines.len(), VoiceCue::ALL.len());
        let mut rules = SystemRules::default();
        let config = Config::default();
        let start = Instant::now();
        let mut notices = Vec::new();
        for (seconds, charging, battery, online) in [
            (0, false, 100, true),
            (30, false, 100, true),
            (31, true, 100, true),
            (32, false, 20, false),
            (37, false, 10, false),
            (38, false, 10, true),
            (43, false, 10, true),
        ] {
            notices.extend(rules.update(
                &SystemSample {
                    cpu: Some(99.),
                    memory: Some(99.),
                    gpu: Some(99.),
                    charging: Some(charging),
                    battery: Some(battery),
                    online: Some(online),
                },
                &config,
                start + Duration::from_secs(seconds),
            ));
        }
        assert_eq!(notices.len(), 9);
        for key in [
            "cpu",
            "memory",
            "gpu",
            "power-on",
            "power-off",
            "battery10",
            "battery20",
            "network-on",
            "network-off",
        ] {
            let line = lines.iter().find(|v| v["id"] == key).unwrap();
            let notice = notices
                .iter()
                .find(|notice| notice.key.split(':').next() == Some(key))
                .unwrap();
            assert_eq!(notice.voice.unwrap().id(), key);
            if matches!(key, "cpu" | "memory" | "gpu") {
                assert!(notice.text.contains("99%"));
                assert!(!line["text"].as_str().unwrap().is_empty());
            } else {
                assert_eq!(line["text"], notice.text);
            }
        }
        assert_eq!(
            Notice::system("platform-error", "某个具体错误").voice,
            Some(VoiceCue::Error)
        );
        assert!(
            Notice::system("unknown", "不能为未知文本猜测音频")
                .voice
                .is_none()
        );
        for (kind, id) in [
            (CodexKind::Stop, "codex-stop"),
            (CodexKind::PermissionRequest, "codex-permission"),
            (CodexKind::Interrupt, "codex-interrupt"),
        ] {
            for simulated in [false, true] {
                let notice = CodexEvent {
                    version: 1,
                    kind,
                    session_id: "session".into(),
                    turn_id: "turn".into(),
                    simulated,
                    ..Default::default()
                }
                .notice();
                let expected = if simulated {
                    format!("demo-{id}")
                } else {
                    id.into()
                };
                assert_eq!(notice.voice.unwrap().id(), expected);
                let line = lines.iter().find(|v| v["id"] == expected).unwrap();
                let spoken = if simulated {
                    format!("这是一条演示提醒。{}", notice.text)
                } else {
                    notice.text
                };
                assert_eq!(line["text"], spoken);
            }
        }
    }

    #[test]
    fn all_embedded_clips_are_audible_mono_pcm_and_fit_bubbles() {
        assert_eq!(VoiceCue::ALL.len(), 18);
        for cue in VoiceCue::ALL {
            let pcm = Pcm::parse(cue.wav()).unwrap();
            let full = pcm.samples(100);
            assert!(full.iter().any(|v| v.unsigned_abs() > 1000), "{cue:?}");
            assert!(full.iter().all(|v| v.unsigned_abs() < 32767), "{cue:?}");
            assert!(pcm.samples(0).iter().all(|v| *v == 0));
            assert_eq!(pcm.samples(101), full);
            assert_eq!(
                full.iter()
                    .flat_map(|v| v.to_le_bytes())
                    .collect::<Vec<_>>(),
                pcm.data
            );
        }
    }

    #[test]
    fn wav_rejects_truncation_invalid_format_and_chunk_boundaries() {
        let original = VoiceCue::Preview.wav();
        for length in [0, 1, 11, 12, 19, 20, 35, original.len() - 1] {
            assert!(Pcm::parse(&original[..length]).is_err());
        }
        for (offset, value) in [(0, b'X'), (8, b'X'), (20, 3), (22, 2), (32, 4), (34, 8)] {
            let mut bytes = original.to_vec();
            bytes[offset] = value;
            assert!(Pcm::parse(&bytes).is_err());
        }
        let mut bytes = original.to_vec();
        bytes[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(Pcm::parse(&bytes).is_err());
        let mut bytes = original.to_vec();
        bytes.splice(12..12, *b"JUNK\x01\x00\x00\x00\x01\x00");
        let len = (bytes.len() - 8) as u32;
        bytes[4..8].copy_from_slice(&len.to_le_bytes());
        assert!(Pcm::parse(&bytes).is_ok());
    }

    #[test]
    fn gain_handles_negative_full_scale_without_overflow() {
        let data: Vec<_> = [i16::MIN, -100, 0, 100, i16::MAX]
            .into_iter()
            .flat_map(i16::to_le_bytes)
            .collect();
        let pcm = Pcm {
            sample_rate: 32000,
            data: &data,
        };
        assert_eq!(pcm.samples(50), [-16384, -50, 0, 50, 16383]);
    }

    #[test]
    fn cancellation_survives_full_queue_and_allows_a_fresh_preview() {
        let (tx, rx) = channel();
        tx.play(VoiceCue::Preview, 70).unwrap();
        let playing = rx.next().unwrap();
        for _ in 0..8 {
            tx.play(VoiceCue::Cpu, 70).unwrap();
        }
        assert!(tx.play(VoiceCue::Cpu, 70).is_err());
        tx.stop();
        assert!(rx.cancelled(&playing));
        assert!(matches!(rx.next(), Err(RecvTimeoutError::Timeout)));
        tx.play(VoiceCue::Preview, 35).unwrap();
        let fresh = rx.next().unwrap();
        assert_eq!(fresh.volume, 35);
        assert!(!rx.cancelled(&fresh));
    }
}
