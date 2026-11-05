//! E10-05: direct tests of the undocumented behaviours of spec 23 (one test per point).

use puce_z80::{Bus, Cpu, SimpleRam};

const START: u16 = 0x0100;
const S: u8 = 0x80;
const XY: u8 = 0x28;

/// RAM with a recorded I/O write log and a fixed value returned by port reads.
struct IoBus {
    ram: SimpleRam,
    io_read_value: u8,
    io_writes: Vec<(u16, u8)>,
}

impl IoBus {
    fn new(prog: &[u8]) -> IoBus {
        let mut ram = SimpleRam::new();
        for (i, b) in prog.iter().enumerate() {
            ram.write(START + i as u16, *b);
        }
        IoBus {
            ram,
            io_read_value: 0xFF,
            io_writes: Vec::new(),
        }
    }
}

impl Bus for IoBus {
    fn read(&mut self, addr: u16) -> u8 {
        self.ram.read(addr)
    }

    fn write(&mut self, addr: u16, data: u8) {
        self.ram.write(addr, data);
    }

    fn io_read(&mut self, _port: u16) -> u8 {
        self.io_read_value
    }

    fn io_write(&mut self, port: u16, data: u8) {
        self.io_writes.push((port, data));
    }
}

fn new_cpu() -> Cpu {
    let mut cpu = Cpu::new();
    cpu.regs.pc = START;
    cpu.regs.sp = 0x8000;
    cpu.regs.f = 0;
    cpu
}

/// Run one instruction of `prog` after `setup`; returns the CPU, its bus and the T-states used.
fn exec(prog: &[u8], setup: impl FnOnce(&mut Cpu, &mut IoBus)) -> (Cpu, IoBus, u32) {
    let mut bus = IoBus::new(prog);
    let mut cpu = new_cpu();
    setup(&mut cpu, &mut bus);
    let t = cpu.step(&mut bus);
    (cpu, bus, t)
}

#[test]
fn e10_05_cp_xy_operand() {
    // CP B: A = 0x00, B = 0x28. X/Y come from the operand (0x28), not from the result (0xD8).
    let (cpu, _, _) = exec(&[0xB8], |c, _| {
        c.regs.a = 0x00;
        c.regs.b = 0x28;
    });
    assert_eq!(cpu.regs.f & XY, 0x28);
    assert_eq!(cpu.regs.a, 0x00, "CP does not store the result");
}

#[test]
fn e10_05_ixh_ixl() {
    let setup = |c: &mut Cpu, _: &mut IoBus| {
        c.regs.ix = 0xABCD;
        c.regs.iy = 0x1234;
    };
    assert_eq!(exec(&[0xDD, 0x44], setup).0.regs.b, 0xAB, "LD B,IXH");
    assert_eq!(exec(&[0xDD, 0x45], setup).0.regs.b, 0xCD, "LD B,IXL");
    assert_eq!(exec(&[0xFD, 0x44], setup).0.regs.b, 0x12, "LD B,IYH");
    assert_eq!(exec(&[0xFD, 0x45], setup).0.regs.b, 0x34, "LD B,IYL");
}

#[test]
fn e10_05_sll() {
    let (cpu, _, _) = exec(&[0xCB, 0x30], |c, _| c.regs.b = 0x80);
    assert_eq!(cpu.regs.b, 0x01);
    assert_eq!(cpu.regs.f & 0x01, 1, "carry = old bit 7");
    let (cpu, _, _) = exec(&[0xCB, 0x30], |c, _| c.regs.b = 0x01);
    assert_eq!(cpu.regs.b, 0x03, "bit 0 is set");
}

#[test]
fn e10_05_ddcb_copy() {
    // DD CB 05 00 = RLC (IX+5),B: the result is also copied into B.
    let (cpu, mut bus, _) = exec(&[0xDD, 0xCB, 0x05, 0x00], |c, b| {
        c.regs.ix = 0x2000;
        b.ram.write(0x2005, 0x81);
    });
    assert_eq!(bus.ram.read(0x2005), 0x03);
    assert_eq!(cpu.regs.b, 0x03);
    // BIT ignores the register field: DD CB 05 40 = BIT 0,(IX+5), B unchanged.
    let (cpu, _, _) = exec(&[0xDD, 0xCB, 0x05, 0x40], |c, b| {
        c.regs.ix = 0x2000;
        c.regs.b = 0x77;
        b.ram.write(0x2005, 0x01);
    });
    assert_eq!(cpu.regs.b, 0x77);
}

#[test]
fn e10_05_out_c_zero() {
    let (_, bus, _) = exec(&[0xED, 0x71], |c, _| c.regs.set_bc(0x1234));
    assert_eq!(bus.io_writes, vec![(0x1234, 0x00)], "NMOS writes 0");
}

#[test]
fn e10_05_in_c_flags_only() {
    let (cpu, _, t) = exec(&[0xED, 0x70], |c, b| {
        b.io_read_value = 0x80;
        c.regs.set_bc(0x1234);
        c.regs.a = 0x11;
        c.regs.set_de(0x2222);
        c.regs.set_hl(0x3333);
    });
    assert_eq!(t, 12);
    assert_ne!(cpu.regs.f & S, 0, "flags from the value read");
    assert_eq!(cpu.regs.bc(), 0x1234);
    assert_eq!(cpu.regs.a, 0x11);
    assert_eq!(cpu.regs.de(), 0x2222);
    assert_eq!(cpu.regs.hl(), 0x3333);
}

#[test]
fn e10_05_ed_unknown_nop() {
    let (cpu, _, t) = exec(&[0xED, 0x00], |c, _| {
        c.regs.a = 0x42;
        c.regs.set_bc(0x1234);
    });
    assert_eq!(t, 8);
    assert_eq!(cpu.regs.a, 0x42);
    assert_eq!(cpu.regs.bc(), 0x1234);
    assert_eq!(cpu.regs.pc, START + 2);
}

#[test]
fn e10_05_dd_ignored_4t() {
    assert_eq!(exec(&[0xDD, 0x00], |_, _| {}).2, 8, "DD NOP");
    let (cpu, _, t) = exec(&[0xFD, 0x80], |c, _| {
        c.regs.a = 1;
        c.regs.b = 2;
    });
    assert_eq!(t, 8, "FD ADD A,B: the prefix costs 4 T");
    assert_eq!(cpu.regs.a, 3);
}

#[test]
fn e10_05_prefix_chain() {
    // DD DD DD 21 nn nn: LD IX,nn (14 T) + 4 T for each of the two extra prefixes.
    let (cpu, _, t) = exec(&[0xDD, 0xDD, 0xDD, 0x21, 0x34, 0x12], |_, _| {});
    assert_eq!(t, 22);
    assert_eq!(cpu.regs.ix, 0x1234);
    // DD FD 21 nn nn: only the last prefix counts (IY loaded).
    let (cpu, _, t) = exec(&[0xDD, 0xFD, 0x21, 0x78, 0x56], |c, _| c.regs.ix = 0xAAAA);
    assert_eq!(t, 18);
    assert_eq!(cpu.regs.iy, 0x5678);
    assert_eq!(cpu.regs.ix, 0xAAAA);
}

#[test]
fn e10_05_r_bit7() {
    // The increment keeps bit 7 and wraps the low 7 bits.
    let (cpu, _, _) = exec(&[0x00], |c, _| c.regs.r = 0xFF);
    assert_eq!(cpu.regs.r, 0x80);
    let (cpu, _, _) = exec(&[0x00], |c, _| c.regs.r = 0x7F);
    assert_eq!(cpu.regs.r, 0x00);
    // LD R,A overwrites the whole register (bit 7 included); the next M1 then increments it.
    let mut bus = IoBus::new(&[0xED, 0x4F, 0x00]);
    let mut cpu = new_cpu();
    cpu.regs.a = 0x80;
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.r, 0x80);
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.r, 0x81);
}

#[test]
fn e10_05_reset_state() {
    let mut cpu = Cpu::new();
    cpu.regs.set_af(0x1234);
    cpu.regs.sp = 0x4000;
    cpu.regs.pc = 0x1000;
    cpu.reset();
    assert_eq!(cpu.regs.af(), 0xFFFF);
    assert_eq!(cpu.regs.sp, 0xFFFF);
    assert_eq!(cpu.regs.pc, 0);
    assert!(!cpu.regs.iff1 && !cpu.regs.iff2);
    assert_eq!(cpu.regs.im, 0);
    assert_eq!(cpu.regs.i, 0);
    assert_eq!(cpu.regs.r, 0);
}

#[test]
fn e10_05_bit_hl_wz() {
    // BIT 0,(HL): X/Y come from the high byte of WZ, which BIT leaves alone.
    let (cpu, _, _) = exec(&[0xCB, 0x46], |c, b| {
        c.regs.set_hl(0x4000);
        c.regs.wz = 0x2800;
        b.ram.write(0x4000, 0x01);
    });
    assert_eq!(cpu.regs.f & XY, 0x28);
    assert_eq!(cpu.regs.wz, 0x2800);
    let (cpu, _, _) = exec(&[0xCB, 0x46], |c, b| {
        c.regs.set_hl(0x4000);
        c.regs.wz = 0xD700;
        b.ram.write(0x4000, 0x01);
    });
    assert_eq!(cpu.regs.f & XY, 0x00);
}
