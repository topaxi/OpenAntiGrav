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

**A region caveat up front, since it bears on everything below**: `oag-unpack
info` identifies `pulse-psp-usa.chd` as `UCUS-98712` by boot path, but its
volume id reads `SCEE` and it also carries a full EU-serial
`PSP_GAME/USRDIR/UCES00465/` subtree alongside the USA one - unresolved this
pass, see `race-setup.md`'s own region note. The `Data.wad`/`FE.wad`/
`FEData.wad` read here sit under the USA root regardless of that ambiguity.

**The one-line finding**: the campaign's **screens, its 32-cell hex grid
shape (shared with the player's own custom-grid editor - `MaxX="7"
MaxY="5"` is a bounding box, not a cell count; count the authored nodes and
it is 32, not 35), its medal-target widgets down to gold/silver/bronze
colour, and its unlock gating (the same named `Grid0`..`Grid10` mechanism
already documented for circuit/craft unlocks) are all authored on the disc,
in plain-readable front-end XML.** What is **not** found authored anywhere
read this pass is which track/mode/medal-time a given *cell* actually
carries - every one of `Cell Selection`'s content slots (`Title`,
`Line1`..`Line8`, `Target0`..`Target2`) and `Grid Selection`'s summary
widgets (`Medals`/`Points`/`Required`) holds a literal placeholder string
(`"SINGLE RACE 01"`, `"l1"`, `"value"`, `"00/16"`), the same
substitution-template pattern this project already reads on `Track
Creation`'s `1/1` counter.

**That said, a separate, genuinely-authored numeric table was found in
`FEData.wad`** (widening the search past `Data.wad`, prompted by how clean
the negative above looked) - 24 entries, one per `PI_Track`, each carrying a
`RaceTimes`/`LapTimes` figure per speed class, a `Physical Length` (one
value exactly matches the already-captured Talon's Junction White distance,
5178m - real corroboration, not a coincidence), a 12-row AI
difficulty-scaling table (`SkillScaleValue` - previously known only as an
unchased string in `ai-stats.md`, now valued), a 4-row mode-modifier table,
and **an identical `Targets Elimination="10" Zone="25"` on all 24 files** -
the first hard numbers behind `MSC_EVENT_ZONE`'s "target number of zones"
and `MSC_EVENT_ELIM`'s kill-count ending. **Whether the campaign's per-cell
content draws on this table is not established** - it is real, disc-authored
per-track data, but nothing connects it to a specific `Cell Selection` slot
yet, and file order does not reliably key it to a named track (checked and
refuted: a naive "extraction index follows `Definition.xml`'s `k` order"
reading is contradicted by the third pair of entries). Read the `race-setup.md`
section for the full record shape and the ordering refutation.

Also recovered, useful beyond the campaign question itself:

- **The disc's full mode list is seven, this project has four.** Arcade
  (-> Single Race here), Time Trial, Speed Lap, Zone are implemented.
  Tournament, Elimination, Head2Head are not, and now have full English
  text: `MSC_EVENT_TOURN`, `MSC_EVENT_ELIM`, `MSC_EVENT_HTH`.
- **Three previously-unrecorded Eliminator mechanics**, straight from
  `MSC_EVENT_ELIM`: no pickup absorption (health regenerates per lap
  instead), weapon damage scaled up, and the race ends on a **kill-count**
  target (now known: **10**) rather than a lap count.
- **Zone's own target is now known too: 25 zones, flat across every
  circuit** - closes part of `race-modes.md`'s "Zone has no ending yet"
  gap on the target-count side (the ending mechanism itself was already
  implemented; only the count was missing).
- **`docs/ui/hud.md` describes `Arcade_HUD.xml`, in passing, as "the
  single-race and tournament layout"** - cited here as that page's reading,
  not independently re-verified. This pass did directly try
  `Data\XML\Head2Head_HUD.xml` (`oag-wad cat`) and it does not resolve,
  consistent with hud.md's five-file census (no separate Tournament or
  Head2Head layout).
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

- **Where the built-in campaign's per-cell content lives is unsettled, and
  now has a third live question layered on it: does it read the `FEData.wad`
  per-track table at all?** Two structural hypotheses, not distinguished by
  this pass: a compiled table inside `BOOT.BIN` (the same shape
  `Definition_IsUnlocked` reads `<Unlock>` rows from), or a campaign
  procedurally built at runtime off `Definition.xml`'s `PI_Track`/`Grid`
  associations plus the `FEData.wad` record, with no separate campaign table
  at all. Distinguishing these needs Ghidra, outside this pass's lane
  (`wavefix` owns the Ghidra bridge).
- **No points-per-position or points-per-race-type table was found anywhere
  on disc.** `MSC_EVENT_TOURN` says points are "awarded between races, and
  tallied" but names no values. `ER_RACE_POINTS`/`ER_RC_POINTS` are
  presentation strings only.
- **`RaceTimes`/`LapTimes` carry one figure per class, not three.** If these
  do feed `Cell Help`'s gold/silver/bronze `Target0..2` tiers, the
  three-way split is a formula this pass did not find or guess at.
- **Grid-tier totals (`"00/16"` medals, `"000/110"` points, `"20"` required)
  are placeholders of unknown cardinality.** Whether every grid tier shares
  these totals or each has its own was not determined.
- **Only the PSP pressing of Pulse was read this pass**, and its own region
  identity is ambiguous (see the caveat above). The PS2 pressing, Pure and
  HD/Fury were not checked for the same campaign-grid shape at all -
  `race-setup.md`'s existing PS2 divergences (bypassed `Racebox` redirect,
  twelve `Tournament C` slots against four) are the only cross-title data
  point so far, and they are about the *custom* grid, not the campaign.
- **Five `Data.wad` GUI files and 40 of `FEData.wad`'s 64 `<code>`-dialect
  blobs never resolved/were never individually read** (`Controls_Definition`,
  `Credits_Definition`, `Debug_Screens`, `MemoryStickBootScreens`,
  `MemoryStickScreens`, and the 40 `FEData.wad` blobs outside the 24-file
  `RaceTimes` family) - unlikely to be campaign-shaped, but not ruled out.

## Next Steps

- **Ghidra (not this contributor's lane): find what populates `Cell
  Selection`'s and `Grid Selection`'s runtime-filled widgets, and whether
  that consumer also touches the `FEData.wad` 24-file family.** Same
  cross-reference approach
  [`race-box-screens.md`](../../docs/ghidra/functions/psp-pulse-usa/race-box-screens.md)
  used for `TrackSelection`/`TeamSelection`, now that the PSP relocation
  patch (2026-09-07) makes `get_xrefs_to` work on screen-class name strings:
  `get_xrefs_to("GridSelection")` / `("CellSelection")`, then decompile
  `OnEnter`/`Update` the same way. That single pass should settle whether a
  compiled table exists and, if so, its shape - which is the fact everything
  else in this thread is downstream of.
- **Also Ghidra: `SkillScaleValue`'s consumer** - `ai-stats.md` already
  names it as an unchased string; this pass supplies the values (a 12-row
  Easy/Medium/Hard x Venom/Flash/Rapier/Phantom table per track) but not the
  code that reads them. Worth folding into the same pass as the campaign
  question since both start from the same `FEData.wad` family.
- **If a campaign table is found, read the points/medal-target values out of
  it and write them into a new `docs/formats/` page** - do not
  hand-transcribe into a Rust `const`, per `CLAUDE.md`'s rule against
  inventing/transcribing what the disc authors; keep the runtime-read path
  if the format allows one.
- **If no table is found and the campaign turns out to be procedurally
  built**, that is also a real, reportable finding - write it up with the
  same confidence discipline, since a negative here is exactly as valuable as
  a discovery (the owner's framing, worth repeating for whoever picks this
  up).
- **Resolve the region ambiguity on `data/images/pulse-psp-usa.chd`** before
  leaning further on its language table or front-end content - confirm
  whether it is genuinely USA (multi-language, unusually) or a mislabelled
  EU/hybrid image, since `race-box-screens.md` already measured real
  USA/EU front-end divergence and this pass's finds all come from this one
  image.
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
  standing-in `CLAUDE.md` forbids. The two flat constants found here (Zone
  25, Eliminator 10) are the one exception - they are measured, not chosen,
  and safe to wire whenever those modes are built.
