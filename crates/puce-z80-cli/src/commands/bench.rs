//! `z80 bench [--seconds N]`: raw CPU throughput in T-states per second (no benchmark crate).

use puce_z80_machines::zx::ZxMachine;
use std::time::{Duration, Instant};

/// Target of E13-20: ZEXDOC/ZEXALL (46.7 G T-states) in under one hour.
pub const TARGET_MHZ: f64 = 15.0;

/// Arguments of the `bench` command.
#[derive(Debug)]
pub struct BenchArgs {
    pub seconds: u64,
}

/// Program run by the benchmark: NOP / LD / ADD / JR in an endless loop at 0x8000.
const LOOP: [u8; 9] = [
    0x00, // NOP
    0x3E, 0x05, // LD A,5
    0x80, // ADD A,B
    0x47, // LD B,A
    0x06, 0x01, // LD B,1
    0x18, 0xF7, // JR -9
];

/// Result of a benchmark run.
#[derive(Debug, PartialEq, Eq)]
pub struct BenchResult {
    pub tstates: u64,
    pub instructions: u64,
}

fn machine() -> ZxMachine {
    let mut m = ZxMachine::new_romless();
    m.load_code(0x8000, &LOOP);
    m.cpu.regs.pc = 0x8000;
    m
}

/// Execute exactly `steps` instructions (deterministic).
pub fn run_steps(steps: u64) -> BenchResult {
    let mut m = machine();
    let mut tstates = 0u64;
    for _ in 0..steps {
        tstates += u64::from(m.step());
    }
    BenchResult {
        tstates,
        instructions: steps,
    }
}

/// Execute for about `limit` of wall-clock time; returns the result and the elapsed time.
pub fn run_for(limit: Duration) -> (BenchResult, Duration) {
    let mut m = machine();
    let start = Instant::now();
    let (mut tstates, mut instructions) = (0u64, 0u64);
    loop {
        for _ in 0..100_000 {
            tstates += u64::from(m.step());
        }
        instructions += 100_000;
        let el = start.elapsed();
        if el >= limit {
            return (
                BenchResult {
                    tstates,
                    instructions,
                },
                el,
            );
        }
    }
}

/// Run the benchmark and print the throughput.
pub fn run(args: BenchArgs) -> Result<(), String> {
    let (r, el) = run_for(Duration::from_secs(args.seconds.max(1)));
    let secs = el.as_secs_f64();
    let mhz = r.tstates as f64 / secs / 1e6;
    println!(
        "bench: {} instructions, {} T-states in {:.2} s",
        r.instructions, r.tstates, secs
    );
    println!(
        "bench: {:.1} MIPS, {:.1} MHz equivalent (target {:.0} MHz)",
        r.instructions as f64 / secs / 1e6,
        mhz,
        TARGET_MHZ
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e13_20_bench_runs() {
        let (r, el) = run_for(Duration::from_millis(100));
        assert!(r.tstates > 0 && r.instructions >= 100_000);
        assert!(el >= Duration::from_millis(100));
        assert_eq!(run(BenchArgs { seconds: 1 }), Ok(()));
    }

    #[test]
    fn e13_20_behavior_unchanged() {
        // 6 instructions per loop: NOP 4 + LD A,n 7 + ADD 4 + LD B,A 4 + LD B,n 7 + JR 12 = 38 T / 6 instr.
        let r = run_steps(6_000);
        assert_eq!(r.tstates, 1_000 * 38);
    }
}
