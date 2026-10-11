use std::collections::HashMap;
use windows::{
    Win32::System::Performance::*,
    core::{PCWSTR, w},
};

pub struct GpuSampler {
    query: PDH_HQUERY,
    counter: PDH_HCOUNTER,
    primed: bool,
}

impl GpuSampler {
    pub fn new() -> Option<Self> {
        let mut query = PDH_HQUERY::default();
        if unsafe { PdhOpenQueryW(PCWSTR::null(), 0, &mut query) } != 0 {
            return None;
        }
        let mut sampler = Self {
            query,
            counter: PDH_HCOUNTER::default(),
            primed: false,
        };
        if unsafe {
            PdhAddEnglishCounterW(
                query,
                w!("\\GPU Engine(*)\\Utilization Percentage"),
                0,
                &mut sampler.counter,
            )
        } != 0
        {
            return None;
        }
        Some(sampler)
    }

    pub fn sample(&mut self) -> Option<f64> {
        if unsafe { PdhCollectQueryData(self.query) } != 0 {
            self.primed = false;
            return None;
        }
        if !std::mem::replace(&mut self.primed, true) {
            return None;
        }
        // 实例列表会随进程变化；一次长度竞争可重读，缓冲区和重试次数均有上限。
        for _ in 0..2 {
            let mut bytes = 0;
            let mut count = 0;
            if unsafe {
                PdhGetFormattedCounterArrayW(
                    self.counter,
                    PDH_FMT_DOUBLE,
                    &mut bytes,
                    &mut count,
                    None,
                )
            } != PDH_MORE_DATA
                || bytes == 0
                || bytes > 4 * 1024 * 1024
            {
                return None;
            }
            let item_size = std::mem::size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>();
            let mut buffer =
                vec![PDH_FMT_COUNTERVALUE_ITEM_W::default(); (bytes as usize).div_ceil(item_size)];
            let status = unsafe {
                PdhGetFormattedCounterArrayW(
                    self.counter,
                    PDH_FMT_DOUBLE,
                    &mut bytes,
                    &mut count,
                    Some(buffer.as_mut_ptr()),
                )
            };
            if status == PDH_MORE_DATA {
                continue;
            }
            if status != 0 || count as usize > buffer.len() {
                return None;
            }
            let start = buffer.as_ptr() as usize;
            let end = start + buffer.len() * item_size;
            let mut values = Vec::new();
            for item in &buffer[..count as usize] {
                if !matches!(
                    item.FmtValue.CStatus,
                    PDH_CSTATUS_VALID_DATA | PDH_CSTATUS_NEW_DATA
                ) {
                    continue;
                }
                let address = item.szName.0 as usize;
                if address < start || address >= end || !address.is_multiple_of(2) {
                    continue;
                }
                let name =
                    unsafe { std::slice::from_raw_parts(item.szName.0, (end - address) / 2) };
                let Some(length) = name.iter().position(|c| *c == 0) else {
                    continue;
                };
                let Ok(name) = String::from_utf16(&name[..length]) else {
                    continue;
                };
                values.push((name, unsafe { item.FmtValue.Anonymous.doubleValue }));
            }
            return busiest_engine(values.iter().map(|(name, value)| (name.as_str(), *value)));
        }
        None
    }
}

impl Drop for GpuSampler {
    fn drop(&mut self) {
        unsafe {
            PdhCloseQuery(self.query);
        }
    }
}

fn busiest_engine<'a>(values: impl Iterator<Item = (&'a str, f64)>) -> Option<f64> {
    let mut engines = HashMap::<&str, f64>::new();
    for (name, value) in values {
        if !value.is_finite() || value < 0. {
            continue;
        }
        let Some((pid, engine)) = name.split_once("_luid_") else {
            continue;
        };
        if !pid
            .strip_prefix("pid_")
            .is_some_and(|id| !id.is_empty() && id.bytes().all(|c| c.is_ascii_digit()))
            || !engine.contains("_phys_")
            || !engine.contains("_eng_")
        {
            continue;
        }
        // 同一硬件引擎的进程用量相加，再取最忙引擎；不同引擎并行执行，不能全部相加。
        *engines.entry(engine).or_default() += value;
    }
    engines
        .values()
        .copied()
        .reduce(f64::max)
        .map(|v| v.clamp(0., 100.))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_combines_processes_but_not_parallel_engines_or_adapters() {
        let values = [
            ("pid_1_luid_0x0_0x1_phys_0_eng_0_engtype_3D", 25.),
            ("pid_2_luid_0x0_0x1_phys_0_eng_0_engtype_3D", 35.),
            ("pid_1_luid_0x0_0x1_phys_0_eng_1_engtype_Copy", 50.),
            ("pid_1_luid_0x0_0x2_phys_0_eng_0_engtype_3D", 45.),
        ];
        assert_eq!(busiest_engine(values.into_iter()), Some(60.));
        assert_eq!(
            busiest_engine([(values[0].0, 125.)].into_iter()),
            Some(100.)
        );
        assert_eq!(busiest_engine([(values[0].0, 0.)].into_iter()), Some(0.));
        for invalid in [f64::NAN, f64::INFINITY, -1.] {
            assert_eq!(busiest_engine([(values[0].0, invalid)].into_iter()), None);
        }
        assert_eq!(
            busiest_engine([("_Total", 80.), ("pid__luid_0_phys_0_eng_0", 30.)].into_iter()),
            None
        );
        assert_eq!(busiest_engine(std::iter::empty()), None);
    }

    #[test]
    #[ignore = "需要本机 GPU 性能计数器，仅手动运行"]
    fn real_gpu_counter_read_only() {
        let mut sampler = GpuSampler::new().expect("本机 GPU 性能计数器不可用");
        assert_eq!(sampler.sample(), None);
        std::thread::sleep(std::time::Duration::from_secs(5));
        let value = sampler.sample().expect("本机 GPU 未返回有效数据");
        assert!((0. ..=100.).contains(&value));
        println!("真实 GPU 引擎使用率：{value:.1}%");
    }
}
