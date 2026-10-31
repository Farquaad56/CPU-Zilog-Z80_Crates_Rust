#!/usr/bin/env python3
"""Insert one validation card ("<id>-V", assignee `validateur`) after each coding card of the board.

Chain after the change:  X (coder + same-card review by `relecteur`)  ->  X-V (validateur)  ->  next task.
GATE tasks keep no assignee (human) and get no -V card.  Idempotent: the key is z80-<id>-V.

  python docs/tools/kanban_validators.py --dry-run --from E03-01
  python docs/tools/kanban_validators.py --from E03-01 --to E03-15
"""
import argparse, json, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WS = "dir:" + str(ROOT).replace("\\", "/")

def hermes(board, *args):
    p = subprocess.run(["hermes", "kanban", "--board", board, *args], capture_output=True, text=True, encoding="utf-8")
    return p.returncode, (p.stdout or "") + (p.stderr or "")

def body(tid, title):
    return f"""# {tid}-V - Valider et commiter {tid}
Role: validateur. Tu ne codes pas. Une seule commande, a la racine du depot:

```
powershell -NoProfile -ExecutionPolicy Bypass -File tools\\validate.ps1 {tid}
```

- Sortie `VALIDATED {tid} COMMIT <hash>`: appelle `kanban_complete(summary="{tid} valide, commit <hash>")` et arrete-toi.
- Sortie `VALIDATION FAILED`: ne corrige RIEN dans le code. Ajoute un `kanban_comment` sur la carte de {tid}
  avec les 10 dernieres lignes utiles de la sortie, puis `kanban_block(kind="needs_input", reason="validation {tid} echouee")`.
- Interdit: git push, `task.py approve`, modifier un fichier, relancer en boucle (une tentative suffit).
Tache validee: {title}
"""

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--board", default="z80"); ap.add_argument("--tenant", default="z80")
    ap.add_argument("--assignee", default="validateur")
    ap.add_argument("--model", default="qwen3.8-27b-turbo-fable-cold-fusion-735-882-heretic-uncensored-neo-coder-max-mtp",
                    help="model pinned on every X and X-V card (relecteur/validateur default to Ollama, which may be off)")
    ap.add_argument("--provider", default="lmstudio")
    ap.add_argument("--from", dest="frm"); ap.add_argument("--to")
    ap.add_argument("--dry-run", action="store_true")
    a = ap.parse_args()
    tasks = json.loads((ROOT / "docs/kanban/tasks.json").read_text(encoding="utf-8"))["tasks"]
    idmap = json.loads((ROOT / "docs/kanban/id_map.json").read_text(encoding="utf-8"))
    ids = [t["id"] for t in tasks]
    lo = ids.index(a.frm) if a.frm else 0
    hi = ids.index(a.to) + 1 if a.to else len(ids)
    for t in tasks[lo:hi]:
        tid = t["id"]
        if t.get("gate") or tid not in idmap:
            continue
        card = idmap[tid]
        rc, out = hermes(a.board, "show", card, "--json")
        if rc != 0:
            print("skip %s: cannot read card %s" % (tid, card)); continue
        info = json.loads(out[out.index("{"):])
        status = info["task"]["status"]
        children = info.get("children", [])
        if a.dry_run:
            print("[dry] %s (%s) children=%s -> create %s-V, relink children" % (tid, status, children, tid)); continue
        rc, out = hermes(a.board, "create", "%s-V Valider et commiter %s" % (tid, tid), "--body", body(tid, t.get("title", "")),
                         "--assignee", a.assignee, "--parent", card, "--tenant", a.tenant, "--workspace", WS,
                         "--priority", str(t.get("priority", 5)), "--idempotency-key", "z80-%s-V" % tid,
                         "--max-runtime", "30m", "--max-retries", "1",
                         "--model", a.model, "--provider", a.provider, "--json")
        if rc != 0:
            print("FAIL create %s-V: %s" % (tid, out.strip()[:200])); sys.exit(1)
        vid = json.loads(out[out.index("{"):])["id"]
        if status in ("todo", "ready"):
            hermes(a.board, "set-model", card, a.model, "--provider", a.provider)
        for c in children:
            if c == vid:
                continue
            hermes(a.board, "unlink", card, c); hermes(a.board, "link", vid, c)
        print("%s -> %s-V %s (children relinked: %d)" % (card, tid, vid, len([c for c in children if c != vid])))

if __name__ == "__main__":
    main()
