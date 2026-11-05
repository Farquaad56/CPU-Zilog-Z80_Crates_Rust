//! `z80 fuse <dir>`: run the FUSE `tests.in` / `tests.expected` found in `dir`.

use puce_z80_machines::fuse::{check_all, parse_expected, parse_in};
use std::path::PathBuf;

/// Arguments of the `fuse` command.
#[derive(Debug)]
pub struct FuseArgs {
    pub dir: PathBuf,
}

/// Maximum number of failing tests printed in detail.
const MAX_SHOWN: usize = 20;

/// The summary line: `TOTAL: <passed>/<total> passed, <failed> failed`.
pub fn summary(total: usize, failed: usize) -> String {
    format!(
        "TOTAL: {}/{} passed, {} failed",
        total - failed,
        total,
        failed
    )
}

/// Run every FUSE test of the directory; Err (exit code 1) when one fails.
pub fn run(args: FuseArgs) -> Result<(), String> {
    let read = |name: &str| {
        let p = args.dir.join(name);
        std::fs::read_to_string(&p).map_err(|e| format!("cannot read {}: {}", p.display(), e))
    };
    let tests = parse_in(&read("tests.in")?)?;
    let expected = parse_expected(&read("tests.expected")?)?;
    let fails = check_all(&tests, &expected);
    for (name, diffs) in fails.iter().take(MAX_SHOWN) {
        println!("FAIL {}", name);
        for d in diffs.iter().take(6) {
            println!("    {}", d);
        }
    }
    if fails.len() > MAX_SHOWN {
        println!("... {} more failing tests", fails.len() - MAX_SHOWN);
    }
    println!("{}", summary(tests.len(), fails.len()));
    if fails.is_empty() {
        Ok(())
    } else {
        Err(format!("{} FUSE tests failed", fails.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e13_13_cli_summary() {
        assert_eq!(summary(10, 2), "TOTAL: 8/10 passed, 2 failed");
    }

    #[test]
    fn e13_13_cli_run_dir() {
        let dir = std::env::temp_dir().join("z80_e13_13_fuse");
        std::fs::create_dir_all(&dir).unwrap();
        let zeros = "0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000";
        std::fs::write(
            dir.join("tests.in"),
            format!("00\n{zeros}\n00 00 0 0 0 0     1\n0000 00 -1\n-1\n\n"),
        )
        .unwrap();
        let good = "0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0001 0000";
        std::fs::write(
            dir.join("tests.expected"),
            format!("00\n    0 MC 0000\n    4 MR 0000 00\n{good}\n00 01 0 0 0 0 4\n\n"),
        )
        .unwrap();
        assert!(run(FuseArgs { dir: dir.clone() }).is_ok());
        // wrong final PC -> failure
        std::fs::write(
            dir.join("tests.expected"),
            format!(
                "00\n    0 MC 0000\n    4 MR 0000 00\n{}\n00 01 0 0 0 0 4\n\n",
                good.replace("0001 0000", "0002 0000")
            ),
        )
        .unwrap();
        assert!(run(FuseArgs { dir }).is_err());
    }
}
