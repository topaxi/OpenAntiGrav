---
status: ready-to-wire
titles: [2048]
updated: 2026-11-16
summary: "2048 race load is built; open: attract still, weapon-pad and first-load tip rows, cell panel, real Vita draw rate"
size: S (60)
difficulty: easy (60)
---

# 2048's race load is not the screen HD draws

HD's and Omega's loading screens are done (`docs/formats/hd-loading.md`, `docs/formats/omega-loading.md`). 2048's race
load is built (below). What the original draws is now fully read and confirmed against
three Vita3K captures: `docs/ghidra/functions/vita-2048-eu-v104/race-load.md` (operands, tables, the meter's law with
live numbers) and `docs/formats/2048-loading.md` (the summary). Omega is closed: its state 5 has no caller
(`docs/ghidra/functions/ps4-omega-eu/loading-screen-start.md`), so this is a 2048-only build.

## Open

Built (lane `vita-race-load-wire`, `docs/formats/2048-loading.md`, "What is built"): items 1 to 5 of the recipe, and item 6
by drawing the box and meter over the ground with a `debug` loader line. What is left:

1. **The attract demo's still** (item 7): `Title_Screen.gxt` at (-32, 144), 1024 by 256 (or a controls still on an
   HD-lineage mode word) instead of a card. It ships; it needs the texture in the front-end sheet and the demo's load to
   say it is the demo (`Session` already knows). Not wired.
2. **Tip rows**: the weapon-pad rows (`GameTip_Pick` step 3, need `mode+0x1f0` on the `EventCard`), the first-load rows
   of a profile (steps 1 and 2: a counter and per-class flags in the profile, `TutorialsActive`), the controls diagrams
   (rows 34 to 36, `0x8106d3ce`).
3. The prototype challenge's mode page (`Tut_Proto_Event_<team>_0/_1`), the HD and Fury cells' own panel
   (`CampaignEventCard_DrawCellPanel`), and the multiplayer "waiting for" page.
4. Which loader step each `LoadingScreen_SetCeiling` caller is (this build's stages 1 to 3 stand for 0.75, 0.95, 0.98),
   `mode+0x188` (the divisor of the race manager's rate), and a real Vita's draw rate.
5. **Restart and next race**: `0x8115f09a` (the `InGame Restart` and next-race path) calls `LoadingScreen_Start(4)` then
   `SetRate(0.5)`; RACE AGAIN on a campaign event is not routed through the card hand-over, so it draws the box and
   meter over the ground with no card.
6. **The header's second line**: the original's card reads `OFFENSIVE WEAPONS ONLY` (a Queens Mall event with
   offensive weapons) where our front end's card, which this load draws, reads the kind word (`RACE`). A front-end card
   finding: which string the original puts there is unread.
7. The backdrop: the original draws the blurred campaign map behind the card; a launch here draws the `White2048` ground.
8. The session hand-over (`follow_launch` to `Stage::race_loading`) was checked by the compiler and by a capture that
   presses Launch on the real front end (`--loading-card`); a windowed campaign launch was not watched.

- **The race-load compare pairs are hand-made (no `pair.json`)**, so `just compare-rerender-ours` rewrites `compare-page/race-load-2048.toml` with no entries and the page build fails; restore it from `data/reference` after a re-render until the pairs carry a `pair.json` (a `--loading-screen` capture path in `scripts/compare-rerender-ours.py`).

## Next Steps

1. The attract still, with the demo's load (item 1 above, an afternoon at most).
2. `m_activeWeaponPads` on `EventCard`, then the weapon rows of the tip pick.
