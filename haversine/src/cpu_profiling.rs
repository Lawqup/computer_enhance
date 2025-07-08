use std::{arch::asm, time::Duration};

use haversine_macro::repeat_asm;
use profiler::metrics::cpu_to_duration;
use rand::{random_iter, rngs::OsRng, TryRngCore};

use crate::repetition_tester::RepetitionTester;
use crate::util::*;

const LOOP_ITERATIONS: usize = 1024 * 1024;
const CPU_FREQ_HZ: u64 = 3_228 * 1_000_000;
const TEST_DUR: Duration = Duration::from_secs(3);

fn test_loop_buf<T>(buf: &Vec<u8>, test: T)
where
    T: Fn(usize, Vec<u8>),
{
    let mut tester = RepetitionTester::new(TEST_DUR, buf.len() as u64);

    while tester.run_new_trial() {
        tester.start_trial_timer();
        test(buf.len(), buf.clone());
        tester.end_trial_timer();

        tester.count_bytes(buf.len() as u64);
    }

    let cycles =
        cpu_to_duration(tester.results.min.time_elapsed as u64).as_secs_f64() * CPU_FREQ_HZ as f64;

    println!("cycles per loop: {}", cycles / buf.len() as f64);
}

fn test_loop<T>(test: T)
where
    T: Fn(usize, Vec<u8>),
{
    let buf = vec![0; LOOP_ITERATIONS];
    test_loop_buf(&buf, test);
}

#[test]
fn profile_write_loop() {
    println!("\nWrite (Rust):");
    test_loop(|count, mut buf| {
        for i in 0..count {
            buf[i] = i as u8;
        }
    });

    println!("\nMov (asm):");
    test_loop(|count, mut buf| unsafe {
        let base_ptr: *mut u8 = buf.as_mut_ptr();

        asm!(
            "mov x8, #0",
            "2:",
            "strb w8, [{base}, x8]",
            "add x8, x8, #1",
            "cmp x8, {count}",
            "b.ne 2b",

            count = in(reg) count,
            base = in(reg) base_ptr,
            out("x8") _,
            options(nostack)
        );
    });

    println!("\nNOP (asm):");
    test_loop(|count, _| unsafe {
        asm!(
            "mov x8, #0",
            "2:",
            "nop",
            "add x8, x8, #1",
            "cmp x8, {count}",
            "b.ne 2b",

            count = in(reg) count,
            out("x8") _,
            options(nostack)
        );
    });

    println!("\nCMP(asm):");
    test_loop(|count, _| unsafe {
        asm!(
            "mov x8, #0",
            "2:",
            "add x8, x8, #1",
            "cmp x8, {count}",
            "b.ne 2b",

            count = in(reg) count,
            out("x8") _,
            options(nostack)
        );
    });
}

#[test]
fn profile_cpu_frontend_ilp() {
    println!("\n1 nop");
    test_loop(|count, _| unsafe {
        asm!(
            "mov x8, #0",
            "2:",
            "nop",
            "add x8, x8, #1",
            "cmp x8, {count}",
            "b.ne 2b",

            count = in(reg) count,
            out("x8") _,
            options(nostack)
        );
    });

    println!("\n2 nops");
    test_loop(|count, _| unsafe {
        asm!(
            "mov x8, #0",
            "2:",
            "nop",
            "nop",
            "add x8, x8, #1",
            "cmp x8, {count}",
            "b.ne 2b",

            count = in(reg) count,
            out("x8") _,
            options(nostack)
        );
    });

    println!("\n4 nops");
    test_loop(|count, _| unsafe {
        asm!(
            "mov x8, #0",
            "2:",
            "nop",
            "nop",
            "nop",
            "nop",
            "add x8, x8, #1",
            "cmp x8, {count}",
            "b.ne 2b",

            count = in(reg) count,
            out("x8") _,
            options(nostack)
        );
    });

    println!("\n8 nops");
    test_loop(|count, _| unsafe {
        asm!(
            "mov x8, #0",
            "2:",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "add x8, x8, #1",
            "cmp x8, {count}",
            "b.ne 2b",

            count = in(reg) count,
            out("x8") _,
            options(nostack)
        );
    });

    println!("\n16 nops");
    test_loop(|count, _| unsafe {
        asm!(
            "mov x8, #0",
            "2:",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "nop",
            "add x8, x8, #1",
            "cmp x8, {count}",
            "b.ne 2b",

            count = in(reg) count,
            out("x8") _,
            options(nostack)
        );
    });
}

#[test]
fn profile_branch_predictor() {
    let filled_bufs = [
        ("Never take branch", vec![0; LOOP_ITERATIONS]),
        ("Always take branch", vec![1; LOOP_ITERATIONS]),
        ("Take branch every 2", [0, 1].repeat(LOOP_ITERATIONS / 2)),
        ("Take branch every 3", [0, 0, 1].repeat(LOOP_ITERATIONS / 3)),
        (
            "Take branch every 4",
            [0, 0, 0, 1].repeat(LOOP_ITERATIONS / 4),
        ),
        ("Rust Rand", random_iter().take(LOOP_ITERATIONS).collect()),
        (
            "OS Rand",
            vec![OsRng.try_next_u32().unwrap() as u8; LOOP_ITERATIONS],
        ),
    ];

    for (desc, filled_buf) in filled_bufs.iter() {
        println!("\n{desc}");
        test_loop_buf(filled_buf, |count, buf| unsafe {
            let base_ptr: *const u8 = buf.as_ptr();

            asm!(
                "mov x8, #0",
                "2:",
                "ldrb w0, [{base}, x8]",
                "add x8, x8, #1",
                "tbnz w0, #0, 3f",
                "nop",
                "3:",
                "cmp x8, {count}",
                "b.ne 2b",

                count = in(reg) count,
                base = in(reg) base_ptr,
                out("x8") _,
                options(nostack)
            );
        });
    }
}

#[test]
fn profile_instr_alignment() {
    println!("\nAligned:");
    test_loop(|count, mut buf| unsafe {
        let base_ptr: *mut u8 = buf.as_mut_ptr();

        asm!(
            "mov x8, #0",
            ".align 7",
            "2:",
            "strb w8, [{base}, x8]",
            "add x8, x8, #1",
            "cmp x8, {count}",
            "b.ne 2b",

            count = in(reg) count,
            base = in(reg) base_ptr,
            out("x8") _,
            options(nostack)
        );
    });

    println!("\nAligned + 4 bytes:");
    test_loop(|count, mut buf| unsafe {
        let base_ptr: *mut u8 = buf.as_mut_ptr();

        asm!(
            "mov x8, #0",
            ".align 7",
            "nop",
            "2:",
            "strb w8, [{base}, x8]",
            "add x8, x8, #1",
            "cmp x8, {count}",
            "b.ne 2b",

            count = in(reg) count,
            base = in(reg) base_ptr,
            out("x8") _,
            options(nostack)
        );
    });

    println!("\nAligned -16 bytes:");
    test_loop(|count, mut buf| unsafe {
        let base_ptr: *mut u8 = buf.as_mut_ptr();

        asm!(
            "mov x8, #0",
            ".align 7",
            repeat_asm!("nop"; 28),
            "2:",
            "strb w8, [{base}, x8]",
            "add x8, x8, #1",
            "cmp x8, {count}",
            "b.ne 2b",

            count = in(reg) count,
            base = in(reg) base_ptr,
            out("x8") _,
            options(nostack)
        );
    });

    println!("\nAligned -12 bytes:");
    test_loop(|count, mut buf| unsafe {
        let base_ptr: *mut u8 = buf.as_mut_ptr();

        asm!(
            "mov x8, #0",
            ".align 7",
            repeat_asm!("nop"; 29),
            "2:",
            "strb w8, [{base}, x8]",
            "add x8, x8, #1",
            "cmp x8, {count}",
            "b.ne 2b",

            count = in(reg) count,
            base = in(reg) base_ptr,
            out("x8") _,
            options(nostack)
        );
    });

    println!("\nAligned -4 bytes:");
    test_loop(|count, mut buf| unsafe {
        let base_ptr: *mut u8 = buf.as_mut_ptr();

        asm!(
            "mov x8, #0",
            ".align 7",
            repeat_asm!("nop"; 31),
            "2:",
            "strb w8, [{base}, x8]",
            "add x8, x8, #1",
            "cmp x8, {count}",
            "b.ne 2b",

            count = in(reg) count,
            base = in(reg) base_ptr,
            out("x8") _,
            options(nostack)
        );
    });
}

#[test]
fn profile_sched_load_ports() {
    println!("\nRead 8x1:");
    test_loop(|count, mut buf| unsafe {
        let base_ptr: *mut u8 = buf.as_mut_ptr();

        asm!(
            "mov x8, #0",
            ".align 7",
            "2:",
            repeat_asm!("ldr x9, [{base}]"; 1),
            "add x8, x8, #1",
            "cmp x8, {count}",
            "b.le 2b",

            count = in(reg) count,
            base = in(reg) base_ptr,
            out("x8") _,
            out("x9") _,
            options(nostack)
        );
    });

    println!("\nRead 8x2:");
    test_loop(|count, mut buf| unsafe {
        let base_ptr: *mut u8 = buf.as_mut_ptr();

        asm!(
            "mov x8, #0",
            ".align 7",
            "2:",
            repeat_asm!("ldr x9, [{base}]"; 2),
            "add x8, x8, #2",
            "cmp x8, {count}",
            "b.le 2b",

            count = in(reg) count,
            base = in(reg) base_ptr,
            out("x8") _,
            out("x9") _,
            options(nostack)
        );
    });

    println!("\nRead 8x3:");
    test_loop(|count, mut buf| unsafe {
        let base_ptr: *mut u8 = buf.as_mut_ptr();

        asm!(
            "mov x8, #0",
            ".align 7",
            "2:",
            repeat_asm!("ldr x9, [{base}]"; 3),
            "add x8, x8, #3",
            "cmp x8, {count}",
            "b.le 2b",

            count = in(reg) count,
            base = in(reg) base_ptr,
            out("x8") _,
            out("x9") _,
            options(nostack)
        );
    });

    println!("\nRead 8x4:");
    test_loop(|count, mut buf| unsafe {
        let base_ptr: *mut u8 = buf.as_mut_ptr();

        asm!(
            "mov x8, #0",
            ".align 7",
            "2:",
            repeat_asm!("ldr x9, [{base}]"; 4),
            "add x8, x8, #4",
            "cmp x8, {count}",
            "b.le 2b",

            count = in(reg) count,
            base = in(reg) base_ptr,
            out("x8") _,
            out("x9") _,
            options(nostack)
        );
    });
}

#[test]
fn profile_sched_store_ports() {
    println!("\nWrite 8x1:");
    test_loop(|count, mut buf| unsafe {
        let base_ptr: *mut u8 = buf.as_mut_ptr();

        asm!(
            "mov x8, #0",
            ".align 7",
            "2:",
            repeat_asm!("str x9, [{base}]"; 1),
            "add x8, x8, #1",
            "cmp x8, {count}",
            "b.le 2b",

            count = in(reg) count,
            base = in(reg) base_ptr,
            out("x8") _,
            out("x9") _,
            options(nostack)
        );
    });

    println!("\nWrite 8x2:");
    test_loop(|count, mut buf| unsafe {
        let base_ptr: *mut u8 = buf.as_mut_ptr();

        asm!(
            "mov x8, #0",
            ".align 7",
            "2:",
            repeat_asm!("ldr x9, [{base}]"; 2),
            "add x8, x8, #2",
            "cmp x8, {count}",
            "b.le 2b",

            count = in(reg) count,
            base = in(reg) base_ptr,
            out("x8") _,
            out("x9") _,
            options(nostack)
        );
    });

    println!("\nWrite 8x3:");
    test_loop(|count, mut buf| unsafe {
        let base_ptr: *mut u8 = buf.as_mut_ptr();

        asm!(
            "mov x8, #0",
            ".align 7",
            "2:",
            repeat_asm!("ldr x9, [{base}]"; 3),
            "add x8, x8, #3",
            "cmp x8, {count}",
            "b.le 2b",

            count = in(reg) count,
            base = in(reg) base_ptr,
            out("x8") _,
            out("x9") _,
            options(nostack)
        );
    });

    println!("\nWrite 8x4:");
    test_loop(|count, mut buf| unsafe {
        let base_ptr: *mut u8 = buf.as_mut_ptr();

        asm!(
            "mov x8, #0",
            ".align 7",
            "2:",
            repeat_asm!("ldr x9, [{base}]"; 4),
            "add x8, x8, #4",
            "cmp x8, {count}",
            "b.le 2b",

            count = in(reg) count,
            base = in(reg) base_ptr,
            out("x8") _,
            out("x9") _,
            options(nostack)
        );
    });
}
