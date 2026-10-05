# Member rules

Every `/oag-drive` member reads this before anything else. The lead's brief
supplies the lane-specific values used below: `<lane>` (worktree, branch and
scratch name), `:9N` (your Xvfb display), any emulator ports, the other lanes
you must not touch, and the gate baseline. If the brief is missing one of
them, ask the lead with `SendMessage` to `main` rather than guessing.

## Step zero

1. `cd /home/topaxi/projects/OpenAntiGrav && git worktree add ../OpenAntiGrav-worktrees/<lane> -b <lane> main`
2. From then on work ONLY inside `/home/topaxi/projects/OpenAntiGrav-worktrees/<lane>`:
   `git merge main`, then `just link-data`.
3. Never write into the main checkout, except your scratch directory
   `/home/topaxi/projects/OpenAntiGrav/data/scratch/<lane>/` (create it).
4. Do not spawn sub-agents or forks. A member fork once wrote into the main
   checkout.

## Safety on a shared machine

- **Display.** Run `oag-game` headless (`--screenshot`, `--dry-run`,
  `--trace-out`) whenever you can. The same goes for `oag-view`: a probe flag
  such as `--draws` without `--screenshot` still opens a window (2026-10-01, on
  the maintainer's display for two minutes). Any windowed launch, emulators included,
  uses this literal prefix inside your own Xvfb:
  `env -u WAYLAND_DISPLAY WINIT_UNIX_BACKEND=x11 DISPLAY=:9N`.
  `DISPLAY` alone is not enough: `WAYLAND_DISPLAY` is inherited and winit
  prefers it, which has put member windows on the maintainer's desktop twice.
  RADV cannot present on Xvfb (no DRI3) and draws black frames, so a windowed
  `oag-game` there also needs software Vulkan:
  `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json`. Expect 5-7 fps.
- **`--race` skips the front end.** Anything built by the front-end flow
  (the EndRace screens, campaign state) never exists in a `--race` run. Walk
  the menus instead, or use `--menu-page` for a still.
- **Scope.** Stop after the steps your brief requires and gate. Stretch
  goals come only after the gate is green, and only if the brief lists them.
  A long wait on a race that will not finish is a signal to stop and find a
  shorter path, not to wait longer.
- **Profile.** Every `oag-game` run, headless or windowed, sets
  `XDG_CONFIG_HOME` (and `XDG_DATA_HOME`/`XDG_STATE_HOME`) to a directory
  under your scratch dir. Progress, records, ghosts and settings persist
  automatically, so a run on the default `~/.config/oag` writes into the
  maintainer's own saves. Found 2026-09-30 when a member's windowed walk
  shared it.
- **Audio.** `oag-game` always gets `--no-audio`; emulators run muted. Verify
  sound by writing WAV and inspecting it, never through the speakers.
- **Processes.** Record the PID of everything you start, and kill only by
  that PID. Never `pkill -f <pattern>`: one pattern kill took down every
  member's `oag-game --race` run.
- **Emulators.** Start your own instance (PPSSPP with its own `HOME` and
  `XDG_CONFIG_HOME`, debugger port and display; RPCS3 with its own config
  copy). Never attach to an instance another member started.
- **Ghidra.** When the GUI bridge is up it listens on `127.0.0.1:8089` and is
  shared. Pass `program=` explicitly on every call (by path, e.g.
  `/pulse/BOOT-psp-pulse-usa.BIN`, `/hdfury/EBOOT-ps3-hdfury-eu.elf`), and
  never call `switch_program`. Headless `analyzeHeadless` works when no GUI
  is up.

## Build and gate

- Build once. Avoid `--release` unless the lane needs it. On
  `No space left on device` or `rustc-LLVM ERROR: IO failure on output stream`,
  **stop and report**: these are not flaky errors, so don't retry.
- **Run a new or changed test under a memory cap first**, before it goes
  near the gate: `systemd-run --user --scope -q -p MemoryMax=8G -p
  MemorySwapMax=0 cargo nextest run -p <crate> <test>`. On 2026-10-05 an
  unfinished `oag-post` test reached 51 GB resident; the kernel's OOM
  pressure killed two members and the lead's own session before the test
  itself died. A capped run fails alone instead.
- Gate before reporting, always through the shared lock:

  ```sh
  flock -o "$HOME/.cache/oag/gate.lock" just
  flock -o "$HOME/.cache/oag/gate.lock" env OAG_REQUIRE_GAME_DATA=1 just test-data 2>&1 \
    | tee /home/topaxi/projects/OpenAntiGrav/data/scratch/<lane>/test-data.log
  ```

  Then check the exit status (`${pipestatus[1]}` in zsh, `${PIPESTATUS[0]}`
  in bash) **in the same Bash call as the pipeline**: each tool call is a
  fresh shell, so a separate `echo` prints blank (two members on 2026-10-05
  could not read theirs). Append `; echo "rc=${pipestatus[1]}"` to the gate
  command itself. Never pipe `test-data` into `tail`: it masks the exit code.
  **The gate may sit for several minutes before it starts** because another
  member holds the lock. That is correct, not a hang. Wait it out and never
  fall back to a bare `just`.
- **Run `git merge main` right before your final gate**, and again if the
  lead tells you main moved. A gate on a stale base proves nothing about the
  merge; on 2026-10-05 a lane gated without the engine-sound change that
  touched the same `race::load`, and the lead had to re-gate the combination.
- A lane that changed only docs, `handover/` or `HANDOVER.md` runs
  `just check-docs` (plus `check-names`, `check-handover` or `check-status`
  when those files moved) instead of the full gate.
- **A commit after your last full gate that touches any `.rs` file, even one
  test line or a doc comment, needs `just` re-run before you report.** On
  2026-10-03 a "docs plus one test line" final commit pushed
  `race_ground_truth.rs` 5 lines over `check-size`'s ceiling, and `main` went
  red. `just` (without `test-data`) is enough when no logic changed.
- The brief states the baseline failure count. Anything red beyond it is
  yours.
- **Never end a turn waiting for a background job to notify you.** No member
  has ever been woken that way. Run long commands in the foreground with a
  600000 ms timeout, or poll the job's output file yourself.

## Commits

- **Read `git diff --cached --stat` before every commit** and unstage
  anything you did not mean to change. On 2026-10-05 a member's commit
  silently deleted six committed examples that docs cite as reproducers.
- Commit early and often. If pinentry blocks, use `git commit --no-gpg-sign`.
  End every message with a `Co-Authored-By:` trailer naming your own model.
- Never merge into main. The lead merges.
- Committed golden hashes are never edited to make a test pass. If behaviour
  legitimately moves, regenerate from your own tree in a **separate** commit
  that names the behaviour change.

## Project rules that bite members

- **Never invent what the assets author.** Parse it and play it. If it won't
  parse, draw or play nothing and say so in the loader report. Anything
  genuinely chosen is labelled **chosen, not measured**, with no confidence
  score; a score means measured off the original.
- **A 2048 finding is checked against Omega, and the reverse, in the same
  change** (CLAUDE.md, "A 2048 finding is checked against Omega"). Record
  `ported`, `checked, applies, not wired`, `checked, differs` or `not
  checkable` in the doc page and say which in your report. A quick census and
  reader run, not a second lane.
- **Keep a reader change from switching on what your lane did not measure.**
  If a fix to a shared lookup (a bank's names, a table, an effect list)
  suddenly makes cues, effects or draws resolve beyond your lane, scope the
  change to your lane's caller and leave the rest as they were. On
  2026-10-05 a hashed bank-name lookup turned on every 2048 sound cue: the
  engine was right, but the player heard noise and a perfect-lap
  announcement mid-lap. "Load report went from 42 missing to 6" is a warning,
  not a win.
- `fd` and `rg` return nothing under `data/` unless you pass `--no-ignore`.
- Judge as a player would: more than one frame, at the size a player sees,
  read with the Read tool. A green test is not a picture.
- A new front-end screen ships with mouse/pointer support in the same change.
- A new `menu.toml` row needs its `string_id` and its `english.toml` text.
- Findings go into `docs/` and the lane's `handover/` thread, not only into
  scratch. Strike what closed; delete a thread file and its `HANDOVER.md`
  index line together once its work has fully landed. Nothing outside
  `handover/` and `HANDOVER.md` may link into `handover/`.
- Avoid em and en dashes in anything you write, and write no decorative
  separator comments.

## Report

Write full findings to
`/home/topaxi/projects/OpenAntiGrav/data/scratch/<lane>/report.md`. Include
commits per step, the gate result you watched finish (counts and exit status),
any regression gate quoted before and after, screenshot paths, and what is
still open. Return ONLY that path plus at most 15 lines: long returns can
silently fail to arrive. A negative result reported honestly is a good
outcome, and "I could not determine X, here is its address" is a result.

**Your report is the end of the lane.** After you send it, do not commit,
merge main into your branch, or edit your worktree unless the lead sends you
back. The lead merges and removes your worktree as soon as the report arrives.
On 2026-10-02 a member kept committing after its report and was mid-way
through merging main when its worktree was removed. Finish everything first,
then report.
