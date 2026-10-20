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
| `id`, `game`, `title` | required; `game` groups the list |
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

## Capturing a pair per title

Every recipe runs in its own display and data directory, muted, and is stopped by
PID. A pair is only a pair when the caption says what matched.

| Title | Status | What matched | How |
| --- | --- | --- | --- |
| Pulse PSP | **works** | ship pose, camera, clock not | `psp-drive.py menu` to the grid, `psp-trace.py --camera --shot-every 2`, then `oag-game --race --pose-from trace.csv --pose-tick 0 --size 960x544`. Crop the 960x544 window out of the Xvfb shot. Full pipeline: [frame comparison](frame-compare.md). A saved state (`psp-state.py load`) did not restore on a fresh instance in the 2026-10-20 run; the cold `menu` walk (about 90 s) did |
| HD (PS3) | **partial** | camera pose and fov | `rpcs3-drive.py capture --shots N` writes a shot and a camera per pose, then `oag-game --race --track <circuit>.vex --team FEISAR --camera-pose=<9 numbers> --camera-fov <deg> --size 1760x990 <image>`. The craft and the opponents are not carried: the original player was in grid slot 8 of 8, ours starts in slot 1. 210 s per capture. `ps3_pose.command_line` printed a command that no longer ran (`.rcsmodel`, image before `--race`); fixed 2026-10-20 |
| Pulse PS2 | **partial** | nothing yet | `pcsx2-drive.py boot` then `state load 1` then `shot`, in a private `OAG_PCSX2_DATA`, about 3 minutes, gives a race-start frame of the original from a saved state. No pose capture (`pcsx2-trace.py` needs a per-state craft pointer) and the saved state's circuit is not recorded, so no pair was made |
| Pure (PSP) | **not reachable, for now** | | `psp-drive.py menu` is Pulse's walk and stops at `the front end is in None`; `psp-trace.py --camera` breaks at Pulse USA's `Camera_UpdatePlayerView` (`0x0883c13c`), and Pure's counterpart is not located ([Pure camera rig](../ghidra/functions/psp-pure-usa/camera.md) is a static read of the rig only). Needs a hand walk to a grid, a save state and the camera node |
| 2048 (Vita) | **partial** | nothing | `vita3k-drive.py race --screenshot` boots Vita3K on a private config, on its own weston and Xwayland display, and writes a frame; its fixed-timing tap walk never changed screen in the 2026-10-20 run and the frame was an intro cinematic, not a race |
| Omega (PS4) | **partial** | layout only | shadPS4 0.20, the extracted base plus patch hard-linked into a game folder, a headless `sway` on the render node and its own `HOME` (recipe: [online lobby, part 3](../networking/online-lobby.md)). It reaches `MAIN MENU` unattended in about 45 s; `grim` takes the frame. Ours is `oag-game <extracted> --menu-page main`, **our own menu**, so the pair is a layout comparison. No race was reached |

The pairs in `live.toml` are the first three rows plus Omega's menu.

## Publishing is locked

`just publish-compare` uploads the built `data/compare/` with `wrangler pages deploy`
to a **separate** Cloudflare Pages project (`openantigrav-compare`, never the main
site's `openantigrav`; the script refuses if the two names are ever equal). It runs
only from the workstation, never in CI, and nothing in `.github/` calls it.

It **refuses to run** until the screenshot-policy ADR that allows publishing curated
screenshots exists and its `## Status` line **starts with** `Accepted`. The ADR's
number is one constant, `POLICY_ADR` in `scripts/compare-page.py`, unset until that
ADR is written; until then the recipe prints `screenshot-policy ADR not found` and
exits 2. The lock is checked before `wrangler` is looked up, so the refusal is what
you see. `python3 scripts/compare-page.py --check-publish-lock` prints the verdict
without publishing.

A curated page should carry only frames the policy covers; the seed and live
manifests are developer material, not a publishing set.
