use std::{cell::RefCell, usize};

use timings::{cpu_time, cpu_timer_freq, cpu_to_duration};

mod timings;

const MAX_TIMERS: usize = 4096;
const TOP_LEVEL: usize = 0;

thread_local! {
    pub static PROFILER: RefCell<Profiler> = const { RefCell::new(Profiler::new()) };
}

struct TimerStack {
    sp: usize,
    stack: [usize; MAX_TIMERS],
}

impl TimerStack {
    const fn new() -> Self {
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
    PROFILER.set(Profiler::new());
}

#[derive(Debug)]
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
                "{:05.2}% of parent, ",
                (100 * self.elapsed) as f64 / parent_elapsed as f64,
            )
        } else {
            "".to_string()
        };

        let p_vals = format!("{:.4}ms {:padding$} cycles ({}{:05.2}% of total)", 
            cpu_to_duration(self.elapsed as u64).as_secs_f64() * 1_000.0,
            self.elapsed,
            p_parent,
            (100 * self.elapsed) as f64 / total_elapsed as f64,
            padding = (total_elapsed.checked_ilog10().unwrap_or(0) + 1) as usize,
            );

        let indent = level * 4 + 4;
        let padding = 35 - self.name.len() - indent;
        println!(
            "{:indent$}{}: {:padding$}{p_vals}",
            "",
            self.name,
            "",
            padding = padding,
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
    ordered: [usize; MAX_TIMERS],
    num_timers: usize,
    timer_stack: TimerStack,
}

impl Profiler {
    const fn new() -> Self {
        Self {
            timers: [const { None }; MAX_TIMERS],
            ordered: [0; MAX_TIMERS],
            num_timers: 0,
            timer_stack: TimerStack::new(),
        }
    }

    pub fn start(&mut self, name: &'static str, id: usize) -> TimerStartHandle {
        if self.is_recursive_call(id) {
            return TimerStartHandle { started: false };
        }

        // println!("TP: {}", tp);
        if self.timers[id].is_none() {
            let timer = Timer::new(name, self.timer_stack.peek(), id);
            self.timers[id] = Some(timer);
            self.ordered[self.num_timers] = id;
            self.num_timers += 1;
        }

        self.timer_stack.push(id);
        self.timers[id].as_mut().unwrap().start();

        TimerStartHandle { started: true }
    }

    fn is_recursive_call(&self, id: usize) -> bool {
        self.timer_stack
            .stack
            .iter()
            .take(self.timer_stack.sp)
            .find(|i| **i == id)
            .is_some()
    }

    pub fn stop(&mut self) {
        let t = self.timer_stack.pop();
        self.timers[t].as_mut().unwrap().stop();
    }
    
    fn report(&self, level: usize, parent: usize, total_elapsed: u64, parent_elapsed: Option<u64>) {
        let curr_level = self
            .ordered
            .iter()
            .take(self.num_timers)
            .map(|id| self.timers[*id].as_ref().unwrap())
            .filter(|t| t.parent == parent);
        

        let total_elapsed = if parent == 0 {
            let mut top_level_elapsed = 0;
            for timer in curr_level.clone() {
                top_level_elapsed += timer.elapsed as u64;
            }

            let pre = "Total time";
            let padding = 35 - pre.len();
            println!(
                "{pre}: {:padding$}{:.4}ms {} cycles (CPU freq {})",
                "",
                cpu_to_duration(top_level_elapsed).as_secs_f64() * 1_000.0,
                top_level_elapsed,
                cpu_timer_freq()
            );

            top_level_elapsed
        } else {
            total_elapsed
        };

        for timer in curr_level {
            timer.report(level, total_elapsed, parent_elapsed);
            self.report(level + 1, timer.id, total_elapsed, Some(timer.elapsed as u64));
        }
    }
}
