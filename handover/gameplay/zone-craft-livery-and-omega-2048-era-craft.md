---
status: ready-to-wire
titles: [omega, 2048]
updated: 2026-10-26
summary: "Omega and 2048 Ship Select preview the Zone hull in default livery; ship_zone callers unread"
---

# Zone hull is shared on 2048 and Omega; native-craft livery and `ship_zone.vex` are open

2026-10-06, `zone-craft` then `omega-2048-craft`. `ZoneCraft::OwnShipAt` loads
`hdships\Zone\Ship.vex` for every craft on 2048 v1.04 and Omega, and now swaps
the team texture by the definition's own `PI_TeamModel name="zone"` livery.
Omega's 2048-era craft start races (`GuestRoster.handling_dir`).

2026-10-26, `zone-team-pick`: **the team is the player's pick in a Zone race.** Pulse's `Team Selection`
runs in Zone (`TeamSelection_Update`'s mode-6 branch: fixed `Zone` label for the cycler, rating bars
forced to 10, the team list and Loyalty bar intact) and `Zone_Create` and `Race_ComputeLoyaltyAward`
both read the picked team (`docs/ghidra/functions/psp-pulse-usa/zone-mode.md`, 84). Zone now opens Ship
Select after Track Select (`oag_title::ZoneTeamStep`; Pure alone skips it, measured), both TEAM rows
are live in Zone, and the Pulse hull is each team's own `Zone.vex` paint (the eight embedded texture
blocks differ).

## Open

- **Omega and 2048 Ship Select preview the Zone hull in its default livery**, not the picked team's
  `zoneship_<x>` paint the race wears. The swap is `oag_livery`'s `zone_livery` (`pub(crate)`); the
  preview builds its model in `oag_game::preview::psp2_scene`, whose texture resolver would take the
  same path rewrite. 2048 has no Ship Select screen (its team is the RACE page's TEAM row), so this is
  Omega's alone in practice.
- **HD, Omega and 2048 `Team Selection` in Zone is unread** (`ZoneTeamStep::Offered` is Pulse's rule,
  chosen). Whether HD's own screen forces its rating bars full is unmeasured; they show the team's own.
  HD's model grid shows the one Zone hull, open, with no thumbnail (the disc's thumbnails are the team's
  race hulls).
- **PS2 Pulse's `Team Selection` in Zone was not read** (the PSP executable only); the PS2 shares the
  code path by sharing the data, not checked.
- **`PI_TeamModel name="Zone"` (`location="zone01"`, `<Unlock>` 16,000 own-team or 90,000 any)** is what
  Zone mode's `Team Selection` looks up; what `zone01.vex` is for is still unknown (the race flies
  `Zone.vex`, the preview is `Zone_FE.vex` through the literal `zone`).
- **The Zone loyalty credit is read, not watched**: a finished Zone race's EndRace row was not walked
  (a Zone race is long on a software renderer). The seams are tested: the picked team reaches
  `race_options.team`, which `Records::record_loyalty` keys on.
- **What a Zone `Team Selection` leaves for the next normal race is untraced**: the original registers the
  Zone model's `zone01` as `FE_TeamModel` (`FUN_088ea7bc`). Ours leaves the saved model and skin alone on a
  row the mode fixes (Zone, an Eliminator race; `session::picker::livery`), chosen, not measured.
- 2048-era and 2048-native craft author no zone livery; the original's fallback
  (`Zoneship_Faisar` literal, entry `+0x1ac`/`+0x218`) was not read, so they keep
  the default skin. Whether the original paints them Feisar is unread.
- On Vita the Zone hull renders bloomed white, so livery paint is not visible
  there (Omega shows it). Not a livery bug.
- The 18 `ship_zone.vex` files: three generic `%s...vex` templates on Vita
  (`0x814e3388`, `0x81464134`, `0x814ca6a0`) not decompiled.
- Zone sibling files (flare, shield, wreck) still compose under the player's team.
- The base (v1.00) 2048 executable's loader was not read.
- Omega Race Remix: CRAFT TITLE "Wipeout 2048" is now offered with no 2048 disc,
  and Omega's own list drops the 2048-era teams to it; confirm that is wanted.

## Next Steps

1. Decompile the three templates' callers to close the `ship_zone` question.
2. Read the entry's livery-name field for a 2048-era craft to settle its Zone skin.
