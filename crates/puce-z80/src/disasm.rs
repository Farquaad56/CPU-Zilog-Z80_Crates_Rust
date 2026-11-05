//! Disassembler (feature `disasm`): mnemonics come from the generated tables, no allocation.
//!
//! E11-01: text buffer, number formatters, and the operand-less, prefix-less case.

use crate::tables::mnem::{MNEM_CB, MNEM_MAIN};
use core::fmt;

/// Capacity of a disassembled text, in bytes.
const TEXT_CAP: usize = 40;

/// Fixed-capacity ASCII text of a disassembled instruction.
#[derive(Clone, Copy)]
pub struct DisText {
    buf: [u8; TEXT_CAP],
    len: u8,
}

impl DisText {
    /// Empty text.
    pub const fn new() -> DisText {
        DisText {
            buf: [0; TEXT_CAP],
            len: 0,
        }
    }

    /// Append one byte; bytes beyond the capacity are dropped (no panic).
    pub fn put_byte(&mut self, b: u8) {
        if (self.len as usize) < TEXT_CAP {
            self.buf[self.len as usize] = b;
            self.len += 1;
        }
    }

    /// Append a string (ASCII); truncated at the capacity.
    pub fn put_str(&mut self, s: &str) {
        for b in s.bytes() {
            self.put_byte(b);
        }
    }

    /// Remove the last byte, if any.
    fn pop(&mut self) {
        if self.len > 0 {
            self.len -= 1;
        }
    }

    /// The text as a string slice.
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.len as usize]).unwrap_or("")
    }

    /// Number of bytes in the text.
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// True if the text is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Default for DisText {
    fn default() -> Self {
        DisText::new()
    }
}

impl fmt::Display for DisText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Debug for DisText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DisText({:?})", self.as_str())
    }
}

/// One upper-case hexadecimal digit.
fn hex_digit(v: u8) -> u8 {
    let v = v & 0x0F;
    if v < 10 {
        b'0' + v
    } else {
        b'A' + (v - 10)
    }
}

/// Append an 8-bit number as `$1F`.
pub fn put_hex8(t: &mut DisText, v: u8) {
    t.put_byte(b'$');
    t.put_byte(hex_digit(v >> 4));
    t.put_byte(hex_digit(v));
}

/// Append a 16-bit number as `$1F2E`.
pub fn put_hex16(t: &mut DisText, v: u16) {
    t.put_byte(b'$');
    t.put_byte(hex_digit((v >> 12) as u8));
    t.put_byte(hex_digit((v >> 8) as u8));
    t.put_byte(hex_digit((v >> 4) as u8));
    t.put_byte(hex_digit(v as u8));
}

/// Append a signed displacement as `+$05` or `-$02`.
pub fn put_disp(t: &mut DisText, d: u8) {
    let d = d as i8;
    if d < 0 {
        t.put_byte(b'-');
        put_hex8(t, (d as i16).unsigned_abs() as u8);
    } else {
        t.put_byte(b'+');
        put_hex8(t, d as u8);
    }
}

/// One disassembled instruction.
#[derive(Clone, Copy, Debug)]
pub struct Insn {
    /// Length in bytes.
    pub len: u8,
    /// Text of the instruction.
    pub text: DisText,
    /// Prefix byte: 0, 0xCB, 0xDD, 0xED or 0xFD (DDCB / FDCB report 0xDD / 0xFD).
    pub prefix: u8,
    /// Opcode byte (the last opcode byte of the instruction).
    pub op: u8,
}

/// Operand values read after the opcode.
#[derive(Clone, Copy, Default)]
struct Operands {
    nn: u16,
    n: u8,
    d: u8,
    target: u16,
}

/// A piece of a mnemonic template: a literal byte or an isolated operand token.
enum Piece<'a> {
    Lit(u8),
    Tok(&'a str),
}

/// True for the bytes that delimit an operand token (`,` `(` `)` space `+`).
fn is_delim(b: u8) -> bool {
    matches!(b, b',' | b'(' | b')' | b' ' | b'+')
}

/// Split a template into literals and isolated tokens `nn`, `n`, `e`, `d`.
/// Other words (INC, NZ, ...) are copied byte by byte.
fn walk<'a>(tmpl: &'a str, mut f: impl FnMut(Piece<'a>)) {
    let bytes = tmpl.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if is_delim(bytes[i]) {
            f(Piece::Lit(bytes[i]));
            i += 1;
            continue;
        }
        let mut j = i;
        while j < bytes.len() && !is_delim(bytes[j]) {
            j += 1;
        }
        let word = &tmpl[i..j];
        if matches!(word, "nn" | "n" | "e" | "d") {
            f(Piece::Tok(word));
        } else {
            for &b in &bytes[i..j] {
                f(Piece::Lit(b));
            }
        }
        i = j;
    }
}

/// Read the operand bytes of `tmpl` starting at `pos` (in order of appearance).
/// Returns the values and the number of operand bytes.
fn fetch_operands(tmpl: &str, mut pos: u16, read: &mut dyn FnMut(u16) -> u8) -> (Operands, u8) {
    let mut ops = Operands::default();
    let mut count = 0u8;
    walk(tmpl, |piece| {
        if let Piece::Tok(tok) = piece {
            match tok {
                "nn" => {
                    let lo = read(pos) as u16;
                    let hi = read(pos.wrapping_add(1)) as u16;
                    ops.nn = (hi << 8) | lo;
                    pos = pos.wrapping_add(2);
                    count += 2;
                }
                "n" => {
                    ops.n = read(pos);
                    pos = pos.wrapping_add(1);
                    count += 1;
                }
                "d" => {
                    ops.d = read(pos);
                    pos = pos.wrapping_add(1);
                    count += 1;
                }
                _ => {
                    // e: relative jump, the target is relative to the end of the instruction.
                    let e = read(pos) as i8;
                    pos = pos.wrapping_add(1);
                    ops.target = pos.wrapping_add(e as i16 as u16);
                    count += 1;
                }
            }
        }
    });
    (ops, count)
}

/// Write `tmpl` with its operand tokens replaced by the values in `ops`.
fn expand(t: &mut DisText, tmpl: &str, ops: &Operands) {
    walk(tmpl, |piece| match piece {
        Piece::Lit(b) => t.put_byte(b),
        Piece::Tok("nn") => put_hex16(t, ops.nn),
        Piece::Tok("n") => put_hex8(t, ops.n),
        Piece::Tok("d") => {
            // The template has "IX+d": drop the '+' and let put_disp write the sign.
            if t.buf[..t.len as usize].last() == Some(&b'+') {
                t.pop();
            }
            put_disp(t, ops.d);
        }
        Piece::Tok(_) => put_hex16(t, ops.target),
    });
}

/// Disassemble the instruction at `addr`, fetching bytes through `read`.
///
/// E11-02 handles the main table and the CB table; ED, DD and FD arrive in E11-03 / E11-04
/// (until then they report the model text of the main table and a length of 1).
pub fn disassemble(addr: u16, read: &mut dyn FnMut(u16) -> u8) -> Insn {
    let first_byte = read(addr);
    let (tmpl, prefix, op, first) = if first_byte == 0xCB {
        let op = read(addr.wrapping_add(1));
        (MNEM_CB[op as usize], 0xCB, op, addr.wrapping_add(2))
    } else {
        (
            MNEM_MAIN[first_byte as usize],
            0,
            first_byte,
            addr.wrapping_add(1),
        )
    };
    let (ops, count) = fetch_operands(tmpl, first, read);
    let mut text = DisText::new();
    expand(&mut text, tmpl, &ops);
    let head: u8 = if prefix == 0 { 1 } else { 2 };
    Insn {
        len: head + count,
        text,
        prefix,
        op,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::format;

    #[test]
    fn e11_01_distext_display() {
        let mut t = DisText::new();
        t.put_str("LD A,B");
        assert_eq!(format!("{}", t), "LD A,B");
        assert_eq!(t.as_str(), "LD A,B");
        assert_eq!(t.len(), 6);
    }

    #[test]
    fn e11_01_hex8() {
        let mut t = DisText::new();
        put_hex8(&mut t, 0x0F);
        assert_eq!(t.as_str(), "$0F");
        let mut t = DisText::new();
        put_hex8(&mut t, 0xAB);
        assert_eq!(t.as_str(), "$AB");
    }

    #[test]
    fn e11_01_hex16() {
        let mut t = DisText::new();
        put_hex16(&mut t, 0xBEEF);
        assert_eq!(t.as_str(), "$BEEF");
        let mut t = DisText::new();
        put_hex16(&mut t, 0x0001);
        assert_eq!(t.as_str(), "$0001");
    }

    #[test]
    fn e11_01_disp_pos_neg() {
        let mut t = DisText::new();
        t.put_str("(IX");
        put_disp(&mut t, 0x05);
        t.put_byte(b')');
        assert_eq!(t.as_str(), "(IX+$05)");
        let mut t = DisText::new();
        t.put_str("(IX");
        put_disp(&mut t, 0xFE);
        t.put_byte(b')');
        assert_eq!(t.as_str(), "(IX-$02)");
        let mut t = DisText::new();
        put_disp(&mut t, 0x80);
        assert_eq!(t.as_str(), "-$80");
        let mut t = DisText::new();
        put_disp(&mut t, 0x7F);
        assert_eq!(t.as_str(), "+$7F");
    }

    #[test]
    fn e11_01_truncation() {
        let mut t = DisText::new();
        for _ in 0..100 {
            t.put_byte(b'X');
        }
        assert_eq!(t.len(), TEXT_CAP);
        let mut t = DisText::new();
        t.put_str(&"ABCDEFGHIJ".repeat(10));
        assert_eq!(t.len(), TEXT_CAP);
        put_hex16(&mut t, 0x1234);
        assert_eq!(t.len(), TEXT_CAP);
    }

    #[test]
    fn e11_01_nop() {
        let mut read = |_a: u16| 0x00u8;
        let insn = disassemble(0, &mut read);
        assert_eq!(insn.text.as_str(), "NOP");
        assert_eq!(insn.len, 1);
        assert_eq!(insn.prefix, 0);
        assert_eq!(insn.op, 0x00);
    }

    /// Disassemble `bytes` placed at `addr`.
    fn dis(addr: u16, bytes: &[u8]) -> Insn {
        let mut read = |a: u16| {
            bytes
                .get(a.wrapping_sub(addr) as usize)
                .copied()
                .unwrap_or(0)
        };
        disassemble(addr, &mut read)
    }

    #[test]
    fn e11_02_ld_bc_nn() {
        let i = dis(0, &[0x01, 0x34, 0x12]);
        assert_eq!(i.text.as_str(), "LD BC,$1234");
        assert_eq!(i.len, 3);
    }

    #[test]
    fn e11_02_jr_target() {
        let i = dis(0x8000, &[0x18, 0xFE]);
        assert_eq!(i.text.as_str(), "JR $8000");
        assert_eq!(i.len, 2);
    }

    #[test]
    fn e11_02_jr_forward() {
        let i = dis(0x8000, &[0x20, 0x05]);
        assert_eq!(i.text.as_str(), "JR NZ,$8007");
    }

    #[test]
    fn e11_02_ld_a_n() {
        let i = dis(0, &[0x3E, 0x7F]);
        assert_eq!(i.text.as_str(), "LD A,$7F");
        assert_eq!(i.len, 2);
    }

    #[test]
    fn e11_02_cb_rlc() {
        let i = dis(0, &[0xCB, 0x00]);
        assert_eq!(i.text.as_str(), "RLC B");
        assert_eq!(i.len, 2);
        assert_eq!(i.prefix, 0xCB);
        assert_eq!(i.op, 0x00);
    }

    #[test]
    fn e11_02_cb_bit_hl() {
        let i = dis(0, &[0xCB, 0x46]);
        assert_eq!(i.text.as_str(), "BIT 0,(HL)");
        assert_eq!(i.len, 2);
    }

    #[test]
    fn e11_02_djnz() {
        let i = dis(0x1000, &[0x10, 0xFC]);
        assert_eq!(i.text.as_str(), "DJNZ $0FFE");
        assert_eq!(i.len, 2);
    }

    #[test]
    fn e11_02_jp_nn() {
        let i = dis(0, &[0xC3, 0x00, 0x80]);
        assert_eq!(i.text.as_str(), "JP $8000");
        assert_eq!(i.len, 3);
    }

    #[test]
    fn e11_02_token_boundaries() {
        // "INC" and "NZ" must not be touched; "(nn)" and "(n)" are replaced.
        assert_eq!(dis(0, &[0x3C]).text.as_str(), "INC A");
        assert_eq!(dis(0, &[0x32, 0x00, 0x90]).text.as_str(), "LD ($9000),A");
        assert_eq!(dis(0, &[0xD3, 0x7E]).text.as_str(), "OUT ($7E),A");
        assert_eq!(dis(0, &[0x36, 0x01]).text.as_str(), "LD (HL),$01");
    }
}
