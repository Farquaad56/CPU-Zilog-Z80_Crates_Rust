//! E10-04: NMOS bug, an INT accepted right after LD A,I / LD A,R resets P/V.

use puce_z80::{Bus, Cpu, SimpleRam};

const PV: u8 = 0x04;

/// RAM with controllable INT and NMI lines.
struct LineBus {
    ram: SimpleRam,
    irq: bool,
    nmi: bool,
}

impl Bus for LineBus {
    fn read(&mut self, addr: u16) -> u8 {
        self.ram.read(addr)
    }

    fn write(&mut self, addr: u16, data: u8) {
        self.ram.write(addr, data);
    }

    fn irq(&mut self) -> bool {
        self.irq
    }

    fn nmi(&mut self) -> bool {
        self.nmi
    }
}

/// CPU at 0x0100 in IM 1 with IFF1 = IFF2 = 1, about to run `prog`.
fn setup(prog: &[u8]) -> (Cpu, LineBus) {
    let mut bus = LineBus {
        ram: SimpleRam::new(),
        irq: false,
        nmi: false,
    };
    for (i, b) in prog.iter().enumerate() {
        bus.ram.write(0x0100 + i as u16, *b);
    }
    let mut cpu = Cpu::new();
    cpu.regs.pc = 0x0100;
    cpu.regs.sp = 0x8000;
    cpu.regs.iff1 = true;
    cpu.regs.iff2 = true;
    cpu.regs.im = 1;
    cpu.regs.i = 0x12;
    cpu.regs.f = 0;
    (cpu, bus)
}

#[test]
fn e10_04_int_after_ld_a_i() {
    let (mut cpu, mut bus) = setup(&[0xED, 0x57]);
    cpu.step(&mut bus);
    assert_ne!(cpu.regs.f & PV, 0, "LD A,I: P/V = IFF2");
    bus.irq = true;
    assert_eq!(cpu.step(&mut bus), 13);
    assert_eq!(cpu.regs.pc, 0x0038);
    assert_eq!(cpu.regs.f & PV, 0, "P/V reset by the accepted INT");
}

#[test]
fn e10_04_int_after_ld_a_r() {
    let (mut cpu, mut bus) = setup(&[0xED, 0x5F]);
    cpu.step(&mut bus);
    assert_ne!(cpu.regs.f & PV, 0);
    bus.irq = true;
    assert_eq!(cpu.step(&mut bus), 13);
    assert_eq!(cpu.regs.f & PV, 0);
}

#[test]
fn e10_04_int_after_nop_keeps() {
    let (mut cpu, mut bus) = setup(&[0x00]);
    cpu.regs.f = PV;
    cpu.step(&mut bus);
    bus.irq = true;
    assert_eq!(cpu.step(&mut bus), 13);
    assert_eq!(cpu.regs.f & PV, PV, "no change after a NOP");
}

#[test]
fn e10_04_nmi_not_affected() {
    let (mut cpu, mut bus) = setup(&[0xED, 0x57]);
    cpu.step(&mut bus);
    assert_ne!(cpu.regs.f & PV, 0);
    bus.nmi = true;
    assert_eq!(cpu.step(&mut bus), 11);
    assert_eq!(cpu.regs.pc, 0x0066);
    assert_ne!(cpu.regs.f & PV, 0, "NMI leaves P/V alone");
}
