use std::{cell::RefCell, usize};

use timings::{cpu_time, cpu_timer_freq, cpu_to_duration};

mod timings;

const MAX_TIMERS: usize = 4096;
const TOP_LEVEL: usize = 0;

thread_local! {
    pub static PROFILER: RefCell<Profiler> = RefCell::new(Profiler::new());
}

struct TimerStack {
    sp: usize,
    stack: [usize; MAX_TIMERS],
}

impl TimerStack {
    fn new() -> Self {
        TimerStack {
            sp: 0,
            stack: [0; MAX_TIMERS],
        }
    }

    fn push(&mut self, timer: usize) {
        self.stack[self.sp] = timer;
        self.sp += 1;
    }

    fn pop(&mut self) -> usize {
        self.sp -= 1;
        self.stack[self.sp]
    }

    fn peek(&self) -> usize {
        if self.sp == 0 {
            0
        } else {
            self.stack[self.sp - 1]
        }
    }
}

pub fn profile_report() {
    PROFILER.with(|p| p.borrow().report(0, TOP_LEVEL, 0, None));
}

pub fn profile_start(name: &'static str, tp: usize) -> TimerStartHandle {
    PROFILER.with(|p| p.borrow_mut().start(name, tp))
}

pub fn profile_stop() {
    PROFILER.with(|p| p.borrow_mut().stop());
}

pub fn clear_profiler() {
    PROFILER.with(|p| *p.borrow_mut() = Profiler::new());
}

pub struct Timer {
    name: &'static str,
    elapsed: i64,
    parent: usize,
    id: usize,
}

impl Timer {
    pub fn new(name: &'static str, parent: usize, id: usize) -> Self {
        Self {
            name,
            elapsed: 0,
            parent,
            id,
        }
    }

    pub fn start(&mut self) {
        self.elapsed -= cpu_time() as i64;
    }

    pub fn stop(&mut self) {
        self.elapsed += cpu_time() as i64;
    }

    pub fn report(&self, level: usize, total_elapsed: u64, parent_elapsed: Option<u64>) {
        let p_parent = if let Some(parent_elapsed) = parent_elapsed {
            format!(
                "{:.2}% of parent, ",
                (100 * self.elapsed) as f64 / parent_elapsed as f64
            )
        } else {
            "".to_string()
        };

        println!(
            "{:indent$}{}: {:.4}ms {} cycles ({}{:.2}% of total)",
            "",
            self.name,
            cpu_to_duration(self.elapsed as u64).as_secs_f64() * 1_000.0,
            self.elapsed,
            p_parent,
            (100 * self.elapsed) as f64 / total_elapsed as f64,
            indent = level * 4,
        );
    }

    pub fn report_standalone(&self) {
        println!(
            "{}: {:.4}ms {} cycles",
            self.name,
            cpu_to_duration(self.elapsed as u64).as_secs_f64() * 1_000.0,
            self.elapsed,
        );
    }
}

pub struct TimerStartHandle {
    started: bool,
}

impl Drop for TimerStartHandle {
    fn drop(&mut self) {
        if self.started {
            profile_stop()
        }
    }
}

pub struct Profiler {
    timers: [Option<Timer>; MAX_TIMERS],
    timer_stack: TimerStack,
    tp: usize,
}

impl Profiler {
    fn new() -> Self {
        Self {
            timers: [const { None }; MAX_TIMERS],
            timer_stack: TimerStack::new(),
            tp: 1,
        }
    }

    pub fn start(&mut self, name: &'static str, id: usize) -> TimerStartHandle {
        if self.is_recursive_call(id) {
            return TimerStartHandle { started: false };
        }

        let tp = self.tp;
        if self.timers[tp].is_none() {
            let timer = Timer::new(name, self.timer_stack.peek(), id);
            self.timers[tp] = Some(timer)
        }

        self.timer_stack.push(tp);
        self.timers[tp].as_mut().unwrap().start();
        self.tp += 1;

        TimerStartHandle { started: true }
    }

    pub fn is_recursive_call(&self, id: usize) -> bool {
        self.timer_stack
            .stack
            .iter()
            .find(|t| self.timers[**t].as_ref().is_some_and(|t| t.id == id))
            .is_some()
    }

    pub fn stop(&mut self) {
        let t = self.timer_stack.pop();
        self.timers[t].as_mut().unwrap().stop();
    }

    fn report(&self, level: usize, parent: usize, total_elapsed: u64, parent_elapsed: Option<u64>) {
        let curr_level = self
            .timers
            .iter()
            .enumerate()
            .filter_map(|(i, t)| t.as_ref().map(|t| (i, t)))
            .filter(|(_, t)| t.parent == parent);

        let mut curr_elapsed = 0;
        for (_, timer) in curr_level.clone() {
            curr_elapsed += timer.elapsed as u64;
        }

        let total_elapsed = if parent == 0 {
            println!(
                "Total time: {:.4}ms {} cycles (CPU freq {})",
                cpu_to_duration(curr_elapsed).as_secs_f64() * 1_000.0,
                curr_elapsed,
                cpu_timer_freq()
            );

            curr_elapsed
        } else {
            total_elapsed
        };

        for (curr, timer) in curr_level {
            timer.report(level, total_elapsed, parent_elapsed);
            self.report(level + 1, curr, total_elapsed, Some(curr_elapsed));
        }
    }
}
