use std::{
    io::{stdout, Write},
    time::Duration,
};

use profiler::metrics::{cpu_time, cpu_to_duration, duration_to_cpu, pagefaults};

#[derive(Default, Clone)]
struct Metrics {
    pagefaults: i64,
    bytes_processed: i64,
    time_elapsed: i64,
    trial_count: u32,
}

struct TestResults {
    min: Metrics,
    max: Metrics,
    total: Metrics,
}

pub struct RepetitionTester {
    end_time: u64,
    expected_bytes_processed: u64,
    curr: Metrics,
    results: TestResults,
    state: TesterState,
}

#[derive(PartialEq)]
enum TesterState {
    NotStarted,
    TrialCompleted,
    Testing,
}

impl Metrics {
    pub fn print_result(&mut self, label: &'static str) {
        let divisor = (self.trial_count + 1) as f64;

        let time_elapsed = self.time_elapsed as u64 / divisor as u64;
        let pagefaults = self.pagefaults as f64 / divisor;
        let bytes_processed = self.bytes_processed as f64 / divisor;

        let p_data = if bytes_processed > 0.0 {
            const MB: usize = 1024 * 1024;
            const GB: usize = MB * 1024;
            format!(
                ", {:.3}mb {:.2}gb/s",
                bytes_processed / MB as f64,
                bytes_processed / GB as f64 / cpu_to_duration(time_elapsed).as_secs_f64()
            )
        } else {
            "".to_string()
        };

        let p_flts = if self.pagefaults > 0 {
            const KB: usize = 1024;

            format!(
                ", PF: {:.4} ({:.4}k/fault)",
                pagefaults,
                bytes_processed / (pagefaults * KB as f64)
            )

            // "".to_string()
        } else {
            "".to_string()
        };

        print!(
            "{label} time {:09.4}ms{p_data}{p_flts}",
            cpu_to_duration(time_elapsed).as_secs_f64() * 1_000.0
        );

        let _ = stdout().flush();
    }
}
impl TestResults {
    fn new() -> Self {
        let min = Metrics {
            pagefaults: i64::MAX,
            bytes_processed: i64::MAX,
            time_elapsed: i64::MAX,
            trial_count: 0,
        };

        Self {
            min,
            max: Metrics::default(),
            total: Metrics::default(),
        }
    }
}

impl RepetitionTester {
    pub fn new(test_dur: Duration, expected_bytes_processed: u64) -> Self {
        Self {
            end_time: cpu_time() + duration_to_cpu(test_dur),
            expected_bytes_processed,
            curr: Metrics::default(),
            results: TestResults::new(),
            state: TesterState::NotStarted,
        }
    }

    pub fn run_new_trial(&mut self) -> bool {
        if self.state == TesterState::Testing {
            self.results.total.bytes_processed += self.curr.bytes_processed;
            self.results.total.time_elapsed += self.curr.time_elapsed;
            self.results.total.pagefaults += self.curr.pagefaults;

            if self.curr.time_elapsed > self.results.max.time_elapsed {
                self.results.max = self.curr.clone();
            }

            if self.curr.time_elapsed < self.results.min.time_elapsed {
                self.results.min = self.curr.clone();
            }
        }

        if cpu_time() >= self.end_time {
            if self.expected_bytes_processed != self.curr.bytes_processed as u64 {
                panic!(
                    "Trial finished with different number of bytes read ({}, expected {})",
                    self.curr.bytes_processed, self.expected_bytes_processed
                );
            }

            self.state = TesterState::TrialCompleted;
            print!("\r                                                                                          \r");
            self.results.min.print_result("Min");
            println!();
            self.results.max.print_result("Max");
            println!();
            self.results.total.print_result("Avg");
            println!();

            return false;
        }

        match self.state {
            TesterState::NotStarted => self.state = TesterState::Testing,
            TesterState::TrialCompleted => {}
            TesterState::Testing => {
                print!("\r                                                                                     \r");
                // print("Trial 1: Min time 0157.3855ms, 1064.356mb 6.60gb/s, PF: 68119 (15k/fault)");
                print!("Trial {}: ", self.results.total.trial_count);
                self.results.min.print_result("Min");
            }
        }

        self.results.total.trial_count += 1;
        self.curr = Metrics::default();
        self.state = TesterState::Testing;

        true
    }

    pub fn start_trial_timer(&mut self) {
        self.curr.time_elapsed -= cpu_time() as i64;
        self.curr.pagefaults -= pagefaults() as i64;
    }

    pub fn end_trial_timer(&mut self) {
        self.curr.time_elapsed += cpu_time() as i64;
        self.curr.pagefaults += pagefaults() as i64;
    }

    pub fn count_bytes(&mut self, bytes: u64) {
        self.curr.bytes_processed += bytes as i64;
    }
}

#[cfg(test)]
mod tests {
    use libc::VM_FLAGS_SUPERPAGE_SIZE_2MB;
    use mach2::{traps::mach_task_self, vm_statistics::VM_FLAGS_ANYWHERE};

    use crate::{generate::gen_input, read_to_string_fast};

    #[cfg(feature = "mmap_alloc")]
    use crate::util::uninit_vec;

    use super::*;

    use core::slice;
    use std::{
        ffi::c_void, io::Read, os::unix::fs::MetadataExt, path::Path, ptr::null_mut, sync::Mutex,
    };
    static FILE_LOCK: Mutex<()> = Mutex::new(());

    const SAMPLES: u64 = 10_000_000;
    const TEST_DUR: Duration = Duration::from_secs(10);

    fn get_file() -> String {
        let _lock = FILE_LOCK.lock().unwrap();

        const UNIFORM: bool = false;

        let path = format!(
            "inputs/test_input_{}_{}.f64",
            SAMPLES,
            if UNIFORM { "uniform" } else { "cluster" }
        );

        if !Path::new(&path).exists() {
            gen_input(&path, UNIFORM, SAMPLES).expect("Failed to generate input");
        }

        path
    }

    fn run_test<T>(test: T)
    where
        T: Fn(&str, &mut RepetitionTester),
    {
        let path = &get_file();
        let infile = std::fs::File::open(path).unwrap();

        let mut tester = RepetitionTester::new(TEST_DUR, infile.metadata().unwrap().size());
        while tester.run_new_trial() {
            test(path, &mut tester)
        }
    }

    fn run_test_prealloc<T>(test: T)
    where
        T: Fn(&str, &mut RepetitionTester, &mut [u8]),
    {
        let path = &get_file();
        let infile = std::fs::File::open(path).unwrap();

        let total_size = infile.metadata().unwrap().size();

        let mut tester = RepetitionTester::new(TEST_DUR, total_size);

        let mut buf = vec![0; total_size as usize];
        while tester.run_new_trial() {
            test(path, &mut tester, &mut buf)
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

            tester.start_trial_timer();
            let mut size_remaining = infile.metadata().unwrap().size();
            let mut data = vec![0; size_remaining as usize];
            let mut pos = 0;

            while size_remaining > 0 {
                let n = infile.read(&mut data[pos..]).unwrap();

                size_remaining -= n as u64;
                pos += n;
            }
            tester.end_trial_timer();

            tester.count_bytes(pos as u64);
        });
    }

    #[test]
    fn repeat_read_fast() {
        run_test(|path, tester| {
            let mut infile = std::fs::File::open(path).unwrap();

            tester.start_trial_timer();
            let out = read_to_string_fast(&mut infile);
            tester.end_trial_timer();

            tester.count_bytes(out.len() as u64);
        });
    }

    #[test]
    fn repeat_various() {
        for _ in 0..2 {
            println!("\nRead:");
            run_test_prealloc(|path, tester, data| {
                let mut infile = std::fs::File::open(path).unwrap();

                let mut size_remaining = infile.metadata().unwrap().size();
                let mut pos = 0;

                while size_remaining > 0 {
                    tester.start_trial_timer();
                    let n = infile.read(&mut data[pos..]).unwrap();
                    tester.end_trial_timer();

                    size_remaining -= n as u64;
                    pos += n;
                }

                tester.count_bytes(pos as u64);
            });

            println!("\nRead + alloc (initialized):");
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
                }

                tester.count_bytes(pos as u64);
            });

            #[cfg(feature = "mmap_alloc")]
            {
                println!("\nRead + alloc (uninitialized):");
                run_test(|path, tester| {
                    let mut infile = std::fs::File::open(path).unwrap();

                    let mut size_remaining = infile.metadata().unwrap().size();
                    let mut data = unsafe { uninit_vec(size_remaining as usize) };
                    let mut pos = 0;

                    while size_remaining > 0 {
                        tester.start_trial_timer();
                        let n = infile.read(&mut data[pos..]).unwrap();
                        tester.end_trial_timer();

                        size_remaining -= n as u64;
                        pos += n;
                    }

                    tester.count_bytes(pos as u64);
                });
            }

            #[cfg(feature = "mmap_alloc")]
            {
                println!("\nRead + alloc + prefetch:");
                run_test(|path, tester| {
                    let mut infile = std::fs::File::open(path).unwrap();

                    let mut size_remaining = infile.metadata().unwrap().size();
                    let mut data = unsafe { uninit_vec(size_remaining as usize) };
                    let mut pos = 0;

                    unsafe {
                        libc::posix_madvise(
                            data.as_mut_ptr() as *mut c_void,
                            data.len(),
                            libc::POSIX_MADV_WILLNEED,
                        );
                    };

                    while size_remaining > 0 {
                        tester.start_trial_timer();
                        let n = infile.read(&mut data[pos..]).unwrap();
                        tester.end_trial_timer();

                        size_remaining -= n as u64;
                        pos += n;
                    }

                    tester.count_bytes(pos as u64);
                });
            }

            // Macos superpages not supported on apple silicon
            #[cfg(none)]
            {
                println!("\nRead + alloc (hugepages):");
                run_test(|path, tester| {
                    let mut infile = std::fs::File::open(path).unwrap();

                    let mut size_remaining = infile.metadata().unwrap().size();
                    let mut data = unsafe { uninit_vec(size_remaining as usize) };

                    let buf = unsafe {
                        let addr = 0;
                        mach2::vm::mach_vm_allocate(
                            mach_task_self(),
                            &addr as *const _ as *mut _,
                            size_remaining,
                            VM_FLAGS_ANYWHERE | VM_FLAGS_SUPERPAGE_SIZE_2MB,
                        );
                        slice::from_raw_parts_mut(addr as *mut u8, size_remaining as usize)
                    };

                    let mut pos = 0;

                    while size_remaining > 0 {
                        tester.start_trial_timer();
                        let n = infile.read(&mut data[pos..]).unwrap();
                        tester.end_trial_timer();

                        size_remaining -= n as u64;
                        pos += n;
                    }

                    tester.count_bytes(pos as u64);

                    unsafe {
                        mach2::vm::mach_vm_deallocate(
                            mach_task_self(),
                            buf.as_mut_ptr() as u64,
                            buf.len() as u64,
                        );
                    }
                });
            }
        }
    }

    #[test]
    fn probe_linear_alloc() {
        const NUM_PAGES: usize = 1024;
        const PAGE_SIZE: usize = 16384;

        const TOTAL_SIZE: usize = NUM_PAGES * PAGE_SIZE;

        for touched_pages in 0..=NUM_PAGES {
            let buf = unsafe {
                match libc::mmap(
                    null_mut(),
                    TOTAL_SIZE,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_SHARED | libc::MAP_ANONYMOUS,
                    -1,
                    0,
                ) {
                    libc::MAP_FAILED => panic!("Failed to map memory"),
                    ptr => slice::from_raw_parts_mut(ptr as *mut _, TOTAL_SIZE),
                }
            };

            let to_write = touched_pages * PAGE_SIZE;

            let start_flts = pagefaults();
            for j in 0..to_write {
                buf[j] = (j % u8::MAX as usize) as u8;
            }
            let flts = pagefaults() - start_flts;

            println!(
                "{touched_pages}, {flts}, {}",
                flts as i64 - touched_pages as i64
            );

            unsafe {
                libc::munmap(buf.as_mut_ptr() as *mut c_void, TOTAL_SIZE);
            }
        }
    }
}
