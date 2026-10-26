---
name: oag-handover
description: Pick one open thread from handover/*.md at random and start working it, following this project's RE/implementation workflow. Use when the user runs /oag-handover or asks to "pick up a handover task", "work on something from HANDOVER", or "grab an open thread".
---

# OAG handover picker

Start one unit of work from the open-thread backlog. Stop and report the moment
progress needs something only the user or maintainer can supply; never guess
past a blocker.

## 0. No file mutation before `EnterWorktree`

Steps 1-4 are read-only: no `Edit`, `Write`, `rm`/`mv`/`sed -i`, not even a
one-line fix, not even in the main checkout. A "dirty" main checkout may be a
second live session's in-flight work (a real session edited it as "leftovers"
and collided). If `git status` shows uncommitted changes there, investigate
read-only (`git diff`, `git log`, `Read`) and check for a live session:

```sh
ls -la .claude/worktrees/          # threads already claimed by a worktree
for p in $(pgrep -f '/opt/claude-code/bin/claude'); do
  echo "$p: $(readlink /proc/$p/cwd 2>/dev/null)"
done                                # any other PID with cwd == main checkout?
```

Another PID with `cwd` in the main checkout means a live session works there:
revert any edit you made there, leave the rest alone, and obey step 5 yourself.
Otherwise the state is a past session's leftovers: still do not edit it from
the main checkout; enter a worktree first, or pick a different thread.

## 1. Orient

Read-only, in order:

- `HANDOVER.md`'s **Read this first** (disc-image and `.gitignore` traps, gate status).
- `git status`.
- `/bin/ls -la data/images/` to see whether disc-backed work is exercisable (`rg`/`fd` return nothing under `data/`; see CLAUDE.md "Data and images").

## 2. Pick a thread at random

List the real files, since a file can lag or lead `HANDOVER.md`'s index by a commit:

```sh
fd . handover -e md | shuf -n1
```

If the pick is not in `HANDOVER.md`'s **Open threads**, note the mismatch in your final report (not a blocker).

## 3. Scope one task

Read the thread in full (`## Open`, `## Next Steps`). If Next Steps has several items, commit to the first actionable one, or whichever most reduces one `## Open` bullet. Then grep `HANDOVER.md`'s **Traps that are live** and **Working rules that were learned expensively** for the thread's subject (title keywords, function names, paths).

## 4. Tell the user before starting

One short message: the thread, the next step, and what will be true once it is done. This is where the user can redirect you cheaply.

## 5. Branch into a worktree before your first mutation

Call `EnterWorktree({ name: "handover-<thread-slug>" })` (the filename without `.md`, truncated under the 64-char cap). It branches off `origin/main` and switches the session in. Leave it for `ExitWorktree` at the user's request or the harness's prompt; do not remove it yourself.

## 6. Do the work

The repo's normal rules apply (CLAUDE.md):

- RE: observe, hypothesise, verify, document, implement; a claim gets a confidence score, no rename below 50, every name lands in `names.tsv` in the same change.
- Never author a stand-in for what the disc's data specifies: parse and play it, or draw nothing and say so.
- Use the Ghidra MCP tools when a project is open; `just view`/`play`/`wad`/`unpack` for asset and runtime checks.
- Non-trivial work gets a task list, checked off as it lands.

**Commit each landed, buildable increment on its own** (a doc page, a rename plus its `names.tsv` row, one passing test, one implementation slice) in the worktree, so an interrupted session's progress is reviewable.

## 7. If you get stuck, stop

Blocked means: a Ghidra project that is not open, a disc image not in `data/images/`, a decompile too uncertain to act on, or a call only the user can make (a design tradeoff, or something observable only by playing the real game; their play-testing is a valid oracle to ask for, but you cannot fabricate its answer). Stop at once: no below-confidence rename, no plausible stand-in, thread not marked resolved. Report the thread and step, what you tried and found, what blocks you and what would unblock it.

Leave the thread file as is if nothing publishable resulted; add findings to its `## Open`/`## Next Steps` if you narrowed the problem. Step 6's commits already cover what landed; do not invent a wrap-up commit.

## 8. If you finish clean

Update docs with evidence and confidence, and the thread's `## Open`/`## Next Steps` (or, if nothing is left, delete the file and its `HANDOVER.md` index line together). Commit that on its own. Run `just` before calling it done and confirm `git status` is clean. Report what now works in concrete terms, not "should work".

**Docs-only exception**: a diff of only docs, `handover/`, `HANDOVER.md` or doc comments runs the lighter check (CLAUDE.md "Gates by change"), not the full gate. Run `just` in full the moment any `.rs` logic changes. It applies to the diff you made, not the thread's topic.

**A pure-RE step usually is not "nothing left"**: it unblocks an implementation task this session should not also attempt (bundling them skips verify-then-document). Write a **new** `handover/*.md` thread for the implementation, with the RE findings as its evidence, its own `## Open`/`## Next Steps` and a `HANDOVER.md` index line, then close out the RE thread (trim its Next Steps, or delete it if nothing else is open). Implement in the same session only when the remaining work is trivial and in scope for the step you committed to in step 3.
