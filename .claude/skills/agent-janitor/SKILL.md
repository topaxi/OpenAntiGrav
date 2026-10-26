---
name: agent-janitor
description: Compact the always-loaded and often-loaded agent instructions (CLAUDE.md, skills, agent definitions, HANDOVER.md) without losing a rule. Use when oag-drive's on-load step finds last-run older than 14 days, or the user asks to shorten the agent instructions.
---

# Agent janitor

last-run: 2026-10-26

Run as a sonnet `oag-docs` lane (docs-only). Bump `last-run` above to the run date in the final commit. oag-drive reads it on load and queues this lane first when it is over 14 days old.

## Targets

1. `CLAUDE.md` (every agent loads it).
2. `.claude/skills/*/SKILL.md` and support files such as `oag-drive/member-rules.md`. Leave scripts (`strip_rs_comments.py`) alone.
3. `.claude/agents/*.md`: destination for role-specific text; tighten in the same style.
4. `HANDOVER.md`: keep the "Open threads" index and every heading other docs cite ("Traps that are live", "Working rules that were learned expensively", "Read this first"). `just check-handover` ceiling is 256 KiB.

Note `.claude/` is gitignored but tracked: `git add -u .claude`, and `git add -f` for a new file.

## Before editing

- Record `wc -lc` of every target to scratch (`before.txt`).
- Make a rule ledger per target: `grep -nEi 'never|must|always|do not|don.t|only|maintainer|20[0-9][0-9]-'`. After compacting, every ledger line is kept, moved (say where) or deduplicated (say into what). That list is the proof no rule was lost.
- Map readers: CLAUDE.md is read by all, including solo `/oag-handover` sessions; `member-rules.md` and agent files only by drive members. Do not move a rule only into a file its readers do not load.
- Grep inbound references before moving or renaming a heading: `rg -n 'CLAUDE\.md|HANDOVER\.md|member-rules' .claude docs scripts crates handover`.

## What to cut

- Text a capable agent already knows.
- History narration: "this used to...", dated pass logs, counts that go stale (test totals, absent-file lists). Git holds history.
- A claim that is false now. Verify against the source (file, script, `ls data/`, `which`) before keeping or fixing it, and list each correction in the report.
- Duplicates across files; keep the copy in the file whose readers need it.
- Long rows and paragraphs: shorten to the rule plus at most one line of why, plus the doc page or address that holds the evidence.

## Survive-list (each kept to one line)

- Hard rules (`never`, `must`, ban lists, ceilings, `_q` below 70 and no rename below 50, determinism, no links into `handover/`, no `Claude-Session:` line).
- Maintainer rulings and their date or "maintainer".
- A dated incident that justifies a rule: shrink to a clause ("a commit once deleted six examples").
- Commands, flags, paths and addresses an agent types.
- Confidence scores and the "chosen, not measured" label.
- Open RE questions with their next step.

## Where moved text goes

- Role-specific (RE naming detail, emulator recipes, perf tools): the matching `.claude/agents/<role>.md`, or `docs/reverse-engineering/emulator-recipes.md`.
- Measurements and tuning history: `docs/` pages that already hold measurements (e.g. `docs/architecture/workspace-layout.md`). `rg` first so it is not already there.
- Leave a one-line pointer where a section moved if agents still need to know it exists.

## Checks

- `just check-docs`, `just check-handover`; `just check-names` and `just check-status` only if a `docs/ghidra` page or `names.tsv` changed (never commit `status.md`).
- Link check for files `check-docs` does not cover: extract `](path)` targets from CLAUDE.md, `.claude/` and HANDOVER.md and `test -e` each.
- No new link from outside `handover/` and `HANDOVER.md` into `handover/`.
- No em or en dashes.
- If any `.rs` file changed (it should not): `just fmt-check` and `just lint`.

## Commits and report

One commit per target file or group, `--no-gpg-sign` if pinentry blocks, `Co-Authored-By:` trailer, `git diff --cached --stat` read first.

Write `data/scratch/<lane>/report.md`: a before/after table (lines and bytes per file), what moved where, stale claims corrected, what was unsure and kept. Return its path plus at most 15 lines and every commit hash.
