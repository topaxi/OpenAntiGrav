# The compare page is built; HD and Pulse PSP give pairs, four titles are partial, publishing is unlocked

`just compare-page` builds a static page of original-vs-ours pairs (split slider,
onion skin, side by side, difference) from manifests under
`data/reference/compare-page/`, and `--serve` puts it on the LAN. Usage, the manifest
and per-title recipes with their status: [`docs/tools/compare-page.md`](../../docs/tools/compare-page.md).
Landed 2026-10-20 (`compare-page` lane). Each image carries a credit (game, platform, original or engine) and the page footer carries ADR-0074's rights-holder statement, read from `legal.md` and checked against the ADR by `just check-docs` (`compare-page-credits` lane, 2026-10-20); `--publish` builds strictly and refuses an entry with no `platform`.

## Open

- **A new manifest entry needs `platform`**; the seed and live manifests under `data/` have it (`compare-seed.py` writes it).
- **`just publish-compare` is unlocked (ADR-0074 Accepted) and has never been run.** The maintainer runs it; `wrangler` is not installed on the lane host.
- **HD: the craft pose is not carried.** `rpcs3-drive.py capture` gives the camera, so
  our craft sits a little off (the original has moved a few frames on). `frame-shot`
  for HD wants a ship pose row too (the autopilot capture `scripts/rpcs3-hd-autopilot.py`
  reads one). A campaign-start pair also has the original in grid slot 8 and ours in
  slot 1: use a Time Trial state.
- **Pure (PSP): not reachable.** `psp-drive.py menu` is Pulse's walk; the camera hook
  `0x0883c13c` is Pulse USA's. Needs a hand walk to a grid, a save state
  (`psp-state.py save --keep`, `--serial` for Pure) and Pure's camera node located.
- **Pulse PS2: one pair, ship pose only** (2026-10-20, `compare-pairs-pulse`): Moa Therma White, Single
  Race, grid slot 8 of 8. The craft pointer is read out of the `.p2s` (`scripts/pcsx2_state.py`), the
  circuit is in `data/saves/pulse-ps2/*.p2s.json`. Ours uses our own chase camera and is letterboxed by
  design; the original's PS2 camera is not read. More PS2 circuits need a menu walk (`pcsx2-drive.py`
  has no menu route).
- **2048 (Vita3K): no race reached.** `vita3k-drive.py race`'s fixed-timing tap walk did
  not change screen and the frame was an intro cinematic.
- **Omega (shadPS4): menu only.** MAIN MENU is reached unattended in about 45 s; no
  race was walked. The pair is the original's menu against our own menu page.
- **Pulse PSP: all 12 circuits have a pair** (White entry of each, Time Trial grid after GO, Assegai/Venom;
  `data/reference/compare-page/pulse.toml`) and a state under `data/saves/pulse-psp/time-trial-<circuit>-white-venom-grid.ppst`
  (Talon's Junction's is the older file name). The Black entries are the same circuits' other layouts, not captured.

### Visible differences, a lead for fidelity lanes (not fixed here)

Seen on the PSP pairs unless noted; each is in at least two circuits unless it says otherwise.

1. **The gantry's GO board is dark in ours** (all 12): the original's start gantry shows a green board with GO after the countdown.
2. **Hull colour under strong coloured light**: Tech de Ra and The Amphiseum show a brown/orange hull in ours against blue-white in the original; Arc Prime and De Konstruct are close.
3. **Engine glow at rest**: a purple glow in the original, none in ours (all PSP pairs); the PS2 original has a magenta flame, ours a dim purple one.
4. **Record panel**: the original shows each circuit's own record (1:24.7, 1:07.3, 0:57.x, 1:09.2 ...), ours shows 1:57.0, 1:40.0, 1:27.x, 1:42.0, so the record value differs per circuit.
5. **Outpost 7**: a white panel at the top left of ours where the original shows blue sky; Platinum Rush and Vertica: a distant sign is yellow or red in ours against red or orange in the original (animated billboards, phase not matched).
6. **Tech de Ra**: a lit box at the top left that the original does not draw.
7. **Race clock**: not matched by design (the original has run since GO).
8. **PS2**: the original's hull fills about 1.35x the width ours does (our PSP-shape fit and our own camera), the original shows "pos 8/8" where ours shows no position, and a faint `(UT EDIT)` string sits at the bottom left of the original (a debug string).

## Next Steps

0. Pulse PS2: more circuits need a menu route for `pcsx2-drive.py` (it only boots and loads states); the original camera's pose (rig eyes at craft-rig `+0x850`/`+0x860`) could be carried like PSP's `--camera`.

1. Make a Time Trial state for each title the maintainer wants reviewed and capture
   one pair per circuit with the recipes in the doc.
2. HD ship pose: add the player body pose to `rpcs3-drive.py capture`'s JSON and let
   `render_with` carry it, so a pair is pose-matched as on Pulse PSP.
