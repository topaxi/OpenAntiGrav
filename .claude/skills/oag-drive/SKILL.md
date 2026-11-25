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
   | haiku, anything else (a mechanical `oag-docs` sweep) | 0.5 |
   | sonnet | 1 |
   | opus | 2 |
   | fable (only with the maintainer's OK) | 3 |

   The budget caps *concurrent* members (3 is three sonnets or one opus and one sonnet). You are excluded, and so is a read-only haiku `oag-scout` (maintainer, 2026-10-26: no builds, writes only a handover file), which may be spawned on top of the budget.

2. **Ask how many slots to drive** with `AskUserQuestion`: 2 (easy to supervise), 3 (usual), 4 (most ever driven). Wait for the answer.

3. **Agent janitor.** Read `last-run:` in `.claude/skills/agent-janitor/SKILL.md`.
   If it is more than 14 days old, queue an `agent-janitor` lane (sonnet
   `oag-docs`) first and hold any freed slot empty until it merges, so later
   members start on the compacted instruction files.

4. **Set up the recurring check.** A member's report wakes the lead regardless of the tick, but **a member whose session dies sends nothing**: the tick is the only thing that notices, so it must check liveness. Run the `loop` skill in dynamic mode (no interval) and call `ScheduleWakeup` (1800 s, the same prompt) at the end of **every** lead turn (maintainer, 2026-10-26: a check only after 30 minutes with no member message). Unverified: whether a new `ScheduleWakeup` replaces the pending one or stacks; if wake-ups stack, fall back to the fixed `30m` cron. A tick may skip merging and filling when the lead acted in the last 30 minutes, **never the liveness check**. The prompt:

   > `Check subagent slot occupation and status. Run ListAgents and total the running cost (read-only haiku oag-scout 0, other haiku 0.5, sonnet 1, opus 2, budget N). Run member-liveness.sh on every running member; replace a DEAD one at once on its worktree. For each finished member: verify its diff, merge into main, run the right gate, then remove its worktree and branch. Only spawn a replacement when usage is under N, then fill to N, never one-for-one. Keep lanes disjoint by Ghidra binary first and crate second, pre-drawing threads. Report slot usage, what merged, and what is still running.`

   Substitute the real budget for N.

5. **Then start**: check state, draw threads, fill to budget.

## The loop, every cycle

1. `ListAgents`; total the running cost. **Peer sessions are not yours**; only subagents you spawned count, plus any mechanical-writer helper a member announced (`member-rules.md`, step zero 4: sonnet 1, haiku 0.5).
2. `git log --oneline -1`, `git status --porcelain`, and per branch `git log --oneline main..<branch> | wc -l`.
3. **Liveness, every member, every tick**: `.claude/skills/oag-drive/member-liveness.sh <member>[=<worktree-dir>] ...`. `ListAgents` keeps showing a dead member as `pane`, and uncommitted edits or a quiet branch say nothing (a dead member sat 2.5 h): the transcript's age is the signal.
   - `DEAD` (transcript idle 30+ min, no process in its worktree): `TaskStop` it at once and spawn a successor on the **same worktree** (keep the uncommitted edits), briefed with what its transcript and scratch show it finished (`jq` over the transcript the script names: last text blocks and last commands).
   - `SUSPECT` (idle 90+ min with processes, possibly a dead session's orphans): `SendMessage` it; no reply by the next tick counts as `DEAD`.
   - **While any member runs, the fallback tick is 30 minutes, not an hour.**
4. For each finished member: verify, merge, gate, reap (below).
5. Only if under budget, fill to budget.
6. Report: slot usage, what merged, what is running, each member's liveness line, anything needing the user.

## Merging a member

- **Read the diff before merging**: `git diff --stat main...<branch>` and the load-bearing change (a `docs(...)` subject with no doc file in the diff has happened twice, as have findings living only in scratch).
- **Merge with `just merge-lane <branch> "<message>"`**: `--no-ff`, the message carrying the project's attribution trailers, and `status.md` regenerated and committed if it moved. A lane on another base (`slipstream`) merges from that checkout the same way.
- **After every merge, before the next**: `cargo check --workspace --all-targets` and `cargo nextest run --workspace` (no disc data, about ten seconds once built). Members gate their branch as it is and do not merge `main` first (maintainer, 2026-10-12); when a merge conflicts, goes red or looks suspicious, send that member back to merge `main`, resolve and re-gate. Two branches that each gated green can conflict semantically, and only the merged test run catches it.
- **Run the gate separately from the merge**:
  - any `.rs` logic changed: full `just` plus `just lead-test-data` (sliced, see "Who runs which gate"), **once per merge batch**, and at once after a merge whose `scripts/affected.py` selection is `FULL` or carries its `core or simulation` notice;
  - docs/handover only: `just check-docs`, `check-names` if `names.tsv` moved, `check-handover`, `check-captures` if captures moved.
- **Reap**: `git worktree remove <path> -f -f`, `git branch -d <branch>`, then `TaskStop` the member (its name as `task_id`) and check `ListAgents` (a member left idle stays in the roster; maintainer, 2026-10-12). Only after the work is merged. Then `rm -rf` the lane's `data/scratch/<lane>/` (absolute path, quoted, guarded as in `member-rules.md` step zero 3) once its findings are in `docs/` or `handover/`. At the end of each merge batch run `just clean-scratch` (7 days, protects open worktrees and branches; scratch once reached 506 GiB with no sweep).
- **A member's report is the final text of its turn** (`member-rules.md`, "Report"). In the spawn prompt, say "reply with" the path and summary, never "send" or "message" it.
- **Never reap a worktree whose agent may still resume**: a `completed` notification is not terminal. `ListAgents` cannot tell you; the member's own report is the ending signal, and `git worktree list` plus `git log --oneline main..<branch>` are ground truth.

## Who runs which gate, and the lock

The gate is the most expensive thing this skill does.

- **A member runs the affected gate on its own tree** before reporting (`scripts/gate-run.sh gate-affected`, then `test-data-affected`). `scripts/affected.py` selects the tests that link what changed since the merge base with `main` (CLAUDE.md, "Gates by change"); every `check-*`, `fmt-check` and `lint` still run in full. The root `Cargo.toml`, the toolchain and `.cargo/` select the full suite; a change to `oag-core` or a simulation crate (`oag-physics`, `oag-gameplay`, `oag-ai`, `oag-race`, `oag-formats`) prints a `core or simulation` notice. What each rule could miss: `docs/architecture/affected-gate.md`.
- **The lead runs the full gate once per merge batch**, on merged `main` in the `lead-gate` worktree: `scripts/gate-run.sh check`, then `just lead-test-data` (the full `test-data` as 4 slices, each under its own lock hold; it takes the lock itself, never wrap it in `flock`; `-n M` changes the count, `--full` goes to every slice). It is the only place the whole suite runs with disc images, so it is not skippable. Run it at once after merging a branch whose selection was `FULL` or printed the `core or simulation` notice.
- **A batch is two or three merges, and the lead's gate yields to members** (maintainer, 2026-11-19). Before taking the lock, `pgrep -af 'flock -o .*gate.lock'`: if a member's gate is waiting or running, start yours after it. Gate at once only for the `FULL`/`core or simulation` case, or when the drive is about to end.
- **A red batch gate is bisected by branch**, not by commit: re-gate `main` at the merge before the newest member's; the failure names its lane, which goes back as a fix lane.
- **CI** (`.github/workflows/ci.yml`) runs `fmt`, `clippy`, the non-disc `cargo nextest run --workspace`, and the `docs`, `msrv`, `web-compile`, `web-shaders`, `determinism` and `leakage` jobs; `nightly.yml` builds binaries and runs no tests. **No CI job can run `test-data`** (no images).

**Never merge into the main checkout while the lead's gate is running on it**: the gate compiles the tree as it finds it. Hold the merge, or gate a detached worktree (`git worktree add --detach ../OpenAntiGrav-worktrees/lead-gate <sha>`, then `just link-data`). A red gate that overlapped a merge proves nothing; re-run. **The `lead-gate` worktree holds one gate at a time**: before `git checkout` there, check no gate runs in it. **A merged submodule commit is not on origin until pushed**, so `git submodule update` in `lead-gate` fails on it and leaves both submodules stale: push the submodule first (its origin is HTTPS; push over SSH, `git push git@github.com:OpenAntiGrav/<repo>.git main`), or `git -C <submodule> fetch <main checkout's submodule path> main` in `lead-gate`; `git status --porcelain` must be empty before gating. Prefer `nice -n 10 ionice -c 3` for the lead's own gate.

Four members gating at once reached load average 40, so every gate runs under the one shared lock: `scripts/gate-run.sh RECIPE` is `flock -o "$HOME/.cache/oag/gate.lock" just RECIPE` that builds what the recipe compiles (`just gate-prep`, idle priority) while it waits and fixes `OAG_GIT_HASH`; it logs every run to `$HOME/.cache/oag/gate-times.tsv`: read that, not a guess, when a gate looks slow (`docs/architecture/gate-wall-time.md`). The bare `flock -o ... just <recipe>` form still works and builds under the lock.

- **Always pass `-o`**: otherwise the `sccache` server a gate spawns outlives it holding the lock (`lsof "$HOME/.cache/oag/gate.lock"`); if a stale server holds it, run `sccache --stop-server` then `sccache --start-server` from the lead's shell. Members never kill it; the lead owns the lock's health.
- **`flock` wraps the outermost `just`** (`check:` is a dependency-list recipe, so a `flock` inside a recipe body does not hold across it); the lockfile lives outside every worktree so all members contend on one inode.
- **Put in every brief that waiting on the lock is not a hang**: the gate may sit several minutes before it starts (sixteen members ended a turn "waiting for a background job").

Do not add a dedicated gate-runner member and do not cap per-member `-j` (`.config/nextest.toml`: `test-data` is *tail-bound*).

## Drawing threads

- **Pre-draw them; never let members pick at random** (two independent picks collide at merge). A thread's `status` hints at the lane: `ready-to-wire` usually `oag-wire`, `re-first` usually `oag-re`, `measure` an `oag-capture` or `oag-perf` lane depending on what is measured; `blocked-*` and `parked` are not drawn until the blocker is cleared.
- **Lanes are disjoint by function set first, crate second; a Ghidra binary may be shared.** Two members may work the same program if their briefs name disjoint function/address sets; each appends its own dated section and `names.tsv` rows (the lead resolves append-append conflicts at merge). Every brief repeats `docs/ghidra/workflow.md`'s "Bridge quirks" (`program=` on every call, never `switch_program`). **Each member spawns its own emulator**; say in every brief which ports and displays are taken.
- Sources: `just threads` (frontmatter index, e.g. `--status ready-to-wire --title pulse`; `--counts` tallies), `handover/**/*.md`, a red test on `main`, what the previous member's report left open.
- **Prefer player-facing value** (standing instruction); a pass that produces only another analysis document is weak.

## The roster

Spawn every member as a named member (no `isolation` flag) of an agent type under `.claude/agents/`; each tells the member to read [`member-rules.md`](member-rules.md) first.

| agent | model / effort | use for |
| --- | --- | --- |
| `oag-re` | opus / high | recovering behaviour from an executable in Ghidra |
| `oag-wire` | sonnet / xhigh | ready-to-wire threads: the RE is done |
| `oag-format` | sonnet / xhigh | a file family to decode: reader, every-file ground truth, format page |
| `oag-capture` | sonnet / high | a question only the running original can answer |
| `oag-docs` | sonnet / medium | docs and handover audits, no code logic; `model: haiku` (0.5 slot) only for mechanical sweeps |
| `oag-scout` | haiku / high | read-only opportunity sweeps: a ranked, measurable list in a handover thread; no code, no builds, **0 slots** |
| `oag-compact` | sonnet / high | the standing comment compact lane: one crate per member, comments only; the lead runs `strip_rs_comments.py` (beside this skill) before every compact merge |
| `oag-perf` | opus / high | a measured speed, memory or size win, before and after |

Pick by the lane's **main output**, not its subject (mostly wiring with one small decompile is `oag-wire`). Overriding `model` at spawn works (say so in the report). Sonnet at xhigh is the default workhorse (maintainer). AI-behaviour and physics lanes run on opus.

## The brief

Members inherit none of your context; `member-rules.md` carries the shared rules, so the brief carries only:

0. **One claim and its acceptance evidence** (maintainer, 2026-10-21). The claim is one sentence that is true or false at the end ("Tech de Ra's hexagon panels render within 10 grey of the clean original in box (x,y,w,h)"). The evidence is the exact pair id and region whose metric must move, or the exact test going red to green; a whole-frame delta of 0.3-1.0 grey is inside re-capture noise and is not acceptance. Say whether it is a **measure** lane (ends with a number, a confidence, a doc page and a thread marked ready to wire; "one value unread, here is its address and read recipe" is a success) or a **wire** lane (starts from a ready-to-wire thread, ends with a test and a pair), never both. If you cannot write the claim, spawn a scout lane. For a maintainer-raised visible problem, never write "stop at the measurement" as the only ending: add "if the cause is unread and the read recipe fits your tools (name them), ask the lead whether to run it before reporting".
1. **The lane-specific values**: `<lane>` name, the member's Xvfb `:9N`, emulator ports, and which displays and ports other members hold. You assign every display number; members start and stop theirs only through `scripts/xvfb-own.sh`, so never tell a member to clean up a display by pattern or by guess.
2. **The lane**, in priority order, and who owns the lanes it may not touch, by crate and Ghidra function set.
3. **What is already established**, marked "do not re-derive", citing scratch reports and doc pages by path.
4. **The hard rules below that apply** and are not in `member-rules.md`.
5. **The baseline failure count**, naming the exact expected failures.
6. "Report every commit, including any made after your report."

If a new failure mode shows up mid-drive, relay it to running members at once, then add it to `member-rules.md` so the next spawn inherits it.

**Do not widen a running lane by message** (extensions were dropped twice): put extra scope in a fresh member's brief or the original brief. Short corrections ("main moved", a new rule) do arrive. A member reads messages at its next tool round, so late commits can follow a scope message: do not reap on the "done" report, check `git log --oneline main..<branch>` again, wait for the follow-up report, then merge and stop the member. `git branch -d` refusing ("not fully merged") means late commits.

## Hard rules to put in briefs, as they apply

The rest is in `member-rules.md` and CLAUDE.md (golden hashes, `_q` naming, judging as a player). Brief-only items:

- **The regression gate**: `race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round` stays all-twelve-clean; quote it verbatim before and after. State the current `01_Track` respawn index yourself (it moves with AI tuning).
- **Report per-lap and end-of-run shield separately, always labelled.**

## Judging a report

- **A negative result reported honestly is a good outcome**; so is "I could not determine X, here is its address".
- **A report that ends "unread, here is the recipe" is not the end of the question.** Before reaping, decide one of: **send it back** to run the recipe (when it fits the lane's tools and the emulator, worktree or context is still warm), or **queue the follow-up as the next lane on that track**, with its kind and estimate.

  **Judge the model with it.** If the member stopped because the read needs deeper reverse-engineering than it managed (a decompile it could not follow, a writer it could not find, two attempts on the same question), the follow-up runs on a more capable model: sonnet goes to opus. Fable needs the maintainer's approval (CLAUDE.md): recommend it with the reason and wait. If the recipe is mechanical (a watchpoint, a capture, a census), keep the model.

  Say the decision, the model and the follow-up's place in the order in the report to the maintainer; if the slot it needs is taken, ask which runs first. Never reap first and decide later (reaping loses the warm worktree and emulator), and never add a follow-up to the end of the queue without a word.
- **Be suspicious of a green claim you did not see**: ask whether the member watched the gate finish (builds that died on disk or LLVM errors read as flaky).
- **Check the diff against the claim.**
- **Verify visual claims yourself** with Read on the screenshot.

## When the user reports something from play

Treat it as a reliable oracle (their observations repeatedly beat static analysis). Turn it into a *measurement* with the discriminating question and what would falsify it; do not have members tune to match a memory (one was an accurate memory of a **different title**). It stays open until a player would no longer notice it, or the maintainer accepts a stop; keep it in the queue as a chain (measure, follow-up, wire).
