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

pub mod ssc_cmd {
    use super::*;

    pub fn run(_args: SscArgs) -> Result<(), String> {
        Err("not implemented".into())
    }

    #[derive(Debug)]
    pub struct SscArgs {
        pub dir: PathBuf,
        pub filter: Option<String>,
    }
}

pub mod fuse_cmd {
    use super::*;

    pub fn run(_args: FuseArgs) -> Result<(), String> {
        Err("not implemented".into())
    }

    #[derive(Debug)]
    pub struct FuseArgs {
        pub dir: PathBuf,
    }
}

pub mod z80test_cmd {
    use super::*;

    pub fn run(_args: Z80testArgs) -> Result<(), String> {
        Err("not implemented".into())
    }

    #[derive(Debug)]
    pub struct Z80testArgs {
        pub rom: PathBuf,
    }
}

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
