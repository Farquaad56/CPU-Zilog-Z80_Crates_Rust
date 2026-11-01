// Helpers are consumed by the instruction handlers added from E05-03 on.
#![allow(dead_code)]

//! Register access helpers for instruction semantics.
//!
//! Provides uniform access to 8-bit registers, register pairs, and conditions
//! with proper handling of index prefixes (IX/IY).

use crate::alu;
use crate::cpu::Cpu;
use crate::engine::decode::Prefix;

/// Read an 8-bit register by index.
/// i: 0=B 1=C 2=D 3=E 4=H 5=L 6=(HL) 7=A
/// (HL) reads from eng.tmp_l.
pub(crate) fn r8(cpu: &Cpu, i: u8) -> u8 {
    match i {
        0 => cpu.regs.b,
        1 => cpu.regs.c,
        2 => cpu.regs.d,
        3 => cpu.regs.e,
        4 => cpu.regs.h,
        5 => cpu.regs.l,
        6 => cpu.eng.tmp_l,
        7 => cpu.regs.a,
        _ => unreachable!("r8 index out of range"),
    }
}

/// Write an 8-bit register by index.
/// i: 0=B 1=C 2=D 3=E 4=H 5=L 6=(HL) 7=A
/// (HL) writes to eng.tmp_l.
pub(crate) fn set_r8(cpu: &mut Cpu, i: u8, v: u8) {
    match i {
        0 => cpu.regs.b = v,
        1 => cpu.regs.c = v,
        2 => cpu.regs.d = v,
        3 => cpu.regs.e = v,
        4 => cpu.regs.h = v,
        5 => cpu.regs.l = v,
        6 => cpu.eng.tmp_l = v,
        7 => cpu.regs.a = v,
        _ => unreachable!("set_r8 index out of range"),
    }
}

/// Read an 8-bit register by index with prefix handling.
/// If eng.prefix is set (IX/IY), H/L map to IXH/IXL or IYH/IYL.
pub(crate) fn r8x(cpu: &Cpu, i: u8) -> u8 {
    if cpu.eng.prefix == Prefix::None {
        return r8(cpu, i);
    }
    match i {
        4 => match cpu.eng.prefix {
            Prefix::Dd => cpu.regs.ixh(),
            Prefix::Fd => cpu.regs.iyh(),
            _ => unreachable!(),
        },
        5 => match cpu.eng.prefix {
            Prefix::Dd => cpu.regs.ixl(),
            Prefix::Fd => cpu.regs.iyl(),
            _ => unreachable!(),
        },
        _ => r8(cpu, i),
    }
}

/// Write an 8-bit register by index with prefix handling.
pub(crate) fn set_r8x(cpu: &mut Cpu, i: u8, v: u8) {
    if cpu.eng.prefix == Prefix::None {
        return set_r8(cpu, i, v);
    }
    match i {
        4 => match cpu.eng.prefix {
            Prefix::Dd => cpu.regs.set_ixh(v),
            Prefix::Fd => cpu.regs.set_iyh(v),
            _ => unreachable!(),
        },
        5 => match cpu.eng.prefix {
            Prefix::Dd => cpu.regs.set_ixl(v),
            Prefix::Fd => cpu.regs.set_iyl(v),
            _ => unreachable!(),
        },
        _ => set_r8(cpu, i, v),
    }
}

/// Read HL pair (or IX/IY if prefix is set).
pub(crate) fn hl(cpu: &Cpu) -> u16 {
    match cpu.eng.prefix {
        Prefix::None => cpu.regs.hl(),
        Prefix::Dd => cpu.regs.ix,
        Prefix::Fd => cpu.regs.iy,
    }
}

/// Write HL pair (or IX/IY if prefix is set).
pub(crate) fn set_hl(cpu: &mut Cpu, v: u16) {
    match cpu.eng.prefix {
        Prefix::None => cpu.regs.set_hl(v),
        Prefix::Dd => cpu.regs.ix = v,
        Prefix::Fd => cpu.regs.iy = v,
    }
}

/// Read a register pair by index.
/// p: 0=BC 1=DE 2=HL(indexed) 3=SP
pub(crate) fn rp(cpu: &Cpu, p: u8) -> u16 {
    match p {
        0 => cpu.regs.bc(),
        1 => cpu.regs.de(),
        2 => hl(cpu),
        3 => cpu.regs.sp,
        _ => unreachable!("rp index out of range"),
    }
}

/// Write a register pair by index.
pub(crate) fn set_rp(cpu: &mut Cpu, p: u8, v: u16) {
    match p {
        0 => cpu.regs.set_bc(v),
        1 => cpu.regs.set_de(v),
        2 => set_hl(cpu, v),
        3 => cpu.regs.sp = v,
        _ => unreachable!("set_rp index out of range"),
    }
}

/// Read a register pair by index (AF variant).
/// p: 0=BC 1=DE 2=HL(indexed) 3=AF
pub(crate) fn rp2(cpu: &Cpu, p: u8) -> u16 {
    match p {
        0 => cpu.regs.bc(),
        1 => cpu.regs.de(),
        2 => hl(cpu),
        3 => cpu.regs.af(),
        _ => unreachable!("rp2 index out of range"),
    }
}

/// Write a register pair by index (AF variant).
pub(crate) fn set_rp2(cpu: &mut Cpu, p: u8, v: u16) {
    match p {
        0 => cpu.regs.set_bc(v),
        1 => cpu.regs.set_de(v),
        2 => set_hl(cpu, v),
        3 => cpu.regs.set_af(v),
        _ => unreachable!("set_rp2 index out of range"),
    }
}

/// Evaluate a condition code.
/// cc: 0=NZ 1=Z 2=NC 3=C 4=PO 5=PE 6=P 7=M
pub(crate) fn cc(cpu: &Cpu, i: u8) -> bool {
    alu::cond(cpu.regs.f, i)
}

/// Write the flags register and mark that F was written.
pub(crate) fn set_f(cpu: &mut Cpu, f: u8) {
    cpu.regs.f = f;
    cpu.eng.f_written = true;
}
