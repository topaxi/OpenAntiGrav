# 2048's campaign is read from `SP.xml`, not a `PI_Grid` grid

2026-09-21, branch `lane/2048-campaign-data`. Full write-up in
[2048-campaign.md](../../docs/formats/2048-campaign.md); the format and every
number below is measured there, not re-derived here.

**The one to know**: 2048 has no `CellMode_Definition.xml` and its sixteen
`data/plugins/grids/grid_NN.xml` are the two **DLC** campaign tiers
(`Campaign="HD"`/`"Fury"`), already reachable through
`oag_tables::race_campaign`. 2048's own, native campaign is a completely
different, previously-unread format: `Data\xml\SP.xml`, a "mjolnir"
typed-instance database (`<mjolnir><instance typedefid=".." name="..">`),
288 instances across eight typedefs. `oag_tables::mjolnir` reads it generically
and `oag_tables::mjolnir::campaign` gives it a typed view (event kinds,
`TrackDefinition`, `WeaponSetDefinition`); `oag_2048::campaign` names what
2048 ships (`SP_XML`/`MP_XML`, the five-class `EClass` ladder, `engine_mode`);
`race::load_event` (`crates/game/src/race/load/campaign.rs`, new) resolves an
event by name onto `race::Options` and calls the existing loader.
`oag-game --event "<name>"` drives the whole chain end to end, verified
against the real EU v1.04 package.

**The evidence method worth reusing elsewhere**: every field that references
another instance carries the referenced typedef's own name right on the field
(`M_TRACKDEF type="TrackDefinition" typedefid="205052969"`), so the type table
for five of the eight typedefs came straight out of the file's own text, not
a structural guess - confidence 95 each. This directly overturned a prior
hypothesis: `typedefid -966434245` (20 instances) was guessed to be
`TrackDefinition` ("2048 ships 20 track profiles"); the file's own text says
it is `WeaponSetDefinition`, and the real `TrackDefinition` (`205052969`, 10
instances, matching 2048's real ten circuits) was sitting right there in the
same census, unlabelled until this pass read the field-type pairs instead of
counting instances.

**A second finding worth flagging loudly**: `oag_race::Mode::Eliminator` is a
real, implemented mode - not a stub - and 2048's own
`TITLE.weapons.elimination` already names `weaponstats_Elimination_2048.xml`.
This lane's own brief anticipated Elimination might need refusing as
unsupported; checking directly instead of trusting that anticipation, all
four of `SP.xml`'s event kinds resolve to a working mode. Nothing on this
title needs the "refuse by name" path SP.xml's campaign would otherwise have
needed.

## Open

- **Which `GameMode_*` C++ class each of the two `Race`-kind typedefs
  (`-1915183557`, `-1353052320`) actually instantiates is not resolved.**
  `eboot.elf`'s own string table names six classes
  (`GameMode_ArcadeRace`/`CheckPointRace`/`EliminatorRace`/`SpeedLapRace`/
  `ZombieRace`/`ZoneRace`); `Zone` and `Elimination` bind cleanly to two of
  them by field shape and corroborating instance names, but the two `Race`
  typedefs do not - their own `M_PNEXTEVENT` chains cross freely between
  them (`"2048 - Event 3"`, one typedef, names `"2048 - Event 4"`, the
  other, as its own next event), which argues against a mode split rather
  than for one. No Ghidra session was opened this pass, per this lane's own
  scope - a decompile of `GameModeFactory::OnNewInst` (the string
  `"GameModeFactory::OnNewInst %s: %s - %s"` is in the binary) is the
  concrete next step if this is worth resolving, since it likely names the
  concrete class per instance at construction time.
- **`GameModeObjective`'s own semantics: closed 2026-09-21, mostly.** See
  `docs/formats/2048-campaign.md`'s "The objective law" section -
  `M_OBJECTIVETYPE`'s four real ordinals (`FINISH`/`POSITION`/`KILLS`/
  `BEAT_VALUE`) are measured against every `(pass, elite)` pair the file's
  80 fully-authored events carry, and 2048's own medal law is genuinely
  two-tier (no third rung `M_PSECONDARYOBJECTIVE`/`M_PTERTIARYOBJECTIVE`
  ever authors). Still open within that: Elimination's own `BEAT_VALUE`
  metric (confidence under 50 for any guess - `oag_race::Standing::kills`
  does not fit the observed `25`-`100` targets), and three orphaned
  objective instances no event ever references.
- **The unlock graph: closed 2026-09-21.** `oag_2048::campaign::
  unlock_gates` folds every event's own incoming `M_PNEXTEVENT`/
  `M_PBRANCHEVENT` edge and its `M_PEVENTREQUIRED` into one prerequisite
  name; `oag_game::records::Store::record_campaign` persists a finished
  event under its own name; `oag_ui::frontend::Frontend::
  refresh_campaign_progress` folds a save into the map, once at boot. See
  `docs/formats/2048-campaign.md`'s "The unlock graph" section and
  `crates/game/tests/vita_2048_campaign_progress_ground_truth.rs`. Still
  chosen, not measured: whether *any* result or only a pass-or-better one
  opens the next event - `SP.xml` does not say, and this build reads
  `Store::campaign_medal`'s own `best_medal` (any earned tier) as the
  gating signal.
- **`WeaponType`'s bit layout is unread.** `WeaponSetDefinition`'s
  `M_WEAPONAVAILABLEBITS` is a raw per-instance value; which bit is which
  weapon is not decoded.
- **The `M_X`/`M_Y` -> screen-pixel projection for a native event is not in
  the data anywhere this pass checked.** The DLC tiers' own scale/bias
  table (`docs/ghidra/functions/vita-2048-eu-v104/frontend-campaign-map.md`)
  is a different, explicitly separate node set (`FE3DCanvas`'s
  `TouchCampaignHD_Item`/`TouchCampaignFury_Item`) and does not apply here -
  see `2048-campaign.md`'s own section on this. If the projection exists at
  all, it is compiled into the executable, not authored in any XML this
  pass opened.
- **`Data\xml\MP.xml`'s own 230-instance season/level schema
  (`1114956821`/`425681076`) is unread beyond its field census.** Not
  `SP.xml`'s shape at all - see `2048-campaign.md`'s own section.
- **The `CanvasLabel linkedevent` count measured here (68 labels, 47 with a
  non-empty `linkedevent`, all 47 resolving) does not quite match
  [`2048s-front-end-is-read-and-not-wired.md`](2048s-front-end-is-read-and-not-wired.md)'s
  own earlier count of "all 69 of `Definition.xml`'s own `FE3DCanvas`
  labels."** Both numbers came off `data/plugins/frontend/NEWGUI/Definition.xml`
  in the base package; the one-label gap was not chased down (a different
  archive precedence, a different XML tool's tag-matching, or a genuine
  off-by-one in one of the two counts are all still possible) and this
  pass's own 68/47/45 is the one `2048-campaign.md` cites, measured with
  `oag_tables::fexml`/`oag_tables::mjolnir` directly rather than a text
  search.
- **The other three lanes' own boundaries were crossed once, narrowly, and
  flagged to the coordinator rather than merged silently.**
  `crates/game/src/main/headless.rs::run_race` needed one match arm to
  dispatch `--event` to `race::load_event` instead of `race::load` - the
  only call site in the workspace that actually invokes `race::load` for
  `--race`, and every other option for wiring the flag through would have
  required editing `race::Options`'s field list, which is built
  exhaustively (no `..Default::default()`) at ~60 call sites this lane does
  not own. See commit `cba5b94f` and the message sent to `team-lead`
  2026-09-21.

## Next Steps

1. Decompile `GameModeFactory::OnNewInst` (and whatever calls it per
   concrete event) to settle the `Race`-typedef-to-`GameMode_*`-class
   question above, if a caller ever needs to tell 2048's own Arcade/
   CheckPoint/SpeedLap/Zombie races apart rather than treating them all as
   `EventKind::Race` the way this pass does.
2. Read `WeaponSetDefinition`'s `M_WEAPONAVAILABLEBITS` bit layout against
   `WeaponType`'s own enum, if a caller wants an event's weapon set to
   actually gate which pickups spawn rather than being carried as an
   unresolved reference.
3. Read `Data\xml\MP.xml`'s season/level schema, if multiplayer's own
   progression ever needs the same treatment this pass gave `SP.xml`.
4. Chase the 68-vs-69 `CanvasLabel` discrepancy against
   `2048s-front-end-is-read-and-not-wired.md`'s own count, if the exact
   number ever matters for something (e.g. asserting a full front-end/data
   parity in a ground-truth test).
5. If a native `M_X`/`M_Y` -> pixel formula is ever wanted (e.g. drawing 2048's
   own campaign map the way the DLC tiers' `FE3DCanvas` hotspots are already
   understood to place), a Ghidra pass over the base-campaign refresh path is
   the way in - out of this lane's own scope entirely this time.
