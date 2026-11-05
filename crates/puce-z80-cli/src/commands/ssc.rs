//! `z80 ssc <dir>`: run SingleStepTests/z80 files.

use std::path::PathBuf;

/// Arguments of the `ssc` command.
#[derive(Debug)]
pub struct SscArgs {
    pub dir: PathBuf,
    pub filter: Option<String>,
}

/// Run the SingleStepTests (not implemented yet).
pub fn run(_args: SscArgs) -> Result<(), String> {
    Err("not implemented".into())
}
