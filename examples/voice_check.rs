use pet_desktop::{
    platform::voice::{self, VoiceEvent},
    voice::{self as clips, VoiceCue},
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

fn main() -> Result<(), String> {
    let (sender, receiver) = clips::channel();
    let (events, output) = mpsc::sync_channel(8);
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = stop.clone();
    let worker = std::thread::spawn(move || voice::run(worker_stop, receiver, events));
    let result = (|| {
        output
            .recv_timeout(Duration::from_secs(2))
            .map_err(|e| e.to_string())?;
        let cues = if std::env::args().any(|arg| arg == "--all") {
            VoiceCue::ALL
        } else {
            &[VoiceCue::Preview]
        };
        for cue in cues {
            sender.play(*cue, 70)?;
            match output
                .recv_timeout(Duration::from_secs(10))
                .map_err(|e| e.to_string())?
            {
                VoiceEvent::Ready => println!("播放完成：{}", cue.id()),
                VoiceEvent::Error(error) => return Err(error),
            }
        }
        sender.play(VoiceCue::Preview, 35)?;
        std::thread::sleep(Duration::from_millis(120));
        for _ in 0..8 {
            sender.play(VoiceCue::Cpu, 35)?;
        }
        let cancelled = Instant::now();
        sender.stop();
        match output
            .recv_timeout(Duration::from_secs(2))
            .map_err(|e| e.to_string())?
        {
            VoiceEvent::Ready => println!("取消当前播放：{} ms", cancelled.elapsed().as_millis()),
            VoiceEvent::Error(error) => return Err(error),
        }
        if output.recv_timeout(Duration::from_millis(200)).is_ok() {
            return Err("取消后出现了积压播放结果。".into());
        }
        sender.play(VoiceCue::Preview, 35)?;
        std::thread::sleep(Duration::from_millis(120));
        Ok(())
    })();
    let quitting = Instant::now();
    stop.store(true, Ordering::Relaxed);
    worker.join().map_err(|_| "语音线程异常退出".to_string())?;
    println!("退出时释放音频：{} ms", quitting.elapsed().as_millis());
    result
}
