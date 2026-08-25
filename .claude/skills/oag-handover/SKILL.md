---
name: oag-handover
description: Pick one open thread from handover/*.md at random and start working it, following this project's RE/implementation workflow. Use when the user runs /oag-handover or asks to "pick up a handover task", "work on something from HANDOVER", or "grab an open thread".
---

# OAG handover picker

Autonomously start one unit of work from this repo's open-thread backlog. Stop and
report back the moment real progress needs something only the user or maintainer can
supply — do not guess past a blocker.

## 1. Orient before picking

Read, in this order:

- `HANDOVER.md`'s **Read this first** section — the disc-image/`.gitignore` traps and
  current gate status live there and are wrong to rediscover the hard way.
- `git status` — a previous session's work may already be sitting uncommitted.
- `/bin/ls -la data/images/` — confirms whether disc-backed work is even exercisable
  this session (see CLAUDE.md's sandbox note; `rg`/`fd` under `data/` silently return
  nothing regardless, so don't use them to check).

## 2. Pick a thread at random

List the real files, not just the index (a file can lag or lead `HANDOVER.md`'s Open
threads list by a commit):

```sh
fd . handover -e md
```

Pick one **with actual randomness**, don't eyeball a favorite:

```sh
fd . handover -e md | shuf -n1
```

If it's not also listed under `HANDOVER.md`'s **Open threads** section, note that
mismatch in your final report — it's worth a human noticing either way, but isn't
itself a blocker to starting.

## 3. Read the thread and scope one task

Read the picked file in full. Every thread ends with `## Open` (the unresolved
questions) and `## Next Steps` (the concrete actions). If `## Next Steps` has more
than one item, don't tackle the whole list — commit to the first actionable one, or
whichever most directly reduces one `## Open` bullet.

Then grep `HANDOVER.md`'s **Traps that are live** and **Working rules that were
learned expensively** sections for the thread's subject (title keywords, function
names, file paths) — these sections exist specifically because someone already lost
time on adjacent ground.

Tell the user, briefly, before starting: which thread, which next step, one line on
the plan. This is the point where a fresh set of eyes could redirect you cheaply —
don't skip it and don't over-explain it.

## 4. Branch into a worktree

Before touching any files, call `EnterWorktree` to isolate this session's work from
the main checkout — that's what lets several handover sessions run at once without
colliding. Name it after the thread so it's identifiable in `git worktree list`, e.g.
the file stem trimmed to something short:

```
EnterWorktree({ name: "handover-<thread-slug>" })
```

(`<thread-slug>` = the picked filename without `.md`, truncated to keep the name
under the 64-char cap — e.g. `a-circuits-billboard-slots-are-a-9-entry.md` →
`handover-a-circuits-billboard-slots`.) This branches fresh off `origin/main` and
switches the session into it. Do the rest of the work there; leave it for
`ExitWorktree` at the user's request or the harness's own end-of-session prompt —
don't remove it yourself.

## 5. Do the work

Follow this repo's normal rules — nothing about this skill changes them:

- RE work follows observe → hypothesize → verify → document → implement
  ([methodology](../../../docs/reverse-engineering/methodology.md)); a claim gets a
  confidence score, a rename below 50 doesn't happen at all, and every recovered name
  lands in `names.tsv` in the same change.
- Never author a stand-in for something the disc's own data already specifies —
  parse it and play it, or draw/implement nothing and say so.
- Use the Ghidra MCP tools directly when a Ghidra project is open; use `just view` /
  `just play` / `just wad` / `just unpack` for asset and runtime checks.
- Non-trivial work gets a task list, checked off as it lands, per the global
  instructions.

## 6. If you get stuck, stop — don't push through on a guess

Blocked means: the next step needs a Ghidra project that isn't open, a disc image
that isn't in `data/images/`, a decompile that doesn't resolve to a confident enough
read to act on, or a call only the user/maintainer can make (a design tradeoff, or
something only observable by playing the real game — see the user's own play-testing
is a valid oracle to ask for, but you can't fabricate its answer).

When that happens: stop immediately. Don't rename anything below-confidence, don't
invent a plausible-looking implementation to fill the gap, don't mark the thread
resolved. Report back:

- which thread and which next step you took
- exactly what you tried and what you found
- what's blocking, specifically, and what would unblock it (a data file, a decision,
  an observation from playing the game, etc.)

Leave the thread file as-is if nothing publishable resulted; add findings to its
`## Open`/`## Next Steps` if you narrowed the problem even without finishing it.

## 7. If you finish clean

Update docs with evidence and confidence per the RE workflow, update the thread
file's `## Open`/`## Next Steps` to reflect what's left (or, if nothing is left,
delete the file and its `HANDOVER.md` index line together — the project's own rule
for closing a thread). Run `just` before calling it done — that's the gate every
commit must pass. Report what now works in concrete terms, not "should work".
