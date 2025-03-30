use std::{borrow::BorrowMut, cell::RefCell, convert::identity, usize};

use timings::{cpu_time, cpu_timer_freq, cpu_to_duration};

mod timings;

const MAX_TIMERS: usize = 4096;

thread_local! {
    pub static PROFILER: RefCell<Profiler> = const { RefCell::new(Profiler::new()) };
}

pub fn profile_report() {
    PROFILER.with(|p| p.borrow().report());
}

pub fn clear_profiler() {
    PROFILER.set(Profiler::new());
}

fn num_digits(num: u64) -> usize {
    (num.checked_ilog10().unwrap_or(0) + 1) as usize
}

#[derive(Debug)]
pub struct ProfileNode {
    name: &'static str,
    elapsed_exclusive: i64,
    elapsed_inclusive: u64,
    calls: u64,
}

impl ProfileNode {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            elapsed_exclusive: 0,
            elapsed_inclusive: 0,
            calls: 0,
        }
    }

    pub fn report(&self, total_elapsed: u64) {
        let p_exclusive = if self.elapsed_exclusive as u64 != self.elapsed_inclusive {
            format!(
                ", {} cycles ({:05.2}%) excluding children",
                self.elapsed_exclusive,
                (100 * self.elapsed_exclusive) as f64 / total_elapsed as f64
            )
        } else {
            "".to_string()
        };

        let p_vals = format!(
            "{:09.4}ms {:padding$} cycles ({:05.2}%){p_exclusive}",
            cpu_to_duration(self.elapsed_inclusive as u64).as_secs_f64() * 1_000.0,
            self.elapsed_inclusive,
            (100 * self.elapsed_inclusive) as f64 / total_elapsed as f64,
            padding = num_digits(total_elapsed),
        );

        let padding = 35 - self.name.len() - num_digits(self.calls);
        println!(
            "{}[{}]: {:padding$}{p_vals}",
            self.name,
            self.calls,
            "",
            padding = padding,
        );
    }
}

pub struct ProfiledBlock {
    start: u64,
    root_elapsed: u64,
    node_id: usize,
    parent_node_id: usize,
}

impl ProfiledBlock {
    pub fn new(name: &'static str, id: usize) -> Self {
        PROFILER.with(|p| {
            let mut p = p.borrow_mut();
            let parent_node_id = p.call_node(name, id);
            Self {
                start: cpu_time(),
                root_elapsed: p.timers[id].as_ref().unwrap().elapsed_inclusive,
                node_id: id,
                parent_node_id,
            }
        })
    }
}

impl Drop for ProfiledBlock {
    fn drop(&mut self) {
        PROFILER.with(|p| {
            let mut p = p.borrow_mut();
            let node = p.timers[self.node_id].as_mut().unwrap();

            let elapsed = cpu_time() - self.start;
            node.elapsed_exclusive += elapsed as i64;
            node.elapsed_inclusive = self.root_elapsed + elapsed;

            if self.parent_node_id != 0 {
                let parent = p.timers[self.parent_node_id].as_mut().unwrap();
                parent.elapsed_exclusive -= elapsed as i64;
            }

            p.parent_node = self.parent_node_id;
        })
    }
}

pub struct Profiler {
    timers: [Option<ProfileNode>; MAX_TIMERS],
    ordered: [usize; MAX_TIMERS],
    parent_node: usize,
    num_timers: usize,
    first_start: u64,
}

impl Profiler {
    const fn new() -> Self {
        Self {
            timers: [const { None }; MAX_TIMERS],
            ordered: [0; MAX_TIMERS],
            parent_node: 0,
            num_timers: 0,
            first_start: 0,
        }
    }

    pub fn call_node(&mut self, name: &'static str, id: usize) -> usize {
        if self.timers[id].is_none() {
            if self.num_timers == 0 {
                self.first_start = cpu_time();
            }

            let timer = ProfileNode::new(name);
            self.timers[id] = Some(timer);
            self.ordered[self.num_timers] = id;
            self.num_timers += 1;
        }

        self.timers[id].as_mut().unwrap().calls += 1;
        let prev_par = self.parent_node;
        self.parent_node = id;
        prev_par
    }

    fn report(&self) {
        let total_elapsed = cpu_time() - self.first_start;

        let pre = "Total time";
        let padding = 37 - pre.len();
        println!(
            "{pre}: {:padding$}{:09.4}ms {} cycles (CPU freq {})",
            "",
            cpu_to_duration(total_elapsed).as_secs_f64() * 1_000.0,
            total_elapsed,
            cpu_timer_freq()
        );

        for id in &self.ordered[..self.num_timers] {
            self.timers[*id].as_ref().unwrap().report(total_elapsed);
        }
    }
}
