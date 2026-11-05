//! Register file and flag constants.
//!
//! All 8-bit registers are stored separately; register pairs (AF, BC, DE, HL)
//! are provided as accessor methods that combine/split the bytes.

/// Z80 register file state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Regs {
    /// Accumulator
    pub a: u8,
    /// Flags
    pub f: u8,
    /// B register
    pub b: u8,
    /// C register
    pub c: u8,
    /// D register
    pub d: u8,
    /// E register
    pub e: u8,
    /// H register
    pub h: u8,
    /// L register
    pub l: u8,
    /// Alternate accumulator (AF')
    pub a2: u8,
    /// Alternate flags (AF')
    pub f2: u8,
    /// Alternate B (BC')
    pub b2: u8,
    /// Alternate C (BC')
    pub c2: u8,
    /// Alternate D (DE')
    pub d2: u8,
    /// Alternate E (DE')
    pub e2: u8,
    /// Alternate H (HL')
    pub h2: u8,
    /// Alternate L (HL')
    pub l2: u8,
    /// Index register X
    pub ix: u16,
    /// Index register Y
    pub iy: u16,
    /// Stack pointer
    pub sp: u16,
    /// Program counter
    pub pc: u16,
    /// Interrupt vector base
    pub i: u8,
    /// Refresh register
    pub r: u8,
    /// WZ temporary register (internal)
    pub wz: u16,
    /// Interrupt flip-flop 1
    pub iff1: bool,
    /// Interrupt flip-flop 2
    pub iff2: bool,
    /// Interrupt mode (0, 1, or 2)
    pub im: u8,
    /// Q flag (internal, undocumented)
    pub q: u8,
    /// HALT state
    pub halted: bool,
}

/// Flag bit constants for the F register.
pub mod flag {
    /// Sign flag (bit 7)
    pub const S: u8 = 0x80;
    /// Zero flag (bit 6)
    pub const Z: u8 = 0x40;
    /// Y flag (bit 5, undocumented)
    pub const Y: u8 = 0x20;
    /// Half-carry flag (bit 4)
    pub const H: u8 = 0x10;
    /// X flag (bit 3, undocumented)
    pub const X: u8 = 0x08;
    /// Parity/overflow flag (bit 2)
    pub const PV: u8 = 0x04;
    /// Subtract flag (bit 1)
    pub const N: u8 = 0x02;
    /// Carry flag (bit 0)
    pub const C: u8 = 0x01;
}

impl Regs {
    /// Read AF pair as a 16-bit value.
    #[inline]
    pub fn af(&self) -> u16 {
        ((self.a as u16) << 8) | (self.f as u16)
    }

    /// Read BC pair as a 16-bit value.
    #[inline]
    pub fn bc(&self) -> u16 {
        ((self.b as u16) << 8) | (self.c as u16)
    }

    /// Read DE pair as a 16-bit value.
    #[inline]
    pub fn de(&self) -> u16 {
        ((self.d as u16) << 8) | (self.e as u16)
    }

    /// Read HL pair as a 16-bit value.
    #[inline]
    pub fn hl(&self) -> u16 {
        ((self.h as u16) << 8) | (self.l as u16)
    }

    /// Write AF pair from a 16-bit value.
    #[inline]
    pub fn set_af(&mut self, v: u16) {
        self.a = (v >> 8) as u8;
        self.f = v as u8;
    }

    /// Write BC pair from a 16-bit value.
    #[inline]
    pub fn set_bc(&mut self, v: u16) {
        self.b = (v >> 8) as u8;
        self.c = v as u8;
    }

    /// Write DE pair from a 16-bit value.
    #[inline]
    pub fn set_de(&mut self, v: u16) {
        self.d = (v >> 8) as u8;
        self.e = v as u8;
    }

    /// Write HL pair from a 16-bit value.
    #[inline]
    pub fn set_hl(&mut self, v: u16) {
        self.h = (v >> 8) as u8;
        self.l = v as u8;
    }

    /// Read alternate AF' pair as a 16-bit value.
    #[inline]
    pub fn af2(&self) -> u16 {
        ((self.a2 as u16) << 8) | (self.f2 as u16)
    }

    /// Read alternate BC' pair as a 16-bit value.
    #[inline]
    pub fn bc2(&self) -> u16 {
        ((self.b2 as u16) << 8) | (self.c2 as u16)
    }

    /// Read alternate DE' pair as a 16-bit value.
    #[inline]
    pub fn de2(&self) -> u16 {
        ((self.d2 as u16) << 8) | (self.e2 as u16)
    }

    /// Read alternate HL' pair as a 16-bit value.
    #[inline]
    pub fn hl2(&self) -> u16 {
        ((self.h2 as u16) << 8) | (self.l2 as u16)
    }

    /// Write alternate AF' pair from a 16-bit value.
    #[inline]
    pub fn set_af2(&mut self, v: u16) {
        self.a2 = (v >> 8) as u8;
        self.f2 = v as u8;
    }

    /// Write alternate BC' pair from a 16-bit value.
    #[inline]
    pub fn set_bc2(&mut self, v: u16) {
        self.b2 = (v >> 8) as u8;
        self.c2 = v as u8;
    }

    /// Write alternate DE' pair from a 16-bit value.
    #[inline]
    pub fn set_de2(&mut self, v: u16) {
        self.d2 = (v >> 8) as u8;
        self.e2 = v as u8;
    }

    /// Write alternate HL' pair from a 16-bit value.
    #[inline]
    pub fn set_hl2(&mut self, v: u16) {
        self.h2 = (v >> 8) as u8;
        self.l2 = v as u8;
    }

    /// Read high byte of IX.
    #[inline]
    pub fn ixh(&self) -> u8 {
        (self.ix >> 8) as u8
    }

    /// Read low byte of IX.
    #[inline]
    pub fn ixl(&self) -> u8 {
        self.ix as u8
    }

    /// Read high byte of IY.
    #[inline]
    pub fn iyh(&self) -> u8 {
        (self.iy >> 8) as u8
    }

    /// Read low byte of IY.
    #[inline]
    pub fn iyl(&self) -> u8 {
        self.iy as u8
    }

    /// Write high byte of IX.
    #[inline]
    pub fn set_ixh(&mut self, v: u8) {
        self.ix = ((v as u16) << 8) | (self.ix & 0xFF);
    }

    /// Write low byte of IX.
    #[inline]
    pub fn set_ixl(&mut self, v: u8) {
        self.ix = (self.ix & 0xFF00) | (v as u16);
    }

    /// Write high byte of IY.
    #[inline]
    pub fn set_iyh(&mut self, v: u8) {
        self.iy = ((v as u16) << 8) | (self.iy & 0xFF);
    }

    /// Write low byte of IY.
    #[inline]
    pub fn set_iyl(&mut self, v: u8) {
        self.iy = (self.iy & 0xFF00) | (v as u16);
    }

    /// Exchange BC DE HL with BC' DE' HL'.
    #[inline]
    pub fn exx(&mut self) {
        let tmp_b = self.b;
        let tmp_c = self.c;
        let tmp_d = self.d;
        let tmp_e = self.e;
        let tmp_h = self.h;
        let tmp_l = self.l;

        self.b = self.b2;
        self.c = self.c2;
        self.d = self.d2;
        self.e = self.e2;
        self.h = self.h2;
        self.l = self.l2;

        self.b2 = tmp_b;
        self.c2 = tmp_c;
        self.d2 = tmp_d;
        self.e2 = tmp_e;
        self.h2 = tmp_h;
        self.l2 = tmp_l;
    }

    /// Exchange AF with AF'.
    #[inline]
    pub fn ex_af(&mut self) {
        let tmp_a = self.a;
        let tmp_f = self.f;

        self.a = self.a2;
        self.f = self.f2;

        self.a2 = tmp_a;
        self.f2 = tmp_f;
    }

    /// Exchange DE with HL.
    #[inline]
    pub fn ex_de_hl(&mut self) {
        let tmp_d = self.d;
        let tmp_e = self.e;

        self.d = self.h;
        self.e = self.l;

        self.h = tmp_d;
        self.l = tmp_e;
    }
}
