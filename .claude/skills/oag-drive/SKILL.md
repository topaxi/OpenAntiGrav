---
name: oag-drive
description: Drive a team of subagents through the handover backlog - pre-draw threads onto disjoint lanes, merge and gate their work into main, reap worktrees, and keep the slots full. Use when the user runs /oag-drive or asks to "drive the team", "run the members", "keep the slots saturated", or "orchestrate the backlog".
---

# OAG drive: orchestrate a team over the handover backlog

You are the coordinator. You do not do the work; you draw the threads, brief the
members, verify what comes back, merge it, and keep the slots full. Members do
the work in isolated worktrees.

## On load, before anything else

1. **Tell the user the slot model and the model costs**, in a short table:

   | model | slots |
   | --- | --- |
   | haiku | 0.5 |
   | sonnet | 1 |
   | opus | 2 |

   Explain in one line: the budget caps *concurrent* members, an opus member
   costs two, so a budget of 3 is either three sonnets or one opus and one
   sonnet. You are excluded from the budget. Haiku is only for purely
   mechanical `oag-docs` sweeps (see "The roster").

2. **Ask how many slots to drive** with `AskUserQuestion`. Offer 2, 3 and 4, and
   say what each means in practice (2 is easy to supervise; 3 is the usual
   working figure; 4 is the most that has been driven at once). Do not guess;
   wait for the answer.

3. **Set up the recurring check.** Invoke the `loop` skill with an interval of
   one hour and a prompt that re-enters this one, so the loop survives you
   stopping. It used to be 30 minutes; the user raised it to an hour because iterations have become slower and longer, and a tick that
   finds nothing finished is pure overhead - a member's own report wakes the
   lead immediately regardless of the tick, so nothing waits on it:

   > `1h Check subagent slot occupation and status. Run ListAgents and total the
   > running cost (haiku 0.5 slot, sonnet 1, opus 2, budget N). For each finished member:
   > verify its diff, merge into main, run the right gate, then remove its
   > worktree and branch. Only spawn a replacement when usage is under N, then
   > fill to N - never replace one-for-one. Keep lanes disjoint by Ghidra binary
   > first and crate second, pre-drawing threads rather than letting members pick
   > randomly. Report slot usage, what merged, and what is still running.`

   Substitute the real budget for N.

4. **Then start**: check the state, draw threads, and fill to budget.

## The loop, every cycle

1. `ListAgents`. Total the running cost. **Peer sessions are not yours** - only
   subagents you spawned count against the budget.
2. `git log --oneline -1`, `git status --porcelain`, and per-branch
   `git log --oneline main..<branch> | wc -l` so you can see who has committed.
3. **For each finished member**: verify, merge, gate, reap (below).
4. **Only if usage is under budget**, fill to budget. Never replace
   one-for-one - a finished opus frees two slots and may become two sonnets.
5. Report: slot usage, what merged, what is still running, and anything that
   needs the user.

## Merging a member

- **Read the diff before merging.** `git diff --stat main...<branch>`, and read
  the actual change for anything load-bearing. A commit subject saying `docs(...)`
  with no doc file in the diff has happened twice; so has a member's findings
  living only in a scratch file outside git.
- **Merge with `--no-ff`** and the project's attribution trailers.
- **Run the right gate**, and run it *separately* from the merge - chaining a
  sub-second merge to a multi-minute gate makes the merge look hung and has
  caused the user to abort a healthy run:
  - any `.rs` logic changed -> full `just`, plus
    `OAG_REQUIRE_GAME_DATA=1 just test-data` when behaviour could move
  - docs/handover only -> `just check-docs`, `check-names` if `names.tsv` moved,
    `check-handover`, `check-captures` if captures moved
- **But do not re-run a gate the member already ran on identical content.**
  See "Who runs which gate" below - this is the single largest source of wasted
  CPU on this skill.
- **Reap**: `git worktree remove <path> -f -f` then `git branch -d <branch>`.
  Only after the work is merged.
- **Never reap a worktree whose agent may still resume.** A `completed`
  notification is not terminal - the same agent can be resumed and will find its
  Bash unusable. Merge first, then reap. **`ListAgents` cannot tell you this** -
  a finished, merged, fully-reaped lane still showed there as `pane` an hour
  later, indistinguishable from the live members beside it. The member's own
  report is the ending signal; `git worktree list` and
  `git log --oneline main..<branch>` are the ground truth.

## Who runs which gate

**The gate is the most expensive thing this skill does, and it was running
twice for every merge.** One session produced two merges and *four* full
`test-data` runs - each member gated its own tree, then the lead re-gated the
merge on content that had not changed. `test-data` is minutes of 16-core work
each time.

The split is asymmetric, and both halves matter:

- **The member always runs the full gate on its own tree**, before reporting.
  Do not move this to the lead. A member that does not gate reports untested
  work, and the breakage then surfaces *after* it is in `main` - which is
  strictly worse than finding it in a worktree that can be fixed without
  touching the mainline.
- **The lead re-gates after merge only when the merge actually combined
  behaviour.** Check `git log --oneline <branch-point>..main` first. If `main`
  has not moved since the member branched, the merge is content-identical to
  the tree the member already gated green and re-running proves nothing - say
  so in the report instead of burning the cycles. If `main` *has* moved and the
  two changes touch crates that interact, re-gate: that combination has been
  tested nowhere.

**Never merge into the main checkout while the lead's own gate is running
on it.** The gate compiles the working tree as it finds it, so a merge
landing mid-run produces a half-old, half-new tree. That once read
as six `E0425`/`E0599` compile errors against code that was fine. Either
hold the next merge until the gate finishes, or gate a detached worktree
(`git worktree add --detach ../OpenAntiGrav-worktrees/lead-gate <sha>`, then
`just link-data`) so merges can keep landing. A red lead gate that overlapped
a merge proves nothing; re-run it.

**Prefer `nice -n 10 ionice -c 3` for the lead's own gate.** The lead's run is
never on the critical path - members are the ones blocked on their own results.

## Serialise gates with `flock`

Four members each running `just` plus `OAG_REQUIRE_GAME_DATA=1 just test-data`
put a 16-core machine at load average 40 with six concurrent `nextest`
processes. Wrap every gate invocation in a shared lock:

```sh
flock -o "$HOME/.cache/oag/gate.lock" just
flock -o "$HOME/.cache/oag/gate.lock" env OAG_REQUIRE_GAME_DATA=1 just test-data
```

Four things about this that are easy to get wrong:

- **Always pass `-o`.** It makes `flock` close its own lock descriptor
  before running the command, so no child process inherits it; `flock`
  still holds the lock and releases it when the command exits. Without it,
  the `sccache` server a gate spawns keeps the lock after the gate is over -
  see below.

- **The lockfile must live outside every worktree** so all members contend on
  one inode. A path inside the repo gives each worktree its own lock and
  serialises nothing.
- **`flock` must wrap the outermost `just`.** `check:` is a dependency-list
  recipe, not a `#!/usr/bin/env bash` one, so a `flock` placed inside a recipe
  body does not hold across the recipe's other lines.
- **Put in every brief that waiting on the lock is not a hang.** A member
  blocked here looks exactly like the "ended a turn waiting for a background
  job" failure that has now hit sixteen members. Tell them the gate may sit
  for several minutes before it starts, and that this is correct.

**`sccache` inherits the lock and holds it after the gate exits.** Found
with two members' gates both "waiting for the lock" and no gate
running: `cargo` under a `flock`-wrapped `just` spawns the `sccache` server
with the lock's file descriptor open, and that server outlives the `just`
by its idle timeout (ten minutes by default), holding the lock the whole
time. `lsof "$HOME/.cache/oag/gate.lock"` shows it - a line for `sccache`
beside the waiting `flock`s. This is most of the "the gate may sit for
minutes" folklore above. **The fix is `flock -o`**, adopted after
it recurred: the old fix - start the server outside any lock before the
first gate - only lasts until that server idles out and the next gate
spawns a fresh one inside the lock, which is what happened. If a stale
server is still holding the lock anyway (a gate started without `-o`),
`sccache --stop-server` then `sccache --start-server` from the lead's own
shell - the waiting gate proceeds within seconds. Do not tell members to
kill it themselves; the lead owns the lock's health.

**Do not solve this with a dedicated gate-runner member** - it costs one of
four slots and needs cross-agent request/response plumbing invented for
something one line of `flock` already does. **Do not cap per-member `-j` as the
primary fix** either: `.config/nextest.toml` documents that `test-data` is
*tail-bound* - 2,323 tests finish in about the time the single slowest test
takes - so fewer threads would slow the throughput-bound compile phase without
touching the tail that actually sets the wall clock.

## Drawing threads

- **Pre-draw them. Never let members pick at random** - two members picking
  independently land in the same crate and collide at merge.
- **Lanes disjoint by function set first, crate second - a binary may be
  shared.** Relaxed by the user: two members may work the *same*
  Ghidra program at once, as long as their briefs name disjoint sets of
  functions/addresses so no two renames land on one function, and each
  appends its own dated section and its own `names.tsv` rows rather than
  editing the other's - the lead resolves the append-append conflict at
  merge. Every brief still repeats the rule from `docs/ghidra/workflow.md`'s
  "Bridge quirks" section: pass `program=` explicitly on every call (by path
  when names collide, e.g. `/psp-pulse-usa/BOOT.BIN`), never call
  `switch_program`. Skipping that once caused the incident in
  `HANDOVER.md` where one member's rename landed on another's active binary.
  Emulators are not shared either way: **each member spawns its own
  instance** (PPSSPP with its own `HOME`/`XDG_CONFIG_HOME`, debugger port and
  Xvfb display; RPCS3 with its own config copy), never attaching to one
  another member started. Say in every brief which ports/displays are taken.
- Sources for threads: `handover/*.md` (now sorted into `rendering/`, `gameplay/`,
  `frontend/`, `tooling/`, `audio/` - `fd`/`find` still reach them recursively),
  `HANDOVER.md`'s open-threads index, a
  red test on `main`, and whatever the previous member's report left open.
- **Prefer player-facing value.** The user's standing instruction. A pass that
  produces only another analysis document is a weak pass.

## The roster

Spawn every member as a named member (no `isolation` flag) of one of these
agent types, defined under `.claude/agents/`. Each carries its role's
guidance and tells the member to read
[`member-rules.md`](member-rules.md) first. That file holds the shared
boilerplate that used to be pasted into every brief: step zero, safety on a
shared machine, the `flock -o` gate, commits and the report format.

| agent | model / effort | use for |
| --- | --- | --- |
| `oag-re` | opus / high | recovering behaviour from an executable in Ghidra: decompile, name, verify live, evidence pages |
| `oag-wire` | sonnet / xhigh | ready-to-wire threads: the RE is done, wire it so a player sees or hears it |
| `oag-format` | sonnet / xhigh | a file family to decode: census, reader, every-file ground truth, format page |
| `oag-capture` | sonnet / high | a question only the running original can answer: PPSSPP, PCSX2 or RPCS3 measurement |
| `oag-docs` | sonnet / medium | docs and handover audits with no code logic change; spawn with `model: haiku` (0.5 slot) only for purely mechanical sweeps such as link fixes or index cleanup |
| `oag-compact` | sonnet / high | the standing comment compact lane: one crate per member, sequential, comments only; the checker `strip_rs_comments.py` sits beside this skill and the lead runs it before every compact merge |

Pick by the lane's **main output**, not by its subject. A lane that is mostly
wiring with one small decompile is `oag-wire`, and the member reads the RE
rules in `oag-re`'s file if it needs them. Overriding `model` at spawn time
works (a hard RE lane on sonnet to save a slot, say); say so in the report.
Sonnet at xhigh is the default workhorse. The maintainer found xhigh a
significant improvement for Sonnet 5.5.

## The brief

Members inherit none of your context. `member-rules.md` covers the shared
rules, so the brief carries only what is specific to the lane:

1. **The lane-specific values `member-rules.md` refers to**: the `<lane>` name
   (worktree, branch and scratch directory), the member's Xvfb display `:9N`,
   and any emulator ports. Say which displays and ports other members hold.
2. **The lane**, in priority order, and who owns the lanes they may not touch,
   by crate and by Ghidra function set.
3. **What is already established**, with a "do not re-derive this" marker. Cite
   the scratch reports and doc pages by path.
4. **The hard rules below that apply** and are not already in
   `member-rules.md`, such as the regression gate or golden hashes for a
   racing lane.
5. **The baseline failure count**, naming the exact expected failures, so a
   member can tell its own breakage from inherited red.

If a new failure mode shows up mid-drive, relay it to the running members at
once, then add it to `member-rules.md` so the next spawn inherits it. Don't
bury it in one brief.

**Do not widen a running lane by message.** A scope extension sent mid-lane
was dropped twice: Omega particles and Omega music each went to
a member already deep in its brief, and each report came back without a word
about Omega. Put the extra scope in a fresh member's brief instead, or in the
original brief if it is known at spawn time. Short corrections, such as "main
moved" or a new rule, do arrive and are acted on.

## Hard rules to put in briefs, as they apply

- **Never invent what the assets already author.** Parse it and play it, or draw
  nothing and say so. Anything genuinely chosen is labelled **chosen, not
  measured**, with NO confidence score - a score means measured off the original.
- **Committed golden hashes are never edited to make a test pass.** If behaviour
  legitimately moves, regenerate from the member's own tree in a **separate**
  commit that names the behaviour change.
- **The regression gate**: `race_ground_truth::a_lone_craft_gets_round_the_
  circuits_it_is_known_to_get_round` stays all-twelve-clean; quote it verbatim
  before and after. Check the current `01_Track` respawn index yourself and state
  it - it moves when AI tuning changes.
- **Report per-lap and end-of-run shield separately, always labelled.**
- **Judge it as a player would.** More than one frame, at the size a player sees.
  A countdown once passed every test while a player saw nothing on screen.
- **Below 70 confidence a Ghidra name gets `_q`; below 50, no rename at all.**
  Every name lands in `names.tsv` in the same change as its evidence page.
- **Do not pipe `just test-data` into `tail`** - it exits 100 on failure and a
  pipe masks it as 0.
- **`fd` and `rg` respect `.gitignore`** and return nothing under `data/` with
  exit code 0.
- **Write findings into `docs/` and `handover/`, not only scratch.** Two passes
  had to be sent back for this.

## Judging a report

- **A negative result reported honestly is a good outcome.** Several of this
  project's best passes killed a theory and invented nothing.
- **Be suspicious of a green claim you did not see.** Ask whether the member
  watched the gate finish. Three members were misled by builds that died on the
  errors above and read as flaky.
- **Check the diff against the claim.** A `docs(...)` subject with no docs in it
  means the findings are outside git.
- **A member that says "I could not determine X, here is its address"** has done
  the job. That is what made a later weapon buildable.
- **Verify visual claims yourself** with the Read tool on the screenshot. Do not
  take "a player would see it" on report.

## When the user reports something from play

Treat it as a reliable oracle - their observations have repeatedly beaten static
analysis on this project. Turn the observation into a *measurement* for a member,
give them the discriminating question, and tell them what would falsify it. Do
not have them tune to match a memory: one "concrete wave" report turned out to be
an accurate memory of a **different title**, and tinting ours would have been
wrong.
