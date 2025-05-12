use std::{arch::asm, mem::MaybeUninit, time::Duration};

pub fn cpu_time() -> u64 {
    let mut x: u64;
    unsafe {
        asm! (
            "MRS {}, CNTVCT_EL0",
            out(reg) x,
        );
    }

    x
}

pub fn cpu_timer_freq() -> u64 {
    let mut x: u64;
    unsafe {
        asm! (
            "MRS {}, CNTFRQ_EL0",
            out(reg) x,
        );
    }

    x
}

pub fn pagefaults() -> u64 {
    let mut usage = MaybeUninit::uninit();
    unsafe {
        libc::getrusage(0, usage.as_mut_ptr());
        let usage = usage.assume_init();

        usage.ru_minflt as u64 + usage.ru_majflt as u64
    }
}

pub fn cpu_to_duration(cpu: u64) -> Duration {
    const SECS_TO_NANOS: u128 = 1_000_000_000;
    Duration::from_nanos((cpu as u128 * SECS_TO_NANOS/cpu_timer_freq() as u128) as u64)
}

pub fn duration_to_cpu(dur: Duration) -> u64 {
    const SECS_TO_NANOS: u128 = 1_000_000_000;
    ((dur.as_nanos() * cpu_timer_freq() as u128) / SECS_TO_NANOS) as u64
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    #[test]
    fn test_cpu_timer() {
        let now = Instant::now();
        let start = cpu_time();

        const TEST_DUR_MILLIS: u64 = 100;

        while now.elapsed() < Duration::from_millis(TEST_DUR_MILLIS) {}
    
        let freq = cpu_timer_freq();
        let end = cpu_time();
        let dur_millis = cpu_to_duration(end - start).as_millis();
        println!("CPU TIMER FREQ {freq}");
        println!("CPU ELAPSED {} ({} -> {})", end - start, start, end);
        println!("TIME ESTIMATE: {}ms", dur_millis);

        assert_eq!(TEST_DUR_MILLIS, dur_millis as u64);
    }
}
