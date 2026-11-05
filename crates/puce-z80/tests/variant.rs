//! NMOS / CMOS variants (E17-03).

use puce_z80::{Bus, Cpu, Variant};

/// 64 KB of RAM that records port writes and can hold the INT line.
struct Board {
    ram: Box<[u8; 65536]>,
    out: Vec<(u16, u8)>,
    int_line: bool,
}

impl Board {
    fn with(program: &[u8]) -> Board {
        let mut ram = Box::new([0u8; 65536]);
        ram[..program.len()].copy_from_slice(program);
        Board {
            ram,
            out: Vec::new(),
            int_line: false,
        }
    }
}

impl Bus for Board {
    fn read(&mut self, addr: u16) -> u8 {
        self.ram[usize::from(addr)]
    }
    fn write(&mut self, addr: u16, data: u8) {
        self.ram[usize::from(addr)] = data;
    }
    fn io_write(&mut self, port: u16, data: u8) {
        self.out.push((port, data));
    }
    fn irq(&mut self) -> bool {
        self.int_line
    }
}

#[test]
fn e17_03_default_is_nmos() {
    assert_eq!(Cpu::new().variant(), Variant::Nmos);
    assert_eq!(Variant::default(), Variant::Nmos);
    assert_eq!(Cpu::with_variant(Variant::Cmos).variant(), Variant::Cmos);
}

#[test]
fn e17_03_nmos_out_c_zero() {
    let mut bus = Board::with(&[0xED, 0x71]); // OUT (C),0
    let mut cpu = Cpu::with_variant(Variant::Nmos);
    cpu.regs.b = 0x12;
    cpu.regs.c = 0x34;
    cpu.step(&mut bus);
    assert_eq!(bus.out, vec![(0x1234, 0x00)]);
}

#[test]
fn e17_03_cmos_out_c_ff() {
    let mut bus = Board::with(&[0xED, 0x71]);
    let mut cpu = Cpu::with_variant(Variant::Cmos);
    cpu.regs.b = 0x12;
    cpu.regs.c = 0x34;
    cpu.step(&mut bus);
    assert_eq!(bus.out, vec![(0x1234, 0xFF)]);
}

/// P/V after `LD A,I` (IFF2 = 1) followed by an accepted INT (IM 1).
fn pv_after_int(variant: Variant) -> bool {
    let mut bus = Board::with(&[0xED, 0x57, 0x00]); // LD A,I ; NOP
    let mut cpu = Cpu::with_variant(variant);
    cpu.regs.iff1 = true;
    cpu.regs.iff2 = true;
    cpu.regs.im = 1;
    cpu.step(&mut bus); // LD A,I: P/V = IFF2 = 1
    assert!(cpu.regs.f & 0x04 != 0, "P/V = IFF2 after LD A,I");
    bus.int_line = true;
    cpu.step(&mut bus); // the INT is accepted
    assert_eq!(cpu.regs.pc, 0x0038);
    cpu.regs.f & 0x04 != 0
}

#[test]
fn e17_03_ld_a_i_int_bug_only_on_nmos() {
    assert!(!pv_after_int(Variant::Nmos), "NMOS: P/V reset by the INT");
    assert!(pv_after_int(Variant::Cmos), "CMOS: no bug");
}

/// X / Y of `SCF` with A = 0, F = 0x28 and Q = 0x28 (the previous instruction wrote F).
fn scf_xy(variant: Variant) -> u8 {
    let mut bus = Board::with(&[0x37]); // SCF
    let mut cpu = Cpu::with_variant(variant);
    cpu.regs.a = 0x00;
    cpu.regs.f = 0x28;
    cpu.regs.q = 0x28;
    cpu.step(&mut bus);
    cpu.regs.f & 0x28
}

#[test]
fn e17_03_scf_q_factor_only_on_nmos() {
    assert_eq!(scf_xy(Variant::Nmos), 0x00, "NMOS: A | (F ^ Q)");
    assert_eq!(scf_xy(Variant::Cmos), 0x28, "CMOS: A | F");
}

#[test]
fn e17_03_reset_keeps_variant() {
    let mut cpu = Cpu::with_variant(Variant::Cmos);
    cpu.reset();
    assert_eq!(cpu.variant(), Variant::Cmos);
}
