# Member rules

Every `/oag-drive` member reads this first. The brief supplies `<lane>`
(worktree, branch, scratch name), `:9N` (your Xvfb display), emulator ports,
lanes you must not touch, and the gate baseline. If one is missing, ask the
lead as "A question you cannot proceed without" (Report) says: end your turn
with it.

## Step zero

1. `cd /home/topaxi/projects/OpenAntiGrav && git worktree add ../OpenAntiGrav-worktrees/<lane> -b <lane> main`. Do not call `EnterWorktree` unless the brief says to (it waits on a permission prompt; a member sat blocked an hour). Use absolute paths (a subagent's `cd` does not carry to its next Bash call).
2. Work ONLY in `/home/topaxi/projects/OpenAntiGrav-worktrees/<lane>`: `git merge main`, then `git submodule update --init assets/mods/examples assets/mods/official` (without the examples test-data tests fail; without the official mods the RACE REMIX tests skip), then `just link-data`. An update that aborts halfway can leave a submodule with every file deleted: check `git -C assets/mods/official status --short` is empty, else rerun with `--force`. "not our ref" means the lead has not pushed the pointer: fetch it from `/home/topaxi/projects/OpenAntiGrav/.git/modules/assets/mods/<name>`. **The same holds for every later `git merge main`** (a conflict send-back included): run the submodule update before any add or commit, stage by path (never `git add -A`, `git add .` or `commit -a` during a merge), and check `git diff --cached -- assets/mods` is empty unless your lane moves a pointer forward. A stale checkout staged into a merge once pushed a mod-author skill update back out of main.
3. Never write into the main checkout except your scratch dir `/home/topaxi/projects/OpenAntiGrav/data/scratch/<lane>/` (create it). **`data/` in your worktree is symlinks into the main checkout's disc images**: never `rm` under it, and in a script `rm` only absolute paths inside your scratch dir, quoted, after a guard (`case "$p" in /home/topaxi/projects/OpenAntiGrav/data/scratch/<lane>/*) ;; *) exit 1;; esac`). (under zsh an unquoted `rm -rf $R` once deleted four disc images, 2026-11-08).
   Keep scratch small: cargo `target/` dirs stay in your worktree; bring disc data in with `ln` from `data/extracted` or `data/images`, never `cp` (an Omega copy is 47 GiB). Delete what you no longer need before your final report.
4. **Helpers.** Never a fork (`subagent_type: "fork"`): it starts in the main checkout and once wrote its findings there. Two kinds are allowed, one of each at a time:
   - **Read-only scan** (maintainer, 2026-10-19): `subagent_type: "Explore"`, `model: "haiku"`, absolute paths into your worktree or `data/`, told to write nothing and return at most 15 lines. Nothing it says is evidence until you have read the cited `path:line`.
   - **Mechanical writer** (maintainer, 2026-11-10): a fully specified change you can check, not judgement or evidence: a file split for `just check-size`, a rename across call sites, a test-file move, call-site fixes after your API change, `fmt`/`clippy` fallout. `subagent_type: "general-purpose"`, `model: "sonnet"` (`"haiku"` for pure text edits); never opus or fable. Before spawning, `SendMessage` `team-lead` one line (what, which model) and **wait for the lead's yes** (the budget may be full: sonnet 1, haiku 0.5); with no reply by your next tool round, do the edit yourself (a script or `sd`). It works **in your worktree** only: its prompt names the absolute path and the files it may edit, and says to `cd` nowhere else, make no commit, branch, stash or worktree, run no gate (one `cargo check` or `cargo nextest run -p <crate>` is fine), and reply in at most 15 lines. Leave those files alone while it runs, then read its `git diff --no-ext-diff`, check the main checkout's `git status --porcelain` is unchanged, and commit it yourself. It is never a second lane: no RE, Ghidra, emulator or doc claims.
5. Never `git stash`: the stash list is shared by all worktrees. Commit work in progress on your branch.

## Safety on a shared machine

- **Display.** Run `oag-game` headless (`--screenshot`, `--dry-run`, `--trace-out`) when you can; `oag-view` too (a probe flag like `--draws` without `--screenshot` still opens a window). Any windowed launch (emulators included) uses this literal prefix inside your own Xvfb: `env -u WAYLAND_DISPLAY WINIT_UNIX_BACKEND=x11 DISPLAY=:9N` (`DISPLAY` alone lets winit pick the inherited Wayland display and put the window on the maintainer's desktop). RADV cannot present on Xvfb, so a windowed `oag-game` there also needs `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json` (5-7 fps).
  - Start a bare Xvfb only with `scripts/xvfb-own.sh start :9N <lane>` and stop it only with `scripts/xvfb-own.sh stop :9N <lane>`: it refuses a display whose socket or lock file exists (pick the next free number, tell the lead), records the pid under your scratch dir, and signals only that pid while it is still an Xvfb serving that `:N`. `scripts/xvfb-own.sh status` lists recorded and unrecorded live displays. A harness that starts its own display (`pcsx2-drive.py`, `rpcs3-drive.py`, `ppsspp-start.sh`) is stopped with its own stop command. Never touch `:0`. Stop your displays before you report.
  - **Never kill a display, Xvfb or emulator by guessing ownership** ("looks unused" is a guess; a member killed another's `:92` that way on 2026-11-23/24). Signal only a pid you started or wrote into your own pid file, after checking `/proc/<pid>/comm` and that it descends from your launch.
  - xwayland-satellite answers any display number and `xdpyinfo` is not installed: stop if a tool says a display you did not start is "already up" (a maintainer's whole desktop session shut down 5 s after a member's retry loop relaunched Vita3K onto one, 2026-11-18; cause unproven).
- **`--race` skips the front end**: anything the front-end flow builds (EndRace screens, campaign state) never exists in a `--race` run. Walk the menus, or use `--menu-page` for a still.
- **Scope.** Do what the brief requires, then gate; stretch goals only after a green gate and only if the brief lists them. A wait on a race that will not finish is a signal to find a shorter path.
- **Profile.** Every `oag-game` run sets `XDG_CONFIG_HOME`, `XDG_DATA_HOME` and `XDG_STATE_HOME` under your scratch dir; the default `~/.config/oag` is the maintainer's saves.
- **Unlockables.** Debug builds unlock everything (`[dev] unlock_everything`, absent means on; release ignores it). To build or debug anything locked (loyalty, craft variants, HUD Style skins, circuits, campaign cells), set `unlock_everything = false` under `[dev]` in your profile's `settings.toml` first, or the check proves nothing.
- **Audio.** `oag-game` always gets `--no-audio`; emulators run muted; verify sound with a WAV or log lines, never by ear. **Waydroid counts** (it plays through the host, no `--no-audio`). Before every Waydroid launch and after any restart, silence both layers and check both read silent; never raise the volume back:

  ```sh
  adb -s <waydroid-serial> shell cmd media_session volume --stream 3 --set 0
  pactl -f json list sink-inputs \
    | jq '.[] | select(.properties["application.name"]=="Waydroid") | .index' \
    | xargs -r -n1 -I{} pactl set-sink-input-mute {} 1
  ```

- **`ls` is a shell function here and runs an executable it is given** (a windowed game with sound once ran on the maintainer's desktop that way). Use `/bin/ls`.
- **Processes.** Record the PID of everything you start; kill only by PID. Never `pkill -f <pattern>` or `pkill -P $(pgrep -f ...)`.
- **Memory.** Run render loops, bisects and batch scripts sequentially, never as a parallel fan-out, and cap any batch: `systemd-run --user --scope -q -p MemoryMax=8G -p MemorySwapMax=0 -p TasksMax=64 <cmd>` (a runaway `python3` fan-out OOM-killed the lead's session). If you see a `python3` count climbing, tell the lead.
- **Profilers and tracers always run under a memory cap**: `systemd-run --user --scope -q -p MemoryMax=16G -p MemorySwapMax=0 heaptrack ...` (likewise `valgrind`, `perf record` on a long run; `heaptrack_interpret` reached 50 GB and killed two panes). A capped run that dies is a finding (use a shorter `--ticks` or another title), not a reason to raise the cap. Run them with `DISPLAY` and `WAYLAND_DISPLAY` unset (`env -u DISPLAY -u WAYLAND_DISPLAY heaptrack ...`; they open windows), read with `heaptrack_print`.
- **Never name a scratch script after a stdlib module** (`bisect.py`, `random.py`, `test.py`, `json.py`, `math.py`): Python puts the script's directory first on `sys.path` (that OOM was a fork bomb). Use descriptive names (`slot_bisect.py`).
- **Emulators.** Start your own instance (`docs/reverse-engineering/emulator-recipes.md`, "Member setup facts"): PPSSPP with its own `HOME`, `XDG_CONFIG_HOME`, debugger port and display; RPCS3 with its own config copy. Never attach to another member's. Virtual pads are system-wide uinput devices: run your pad drivers with the `OAG_PAD_SEAT=N` your brief gives you (`scripts/pad_seat.py`, `docs/tools/compare-page.md` "Pad seats") and never touch another seat's pad.
- **Ghidra.** The GUI bridge is shared on `127.0.0.1:8089`. Pass `program=` explicitly on every call (by path, e.g. `/pulse/BOOT-psp-pulse-usa.BIN`, `/hdfury/EBOOT-ps3-hdfury-eu.elf`); never call `switch_program`. Headless `analyzeHeadless` works when no GUI is up.

## Waiting on an emulator capture

Recipes and tools: `docs/reverse-engineering/emulator-recipes.md`.

- **Never poll a capture log for a success marker** (it cost about 9.6 h on 2026-10-08). Run the capture in the foreground under the watchdog with a timeout under 600000 ms: `uv run --with evdev python3 scripts/emu-run.py --child-status <log-dir>/status.json -- python3 scripts/<capture>.py ...`. It exits within 150 s of the last output (`STALLED ... child stage`) and at 570 s overall; read the stage and do not retry unchanged. Longer than ~9 minutes: `emu-run.py --bg --status F -- ...`, then `emu-run.py wait F` in a loop (returns 4 while healthy).
- **Keep the emulator running between captures** (`rpcs3-drive.py serve`, `scripts/ppsspp-start.sh`, `OAG_RPCS3_ATTACH=1`); boot fresh only when the measurement needs the boot. Build your private tree with `scripts/emu-env.sh`.
- **Never set `OAG_RPCS3_OWN_HOME`**: it is the maintainer's opt-out of `rpcs3-drive.py`'s private-env guard; source `env.sh` instead.
- **Stop emulators by pid**: `rpcs3-drive.py stop` (never `--all`; never `pkill -x rpcs3`), `ppsspp-start.sh <lane> stop`. **No unattended emulator retry loops**: launch one run at a time, watch it, on a display you started.
- Save states live only under `data/saves/<title>/`, never committed.

## Build and gate

- Build once; avoid `--release` unless needed. On `No space left on device` or `rustc-LLVM ERROR: IO failure on output stream`, **stop and report**; do not retry.
- **Run a new or changed test under a memory cap first**: `systemd-run --user --scope -q -p MemoryMax=8G -p MemorySwapMax=0 cargo nextest run -p <crate> <test>` (an unfinished test once hit 51 GB).
- **Break it before you report** (independent reviews found a test that could not fail in four of seven lanes, 2026-11-23/24). For every new or changed test that guards a fix: commit the fix, revert it (or the guarded branch) in the working tree, run the test and see it fail, restore with `git checkout HEAD -- <path>` (never `git stash`), and see it pass again. Say "reverting X turns Y red" in the report. Where it cannot be done (it needs a real pad or window), say why.
- Gate before reporting, always through the shared lock, with the **affected** gate:

  ```sh
  scripts/gate-run.sh gate-affected
  scripts/gate-run.sh test-data-affected 2>&1 \
    | tee /home/topaxi/projects/OpenAntiGrav/data/scratch/<lane>/test-data.log
  ```

  `scripts/gate-run.sh` is the shared `flock -o` gate lock with the build moved out of it, so the lock is held for the checks and tests only; it sets `OAG_REQUIRE_GAME_DATA=1` for `test-data-affected` and logs every run (`docs/architecture/gate-wall-time.md`). Both run every check in full and the tests that link your change (`docs/architecture/affected-gate.md`). **Quote the printed `affected:` selection in your report.** The root `Cargo.toml`, the toolchain or `.cargo/` selects the full suite, and a change to `oag-core` or a simulation crate selects most of it; that is expected. The lead runs the full pair per merge batch.

  **Which GPU.** Use the discrete GPU by default; the `gpu-vram` group caps game-spawning tests at 4 at once. A SIGSEGV at exit means a GPU-touching thread outlived `main`: register it with `oag_thread::gpu_workers`, do not switch GPU. Time ceilings are judged on the full-size machine: on a smaller one a red that is only `check-test-budget` time is green.

  **wgpu's GL backend is best effort** (maintainer, 2026-10-15). It must never block the default backends (Vulkan, Metal, Dx12, WebGPU): switch a feature GL cannot do off on GL and name it in the loader report. Do not gate on a GL run.

  **A render bench takes the same lock itself** (`just bench-render`): never wrap it in `flock`, run it inside a gate or a gate inside it (the same lock twice deadlocks), or set `OAG_BENCH_NO_LOCK=1`. Quote the load average its result file records.

  Check the exit status **in the same Bash call as the pipeline** (each call is a fresh shell): append `; echo "rc=${pipestatus[1]}"` (zsh) or `${PIPESTATUS[0]}` (bash). Never pipe `test-data` into `tail` (it masks the exit code). **The gate may sit several minutes before it starts** because another member holds the lock; wait it out and never run the gate outside the lock.
- **Gate your branch as it is; do not merge `main` into it first** (maintainer, 2026-10-12); merge `main` only when the lead sends you back for a conflict or red merge, then gate again.
- A docs/`handover/`/`HANDOVER.md`-only lane runs `just check-docs` (plus `check-names`, `check-handover` when those files moved) instead of the gate.
- **A commit after your last gate that touches any `.rs` file, even a doc comment, needs `just gate-affected` re-run before you report** (a "docs plus one test line" commit once turned `main` red on `check-size`).
- The brief states the baseline failure count; anything red beyond it is yours.
- **Never end a turn waiting for a background job to notify you**; no member has ever been woken that way. Run long commands in the foreground with a 600000 ms timeout, or poll the output file. (Ending a turn to wait for the lead's answer is the opposite case and correct: see "A question you cannot proceed without".)
- **No fine-grained Monitors** (maintainer, 2026-10-19): each event wakes you and your idle turn wakes the lead. Watch only the whole run finishing, or poll at 10 minute intervals or more.

**A test that proves an open defect is `#[ignore]`d and also returns early unless `OAG_REVIEW_FINDINGS` is set.** `just test-data` and `lead-test-data` run every ignored test, so a bare `#[ignore]` on a test that fails today turns the lead's gate red. Run it with `OAG_REVIEW_FINDINGS=1` to see it fail; the fix removes both the guard and the `#[ignore]`.

## Commits

- **A `git diff`/`git show` whose output is applied, parsed or saved takes `--no-ext-diff`** (the maintainer's `diff.external = difft` prints a structural view). Never change that config. `git apply -R` also fails on a file later commits touched; use `--3way` or `git checkout A -- <path>`.
- **Read `git diff --cached --stat` before every commit** and unstage what you did not mean to change (a commit once silently deleted six examples that docs cite).
- Commit early and often. If pinentry blocks, `git commit --no-gpg-sign`. End every message with a `Co-Authored-By:` trailer naming your own model; never a `Claude-Session:` line or other session link (maintainer's rule).
- Never merge into main; the lead merges.
- Committed golden hashes are never edited to make a test pass. If behaviour legitimately moves, regenerate from your tree in a **separate** commit naming the behaviour change.

## Project rules that bite members

- **No compatibility re-exports.** No crate is published: when you move an item, update every call site in the same change; no `pub use old::path` (maintainer).
- **The maintainer may type into your pane directly**: act on it like the brief and name it as such in your report.
- **Never invent what the assets author, and label provenance only where in doubt** (CLAUDE.md, both rules in full, including the 2026-11-02 ruling).
- **The lead's gate has the mod examples and official mods, which change the menus** (RACE CAMPAIGN opens a CAMPAIGNS picker, RACEBOX gains REMIX): a test that drives the menus must pass with and without them, or pin `OAG_MODS` to an empty directory.
- **A mod API change updates the mod author skill in the same change** (CLAUDE.md; maintainer, 2026-10-30), also when how a mod is built, tested or run changes. Commit it on your `assets/mods/examples` submodule branch and say in your report whether it needed a change.
- **Artificial load runs under the gate lock**: a lane that loads the machine on purpose (`stress-ng`, `yes`) does it inside `flock -o "$HOME/.cache/oag/gate.lock"` and kills its generators by PID before releasing it (unlocked generators once pushed the lead's full gate to 621 s).
- **Pulse PS2 inherits what PSP measured** (maintainer, 2026-10-29). Where a PS2 value or behaviour is invented or chosen because PS2 is unmeasured, use PSP's measured law, labelled **inherited from PSP**, with no PS2 confidence. Never gate a measured PSP law off on PS2 and leave the old stand-in. PS2-measured values and PS2-authored disc data stay.
- **A weapon change updates `docs/gameplay/weapon-lineage.md` in the same change** (law, tuning, AI use, visuals or sound): move the matching cell (measured with evidence link, inherited from X, chosen, absent, not investigated); a new `Weapons`/`WeaponModels` field or effect trigger gets its row in "Checked axes". `just check-weapon-lineage` checks only part of this (maintainer, 2026-10-19: a Pulse law ran on HD with HD's numbers and wrecked the field).
- **A 2048 finding is checked against Omega, and the reverse, in the same change** (CLAUDE.md); say which of its four outcomes in your report.
- **Keep a reader change from switching on what your lane did not measure**: if a fix to a shared lookup (bank names, a table, an effect list) makes cues, effects or draws resolve beyond your lane, scope it to your lane's caller. "Load report went from 42 missing to 6" is a warning, not a win.
- Judge as a player would: more than one frame, at the size a player sees, read with the Read tool. A green test is not a picture.
- A new front-end screen ships with mouse/pointer support in the same change.
- Findings go into `docs/` and the lane's `handover/` thread, not only scratch. Strike what closed; delete a thread file once it has fully landed. A new thread needs `status`, `titles`, `updated`, `summary` (`HANDOVER.md`, "Open threads"); when you change what is open, update those too. Nothing outside `handover/` and `HANDOVER.md` links into `handover/`.
- **No tracked file may cite `data/scratch/`** (maintainer, 2026-10-09): scratch is deleted after a lane merges. Put the evidence in the doc (numbers, table, reproducing command); a tool worth keeping goes to `scripts/`; a reference frame worth keeping goes to `data/reference/`.
- **Never commit `docs/overview/status.md`, and do not run `gen-status` or `check-status`.** `gate-affected` leaves `check-status` out; the lead regenerates the file per merge. This overrides any agent file that says otherwise.
- **Never write a compare-page manifest (`data/reference/compare-page/*.toml`) from your worktree** (`data/` there is a symlink into the main checkout; a manifest pointing into a reaped worktree broke the maintainer's page, 2026-10-21), and **do not run `scripts/compare-rerender-ours.py` on `data/reference/compare-page`** (it rewrites the shared pairs with your branch's binary). Copy the pair dirs to `data/scratch/<lane>/pairs/`, call its `rerender(pair_json, <worktree>/target/debug/oag-game)` on the copies, and give the lead the commands to adopt them; the lead re-renders the page after merging.
- **Steam Deck builds from a worktree**: `data/appimage` must be a real directory, not the `link-data` symlink. Jobs started detached over ssh die when the session closes; run them in the foreground.
- **The original's Autopilot is not a physics reference** (maintainer, 2026-10-20). Never measure a physics law, jump, landing, wall response or speed curve off an Autopilot run. Drive the original with player input (held throttle, scripted stick via the emulator's virtual pad), run ours under the same script, aligned by track position. Autopilot is fine for getting somewhere.
- No em or en dashes in anything you write; no decorative separator comments.

## Report

Write full findings to `/home/topaxi/projects/OpenAntiGrav/data/scratch/<lane>/report.md`: commits per step, the gate result you watched finish (counts and exit status), any regression gate quoted before and after, screenshot paths, what is still open. Quote your brief's claim and say whether it came out true, false or unknown, with the acceptance evidence. Return ONLY that path plus at most 15 lines (long returns can silently fail to arrive). A negative result reported honestly is a good outcome. If your claim came out false or unknown, end with a **Next:** line: the follow-up that would settle it, its kind (read, capture, wire), an estimate, whether you could run it now with what you have running, and whether it needs a more capable model than yours (say why). Then stop for the lead's answer before tearing down your emulator.

**Send each report once: as the final text of your turn, not with `SendMessage`.** The lead receives that text when your turn ends; a `SendMessage` report plus a closing summary arrives twice. The same applies to replies to the lead.

**A question you cannot proceed without: end your turn.** Make it the final text ("Question for the lead: ... Options, each in full: ... Default if no answer: ...") and stop. Do not `SendMessage` it and keep working, and do not poll or sleep for an answer: the lead may not see a mid-turn message until your turn ends, and the answer arrives as a new turn. A question you can proceed without: put your default in the message, proceed on it, and treat any reply as a correction. `SendMessage` mid-turn is for that second kind only.

**Ask before leaving part of your claim unwired** (maintainer, 2026-11-20). If you are about to report a claim as done while a screen, title, case or string it covers stays unwired, stop and ask the lead first, as a question you cannot proceed without: name what you would leave out, why, and the work it would take. Do not file it as an "open" leftover on your own. A part the brief itself excludes needs no question, but quote the brief's words in your report.

**Read your messages before the final reply.** A lead message reaches you only at a tool round, so after your last edit run one more tool call (`git log --oneline main..HEAD`, or the check you were about to run), act on any lead message it delivers, and only then write the report (five reports crossed a lead message on the 2026-11-23/24 overnight drive).

**Your report is the end of the lane.** After sending it, do not commit, merge main or edit your worktree unless the lead sends you back; the lead removes the worktree as soon as the report arrives. Finish everything first.

**If a message wakes you after your report**, your worktree is gone or belongs to someone else. Answer in text only: never `cd` into another lane's worktree, rename a branch or commit. If it asks for more work, say so and stop; the lead spawns a fresh member.
