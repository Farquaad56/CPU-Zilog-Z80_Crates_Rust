//! `z80 ssc <dir> [--filter 00,dd/21] [--max-fail N]`: run SingleStepTests/z80 files and
//! compare the final CPU / memory state (bus cycles are compared by later tasks).

use puce_z80_machines::ssc::{check_case, check_case_cycles, load_file};
use serde::Deserialize;
use std::collections::{BTreeSet, HashMap};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Arguments of the `ssc` command.
#[derive(Debug)]
pub struct SscArgs {
    pub dir: PathBuf,
    pub filter: Option<String>,
    pub max_fail: Option<u32>,
    /// Also compare the bus cycles T-state by T-state
    pub cycles: bool,
}

/// `tests/baseline.json`: opcodes whose failures are known and explained.
#[derive(Debug, Default, Deserialize)]
pub struct Baseline {
    #[serde(default)]
    pub known_failures: Vec<String>,
    #[serde(default)]
    pub reasons: HashMap<String, String>,
}

/// Totals of a run.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Totals {
    pub cases: u64,
    pub passed: u64,
    /// Failing cases of opcodes that are not in the baseline.
    pub new_failures: u64,
    /// Failing cases of opcodes listed in the baseline.
    pub known_failures: u64,
}

/// Convert one filter token to a file stem: `00` -> `00`, `dd/21` -> `dd 21`,
/// `dd/cb/06` -> `dd cb __ 06`.
fn token_to_stem(token: &str) -> String {
    let parts: Vec<String> = token
        .trim()
        .to_lowercase()
        .split('/')
        .map(str::to_string)
        .collect();
    if parts.len() == 3 && parts[1] == "cb" {
        format!("{} cb __ {}", parts[0], parts[2])
    } else {
        parts.join(" ")
    }
}

/// Parse a comma-separated filter into a set of file stems.
pub fn parse_filter(filter: &str) -> BTreeSet<String> {
    filter
        .split(',')
        .filter(|t| !t.trim().is_empty())
        .map(token_to_stem)
        .collect()
}

/// One summary line: `<opcode>: <passed>/<total>`, with a note for known failures.
pub fn summary_line(stem: &str, passed: usize, total: usize, note: Option<&str>) -> String {
    match note {
        Some(n) if passed < total => format!("{}: {}/{} (known: {})", stem, passed, total, n),
        _ => format!("{}: {}/{}", stem, passed, total),
    }
}

/// Test files of `dir` (or of `dir/v1`), sorted by name, filtered by stem.
fn list_files(dir: &Path, filter: &Option<BTreeSet<String>>) -> Result<Vec<PathBuf>, String> {
    let root = if dir.join("v1").is_dir() {
        dir.join("v1")
    } else {
        dir.to_path_buf()
    };
    let rd =
        std::fs::read_dir(&root).map_err(|e| format!("cannot read {}: {}", root.display(), e))?;
    let mut files: Vec<PathBuf> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "json").unwrap_or(false))
        .filter(|p| {
            let stem = p
                .file_stem()
                .map(|s| s.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            filter.as_ref().map(|f| f.contains(&stem)).unwrap_or(true)
        })
        .collect();
    files.sort();
    Ok(files)
}

/// Load `tests/baseline.json` (current directory, then the source tree); empty if absent.
fn load_baseline() -> Result<Baseline, String> {
    let candidates = [
        PathBuf::from("tests/baseline.json"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/baseline.json"),
    ];
    for c in candidates {
        if c.exists() {
            let text = std::fs::read_to_string(&c)
                .map_err(|e| format!("cannot read {}: {}", c.display(), e))?;
            return serde_json::from_str(&text)
                .map_err(|e| format!("invalid {}: {}", c.display(), e));
        }
    }
    Ok(Baseline::default())
}

/// Run the selected files, writing the report to `out`.
pub fn run_dir(
    dir: &Path,
    filter: Option<&str>,
    max_fail: u32,
    cycles: bool,
    baseline: &Baseline,
    out: &mut dyn Write,
) -> Result<Totals, String> {
    let filter = filter.map(parse_filter);
    let files = list_files(dir, &filter)?;
    if files.is_empty() {
        return Err(format!("no test file found in {}", dir.display()));
    }
    let io_err = |e: std::io::Error| format!("cannot write report: {}", e);
    let mut totals = Totals::default();
    let mut shown: u32 = 0;
    'files: for path in files {
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let known = baseline.known_failures.contains(&stem);
        let cases = load_file(&path)?;
        let mut passed = 0usize;
        for case in &cases {
            let diffs = if cycles {
                check_case_cycles(case)
            } else {
                check_case(case)
            };
            totals.cases += 1;
            if diffs.is_empty() {
                passed += 1;
                totals.passed += 1;
            } else if known {
                totals.known_failures += 1;
            } else {
                totals.new_failures += 1;
                shown += 1;
                writeln!(out, "FAIL {} [{}]", stem, case.name).map_err(io_err)?;
                for d in &diffs {
                    writeln!(out, "    {}", d).map_err(io_err)?;
                }
                if shown >= max_fail {
                    writeln!(out, "stopped after {} failing cases", shown).map_err(io_err)?;
                    break 'files;
                }
            }
        }
        let note = baseline.reasons.get(&stem).map(String::as_str);
        let line = summary_line(
            &stem,
            passed,
            cases.len(),
            note.or(known.then_some("baseline")),
        );
        writeln!(out, "{}", line).map_err(io_err)?;
    }
    writeln!(
        out,
        "TOTAL: {}/{} passed, {} new failures, {} known failures",
        totals.passed, totals.cases, totals.new_failures, totals.known_failures
    )
    .map_err(io_err)?;
    Ok(totals)
}

/// Run the SingleStepTests; Err (exit code 1) when a case fails outside the baseline.
pub fn run(args: SscArgs) -> Result<(), String> {
    let baseline = load_baseline()?;
    let mut stdout = std::io::stdout().lock();
    let totals = run_dir(
        &args.dir,
        args.filter.as_deref(),
        args.max_fail.unwrap_or(20),
        args.cycles,
        &baseline,
        &mut stdout,
    )?;
    if totals.new_failures > 0 {
        return Err(format!("{} failing case(s)", totals.new_failures));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOP_CASE: &str = r#"[{"name": "00 0000",
       "initial": {"a":1,"b":0,"c":0,"d":0,"e":0,"f":0,"h":0,"l":0,"i":0,"r":0,
         "af_":0,"bc_":0,"de_":0,"hl_":0,"ix":0,"iy":0,"pc":100,
         "sp":0,"wz":0,"iff1":0,"iff2":0,"im":0,"ei":0,"p":0,"q":0,"ram":[[100,0]]},
       "final": {"a":1,"b":0,"c":0,"d":0,"e":0,"f":0,"h":0,"l":0,"i":0,"r":1,
         "af_":0,"bc_":0,"de_":0,"hl_":0,"ix":0,"iy":0,"pc":101,
         "sp":0,"wz":0,"iff1":0,"iff2":0,"im":0,"ei":0,"p":0,"q":0,"ram":[[100,0]]},
       "cycles": [[100,null,"----"],[100,null,"r-m-"],[1,0,"----"],[1,null,"----"]]}]"#;

    fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("puce_z80_ssc_{}_{}", std::process::id(), tag));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn e13_08_summary_format() {
        assert_eq!(summary_line("00", 1000, 1000, None), "00: 1000/1000");
        assert_eq!(
            summary_line("76", 990, 1000, Some("halt")),
            "76: 990/1000 (known: halt)"
        );
        assert_eq!(
            summary_line("76", 1000, 1000, Some("halt")),
            "76: 1000/1000"
        );
    }

    #[test]
    fn e13_10_cycles_match_sample() {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/roms/singlestep");
        if !dir.exists() {
            eprintln!("SKIP: run python docs/tools/fetch_roms.py --optional singlestep");
            return;
        }
        let sample =
            "00,01,3e,3a,c5,cd,db,d3,dd/cb/06,76,ed/a0,c9,10,dd/21,fd/34,cb/46,e3,ed/45,ed/b0,f9";
        let mut out: Vec<u8> = Vec::new();
        let t = run_dir(
            &dir,
            Some(sample),
            20,
            true,
            &load_baseline().unwrap(),
            &mut out,
        )
        .unwrap();
        let text = String::from_utf8(out).unwrap();
        assert_eq!(t.new_failures, 0, "{}", text);
        assert!(t.cases >= 20000, "{}", text);
    }

    #[test]
    fn e13_08_filter() {
        let f = parse_filter("00,dd/21,dd/cb/06");
        let want: BTreeSet<String> = ["00", "dd 21", "dd cb __ 06"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(f, want);

        let dir = temp_dir("filter");
        std::fs::write(dir.join("00.json"), NOP_CASE).unwrap();
        std::fs::write(dir.join("01.json"), "not json").unwrap();
        let mut out: Vec<u8> = Vec::new();
        let t = run_dir(&dir, Some("00"), 5, false, &Baseline::default(), &mut out).unwrap();
        std::fs::remove_dir_all(&dir).ok();
        let text = String::from_utf8(out).unwrap();
        assert_eq!(t.cases, 1);
        assert_eq!(t.passed, 1);
        assert!(text.contains("00: 1/1"), "{}", text);
        assert!(!text.contains("01"), "{}", text);
    }
}
