#!/usr/bin/env python
"""qa.py - question / answer tool (docs/02_QA_PROTOCOL.md). Run by the USER or by Claude, never by Hermes.

    python docs/tools/qa.py list
    python docs/tools/qa.py show E02-03-Q1
    python docs/tools/qa.py answer E02-03-Q1 --choice A --text "Short precise answer" [--durable] [--author NAME]

'answer' (1) appends an entry to docs/qa/QA_LOG.md (append-only), (2) with --durable adds a numbered decision
D-nnn to docs/qa/DECISIONS.md, (3) moves the question to docs/qa/answered/, (4) prints the Kanban commands.
Python 3.8+, standard library only.
"""
import argparse
import datetime
import json
import re
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
QA = ROOT / "docs" / "qa"
INBOX = QA / "inbox"
ANSWERED = QA / "answered"
LOG = QA / "QA_LOG.md"
DEC = QA / "DECISIONS.md"
IDMAP = ROOT / "docs" / "kanban" / "id_map.json"


def die(m):
    print("ERROR: " + m)
    sys.exit(2)


def qfile(qid):
    f = INBOX / ("%s.md" % qid)
    if not f.exists():
        die("no open question %s (see: python docs/tools/qa.py list)" % qid)
    return f


def field(text, name):
    m = re.search(r"^" + name + r":\s*(.*)$", text, re.M)
    return m.group(1).strip() if m else ""


def cmd_list(a):
    files = sorted(INBOX.glob("*-Q*.md")) if INBOX.exists() else []
    if not files:
        print("no open question")
        return 0
    for f in files:
        t = f.read_text(encoding="utf-8", errors="replace")
        print("%s  [%s]  %s" % (f.stem, field(t, "TYPE"), field(t, "QUESTION")))
    return 0


def cmd_show(a):
    print(qfile(a.id).read_text(encoding="utf-8", errors="replace"))
    return 0


def next_decision_number():
    nums = [int(x) for x in re.findall(r"\bD-(\d{3})\b", DEC.read_text(encoding="utf-8"))]
    return (max(nums) if nums else 0) + 1


def cmd_answer(a):
    f = qfile(a.id)
    text = f.read_text(encoding="utf-8", errors="replace")
    task = field(text, "TASK") or a.id.rsplit("-Q", 1)[0]
    question = field(text, "QUESTION")
    today = datetime.date.today().isoformat()
    dec = "aucune"
    if a.durable:
        dec = "D-%03d" % next_decision_number()
        with open(DEC, "a", encoding="utf-8", newline="\n") as d:
            d.write("- %s | %s | %s | %s (%s: %s)\n" % (dec, today, a.author, a.text, a.id, question))
    choice = (" (choix %s)" % a.choice) if a.choice else ""
    with open(LOG, "a", encoding="utf-8", newline="\n") as lg:
        lg.write("\n## %s %s | tache %s | auteur %s\nQ: %s\nA:%s %s\nDecision: %s\n" % (
            today, a.id, task, a.author, question, choice, a.text, dec))
    ANSWERED.mkdir(parents=True, exist_ok=True)
    shutil.move(str(f), str(ANSWERED / f.name))
    print("answered %s -> QA_LOG.md%s ; question moved to docs/qa/answered/" % (a.id, (" + DECISIONS.md " + dec) if a.durable else ""))
    kid = "<kanban_id>"
    if IDMAP.exists():
        kid = json.loads(IDMAP.read_text(encoding="utf-8")).get(task, kid)
    msg = "ANSWER %s%s: %s. Logged in docs/qa/QA_LOG.md" % (a.id, (" (choice %s)" % a.choice) if a.choice else "", a.text.replace('"', "'"))
    print("")
    print("If the answer changes a spec, update docs/specs/*.md and the task files, then: python docs/tools/build_tasks.py")
    print("Kanban commands:")
    print('  hermes kanban comment %s "%s"' % (kid, msg))
    print("  hermes kanban unblock %s" % kid)
    return 0


def main():
    ap = argparse.ArgumentParser(description="Z80 crate Q&A tool")
    sub = ap.add_subparsers(dest="cmd")
    sub.required = True
    sub.add_parser("list").set_defaults(fn=cmd_list)
    s = sub.add_parser("show"); s.add_argument("id"); s.set_defaults(fn=cmd_show)
    s = sub.add_parser("answer"); s.add_argument("id"); s.add_argument("--choice", default="")
    s.add_argument("--text", required=True); s.add_argument("--durable", action="store_true")
    s.add_argument("--author", default="Claude"); s.set_defaults(fn=cmd_answer)
    a = ap.parse_args()
    return a.fn(a)


if __name__ == "__main__":
    sys.exit(main())
