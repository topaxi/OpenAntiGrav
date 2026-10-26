---
name: oag-drive
description: Drive a team of subagents through the handover backlog - pre-draw threads onto disjoint lanes, merge and gate their work into main, reap worktrees, and keep the slots full. Use when the user runs /oag-drive or asks to "drive the team", "run the members", "keep the slots saturated", or "orchestrate the backlog".
---

# OAG drive: orchestrate a team over the handover backlog

You are the coordinator. You do not do the work: you draw threads, brief
members, verify what comes back, merge it, and keep the slots full. Members work
in isolated worktrees.

## On load, before anything else

1. **Tell the user the slot model**:

   | model | slots |
   | --- | --- |
   | haiku, read-only `oag-scout` | 0 |
   | haiku, anything else | 0.5 |
   | sonnet | 1 |
   | opus | 2 |

   The budget caps *concurrent* members (3 is either three sonnets or one opus
   and one sonnet). You are excluded, and so is a read-only haiku `oag-scout`
   (maintainer, 2026-10-26: no builds, writes only a handover file), which may
   be spawned on top of the budget. Another haiku member (a mechanical
   `oag-docs` sweep) costs 0.5.

2. **Ask how many slots to drive** with `AskUserQuestion`: 2 (easy to
   supervise), 3 (usual), 4 (most ever driven). Wait for the answer.

3. **Agent janitor.** Read `last-run:` in `.claude/skills/agent-janitor/SKILL.md`.
   If it is more than 14 days old, queue an `agent-janitor` lane (sonnet
   `oag-docs`) first and hold any freed slot empty until it merges, so later
   members start on the compacted instruction files.

4. **Set up the recurring check.** Invoke the `loop` skill at one hour
   (iterations are slow, and a member's own report wakes the lead regardless of
   the tick) with a prompt that re-enters this skill:

   > `1h Check subagent slot occupation and status. Run ListAgents and total the
   > running cost (read-only haiku oag-scout 0, other haiku 0.5, sonnet 1, opus 2, budget N). For each finished member:
   > verify its diff, merge into main, run the right gate, then remove its
   > worktree and branch. Only spawn a replacement when usage is under N, then
   > fill to N - never replace one-for-one. Keep lanes disjoint by Ghidra binary
   > first and crate second, pre-drawing threads rather than letting members pick
   > randomly. Report slot usage, what merged, and what is still running.`

   Substitute the real budget for N.

5. **Then start**: check state, draw threads, fill to budget.

## The loop, every cycle

1. `ListAgents`; total the running cost. **Peer sessions are not yours**; only subagents you spawned count.
2. `git log --oneline -1`, `git status --porcelain`, and per branch `git log --oneline main..<branch> | wc -l`.
3. For each finished member: verify, merge, gate, reap (below).
4. Only if under budget, fill to budget (a finished opus frees two slots and may become two sonnets).
5. Report: slot usage, what merged, what is running, anything needing the user.

## Merging a member

- **Read the diff before merging**: `git diff --stat main...<branch>` and the load-bearing change. A `docs(...)` subject with no doc file in the diff has happened twice, as have findings living only in scratch.
- **Merge with `--no-ff`** and the project's attribution trailers.
- **After every merge, before the next**: `cargo check --workspace --all-targets` and `cargo nextest run --workspace` (no disc data, about ten seconds once built). Members gate their branch as it is and do not merge `main` first (maintainer, 2026-10-12); when a merge conflicts, goes red or looks suspicious, send that member back to merge `main`, resolve and re-gate. Two branches that each gated green can conflict semantically (a removed `SpeedClass::from_name` vs a new call to it), and only the merged test run catches it.
- **Run the gate separately from the merge** (chaining a sub-second merge to a multi-minute gate looks hung; a user aborted a healthy run):
  - any `.rs` logic changed: full `just` plus `OAG_REQUIRE_GAME_DATA=1 just test-data`, **once per merge batch**, and at once after a merge whose `scripts/affected.py` selection is `FULL`;
  - docs/handover only: `just check-docs`, `check-names` if `names.tsv` moved, `check-handover`, `check-captures` if captures moved.
- **Reap**: `git worktree remove <path> -f -f`, `git branch -d <branch>`, then `TaskStop` the member (its name as `task_id`) and check `ListAgents` (a member left idle stays in the roster; maintainer, 2026-10-12). Only after the work is merged.
- **Never reap a worktree whose agent may still resume**: a `completed` notification is not terminal. `ListAgents` cannot tell you (a reaped lane still showed as `pane` an hour later); the member's own report is the ending signal, and `git worktree list` plus `git log --oneline main..<branch>` are ground truth.

## Who runs which gate

The gate is the most expensive thing this skill does.

- **A member runs the affected gate on its own tree** before reporting:

  ```sh
  flock -o "$HOME/.cache/oag/gate.lock" just gate-affected
  flock -o "$HOME/.cache/oag/gate.lock" env OAG_REQUIRE_GAME_DATA=1 just test-data-affected
  ```

  `scripts/affected.py` selects the packages changed since the merge base with `main` plus their dependents and prints what and why. Every `check-*`, `fmt-check` and `lint` still run in full. A change to `oag-core`, a simulation crate (`oag-physics`, `oag-gameplay`, `oag-ai`, `oag-race`, `oag-formats`), `Cargo.toml`, `Cargo.lock`, the toolchain, the `justfile`, `.config/` or any `build.rs` selects the full suite by itself; a docs-only change selects no tests, and the member still gates.
- **The lead runs the full gate once per merge batch**, on merged `main`, in the `lead-gate` worktree: `flock -o ... just` and `flock -o ... env OAG_REQUIRE_GAME_DATA=1 just test-data`. It is the only place the whole suite runs with disc images (each member was green only on its selection), so it is not skippable. At once, without waiting for a batch, after merging a branch whose selection was `FULL`.
- **A red batch gate is bisected by branch**, not by commit: re-gate `main` at the merge before the newest member's; the failure names its lane. Send it back as a fix lane.
- **CI** (`.github/workflows/ci.yml`) runs `fmt`, `clippy` and the non-disc `cargo nextest run --workspace`; `nightly.yml` builds binaries and runs no tests. **No CI job can run `test-data`** (no images); the lead's batch gate is the only full run.

**Never merge into the main checkout while the lead's gate is running on it**: the gate compiles the tree as it finds it, and a mid-run merge gave six bogus `E0425`/`E0599` errors. Hold the merge, or gate a detached worktree (`git worktree add --detach ../OpenAntiGrav-worktrees/lead-gate <sha>`, then `just link-data`). A red gate that overlapped a merge proves nothing; re-run.

**The `lead-gate` worktree holds one gate at a time.** Before `git checkout` there, check no gate runs in it (`pgrep -af 'flock -o .*gate.lock'`); two chains once produced `E0599` against code that compiled and wrote the same logs.

Prefer `nice -n 10 ionice -c 3` for the lead's own gate: it is never on the critical path.

## Serialise gates with `flock`

Four members gating at once reached load average 40. Wrap every gate (affected or full) in one shared lock:

```sh
flock -o "$HOME/.cache/oag/gate.lock" just gate-affected
flock -o "$HOME/.cache/oag/gate.lock" env OAG_REQUIRE_GAME_DATA=1 just test-data-affected
flock -o "$HOME/.cache/oag/gate.lock" just                     # lead, per batch
flock -o "$HOME/.cache/oag/gate.lock" env OAG_REQUIRE_GAME_DATA=1 just test-data
```

- **Always pass `-o`**: it closes flock's own descriptor before the command, so no child inherits the lock. Without it the `sccache` server a gate spawns outlives the gate by its idle timeout (ten minutes) holding the lock (`lsof "$HOME/.cache/oag/gate.lock"` shows it). If a stale server still holds it (a gate started without `-o`), run `sccache --stop-server` then `sccache --start-server` from the lead's shell; the waiting gate proceeds in seconds. Members never kill it; the lead owns the lock's health.
- **The lockfile lives outside every worktree** so all members contend on one inode.
- **`flock` wraps the outermost `just`** (`check:` is a dependency-list recipe, so a `flock` inside a recipe body does not hold across it).
- **Put in every brief that waiting on the lock is not a hang** (sixteen members have ended a turn "waiting for a background job"): the gate may sit several minutes before it starts.

Do not add a dedicated gate-runner member (costs a slot, needs plumbing `flock` already gives) and do not cap per-member `-j`: `.config/nextest.toml` documents `test-data` as *tail-bound*, so fewer threads slow the compile phase without touching the tail.

## Drawing threads

- **Pre-draw them; never let members pick at random** (two independent picks collide at merge).
- **Lanes are disjoint by function set first, crate second; a Ghidra binary may be shared.** Two members may work the same program if their briefs name disjoint function/address sets, each appends its own dated section and `names.tsv` rows (the lead resolves append-append conflicts at merge). Every brief repeats `docs/ghidra/workflow.md`'s "Bridge quirks": pass `program=` explicitly on every call (by path when names collide, e.g. `/psp-pulse-usa/BOOT.BIN`), never `switch_program` (one rename once landed on another member's active binary). **Each member spawns its own emulator** (own `HOME`/`XDG_CONFIG_HOME`, debugger port and Xvfb for PPSSPP; own config copy for RPCS3); say in every brief which ports and displays are taken.
- Sources: `handover/**/*.md`, `HANDOVER.md`'s open-threads index, a red test on `main`, what the previous member's report left open.
- **Prefer player-facing value** (standing instruction); a pass that produces only another analysis document is weak.

## The roster

Spawn every member as a named member (no `isolation` flag) of an agent type under `.claude/agents/`. Each tells the member to read [`member-rules.md`](member-rules.md) first (step zero, shared-machine safety, the `flock -o` gate, commits, report).

| agent | model / effort | use for |
| --- | --- | --- |
| `oag-re` | opus / high | recovering behaviour from an executable in Ghidra: decompile, name, verify live, evidence pages |
| `oag-wire` | sonnet / xhigh | ready-to-wire threads: the RE is done, wire it so a player sees or hears it |
| `oag-format` | sonnet / xhigh | a file family to decode: census, reader, every-file ground truth, format page |
| `oag-capture` | sonnet / high | a question only the running original can answer: PPSSPP, PCSX2 or RPCS3 measurement |
| `oag-docs` | sonnet / medium | docs and handover audits with no code logic change; `model: haiku` (0.5 slot) only for mechanical sweeps (link fixes, index cleanup) |
| `oag-scout` | haiku / high | read-only opportunity sweeps (render performance, allocations): a ranked, measurable list in a handover thread, advisor-checked; no code, no builds, **0 slots** |
| `oag-compact` | sonnet / high | the standing comment compact lane: one crate per member, sequential, comments only; the lead runs `strip_rs_comments.py` (beside this skill) before every compact merge |
| `oag-perf` | opus / high | a measured speed, memory or size win with before and after |

Pick by the lane's **main output**, not its subject (mostly wiring with one small decompile is `oag-wire`). Overriding `model` at spawn works (say so in the report). Sonnet at xhigh is the default workhorse (maintainer: a significant improvement for Sonnet 5.5). AI-behaviour and physics lanes run on opus.

## The brief

Members inherit none of your context. `member-rules.md` carries the shared rules, so the brief carries only:

0. **One claim and its acceptance evidence** (maintainer, 2026-10-21). The claim is one sentence that is true or false at the end ("Tech de Ra's hexagon panels render within 10 grey of the clean original in box (x,y,w,h)"). The evidence is the exact pair id and region whose metric must move, or the exact test going red to green; a whole-frame delta of 0.3-1.0 grey is inside re-capture noise and is not acceptance. Say whether it is a **measure** lane (ends with a number, a confidence, a doc page and a thread marked ready to wire; "one value unread, here is its address and read recipe" is a success) or a **wire** lane (starts from a ready-to-wire thread, ends with a test and a pair). Never both. If you cannot write the claim, spawn a scout lane.
1. **The lane-specific values**: `<lane>` name, the member's Xvfb `:9N`, emulator ports, and which displays and ports other members hold.
2. **The lane**, in priority order, and who owns the lanes it may not touch, by crate and Ghidra function set.
3. **What is already established**, marked "do not re-derive", citing scratch reports and doc pages by path.
4. **The hard rules below that apply** and are not in `member-rules.md`.
5. **The baseline failure count**, naming the exact expected failures.
6. "Report every commit, including any made after your report."

If a new failure mode shows up mid-drive, relay it to running members at once, then add it to `member-rules.md` so the next spawn inherits it.

**Do not widen a running lane by message**: scope extensions were dropped twice (Omega particles, Omega music). Put extra scope in a fresh member's brief, or the original brief. Short corrections ("main moved", a new rule) do arrive.

**Extra scope often lands after the "done" report**: a member reads messages at its next tool round, so late commits followed three scope messages on 2026-10-14. After sending scope mid-lane, do not read its absence from the report as dropped and do not reap on that report: check `git log --oneline main..<branch>` again, wait for the follow-up report, then merge and stop the member. `git branch -d` refusing ("not fully merged") means late commits.

## Hard rules to put in briefs, as they apply

The rest is in `member-rules.md`. Brief-only items:

- **The regression gate**: `race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round` stays all-twelve-clean; quote it verbatim before and after. State the current `01_Track` respawn index yourself (it moves with AI tuning).
- **Report per-lap and end-of-run shield separately, always labelled.**
- **Committed golden hashes are never edited to make a test pass**; regenerate in a separate commit naming the behaviour change.
- **Below 70 confidence a Ghidra name gets `_q`; below 50, no rename.** Every name lands in `names.tsv` with its evidence page.
- **Judge it as a player would** (a countdown once passed every test while a player saw nothing).

## Judging a report

- **A negative result reported honestly is a good outcome**; so is "I could not determine X, here is its address".
- **Be suspicious of a green claim you did not see**: ask whether the member watched the gate finish (three were misled by builds that died on disk or LLVM errors and read as flaky).
- **Check the diff against the claim.**
- **Verify visual claims yourself** with Read on the screenshot.

## When the user reports something from play

Treat it as a reliable oracle (their observations repeatedly beat static analysis). Turn it into a *measurement* for a member with the discriminating question and what would falsify it. Do not have members tune to match a memory (one "concrete wave" report was an accurate memory of a **different title**).
