//! The host of `examples/shared_bus.rs` (copied), checked.

use puce_z80::{Cpu, Pins};

struct Host {
    cpu: Cpu,
    ram: Box<[u8; 65536]>,
    out: Pins,
}

impl Host {
    fn new() -> Host {
        let mut ram = Box::new([0x3Cu8; 65536]); // INC A everywhere ...
        ram[0] = 0x3E; // ... after LD A,0
        ram[1] = 0x00;
        Host {
            cpu: Cpu::new(),
            ram,
            out: Pins(0),
        }
    }

    fn tick(&mut self, extra: u64) -> Pins {
        let mut inp = Pins(extra);
        if self.out.has(Pins::MREQ) && self.out.has(Pins::RD) {
            inp = inp.set_data(self.ram[usize::from(self.out.addr())]);
        }
        if self.out.has(Pins::MREQ) && self.out.has(Pins::WR) {
            self.ram[usize::from(self.out.addr())] = self.out.data();
        }
        self.out = self.cpu.tick(inp);
        self.out
    }

    /// Grant the bus: returns once BUSAK is active.
    fn take_bus(&mut self) {
        let mut guard = 0;
        while !self.tick(Pins::BUSRQ).has(Pins::BUSAK) {
            guard += 1;
            assert!(guard < 50, "BUSAK never came");
        }
    }
}

#[test]
fn e16_03_frozen_during_busak() {
    let mut z = Host::new();
    for _ in 0..30 {
        z.tick(0);
    }
    z.take_bus();
    let (pc, af, r) = (z.cpu.regs.pc, z.cpu.regs.af(), z.cpu.regs.r);
    for _ in 0..40 {
        let out = z.tick(Pins::BUSRQ);
        assert!(out.has(Pins::BUSAK));
        assert!(!out.has(Pins::MREQ) && !out.has(Pins::RD) && !out.has(Pins::WR));
    }
    assert_eq!(z.cpu.regs.pc, pc);
    assert_eq!(z.cpu.regs.af(), af);
    assert_eq!(z.cpu.regs.r, r);
}

#[test]
fn e16_03_resumes() {
    // Z80 A: runs freely for `free` T-states.
    // Z80 B: same program, but the bus is taken for 25 T-states in the middle.
    let mut b = Host::new();
    let mut free = 0u32;
    for _ in 0..30 {
        b.tick(0);
        free += 1;
    }
    // BUSRQ rises: the current cycle completes (free ticks), then the CPU is frozen.
    loop {
        if b.tick(Pins::BUSRQ).has(Pins::BUSAK) {
            break;
        }
        free += 1;
    }
    for _ in 0..25 {
        assert!(b.tick(Pins::BUSRQ).has(Pins::BUSAK));
    }
    for _ in 0..40 {
        b.tick(0);
        free += 1;
    }

    let mut a = Host::new();
    for _ in 0..free {
        a.tick(0);
    }
    assert_eq!(b.cpu.regs.pc, a.cpu.regs.pc);
    assert_eq!(b.cpu.regs.af(), a.cpu.regs.af());
    assert_eq!(b.cpu.regs.r, a.cpu.regs.r);
    assert!(b.cpu.regs.pc > 10, "the program did not move on");
}

#[test]
fn e16_03_reset_pc_zero() {
    let mut z = Host::new();
    for _ in 0..50 {
        z.tick(0);
    }
    assert_ne!(z.cpu.regs.pc, 0);
    z.tick(Pins::RESET);
    assert_eq!(z.cpu.regs.pc, 0);
    assert_eq!(z.cpu.regs.i, 0);
}
