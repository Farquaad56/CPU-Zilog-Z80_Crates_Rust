#!/usr/bin/env python
"""task.py - Z80 crate task pipeline (run by the USER or by Claude, NEVER by Hermes).

    python docs/tools/task.py next               # next task to work on (strict order, D-020)
    python docs/tools/task.py status             # progress per epic
    python docs/tools/task.py list [--epic E03]  # all tasks with state
    python docs/tools/task.py show <ID>          # print the task file
    python docs/tools/task.py start <ID>         # todo -> in_progress
    python docs/tools/task.py scope <ID>         # only check that changed files are inside the task scope
    python docs/tools/task.py check <ID>         # scope + named tests exist + hygiene + acceptance commands
    python docs/tools/task.py done <ID>          # checked -> done (prints the commit commands)
    python docs/tools/task.py approve <ID>       # GATE tasks: user validation -> approved
    python docs/tools/task.py reset <ID>         # back to todo

States: todo / in_progress / checked / done / approved (a GATE ends 'approved', other tasks end 'done').
Python 3.8+, standard library only.  Works on Windows, Linux, macOS.
"""
import argparse
import fnmatch
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DOCS = ROOT / "docs"
PROGRESS = ROOT / "PROGRESS.json"
INDEX = DOCS / "tasks" / "INDEX.tsv"
FINISHED = ("done", "approved")
ALWAYS_OK = ["docs/qa/inbox/**", "PROGRESS.json", "Cargo.lock"]


# ------------------------------------------------------------------ data

def load_index():
    rows = []
    lines = INDEX.read_text(encoding="utf-8").splitlines()
    head = lines[0].split("\t")
    for l in lines[1:]:
        if l.strip():
            rows.append(dict(zip(head, l.split("\t"))))
    return rows


def load_progress():
    if PROGRESS.exists():
        return json.loads(PROGRESS.read_text(encoding="utf-8"))
    return {"version": 1, "tasks": {}}


def save_progress(p):
    PROGRESS.write_text(json.dumps(p, indent=1, sort_keys=True) + "\n", encoding="utf-8", newline="\n")


def state_of(p, tid):
    return p.get("tasks", {}).get(tid, "todo")


def norm_id(tid):
    """E02-26D -> E02-26d (prefix upper-case, optional sub-step letter lower-case)"""
    m = re.match(r"^(E\d\d-\d\d)([A-Za-z]?)$", tid.strip(), re.I)
    return (m.group(1).upper() + m.group(2).lower()) if m else tid.strip().upper()


def find(rows, tid):
    tid = norm_id(tid)
    for i, r in enumerate(rows):
        if r["id"] == tid:
            return i, r
    die("unknown task id %s" % tid)


def die(msg, code=2):
    print("ERROR: " + msg)
    sys.exit(code)


def is_deferred(r):
    return r["role"] == "none"


def previous_unfinished(rows, p, idx):
    """first task before idx (in order) that is not finished; deferred tasks never block"""
    for r in rows[:idx]:
        if is_deferred(r):
            continue
        if state_of(p, r["id"]) not in FINISHED:
            return r
    return None


# ------------------------------------------------------------------ task file parsing

def task_text(tid):
    f = DOCS / "tasks" / ("%s.md" % tid)
    if not f.exists():
        die("task file missing: %s (run python docs/tools/build_tasks.py)" % f)
    return f.read_text(encoding="utf-8")


def section(text, name):
    m = re.search(r"^## " + re.escape(name) + r"[^\n]*\n(.*?)(?=^## |\Z)", text, re.S | re.M)
    return m.group(1) if m else ""


def write_patterns(tid):
    pats = []
    for l in section(task_text(tid), "Write only in").splitlines():
        m = re.match(r"^- `([^`]+)`", l)
        if m and not m.group(1).startswith("("):
            pats.append(m.group(1))
    return pats


def acceptance_commands(tid):
    sec = section(task_text(tid), "Acceptance")
    m = re.search(r"```acceptance\n(.*?)```", sec, re.S)
    if not m:
        return []
    return [l.strip() for l in m.group(1).splitlines() if l.strip()]


def named_tests(tid):
    sec = section(task_text(tid), "Tests")
    names = []
    for l in sec.splitlines():
        m = re.match(r"^- `([a-z0-9_]+)` :", l)
        if m:
            names.append(m.group(1))
    return names


# ------------------------------------------------------------------ git helpers

def git(*args):
    try:
        r = subprocess.run(["git"] + list(args), cwd=str(ROOT), stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                           universal_newlines=True)
    except OSError:
        return None
    if r.returncode != 0:
        return None
    return r.stdout


def changed_files():
    """paths (posix, relative to root) modified or untracked, or None if not a git repo"""
    out = git("status", "--porcelain", "--untracked-files=all")
    if out is None:
        return None
    files = []
    for l in out.splitlines():
        if len(l) < 4:
            continue
        path = l[3:]
        if " -> " in path:
            path = path.split(" -> ")[1]
        path = path.strip().strip('"')
        files.append(path.replace("\\", "/"))
    return files


def match(path, pat):
    if pat.endswith("/**"):
        return path.startswith(pat[:-3] + "/") or path == pat[:-3]
    if "**" in pat:
        rx = re.escape(pat).replace(r"\*\*", ".*").replace(r"\*", "[^/]*")
        return re.fullmatch(rx, path) is not None
    return fnmatch.fnmatch(path, pat)


def check_scope(tid):
    files = changed_files()
    if files is None:
        print("scope: not a git repository -> skipped")
        return True
    pats = write_patterns(tid) + ALWAYS_OK
    bad = [f for f in files if not any(match(f, p) for p in pats)]
    if bad:
        print("SCOPE FAIL: files changed outside the task scope of %s:" % tid)
        for f in bad:
            print("  " + f)
        print("(each finished task must be committed before the next one starts; see 'done')")
        return False
    print("scope OK (%d changed file(s))" % len(files))
    return True


# ------------------------------------------------------------------ hygiene checks

def rust_files(paths):
    out = []
    for p in paths:
        fp = ROOT / p
        if fp.is_file() and fp.suffix == ".rs":
            out.append(fp)
    return out


def check_tests_exist(tid):
    names = named_tests(tid)
    if not names:
        return True
    pats = write_patterns(tid)
    haystack = ""
    for f in sorted((ROOT / "crates").rglob("*.rs")) if (ROOT / "crates").exists() else []:
        rel = f.relative_to(ROOT).as_posix()
        if any(match(rel, p) for p in pats):
            try:
                haystack += f.read_text(encoding="utf-8", errors="replace") + "\n"
            except OSError:
                pass
    missing = [n for n in names if not re.search(r"\bfn\s+" + re.escape(n) + r"\s*\(", haystack)]
    if missing:
        print("TESTS FAIL: %d named test(s) not found as `fn <name>(` in the task scope:" % len(missing))
        for n in missing:
            print("  " + n)
        return False
    print("tests present (%d named tests found)" % len(names))
    return True


def check_hygiene(tid):
    files = changed_files()
    if files is None:
        return True
    ok = True
    for f in rust_files(files):
        rel = f.relative_to(ROOT).as_posix()
        data = f.read_bytes()
        try:
            text = data.decode("ascii")
        except UnicodeDecodeError:
            print("HYGIENE FAIL: non-ASCII characters in %s (code and comments must be ASCII)" % rel)
            ok = False
            continue
        if rel.startswith("crates/puce-z80/src/"):
            for n, l in enumerate(text.splitlines(), 1):
                code = l.split("//")[0]
                if re.search(r"\b(f32|f64)\b", code):
                    print("HYGIENE FAIL: float type in puce-z80 (%s:%d) - D-009: integers only" % (rel, n))
                    ok = False
                if re.search(r"\bunsafe\b", code):
                    print("HYGIENE FAIL: unsafe in puce-z80 (%s:%d)" % (rel, n))
                    ok = False
                if re.search(r"\b(HashMap|HashSet|SystemTime|Instant)\b", code):
                    print("HYGIENE FAIL: non-deterministic type in puce-z80 (%s:%d)" % (rel, n))
                    ok = False
        for n, l in enumerate(text.splitlines(), 1):
            if "UNKNOWN - to confirm" in l:
                print("note: UNKNOWN left in %s:%d (a question must exist in docs/qa/)" % (rel, n))
            if re.search(r"\b(todo|unimplemented)!\s*\(", l) and "UNKNOWN" not in l:
                print("note: todo!/unimplemented! in %s:%d" % (rel, n))
    if ok:
        print("hygiene OK")
    return ok


def run_acceptance(tid):
    cmds = acceptance_commands(tid)
    if not cmds:
        print("acceptance: no command for this task")
        return True
    for c in cmds:
        print("$ " + c)
        sys.stdout.flush()
        r = subprocess.run(c, shell=True, cwd=str(ROOT))
        if r.returncode != 0:
            print("ACCEPTANCE FAIL (exit %d): %s" % (r.returncode, c))
            return False
    return True


# ------------------------------------------------------------------ commands

def cmd_next(a):
    rows, p = load_index(), load_progress()
    for i, r in enumerate(rows):
        if is_deferred(r):
            continue
        st = state_of(p, r["id"])
        if st in FINISHED:
            continue
        print("%s  %s   [%s]" % (r["id"], r["title"], st))
        if r["gate"] == "1":
            print("  GATE: validation humaine. Lis docs/tasks/%s.md, execute les commandes, puis:" % r["id"])
            print("  python docs/tools/task.py approve %s" % r["id"])
        else:
            print("  fichier: docs/tasks/%s.md   (kanban: tache assignee a Hermes)" % r["id"])
            print("  apres le travail de Hermes: python docs/tools/task.py check %s" % r["id"])
        return 0
    print("all non-deferred tasks are finished")
    return 0


def cmd_status(a):
    rows, p = load_index(), load_progress()
    epics = {}
    for r in rows:
        e = epics.setdefault(r["epic"], {"total": 0, "fin": 0, "deferred": 0})
        if is_deferred(r):
            e["deferred"] += 1
            continue
        e["total"] += 1
        if state_of(p, r["id"]) in FINISHED:
            e["fin"] += 1
    tf = tt = 0
    for k in sorted(epics):
        e = epics[k]
        tf += e["fin"]
        tt += e["total"]
        extra = ("  (+%d deferred)" % e["deferred"]) if e["deferred"] else ""
        print("%s  %3d / %3d done%s" % (k, e["fin"], e["total"], extra))
    print("TOTAL %d / %d" % (tf, tt))
    return 0


def cmd_list(a):
    rows, p = load_index(), load_progress()
    for r in rows:
        if a.epic and r["epic"] != a.epic.upper():
            continue
        flag = "G" if r["gate"] == "1" else ("-" if is_deferred(r) else " ")
        print("%s %-9s %s" % (flag, state_of(p, r["id"]), r["id"] + "  " + r["title"]))
    return 0


def cmd_show(a):
    rows = load_index()
    find(rows, a.id)
    print(task_text(norm_id(a.id)))
    return 0


def cmd_start(a):
    rows, p = load_index(), load_progress()
    i, r = find(rows, a.id)
    prev = previous_unfinished(rows, p, i)
    if prev and not a.force:
        die("%s is not finished yet (strict order D-020). Use --force to override." % prev["id"], 1)
    p["tasks"][r["id"]] = "in_progress"
    save_progress(p)
    print("%s -> in_progress" % r["id"])
    return 0


def cmd_scope(a):
    rows = load_index()
    find(rows, a.id)
    return 0 if check_scope(norm_id(a.id)) else 1


def cmd_check(a):
    rows, p = load_index(), load_progress()
    i, r = find(rows, a.id)
    tid = r["id"]
    if is_deferred(r):
        die("%s is deferred (outside v1)." % tid, 1)
    if r["gate"] == "1":
        print("%s is a GATE: no automatic check; run the commands listed in docs/tasks/%s.md then 'approve'." % (tid, tid))
        return 0
    prev = previous_unfinished(rows, p, i)
    if prev and not a.force:
        die("%s is not finished yet (strict order D-020). Use --force to override." % prev["id"], 1)
    if getattr(a, "no_scope", False):
        print("scope: skipped (--no-scope, catch-up mode: several tasks share the same uncommitted changes)")
        ok = True
    else:
        ok = check_scope(tid)
    ok = check_tests_exist(tid) and ok
    ok = check_hygiene(tid) and ok
    if ok:
        ok = run_acceptance(tid)
    if ok:
        p["tasks"][tid] = "checked"
        save_progress(p)
        print("CHECK OK %s" % tid)
        return 0
    print("CHECK FAILED %s" % tid)
    return 1


def cmd_done(a):
    rows, p = load_index(), load_progress()
    i, r = find(rows, a.id)
    tid = r["id"]
    if r["gate"] == "1":
        die("%s is a GATE: use 'approve' (user only)." % tid, 1)
    if state_of(p, tid) != "checked" and not a.force:
        die("%s is not 'checked' (run: python docs/tools/task.py check %s)" % (tid, tid), 1)
    p["tasks"][tid] = "done"
    save_progress(p)
    print("%s -> done" % tid)
    print("")
    print("Commit (no Co-Authored-By / Claude-Session lines, D-001):")
    print("  git add -A")
    print('  git commit -m "%s: %s"' % (tid, r["title"]))
    print("  git push")
    return 0


def cmd_approve(a):
    rows, p = load_index(), load_progress()
    i, r = find(rows, a.id)
    tid = r["id"]
    if r["gate"] != "1":
        die("%s is not a GATE." % tid, 1)
    prev = previous_unfinished(rows, p, i)
    if prev and not a.force:
        die("%s is not finished yet (strict order D-020)." % prev["id"], 1)
    p["tasks"][tid] = "approved"
    save_progress(p)
    print("%s -> approved" % tid)
    print("")
    print("  git add -A")
    print('  git commit -m "%s: %s"' % (tid, r["title"]))
    print("  git push")
    return 0


def cmd_reset(a):
    rows, p = load_index(), load_progress()
    i, r = find(rows, a.id)
    p["tasks"][r["id"]] = "todo"
    save_progress(p)
    print("%s -> todo" % r["id"])
    return 0


def main():
    ap = argparse.ArgumentParser(description="Z80 crate task pipeline")
    sub = ap.add_subparsers(dest="cmd")
    sub.required = True
    sub.add_parser("next").set_defaults(fn=cmd_next)
    sub.add_parser("status").set_defaults(fn=cmd_status)
    s = sub.add_parser("list"); s.add_argument("--epic"); s.set_defaults(fn=cmd_list)
    for name, fn in (("show", cmd_show), ("scope", cmd_scope), ("reset", cmd_reset)):
        s = sub.add_parser(name); s.add_argument("id"); s.set_defaults(fn=fn)
    for name, fn in (("start", cmd_start), ("check", cmd_check), ("done", cmd_done), ("approve", cmd_approve)):
        s = sub.add_parser(name); s.add_argument("id"); s.add_argument("--force", action="store_true")
        if name == "check":
            s.add_argument("--no-scope", action="store_true", help="skip the git scope check (batch catch-up only)")
        s.set_defaults(fn=fn)
    a = ap.parse_args()
    return a.fn(a)


if __name__ == "__main__":
    sys.exit(main())
