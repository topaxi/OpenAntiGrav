---
categories: [gameplay]
---

# The multiplayer menu flow: one MULTIPLAYER entry, local now, LAN and online greyed

2026-10-14, planned with the maintainer. Nothing built yet. Replaces the per-title split
entries (Pulse and Pure's two SPLIT SCREEN rows, HD/Omega/2048's race-box NUMBER OF PLAYERS
and SPLIT SCREEN rows) with one flow on every title.

## What the originals do

- **Pulse PSP**: main menu row 3 is `FE_MP` "MULTIPLAYER & SHARING" ("Race online or over Ad
  Hoc, and share data"). `Multiplayer_Definition.xml` (`Data.wad`) has a `Multiplayer` screen
  listing `Create Single Race`, `Create Tournament`, `Create Time Trial` and `Join Game`, then
  `Online Team Selection` or `Class Selection`: the ad-hoc network flow, no local split.
- **Pulse PS2**: main menu rows `FE_RACE_CAM_SS`/`_SSH` (SPLIT SCREEN VERTICAL/HORIZONTAL) ->
  `TournamentLoad` -> `Split ScreenV`/`H` (mode `Arcade`/`Tournament`/`Head2Head`, class,
  weapons, difficulty, eliminations) -> `Track CreationSplit` (or `Tournament CSplitV`/`H`)
  -> `Team SelectionSplit` per player. Two pads in port order, no join screen.
- **Pure**: main menu `MULTIPLAYER` (capture in `docs/ui/menus-original.md`).
- **HD/Fury, Omega**: race box RACE page rows `NumberOfPlayers` (1, 2) and `SplitScreen`
  (H, V); `FE_GUEST_JOIN` "PLAYER 2 PRESS START TO JOIN" authored; Track Select; `Ship Select
  - Player 1`/`- Player 2`. Online is `FE_ONLINE` and `multiplayer_definition.xml` (`DATA02`).
- Evidence and our current entry: `docs/rendering/split-screen.md`, "Where the entry is, per
  title" and "The Pulse and Pure entry"; `docs/formats/fe-menu-definitions.md`.

## The flow (maintainer's decisions, 2026-10-14; ours, chosen, not measured)

1. **MULTIPLAYER** on the main menu of all five titles, with the title's own row text where it
   authors one (Pulse `FE_MP`, Pure `MULTIPLAYER`), ours elsewhere. It replaces the split rows
   above.
2. **Multiplayer page**: LOCAL MULTIPLAYER; LAN and ONLINE greyed, not selectable, help line
   "Coming later" in every shipped language (`human = false`).
3. **Join**: each player presses on their device to take a seat, worded with the title's own
   join text where authored (`FE_GUEST_JOIN` on HD/Omega). Devices seated earlier **this
   session** are pre-seated if still connected; held in memory only, so a restart joins again.
4. **Layout**: windows, monitor, cut, fullscreen (the existing WINDOW LAYOUT screen). Remembered
   on disk.
5. **Seat assignment** (new): each player moves their own marker onto the pane they want.
   Remembered on disk.
6. **Race settings**: our RACE page (mode, class, weapons, AI difficulty, AI on/off); Head to
   Head greyed unless exactly two are seated (`Mode::max_people`). Remembered.
7. **Track Select**, then **Ship Select** per seat, then the race.

**Quick paths**: race again and change track from the end screens skip 3 to 5. A fresh entry
pre-fills every step, so it is OK, OK, OK.

## Lanes

- `mp-menu-flow`: steps 1 to 4 and 6 to 8. About 1.5 days.
- `mp-seat-assign`: step 5. About 1 day, after `mp-menu-flow` merges (both edit the session
  flow).
- Not before `h2h-port-titles` adds Head to Head to the mode list, or coordinate: both touch
  the RACE page's MODE row.

## Open

- Whether the CLI (`--players`, `--window-seats`) gains a way to skip into the flow at a step.
