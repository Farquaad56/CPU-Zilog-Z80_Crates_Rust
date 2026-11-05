//! The page (`docs/frontend/index.html`) only sends commands and displays states: this file
//! checks that mechanically. The page is not modified by these tests.

use std::path::PathBuf;

fn page() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/frontend/index.html");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {}", path.display(), e));
    String::from_utf8_lossy(&bytes).into_owned()
}

/// The page without its translation / description dictionaries (plain English or French
/// prose, where words such as "parity" legitimately appear).
fn code_only(page: &str) -> String {
    const DICTIONARIES: [&str; 5] = [
        "const EN=",
        "const EN2=",
        "const DESCEN=",
        "const ATTR_EN=",
        "const DESC=",
    ];
    let mut out = String::new();
    let mut skipping = false;
    for line in page.lines() {
        let t = line.trim_start();
        if DICTIONARIES.iter().any(|d| t.starts_with(d)) {
            // A multi-line dictionary ends at the line holding "};".
            skipping = !t.trim_end().ends_with("};") && !t.trim_end().ends_with("}");
            continue;
        }
        if skipping {
            if line.trim_end().ends_with("};") || line.trim() == "}" {
                skipping = false;
            }
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

#[test]
fn e15_01_title() {
    let p = page();
    let start = p.find("<title>").expect("<title>") + "<title>".len();
    let end = p[start..].find("</title>").expect("</title>") + start;
    assert!(
        p[start..end].contains("Zilog Z80 Emulator"),
        "{}",
        &p[start..end]
    );
}

#[test]
fn e15_01_ws_endpoint() {
    let p = page();
    assert!(p.contains("new WebSocket("));
    assert!(p.contains("/ws"), "the WebSocket must point at /ws");
}

#[test]
fn e15_01_all_commands() {
    let p = page();
    for cmd in [
        "hello",
        "list_roms",
        "load",
        "upload",
        "run",
        "pause",
        "step",
        "reset",
        "speed",
        "bp_add",
        "bp_del",
        "view",
        "console_clear",
    ] {
        let single = format!("cmd:'{}'", cmd);
        let double = format!("cmd:\"{}\"", cmd);
        let ternary = format!("'{}'", cmd); // e.g. cmd: isRunning() ? 'pause' : 'run'
        assert!(
            p.contains(&single) || p.contains(&double) || p.contains(&ternary),
            "command '{}' is not sent by the page",
            cmd
        );
    }
}

#[test]
fn e15_01_no_cpu_logic() {
    let code = code_only(&page());
    for banned in [
        "execMain",
        "execED",
        "execCB",
        "alu(",
        "parity",
        "class Z80",
        "function exec",
        "opcode ===",
    ] {
        assert!(
            !code.contains(banned),
            "the page contains CPU logic: '{}'",
            banned
        );
    }
}

#[test]
fn e15_01_i18n_fr_en() {
    let p = page();
    assert!(p.contains("id=\"langBar\""), "language bar");
    assert!(p.contains("data-l=\"fr\""), "FR button");
    assert!(p.contains("data-l=\"en\""), "EN button");
    assert!(p.contains("const EN="), "English dictionary");
    assert!(p.contains("lang=\"fr\""), "French is the base language");
    assert!(p.contains("LANG==='en'"), "English switch");
}

#[test]
fn e15_01_no_foreign_hosts() {
    const ALLOWED: [&str; 3] = [
        "cdn.tailwindcss.com",
        "fonts.googleapis.com",
        "fonts.gstatic.com",
    ];
    let p = page();
    let mut rest = p.as_str();
    while let Some(i) = rest.find("http") {
        let tail = &rest[i..];
        let url = if let Some(u) = tail.strip_prefix("https://") {
            Some(u)
        } else {
            tail.strip_prefix("http://")
        };
        if let Some(u) = url {
            let host: String = u
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-')
                .collect();
            assert!(ALLOWED.contains(&host.as_str()), "foreign host '{}'", host);
        }
        rest = &rest[i + 4..];
    }
    // No fetch() at all, or only to a relative URL.
    let mut rest = p.as_str();
    while let Some(i) = rest.find("fetch(") {
        let arg = &rest[i + 6..];
        assert!(
            !arg.trim_start_matches(['\'', '"', '`']).starts_with("http"),
            "fetch() to another host"
        );
        rest = arg;
    }
}
