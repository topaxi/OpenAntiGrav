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
test pairs. **A craft resting after GO turns, in the original as in ours** (bank-to-yaw; ours unposed lands within 0.101
degrees of the original's heading on all 24 Pulse PSP pairs and 0.694 on all 13 HD pairs, see
[grid state](../physics/grid-state.md#a-resting-craft-after-go-turns-in-the-original-too-2026-10-20-resting-yaw)).
Posing ours on tick 0 at the original's already-turned pose and then simulating the clock again turns it twice, so
`rpcs3-hd-pair.py` and `psp-pulse-pair.py` correct the yaw they hand to ours until the heading measured at the
rendered tick is the original's (`pair.json` `ours.yaw_corrections`, `ours.heading_corrections`).

## Capturing a pair per title

Every recipe runs in its own display and data directory, muted, and is stopped by PID. A pair is only a pair when
its `pair.json` has no failed check.

| Title | Status | What matched | How |
| --- | --- | --- | --- |
| Pulse PSP | **works**, about 1.5 min per circuit | everything in the checks above | `PPSSPP_EXTRA_INI=<file from compare_env.py ppsspp-ini pulse-psp> scripts/ppsspp-start.sh <lane> <display> <port> <image>`, then `scripts/psp-pulse-pair.py --port P --home $PP_HOME --out D --track NN_Track --state <ppst>` (it resizes the SDL window to 960x544 for `psp-state.py`'s pause-menu clicks, loads the state, stops the CPU, resizes the window to the native 480x272 and crops the Xvfb root at the window origin: nothing is scaled), or `scripts/compare-pulse-psp-batch.py --out D` for all 24 kept states. No state yet: `psp-drive.py menu --unlock --track-down N --any-track` (about 105 s from a fresh profile, N from [the table](../reverse-engineering/ppsspp-debugger.md#every-circuit-not-three-the-dev-unlock-byte-2026-09-29)), then `psp-state.py save 1 --keep`. Full pipeline: [frame comparison](frame-compare.md) |
| HD (PS3) | **works**, about 1.5 min per circuit | pose, camera, fov, clock | `export OAG_COMPARE_PLATFORM=hd-fury` before `scripts/emu-restore-state.sh` (so `rpcs3-drive.py` writes the profile's video settings into the private config **and** the game's custom config), restore the state with `--restart`, then `OAG_RPCS3_ATTACH=1 scripts/rpcs3-hd-pair.py --out D --team harimau --state <state>`: it resizes RPCS3's window to 1920x1080 (the default 1760x1072 window carries black bars), captures, and checks the frame is exactly 1920x1080. One state per circuit: `scripts/emu-hd-timetrial-state.sh <lane> <circuit-index> <team-rights> <classic 0\|1> <name>` (Track Select index 0 is Vineta K, 8 Talon's Junction; Harimau classic is 9 `right`s and a `down`). The state only names the circuit and craft: **the pair is taken after `Restart Race`, a few seconds into a fresh Time Trial**, so the clock is small and the double-counted resting turn small. `--team` carries HD's variant (`Assegai_c1` is the Fury Concept craft). |
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
