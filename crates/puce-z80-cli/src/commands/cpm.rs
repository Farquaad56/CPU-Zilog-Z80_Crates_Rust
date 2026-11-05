//! `z80 cpm <rom> [--max-instr N]`: run a CP/M program with the emulated BDOS.

use puce_z80_machines::cpm::{CpmMachine, RunEnd};
use std::io::Write;
use std::path::PathBuf;

/// Arguments of the `cpm` command.
#[derive(Debug)]
pub struct CpmArgs {
    pub rom: PathBuf,
    pub max_instr: Option<u64>,
}

/// Load `rom`, run it, print its output on stdout.
///
/// Ok when the program reached the warm boot and printed no `ERROR` line.
pub fn run(args: CpmArgs) -> Result<(), String> {
    let bytes = std::fs::read(&args.rom)
        .map_err(|e| format!("cannot read {}: {}", args.rom.display(), e))?;
    let mut m = CpmMachine::new();
    m.load_com(&bytes);
    let end = m.run(args.max_instr.unwrap_or(u64::MAX));

    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(&m.out)
        .and_then(|_| stdout.flush())
        .map_err(|e| format!("cannot write output: {}", e))?;

    if m.tests_err > 0 {
        return Err(format!("{} ERROR line(s)", m.tests_err));
    }
    match end {
        RunEnd::Exited => Ok(()),
        RunEnd::MaxInstr => Err(format!("stopped after {} instructions", m.instr)),
        RunEnd::Halted => Err(format!(
            "CPU halted at {:04X} after {} instructions",
            m.cpu.regs.pc, m.instr
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_rom(name: &str, bytes: &[u8]) -> PathBuf {
        let p = std::env::temp_dir().join(format!("puce_z80_{}_{}", std::process::id(), name));
        std::fs::write(&p, bytes).unwrap();
        p
    }

    #[test]
    fn e13_03_cpm_exit_code_ok() {
        // LD E,'A'; LD C,2; CALL 5; JP 0
        let rom = temp_rom(
            "ok.com",
            &[0x1E, b'A', 0x0E, 0x02, 0xCD, 0x05, 0x00, 0xC3, 0x00, 0x00],
        );
        let r = run(CpmArgs {
            rom: rom.clone(),
            max_instr: Some(1000),
        });
        std::fs::remove_file(rom).ok();
        assert_eq!(r, Ok(()));
    }

    #[test]
    fn e13_03_cpm_missing_file() {
        let r = run(CpmArgs {
            rom: PathBuf::from("does/not/exist.com"),
            max_instr: None,
        });
        assert!(r.is_err());
    }
}
