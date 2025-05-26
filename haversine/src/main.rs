use core::panic;
use std::io;

use profiler::metrics::{cpu_time, cpu_to_duration};

pub mod calc;
pub mod generate;
pub mod parse;
pub mod repetition_tester;
pub mod util;

#[cfg(feature = "mmap_alloc")]
pub mod allocator;

pub use util::*;

fn main() -> io::Result<()> {
    let start = cpu_time();
    let mut uniform = true;
    let mut samples: Option<u64> = None;

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-u" | "--uniform" => uniform = true,
            "-c" | "--cluster" => uniform = false,
            _ => {
                if let Ok(n) = arg.parse() {
                    samples = Some(n)
                } else {
                    panic!("Bad args");
                }
            }
        }
    }
    let samples = samples.unwrap();

    test_samples(uniform, samples);

    println!(
        "Total time elapsed: {:09.4}ms",
        cpu_to_duration(cpu_time() - start).as_secs_f64() * 1_000.0
    );
    Ok(())
}
