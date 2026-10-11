use super::system::ComApartment;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, SyncSender},
    },
    time::Duration,
};
use windows::{
    Win32::{Media::Speech::*, System::Com::*, UI::WindowsAndMessaging::*},
    core::BSTR,
};

pub enum VoiceCommand {
    Say {
        text: String,
        voice_id: String,
        volume: u16,
    },
    Stop,
}
pub enum VoiceEvent {
    Ready(Vec<(String, String)>),
    Error(String),
}
pub fn run(stop: Arc<AtomicBool>, rx: Receiver<VoiceCommand>, tx: SyncSender<VoiceEvent>) {
    let Ok(_com) = ComApartment::new(COINIT_APARTMENTTHREADED) else {
        let _ = tx.try_send(VoiceEvent::Error(
            "语音初始化失败，文字提醒仍可使用。".into(),
        ));
        return;
    };
    let result: windows::core::Result<ISpeechVoice> =
        unsafe { CoCreateInstance(&SpVoice, None, CLSCTX_ALL) };
    let voice = match result {
        Ok(v) => v,
        Err(_) => {
            let _ = tx.try_send(VoiceEvent::Error(
                "系统语音不可用，文字提醒仍可使用。".into(),
            ));
            return;
        }
    };
    let mut tokens = Vec::new();
    let mut labels = Vec::new();
    unsafe {
        if let Ok(list) = voice.GetVoices(&BSTR::new(), &BSTR::new())
            && let Ok(count) = list.Count()
        {
            for index in 0..count {
                if let Ok(token) = list.Item(index)
                    && let (Ok(id), Ok(name)) = (token.Id(), token.GetDescription(0))
                {
                    let id = id.to_string();
                    labels.push((id.clone(), name.to_string()));
                    tokens.push((id, token));
                }
            }
        }
    }
    if labels.is_empty() {
        let _ = tx.try_send(VoiceEvent::Error(
            "没有可用系统音色，请先在 Windows 中安装语音。".into(),
        ));
    } else {
        let _ = tx.try_send(VoiceEvent::Ready(labels));
    }
    while !stop.load(Ordering::Relaxed) {
        match rx.recv_timeout(Duration::from_millis(30)) {
            Ok(VoiceCommand::Say {
                text,
                voice_id,
                volume,
            }) => {
                let result = (|| unsafe {
                    if let Some((_, token)) = tokens
                        .iter()
                        .find(|(id, _)| id == &voice_id)
                        .or_else(|| tokens.first())
                    {
                        voice.putref_Voice(token)?;
                    }
                    voice.SetVolume(volume as i32)?;
                    voice.Speak(
                        &BSTR::from(text),
                        SpeechVoiceSpeakFlags(
                            SVSFlagsAsync.0 | SVSFPurgeBeforeSpeak.0 | SVSFIsNotXML.0,
                        ),
                    )?;
                    Ok::<_, windows::core::Error>(())
                })();
                if result.is_err() {
                    let _ = tx.try_send(VoiceEvent::Error(
                        "语音播放失败，请检查音色和音频设备。".into(),
                    ));
                }
            }
            Ok(VoiceCommand::Stop) => unsafe {
                let _ = voice.Speak(
                    &BSTR::new(),
                    SpeechVoiceSpeakFlags(SVSFlagsAsync.0 | SVSFPurgeBeforeSpeak.0),
                );
            },
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
        }
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
    unsafe {
        let _ = voice.Speak(
            &BSTR::new(),
            SpeechVoiceSpeakFlags(SVSFlagsAsync.0 | SVSFPurgeBeforeSpeak.0),
        );
    }
}
