use crate::voice::{Pcm, PlayRequest, VoiceReceiver};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{RecvTimeoutError, SyncSender},
    },
    time::{Duration, Instant},
};
use windows::{Win32::Media::Audio::*, core::PSTR};

pub enum VoiceEvent {
    Ready,
    Error(String),
}
fn checked(code: u32, action: &str) -> Result<(), String> {
    if code == 0 {
        Ok(())
    } else {
        Err(format!(
            "流萤语音{action}失败（音频错误 {code}），请检查输出设备后重新试听。文字提醒仍可使用。"
        ))
    }
}
#[repr(C, align(8))]
struct AlignedHeader(WAVEHDR);
struct Playback {
    device: HWAVEOUT,
    header: Option<Box<AlignedHeader>>,
    samples: Option<Vec<i16>>,
    prepared: bool,
}
impl Playback {
    fn open(pcm: &Pcm<'_>, volume: u16) -> Result<Self, String> {
        if unsafe { waveOutGetNumDevs() } == 0 {
            return Err("没有可用音频输出设备，连接设备后可重新试听。文字提醒仍可使用。".into());
        }
        let format = WAVEFORMATEX {
            wFormatTag: WAVE_FORMAT_PCM as u16,
            nChannels: 1,
            nSamplesPerSec: pcm.sample_rate,
            nAvgBytesPerSec: pcm.sample_rate * 2,
            nBlockAlign: 2,
            wBitsPerSample: 16,
            cbSize: 0,
        };
        let mut device = HWAVEOUT::default();
        checked(
            unsafe {
                waveOutOpen(
                    Some(&mut device),
                    WAVE_MAPPER,
                    &format,
                    None,
                    None,
                    CALLBACK_NULL,
                )
            },
            "打开设备",
        )?;
        let mut samples = pcm.samples(volume);
        let header = Box::new(AlignedHeader(WAVEHDR {
            lpData: PSTR(samples.as_mut_ptr().cast()),
            dwBufferLength: (samples.len() * 2) as u32,
            ..WAVEHDR::default()
        }));
        let mut playback = Self {
            device,
            header: Some(header),
            samples: Some(samples),
            prepared: false,
        };
        let header = playback.header.as_deref_mut().unwrap();
        checked(
            unsafe { waveOutPrepareHeader(device, &mut header.0, size_of::<WAVEHDR>() as u32) },
            "准备播放",
        )?;
        playback.prepared = true;
        checked(
            unsafe {
                waveOutWrite(
                    device,
                    &mut playback.header.as_deref_mut().unwrap().0,
                    size_of::<WAVEHDR>() as u32,
                )
            },
            "播放",
        )?;
        Ok(playback)
    }
    fn done(&self) -> bool {
        // 系统异步更新标志；Box 固定地址，当前布局的标志字段按 4 字节对齐。
        let flags = unsafe {
            std::ptr::addr_of!(self.header.as_deref().unwrap().0.dwFlags).read_volatile()
        };
        flags & WHDR_DONE != 0
    }
    fn close(&mut self) -> Result<(), String> {
        if self.device.0.is_null() {
            return Ok(());
        }
        let mut error = None;
        if self.prepared {
            if let Err(e) = checked(unsafe { waveOutReset(self.device) }, "停止播放") {
                error = Some(e);
            }
            let result = unsafe {
                waveOutUnprepareHeader(
                    self.device,
                    &mut self.header.as_deref_mut().unwrap().0,
                    size_of::<WAVEHDR>() as u32,
                )
            };
            if result != 0 {
                // 驱动拒绝归还缓冲区时保留其内存，防止系统继续访问已释放的地址。
                if let Some(header) = self.header.take() {
                    std::mem::forget(header);
                }
                if let Some(samples) = self.samples.take() {
                    std::mem::forget(samples);
                }
                error = checked(result, "释放播放缓冲区").err();
            }
            self.prepared = false;
        }
        let result = unsafe { waveOutClose(self.device) };
        self.device = HWAVEOUT::default();
        if let Err(e) = checked(result, "关闭设备") {
            error.get_or_insert(e);
        }
        error.map_or(Ok(()), Err)
    }
}
impl Drop for Playback {
    fn drop(&mut self) {
        // 正常路径显式关闭并报告错误；此处覆盖创建过程中提前返回的路径。
        let _ = self.close();
    }
}
fn play(request: &PlayRequest, cancelled: impl Fn() -> bool) -> Result<(), String> {
    let pcm = Pcm::parse(request.cue.wav())?;
    if cancelled() || request.volume == 0 {
        return Ok(());
    }
    let mut playback = Playback::open(&pcm, request.volume)?;
    let started = Instant::now();
    let mut timed_out = false;
    while !playback.done() && !cancelled() {
        if started.elapsed() >= Duration::from_secs(8) {
            timed_out = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    playback.close()?;
    if timed_out {
        Err("流萤语音播放超时，已停止。请检查输出设备后重新试听。".into())
    } else {
        Ok(())
    }
}
pub fn run(stop: Arc<AtomicBool>, rx: VoiceReceiver, tx: SyncSender<VoiceEvent>) {
    run_with_player(stop, rx, tx, |request, cancelled| play(request, cancelled));
}

fn run_with_player(
    stop: Arc<AtomicBool>,
    rx: VoiceReceiver,
    tx: SyncSender<VoiceEvent>,
    mut player: impl FnMut(&PlayRequest, &dyn Fn() -> bool) -> Result<(), String>,
) {
    let _ = tx.try_send(VoiceEvent::Ready);
    while !stop.load(Ordering::Relaxed) {
        match rx.next() {
            Ok(request) => {
                let cancelled = || stop.load(Ordering::Relaxed) || rx.cancelled(&request);
                let event = match player(&request, &cancelled) {
                    Ok(()) => VoiceEvent::Ready,
                    Err(error) => VoiceEvent::Error(error),
                };
                let _ = tx.try_send(event);
            }
            Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice::{self, VoiceCue};
    use std::sync::mpsc;

    #[test]
    fn failed_output_can_retry_without_restarting_worker() {
        let (sender, receiver) = voice::channel();
        let (events, output) = mpsc::sync_channel(8);
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let worker = std::thread::spawn(move || {
            let mut attempts = 0;
            run_with_player(worker_stop, receiver, events, |_, _| {
                attempts += 1;
                if attempts == 1 {
                    Err("没有可用音频输出设备".into())
                } else {
                    Ok(())
                }
            });
        });
        assert!(matches!(
            output.recv_timeout(Duration::from_secs(2)),
            Ok(VoiceEvent::Ready)
        ));
        sender.play(VoiceCue::Preview, 70).unwrap();
        assert!(matches!(
            output.recv_timeout(Duration::from_secs(2)),
            Ok(VoiceEvent::Error(_))
        ));
        sender.play(VoiceCue::Preview, 35).unwrap();
        assert!(matches!(
            output.recv_timeout(Duration::from_secs(2)),
            Ok(VoiceEvent::Ready)
        ));
        stop.store(true, Ordering::Relaxed);
        worker.join().unwrap();
    }

    #[test]
    fn stop_and_exit_interrupt_active_playback_without_replaying_queue() {
        for exit in [false, true] {
            let (sender, receiver) = voice::channel();
            let (events, _) = mpsc::sync_channel(8);
            let (started, started_rx) = mpsc::sync_channel(8);
            let stop = Arc::new(AtomicBool::new(false));
            let worker_stop = stop.clone();
            let worker = std::thread::spawn(move || {
                run_with_player(worker_stop, receiver, events, |_, cancelled| {
                    started.send(()).unwrap();
                    let deadline = Instant::now() + Duration::from_secs(2);
                    while !cancelled() && Instant::now() < deadline {
                        std::thread::sleep(Duration::from_millis(2));
                    }
                    assert!(cancelled());
                    Ok(())
                });
            });
            sender.play(VoiceCue::Preview, 70).unwrap();
            started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            for _ in 0..8 {
                sender.play(VoiceCue::Cpu, 70).unwrap();
            }
            if exit {
                stop.store(true, Ordering::Relaxed);
            } else {
                sender.stop();
            }
            assert!(started_rx.recv_timeout(Duration::from_millis(100)).is_err());
            stop.store(true, Ordering::Relaxed);
            worker.join().unwrap();
        }
    }
}
