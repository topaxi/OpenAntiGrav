# Compare page: the original and ours, side by side, for a human to judge

`just compare-page` builds one static web page of review pairs: a screenshot of
the original running in its emulator and one of this engine, same ship, same
circuit, same state. Views per pair: a **split** slider (drag the handle, works by
touch), an **onion skin** opacity slider, **side by side**, and a precomputed
**difference** (twice the absolute difference). Keys: up/down or j/k change pair,
1-4 change view, left/right move the split. A pair that has an original, a
"before" and an "ours" offers each combination.

```sh
just compare-page                 # builds data/compare/index.html from every manifest in data/reference/compare-page/
just compare-page --serve         # the same, then serves it on all interfaces, port 8097
just compare-page data/reference/compare-page/live.toml --out /tmp/page --serve --port 8123
```

`--serve` prints every LAN address, so the desktop, a Steam Deck or a phone opens
`http://<address>:8097/`. The page and its images stay under `data/` (gitignored):
**nothing captured or rendered is ever committed.** Needs ImageMagick (`magick`),
the same as [frame comparison](frame-compare.md).

## The manifest

TOML, one `[page]` table (title, intro) and an `[[entry]]` per pair; paths are
relative to the manifest. A directory argument reads every `*.toml` in it, so a
lane drops its own file next to the others. This is a developer file under `data/`
that the game never loads, so it carries no `[toml-schema]` header
([ADR-0064](../architecture/adr/0064-project-toml-files-declare-a-toml-schema.md) is for
files a player or modder edits); a wrong shape is refused by name instead
(missing `id`, an image that does not exist, fewer than two images, an unknown key).

| Key | Meaning |
| --- | --- |
| `id`, `game`, `title` | required; `game` is the title and region (`Wipeout HD (EU)`) and groups the list |
| `platform` | the console the original ran on (`PS3`, `PSP`, `PS4`, `Vita`); required to publish, a warning otherwise (see Credits) |
| `original`, `before`, `ours` | image paths; at least two |
| `track`, `craft`, `state`, `setting`, `environment` | the caption: circuit, ship, what moment and how the pose was matched, emulator and size, and the environment line `scripts/compare_env.py manifest` writes from `pair.json` (profile, output size, every setting that changes the picture) |
| `note` | free text, shown under the caption (what is not matched) |
| `not_captured` | why there is no original; shown as a warning |

Build time normalises each pair with ImageMagick: a black letterbox is trimmed from
the original, and every image is brought to one size. The aspect is never stretched
(a different aspect is fitted and padded, so a mismatch stays visible). **An entry
with no `original` is shown as before and after, and `not_captured` says why:** one
of ours is never put in the original slot.

`just compare-seed` rebuilds `seed.toml` from the HD lanes' reference sheets
(`scripts/compare-seed.py` splits the 3x3 composites into one entry per column).
`live.toml` is hand-written from the captures below.

## Credits and the rights-holder statement

ADR-0074 requires both on every published page, so every page carries them:

- **A credit under each image** on view: `<game> - <platform> - Original game` or
  `... - OpenAntiGrav engine` (`before` reads `OpenAntiGrav engine, before`), built from
  the entry's `game` and `platform`; in side by side it is the figure caption. An entry
  with no `platform` is **warned about by file and entry** in a local build and shows
  "platform not recorded"; `--strict`, and `--publish` always, **refuse the build naming
  the manifest** instead of guessing.
- **The rights-holder statement** in the page footer. Its text is read at build time from
  `docs/overview/legal.md`, "Screenshots and rights holders", the one source, and the
  build refuses if that no longer equals the statement quoted in ADR-0074 word for word
  (markdown emphasis and line breaks ignored). `python3 scripts/compare-page.py
  --check-statement` runs only that check; `just check-docs` runs it, so the gate fails on
  drift. The ADR is immutable, so a mismatch is fixed in `legal.md`.

## Environment parity: the profile and the checks (2026-10-20)

**The set from before 2026-10-20 is archived** (the maintainer's request: start fresh, make the original's and
our environment as close as possible *before* anything renders). The old pairs were taken at whatever size the
emulator window had (PPSSPP 960x544, RPCS3 1760x990 with black bars, PCSX2 512 lines), with the original's race
clock running on from the state and ours at its own tick 300, so the GO board, the record panel and the HUD clock
differed by construction. None of them is a reference now.

`scripts/compare-environment.toml` is the one **per-title environment profile**; `scripts/compare_env.py` reads
it. A pair script builds a `compare_env.Pair`, records what each side actually ran with, and `Pair.finish` writes
`pair.json` and **refuses** (exit 3, every mismatch named) when a check fails. `python3 scripts/compare_env.py
manifest OUT.toml DIR...` turns the non-refused `pair.json` files into a page manifest whose `environment` line
the page shows under each pair.

| Platform | Output | Original's emulator settings (in the profile) | Ours |
| --- | --- | --- | --- |
| Pulse PSP (and Pure PSP, same values) | 480x272, the PSP's own | PPSSPP v1.20.4 software renderer, `InternalResolution = 1`, `AnisotropyLevel = 0`, no post shader | `--size 480x272`, aspect `psp` |
| HD/Fury | 1920x1080 | RPCS3 Vulkan, `Resolution 1280x720`, `Resolution Scale 150`, `Output Scaling Mode Nearest`, `MSAA Disabled`, anisotropic override 0, `Write Color Buffers: true` in `config.yml` **and** the game's custom config | `--size 1920x1080`, aspect `wide` |
| Pulse PS2 | **960x720, a chosen pick** (the maintainer asked for 720p; the saved state is the game's own 4:3, so 720 lines at 4:3, adjustable in the profile) | PCSX2 OpenGL hardware renderer (the software renderer needs a Vulkan present queue Xvfb does not give), `upscale_multiplier 2`, `ScreenshotSize 1` (internal, aspect corrected, 1365x1024), then ONE Lanczos resize to 960x720 | `--size 960x720`, aspect `ps2`, rendered natively |
| Omega | 1920x1080 | shadPS4, no race reached, no pair script | not run |
| 2048 | 960x544, the Vita's own | Vita3K 1x, no race reached, no pair script | not run |

**Ours is the same on every platform** (`[ours_defaults]`): every render setting with an `original` value is set
to it (shadows `original`, model and texture detail `original`), upscaler, FSR, SMAA, MSAA, motion blur and screen
filter off, render scale 100, anisotropic filtering off (no original forces it), the boost fov kick off (our own
effect), fov 100 %, an ungraded capture at exactly `--size` (no `--presented`), the HUD on (the original cannot
switch it off), and the camera is the original's own pose and fov. The profile is written into the pair's private
`settings.toml` **and** passed as flags where a flag exists, and the file the run rewrites is **read back** and
compared with the profile (`read_back`).

**State parity, recorded in `pair.json` on both sides:** circuit and layout file (our own start on each layout
against the original's grid pose, `--layout auto`; refused beyond 25 units or 5 degrees), game mode and speed
class (declared by the state's file name or sidecar; **not read from RAM**), team, **race clock** (below), ship
position and heading **at the rendered tick** (measured from a second, `--trace-out` run of the same command,
within 0.5 units and 1 degree, so a pose fed to ours is not echoed back as evidence), and the field of view. The
binary that rendered ours is recorded by hash.

**The race clock** is the state difference that mattered most: the original's HUD clock runs on from the state
(PPSSPP 40 s after GO on the grid states) and its GO board fades and its record panel counts down with it, so ours
runs `GO_TICK = 276` ticks (our countdown) plus the original's clock times 60 (`--ours-ticks`). The original's clock
is read out of its RAM: PPSSPP f32 seconds at `0x08B344B0` (the CPU is stopped straight after the load so it stands
still), HD f32 at craft `+0x7014` (read in the same pause as the pose), PS2 f32 at EE `0x70EE9C` of the state file
(the HUD reads 0.19 s under it). With it the GO board, the record panel and the HUD clock match on the Pulse PSP and HD
test pairs. **A stationary craft in ours yaws about 0.26 degrees a second on HD (Talon's: 26 degrees over 6,000
ticks), the original's does not** - a simulation lead, not an environment one - so `rpcs3-hd-pair.py` corrects the
yaw it hands to `--pose` until the heading measured at the rendered tick is the original's (`pair.json`
`ours.yaw_corrections`). Pulse does not drift.

## Capturing a pair per title

Every recipe runs in its own display and data directory, muted, and is stopped by PID. A pair is only a pair when
its `pair.json` has no failed check.

| Title | Status | What matched | How |
| --- | --- | --- | --- |
| Pulse PSP | **works**, about 1.5 min per circuit (a mid-race **race shot** with the whole field: see "Race shots" below) | everything in the checks above | `PPSSPP_EXTRA_INI=<file from compare_env.py ppsspp-ini pulse-psp> scripts/ppsspp-start.sh <lane> <display> <port> <image>`, then `scripts/psp-pulse-pair.py --port P --home $PP_HOME --out D --track NN_Track --state <ppst>` (it resizes the SDL window to 960x544 for `psp-state.py`'s pause-menu clicks, loads the state, stops the CPU, resizes the window to the native 480x272 and crops the Xvfb root at the window origin: nothing is scaled), or `scripts/compare-pulse-psp-batch.py --out D` for all 24 kept states. No state yet: `psp-drive.py menu --unlock --track-down N --any-track` (about 105 s from a fresh profile, N from [the table](../reverse-engineering/ppsspp-debugger.md#every-circuit-not-three-the-dev-unlock-byte-2026-09-29)), then `psp-state.py save 1 --keep`. Full pipeline: [frame comparison](frame-compare.md) |
| HD (PS3) | **works**, about 1.5 min per circuit | pose, camera, fov, clock | `export OAG_COMPARE_PLATFORM=hd-fury` before `scripts/emu-restore-state.sh` (so `rpcs3-drive.py` writes the profile's video settings into the private config **and** the game's custom config), restore the state with `--restart`, then `OAG_RPCS3_ATTACH=1 scripts/rpcs3-hd-pair.py --out D --team harimau --state <state>`: it resizes RPCS3's window to 1920x1080 (the default 1760x1072 window carries black bars), captures, and checks the frame is exactly 1920x1080. One state per circuit: `scripts/emu-hd-timetrial-state.sh <lane> <circuit-index> <team-rights> <classic 0\|1> <name>` (Track Select index 0 is Vineta K, 8 Talon's Junction; Harimau classic is 9 `right`s and a `down`). The state only names the circuit and craft: **the pair is taken after `Restart Race`, a few seconds into a fresh Time Trial**, so the clock is small and our drift small. `--team` carries HD's variant (`Assegai_c1` is the Fury Concept craft). |
| Pulse PS2 | **works for Moa Therma White; the camera is not matched** | pose, clock, circuit and layout | PCSX2 booted by `pcsx2-drive.py boot` with `OAG_COMPARE_PLATFORM=pulse-ps2` in the environment (its ini carries the profile's upscale and screenshot size), then `scripts/pcsx2-pulse-pair.py --slot S --state <p2s> --track 03_Track --out D --allow-own-camera`. Pose, craft and clock are read out of the state file's own EE RAM (`pcsx2_state.py`), the frame is PCSX2's GS screenshot of the loaded state. **The PS2 camera is not recovered**: a node whose position matches the PSP's follow geometry (11.64 units behind and 3.0 above the craft) is in the state, but its orientation convention differs from the PSP's (the pitch sign is opposite) and the 4:3 fov is unmeasured; rendering the node with the pitch flipped and a fov sweep gives only an ambiguous best fit (RMSE 0.184 at 48 degrees against 0.217 at 60), so the pair is refused unless `--allow-own-camera` is given, and then names the camera in its note. Only Moa Therma White has a state; more need a menu route for `pcsx2-drive.py` |
| Pure (PSP) | **not reachable, for now** | | `psp-drive.py menu` is Pulse's walk and stops at `the front end is in None`; `psp-trace.py --camera` breaks at Pulse USA's `Camera_UpdatePlayerView` (`0x0883c13c`), and Pure's counterpart is not located ([Pure camera rig](../ghidra/functions/psp-pure-usa/camera.md) is a static read of the rig only). Needs a hand walk to a grid, a save state and the camera node. The profile row `pure-psp` carries the PSP values |
| 2048 (Vita) | **partial** | nothing | `vita3k-drive.py race --screenshot` boots Vita3K on a private config, on its own weston and Xwayland display, and writes a frame; its fixed-timing tap walk never changed screen in the 2026-10-20 run and the frame was an intro cinematic, not a race. Profile row `2048`: 960x544 at 1x |
| Omega (PS4) | **partial** | layout only | shadPS4 0.20, the extracted base plus patch hard-linked into a game folder, a headless `sway` on the render node and its own `HOME` (recipe: [online lobby, part 3](../networking/online-lobby.md)). It reaches `MAIN MENU` unattended in about 45 s; `grim` takes the frame. No race was reached. Profile row `omega`: 1920x1080 |

**Re-making a pair in place.** When the renderer changes, `just compare-rerender [--only <substring>] [dir]`
(`scripts/compare-rerender.py`, default `data/reference/compare-page`) re-runs each pair's script with the arguments
recorded in its `pair.json` (`argv`): ours is therefore **always posed from the emulator's own savestate**, the ship
pose, camera and race clock are read out of RAM fresh and the pose row and heading correction derived from that
read, never taken from a stored `pair.json` or CSV. `theirs.png` is captured in the same run, every environment and
parity check runs again, and on success the pair's files are replaced; the old values become a record in the new
`pair.json` (`previous`), with `drift` (ship position and heading, camera, clock, the RMSE between the old and new
`theirs.png`) and a WARNING printed when the drift is beyond the profile's tolerance. A pair whose fresh run is
refused by a check is named, keeps its old files, and makes the exit status non-zero. The emulator has to be up
exactly as when the pair was made (the per-title rows above); the pair's own script is the only place the capture
is defined, so the command works for PSP, HD and PS2 alike. Then rebuild the page with `just compare-page`.

The fresh set lives in `data/reference/compare-page/` (`pulse-psp/`, `hd/`, `pulse-ps2/`, one directory per pair
holding `theirs.png`, `ours.png`, `side.png` and `pair.json`); the manifest is built with
`python3 scripts/compare_env.py manifest data/reference/compare-page/fresh.toml data/reference/compare-page/<dirs>`.

## Race shots: a mid-pack frame of a Single Race, weapons off (2026-10-20, `compare-race-shots`)

The grid and Time Trial pairs have one craft. A race shot is the player **in the pack of a Single Race**, flown by the
original's own Autopilot, with every craft of the field, the HUD's race state and the camera taken from the original's RAM
at the one stop the screenshot is taken at. `just compare-rerender` re-makes it from its savestate like any pair.

**Pulse PSP (works, measured).**

1. **Weapons off.** The Racebox page (`RACEBOX` > `CUSTOM RACE`) has the rows `RACE TYPE`, `SPEED CLASS`, `WEAPONS`, `AI
   DIFFICULTY` (screenshotted, 2026-10-20), and `WEAPONS` is editable in a Single Race (a Time Trial forces it off, Head to Head
   locks it). `psp-drive.py menu --single-race --weapons-off` sets it (`down` to `WEAPONS`, `right` once: On, then Off) and prints
   `g_weapons_enabled` (`0x08b30fa8`, a byte) once the race is up: it reads `0` ([shield.md](../ghidra/functions/psp-pulse-usa/shield.md)).
   **The control, measured:** `--weapons-on` (the row pinned to On) reads `1` at the same address, so the byte is the setting and not
   a constant. The pair script reads the byte out of the loaded state and **refuses** a state where it is not `0`.
2. **Autopilot.** `psp-race-field.py hunt` arms the player's Autopilot the way `psp-autopilot-rail.py` does (fire bit `0x1000`, timer
   `record+0x148` written huge); the original's driver then flies the race. It also works with weapons off.
3. **The hunt.** One conditional breakpoint on `Ship_UpdateCraft` (`0x08849618`) for the player's craft, once a tick; the update
   order is the player first and then slots 6 down to 0, so at that stop **all eight bodies are on the same tick boundary**. Per
   stop it reads the player's place (`entity+0xbd4`, 1-based, in the racer entity at `manager+0x78+4n`, not the craft), and only
   when that is in the wanted range the whole field and the camera node, and tests: other craft inside the camera's cone
   (`--half-fov`, `--max-dist` from the eye) and at least `--min-near` of them within `--near-dist` of the player. The player
   starts eighth and is on the grid's back for the first lap; with weapons off the autopilot reaches 6th to 5th only late in lap 1
   or in lap 2, and the field is spread out (the grid alone spans 344 units), so a hunt takes one to nine minutes and some
   races never match. **The camera's look direction is the negated third COLUMN of the recorded rows** (the node stores its rotation
   transposed), not the `cam_fwd` row: using the row put craft behind the camera "in view" in the first hunts. **The criteria are the brief's (place 3 to 6, craft close and in view); where a hunt had to settle for less,
   the state's note says so.**
4. **The state.** The pause menu cannot be opened while the debugger holds the CPU (its `paused()` reads the stop as the menu), so
   the hunt resumes, presses Escape at once and saves slot 3: the saved tick is a few frames after the match. The pair script
   reads the field again after loading, so nothing is trusted from the hunt. A state is `data/saves/pulse-psp/race-*.ppst` with a
   `.json` sidecar naming `mode single_race`, the class, the player's team and the layout.
5. **The pair.** `psp-pulse-pair.py --field --layout forward --team <player team> --track 16_Track --state race-*.ppst`: load,
   stop the CPU, `psp-trace.py --craft <player craft> --field-out field.json --camera --shot-every 2` captures at the first stop
   of the player's craft the player's row, the camera node, **every racer's body, team name, place, lap, lap time and best lap**
   (`psp-race-field.py` rows) and the screenshot. Ours is `oag-game --race --pose-from trace.csv --field-from field.csv
   --hud-state PLACE,LAP,LAP_S,RACE_S,KMH,SHIELD_PERCENT,BEST_S --ticks 0`:
   - `--field-from` (`race::Options::field`, `grid_teams`) keeps the Single Race grid's seven opponents alive, puts each on the
     pose its row carries (the same left/up/forward reading `--pose-from` uses, `oag_trace::replay::orientation_of_rows`) and flies
     the team the row names; the original's slot `n` is our opponent `n + 1`. The craft are drawn at rest: no tick runs.
   - `--hud-state` (`race::CaptureHud`) imposes the original's place, lap, lap time, race clock, speed, shield (as a percentage
     of the pool: PSP's maximum is `*(*(craft+0x6c) + g_skill_level*4 + 0x84)`, 135 here, so the 134.5 pool reads 99 %, and
     HD's is 140; ours is the handling file's) and best lap on the primary readout: all read from RAM, none
     derived. The lap on screen is `entity+0xacc - 1`.
   - Ours races with weapons off (`[race] weapons = "Off"` in the pair's private settings); the layout is **asserted** with
     `--layout` (the grid probe is a start-line test), and checked against the sidecar.
6. **Checks.** A race shot is refused unless **the frame is a pack**, measured on the captured RAM (`compare_env.pack_check`, recorded
   as `pair.json` `pack`): the player 3rd to 6th, at least two craft in the camera's cone (28 degrees half-angle, up to 110 units from
   the eye; HD: 45 degrees, 160 units, a 16:9 frame) and at least one within `--pack-near` (45) units of the player. The capture
   instant cannot be chosen (the state runs a few ticks before the CPU stops, PSP loads differ by up to about 90 units), so a miss is
   a refusal and a rerun; a rerender that lands off the pack keeps the old files and counts as failed. Everything in "Environment
   parity" except the race-clock phase (there is no run-up: ours renders at tick 0 and
   takes the clocks from `--hud-state`), plus `weapons off in the original's own RAM`, eight craft, a team name for each, and the
   pose and fov at the rendered tick. The re-capture drifts: the state loads, the CPU stops a few ticks after, so two captures of
   one state differ by about 4 ticks (10 units, 2.5 degrees in the first rerender); the pose, camera and field of each capture are
   from its own stop, so ours matches whichever stop it was.

**HD (works, one circuit).** The same four steps, with RPCS3's tools:

1. **Weapons off, Autopilot.** `scripts/emu-hd-race-walk.sh <lane> 8` cold-boots in the state-saving flavour (stub off, Compatible
   Savestate Mode) and walks `RACEBOX` (Single Race is the default), `down` x2 to `WEAPONS`, `right` (On to Off, the page was
   screenshotted: `RACE TYPE, SPEED CLASS, WEAPONS, AI DIFFICULTY, NUMBER OF PLAYERS`), Track Select index 8 (Talon's Junction),
   Ship Select. There is no stub in that flavour, so the Autopilot is armed through **`serve`'s memory, which can now be written**
   (`RemoteMem.write`, op `memw`: `ptrace_scope` is 1, only `serve` may touch the emulator's `/proc/<pid>/mem`):
   `*(ship+0x5edc)+0x200 |= 0x800`, `+0x158 = 10000`.
2. **The hunt.** `scripts/rpcs3-hd-race-hunt.py` polls guest memory without stopping the game (a few ms a read): the HUD's
   per-player record (found by matching the player's shield, `ship+0x5fa0`, bit for bit at `+0x48` with the pool maximum 140 at
   `+0x50` and the race's laps at `+0x10`; the heap moves per boot) for the place, and the eight bodies (`rpcs3_hd_field.py`) for
   a craft within `--near-dist` of the player and two in front. It then presses the PS menu's "Save Emulation State And Exit"
   (`emu-save-state.sh`), which writes the state about a second later and ends the emulator. The Autopilot passes the field in
   the first lap (8th at GO, 6th about 50 s in on `Novice`), so a match came on the first race.
3. **The pair.** `emu-restore-state.sh <lane> <state> --pause` restores and **pauses on the state's own tick** (`serve
   --pause-after-load`: the restored game otherwise runs on for the seconds the script takes), then `OAG_RPCS3_ATTACH=1
   scripts/rpcs3-hd-pair.py --field --team feisar --track 'Data\Environments\Talons_Junction\track.vex' --state <state>`:
   `rpcs3-drive.py capture --field --in-race --settle 0` takes the screenshot, the camera (from the push buffer, two shots 0.12 s
   apart, the first used), the player's body and **every craft's body, team string and lap clock, plus the HUD record**, in the
   same pause (`--first-at-pause`: the first shot is taken where the restored game stands, with no run-up, so the frame, pose, field and HUD record are
   one instant; the second shot runs 0.12 s, for the camera pick). Ours gets `field.csv` with the player as slot 0 (an exact pose, so no tilt is left uncarried) and the seven
   opponents on `--field-from`, `--camera-pose`, `--hud-state`, `--ticks 0`, weapons off in the private settings
   (`compare_env.set_ours_weapons_off`).
4. **The HUD record's word order differs between states**: `(lap, laps, place)` at `+0xc`, `+0x10`, `+0x14` in the hunt's boot, and
   `(place, laps, lap - 1)` in one read taken after the game had run on (`6, 3, 1`, which fits that order but matched no frame: no shot was taken at that instant). `hud()`
   decodes both and refuses (`place` is None, the pair stops) when both or neither is a valid reading. Not understood: it is a read
   of one structure at one address in two boots. The restored-and-paused state reads `(2, 3, 6)`, which matches the frame.

`just compare-rerender` re-makes a race pair only if the emulator is in the state it was made from: restore the state with
`--pause` first (the capture lets the game run for 0.12 s, so a second rerender needs another restore). Measured drifts between two
captures of one state: PSP 15 to 90 units (the CPU stops a few ticks after the load; two direct runs agreed exactly, a rerender did not),
HD 14.0 to 14.2 s on the lap clock in five restores and 19.7 s in one (the pause after the load did not hold).

**What is not matched in a race shot** (the pair's caveat says it): the exhaust plume and the engine's light (a craft at rest in
ours; stage 3, a frozen warm-up that gives every craft its trails, is not built), the Autopilot icon and its counter (the pickup
row), HD's `Absorb` and `DAMAGE` strips and its best-lap line, the opponents' own trails and any weapon or effect in flight, and the scene objects that animate (the billboards).

## Publishing is locked

`just publish-compare` uploads the built `data/compare/` with `wrangler pages deploy`
to a **separate** Cloudflare Pages project (`openantigrav-compare`, never the main
site's `openantigrav`; the script refuses if the two names are ever equal). It runs
only from the workstation, never in CI, and nothing in `.github/` calls it.

It **refuses to run** unless the screenshot-policy ADR,
[ADR-0074](../architecture/adr/0074-curated-screenshots-may-be-published-no-content-may-be-shipped.md),
has a `## Status` line that **starts with** `Accepted` (markdown emphasis ignored). Its number is one
constant, `POLICY_ADR` in `scripts/compare-page.py`. A missing or non-Accepted ADR prints the reason and
exits 2. The lock is checked before `wrangler` is looked up, so the refusal is what you see.
`just publish-compare <dir>` rebuilds the page strictly from the curated manifest directory it is
given first, so a missing `platform` refuses it before anything uploads. The directory is required:
the seed and live manifests are developer material, ADR-0074 wants every published image picked by
a person, and a default would publish them. `python3 scripts/compare-page.py --check-publish-lock` prints the verdict without publishing; with
ADR-0074 Accepted it reads `unlocked`. The maintainer runs the command.

A curated page should carry only frames the policy covers; the seed and live
manifests are developer material, not a publishing set.
