//! `z80 suite zexdoc|zexall|prelim [--max-instr N]`: run a CP/M test suite from `tests/roms/`.

use puce_z80_machines::cpm::{CpmMachine, RunEnd};
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

/// Arguments of the `suite` command.
#[derive(Debug)]
pub struct SuiteArgs {
    pub name: String,
    pub max_instr: Option<u64>,
}

/// Instructions executed between two progress outputs.
const CHUNK: u64 = 20_000_000;

/// File name of a suite inside `tests/roms/`.
fn rom_file(name: &str) -> Result<&'static str, String> {
    match name {
        "prelim" => Ok("prelim.com"),
        "zexdoc" => Ok("zexdoc.cim"),
        "zexall" => Ok("zexall.cim"),
        _ => Err(format!(
            "unknown suite '{}' (expected prelim, zexdoc or zexall)",
            name
        )),
    }
}

/// Find `tests/roms/<file>`: relative to the current directory, then to the source tree.
fn find_rom(file: &str) -> Result<PathBuf, String> {
    let candidates = [
        PathBuf::from("tests/roms").join(file),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/roms")
            .join(file),
    ];
    for c in candidates {
        if c.exists() {
            return Ok(c);
        }
    }
    Err(format!(
        "tests/roms/{} not found: run python docs/tools/fetch_roms.py",
        file
    ))
}

/// Result of a suite run.
pub struct Summary {
    pub suite: String,
    pub tests: u32,
    pub ok: u32,
    pub err: u32,
    pub instr: u64,
    pub tstates: u64,
    pub seconds: f64,
    pub exited: bool,
}

impl Summary {
    /// One-line JSON `{suite, tests, ok, err, instr, tstates, seconds, mips}`.
    pub fn json(&self) -> String {
        let mips = if self.seconds > 0.0 {
            self.instr as f64 / self.seconds / 1e6
        } else {
            0.0
        };
        serde_json::json!({
            "suite": self.suite,
            "tests": self.tests,
            "ok": self.ok,
            "err": self.err,
            "instr": self.instr,
            "tstates": self.tstates,
            "seconds": self.seconds,
            "mips": mips,
        })
        .to_string()
    }
}

/// Run `bytes` as a CP/M program, streaming its output to `out`.
pub fn run_program(
    suite: &str,
    bytes: &[u8],
    max_instr: u64,
    out: &mut dyn Write,
) -> Result<Summary, String> {
    let mut m = CpmMachine::new();
    m.load_com(bytes);
    let start = Instant::now();
    let mut left = max_instr;
    let mut shown = 0;
    let io_err = |e: std::io::Error| format!("cannot write output: {}", e);
    let end = loop {
        let n = left.min(CHUNK);
        let end = m.run(n);
        left -= n;
        out.write_all(&m.out[shown..]).map_err(io_err)?;
        out.flush().map_err(io_err)?;
        shown = m.out.len();
        if end != RunEnd::MaxInstr || left == 0 {
            break end;
        }
    };
    Ok(Summary {
        suite: suite.to_string(),
        tests: m.tests_ok + m.tests_err,
        ok: m.tests_ok,
        err: m.tests_err,
        instr: m.instr,
        tstates: m.tstates,
        seconds: start.elapsed().as_secs_f64(),
        exited: end == RunEnd::Exited,
    })
}

/// Run a suite; Ok when the program exited and printed no `ERROR` line.
pub fn run(args: SuiteArgs) -> Result<(), String> {
    let path = find_rom(rom_file(&args.name)?)?;
    let bytes =
        std::fs::read(&path).map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
    let mut stdout = std::io::stdout().lock();
    let s = run_program(
        &args.name,
        &bytes,
        args.max_instr.unwrap_or(u64::MAX),
        &mut stdout,
    )?;
    writeln!(stdout, "\n{}", s.json()).map_err(|e| e.to_string())?;
    if s.err > 0 {
        return Err(format!("{} ERROR line(s)", s.err));
    }
    if !s.exited {
        return Err(format!(
            "{} did not finish after {} instructions",
            s.suite, s.instr
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e13_04_suite_json_summary() {
        // LD E,'A'; LD C,2; CALL 5; JP 0
        let prog = [0x1E, b'A', 0x0E, 0x02, 0xCD, 0x05, 0x00, 0xC3, 0x00, 0x00];
        let mut buf: Vec<u8> = Vec::new();
        let s = run_program("mini", &prog, 1000, &mut buf).unwrap();
        writeln!(buf, "\n{}", s.json()).unwrap();
        let text = String::from_utf8(buf).unwrap();
        let last = text.lines().last().unwrap();
        let v: serde_json::Value = serde_json::from_str(last).unwrap();
        for k in [
            "suite", "tests", "ok", "err", "instr", "tstates", "seconds", "mips",
        ] {
            assert!(v.get(k).is_some(), "missing field {}", k);
        }
        assert_eq!(v["suite"], "mini");
        assert_eq!(v["instr"], 5);
    }

    #[test]
    fn e13_04_unknown_suite() {
        assert!(rom_file("bogus").is_err());
    }
}
