# The Race Campaign is authored shape, not authored content

2026-09-08. Scoping pass, explicitly archaeology rather than implementation
(the owner's own framing): this project has no game around the race at all -
one drivable race and then nothing - and Wipeout Pulse's actual structure is
a `RACE CAMPAIGN` main-menu entry (`FE_RACE_CAM`, sitting *before*
`FE_RACEBOX` in `MainMenu_Definition.xml`'s `Mode` list, so it is its own
top-level thing, not a mode of the custom race box `race-setup.md` already
covered) leading to a persistent, medal-and-unlock-driven grid of events.
Full write-up with every string and every XML path is
[`docs/formats/race-setup.md`'s new Race Campaign
section](../../docs/formats/race-setup.md#the-race-campaign-the-discs-own-campaign-grid-shape-yes-content-no);
[`docs/gameplay/race-modes.md`](../../docs/gameplay/race-modes.md) carries a
shorter pointer and the Eliminator/Tournament/Head2Head strings. This file is
the index entry and the next-steps list; read the docs pages first, this is
just the map.

**The one-line finding**: the campaign's **screens, its 35-cell hex grid
shape (shared with the player's own custom-grid editor), its medal-target
widgets down to gold/silver/bronze colour, and its unlock gating (the same
named `Grid0`..`Grid10` mechanism already documented for circuit/craft
unlocks) are all authored on the disc, in plain-readable front-end XML.**
What is **not** found authored anywhere read this pass is the actual
*content* of a cell - which track, which mode, its medal target times, how
many points a race is worth, how many points a grid tier requires to clear.
Every one of those slots in the XML (`Cell Selection`'s `Title`,
`Line1`..`Line8`, `Target0`..`Target2`; `Grid Selection`'s `Medals`/`Points`/
`Required`) holds a literal placeholder string (`"SINGLE RACE 01"`, `"l1"`,
`"value"`, `"00/16"`), the same substitution-template pattern this project
already reads on `Track Creation`'s `1/1` counter. Contrast: the **player's**
custom grid *does* author its own medal targets, via a `TargetInput
type="time"` widget the player types into (`SetTargetTimes`, reached from
`Cell Creation`) - so the widget shape existing is not evidence the built-in
campaign's numbers are typed the same way; a player never sees a
`TargetInput` on the campaign's own `Cell Help`.

Also recovered, useful beyond the campaign question itself:

- **The disc's full mode list is seven, this project has four.** Arcade
  (-> Single Race here), Time Trial, Speed Lap, Zone are implemented.
  Tournament, Elimination, Head2Head are not, and now have full English
  text: `MSC_EVENT_TOURN`, `MSC_EVENT_ELIM`, `MSC_EVENT_HTH`.
- **Three previously-unrecorded Eliminator mechanics**, straight from
  `MSC_EVENT_ELIM`: no pickup absorption (health regenerates per lap
  instead), weapon damage scaled up, and the race ends on a **kill-count**
  target rather than a lap count.
- **`Arcade_HUD.xml` is confirmed to serve both single race and tournament**
  in-race (per `docs/ui/hud.md`'s independent reading of its place/time
  widgets) - no separate Tournament HUD exists. `Elimination_HUD.xml` does
  exist and is its own layout, already read.
- **Per-speed-class lap-count help text** (`MSC_LOAD_VENOM`/`FLASH`/
  `RAPIER`/`PHANTOM`): Venom "most... 3 laps", Flash/Rapier "usually/most...
  4 laps", Phantom "most... 5 laps". Hedged wording, confidence 78, but a
  concrete lead against `race-modes.md`'s "Single Race's lap count is ours"
  gap - it currently guesses 3, which only the default class (Venom) text
  agrees with.
- All eight `ER_END_TOUR_1`..`8` tournament-placement strings, `ER_TOUR_COM`,
  `ER_WON_TOUR`, `ER_TOUR_STAN`, `ER_RACE_POINTS`, `ER_RC_POINTS`
  ("Race Campaign points"), and the four medal-award strings
  (`ER_GMA`/`SMA`/`BMA`/`NMA`).

## Open

- **Where the built-in campaign's per-cell content lives is unsettled.** Two
  live hypotheses, not distinguished from XML: a compiled table inside
  `BOOT.BIN` (the same shape `Definition_IsUnlocked` reads `<Unlock>` rows
  from, i.e. shape-in-XML/content-in-code), or a campaign procedurally built
  at runtime from `Definition.xml`'s existing `PI_Track`/`Grid` associations
  with no separate table at all. Distinguishing these needs Ghidra, which is
  outside this pass's lane (`wavefix` owns the Ghidra bridge).
- **No points-per-position or points-per-race-type table was found anywhere
  on disc.** `MSC_EVENT_TOURN` says points are "awarded between races, and
  tallied" but names no values. `ER_RACE_POINTS`/`ER_RC_POINTS` are
  presentation strings only.
- **Grid-tier totals (`"00/16"` medals, `"000/110"` points, `"20"` required)
  are placeholders of unknown cardinality.** Whether every grid tier shares
  these totals or each has its own was not determined - the XML shows one
  template, filled at runtime, not per-tier authored numbers.
- **Only the PSP pressing of Pulse was read this pass.** The PS2 pressing,
  Pure and HD/Fury were not checked for the same campaign-grid shape at all -
  `race-setup.md`'s existing PS2 divergences (bypassed `Racebox` redirect,
  twelve `Tournament C` slots against four) are the only cross-title data
  point so far, and they are about the *custom* grid, not the campaign.
- **Five `Data.wad` GUI files never resolved by path** (`Controls_Definition`,
  `Credits_Definition`, `Debug_Screens`, `MemoryStickBootScreens`,
  `MemoryStickScreens`) - unlikely to be campaign-shaped by name, but not
  ruled out.

## Next Steps

- **Ghidra (not this contributor's lane): find what populates `Cell
  Selection`'s and `Grid Selection`'s runtime-filled widgets.** Same
  cross-reference approach
  [`race-box-screens.md`](../../docs/ghidra/functions/psp-pulse-usa/race-box-screens.md)
  used for `TrackSelection`/`TeamSelection`, now that the PSP relocation
  patch (2026-09-07) makes `get_xrefs_to` work on screen-class name strings:
  `get_xrefs_to("GridSelection")` / `("CellSelection")`, then decompile
  `OnEnter`/`Update` the same way. That single pass should settle whether a
  compiled table exists and, if so, its shape - which is the fact everything
  else in this thread is downstream of.
- **If a table is found, read the points/medal-target values out of it and
  write them into a new `docs/formats/` page** (`race-campaign-data.md` or
  similar) - do not hand-transcribe into a Rust `const`, per `CLAUDE.md`'s
  rule against inventing/transcribing what the disc authors; keep the
  runtime-read path if the format allows one, the way `Data\Ships\...
  handlingstats.xml` is read live rather than baked in.
- **If no table is found and the campaign turns out to be procedurally
  built**, that is also a real, reportable finding - write it up with the
  same confidence discipline, since a negative here is exactly as valuable as
  a discovery (the owner's framing, worth repeating for whoever picks this
  up).
- **Check the PS2, Pure and HD/Fury pressings for the same `GridSelection`/
  `CellSelection`-shaped screens** before assuming Pulse's shape generalises
  - `docs/formats/pure-status.md` and `docs/formats/hd-frontend.md` are the
  places that would carry it.
- **Cross-check `MSC_LOAD_<CLASS>`'s per-class lap-count text against a real
  `<Values laps="%d">` record** (a saved custom grid, or a captured
  `TournamentTrack`) before treating it as more than a lead - it currently
  rests on hedged manual prose alone.
- **A full implementation of Tournament/Elimination/Head2Head is downstream
  of the Ghidra step above and is out of scope for whoever picks this up
  next unless that step lands first** - building `Mode::Tournament` etc.
  against invented points/medal numbers would be exactly the kind of
  standing-in `CLAUDE.md` forbids.
