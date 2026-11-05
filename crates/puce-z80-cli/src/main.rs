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
        dir: PathBuf,
        filter: Option<String>,
    },
    /// Run FUSE test ROMs from a directory
    Fuse { dir: PathBuf },
    /// Run Z80test ROM
    Z80test { rom: PathBuf },
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
        Commands::Ssc { dir, filter } => {
            commands::ssc_cmd::run(commands::ssc_cmd::SscArgs { dir, filter })
        }
        Commands::Fuse { dir } => commands::fuse_cmd::run(commands::fuse_cmd::FuseArgs { dir }),
        Commands::Z80test { rom } => {
            commands::z80test_cmd::run(commands::z80test_cmd::Z80testArgs { rom })
        }
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
