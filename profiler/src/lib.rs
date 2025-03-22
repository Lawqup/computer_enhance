use std::sync::Mutex;

use lazy_static::lazy_static;
use timings::{cpu_time, cpu_timer_freq, cpu_to_duration};

mod timings;

lazy_static! {
    pub static ref profiler: Mutex<Profiler> = Mutex::new(Profiler::new());
}

pub fn profile_report() {
    profiler.lock().unwrap().report()
}

pub struct Timer {
    name: &'static str,
    sum_delta: i64,
}

impl Timer {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            sum_delta: 0,
        }
    }

    pub fn start(&mut self) {
        self.sum_delta -= cpu_time() as i64;
    }

    pub fn stop(&mut self) {
        self.sum_delta += cpu_time() as i64;
    }

    pub fn report(&self, total: u64) {
        println!(
            "{}: {:.4}ms {} cycles ({:.2}%)",
            self.name,
            cpu_to_duration(self.sum_delta as u64).as_secs_f64() * 1_000.0,
            self.sum_delta,
            (100 * self.sum_delta) as f64 / total as f64,
        );
    }

    pub fn report_standalone(&self) {
        println!(
            "{}: {:.4}ms {} cycles",
            self.name,
            cpu_to_duration(self.sum_delta as u64).as_secs_f64() * 1_000.0,
            self.sum_delta,
        );
    }
}

pub struct Profiler {
    sub_timers: [Option<Timer>; 16],
    timers: usize,
}

impl Profiler {
    pub fn new() -> Self {
        Self {
            sub_timers: [const { None }; 16],
            timers: 0,
        }
    }
    
    fn get_timer_mut(&mut self, name: &'static str) -> Option<&mut Timer> {
        if let Some(sub_timer) = self.sub_timers.iter_mut().find(|t| t.as_ref().is_some_and(|t| t.name == name)) {
            sub_timer.as_mut()
        } else {
            None
        }
    }

    pub fn start(&mut self, name: &'static str) {
        let sub_timer: Option<&mut Timer> = if let Some(sub_timer) = self.get_timer_mut(name) {
            Some(sub_timer)
        } else {
            let sub_timer = Some(Timer::new(name));
            self.sub_timers[self.timers] = sub_timer;
            self.timers += 1;
            self.sub_timers[self.timers - 1].as_mut()
        };

        sub_timer.unwrap().start()
    }

    pub fn stop(&mut self, name: &'static str) {
        self.get_timer_mut(name).expect("Could not find sub timer").stop();
    }

    pub fn report(&self) {
        let total = self
            .sub_timers
            .iter()
            .filter_map(|t| t.as_ref().map(|t| t.sum_delta as u64))
            .sum();

        println!(
            "Total time: {:.4}ms {} cycles (CPU freq {})",
            cpu_to_duration(total).as_secs_f64() * 1_000.0,
            total,
            cpu_timer_freq()
        );

        for timer in self.sub_timers.iter().filter_map(|t| t.as_ref()) {
            timer.report(total)
        }
    }
}
