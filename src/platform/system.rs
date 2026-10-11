use super::gpu::GpuSampler;
use crate::notices::SystemSample;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::SyncSender,
    },
    time::{Duration, Instant},
};
use windows::Win32::{
    Foundation::FILETIME,
    Networking::NetworkListManager::*,
    System::{Com::*, Power::*, SystemInformation::*, Threading::GetSystemTimes},
};

pub struct ComApartment;
impl ComApartment {
    pub fn new(flags: COINIT) -> windows::core::Result<Self> {
        unsafe {
            CoInitializeEx(None, flags).ok()?;
        }
        Ok(Self)
    }
}
impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}
fn ticks(f: FILETIME) -> u64 {
    (f.dwHighDateTime as u64) << 32 | f.dwLowDateTime as u64
}
pub fn cpu_percent(previous: (u64, u64), current: (u64, u64)) -> Option<f64> {
    let total = current.1.checked_sub(previous.1)?;
    let idle = current.0.checked_sub(previous.0)?;
    if total == 0 || idle > total {
        None
    } else {
        Some((total - idle) as f64 / total as f64 * 100.)
    }
}
pub fn run(stop: Arc<AtomicBool>, tx: SyncSender<SystemSample>) {
    let _com = ComApartment::new(COINIT_MULTITHREADED).ok();
    let network: Option<INetworkListManager> = if _com.is_some() {
        unsafe { CoCreateInstance(&NetworkListManager, None, CLSCTX_ALL).ok() }
    } else {
        None
    };
    let mut previous = None;
    let mut gpu = GpuSampler::new();
    while !stop.load(Ordering::Relaxed) {
        let mut sample = SystemSample::default();
        // 不可用时在下一轮重建计数器，支持驱动或设备恢复；首次采样保留未知。
        if gpu.is_none() {
            gpu = GpuSampler::new();
        }
        sample.gpu = gpu.as_mut().and_then(GpuSampler::sample);
        unsafe {
            let mut idle = FILETIME::default();
            let mut kernel = FILETIME::default();
            let mut user = FILETIME::default();
            if GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)).is_ok() {
                let now = (ticks(idle), ticks(kernel).saturating_add(ticks(user)));
                sample.cpu = previous.and_then(|last| cpu_percent(last, now));
                previous = Some(now);
            } else {
                previous = None;
            }
            let mut memory = MEMORYSTATUSEX {
                dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
                ..Default::default()
            };
            if GlobalMemoryStatusEx(&mut memory).is_ok() {
                sample.memory = Some(memory.dwMemoryLoad as f64);
            }
            let mut power = SYSTEM_POWER_STATUS::default();
            if GetSystemPowerStatus(&mut power).is_ok() {
                if power.BatteryFlag != 255
                    && power.BatteryFlag & 128 == 0
                    && power.BatteryLifePercent <= 100
                {
                    sample.battery = Some(power.BatteryLifePercent);
                }
                sample.charging = match power.ACLineStatus {
                    0 => Some(false),
                    1 => Some(true),
                    _ => None,
                };
            }
            sample.online = network
                .as_ref()
                .and_then(|n| n.GetConnectivity().ok())
                .map(|v| {
                    v.0 & (NLM_CONNECTIVITY_IPV4_INTERNET.0 | NLM_CONNECTIVITY_IPV6_INTERNET.0) != 0
                });
        }
        if tx.try_send(sample).is_err() && stop.load(Ordering::Relaxed) {
            break;
        }
        let until = Instant::now() + Duration::from_secs(5);
        while Instant::now() < until && !stop.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cpu_counters_validate_deltas() {
        assert_eq!(cpu_percent((100, 200), (125, 300)), Some(75.));
        assert_eq!(cpu_percent((100, 200), (100, 200)), None);
        assert_eq!(cpu_percent((100, 200), (99, 300)), None);
        assert_eq!(cpu_percent((100, 200), (300, 300)), None);
    }
}
