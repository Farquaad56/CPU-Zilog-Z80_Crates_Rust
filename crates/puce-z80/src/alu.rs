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
        4 => and8(a, b),
        5 => xor8(a, b),
        6 => or8(a, b),
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

fn and8(a: u8, b: u8) -> (u8, u8) {
    let res = a & b;
    let flags = compute_logic_flags(res, flag::H);
    (res, flags)
}

fn xor8(a: u8, b: u8) -> (u8, u8) {
    let res = a ^ b;
    let flags = compute_logic_flags(res, 0);
    (res, flags)
}

fn or8(a: u8, b: u8) -> (u8, u8) {
    let res = a | b;
    let flags = compute_logic_flags(res, 0);
    (res, flags)
}

fn compute_logic_flags(res: u8, h: u8) -> u8 {
    let mut flags = res & 0xA8; // S, Y, X from result
    if res == 0 {
        flags |= flag::Z;
    }
    flags |= h;
    flags |= parity(res);
    flags
}

/// INC 8-bit: increment value, set flags (C preserved).
pub fn inc8(v: u8, f: u8) -> (u8, u8) {
    let res = v.wrapping_add(1);
    let mut flags = (f & flag::C) | (res & 0xA8);
    if res == 0 {
        flags |= flag::Z;
    }
    if (res & 0x0F) == 0 {
        flags |= flag::H;
    }
    if res == 0x80 {
        flags |= flag::PV;
    }
    (res, flags)
}

/// DEC 8-bit: decrement value, set flags (C preserved, N set).
pub fn dec8(v: u8, f: u8) -> (u8, u8) {
    let res = v.wrapping_sub(1);
    let mut flags = (f & flag::C) | (res & 0xA8) | flag::N;
    if res == 0 {
        flags |= flag::Z;
    }
    if (res & 0x0F) == 0x0F {
        flags |= flag::H;
    }
    if res == 0x7F {
        flags |= flag::PV;
    }
    (res, flags)
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

/// Rotation and shift operations (CB prefix).
///
/// y: 0=RLC, 1=RRC, 2=RL, 3=RR, 4=SLA, 5=SRA, 6=SLL, 7=SRL
/// v: value to rotate/shift
/// f: current flags (only bit 0 = carry matters for RL/RR)
/// Returns (result, flags).
pub fn rot(y: u8, v: u8, f: u8) -> (u8, u8) {
    let c = f & 1;
    let (res, out_c) = match y {
        0 => { // RLC: rotate left through carry
            let oc = (v >> 7) & 1;
            let r = ((v << 1) | oc) & 0xFF;
            (r, oc)
        }
        1 => { // RRC: rotate right through carry
            let oc = v & 1;
            let r = (v >> 1) | ((oc << 7) & 0xFF);
            (r, oc)
        }
        2 => { // RL: rotate left through carry (uses existing carry)
            let oc = (v >> 7) & 1;
            let r = ((v << 1) | c) & 0xFF;
            (r, oc)
        }
        3 => { // RR: rotate right through carry (uses existing carry)
            let oc = v & 1;
            let r = (v >> 1) | ((c << 7) & 0xFF);
            (r, oc)
        }
        4 => { // SLA: shift left arithmetic
            let oc = (v >> 7) & 1;
            let r = (v << 1) & 0xFF;
            (r, oc)
        }
        5 => { // SRA: shift right arithmetic
            let oc = v & 1;
            let r = (v >> 1) | (v & 0x80);
            (r, oc)
        }
        6 => { // SLL: shift left logical (sets bit 0)
            let oc = (v >> 7) & 1;
            let r = ((v << 1) | 1) & 0xFF;
            (r, oc)
        }
        7 => { // SRL: shift right logical
            let oc = v & 1;
            let r = v >> 1;
            (r, oc)
        }
        _ => unreachable!("invalid rot op"),
    };

    // Flags: S,Y,X from result; Z if result is 0; H=N=0; P/V=parity of result; C=out_c
    let mut flags = res & 0xA8;
    if res == 0 {
        flags |= flag::Z;
    }
    flags |= parity(res);
    flags |= out_c;
    (res, flags)
}
