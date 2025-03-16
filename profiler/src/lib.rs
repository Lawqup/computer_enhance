use std::{cell::RefCell, rc::Rc};

use timings::{cpu_time, cpu_timer_freq, cpu_to_duration};

pub mod timings;

pub struct Timer {
    name: &'static str,
    start: u64,
    end: u64,
}

impl Timer {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            start: 0,
            end: 0,
        }
    }

    pub fn start(&mut self) {
        self.start = cpu_time();
    }

    pub fn stop(&mut self) {
        self.end = cpu_time();
    }

    pub fn report(&self, total: u64) {
        let elapsed = self.end - self.start;
        println!(
            "{}: {:.4}ms {} cycles ({:.2}%)",
            self.name,
            cpu_to_duration(self.end - self.start).as_secs_f64() * 1_000.0,
            elapsed,
            (100 * elapsed) as f64 / total as f64,
        );
    }

    pub fn report_standalone(&self) {
        let elapsed = self.end - self.start;
        println!(
            "{}: {:.4}ms {} cycles",
            self.name,
            cpu_to_duration(self.end - self.start).as_secs_f64() * 1_000.0,
            elapsed,
        );
    }
}

pub struct Profiler {
    sub_timers: Vec<Rc<RefCell<Timer>>>,
}

impl Profiler {
    pub fn new() -> Self {
        Self {
            sub_timers: Vec::new(),
        }
    }

    pub fn register(&mut self, name: &'static str) -> Rc<RefCell<Timer>> {
        let sub_timer = Rc::new(RefCell::new(Timer::new(name)));
        self.sub_timers.push(sub_timer.clone());
        sub_timer
    }

    pub fn report(&self) {
        let total = self
            .sub_timers
            .iter()
            .map(|t| {
                let t = t.borrow();
                t.end - t.start
            })
            .sum();

        println!(
            "Total time: {:.4}ms {} cycles (CPU freq {})",
            cpu_to_duration(total).as_secs_f64() * 1_000.0,
            total,
            cpu_timer_freq()
        );

        for timer in self.sub_timers.iter().cloned() {
            timer.borrow().report(total);
        }
    }
}
