---
categories: [gameplay]
---

# The multiplayer menu flow: one MULTIPLAYER entry, local now, LAN and online greyed

2026-10-14, planned with the maintainer. **Landed the same day (`mp-menu-flow`): steps 1 to 4 and 6 to 8**; step 5, seat
assignment, **landed 2026-10-14 as `mp-seat-assign`** (`session::seat_assign`, between layout and settings). Replaces the per-title split
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
5. **Seat assignment** (**landed**): each player moves their own marker onto the pane they want (moving onto a held pane
   swaps; a moved person confirms again). Remembered on disk by device. Opens for any layout with more than one view, one window holding a split included; skipped for a single view.
6. **Race settings**: our RACE page (mode, class, weapons, AI difficulty, AI on/off); Head to
   Head greyed unless exactly two are seated (`Mode::max_people`). Remembered.
7. **Track Select**, then **Ship Select** per seat, then the race.

**Quick paths**: race again and change track from the end screens skip 3 to 5. A fresh entry
pre-fills every step, so it is OK, OK, OK.

## Landed (`mp-menu-flow`, 2026-10-14)

Written up in `docs/rendering/split-screen.md`, "Where the entry is, per title". What exists now:

- A MULTIPLAYER row on all five titles (Pulse `FE_MP`, worded `MULTIPLAYER & SHARING` from the disc; Pure's
  `Multiplayer`; the built-in extras for HD and Omega; a third tile on 2048's grid) and its page: LOCAL
  MULTIPLAYER, and LAN and ONLINE as `Action::ComingLater` rows (dimmed, focusable, the line says `Coming later`,
  every shipped language with `human = false`).
- The old entries are gone: Pulse and Pure's borrowed SPLIT SCREEN rows, the extras' SPLIT SCREEN row, the
  race-box `NUMBER OF PLAYERS` and `SPLIT SCREEN` rows and their settings, `FrontEnd::local_split`.
- Join (confirm or START joins, START needs two, devices seated earlier this session are seated again,
  seat 2 says the title's `FE_GUEST_JOIN` on HD and Omega), a row per keyboard where the platform names
  keyboards (X11 and Windows), layout, the RACE page (the mode row marks a mode `Mode::max_people` caps),
  Track and Ship Select per seat. RETURN TO MENU after a multiplayer race lands on the RACE page with the
  same people seated (**chosen**); race again was already seatless.

## Lanes

- ~~`mp-menu-flow`: steps 1 to 4 and 6 to 8.~~ **Landed 2026-10-14.**
- ~~`mp-seat-assign`: step 5.~~ **Landed 2026-10-14**: `oag_ui_screens::seat_assign`, `session::seat_assign`, `[race] window_panes`;
  opens for one window holding a split too (`seat-assign-split`). Documented in `docs/rendering/multi-window.md`, "The seat assignment screen".
- Not before `h2h-port-titles` adds Head to Head to the mode list, or coordinate: both touch
  the RACE page's MODE row.

## Open

- ~~**A second keyboard cannot join through the menu** (found by `keyboard-seats`, 2026-10-14).~~ **Done on X11 and
  Windows** (`Device::Keys`, a row per keyboard); **open on Wayland and macOS**, which name no keyboard, so a join *row* is
  impossible there (the windows open only after the layout): a second Wayland keyboard still claims a seat by a
  press in its window. Original finding:
  The windowed-race path (`--windows`, `--window-seats`) routes keyboards per seat (Wayland
  seats by window, X11/Windows by raw device). The join screen still treats the keyboard as one
  device, so `mp-menu-flow`'s join step should take its design: `lobby::Device::Keyboard`
  carries a source, and the join screen shows a row per keyboard. Details in
  `docs/rendering/multi-window.md`'s Open list.
- Whether the CLI (`--players`, `--window-seats`) gains a way to skip into the flow at a step.
- **RETURN TO MENU re-enters the flow's RACE page after a multiplayer race** (chosen, no maintainer answer yet to "change
  track" as an end-screen option): the disc's end menu has no CHANGE TRACK and HD's blocks sit at authored y values, so
  none was added. BACK from that page lands on the MULTIPLAYER page (not on layout or join).
- The mode row's cap (`oag_ui::menu::cap_modes`) is unit-tested over a synthetic Head to Head and shows nothing on
  screen until `h2h-port-titles` adds the mode; that lane should also settle what a stored Head to Head does when three
  people are seated (the row dims, `launch_from_settings` does not yet refuse).
- The cut on the web and phones (no layout screen) is the remembered layout's first window, else horizontal; nothing there can
  choose a vertical split.
- **Seat assignment, still open** (`mp-seat-assign`, `seat-assign-split`): a race again and a RETURN TO MENU race after an
  assigned layout were not walked (no race finishes on lavapipe; pinned at `multiplayer::reenter` and the functions the path
  calls, see `docs/rendering/multi-window.md`), a keyboard player flying a pane in a race on Xvfb (not even in the join-order
  control: a held key moved nothing there), Pure, Omega and 2048 stills. The browser and the phones have no layout screen,
  so a split there cannot open the seat screen (a `WindowLayoutStage` built from `remembered_cut` would).
