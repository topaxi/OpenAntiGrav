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

### Visible differences (`pulse-pair-diffs`, 2026-10-20)

Seen on the PSP pairs unless noted. Items 1-4 are closed; 5-8 are still leads.

1. ~~**The gantry's GO board is dark in ours**~~ **A pair artifact, closed.** `psp-pulse-pair.py` rendered ours at `--ticks 0`, the pre-countdown clock, and the state is after GO. At `--ticks 300` (now `--ours-ticks`, default 300) the board is green with GO on all 12, from the disc's own clock (`start-gantry.md`).
2. ~~**Engine glow at rest**~~ **The same artifact on the PSP, closed**: ours draws the purple glow at tick 300 on the PSP pairs. **PS2 still differs**: the original's flame is a narrow magenta plume of sprites below a white-hot nozzle, ours the PSP-style purple star. Lead: how `WO_SHIP_ENGINEFLARE.POB` (the PS2's authored flare, `pulse-absent-effects.md`) is played at rest; not read.
3. ~~**Hull colour under strong coloured light**~~ **Fixed**: the GE's fourth hull light is the circuit's nearest `wopoint`/`wospot`, not modelled before. Mean wing colour on Tech de Ra now `(87,106,115)` against the original's `(87,105,115)`, The Amphiseum `(107,129,144)` against `(108,131,145)`. The law, read live and reproduced on three circuits' lists: [`scene-light.md`](../../docs/ghidra/functions/psp-pulse-usa/scene-light.md) "The point and spot lights". De Konstruct is `(102,106,89)` against `(94,98,78)`: a residual of about 8 levels that these lights do not explain (ours takes none there; whether the original's list holds one was not read live).
4. ~~**Record panel**~~ **Matches, closed.** The panel is a countdown: the original's `1:46.3` on The Amphiseum at a clock of `0:20.7` is the authored `<RaceTimes>` figure `2:07.0`, ours reads `2:07.0` at clock 0. Every circuit agrees to 0.1 s once the clock is added (Arc Prime `1:24.7 + 0:32.2 = 1:56.9`, ours `1:56.5 + 0:00.4`).
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
