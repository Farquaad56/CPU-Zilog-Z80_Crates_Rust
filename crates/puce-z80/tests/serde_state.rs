//! Save states with serde (E17-04).
#![cfg(feature = "serde")]

use puce_z80::{Bus, Cpu, CpuState, Variant};

struct Ram(Box<[u8; 65536]>);

impl Bus for Ram {
    fn read(&mut self, a: u16) -> u8 {
        self.0[usize::from(a)]
    }
    fn write(&mut self, a: u16, d: u8) {
        self.0[usize::from(a)] = d;
    }
}

/// Loop touching registers, shadow registers, R and memory.
fn ram() -> Ram {
    let prog = [
        0x21, 0x00, 0x40, // LD HL,4000
        0x06, 0x00, // LD B,0
        0x34, // INC (HL)
        0x86, // ADD A,(HL)
        0x08, // EX AF,AF'
        0xED, 0x5F, // LD A,R
        0x10, 0xF8, // DJNZ -> INC (HL)
        0xFB, // EI
        0x18, 0xEB, // JR -> INC (HL)
    ];
    let mut m = Box::new([0u8; 65536]);
    m[..prog.len()].copy_from_slice(&prog);
    Ram(m)
}

#[test]
fn e17_04_roundtrip() {
    let mut bus = ram();
    let mut cpu = Cpu::with_variant(Variant::Cmos);
    for _ in 0..137 {
        cpu.step(&mut bus);
    }
    let state = cpu.save_state().expect("boundary");
    let json = serde_json::to_string(&state).unwrap();
    let back: CpuState = serde_json::from_str(&json).unwrap();
    assert_eq!(state, back);
    let restored = Cpu::from_state(&back);
    assert_eq!(restored.save_state(), Some(state));
    assert_eq!(restored.regs, cpu.regs);
    assert_eq!(restored.variant(), Variant::Cmos);
}

#[test]
fn e17_04_resume_identical() {
    let mut bus_a = ram();
    let mut cpu_a = Cpu::new();
    for _ in 0..91 {
        cpu_a.step(&mut bus_a);
    }
    let json = serde_json::to_string(&cpu_a.save_state().unwrap()).unwrap();
    let mut bus_b = Ram(bus_a.0.clone());
    let mut cpu_b = Cpu::from_state(&serde_json::from_str(&json).unwrap());
    for _ in 0..500 {
        assert_eq!(cpu_a.step(&mut bus_a), cpu_b.step(&mut bus_b));
        assert_eq!(cpu_a.regs, cpu_b.regs);
    }
    assert_eq!(cpu_a.save_state(), cpu_b.save_state());
}

#[test]
fn e17_04_mid_instruction_refused() {
    use puce_z80::Pins;
    let mut cpu = Cpu::new();
    let _ = cpu.tick(Pins(0));
    assert!(!cpu.at_instruction_boundary());
    assert_eq!(cpu.save_state(), None);
}

#[test]
fn e17_04_json_fields() {
    let v = serde_json::to_value(Cpu::new().save_state().unwrap()).unwrap();
    assert_eq!(v["regs"]["sp"], 0xFFFF);
    assert_eq!(v["variant"], "Nmos");
}
