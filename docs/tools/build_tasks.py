#!/usr/bin/env python
"""build_tasks.py - regenerate docs/tasks/*.md, INDEX.tsv, ORDER.md, docs/checks/*.sh,
docs/kanban/tasks.json from docs/tasks_src/*.py.  (Claude maintenance tool, not for Hermes.)

Usage:
    python docs/tools/build_tasks.py            # validate + generate
    python docs/tools/build_tasks.py --check    # validate only, write nothing
    python docs/tools/build_tasks.py --stats    # print statistics

Python 3.8+, standard library only.  Output is ASCII-safe (utf-8 files).
"""
import argparse
import importlib
import json
import os
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DOCS = ROOT / "docs"
SRC = DOCS / "tasks_src"
SPECS = DOCS / "specs"

MAX_RUNTIME = "2h"
MAX_RETRIES = 1
TENANT = "z80"

FOOTER_RULES = """
---
## Regles permanentes (rappel)
- Code, identifiants, commentaires: anglais, ASCII uniquement.
- Valeur inconnue ou non specifiee: commentaire exact `UNKNOWN - to confirm` + question bloquante (voir ci-dessous). Ne jamais deviner.
- Ecrire UNIQUEMENT dans les chemins de "Write only in" (plus `docs/qa/inbox/**` pour les questions).
- `puce-z80`: entiers uniquement (pas de f32/f64), deterministe, `#![forbid(unsafe_code)]`, pas de dependance externe.
- Ne lance JAMAIS `python docs/tools/task.py check|done|approve`, ni `git commit|push`. Lance seulement les commandes de "Acceptance".
- Reponds uniquement avec ce qui est demande, sans recapitulatif.

## Question bloquante (protocole docs/02_QA_PROTOCOL.md)
Ecris `docs/qa/inbox/{id}-Q1.md` (format: TYPE, TASK, BLOCKED_ON, QUESTION, WHAT_I_KNOW, OPTIONS A/B, DEFAULT_I_WOULD_PICK, CODE_STATE),
puis `kanban_block(reason="QUESTION {id}-Q1: <question>. See docs/qa/inbox/{id}-Q1.md")` et arrete-toi.
Avant de poser une question: relis cette tache, les specs citees, `docs/qa/FAQ_Z80.md` et `docs/qa/DECISIONS.md`.
"""

ALWAYS_WRITABLE = ["docs/qa/inbox/**"]


# ---------------------------------------------------------------- helpers

def prefix_of(tid):
    return tid.lower().replace("-", "_") + "_"


def epic_of(tid):
    return tid.split("-")[0]


def read_text(path):
    return Path(path).read_text(encoding="utf-8")


def iter_headings(lines):
    """yield (index, level, text) for markdown headings outside code fences"""
    fence = False
    for i, l in enumerate(lines):
        if l.lstrip().startswith("```"):
            fence = not fence
            continue
        if fence:
            continue
        m = re.match(r"^(#+)\s+(.*?)\s*$", l)
        if m:
            yield i, len(m.group(1)), m.group(2)


def resolve_doc(relpath):
    """paths are relative to docs/ ; 'specs/' prefix optional"""
    for cand in (DOCS / relpath, SPECS / relpath):
        if cand.exists():
            return cand
    raise FileNotFoundError(relpath)


def rel_to_root(relpath):
    return resolve_doc(relpath).relative_to(ROOT).as_posix()


def extract_section(relpath, head_prefix):
    text = read_text(resolve_doc(relpath))
    lines = text.splitlines()
    if head_prefix == "*":
        return "\n".join(lines).rstrip()
    heads = list(iter_headings(lines))
    for k, (i, lvl, t) in enumerate(heads):
        if t.startswith(head_prefix):
            end = len(lines)
            for (j, lvl2, _t2) in heads[k + 1:]:
                if lvl2 <= lvl:
                    end = j
                    break
            return "\n".join(lines[i:end]).rstrip()
    raise KeyError("heading %r not found in %s" % (head_prefix, relpath))


def tsv_rows(relpath, keys):
    rows = read_text(resolve_doc(relpath)).splitlines()
    head = rows[0]
    out = [head]
    index = {r.split("\t")[0].upper(): r for r in rows[1:]}
    for k in keys:
        if k.upper() not in index:
            raise KeyError("opcode %s not in %s" % (k, relpath))
        out.append(index[k.upper()])
    return "\n".join(out)


# ---------------------------------------------------------------- load

def load_tasks():
    sys.path.insert(0, str(SRC))
    import _dsl
    _dsl.TASKS.clear()
    for f in sorted(SRC.glob("e[0-9][0-9]*.py")):
        name = f.stem
        if name in sys.modules:
            del sys.modules[name]
        importlib.import_module(name)
    # D-020: strictly sequential execution, ONE task at a time. Declaration order = execution order.
    prev = None
    for t in _dsl.TASKS:
        t["declared_deps"] = list(t["deps"])
        if prev is not None and prev not in t["deps"]:
            t["deps"] = t["deps"] + [prev]
        prev = t["id"]
    return _dsl.TASKS


# ---------------------------------------------------------------- validate

def validate(tasks):
    errors, warns = [], []
    ids = [t["id"] for t in tasks]
    if len(set(ids)) != len(ids):
        dup = sorted({i for i in ids if ids.count(i) > 1})
        errors.append("duplicate ids: %s" % dup)
    pos = {t["id"]: n for n, t in enumerate(tasks)}
    for t in tasks:
        if not re.match(r"^E\d\d-\d\d[a-z]?$", t["id"]):
            errors.append("%s: bad id format (E00-00)" % t["id"])
        for d in t["deps"]:
            if d not in pos:
                errors.append("%s: unknown dep %s" % (t["id"], d))
            elif pos[d] >= pos[t["id"]]:
                errors.append("%s: dep %s declared after the task" % (t["id"], d))
        if t["kind"] in ("code", "ui"):
            if t["kind"] == "code" and not t["tests"]:
                errors.append("%s: code task without tests" % t["id"])
            if not t["write"]:
                errors.append("%s: code task without write scope" % t["id"])
            if t["kind"] == "ui" and not t["accept"]:
                errors.append("%s: ui task needs explicit acceptance" % t["id"])
            if t["size"] > 150:
                warns.append("%s: size %d > 150 (D-014), split it" % (t["id"], t["size"]))
            if t["size"] < 20:
                warns.append("%s: size %d is tiny, merge it?" % (t["id"], t["size"]))
        if not t["context"]:
            errors.append("%s: empty context" % t["id"])
        if t["gate"] and not (t["user_cmds"] or t["accept"]):
            warns.append("%s: gate without commands" % t["id"])
        names = [s for s, _ in t["tests"]]
        if len(set(names)) != len(names):
            errors.append("%s: duplicate test suffix" % t["id"])
        for ref in t["embed"]:
            try:
                f, h = ref.split("#", 1)
                extract_section(f, h)
            except Exception as e:  # noqa
                errors.append("%s: embed %r: %s" % (t["id"], ref, e))
        for f, keys in t["tsv"]:
            try:
                tsv_rows(f, keys)
            except Exception as e:  # noqa
                errors.append("%s: tsv %r: %s" % (t["id"], f, e))
    # transitive closure (ancestors) for write-overlap check
    anc = {}
    for t in tasks:
        s = set()
        for d in t["deps"]:
            if d in anc:
                s.add(d)
                s |= anc[d]
        anc[t["id"]] = s
    owners = {}
    for t in tasks:
        for w in t["write"]:
            if "*" in w:
                continue
            owners.setdefault(w, []).append(t["id"])
    for path, lst in owners.items():
        for a in range(len(lst)):
            for b in range(a + 1, len(lst)):
                x, y = lst[a], lst[b]
                if x not in anc[y] and y not in anc[x]:
                    errors.append("write overlap without ordering: %s and %s both write %s" % (x, y, path))
    return errors, warns, anc


# ---------------------------------------------------------------- render

def render_task_md(t, by_id):
    tid = t["id"]
    pre = prefix_of(tid)
    out = []
    out.append("# %s - %s" % (tid, t["title"]))
    deps = ", ".join(t["deps"]) if t["deps"] else "aucune"
    flags = []
    if t["gate"]:
        flags.append("GATE (validation humaine)")
    if t["deferred"]:
        flags.append("DIFFEREE (hors v1)")
    out.append("Epopee: %s | Type: %s | Dependances: %s | Taille estimee: ~%d lignes%s" % (
        epic_of(tid), t["kind"], deps, t["size"], (" | " + ", ".join(flags)) if flags else ""))
    out.append("")
    out.append("## Contexte")
    out.append(t["context"])
    out.append("")
    out.append("## Lire d'abord")
    reads = []
    for ref in t["embed"]:
        f, h = ref.split("#", 1)
        reads.append("- `%s` section \"%s\" (recopiee ci-dessous)" % (rel_to_root(f), h if h != "*" else "fichier entier"))
    for f, keys in t["tsv"]:
        reads.append("- `%s` lignes %s (recopiees ci-dessous)" % (rel_to_root(f), ", ".join(keys)))
    for r in t["reads"]:
        reads.append("- %s" % r)
    reads.append("- `docs/qa/DECISIONS.md` et `docs/qa/FAQ_Z80.md` (a consulter avant toute question)")
    out.extend(reads)
    out.append("")
    out.append("## Write only in")
    for w in t["write"]:
        out.append("- `%s`" % w)
    if not t["write"]:
        out.append("- (aucun fichier de code: voir Specification)")
    out.append("- `docs/qa/inbox/**` (questions uniquement)")
    out.append("")
    out.append("## Specification")
    if t["spec"]:
        out.append(t["spec"])
        out.append("")
    if t["embed"] or t["tsv"]:
        out.append("### Extraits des specs (recopies automatiquement; la spec source fait foi, ne pas les modifier)")
        for ref in t["embed"]:
            f, h = ref.split("#", 1)
            out.append("")
            out.append("<!-- begin %s#%s -->" % (rel_to_root(f), h))
            out.append(extract_section(f, h))
            out.append("<!-- end -->")
        for f, keys in t["tsv"]:
            out.append("")
            out.append("Lignes de `%s` (colonnes separees par tabulation):" % rel_to_root(f))
            out.append("```")
            out.append(tsv_rows(f, keys))
            out.append("```")
    if t["notes"]:
        out.append("")
        out.append("### Notes")
        out.append(t["notes"])
    out.append("")
    out.append("## Tests")
    if t["tests"]:
        out.append("Tous les tests sont prefixes par `%s`.\n" % pre)
        for suf, text in t["tests"]:
            out.append("- `%s%s` : %s" % (pre, suf, text))
    else:
        out.append("- (pas de test unitaire: voir Acceptance)")
    out.append("")
    out.append("## Acceptance")
    cmds = t["accept"]
    if cmds is None:
        cmds = ["cargo test -p %s %s" % (t["crate"], pre)] if t["kind"] == "code" else []
    if t["gate"] and t["user_cmds"]:
        out.append("Validation humaine. Commandes a executer par l'utilisateur:")
        out.append("```")
        for c in t["user_cmds"]:
            out.append(c)
        out.append("```")
        out.append("Puis: `python docs/tools/task.py approve %s`" % tid)
    if cmds:
        out.append("```acceptance")
        for c in cmds:
            out.append(c)
        out.append("```")
    out.append("")
    out.append("## Si tu bloques")
    out.append("Voir `docs/02_QA_PROTOCOL.md`: ecris `docs/qa/inbox/%s-Q1.md` et bloque la tache (`kanban_block`). Ne devine pas." % tid)
    out.append("")
    return "\n".join(out)


def render_check_sh(t):
    tid = t["id"]
    pre = prefix_of(tid)
    cmds = t["accept"]
    if cmds is None:
        cmds = ["cargo test -p %s %s" % (t["crate"], pre)] if t["kind"] == "code" else []
    lines = ["#!/usr/bin/env bash",
             "# Generated by build_tasks.py - do not edit. Equivalent native check: python docs/tools/task.py check %s" % tid,
             "set -e",
             'cd "$(dirname "$0")/../.."',
             "python docs/tools/task.py scope %s" % tid]
    lines += cmds
    lines.append('echo "CHECK OK %s"' % tid)
    return "\n".join(lines) + "\n"


def body_for_kanban(t, md):
    return md + "\n" + FOOTER_RULES.replace("{id}", t["id"])


# ---------------------------------------------------------------- levels

def levels(tasks, anc):
    lvl = {}
    for t in tasks:
        lvl[t["id"]] = 1 + max([lvl[d] for d in t["deps"]], default=0)
    return lvl


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--stats", action="store_true")
    a = ap.parse_args()

    tasks = load_tasks()
    errors, warns, anc = validate(tasks)
    for w in warns:
        print("WARN  " + w)
    for e in errors:
        print("ERROR " + e)
    if errors:
        print("%d error(s): nothing written" % len(errors))
        return 1

    lvl = levels(tasks, anc)
    by_epic = {}
    for t in tasks:
        by_epic.setdefault(epic_of(t["id"]), []).append(t)

    if a.stats or a.check:
        total_lines = sum(t["size"] for t in tasks if t["kind"] == "code")
        print("tasks: %d  (code lines estimated: %d)" % (len(tasks), total_lines))
        for e in sorted(by_epic):
            ts = by_epic[e]
            print("  %s: %3d tasks, %5d lines, %d gates" % (
                e, len(ts), sum(x["size"] for x in ts if x["kind"] == "code"), sum(1 for x in ts if x["gate"])))
        width = {}
        for t in tasks:
            width[lvl[t["id"]]] = width.get(lvl[t["id"]], 0) + 1
        print("strictly sequential: %d tasks in one chain (D-020)" % len(tasks))
        if a.check:
            print("OK (validation only)")
            return 0
        return 0

    (DOCS / "tasks").mkdir(exist_ok=True)
    (DOCS / "checks").mkdir(exist_ok=True)
    (DOCS / "kanban").mkdir(exist_ok=True)
    keep = set()
    by_id = {t["id"]: t for t in tasks}
    kanban = []
    index_rows = ["id\ttitle\tepic\tdeps\tgate\tkind\tsize\trole"]
    for t in tasks:
        md = render_task_md(t, by_id)
        (DOCS / "tasks" / ("%s.md" % t["id"])).write_text(md, encoding="utf-8", newline="\n")
        keep.add("%s.md" % t["id"])
        sh = DOCS / "checks" / ("%s.sh" % t["id"])
        sh.write_text(render_check_sh(t), encoding="utf-8", newline="\n")
        keep.add("%s.sh" % t["id"])
        role = "user" if t["gate"] else ("none" if t["deferred"] else ("claude" if t["kind"] in ("doc",) and t["assignee"] == "claude" else "hermes"))
        if t["assignee"]:
            role = t["assignee"]
        index_rows.append("\t".join([t["id"], t["title"], epic_of(t["id"]), ",".join(t["deps"]),
                                     "1" if t["gate"] else "0", t["kind"], str(t["size"]), role]))
        epic_no = int(epic_of(t["id"])[1:])
        prev_id = tasks[tasks.index(t) - 1]["id"] if tasks.index(t) > 0 else None
        kanban.append({
            "id": t["id"],
            "title": "%s %s" % (t["id"], t["title"]),
            "body": body_for_kanban(t, md),
            "parents": [prev_id] if prev_id else [],
            "gate": bool(t["gate"] or t["deferred"] or role in ("user", "claude", "none")),
            "role": role,
            "priority": max(1, 10 - epic_no),
            "idempotency_key": "z80-%s" % t["id"],
            "max_runtime": MAX_RUNTIME,
            "max_retries": MAX_RETRIES,
            "tenant": TENANT,
        })
    # remove stale generated files
    for d, ext in ((DOCS / "tasks", ".md"), (DOCS / "checks", ".sh")):
        for f in d.glob("E[0-9][0-9]-[0-9][0-9]*" + ext):
            if f.name not in keep:
                f.unlink()
    (DOCS / "tasks" / "INDEX.tsv").write_text("\n".join(index_rows) + "\n", encoding="utf-8", newline="\n")
    (DOCS / "kanban" / "tasks.json").write_text(
        json.dumps({"version": 1, "board": "z80", "tenant": TENANT, "tasks": kanban}, indent=1, ensure_ascii=True) + "\n",
        encoding="utf-8", newline="\n")

    # ORDER.md (D-020: one task at a time, strict order)
    old = DOCS / "tasks" / "WAVES.md"
    if old.exists():
        old.unlink()
    w = ["# ORDER - ordre d'execution strict (genere)", "",
         "D-020: UNE seule tache a la fois, dans cet ordre. Une tache n'est prete que quand la precedente est `done` (et approuvee si GATE).", ""]
    cur = None
    for n, t in enumerate(tasks, 1):
        e = epic_of(t["id"])
        if e != cur:
            w.append("")
            w.append("## %s" % e)
            cur = e
        w.append("%3d. %s %s%s" % (n, t["id"], t["title"], "  [GATE]" if t["gate"] else ""))
    (DOCS / "tasks" / "ORDER.md").write_text("\n".join(w) + "\n", encoding="utf-8", newline="\n")

    # PROGRESS.json (never reset existing state)
    pj = ROOT / "PROGRESS.json"
    prog = {}
    if pj.exists():
        prog = json.loads(read_text(pj))
    prog.setdefault("version", 1)
    st = prog.setdefault("tasks", {})
    for t in tasks:
        st.setdefault(t["id"], "todo")
    for k in list(st):
        if k not in by_id:
            del st[k]
    pj.write_text(json.dumps(prog, indent=1, sort_keys=True) + "\n", encoding="utf-8", newline="\n")

    print("generated %d tasks, INDEX.tsv, ORDER.md, kanban/tasks.json, PROGRESS.json" % len(tasks))
    return 0


if __name__ == "__main__":
    sys.exit(main())
