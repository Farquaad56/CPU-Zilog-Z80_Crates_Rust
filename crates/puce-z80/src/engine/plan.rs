//! Plan token types for the Z80 execution engine.
//!
//! Plans are micro-programs that describe how each instruction executes in terms of
//! bus cycles, internal operations, and hooks. The generated opcode tables use these
//! types to encode the execution semantics of every Z80 instruction.

/// Token kind: what type of operation this token represents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// M1 cycle (opcode or prefix fetch + refresh). len = 4/5/6 T-states.
    F,
    /// Memory read from address to destination register. len = 3/4/5 T-states.
    R,
    /// Memory write from source register to address. len = 3/5 T-states.
    W,
    /// Port read (IN instruction). len = 4 T-states.
    N,
    /// Port write (OUT instruction). len = 4 T-states.
    O,
    /// Internal cycle with no bus activity. len = number of T-states.
    I,
    /// Hook: calls sem(op_table, opcode, hook_no). len stores hook number (0 or 1), takes 0 T-states.
    X,
}

/// Address source for the address bus during a cycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Addr {
    /// No address on the bus (internal cycles, hooks).
    None,
    /// PC then increment (opcode fetch).
    PcInc,
    /// Stack pointer value.
    Sp,
    /// SP then increment (push low byte).
    SpInc,
    /// SP + 1 (push high byte).
    SpPlus1,
    /// Decrement SP then use new value (pop).
    DecSp,
    /// HL register pair as address.
    Hl,
    /// BC register pair as address.
    Bc,
    /// DE register pair as address.
    De,
    /// 16-bit immediate (WZ register).
    Nn,
    /// WZ + 1 (second byte of 16-bit write).
    NnPlus1,
    /// IX/IY + displacement (computed into WZ).
    Xy,
    /// Port address: A on high byte, TMP.L on low byte.
    An,
}

/// Temporary register used as source or destination within a cycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reg {
    /// No register involved.
    None,
    /// TMP.L (low byte of data read/written).
    L,
    /// TMP.H (high byte of data read/written).
    H,
    /// WZ.Z (low byte of address pointer).
    Z,
    /// WZ.W (high byte of address pointer).
    W,
    /// Signed displacement byte for IX/IY indexed addressing.
    D,
    /// Opcode byte read as data (for DD CB d op sequences).
    Op,
}

/// A single plan token describing one operation in an instruction's execution sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tok {
    /// The kind of operation.
    pub kind: Kind,
    /// For F/R/W/N/O/I: T-state count. For X (hook): hook number (0 or 1).
    pub len: u8,
    /// Address source for the address bus during this cycle.
    pub addr: Addr,
    /// Source/destination temporary register.
    pub reg: Reg,
    /// If true, this token only executes when the internal cond flag is set (by a preceding hook).
    pub cond: bool,
}

impl Tok {
    /// Construct a new plan token.
    pub const fn new(kind: Kind, len: u8, addr: Addr, reg: Reg, cond: bool) -> Tok {
        Tok {
            kind,
            len,
            addr,
            reg,
            cond,
        }
    }

    /// Returns true if this is a hook token (Kind::X).
    pub const fn is_hook(&self) -> bool {
        matches!(self.kind, Kind::X)
    }

    /// Returns the hook number for hook tokens. For non-hook tokens, returns 0.
    /// Hook numbers are 0 or 1, corresponding to X0 and X1 in the plan grammar.
    pub const fn hook_no(&self) -> u8 {
        match self.kind {
            Kind::X => self.len,
            _ => 0,
        }
    }

    /// Returns true if this token involves bus activity (F, R, W, N, O).
    pub const fn is_bus(&self) -> bool {
        matches!(self.kind, Kind::F | Kind::R | Kind::W | Kind::N | Kind::O)
    }
}

/// A plan is a static slice of tokens describing an instruction's execution.
pub type Plan = &'static [Tok];

/// Compute the minimum T-state count for a plan (conditional tokens skipped).
/// Hook tokens (Kind::X) contribute 0 T-states regardless of their len field.
pub fn plan_t_min(p: Plan) -> u32 {
    let mut total = 0u32;
    for tok in p.iter() {
        if tok.is_hook() {
            continue; // hooks take 0 T-states
        }
        if !tok.cond {
            total += u32::from(tok.len);
        }
    }
    total
}

/// Compute the maximum T-state count for a plan (all conditional tokens included).
/// Hook tokens (Kind::X) contribute 0 T-states regardless of their len field.
pub fn plan_t_max(p: Plan) -> u32 {
    let mut total = 0u32;
    for tok in p.iter() {
        if tok.is_hook() {
            continue; // hooks take 0 T-states
        }
        total += u32::from(tok.len);
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::boxed::Box;

    #[test]
    fn e02_01_tok_new_const() {
        // Verify Tok::new works in const context
        const T: Tok = Tok::new(Kind::F, 4, Addr::PcInc, Reg::None, false);
        assert_eq!(T.kind, Kind::F);
        assert_eq!(T.len, 4);
        assert_eq!(T.addr, Addr::PcInc);
        assert_eq!(T.reg, Reg::None);
        let cond = T.cond;
        assert!(!cond);
    }

    #[test]
    fn e02_01_t_min_max() {
        // Plan: F4, R3, ?I5 (conditional), X0 (hook)
        // t_min = 4 + 3 = 7 (skip conditional I5 and hook contributes 0)
        // t_max = 4 + 3 + 5 = 12 (include conditional I5, hook still 0)
        let tokens: Box<[Tok]> = Box::new([
            Tok::new(Kind::F, 4, Addr::PcInc, Reg::None, false),
            Tok::new(Kind::R, 3, Addr::Hl, Reg::L, false),
            Tok::new(Kind::I, 5, Addr::None, Reg::None, true), // conditional
            Tok::new(Kind::X, 0, Addr::None, Reg::None, false), // hook X0
        ]);
        let plan: Plan = Box::leak(tokens);

        assert_eq!(plan_t_min(plan), 7);
        assert_eq!(plan_t_max(plan), 12);
    }

    #[test]
    fn e02_01_hook_no() {
        let x0 = Tok::new(Kind::X, 0, Addr::None, Reg::None, false);
        let x1 = Tok::new(Kind::X, 1, Addr::None, Reg::None, false);

        assert_eq!(x0.hook_no(), 0);
        assert_eq!(x1.hook_no(), 1);
    }

    #[test]
    fn e02_01_is_bus() {
        let f = Tok::new(Kind::F, 4, Addr::PcInc, Reg::None, false);
        let r = Tok::new(Kind::R, 3, Addr::Hl, Reg::L, false);
        let w = Tok::new(Kind::W, 3, Addr::Hl, Reg::L, false);
        let n = Tok::new(Kind::N, 4, Addr::An, Reg::L, false);
        let o = Tok::new(Kind::O, 4, Addr::An, Reg::L, false);
        let i = Tok::new(Kind::I, 2, Addr::None, Reg::None, false);
        let x = Tok::new(Kind::X, 0, Addr::None, Reg::None, false);

        assert!(f.is_bus());
        assert!(r.is_bus());
        assert!(w.is_bus());
        assert!(n.is_bus());
        assert!(o.is_bus());
        assert!(!i.is_bus());
        assert!(!x.is_bus());
    }
}
