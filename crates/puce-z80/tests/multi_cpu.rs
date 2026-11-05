//! Several CPUs in one program (arcade boards), and determinism.
//!
//! A `Cpu` owns all of its state: no global, no `static mut`, no hidden clock.

use puce_z80::{Cpu, Pins};
use std::path::Path;

/// A small program that exercises registers, ALU, flags and memory writes:
///
/// ```text
///   LD HL,9000h ; LD B,1 ; LD C,7
/// loop: LD A,(HL) ; ADD A,B ; RLCA ; XOR C ; LD (HL),A ; INC HL ; INC B ; LD C,A ; JR loop
/// ```
const PROGRAM: [u8; 18] = [
    0x21, 0x00, 0x90, 0x06, 0x01, 0x0E, 0x07, 0x7E, 0x80, 0x07, 0xA9, 0x77, 0x23, 0x04, 0x4F, 0x18,
    0xF6, 0x00,
];

/// One Z80 with its own RAM, driven pin by pin.
struct Machine {
    cpu: Cpu,
    ram: Box<[u8; 65536]>,
    out: Pins,
}

impl Machine {
    fn new(seed: u8) -> Machine {
        let mut ram = Box::new([0u8; 65536]);
        ram[..PROGRAM.len()].copy_from_slice(&PROGRAM);
        ram[0x9000] = seed;
        Machine {
            cpu: Cpu::new(),
            ram,
            out: Pins(0),
        }
    }

    fn tick(&mut self) {
        let mut inp = Pins(0);
        if self.out.has(Pins::MREQ) && self.out.has(Pins::RD) {
            inp = inp.set_data(self.ram[usize::from(self.out.addr())]);
        }
        if self.out.has(Pins::MREQ) && self.out.has(Pins::WR) {
            self.ram[usize::from(self.out.addr())] = self.out.data();
        }
        self.out = self.cpu.tick(inp);
    }

    /// FNV-1a over the registers and the whole RAM.
    fn hash(&self) -> u64 {
        let r = &self.cpu.regs;
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut put = |b: u8| {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        };
        for v in [r.af(), r.bc(), r.de(), r.hl(), r.ix, r.iy, r.sp, r.pc, r.wz] {
            put(v as u8);
            put((v >> 8) as u8);
        }
        put(r.i);
        put(r.r);
        for &b in self.ram.iter() {
            put(b);
        }
        h
    }
}

fn run_alone(seed: u8, ticks: u32) -> Machine {
    let mut m = Machine::new(seed);
    for _ in 0..ticks {
        m.tick();
    }
    m
}

#[test]
fn e16_05_two_cpus_interleaved() {
    let ticks = 200_000;
    let mut a = Machine::new(0x11);
    let mut b = Machine::new(0x55);
    for _ in 0..ticks {
        a.tick();
        b.tick();
    }
    let (solo_a, solo_b) = (run_alone(0x11, ticks), run_alone(0x55, ticks));
    assert_eq!(a.hash(), solo_a.hash());
    assert_eq!(b.hash(), solo_b.hash());
    assert_ne!(
        a.hash(),
        b.hash(),
        "different data must give different states"
    );
    assert_eq!(a.cpu.regs.pc, solo_a.cpu.regs.pc);
    assert_eq!(b.cpu.regs.af(), solo_b.cpu.regs.af());
}

#[test]
fn e16_05_deterministic_hash() {
    let m = run_alone(0x11, 1_000_000);
    // The program really ran: the loop is alive and HL walked through RAM (about 19 000 turns).
    assert!(
        (0x0007..=0x0011).contains(&m.cpu.regs.pc),
        "pc {:04X}",
        m.cpu.regs.pc
    );
    assert!(m.cpu.regs.hl() > 0xD000, "hl {:04X}", m.cpu.regs.hl());
    let first = m.hash();
    let second = run_alone(0x11, 1_000_000).hash();
    assert_eq!(first, second);
    // Pinned value: any change of timing or semantics on this program shows up here.
    assert_eq!(first, EXPECTED_HASH);
}

const EXPECTED_HASH: u64 = 11373182561020625163;

fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let p = entry.path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

#[test]
fn e16_05_no_global_state() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    assert!(!files.is_empty());
    for f in files {
        let text = std::fs::read_to_string(&f).unwrap();
        for (n, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            for banned in [
                "static mut",
                "thread_local!",
                "lazy_static",
                "OnceLock",
                "OnceCell",
            ] {
                assert!(
                    !code.contains(banned),
                    "{}:{}: global state ({})",
                    f.display(),
                    n + 1,
                    banned
                );
            }
        }
    }
}
