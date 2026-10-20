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
| `track`, `craft`, `state`, `setting` | the caption: circuit, ship, what moment and how the pose was matched, emulator and size |
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

## Capturing a pair per title

Every recipe runs in its own display and data directory, muted, and is stopped by
PID. A pair is only a pair when the caption says what matched.

| Title | Status | What matched | How |
| --- | --- | --- | --- |
| Pulse PSP | **works**, 2-3 min per circuit | ship pose, camera, clock not | `psp-state.py load` a Time Trial grid state (about 3 s, `data/saves/pulse-psp/time-trial-<circuit>-venom-grid.ppst`), then `scripts/psp-pulse-pair.py --port P --home $PP_HOME --out D --track NN_Track [--reversed] --state <state>`: it restores, waits out PPSSPP's "State loaded" toast, captures four ticks with `psp-trace.py --camera --shot-every 2`, crops the 960x544 window out of the Xvfb shot and renders ours with `oag-game --race --pose-from trace.csv --pose-tick 0 --ticks 300 --size 960x544` (`--ours-ticks`: the saved state is after GO, and tick 0 would draw the pre-countdown gantry, a dark board, and no engine glow). No state yet: `psp-drive.py menu --unlock --track-down N --any-track` (about 105 s from a fresh profile, N from [the table](../reverse-engineering/ppsspp-debugger.md#every-circuit-not-three-the-dev-unlock-byte-2026-09-29)), then `psp-state.py save 1 --keep`. Which layout the entry loaded is read from the start position against `oag-game`'s own start on `NN_Track` forward and reversed (five White entries load the reversed file, `docs/formats/track.md`). Full pipeline: [frame comparison](frame-compare.md) |
| HD (PS3) | **partial** | camera pose and fov | Restore a Time Trial state (`emu-restore-state.sh <lane> data/saves/hd-fury/hd-timetrial-talons-harimau-classic-start.SAVESTAT.zst`, 5-80 s), then `OAG_RPCS3_ATTACH=1 rpcs3-drive.py capture --shots 2 --interval 0.12 --load 0 --team harimau`; `capture` needs two frames to pick the camera, and an attached run skips the menu walk. Ours: `oag-game --race --track <circuit>.vex --team HARIMAU --camera-pose=<9 numbers> --camera-fov <deg> --size 1760x990 <image>`. The craft pose is not carried, so the craft sits a little off, and a campaign start would put the original in grid slot 8 against ours in slot 1: use a Time Trial. `ps3_pose.command_line` printed a command that no longer ran (`.rcsmodel`, image before `--race`); fixed 2026-10-20 |
| Pulse PS2 | **works**, ship pose only | ship pose; our camera is our own | A `.p2s` is a zip holding EE RAM, so `scripts/pcsx2_state.py find STATE.p2s --write --circuit "..."` reads the craft pointer out of the file (the one struct with a body pointer at `+0x1ec`, a basis equal to the body's, and a camera rig pointing at the body) and records it, with the circuit, in `STATE.p2s.json`; `pcsx2-trace.py --state-file STATE.p2s` reads it in place of `--craft`. Pair: `scripts/pcsx2-pulse-pair.py --slot S --state <p2s> --track NN_Track` on a booted private instance (`OAG_PCSX2_DATA`, `OAG_PCSX2_DISPLAY`): restore, GS screenshot, three ticks of pose, ours from `--pose-from`. The saved state's circuit was not recorded; it is Moa Therma White (03_Track **forward**, Single Race, the player in grid slot 8 of 8: the start is 1.4 units from the *reversed* layout's start line but faces the forward direction, so a reversed render says WRONG WAY) |
| Pure (PSP) | **not reachable, for now** | | `psp-drive.py menu` is Pulse's walk and stops at `the front end is in None`; `psp-trace.py --camera` breaks at Pulse USA's `Camera_UpdatePlayerView` (`0x0883c13c`), and Pure's counterpart is not located ([Pure camera rig](../ghidra/functions/psp-pure-usa/camera.md) is a static read of the rig only). Needs a hand walk to a grid, a save state and the camera node |
| 2048 (Vita) | **partial** | nothing | `vita3k-drive.py race --screenshot` boots Vita3K on a private config, on its own weston and Xwayland display, and writes a frame; its fixed-timing tap walk never changed screen in the 2026-10-20 run and the frame was an intro cinematic, not a race |
| Omega (PS4) | **partial** | layout only | shadPS4 0.20, the extracted base plus patch hard-linked into a game folder, a headless `sway` on the render node and its own `HOME` (recipe: [online lobby, part 3](../networking/online-lobby.md)). It reaches `MAIN MENU` unattended in about 45 s; `grim` takes the frame. Ours is `oag-game <extracted> --menu-page main`, **our own menu**, so the pair is a layout comparison. No race was reached |

The pairs in `live.toml` are Pulse PSP, HD (a Time Trial start, Harimau classic) and Omega's menu; `pulse.toml` holds the Pulse PSP circuits and the first Pulse PS2 pair.

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
