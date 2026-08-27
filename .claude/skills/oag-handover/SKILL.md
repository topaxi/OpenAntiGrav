---
name: oag-handover
description: Pick one open thread from handover/*.md at random and start working it, following this project's RE/implementation workflow. Use when the user runs /oag-handover or asks to "pick up a handover task", "work on something from HANDOVER", or "grab an open thread".
---

# OAG handover picker

Autonomously start one unit of work from this repo's open-thread backlog. Stop and
report back the moment real progress needs something only the user or maintainer can
supply — do not guess past a blocker.

## 0. Hard rule: no file mutation before `EnterWorktree`

**Everything before step 5 is read-only.** No `Edit`, no `Write`, no `rm`/`mv`/`sed -i`,
no fixing a typo you happen to notice — not even in the main checkout, and not even if
it looks like a one-line fix. This bit a real session: uncommitted work was found
sitting in the main checkout, read (per step 1 below) as "a previous session's
leftovers", and edited directly — only to discover mid-edit that a *second, still
running* session was live in that same directory, not a past one, and the edits
collided with its in-flight changes. "The tree is dirty" alone doesn't tell you which
case you're in; only checking does.

If `git status` in step 1 shows uncommitted changes in the main checkout, investigate
read-only (`git diff`, `git log`, `Read`) — don't edit, fix, or revert anything yet —
and check whether the main checkout is actually claimed by a live session right now:

```sh
ls -la .claude/worktrees/          # threads already claimed by a dedicated worktree
for p in $(pgrep -f '/opt/claude-code/bin/claude'); do
  echo "$p: $(readlink /proc/$p/cwd 2>/dev/null)"
done                                # any PID besides this session's own with cwd == main checkout?
```

A PID other than this session's own with `cwd` equal to the main checkout means a live
session is working there directly, right now, outside any worktree — revert any edit
you already made there, leave the rest of that checkout untouched, and take it purely
as a reminder to obey step 5 yourself, not as something to fix on that session's behalf.
Otherwise, the uncommitted state really is a past session's leftovers — still don't
edit it from the main checkout; either pick it up properly by entering a worktree
first, or leave it alone and pick a different thread.

## 1. Orient before picking

Read, in this order — read-only, per the rule above:

- `HANDOVER.md`'s **Read this first** section — the disc-image/`.gitignore` traps and
  current gate status live there and are wrong to rediscover the hard way.
- `git status` — a previous session's work may already be sitting uncommitted, or a
  concurrent session may be live in the main checkout right now; see step 0.
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

## 4. Tell the user, briefly, before starting

Before touching a worktree or a file, tell the user: which thread got picked, which
next step, and the expected outcome — what will be true or land differently once this
step is done. This is the point where a fresh set of eyes could redirect you cheaply —
don't skip it and don't over-explain it.

## 5. Branch into a worktree — before your first file mutation, no exceptions

This is the gate step 0 exists to enforce: the *first* `Edit`/`Write`/mutating `Bash`
call of the session happens after this step, never before it. Call `EnterWorktree` to
isolate this session's work from the main checkout — that's what lets several handover
sessions run at once without colliding. Name it after the thread so it's identifiable
in `git worktree list`, e.g. the file stem trimmed to something short:

```
EnterWorktree({ name: "handover-<thread-slug>" })
```

(`<thread-slug>` = the picked filename without `.md`, truncated to keep the name
under the 64-char cap — e.g. `a-circuits-billboard-slots-are-a-9-entry.md` →
`handover-a-circuits-billboard-slots`.) This branches fresh off `origin/main` and
switches the session into it. Do the rest of the work there; leave it for
`ExitWorktree` at the user's request or the harness's own end-of-session prompt —
don't remove it yourself.

## 6. Do the work

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

Commit as you go, in the worktree — don't let one giant diff accumulate for step 8 to
squash at the end. The worktree exists precisely so this is safe: commit each landed,
buildable increment on its own (a doc page written, a rename plus its `names.tsv` row,
one passing test, one implementation slice) as soon as it's true, rather than batching
unrelated changes into one commit. Granular commits are what make a blocked or
interrupted session's partial progress reviewable — a maintainer picking the thread
back up sees exactly which increments landed and which didn't, instead of one opaque
blob or, worse, uncommitted work that a crash or context loss can still lose even
though the worktree itself persists. Small commit messages are fine; they don't need
the full PR-body treatment step 8's final state does.

## 7. If you get stuck, stop — don't push through on a guess

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
`## Open`/`## Next Steps` if you narrowed the problem even without finishing it. If
step 6's granular commits already cover everything real that landed, there's nothing
further to commit — don't invent a wrap-up commit just to have one; an uncommitted
scratch note about the blocker is fine to leave sitting in the worktree.

## 8. If you finish clean

Update docs with evidence and confidence per the RE workflow, update the thread
file's `## Open`/`## Next Steps` to reflect what's left (or, if nothing is left,
delete the file and its `HANDOVER.md` index line together — the project's own rule
for closing a thread). Commit that update on its own — it's one more granular step,
not a tail to fold into the last implementation commit. Run `just` before calling it
done — that's the gate every commit must pass — and confirm `git status` is clean
before reporting: nothing from this thread should be left sitting uncommitted in the
worktree. Report what now works in concrete terms, not "should work".

**A step that was pure RE — a name recovered, a format decoded, a hypothesis
confirmed, evidence documented — usually isn't "nothing left".** It unblocks an
implementation task that this session probably shouldn't also attempt in the same
sitting: the RE half and the implementation half are different kinds of work, and
bundling them back-to-back in one pass is how the "verify → document" step gets
skipped in practice. Default to writing a **new** `handover/*.md` thread for the
implementation, with the RE thread's findings as its evidence, its own `## Open`/
`## Next Steps`, and an index line in `HANDOVER.md`; then close out the original RE
thread per the paragraph above (trim its `## Next Steps` to reflect it's done, or
delete it if truly nothing else is open on it). Only implement in the same session
when the remaining work is trivial and obviously in-scope for the next step you
already committed to in step 3 — not as the default path.
