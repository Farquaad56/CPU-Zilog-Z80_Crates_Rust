//! Pure 8-bit ALU operations with Z80 flag computation.
//!
//! Each function takes the operation, operands, and current flags,
//! and returns the new accumulator value and new flags.

use crate::regs::flag;

/// Compute 8-bit ALU operation and return (new A, new F).
///
/// `op` values: 0=ADD, 1=ADC, 2=SUB, 3=SBC, 4=AND, 5=XOR, 6=OR, 7=CP.
/// `f` is the current flags register (only bit 0 matters for ADC/SBC).
/// For CP, A is unchanged but flags are set as if A-b was computed.
pub fn alu8(op: u8, a: u8, b: u8, f: u8) -> (u8, u8) {
    match op {
        0 => add8(a, b),
        1 => adc8(a, b, f),
        2 => sub8(a, b),
        3 => sbc8(a, b, f),
        4 => unreachable!("AND not implemented in E04-02"),
        5 => unreachable!("XOR not implemented in E04-02"),
        6 => unreachable!("OR not implemented in E04-02"),
        7 => cp8(a, b),
        _ => unreachable!("invalid 8-bit ALU op"),
    }
}

fn add8(a: u8, b: u8) -> (u8, u8) {
    let t = (a as u16) + (b as u16);
    let res = (t & 0xFF) as u8;
    let flags = compute_add_flags(a, b, res, t);
    (res, flags)
}

fn adc8(a: u8, b: u8, f: u8) -> (u8, u8) {
    let c = (f & 1) as u16;
    let t = (a as u16) + (b as u16) + c;
    let res = (t & 0xFF) as u8;
    let flags = compute_add_flags(a, b, res, t);
    (res, flags)
}

fn compute_add_flags(a: u8, b: u8, res: u8, t: u16) -> u8 {
    let mut flags = res & 0xA8; // S, Y, X from result
    if res == 0 {
        flags |= flag::Z;
    }
    flags |= (a ^ b ^ res) & flag::H;
    if ((a ^ res) & (b ^ res) & 0x80) != 0 {
        flags |= flag::PV;
    }
    if (t >> 8) != 0 {
        flags |= flag::C;
    }
    flags
}

fn sub8(a: u8, b: u8) -> (u8, u8) {
    let t = (a as u16).wrapping_sub(b as u16);
    let res = (t & 0xFF) as u8;
    let flags = compute_sub_flags(a, b, res, t);
    (res, flags)
}

fn sbc8(a: u8, b: u8, f: u8) -> (u8, u8) {
    let c = (f & 1) as u16;
    let t = (a as u16).wrapping_sub((b as u16) + c);
    let res = (t & 0xFF) as u8;
    let flags = compute_sub_flags(a, b, res, t);
    (res, flags)
}

fn cp8(a: u8, b: u8) -> (u8, u8) {
    let t = (a as u16).wrapping_sub(b as u16);
    let res = (t & 0xFF) as u8;
    // For CP, Y and X come from the operand b, not the result
    let flags = compute_sub_flags_with_xy(a, b, res, t, b);
    (a, flags)
}

fn compute_sub_flags(a: u8, b: u8, res: u8, t: u16) -> u8 {
    compute_sub_flags_with_xy(a, b, res, t, res)
}

fn compute_sub_flags_with_xy(a: u8, b: u8, res: u8, t: u16, xy_src: u8) -> u8 {
    let mut flags = (xy_src & 0x28) | (res & 0x80); // Y, X from operand; S from result
    if res == 0 {
        flags |= flag::Z;
    }
    flags |= (a ^ b ^ res) & flag::H;
    if ((a ^ b) & (a ^ res) & 0x80) != 0 {
        flags |= flag::PV;
    }
    flags |= flag::N;
    if (t >> 8) != 0 {
        flags |= flag::C;
    }
    flags
}

/// Compute even parity bit. Returns flag::PV if bit count is even, 0 otherwise.
pub fn parity(v: u8) -> u8 {
    let mut bits = v;
    let mut count = 0;
    while bits != 0 {
        if bits & 1 != 0 {
            count += 1;
        }
        bits >>= 1;
    }
    if count % 2 == 0 {
        flag::PV
    } else {
        0
    }
}
