//! FUSE `z80/tests` reader and runner (format: docs/specs/34_FUSE_FORMAT.md).
//!
//! `tests.in` gives an initial state and a T-state budget; `tests.expected` the bus events, the
//! final state and the modified memory. [`run_fuse`] executes whole instructions until the
//! budget is reached and compares everything.
//!
//! Comparison rule: the memory / port events MR MW PR PW are compared exactly (order, time,
//! address, data) together with the registers, the final time and the memory changes. The
//! contention markers MC / PC are produced by [`run_fuse`] for the memory and port cycles of the
//! CPU, but internal-cycle contentions (repeated MC without bus access) have no pin, so MC / PC
//! are not part of the pass / fail criterion (decision recorded in 34_FUSE_FORMAT.md).

use puce_z80::{Bus, Cpu, Pins};
use std::collections::{BTreeMap, BTreeSet};

/// Kind of a bus event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Memory read.
    Mr,
    /// Memory write.
    Mw,
    /// Memory contend (address presented).
    Mc,
    /// Port read.
    Pr,
    /// Port write.
    Pw,
    /// Port contend.
    Pc,
}

impl Kind {
    /// Parse the two-letter FUSE code.
    pub fn parse(s: &str) -> Option<Kind> {
        Some(match s {
            "MR" => Kind::Mr,
            "MW" => Kind::Mw,
            "MC" => Kind::Mc,
            "PR" => Kind::Pr,
            "PW" => Kind::Pw,
            "PC" => Kind::Pc,
            _ => return None,
        })
    }

    /// The two-letter FUSE code.
    pub fn code(self) -> &'static str {
        match self {
            Kind::Mr => "MR",
            Kind::Mw => "MW",
            Kind::Mc => "MC",
            Kind::Pr => "PR",
            Kind::Pw => "PW",
            Kind::Pc => "PC",
        }
    }

    /// True for the events that carry data and take part in the comparison.
    pub fn is_data(self) -> bool {
        matches!(self, Kind::Mr | Kind::Mw | Kind::Pr | Kind::Pw)
    }
}

/// One timestamped bus event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    /// T-state at which the event occurs.
    pub time: u32,
    /// Event type.
    pub kind: Kind,
    /// Memory address or port.
    pub addr: u16,
    /// Data (absent for contentions).
    pub data: Option<u8>,
}

impl Event {
    /// Text form used in messages: `  4 MR 0000 00`.
    pub fn text(&self) -> String {
        match self.data {
            Some(d) => format!(
                "{} {} {:04x} {:02x}",
                self.time,
                self.kind.code(),
                self.addr,
                d
            ),
            None => format!("{} {} {:04x}", self.time, self.kind.code(), self.addr),
        }
    }
}

/// Registers and flags (the two state lines of a record).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FuseRegs {
    /// AF, BC, DE, HL.
    pub main: [u16; 4],
    /// AF', BC', DE', HL'.
    pub alt: [u16; 4],
    /// IX.
    pub ix: u16,
    /// IY.
    pub iy: u16,
    /// SP.
    pub sp: u16,
    /// PC.
    pub pc: u16,
    /// MEMPTR (WZ).
    pub memptr: u16,
    /// I.
    pub i: u8,
    /// R.
    pub r: u8,
    /// IFF1.
    pub iff1: bool,
    /// IFF2.
    pub iff2: bool,
    /// Interrupt mode.
    pub im: u8,
    /// Halted.
    pub halted: bool,
    /// T-states: budget in `tests.in`, final time in `tests.expected`.
    pub tstates: u32,
}

/// Memory blocks: start address and bytes.
pub type MemBlocks = Vec<(u16, Vec<u8>)>;

/// A record of `tests.in`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FuseTest {
    /// Description (the test name, e.g. `edb0_1`).
    pub name: String,
    /// Initial state.
    pub regs: FuseRegs,
    /// Initial memory.
    pub mem_blocks: MemBlocks,
    /// T-state budget.
    pub tstates: u32,
}

/// A record of `tests.expected`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FuseExpected {
    /// Description.
    pub name: String,
    /// Bus events.
    pub events: Vec<Event>,
    /// Final state (`tstates` = final time).
    pub regs: FuseRegs,
    /// Memory changed since the initial setup.
    pub mem_changes: MemBlocks,
}

fn hex16(s: &str, what: &str) -> Result<u16, String> {
    u16::from_str_radix(s, 16).map_err(|_| format!("bad {} '{}'", what, s))
}

fn hex8(s: &str, what: &str) -> Result<u8, String> {
    u8::from_str_radix(s, 16).map_err(|_| format!("bad {} '{}'", what, s))
}

fn parse_regs(l1: &str, l2: &str) -> Result<FuseRegs, String> {
    let a: Vec<&str> = l1.split_whitespace().collect();
    let b: Vec<&str> = l2.split_whitespace().collect();
    if a.len() != 13 {
        return Err(format!(
            "register line needs 13 words, got {}: '{}'",
            a.len(),
            l1
        ));
    }
    if b.len() != 7 {
        return Err(format!(
            "state line needs 7 words, got {}: '{}'",
            b.len(),
            l2
        ));
    }
    let w = |i: usize| hex16(a[i], "register");
    Ok(FuseRegs {
        main: [w(0)?, w(1)?, w(2)?, w(3)?],
        alt: [w(4)?, w(5)?, w(6)?, w(7)?],
        ix: w(8)?,
        iy: w(9)?,
        sp: w(10)?,
        pc: w(11)?,
        memptr: w(12)?,
        i: hex8(b[0], "I")?,
        r: hex8(b[1], "R")?,
        iff1: b[2] != "0",
        iff2: b[3] != "0",
        im: b[4].parse().map_err(|_| format!("bad IM '{}'", b[4]))?,
        halted: b[5] != "0",
        tstates: b[6]
            .parse()
            .map_err(|_| format!("bad tstates '{}'", b[6]))?,
    })
}

/// Parse `<addr> <b1> ... -1`; the first word `-1` alone means "end of list" (None).
fn parse_mem_line(line: &str) -> Result<Option<(u16, Vec<u8>)>, String> {
    let w: Vec<&str> = line.split_whitespace().collect();
    if w.is_empty() || w[0] == "-1" {
        return Ok(None);
    }
    let addr = hex16(w[0], "address")?;
    let mut bytes = Vec::new();
    for t in &w[1..] {
        if *t == "-1" {
            break;
        }
        bytes.push(hex8(t, "byte")?);
    }
    Ok(Some((addr, bytes)))
}

/// Parse the whole `tests.in`.
pub fn parse_in(text: &str) -> Result<Vec<FuseTest>, String> {
    let mut out = Vec::new();
    let mut lines = text.lines().peekable();
    while let Some(name) = lines.find(|l| !l.trim().is_empty()) {
        let l1 = lines
            .next()
            .ok_or_else(|| format!("{}: missing register line", name))?;
        let l2 = lines
            .next()
            .ok_or_else(|| format!("{}: missing state line", name))?;
        let regs = parse_regs(l1, l2).map_err(|e| format!("{}: {}", name, e))?;
        let mut mem_blocks = Vec::new();
        loop {
            let l = lines
                .next()
                .ok_or_else(|| format!("{}: missing end -1", name))?;
            match parse_mem_line(l).map_err(|e| format!("{}: {}", name, e))? {
                Some(b) => mem_blocks.push(b),
                None => break,
            }
        }
        out.push(FuseTest {
            name: name.trim().to_string(),
            tstates: regs.tstates,
            regs,
            mem_blocks,
        });
    }
    Ok(out)
}

fn parse_event(line: &str) -> Option<Event> {
    let w: Vec<&str> = line.split_whitespace().collect();
    if w.len() < 3 {
        return None;
    }
    let kind = Kind::parse(w[1])?;
    Some(Event {
        time: w[0].parse().ok()?,
        kind,
        addr: u16::from_str_radix(w[2], 16).ok()?,
        data: w.get(3).and_then(|d| u8::from_str_radix(d, 16).ok()),
    })
}

/// Parse the whole `tests.expected`.
pub fn parse_expected(text: &str) -> Result<Vec<FuseExpected>, String> {
    let mut out = Vec::new();
    let mut lines = text.lines().peekable();
    while let Some(name) = lines.find(|l| !l.trim().is_empty()) {
        let mut events = Vec::new();
        while let Some(l) = lines.peek() {
            match parse_event(l) {
                Some(e) => {
                    events.push(e);
                    lines.next();
                }
                None => break,
            }
        }
        let l1 = lines
            .next()
            .ok_or_else(|| format!("{}: missing register line", name))?;
        let l2 = lines
            .next()
            .ok_or_else(|| format!("{}: missing state line", name))?;
        let regs = parse_regs(l1, l2).map_err(|e| format!("{}: {}", name, e))?;
        let mut mem_changes = Vec::new();
        while let Some(l) = lines.peek() {
            if l.trim().is_empty() {
                break;
            }
            if let Some(b) = parse_mem_line(l).map_err(|e| format!("{}: {}", name, e))? {
                mem_changes.push(b);
            }
            lines.next();
        }
        out.push(FuseExpected {
            name: name.trim().to_string(),
            events,
            regs,
            mem_changes,
        });
    }
    Ok(out)
}

/// 64 KiB of RAM, zero-filled; port reads return the high byte of the port (FUSE convention).
struct FuseBus {
    ram: Box<[u8; 65536]>,
}

impl Bus for FuseBus {
    fn read(&mut self, addr: u16) -> u8 {
        self.ram[addr as usize]
    }
    fn write(&mut self, addr: u16, data: u8) {
        self.ram[addr as usize] = data;
    }
    fn io_read(&mut self, port: u16) -> u8 {
        (port >> 8) as u8
    }
    fn io_write(&mut self, _port: u16, _data: u8) {}
}

/// Result of running one test.
#[derive(Clone, Debug)]
pub struct FuseResult {
    /// Events produced by the CPU (MC, MR, MW, PC, PR, PW).
    pub events: Vec<Event>,
    /// Final state, `tstates` = final time.
    pub regs: FuseRegs,
    /// Memory changed since the initial setup, as contiguous blocks.
    pub mem_changes: MemBlocks,
}

/// Longest instruction (tick safety bound for the loop of one instruction).
const MAX_INSTR_TICKS: u32 = 64;

/// Offsets from the tick of the RD / WR pin to the FUSE event time (measured on the pins:
/// opcode fetch MR at start+4, other memory cycles at start+3, the pin sits at start+1).
const M1_END: u32 = 3;
const MEM_END: u32 = 2;

fn regs_of(cpu: &Cpu, tstates: u32) -> FuseRegs {
    let r = &cpu.regs;
    FuseRegs {
        main: [r.af(), r.bc(), r.de(), r.hl()],
        alt: [r.af2(), r.bc2(), r.de2(), r.hl2()],
        ix: r.ix,
        iy: r.iy,
        sp: r.sp,
        pc: r.pc,
        memptr: r.wz,
        i: r.i,
        r: r.r,
        iff1: r.iff1,
        iff2: r.iff2,
        im: r.im,
        halted: r.halted,
        tstates,
    }
}

fn apply_regs(cpu: &mut Cpu, g: &FuseRegs) {
    let r = &mut cpu.regs;
    r.set_af(g.main[0]);
    r.set_bc(g.main[1]);
    r.set_de(g.main[2]);
    r.set_hl(g.main[3]);
    r.set_af2(g.alt[0]);
    r.set_bc2(g.alt[1]);
    r.set_de2(g.alt[2]);
    r.set_hl2(g.alt[3]);
    r.ix = g.ix;
    r.iy = g.iy;
    r.sp = g.sp;
    r.pc = g.pc;
    r.wz = g.memptr;
    r.i = g.i;
    r.r = g.r;
    r.iff1 = g.iff1;
    r.iff2 = g.iff2;
    r.im = g.im;
    r.halted = g.halted;
    r.q = 0;
}

/// Group the changed addresses into contiguous blocks.
fn blocks_of(changes: &BTreeMap<u16, u8>) -> MemBlocks {
    let mut out: MemBlocks = Vec::new();
    for (&a, &v) in changes {
        match out.last_mut() {
            Some((start, bytes)) if (*start as usize + bytes.len()) == a as usize => bytes.push(v),
            _ => out.push((a, vec![v])),
        }
    }
    out
}

/// Execute `test` and return the events, the final state and the memory changes.
pub fn run_fuse(test: &FuseTest) -> FuseResult {
    let mut cpu = Cpu::new();
    apply_regs(&mut cpu, &test.regs);
    let mut bus = FuseBus {
        ram: Box::new([0u8; 65536]),
    };
    let mut listed: BTreeSet<u16> = BTreeSet::new();
    for (addr, bytes) in &test.mem_blocks {
        for (i, b) in bytes.iter().enumerate() {
            bus.ram[addr.wrapping_add(i as u16) as usize] = *b;
            listed.insert(addr.wrapping_add(i as u16));
        }
    }
    let initial = bus.ram.clone();
    let mut written: BTreeSet<u16> = BTreeSet::new();

    let mut events: Vec<Event> = Vec::new();
    let mut time: u32 = 0;
    let mut last = Pins(0);
    loop {
        let start = time;
        for _ in 0..MAX_INSTR_TICKS {
            let mut pins_in = Pins(0);
            let mut fed = None;
            if last.has(Pins::MREQ) && last.has(Pins::RD) {
                fed = Some(bus.read(last.addr()));
            } else if last.has(Pins::IORQ) && last.has(Pins::RD) && !last.has(Pins::M1) {
                fed = Some(bus.io_read(last.addr()));
            }
            if let Some(v) = fed {
                pins_in = pins_in.set_data(v);
            }
            let out = cpu.tick(pins_in);
            let t = time;
            time += 1;
            // `t` is the tick of the pin assertion (one tick after the address appears, T2 of
            // the cycle). FUSE stamps the cycle start (MC) and its end (MR / MW).
            if out.has(Pins::MREQ) && out.has(Pins::RD) {
                let a = out.addr();
                let end = if out.has(Pins::M1) {
                    t + M1_END
                } else {
                    t + MEM_END
                };
                events.push(Event {
                    time: t - 1,
                    kind: Kind::Mc,
                    addr: a,
                    data: None,
                });
                events.push(Event {
                    time: end,
                    kind: Kind::Mr,
                    addr: a,
                    data: Some(bus.ram[a as usize]),
                });
            } else if out.has(Pins::MREQ) && out.has(Pins::WR) {
                let a = out.addr();
                bus.write(a, out.data());
                written.insert(a);
                events.push(Event {
                    time: t - 1,
                    kind: Kind::Mc,
                    addr: a,
                    data: None,
                });
                events.push(Event {
                    time: t + MEM_END,
                    kind: Kind::Mw,
                    addr: a,
                    data: Some(out.data()),
                });
            } else if out.has(Pins::IORQ) && out.has(Pins::WR) {
                bus.io_write(out.addr(), out.data());
                events.push(Event {
                    time: t - 1,
                    kind: Kind::Pw,
                    addr: out.addr(),
                    data: Some(out.data()),
                });
            } else if out.has(Pins::IORQ) && out.has(Pins::RD) && !out.has(Pins::M1) {
                let a = out.addr();
                events.push(Event {
                    time: t - 1,
                    kind: Kind::Pr,
                    addr: a,
                    data: Some((a >> 8) as u8),
                });
            }
            last = out;
            if cpu.at_instruction_boundary() {
                break;
            }
        }
        if time >= test.tstates || time == start {
            break;
        }
    }

    // An address counts as modified when its value differs from the initial setup; an address
    // the test did not initialise counts as modified as soon as it is written (even with 0).
    let changes: BTreeMap<u16, u8> = written
        .iter()
        .filter(|&&a| !listed.contains(&a) || bus.ram[a as usize] != initial[a as usize])
        .map(|&a| (a, bus.ram[a as usize]))
        .collect();
    FuseResult {
        events,
        regs: regs_of(&cpu, time),
        mem_changes: blocks_of(&changes),
    }
}

/// Compare a result with the expected record; one readable line per difference.
pub fn compare(result: &FuseResult, expected: &FuseExpected) -> Vec<String> {
    let mut out = Vec::new();
    let got: Vec<&Event> = result.events.iter().filter(|e| e.kind.is_data()).collect();
    let exp: Vec<&Event> = expected
        .events
        .iter()
        .filter(|e| e.kind.is_data())
        .collect();
    // FUSE quirk: the operand read of a not-taken JR cc / DJNZ is stamped MC but never MR.
    // Drop our MR when the reference has the MC at the cycle start and no data event at all.
    let got: Vec<&Event> = got
        .into_iter()
        .filter(|g| {
            let quirk = g.kind == Kind::Mr
                && g.time >= 3
                && expected
                    .events
                    .iter()
                    .any(|e| e.kind == Kind::Mc && e.addr == g.addr && e.time == g.time - 3)
                && !expected
                    .events
                    .iter()
                    .any(|e| e.kind.is_data() && e.addr == g.addr && e.time == g.time);
            !quirk
        })
        .collect();
    for i in 0..got.len().max(exp.len()) {
        match (got.get(i), exp.get(i)) {
            (Some(g), Some(e)) if g == e => {}
            (g, e) => {
                let show = |x: Option<&&Event>| x.map_or("(none)".to_string(), |v| v.text());
                let t = e.or(g).map_or(0, |v| v.time);
                out.push(format!(
                    "event {} at T{}: expected {}, got {}",
                    i,
                    t,
                    show(e),
                    show(g)
                ));
            }
        }
    }
    let (g, e) = (&result.regs, &expected.regs);
    let names = ["AF", "BC", "DE", "HL"];
    for (k, name) in names.iter().enumerate() {
        if g.main[k] != e.main[k] {
            out.push(format!(
                "{}: expected {:04x}, got {:04x}",
                name, e.main[k], g.main[k]
            ));
        }
        if g.alt[k] != e.alt[k] {
            out.push(format!(
                "{}': expected {:04x}, got {:04x}",
                name, e.alt[k], g.alt[k]
            ));
        }
    }
    let words = [
        ("IX", g.ix, e.ix),
        ("IY", g.iy, e.iy),
        ("SP", g.sp, e.sp),
        ("PC", g.pc, e.pc),
        ("MEMPTR", g.memptr, e.memptr),
    ];
    for (n, a, b) in words {
        if a != b {
            out.push(format!("{}: expected {:04x}, got {:04x}", n, b, a));
        }
    }
    let bytes = [
        ("I", g.i as u32, e.i as u32),
        ("R", g.r as u32, e.r as u32),
        ("IM", g.im as u32, e.im as u32),
    ];
    for (n, a, b) in bytes {
        if a != b {
            out.push(format!("{}: expected {:02x}, got {:02x}", n, b, a));
        }
    }
    if g.iff1 != e.iff1 || g.iff2 != e.iff2 {
        out.push(format!(
            "IFF: expected {}{}, got {}{}",
            e.iff1 as u8, e.iff2 as u8, g.iff1 as u8, g.iff2 as u8
        ));
    }
    if g.halted != e.halted {
        out.push(format!("halted: expected {}, got {}", e.halted, g.halted));
    }
    if g.tstates != e.tstates {
        out.push(format!(
            "tstates: expected {}, got {}",
            e.tstates, g.tstates
        ));
    }
    if result.mem_changes != expected.mem_changes {
        out.push(format!(
            "memory changes: expected {:?}, got {:?}",
            expected.mem_changes, result.mem_changes
        ));
    }
    out
}

/// FUSE tests that are known to disagree with the hardware-accurate behaviour of this crate
/// (and with SingleStepTests / z80test); each entry is documented in DECISIONS.md
/// (D-026) and `docs/qa/inbox/E13-14-Q1.md` and is excluded from [`check_all`].
pub const KNOWN: &[(&str, &str)] = &[
    ("edb9_2", "CPIR repeat: FUSE takes X/Y from the result, silicon from PC high (D-026)"),
    ("edb2_1", "INIR repeat: FUSE keeps MEMPTR=BC+1 and old H/PV, silicon gives PC+1 (D-026)"),
    ("edb3_1", "OTIR repeat: FUSE keeps MEMPTR and old H/PV, silicon gives PC+1 (D-026)"),
    ("edba_1", "INDR repeat: FUSE keeps MEMPTR, silicon gives PC+1 (D-026)"),
    ("edbb_1", "OTDR repeat: FUSE keeps MEMPTR and old H/PV, silicon gives PC+1 (D-026)"),
];

/// Run every test of `tests.in` and compare with `tests.expected` (matched by name).
///
/// Returns `(name, differences)` for each test that fails and is not in [`KNOWN`].
pub fn check_all(tests: &[FuseTest], expected: &[FuseExpected]) -> Vec<(String, Vec<String>)> {
    let mut fails = Vec::new();
    for t in tests {
        match expected.iter().find(|e| e.name == t.name) {
            None => fails.push((t.name.clone(), vec!["no expected record".to_string()])),
            Some(e) => {
                let d = compare(&run_fuse(t), e);
                let known = KNOWN.iter().any(|(n, _)| *n == t.name);
                if !d.is_empty() && !known {
                    fails.push((t.name.clone(), d));
                }
            }
        }
    }
    fails
}

#[cfg(test)]
mod tests {
    use super::*;

    const IN_NOP: &str = "00\n0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000\n00 00 0 0 0 0     1\n0000 00 -1\n-1\n\n";
    const EXP_NOP: &str = "00\n    0 MC 0000\n    4 MR 0000 00\n0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0001 0000\n00 01 0 0 0 0 4\n\n";

    #[test]
    fn e13_13_parse_in() {
        let t = parse_in(IN_NOP).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].name, "00");
        assert_eq!(t[0].tstates, 1);
        assert_eq!(t[0].mem_blocks, vec![(0u16, vec![0u8])]);
        let multi = "x\n1300 6b65 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000\n00 00 0 0 1 0 21\n4000 ed b0 -1\nc000 12 -1\n-1\n";
        let t = parse_in(multi).unwrap();
        assert_eq!(t[0].regs.main[0], 0x1300);
        assert_eq!(t[0].regs.im, 1);
        assert_eq!(t[0].tstates, 21);
        assert_eq!(t[0].mem_blocks.len(), 2);
    }

    #[test]
    fn e13_13_parse_expected() {
        let e = parse_expected(EXP_NOP).unwrap();
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].events.len(), 2);
        assert_eq!(
            e[0].events[1],
            Event {
                time: 4,
                kind: Kind::Mr,
                addr: 0,
                data: Some(0)
            }
        );
        assert_eq!(e[0].regs.pc, 1);
        assert_eq!(e[0].regs.r, 1);
        assert_eq!(e[0].regs.tstates, 4);
        assert!(e[0].mem_changes.is_empty());
        let w = "02\n    0 MC 0000\n    4 MR 0000 02\n    4 MC 0001\n    7 MW 0001 56\n5600 0001 0000 0000 0000 0000 0000 0000 0000 0000 0000 0001 5602\n00 01 0 0 0 0 7\n0001 56 -1\n\n";
        let e = parse_expected(w).unwrap();
        assert_eq!(e[0].mem_changes, vec![(1u16, vec![0x56u8])]);
    }

    #[test]
    fn e13_13_run_nop() {
        let t = parse_in(IN_NOP).unwrap();
        let e = parse_expected(EXP_NOP).unwrap();
        let diff = compare(&run_fuse(&t[0]), &e[0]);
        assert!(diff.is_empty(), "{:?}", diff);
    }

    #[test]
    fn e13_13_event_order() {
        // LD A,(0100h) with (0100h)=77h at address 0
        let src = "3a\n0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000\n00 00 0 0 0 0     1\n0000 3a 00 01 -1\n0100 77 -1\n-1\n";
        let t = parse_in(src).unwrap();
        let r = run_fuse(&t[0]);
        let seq: Vec<String> = r.events.iter().map(|e| e.text()).collect();
        assert_eq!(
            seq,
            [
                "0 MC 0000",
                "4 MR 0000 3a",
                "4 MC 0001",
                "7 MR 0001 00",
                "7 MC 0002",
                "10 MR 0002 01",
                "10 MC 0100",
                "13 MR 0100 77"
            ]
        );
        assert_eq!(r.regs.main[0] >> 8, 0x77);
        assert_eq!(r.regs.tstates, 13);
    }

    #[test]
    fn e13_13_mismatch_reported() {
        let t = parse_in(IN_NOP).unwrap();
        let mut e = parse_expected(EXP_NOP).unwrap();
        e[0].events[1].data = Some(0x99);
        let diff = compare(&run_fuse(&t[0]), &e[0]);
        assert_eq!(diff.len(), 1, "{:?}", diff);
        assert!(diff[0].contains("T4"), "{}", diff[0]);
        assert!(diff[0].contains("expected 4 MR 0000 99"), "{}", diff[0]);
    }
}
