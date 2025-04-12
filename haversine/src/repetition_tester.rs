use std::{cmp, io::{stdout, Write}, time::Duration, u64};

use profiler::timings::{cpu_time, cpu_to_duration, duration_to_cpu};

pub struct RepetitionTester {
    end_time: u64,
    curr_min: u64,
    curr_max: u64,
    total_elapsed: u64,
    trial_start_time: u64,
    trial_bytes_processed: u64,
    expected_bytes_processed: u64,
    n_trials: u32,
    state: TesterState,
}

enum TesterState {
    TrialCompleted,
    Testing,
}

impl RepetitionTester {
    pub fn new(test_dur: Duration, expected_bytes_processed: u64) -> Self {
        Self {
            end_time: cpu_time() + duration_to_cpu(test_dur),
            curr_min: u64::MAX,
            curr_max: 0,
            total_elapsed: 0,
            trial_start_time: 0,
            trial_bytes_processed: 0,
            expected_bytes_processed,
            n_trials: 0,
            state: TesterState::Testing,
        }
    }

    pub fn run_new_trial(&mut self) -> bool {
        self.trial_start_time = cpu_time();
        if self.trial_start_time >= self.end_time {
            if self.expected_bytes_processed != self.trial_bytes_processed {
                panic!(
                    "Trial finished with different number of bytes read ({}, expected {})",
                    self.trial_bytes_processed, self.expected_bytes_processed
                );
            }
            
            self.state = TesterState::TrialCompleted;
            print!("                                                          \r");
            self.print_result("Min", self.curr_min);
            println!();
            self.print_result("Max", self.curr_max);
            println!();
            self.print_result("Avg", self.total_elapsed / self.n_trials as u64);
            println!();

            return false;
        }


        match self.state {
            TesterState::TrialCompleted => {},
            TesterState::Testing => {
                print!("Trial {}: ", self.n_trials);
                self.print_result("Min", self.curr_min);
            }
        }

        self.n_trials += 1;
        self.trial_bytes_processed = 0;
        self.state = TesterState::Testing;

        true
    }

    pub fn start_trial_timer(&mut self) {
        self.trial_start_time = cpu_time();
    }

    pub fn end_trial_timer(&mut self) {
        let elapsed = cpu_time() - self.trial_start_time;

        self.total_elapsed += elapsed;
        self.curr_min = cmp::min(self.curr_min, elapsed);
        self.curr_max = cmp::max(self.curr_max, elapsed);
    }

    pub fn count_bytes(&mut self, bytes: u64) {
        self.trial_bytes_processed += bytes;
    }

    pub fn print_result(&self, label: &'static str, result: u64) {
        let p_data = if self.trial_bytes_processed > 0 {
            const MB: usize = 1024 * 1024;
            const GB: usize = MB * 1024;
            format!(
                ", {:.3}mb {:.2}gb/s",
                self.trial_bytes_processed as f64 / MB as f64,
                self.trial_bytes_processed as f64
                    / GB as f64
                    / cpu_to_duration(result).as_secs_f64()
            )
        } else {
            "".to_string()
        };

        print!(
            "{label} time {:09.4}ms{p_data}",
            cpu_to_duration(result).as_secs_f64() * 1_000.0
        );
        print!("                                                          \r");

        let _ = stdout().flush();
    }
}

#[cfg(test)]
mod tests {
    use crate::generate::gen_input;

    use super::*;

    use std::{io::Read, os::unix::fs::MetadataExt, path::Path, sync::Mutex};
    static FILE_LOCK: Mutex<()> = Mutex::new(());

    fn run_test<T>(test: T)
    where
        T: Fn(&str, &mut RepetitionTester),
    {
        let _lock = FILE_LOCK.lock().unwrap();

        let samples = 10_000_000;
        let uniform = false;

        let path = &format!(
            "inputs/test_input_{}_{}.f64",
            samples,
            if uniform { "uniform" } else { "cluster" }
        );

        if !Path::new(path).exists() {
            gen_input(path, uniform, samples).expect("Failed to generate input");
        }

        let infile = std::fs::File::open(path).unwrap();
        let mut tester =
            RepetitionTester::new(Duration::from_secs(10), infile.metadata().unwrap().size());
        while tester.run_new_trial() {
            test(path, &mut tester)
        }
    }

    #[test]
    fn repeat_read_to_string() {
        run_test(|path, tester| {
            let mut infile = std::fs::File::open(path).unwrap();
            let mut data = String::with_capacity(infile.metadata().unwrap().size() as usize);
            tester.start_trial_timer();
            let bytes = infile.read_to_string(&mut data).unwrap();
            tester.end_trial_timer();

            tester.count_bytes(bytes as u64);
        });
    }

    #[test]
    fn repeat_raw_read() {
        run_test(|path, tester| {
            let mut infile = std::fs::File::open(path).unwrap();
            let mut size_remaining = infile.metadata().unwrap().size();
            let mut data = vec![0; size_remaining as usize];
            let mut pos = 0;

            while size_remaining > 0 {
                tester.start_trial_timer();
                let n = infile.read(&mut data[pos..]).unwrap();
                tester.end_trial_timer();
                
                size_remaining -= n as u64;
                pos += n;
                tester.count_bytes(n as u64);
            }

        });
    }
}
