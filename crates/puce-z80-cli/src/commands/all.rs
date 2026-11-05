//! `z80 suite all [--skip a,b] [--json out.json]`: run every available suite and summarise.
//!
//! Each suite is run by re-invoking this executable with the matching sub-command, so the
//! result is exactly what the sub-command reports. Status per suite: PASS, FAIL, SKIP(reason)
//! or UNAVAILABLE. The exit code is 0 when no suite failed and no mandatory suite was skipped.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

/// Arguments of `suite all`.
#[derive(Debug)]
pub struct AllArgs {
    pub skip: Option<String>,
    pub json: Option<PathBuf>,
}

/// Suites that must run and pass for exit code 0.
pub const MANDATORY: [&str; 6] = ["prelim", "zexdoc", "zexall", "ssc", "fuse", "z80test"];

/// The six z80test programs (tests/roms/z80test/release/z80test-1.2a/<name>.tap).
const Z80TEST_TAPS: [&str; 6] = [
    "z80doc",
    "z80docflags",
    "z80flags",
    "z80full",
    "z80ccf",
    "z80memptr",
];

/// Outcome of a suite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Pass,
    Fail,
    Skip,
    Unavailable,
}

impl Status {
    /// Upper-case name used in the JSON and the report.
    pub fn name(self) -> &'static str {
        match self {
            Status::Pass => "PASS",
            Status::Fail => "FAIL",
            Status::Skip => "SKIP",
            Status::Unavailable => "UNAVAILABLE",
        }
    }
}

/// Result of one suite.
#[derive(Debug, Clone)]
pub struct SuiteResult {
    pub name: String,
    pub status: Status,
    pub details: String,
    pub seconds: f64,
}

fn result(name: &str, status: Status, details: &str, seconds: f64) -> SuiteResult {
    SuiteResult {
        name: name.to_string(),
        status,
        details: details.to_string(),
        seconds,
    }
}

/// `tests/roms` relative to the current directory, else to the source tree.
pub fn roms_dir() -> PathBuf {
    let here = PathBuf::from("tests/roms");
    if here.exists() {
        here
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/roms")
    }
}

/// Run `exe args...`; returns (success, last meaningful output line).
fn spawn(exe: &Path, args: &[&str]) -> (bool, String) {
    match Command::new(exe).args(args).output() {
        Err(e) => (false, format!("cannot run {}: {}", exe.display(), e)),
        Ok(o) => {
            let last = |b: &[u8]| {
                String::from_utf8_lossy(b)
                    .lines()
                    .rev()
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or("")
                    .chars()
                    .take(200)
                    .collect::<String>()
            };
            let ok = o.status.success();
            let line = if ok || o.stderr.is_empty() {
                last(&o.stdout)
            } else {
                last(&o.stderr)
            };
            (ok, line)
        }
    }
}

/// Finish a suite that ran `exe args`.
fn run_sub(name: &str, exe: Option<&Path>, args: &[&str]) -> SuiteResult {
    let Some(exe) = exe else {
        return result(name, Status::Skip, "no executable to run the suite", 0.0);
    };
    let start = Instant::now();
    let (ok, line) = spawn(exe, args);
    let st = if ok { Status::Pass } else { Status::Fail };
    result(name, st, &line, start.elapsed().as_secs_f64())
}

fn missing(name: &str, what: &Path) -> SuiteResult {
    let reason = format!(
        "{} not found (python docs/tools/fetch_roms.py)",
        what.display()
    );
    result(name, Status::Skip, &reason, 0.0)
}

/// Run every suite not listed in `skip`. `exe` is the z80 executable (None: nothing can run).
pub fn run_all(roms: &Path, skip: &[String], exe: Option<&Path>) -> Vec<SuiteResult> {
    let wanted = |n: &str| !skip.iter().any(|s| s == n);
    let mut out = Vec::new();
    for (name, file) in [
        ("prelim", "prelim.com"),
        ("zexdoc", "zexdoc.cim"),
        ("zexall", "zexall.cim"),
    ] {
        if !wanted(name) {
            continue;
        }
        let p = roms.join(file);
        out.push(if p.exists() {
            run_sub(name, exe, &["suite", name])
        } else {
            missing(name, &p)
        });
    }
    if wanted("ssc") {
        let p = roms.join("singlestep");
        out.push(if p.exists() {
            run_sub("ssc", exe, &["ssc", &p.to_string_lossy(), "--cycles"])
        } else {
            missing("ssc", &p)
        });
    }
    if wanted("fuse") {
        let p = roms.join("fuse");
        out.push(if p.join("tests.in").exists() {
            run_sub("fuse", exe, &["fuse", &p.to_string_lossy()])
        } else {
            missing("fuse", &p)
        });
    }
    if wanted("z80test") {
        let dir = roms.join("z80test/release/z80test-1.2a");
        let absent: Vec<&str> = Z80TEST_TAPS
            .iter()
            .copied()
            .filter(|t| !dir.join(format!("{}.tap", t)).exists())
            .collect();
        out.push(if absent.is_empty() {
            let start = Instant::now();
            let mut fail = None;
            for t in Z80TEST_TAPS {
                let tap = dir.join(format!("{}.tap", t));
                let r = run_sub(t, exe, &["z80test", &tap.to_string_lossy()]);
                if r.status != Status::Pass && fail.is_none() {
                    fail = Some(format!("{}: {}", t, r.details));
                }
            }
            let secs = start.elapsed().as_secs_f64();
            match fail {
                None => result("z80test", Status::Pass, "6/6 programs passed", secs),
                Some(f) => result("z80test", Status::Fail, &f, secs),
            }
        } else {
            let reason = format!("missing tapes: {}", absent.join(", "));
            result("z80test", Status::Skip, &reason, 0.0)
        });
    }
    if wanted("timing") {
        out.push(result(
            "timing",
            Status::Unavailable,
            "run: cargo test --release -p puce-z80-cli --test timing_roms -- --ignored",
            0.0,
        ));
    }
    if wanted("diff") {
        out.push(result(
            "diff",
            Status::Unavailable,
            "run: Z80_DIFF_TABLE=all cargo test --release -p puce-z80 --test diff",
            0.0,
        ));
    }
    out
}

/// True when no suite failed and no mandatory suite was skipped.
pub fn exit_ok(results: &[SuiteResult]) -> bool {
    results.iter().all(|r| {
        r.status != Status::Fail
            && !(r.status == Status::Skip && MANDATORY.contains(&r.name.as_str()))
    })
}

/// `{suites:[{name,status,details,seconds}]}`.
pub fn to_json(results: &[SuiteResult]) -> serde_json::Value {
    let suites: Vec<_> = results
        .iter()
        .map(|r| {
            serde_json::json!({
                "name": r.name,
                "status": r.status.name(),
                "details": r.details,
                "seconds": r.seconds,
            })
        })
        .collect();
    serde_json::json!({ "suites": suites })
}

/// Run `suite all`.
pub fn run(args: AllArgs) -> Result<(), String> {
    let skip: Vec<String> = args
        .skip
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let exe = std::env::current_exe().ok();
    let results = run_all(&roms_dir(), &skip, exe.as_deref());
    for r in &results {
        println!(
            "{:<12} {:<12} {:>8.1}s  {}",
            r.name,
            r.status.name(),
            r.seconds,
            r.details
        );
    }
    if let Some(p) = &args.json {
        let text = serde_json::to_string_pretty(&to_json(&results)).map_err(|e| e.to_string())?;
        std::fs::write(p, text).map_err(|e| format!("cannot write {}: {}", p.display(), e))?;
    }
    if exit_ok(&results) {
        Ok(())
    } else {
        Err("a suite failed or a mandatory suite was skipped".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_dir(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("puce_z80_all_{}_{}", std::process::id(), tag));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn e13_21_all_skip_missing() {
        let d = empty_dir("missing");
        let r = run_all(&d, &[], None);
        for s in &r {
            assert!(
                matches!(s.status, Status::Skip | Status::Unavailable),
                "{} is {:?}",
                s.name,
                s.status
            );
        }
        assert!(r.iter().filter(|s| s.status == Status::Skip).count() >= 6);
        assert!(!exit_ok(&r));
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn e13_21_json_shape() {
        let d = empty_dir("json");
        let v = to_json(&run_all(&d, &[], None));
        let suites = v["suites"].as_array().expect("suites array");
        assert!(!suites.is_empty());
        for s in suites {
            assert!(s["name"].is_string() && s["status"].is_string());
            assert!(s.get("details").is_some() && s.get("seconds").is_some());
        }
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn e13_21_skip_flag() {
        let d = empty_dir("skip");
        let skip = vec!["zexall".to_string(), "timing".to_string()];
        let r = run_all(&d, &skip, None);
        assert!(r.iter().all(|s| s.name != "zexall" && s.name != "timing"));
        assert!(r.iter().any(|s| s.name == "zexdoc"));
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn e13_21_report_table() {
        let script =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/tools/suites_report.py");
        let d = empty_dir("report");
        let json = d.join("s.json");
        std::fs::write(&json, to_json(&run_all(&d, &[], None)).to_string()).unwrap();
        let out = match Command::new("python").arg(&script).arg(&json).output() {
            Ok(o) => o,
            Err(e) => {
                eprintln!("SKIP: python not available ({})", e);
                std::fs::remove_dir_all(d).ok();
                return;
            }
        };
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(text.contains("| Suite | Status |"), "{}", text);
        assert!(text.contains("| prelim | SKIP |"), "{}", text);
        std::fs::remove_dir_all(d).ok();
    }
}
