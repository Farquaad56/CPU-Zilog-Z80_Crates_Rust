//! `z80 z80test <file.tap> [--rom zx48.rom] [--max-frames N]`: run one z80test program
//! (docs/specs/35_Z80TEST.md) on the minimal ZX Spectrum and report its verdict.

use puce_z80_machines::zx::{parse_tap, ZxMachine, FRAME_TSTATES};
use std::io::Write;
use std::path::PathBuf;

/// Arguments of the `z80test` command.
#[derive(Debug)]
pub struct Z80testArgs {
    pub tap: PathBuf,
    pub rom: Option<PathBuf>,
    pub max_frames: Option<u64>,
}

/// Text printed by z80test when every test passed.
pub const PASS_TEXT: &str = "all tests passed.";
/// Where the program is entered from: the sentinel return address.
const SENTINEL: u16 = 0x0000;
/// Stack of the BASIC program after `CLEAR 32767`.
const STACK: u16 = 0x7FF0;
/// Frames run between two flushes of the captured output.
const CHUNK_FRAMES: u64 = 50;

/// The verdict derived from the captured screen text.
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    /// `Result: all tests passed.` and no `FAILED`.
    Passed,
    /// The program reported failures.
    Failed,
    /// No final `Result:` line (not finished or garbled output).
    Unfinished,
}

/// Decide the verdict of a run from its output.
pub fn verdict(out: &str) -> Verdict {
    if out.contains("FAILED") || out.contains("tests failed") {
        Verdict::Failed
    } else if out.contains(PASS_TEXT) {
        Verdict::Passed
    } else {
        Verdict::Unfinished
    }
}

/// Build the machine: tape loaded, CODE block copied to its address, ready to run.
pub fn build_machine(tap: &[u8], rom: Option<&[u8]>) -> Result<ZxMachine, String> {
    let blocks = parse_tap(tap)?;
    let hdr = blocks
        .iter()
        .position(|b| b.flag == 0 && b.data.first() == Some(&3) && b.data.len() >= 15)
        .ok_or("no CODE header in the tape")?;
    let start = u16::from(blocks[hdr].data[13]) | (u16::from(blocks[hdr].data[14]) << 8);
    let code = blocks
        .get(hdr + 1)
        .ok_or("CODE header without data block")?;
    let mut m = match rom {
        Some(r) => {
            let mut m = ZxMachine::new(r)?;
            // The ROM is not booted: keep the simulated CHAN-OPEN.
            m.rom_loaded = false;
            m
        }
        None => {
            let mut m = ZxMachine::new_romless();
            // Interrupt routine of the ROM, reduced to EI / RET.
            m.load_code(0x0038, &[0xFB, 0xC9]);
            m
        }
    };
    m.load_code(start, &code.data);
    m.cpu.regs.pc = start;
    m.cpu.regs.sp = STACK - 2;
    m.load_code(STACK - 2, &SENTINEL.to_le_bytes());
    Ok(m)
}

/// Run the program, streaming the new output to `sink`; returns the whole output and whether
/// the program returned to the sentinel.
pub fn run_machine(
    m: &mut ZxMachine,
    max_frames: u64,
    sink: &mut dyn Write,
) -> Result<(String, bool), String> {
    let mut printed = 0usize;
    let mut finished = false;
    let mut frames = 0u64;
    while frames < max_frames {
        let n = CHUNK_FRAMES.min(max_frames - frames);
        finished = m.run_until_pc(SENTINEL, n * u64::from(FRAME_TSTATES));
        frames += n;
        if m.out.len() > printed {
            sink.write_all(&m.out.as_bytes()[printed..])
                .and_then(|_| sink.flush())
                .map_err(|e| format!("cannot write output: {}", e))?;
            printed = m.out.len();
        }
        if finished || m.cpu.regs.halted {
            break;
        }
    }
    Ok((m.out.clone(), finished))
}

/// Run a z80test tape; Err (exit code 1) unless it reports `all tests passed.`.
pub fn run(args: Z80testArgs) -> Result<(), String> {
    let tap = std::fs::read(&args.tap)
        .map_err(|e| format!("cannot read {}: {}", args.tap.display(), e))?;
    let rom = match &args.rom {
        Some(p) => {
            Some(std::fs::read(p).map_err(|e| format!("cannot read {}: {}", p.display(), e))?)
        }
        None => None,
    };
    let mut m = build_machine(&tap, rom.as_deref())?;
    let mut stdout = std::io::stdout().lock();
    let (out, finished) = run_machine(&mut m, args.max_frames.unwrap_or(1_000_000), &mut stdout)?;
    match verdict(&out) {
        Verdict::Passed if finished => Ok(()),
        Verdict::Failed => Err("z80test reported failures".into()),
        _ => Err(format!(
            "z80test did not finish (frames budget or HALT, PC={:04x})",
            m.cpu.regs.pc
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tap_block(flag: u8, data: &[u8]) -> Vec<u8> {
        let mut body = vec![flag];
        body.extend_from_slice(data);
        let xor = body.iter().fold(0u8, |a, b| a ^ b);
        body.push(xor);
        let mut out = vec![body.len() as u8, (body.len() >> 8) as u8];
        out.extend(body);
        out
    }

    /// A tape whose CODE prints "all tests passed." through RST 10h and returns.
    fn passing_tape() -> Vec<u8> {
        let mut code = Vec::new();
        for ch in b"Result: all tests passed.\r" {
            code.extend_from_slice(&[0x3E, *ch, 0xD7]);
        }
        code.push(0xC9);
        let mut hdr = vec![3u8];
        hdr.extend_from_slice(b"test      ");
        hdr.extend_from_slice(&(code.len() as u16).to_le_bytes());
        hdr.extend_from_slice(&0x8000u16.to_le_bytes());
        hdr.extend_from_slice(&0u16.to_le_bytes());
        let mut tap = tap_block(0, &hdr);
        tap.extend(tap_block(0xFF, &code));
        tap
    }

    #[test]
    fn e13_17_verdict_texts() {
        assert_eq!(verdict("Result: all tests passed.\n"), Verdict::Passed);
        assert_eq!(
            verdict("1 x FAILED\nResult: 1 of 163 tests failed.\n"),
            Verdict::Failed
        );
        assert_eq!(verdict("Z80 doc test"), Verdict::Unfinished);
    }

    #[test]
    fn e13_17_run_passing_tape() {
        let mut m = build_machine(&passing_tape(), None).unwrap();
        let mut sink = Vec::new();
        let (out, finished) = run_machine(&mut m, 10, &mut sink).unwrap();
        assert!(finished);
        assert_eq!(verdict(&out), Verdict::Passed);
        assert_eq!(String::from_utf8(sink).unwrap(), out);
    }

    #[test]
    fn e13_17_tape_without_code_is_refused() {
        let tap = tap_block(0xFF, &[1, 2, 3]);
        assert!(build_machine(&tap, None).is_err());
    }
}
