# Member rules

Every `/oag-drive` member reads this first. The brief supplies `<lane>`
(worktree, branch, scratch name), `:9N` (your Xvfb display), emulator ports,
lanes you must not touch, and the gate baseline. If one is missing, ask
`team-lead` with `SendMessage` (`main` may not resolve from a member).

## Step zero

1. `cd /home/topaxi/projects/OpenAntiGrav && git worktree add ../OpenAntiGrav-worktrees/<lane> -b <lane> main`
2. Work ONLY in `/home/topaxi/projects/OpenAntiGrav-worktrees/<lane>`: `git merge main`, then `git submodule update --init assets/mods/examples assets/mods/official` (without the examples test-data tests fail; without the official mods the RACE REMIX tests skip), then `just link-data`.
3. Never write into the main checkout except your scratch dir `/home/topaxi/projects/OpenAntiGrav/data/scratch/<lane>/` (create it). **`data/` in your worktree is symlinks into the main checkout's disc images**: never `rm` under it, and in a script `rm` only absolute paths inside your scratch dir, quoted, after a guard (`case "$p" in /home/topaxi/projects/OpenAntiGrav/data/scratch/<lane>/*) ;; *) exit 1;; esac`). Under zsh an unquoted variable does not word-split, and on 2026-11-08 one `rm -rf $R` deleted four disc images and an extract.
   Keep scratch small: keep cargo `target/` dirs in your worktree (reaped with it), never under scratch, and bring disc data in with `ln` from `data/extracted` or `data/images`, never `cp` (an Omega game copy is 47 GiB; five copies filled the disk). Delete what you no longer need before your final report; the lead removes the rest when it reaps you.
4. **Helpers.** Never a fork (`subagent_type: "fork"`): it starts in the main checkout and once wrote its findings there. Two kinds of helper are allowed, one of each at a time:
   - **Read-only scan** (maintainer, 2026-10-19): `subagent_type: "Explore"`, `model: "haiku"`, absolute paths into your worktree or `data/`, told to write nothing and return at most 15 lines. Nothing it says is evidence until you have read the cited `path:line`.
   - **Mechanical writer** (maintainer, 2026-11-10): a change whose result is fully specified and checkable by you, not judgement or evidence: a file split for the 1,500-line or 500-line test-module ratchet (`just check-size`), a rename across call sites, a test-file move, fixing call sites after an API change you made, `fmt`/`clippy` fallout. `subagent_type: "general-purpose"`, `model: "sonnet"` (or `"haiku"` for pure text edits); never opus or fable. Before spawning, `SendMessage` `team-lead` one line (what, which model) so the lead can count it against the budget (sonnet 1, haiku 0.5). It works **in your worktree**, never a new worktree or branch: its prompt names your worktree's absolute path, lists the files it may edit, and says to `cd` nowhere else (never the main checkout), to make no commit, branch, stash or worktree, to run no gate (a `cargo check` or one `cargo nextest run -p <crate>` is fine), and to reply in at most 15 lines with the files it changed. Leave those files alone while it runs. Then read its `git diff --no-ext-diff`, check the main checkout's `git status --porcelain` is unchanged, commit it yourself, and your gate covers it. It is never a second lane: no RE, no Ghidra, no emulator, no doc claims.
5. Never `git stash`: the stash list is shared by all worktrees. Commit work in progress on your branch.

## Safety on a shared machine

- **Display.** Run `oag-game` headless (`--screenshot`, `--dry-run`, `--trace-out`) when you can. `oag-view` too: a probe flag like `--draws` without `--screenshot` still opens a window. Any windowed launch (emulators included) uses this literal prefix inside your own Xvfb: `env -u WAYLAND_DISPLAY WINIT_UNIX_BACKEND=x11 DISPLAY=:9N` (`DISPLAY` alone lets winit pick the inherited Wayland display and put the window on the maintainer's desktop). RADV cannot present on Xvfb, so a windowed `oag-game` there also needs `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json` (5-7 fps). Before starting a display on `:9N`, check it is free (`test -e /tmp/.X11-unix/X9N || test -e /tmp/.X9N-lock` means taken); if taken, pick the next free one and tell the lead. Never touch `:0`. Stop your Xvfb by PID before you report.
- **`--race` skips the front end**: anything the front-end flow builds (EndRace screens, campaign state) never exists in a `--race` run. Walk the menus, or use `--menu-page` for a still.
- **Scope.** Do what the brief requires, then gate. Stretch goals only after a green gate and only if the brief lists them. A wait on a race that will not finish is a signal to find a shorter path.
- **Profile.** Every `oag-game` run sets `XDG_CONFIG_HOME`, `XDG_DATA_HOME` and `XDG_STATE_HOME` under your scratch dir; the default `~/.config/oag` is the maintainer's saves.
- **Unlockables.** Debug builds unlock everything (`[dev] unlock_everything`, absent means on; release ignores it). When you build or debug anything locked (loyalty, craft variants, HUD Style skins, circuits, campaign cells), set `[dev]` / `unlock_everything = false` in your profile's `settings.toml` first, or the check proves nothing.
- **Audio.** `oag-game` always gets `--no-audio`; emulators run muted; verify sound with a WAV or log lines, never by ear. **Waydroid counts** (no `--no-audio`; it plays through the host). Before every Waydroid launch and after any restart, silence both layers and check both read silent; never raise the volume back:

  ```sh
  adb -s <waydroid-serial> shell cmd media_session volume --stream 3 --set 0
  pactl -f json list sink-inputs \
    | jq '.[] | select(.properties["application.name"]=="Waydroid") | .index' \
    | xargs -r -n1 -I{} pactl set-sink-input-mute {} 1
  ```

- **`ls` is a shell function here and runs an executable it is given**: `ls target/quick/oag-game` once put a windowed game with sound on the maintainer's desktop for nine minutes. Use `/bin/ls`.
- **Processes.** Record the PID of everything you start; kill only by PID. Never `pkill -f <pattern>` (it once killed every member's run).
- **Memory.** Run render loops, bisects and batch scripts sequentially, never as a parallel fan-out. Cap any batch: `systemd-run --user --scope -q -p MemoryMax=8G -p MemorySwapMax=0 -p TasksMax=64 <cmd>` (a runaway fan-out of `python3` processes OOM-killed the lead's session on 2026-10-22). If you see a `python3` count climbing, tell the lead.
- **Profilers and tracers always run under a memory cap**: `systemd-run --user --scope -q -p MemoryMax=16G -p MemorySwapMax=0 heaptrack ...` (likewise `valgrind`, `perf record` on a long run). Without it the OOM killer takes your whole session with it: `heaptrack_interpret` reached 50 GB on an Omega race twice on 2026-11-10 and killed two members' panes outright. A capped run that dies is a finding (record it, use a shorter `--ticks` or another title), not a reason to raise the cap.
- **Never name a scratch script after a stdlib module** (`bisect.py`, `random.py`, `test.py`, `json.py`, `math.py`): Python puts the script's directory first on `sys.path`, so other scripts import yours (that OOM was a fork bomb). Use descriptive names (`slot_bisect.py`).
- **Profilers open windows.** Run them with `DISPLAY` and `WAYLAND_DISPLAY` unset (`env -u DISPLAY -u WAYLAND_DISPLAY heaptrack ...`), read with `heaptrack_print`.
- **Emulators.** Start your own instance (recipes: `docs/reverse-engineering/emulator-recipes.md`): PPSSPP with its own `HOME`, `XDG_CONFIG_HOME`, debugger port and display; RPCS3 with its own config copy. Never attach to another member's. PPSSPP returns zeroed framebuffers unless its config has `[Graphics] SoftwareRenderer = True` (`scripts/ppsspp-start.sh` sets it; a hand-written config must). RPCS3 comparison frames need `Write Color Buffers: true` in the private `config.yml` **and** `custom_configs/config_BCES00664.yml` (off, the original reads about 1.74x too bright).
- **Ghidra.** The GUI bridge is shared on `127.0.0.1:8089`. Pass `program=` explicitly on every call (by path, e.g. `/pulse/BOOT-psp-pulse-usa.BIN`, `/hdfury/EBOOT-ps3-hdfury-eu.elf`); never call `switch_program`. Headless `analyzeHeadless` works when no GUI is up.

## Waiting on an emulator capture

Recipes: `docs/reverse-engineering/emulator-recipes.md`.

- **Never poll a capture log for a success marker** (it cost about 9.6 h on 2026-10-08).
- **Run the capture in the foreground under the watchdog**, timeout under 600000 ms: `uv run --with evdev python3 scripts/emu-run.py --child-status <log-dir>/status.json -- python3 scripts/<capture>.py ...`. It exits within 150 s of the last output (`STALLED ... child stage`) and at 570 s overall; an RPCS3 `Session` stops itself with `STALLED at <stage>`. Read the stage; do not retry unchanged.
- **Longer than ~9 minutes**: `emu-run.py --bg --status F -- ...`, then `emu-run.py wait F` in a loop (returns 4 while healthy).
- **Keep the emulator running between captures**: start once (`rpcs3-drive.py serve`, `scripts/ppsspp-start.sh`), set `OAG_RPCS3_ATTACH=1`, drive the next state from the current one. Boot fresh only when the measurement needs the boot. Build your private tree with `scripts/emu-env.sh`.
- **Stop emulators by pid**: `rpcs3-drive.py stop` (never `--all`; never `pkill -x rpcs3`), `ppsspp-start.sh <lane> stop`.
- Save states live only under `data/saves/<title>/`, never committed.
- `rpcs3-drive.py serve` needs a short `XDG_CACHE_HOME` (e.g. `/tmp/oag-<lane>-cache`; else `AF_UNIX path too long`) and its `uv` run with `UV_CACHE_DIR=~/.cache/uv UV_OFFLINE=1`.

## Build and gate

- Build once; avoid `--release` unless needed. On `No space left on device` or `rustc-LLVM ERROR: IO failure on output stream`, **stop and report**; do not retry.
- **Run a new or changed test under a memory cap first**: `systemd-run --user --scope -q -p MemoryMax=8G -p MemorySwapMax=0 cargo nextest run -p <crate> <test>` (an unfinished test once hit 51 GB and the OOM killed two members and the lead).
- Gate before reporting, always through the shared lock, with the **affected** gate:

  ```sh
  flock -o "$HOME/.cache/oag/gate.lock" just gate-affected
  flock -o "$HOME/.cache/oag/gate.lock" env OAG_REQUIRE_GAME_DATA=1 just test-data-affected 2>&1 \
    | tee /home/topaxi/projects/OpenAntiGrav/data/scratch/<lane>/test-data.log
  ```

  Both run every check in full and the tests that link your change: the packages that link a changed one, narrowed to the test modules that do (`docs/architecture/affected-gate.md`). **Quote the printed `affected:` selection in your report.** The root `Cargo.toml`, the toolchain or `.cargo/` selects the full suite by itself, and a change to `oag-core` or a simulation crate selects most of it; that is expected. The lead runs the full pair per merge batch; run it yourself only if the brief says so.

  **Which GPU the tests run on.** Use the discrete GPU by default; the `gpu-vram` group (CLAUDE.md, "The test-data suite") caps game-spawning tests at 4 at once. A SIGSEGV at exit means a GPU-touching thread outlived `main`: register it with `oag_thread::gpu_workers`, do not switch GPU. Time ceilings are judged on the full-size machine: on a smaller one a red that is only `check-test-budget` time is green.

  **wgpu's GL backend is best effort** (maintainer, 2026-10-15). It must never block the default backends (Vulkan, Metal, Dx12, WebGPU): switch a feature GL cannot do off on GL and name it in the loader report. Do not gate on a GL run.

  **A render bench takes the same lock itself** (`just bench-render`, `docs/tools/render-bench.md`): never wrap it in `flock`, never run it inside a gate or a gate inside it (same lock twice deadlocks), never set `OAG_BENCH_NO_LOCK=1`. Quote the load average its result file records.

  Check the exit status **in the same Bash call as the pipeline** (each call is a fresh shell): append `; echo "rc=${pipestatus[1]}"` (zsh) or `${PIPESTATUS[0]}` (bash) to the gate command. Never pipe `test-data` into `tail` (it masks the exit code). **The gate may sit several minutes before it starts** because another member holds the lock; wait it out and never run the gate outside the lock.
- **Gate your branch as it is; do not merge `main` into it first** (maintainer, 2026-10-12). Merge `main` only when the lead sends you back for a conflict or red merge, then gate again.
- A docs/`handover/`/`HANDOVER.md`-only lane runs `just check-docs` (plus `check-names`, `check-handover` when those files moved) instead of the gate.
- **A commit after your last gate that touches any `.rs` file, even a doc comment, needs `just gate-affected` re-run before you report** (a "docs plus one test line" commit once pushed a file over `check-size`'s ceiling and `main` went red).
- The brief states the baseline failure count; anything red beyond it is yours.
- **Never end a turn waiting for a background job to notify you**; no member has ever been woken that way. Run long commands in the foreground with a 600000 ms timeout, or poll the output file.
- **No fine-grained Monitors** (maintainer, 2026-10-19): each event wakes you and your idle turn wakes the lead. Watch only the whole run or gate finishing, or poll at 10 minute intervals or more.

## Commits

- **A `git diff`/`git show` whose output is applied, parsed or saved takes `--no-ext-diff`** (the maintainer's `diff.external = difft` prints a structural view, not a patch). Never change that config. `git apply -R` also fails on a file later commits touched; use `--3way` or `git checkout A -- <path>`.
- **Read `git diff --cached --stat` before every commit** and unstage what you did not mean to change (a commit once silently deleted six examples that docs cite).
- Commit early and often. If pinentry blocks, `git commit --no-gpg-sign`. End every message with a `Co-Authored-By:` trailer naming your own model; never a `Claude-Session:` line or other session link (maintainer's rule).
- Never merge into main; the lead merges.
- Committed golden hashes are never edited to make a test pass. If behaviour legitimately moves, regenerate from your tree in a **separate** commit naming the behaviour change.

## Project rules that bite members

- **No compatibility re-exports.** No crate is published: when you move an item, update every call site in the same change; no `pub use old::path` (maintainer).
- **The maintainer may type into your pane directly.** Act on it like the brief and name it as the maintainer's instruction in your report.
- **Never invent what the assets author, and label provenance only where in doubt** (CLAUDE.md, both rules in full, including the 2026-11-02 ruling).
- **The lead's gate has the mod examples and official mods, and they change the menus**: RACE CAMPAIGN opens a CAMPAIGNS picker, RACEBOX gains REMIX (`oag.remix`). A test that drives the menus must pass both with and without them, or pin `OAG_MODS` to an empty directory (a branch once passed without the examples and failed in the lead's gate).
- **A mod API change updates the mod author skill in the same change** (CLAUDE.md; maintainer, 2026-10-30), also when how a mod is built, tested or run changes. Commit it on your `assets/mods/examples` submodule branch and say in your report whether it needed a change.
- **Artificial load runs under the gate lock.** A lane that loads the machine on purpose (`stress-ng`, `yes`, a parallel build used as load) does it inside `flock -o "$HOME/.cache/oag/gate.lock"`, and kills its generators by PID before releasing the lock. On 2026-10-29, unlocked generators pushed the lead's full gate to 621 s against its 450 s ceiling.
- **Pulse PS2 inherits what PSP measured** (maintainer, 2026-10-29). Where a PS2 value or behaviour is invented or chosen because PS2 is unmeasured, use PSP's measured law, labelled **inherited from PSP**, with no PS2 confidence. Never gate a measured PSP law off on PS2 and leave the old stand-in. PS2-measured values and PS2-authored disc data stay.
- **A weapon change updates `docs/gameplay/weapon-lineage.md` in the same change** (law, tuning, AI use, visuals or sound): move the matching cell (measured with evidence link, inherited from X, chosen, absent, not investigated); a new `Weapons`/`WeaponModels` field or effect trigger gets its row in "Checked axes". `just check-weapon-lineage` checks only part of this (maintainer, 2026-10-19: a Pulse law ran on HD with HD's numbers and wrecked the field).
- **A 2048 finding is checked against Omega, and the reverse, in the same change** (CLAUDE.md); say which of its four outcomes in your report.
- **Keep a reader change from switching on what your lane did not measure.** If a fix to a shared lookup (bank names, a table, an effect list) makes cues, effects or draws resolve beyond your lane, scope it to your lane's caller (a hashed bank-name lookup once turned on every 2048 sound cue and a perfect-lap announcement played mid-lap). "Load report went from 42 missing to 6" is a warning, not a win.
- Judge as a player would: more than one frame, at the size a player sees, read with the Read tool. A green test is not a picture.
- A new front-end screen ships with mouse/pointer support in the same change.
- Findings go into `docs/` and the lane's `handover/` thread, not only scratch. Strike what closed; delete a thread file once it has fully landed. A new thread needs `status`, `titles`, `updated`, `summary` (`HANDOVER.md`, "Open threads"); when you change what is open, update those too. Nothing outside `handover/` and `HANDOVER.md` links into `handover/`.
- **No tracked file may cite `data/scratch/`** (maintainer, 2026-10-09): scratch is deleted after a lane merges. Put the evidence in the doc (numbers, table, reproducing command); a tool worth keeping goes to `scripts/`; a reference frame worth keeping goes to `data/reference/`.
- **Never commit `docs/overview/status.md`, and do not run `gen-status` or `check-status`.** `gate-affected` leaves `check-status` out; the lead regenerates the file per merge and the full `just` checks it. This overrides any agent file that says otherwise.
- **Never write a compare-page manifest (`data/reference/compare-page/*.toml`) from your worktree**: `data/` there is a symlink into the main checkout, and a manifest pointing into a reaped worktree broke the maintainer's page (2026-10-21). Keep re-made pairs and manifests in scratch and give the lead the commands to adopt them.
- **Do not run `scripts/compare-rerender-ours.py` on `data/reference/compare-page` from a worktree**: it rewrites the shared pairs with your branch's binary. Copy the pair dirs to `data/scratch/<lane>/pairs/` and call its `rerender(pair_json, <worktree>/target/debug/oag-game)` on the copies; the lead re-renders the page after merging.
- **Steam Deck builds from a worktree**: `data/appimage` must be a real directory, not the `link-data` symlink (the container build breaks on it). Jobs started detached over ssh die when the session closes; run them in the foreground.
- **The original's Autopilot is not a physics reference** (maintainer, 2026-10-20). Never measure a physics law, jump, landing, wall response or speed curve off an Autopilot run. Drive the original with player input (held throttle, scripted stick via the emulator's virtual pad), run ours under the same script, aligned by track position. Autopilot is fine for getting somewhere (a mid-pack screenshot position, a state to start from).
- No em or en dashes in anything you write; no decorative separator comments.

## Report

Write full findings to `/home/topaxi/projects/OpenAntiGrav/data/scratch/<lane>/report.md`: commits per step, the gate result you watched finish (counts and exit status), any regression gate quoted before and after, screenshot paths, what is still open. Quote your brief's claim and say whether it came out true, false or unknown, with the acceptance evidence. Return ONLY that path plus at most 15 lines (long returns can silently fail to arrive). A negative result reported honestly is a good outcome; "I could not determine X, here is its address" is a result.

**Send each report once: as the final text of your turn, not with `SendMessage`.** The lead receives that text when your turn ends. A `SendMessage` report followed by a closing summary reaches the lead twice. Use `SendMessage` only mid-turn, for a question you must wait on or a blocker. In that case, end the turn with one line, for example "Waiting on the lead: spu-objdump." The same applies to replies to the lead: the reply is your turn's final text.

**Your report is the end of the lane.** After sending it, do not commit, merge main or edit your worktree unless the lead sends you back; the lead removes the worktree as soon as the report arrives. Finish everything first.

**If a message wakes you after your report**, your worktree is gone or belongs to someone else. Answer in text only: never `cd` into another lane's worktree, rename a branch or commit. If it asks for more work, say so and stop; the lead spawns a fresh member.
