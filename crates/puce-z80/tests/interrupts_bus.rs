//! E09-08: interrupts through the `Bus` trait and `step()`; RETI visible on the bus.

use puce_z80::{Bus, Cpu, Pins, SimpleRam};

/// Bus with a device that raises INT after `fire_at` T-states and releases it on acknowledge.
struct IrqBus {
    ram: SimpleRam,
    ticks: u32,
    fire_at: u32,
    irq_line: bool,
    /// NMI line level.
    nmi_line: bool,
    /// If true the device never releases INT (level kept after the acknowledge).
    hold: bool,
    acks: u32,
    /// Opcode bytes served by M1 cycles, in order.
    m1_bytes: Vec<u8>,
}

impl IrqBus {
    fn new(fire_at: u32) -> IrqBus {
        IrqBus {
            ram: SimpleRam::new(),
            ticks: 0,
            fire_at,
            irq_line: false,
            nmi_line: false,
            hold: false,
            acks: 0,
            m1_bytes: Vec::new(),
        }
    }

    fn load(&mut self, addr: u16, bytes: &[u8]) {
        for (i, b) in bytes.iter().enumerate() {
            self.ram.write(addr.wrapping_add(i as u16), *b);
        }
    }
}

impl Bus for IrqBus {
    fn read(&mut self, addr: u16) -> u8 {
        self.ram.read(addr)
    }

    fn write(&mut self, addr: u16, data: u8) {
        self.ram.write(addr, data);
    }

    fn irq(&mut self) -> bool {
        self.irq_line
    }

    fn nmi(&mut self) -> bool {
        self.nmi_line
    }

    fn on_tick(&mut self, pins: Pins) {
        self.ticks += 1;
        if self.ticks >= self.fire_at && (self.acks == 0 || self.hold) {
            self.irq_line = true;
        }
        if pins.is_int_ack() {
            self.acks += 1;
            self.irq_line = self.hold;
        }
        // The data bus byte of an opcode fetch, as a daisy-chain peripheral would see it.
        if pins.has(Pins::M1) && pins.has(Pins::MREQ) && pins.has(Pins::RD) {
            let b = self.ram.read(pins.addr());
            self.m1_bytes.push(b);
        }
    }
}

#[test]
fn e09_08_irq_scenario() {
    let mut bus = IrqBus::new(100);
    // 0000: IM 1; EI; HALT        0004: LD (9000h),A; HALT
    bus.load(0x0000, &[0xED, 0x56, 0xFB, 0x76, 0x32, 0x00, 0x90, 0x76]);
    // 0038: INC A; EI; RETI
    bus.load(0x0038, &[0x3C, 0xFB, 0xED, 0x4D]);
    let mut cpu = Cpu::new();
    cpu.regs.a = 0;
    cpu.regs.sp = 0xF000;

    let mut steps = 0;
    while !(bus.ram.read(0x9000) == 1 && cpu.regs.halted && cpu.regs.pc == 0x0007) {
        cpu.step(&mut bus);
        steps += 1;
        assert!(steps < 200, "scenario did not finish");
    }
    assert_eq!(cpu.regs.a, 1, "the handler ran exactly once");
    assert_eq!(bus.acks, 4, "one acknowledge cycle (4 ticks with M1+IORQ)");
    assert_eq!(bus.ram.read(0x9000), 1);
    assert!(cpu.regs.iff1, "EI in the handler re-enabled interrupts");
    assert_eq!(cpu.regs.sp, 0xF000);
}

#[test]
fn e09_08_step_returns_ack_duration() {
    let mut bus = IrqBus::new(0);
    bus.irq_line = true;
    let mut cpu = Cpu::new();
    cpu.regs.sp = 0x8000;
    cpu.regs.iff1 = true;
    cpu.regs.im = 1;
    // The pending interrupt is accepted at once: step() returns the 13 T sequence alone.
    assert_eq!(cpu.step(&mut bus), 13);
    assert_eq!(cpu.regs.pc, 0x0038);
}

#[test]
fn e09_08_reti_visible() {
    let mut bus = IrqBus::new(u32::MAX);
    bus.load(0x0010, &[0xED, 0x4D]);
    let mut cpu = Cpu::new();
    cpu.regs.pc = 0x0010;
    cpu.regs.sp = 0x8000;
    bus.ram.write(0x8000, 0x34);
    bus.ram.write(0x8001, 0x12);
    assert_eq!(cpu.step(&mut bus), 14);
    assert_eq!(bus.m1_bytes, vec![0xED, 0x4D], "the chain sees ED then 4D");
    assert_eq!(cpu.regs.pc, 0x1234);
}

#[test]
fn e09_08_nmi_from_bus() {
    let mut bus = IrqBus::new(u32::MAX);
    bus.nmi_line = true;
    let mut cpu = Cpu::new();
    cpu.regs.pc = 0x0100;
    cpu.regs.sp = 0x8000;
    assert_eq!(cpu.step(&mut bus), 11);
    assert_eq!(cpu.regs.pc, 0x0066);
    assert_eq!(bus.ram.read(0x7FFE), 0x00);
    assert_eq!(bus.ram.read(0x7FFF), 0x01);
    // The line stays high: no new edge, so the next step runs the NOP at 0x0066.
    assert_eq!(cpu.step(&mut bus), 4);
    assert_eq!(cpu.regs.pc, 0x0067);
}

#[test]
fn e09_08_irq_level_retriggers() {
    let mut bus = IrqBus::new(0);
    bus.irq_line = true;
    bus.hold = true;
    // 0038: EI; RETI
    bus.load(0x0038, &[0xFB, 0xED, 0x4D]);
    let mut cpu = Cpu::new();
    cpu.regs.sp = 0x8000;
    cpu.regs.iff1 = true;
    cpu.regs.im = 1;
    // accept (13), EI (4), RETI (14), accept again (13): INT is a level.
    assert_eq!(cpu.step(&mut bus), 13);
    assert_eq!(cpu.step(&mut bus), 4);
    assert_eq!(cpu.step(&mut bus), 14);
    assert_eq!(cpu.step(&mut bus), 13);
    assert_eq!(bus.acks, 8, "two acknowledge cycles of 4 ticks");
}
