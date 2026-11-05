use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::exit;

mod commands;

#[derive(Parser)]
#[command(name = "z80", version, about = "Z80 emulator CLI tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run a ROM file
    Run { rom: PathBuf },
    /// CP/M mode with BDOS calls
    Cpm {
        rom: PathBuf,
        /// Stop after this many instructions
        #[arg(long)]
        max_instr: Option<u64>,
    },
    /// Run a test suite by name
    Suite {
        /// prelim, zexdoc or zexall
        name: String,
        /// Stop after this many instructions
        #[arg(long)]
        max_instr: Option<u64>,
    },
    /// Run SingleStepTests from a directory
    Ssc {
        /// Directory of the SingleStepTests clone (or of its v1 folder)
        dir: PathBuf,
        /// Comma-separated opcodes, e.g. 00,dd/21,dd/cb/06
        #[arg(long)]
        filter: Option<String>,
        /// Stop after this many failing cases (default 20)
        #[arg(long)]
        max_fail: Option<u32>,
        /// Also compare the bus cycles T-state by T-state
        #[arg(long)]
        cycles: bool,
    },
    /// Run FUSE test ROMs from a directory
    Fuse { dir: PathBuf },
    /// Run a z80test program (.tap) on the minimal ZX Spectrum
    Z80test {
        /// The .tap file (z80doc.tap, z80full.tap, ...)
        tap: PathBuf,
        /// Optional 16 KiB ZX Spectrum 48K ROM image
        #[arg(long)]
        rom: Option<PathBuf>,
        /// Stop after this many 50 Hz frames
        #[arg(long)]
        max_frames: Option<u64>,
    },
    /// Disassemble a binary file
    Disasm { file: PathBuf, org: String },
    /// Fetch test ROMs and data files
    Fetch,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Run { rom } => commands::run_cmd::run(commands::run_cmd::RunArgs { rom }),
        Commands::Cpm { rom, max_instr } => {
            commands::cpm_cmd::run(commands::cpm_cmd::CpmArgs { rom, max_instr })
        }
        Commands::Suite { name, max_instr } => {
            commands::suite_cmd::run(commands::suite_cmd::SuiteArgs { name, max_instr })
        }
        Commands::Ssc {
            dir,
            filter,
            max_fail,
            cycles,
        } => commands::ssc_cmd::run(commands::ssc_cmd::SscArgs {
            dir,
            filter,
            max_fail,
            cycles,
        }),
        Commands::Fuse { dir } => commands::fuse_cmd::run(commands::fuse_cmd::FuseArgs { dir }),
        Commands::Z80test {
            tap,
            rom,
            max_frames,
        } => commands::z80test_cmd::run(commands::z80test_cmd::Z80testArgs {
            tap,
            rom,
            max_frames,
        }),
        Commands::Disasm { file, org } => {
            commands::disasm_cmd::run(commands::disasm_cmd::DisasmArgs { file, org })
        }
        Commands::Fetch => commands::fetch_cmd::run(()),
    };

    if let Err(msg) = result {
        eprintln!("error: {}", msg);
        exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e01_02_parse_cpm() {
        let cli = Cli::try_parse_from(["z80", "cpm", "x.com"]);
        assert!(
            cli.is_ok(),
            "Failed to parse cpm subcommand: {:?}",
            cli.err()
        );
        match cli.unwrap().command {
            Commands::Cpm { rom, max_instr } => {
                assert_eq!(rom.to_string_lossy(), "x.com");
                assert!(max_instr.is_none());
            }
            other => panic!("Expected Cpm subcommand, got {:?}", other),
        }
    }

    #[test]
    fn e01_02_unknown_cmd() {
        let cli = Cli::try_parse_from(["z80", "bogus"]);
        assert!(cli.is_err(), "Expected parse error for unknown subcommand");
    }
}
