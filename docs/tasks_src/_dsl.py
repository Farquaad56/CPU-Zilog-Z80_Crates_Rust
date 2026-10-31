"""Task description DSL for Z80 crate (read by docs/tools/build_tasks.py).

Every task is declared with T(...). Order of declaration = order of INDEX.tsv.
Edit THIS directory to change tasks, then run: python docs/tools/build_tasks.py
"""

TASKS = []

COMMON_SCOPE_NOTE = "docs/qa/inbox/**"   # always writable (questions)


def T(id, title, *, deps=(), gate=False, kind="code", size=100, context="",
      embed=(), tsv=(), reads=(), spec="", write=(), tests=(), accept=None,
      crate="puce-z80", notes="", assignee=None, deferred=False, user_cmds=()):
    """Declare one task.

    id        'E02-03'
    kind      code | doc | data | tooling | gate
    size      expected lines of code (50-150 for code tasks, D-014)
    embed     list of 'specs/FILE.md#Heading prefix' (section copied verbatim into the task)
    tsv       list of ('specs/data/FILE.tsv', ['69','6D']) rows copied into the task
    reads     extra plain references shown in 'Lire d'abord'
    write     closed list of writable path patterns (fnmatch, '**' = any depth)
    tests     list of (suffix, text) ; real test name = <id lower, '-'->'_'>_<suffix>
    accept    list of shell commands (default: cargo test -p <crate> <prefix>)
    gate      human validation required (no assignee in Kanban)
    user_cmds commands the human runs for a GATE (shown in the task)
    deferred  task is NOT part of v1 (created without assignee, flagged)
    """
    TASKS.append(dict(id=id, title=title, deps=list(deps), gate=gate, kind=kind,
                      size=size, context=context.strip(), embed=list(embed),
                      tsv=list(tsv), reads=list(reads), spec=spec.strip(),
                      write=list(write), tests=list(tests), accept=accept,
                      crate=crate, notes=notes.strip(), assignee=assignee,
                      deferred=deferred, user_cmds=list(user_cmds)))
