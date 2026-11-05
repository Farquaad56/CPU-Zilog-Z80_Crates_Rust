//! Disassembler (feature `disasm`): mnemonics come from the generated tables, no allocation.
//!
//! E11-01: text buffer, number formatters, and the operand-less, prefix-less case.

use crate::tables::mnem::MNEM_MAIN;
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

/// Disassemble the instruction at `addr`, fetching bytes through `read`.
///
/// E11-01 handles only the prefix-less case by copying the model text; operands are added
/// by E11-02 and the prefixed tables by E11-02..04.
pub fn disassemble(addr: u16, read: &mut dyn FnMut(u16) -> u8) -> Insn {
    let op = read(addr);
    let mut text = DisText::new();
    text.put_str(MNEM_MAIN[op as usize]);
    Insn {
        len: 1,
        text,
        prefix: 0,
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
}
