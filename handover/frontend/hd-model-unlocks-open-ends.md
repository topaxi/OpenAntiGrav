---
status: ready-to-wire
titles: [hd, omega]
updated: 2026-11-14
summary: "HD/Omega skin unlocks wired; open: split-screen seats' livery, the unread <Unlock> evaluator, the RACE page still's uncut row"
size: S (60)
difficulty: easy (60)
---

# HD-era model unlocks: what the first pass left

Landed 2026-11-14 (lane `hd-skin-unlocks`): Ship Select on HD and Omega offers each HD-era team's
seven `PI_TeamModel`s by their own `<Unlock>` rows, and a picked skin races its own livery folder.
Docs: `docs/formats/race-setup.md` ("HD-era model unlocks"), evidence
`docs/ghidra/functions/ps3-hdfury-eu/model-unlock.md`.

## Open

1. **Split-screen seats past the first** race their model's directory in its own livery:
   `oag_raceplay::SeatPick` carries `hull_variant` and `skin`, no livery, and
   `oag_livery::load` paints only slot 0 from `LoadContext::player_livery`. Wiring: a `livery`
   on `SeatPick` (built in `crates/game/src/app/session/picker/seat.rs` from the seat's variant id
   via `oag_title::race::split_livery`), resolved per seat like `load/model_livery.rs` does slot 0.
2. **The evaluator is unread** on both titles: medal counts are wired as golds over the matching
   campaign's grids (87/80 = those cell counts), loyalty as the named team's total, `grid` as Pulse's
   `Unlock_GridPointsMet`. Next step on Omega v1.07: the function that walks the parser's list at
   owner `+0x120` and reads `+0x78` (`MedalCountHD`).
3. **The `--menu-page` RACE page still** (`capture/menu_page/race_sources.rs`) offers the uncut
   variant table, since no grids are read for it; the live page and the Ship Select still are gated.

## Next Steps

1. Item 1 (about 1 hour, with a split-screen `--screenshot` of two seats on different skins).
2. Item 2 is a `re-first` read; record the result on `model-unlock.md` and drop the "chosen" labels it settles.
