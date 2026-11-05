// Temporary: the command arguments are not read until the commands are implemented.
#![allow(dead_code)]

use std::path::PathBuf;

pub mod run_cmd {
    use super::*;

    pub fn run(_args: RunArgs) -> Result<(), String> {
        Err("not implemented".into())
    }

    #[derive(Debug)]
    pub struct RunArgs {
        pub rom: PathBuf,
    }
}

#[path = "cpm.rs"]
pub mod cpm_cmd;

#[path = "suite.rs"]
pub mod suite_cmd;

#[path = "ssc.rs"]
pub mod ssc_cmd;

#[path = "fuse.rs"]
pub mod fuse_cmd;

#[path = "z80test.rs"]
pub mod z80test_cmd;

#[path = "bench.rs"]
pub mod bench_cmd;

#[path = "all.rs"]
pub mod all_cmd;

pub mod disasm_cmd {
    use super::*;

    pub fn run(_args: DisasmArgs) -> Result<(), String> {
        Err("not implemented".into())
    }

    #[derive(Debug)]
    pub struct DisasmArgs {
        pub file: PathBuf,
        pub org: String,
    }
}

pub mod fetch_cmd {
    pub fn run(_args: ()) -> Result<(), String> {
        Err("not implemented".into())
    }
}
