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

**2026-09-28: the user's own play observation ("2048 may restrict which craft
you can pick, or force one") is confirmed, and the field this project expected
to carry it was the wrong one.** `WOShipCreatorParams` is authored on zero of
`SP.xml`'s 141 events; the real mechanism, `GameModeBase_RegisterFields`
(`0x812b0e2a`, `docs/ghidra/functions/vita-2048-eu-v104/game-mode-base-fields.md`)
names two fields no prior pass had read: `M_PPLAYERSHIPMODELDATA` forces a
specific craft (14 events - all five `"* P Ship Challenge"`, four `E3_Demo*`,
five ordinary numbered events) and `M_bPrevent{Combat,Agility,Speed,Proto}Ships`
restricts choice to a subset (6 events, all "2050" bar one). `oag_2048::
campaign::craft` resolves the first onto `race::Options::team` and
`race::load_event` applies it, verified live (`--race --event "2048 - Event
4-2"` -> `Qirex2048\1`). The restriction mask is read and reported but **not
enforced, because the original has no craft-selection step on the campaign
launch path at all** to enforce it against - see "Open" below, closed
negative. Full write-up: `docs/formats/2048-campaign.md`'s "Craft choice"
section. Also surfaced, not chased: `M_PGRIDSHIPMODELDATA` authors the AI
grid explicitly on most events (a `teams_for_slots` replacement, unwired) and
the five/ten Ship/Phantom Challenge side events - "open on a fresh save" by
the unlock graph - also carry `M_RankRequired`, unread and unenforced.

## Open

- **Closed 2026-09-28, negative result: the 2048 campaign launch path has no
  craft-selection step at all, in the original.** `NEW_FE_SHELL`'s campaign
  tap redirects straight to `Launch 2048`, which `2048-frontend.md` already
  measures as "nothing but a `<BackendController task="Launch">` ... no
  confirm screen in between"; `team` (`Team_Definition.xml`) is reached by
  exactly one authored edge, `HOME`'s own `ER_TEAM` tile. So a campaign race
  flies whatever the player last set at `Home -> Team` (forced-craft events
  aside), and `M_bPrevent{Combat,Agility,Speed,Proto}Ships` has no UI to
  enforce it against on this path - not a function this pass failed to find,
  but one that does not exist here. `TeamSelection_Screen.cpp`'s own
  constructor and three vtable methods (`0x8113b4dc`, `0x8113b668`,
  `0x8113b77e`, `0x8113bec8`) were decompiled looking for it anyway and, as
  expected, touch neither `GameModeBase+0x80..0x83` nor `+0x3c`. No picker was
  added - inventing one the disc does not author would violate this project's
  own rule. See `docs/formats/2048-campaign.md`'s "Craft choice" section.
- **Closed 2026-09-28: `M_PGRIDSHIPMODELDATA`'s authored grid is wired against
  `oag_game::livery::teams_for_slots`'s own "chosen, not measured" one.**
  `oag_2048::campaign::craft::grid_craft` resolves the field slot-by-slot
  (not `Field::references`, which drops an unauthored slot's own position);
  `race::Options::grid_teams` (new, per-slot, defaults to "no override" so
  only one exhaustive `Options` call site needed a line) carries it into
  `crates/game/src/race/load/roster.rs::apply_grid_teams`, which overlays it
  onto `teams_for_slots`' own placement one slot at a time rather than
  replacing the whole grid - the "own design rather than a drop-in" this
  entry used to call for. Measured against the real EU v1.04 `SP.xml`: 55 of
  141 events author all 7 slots, 2 author only their first, 84 resolve none.
  Verified live and against a headless `--screenshot`: `--race --event
  "2048 - Event 6"` reports `"grid: 7 of 7 AI slot(s) authored..."` and the
  exact authored roster (three Feisar, two Auricom, two AG_Systems). See
  `docs/formats/2048-campaign.md`'s "Craft choice" section and
  `crates/game/tests/vita_2048_campaign_grid_ground_truth.rs`.
- **The rank gate (`M_RankRequired`) on the Ship/Phantom Challenge side
  events.** This engine has no player rank at all; live verification needs a
  progressed save, which `data/extracted/vita/PCSF00007/base/savedata` does
  not ship (empty).
- **Closed 2026-09-28 (Q2): which `GameMode_*` C++ class each of the four
  event typedefs instantiates.** Not by decompiling a factory dispatch
  function (the string `"GameModeFactory::OnNewInst %s: %s - %s"` this
  thread's own earlier note named as the way in does not exist in this
  binary - searched, zero hits) but by hash: `oag_formats::wad::hash_name`
  of each of the six `GameMode_*` class names `eboot.elf`'s string table
  carries lands exactly on four of `SP.xml`'s own typedef ids, the same
  case-folded-CRC-32 convention this module's own five in-file names already
  confirm these ids use, zero misses across all ten names checked.
  `RACE_A` is `GameMode_SpeedLapRace`, `RACE_B` is `GameMode_ArcadeRace`,
  `ELIMINATION` is `GameMode_EliminatorRace`, `ZONE` is `GameMode_ZoneRace` -
  confidence 92, pinned in a new test
  (`crates/tables/src/mjolnir/campaign/tests.rs::
  typedef_ids_are_hash_name_of_the_class_they_are`). `GameMode_CheckPointRace`
  and `GameMode_ZombieRace` hash to ids `SP.xml` does not carry at all -
  shipped classes the campaign never instantiates. The `M_PNEXTEVENT`
  cross-chaining between `RACE_A`/`RACE_B` this thread's own earlier note
  read as evidence against a mode split was never that - it only shows the
  unlock graph chains across concrete classes freely; `EventKind::Race` still
  merges them, correctly, since `engine_mode` already derives the Speed
  Lap/ordinary split off `laps == Some(0)` independently. See
  `docs/formats/2048-campaign.md`'s typedef table and
  `crates/tables/src/mjolnir.rs`'s own updated module doc.
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
- **Closed 2026-09-28, fully as of the same day's second pass.** `WeaponType`'s
  bit layout: the first pass pinned eight bits (Rocket/Missile/Quake/Turbo/
  Cannon/Autopilot/Plasma/LeachBeam) at confidence 95 from the 20
  `WeaponSetDefinition` names alone, plus bits 8/9 as one joint gate for
  Mine+Bomb at confidence 75, and left bit 4 unpinned (`eboot.elf`'s own
  held-weapon id order, `pickup-icon-uv-table.md`, corroborated these as
  eleven real, distinct pickups but used a different bit/id order past
  position 5, so it did not resolve the assignment further). **A same-day
  second pass chased `WeaponType`'s own enum declaration in `eboot.elf`
  instead of a runtime consumer** (`WeaponType_RegisterValues`,
  `0x8100e650`) and settled all three remaining questions: bit 4 is
  `Shield` (confidence 95), and bits 8/9 split individually - `Bomb`=8,
  `Mine`=9 (confidence 90 each), the opposite order from the held-weapon id
  table. `"NoQuake"` (`32`, identical to `"Cannons Only"`'s value) stays an
  authored anomaly, unresolved by either pass. `WeaponSet::allowed_weapons`
  (`crates/tables/src/mjolnir/campaign.rs`) now decodes all eleven bits and
  `race::load_event` wires the result onto `Setup::allowed_weapons`, which
  `oag_gameplay::pickup::draw`'s own `allowed` parameter gates a `Weapon
  Pad`'s draw by - an event whose set decodes to nothing recognised races
  unrestricted rather than guessing. See `docs/formats/2048-campaign.md`'s
  "The weapon set gate" section,
  `docs/ghidra/functions/vita-2048-eu-v104/weapon-type-bits.md`,
  `crates/tables/src/mjolnir/campaign/tests.rs::
  allowed_weapons_decodes_every_weapon_type_bit`,
  `crates/2048/tests/campaign_ground_truth.rs::
  a_named_weapon_set_decodes_to_what_its_own_name_says` and
  `crates/game/tests/vita_2048_campaign_progress_ground_truth.rs::
  load_event_wires_2048_event_6s_own_weapon_set_onto_setup`.
- **The `M_X`/`M_Y` -> screen-pixel projection for a native event is not in
  the data anywhere this pass checked.** The DLC tiers' own scale/bias
  table (`docs/ghidra/functions/vita-2048-eu-v104/frontend-campaign-map.md`)
  is a different, explicitly separate node set (`FE3DCanvas`'s
  `TouchCampaignHD_Item`/`TouchCampaignFury_Item`) and does not apply here -
  see `2048-campaign.md`'s own section on this. If the projection exists at
  all, it is compiled into the executable, not authored in any XML this
  pass opened. **2026-09-25: still true, but the map's own *scale* (not this
  per-event formula) is now measured off the reference captures rather than
  chosen** - see
  [2048s-front-end-is-read-and-not-wired.md](2048s-front-end-is-read-and-not-wired.md)'s
  own 2026-09-25 note and `docs/formats/2048-frontend.md`'s campaign-map
  row. That pass also ruled out one plausible shortcut: fitting the 47
  `linkedevent`-carrying `<CanvasLabel>`s' own `x`/`y` against this file's
  `M_X`/`M_Y` looks like it could be the projection, but the same label set's
  twenty `MPSeason01`-`20` entries carry no `linkedevent` and no relation to
  `SP.xml` at all, yet sit in the identical small coordinate range - so the
  fit is spurious, not a real one, and is not worth re-attempting without a
  decompile. **2026-09-25: the per-event detail screen (real game's tap
  target, not this build's now-removed bottom bar) was also searched for as
  an authored `NEWGUI` screen and is not one** - see
  [2048s-front-end-is-read-and-not-wired.md](2048s-front-end-is-read-and-not-wired.md)'s
  own 2026-09-25 note. `MapEvent::detail` (built from this file's own
  `M_TRACKDEF`/engine-mode/laps/weapon-set fields, `crates/game/src/boot/
  campaign2048.rs`) is still correct, real data - only where the front end
  draws it changed, not how it is derived here.
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
2. **Closed 2026-09-28, fully.** See the "Open" section's own entry above for
   what landed across both same-day passes (`WeaponSet::allowed_weapons`,
   `Setup::allowed_weapons`, `pickup::draw`'s `allowed` parameter, and the
   second pass's `WeaponType` enum-declaration read that pinned bit 4 and
   the `Bomb`/`Mine` split) and where it is measured and verified. Nothing
   left open within it except `"NoQuake"`'s own authored anomaly, which no
   consumer was found to check against either.
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
6. **Superseded 2026-09-28, in the other direction from the 2026-09-28 entry
   above**: `M_bPrevent*Ships` *is* enforced by the original, at
   `GameModeBase_IsShipTypeAllowed` (`0x812b41da`), called from a
   previously-unlocated native screen - the campaign map's own per-event card
   (`docs/ghidra/functions/vita-2048-eu-v104/campaign-event-card.md`), not
   `TeamSelection_Screen.cpp`. This project now enforces the same gate at
   `oag_ui::frontend::campaign_map::Frontend::launch_selected_event`. See
   `docs/formats/2048-campaign.md`'s "Craft choice" section for the full
   finding. Still open: the card's own *layout* (photo backdrop, per-event
   art, pagination dots) was not decompiled past its three buttons' own hit
   rects, so building that screen for real is still its own, separate piece
   of work - see `campaign-event-card.md`'s own "not chased" note on the
   `M_X`/`M_Y` projection its draw call also computes.
   **Card pages finished 2026-09-29** (branch `2048-card-pages`): the
   `BEAT_VALUE` wording is read (per-mode `vtable+0x6c` slot: SpeedLapRace
   `BEAT M:SS`, Zone `ZONE TARGET : n`, the rest `SCORE n POINTS`, which is
   in the string table after all), page kinds `1` (leaderboard tabs) and `4`
   (rules) draw, `BuildPageList` is mapped field by field, the kind line
   and mode icon follow the class (`GameMode_GetKindLabel`: the 13 timed races
   are `TIME TRIAL`), and Zone cards do have title, `Zone<Name>` photo and
   emblem (`M_TRACKDEF` is authored on them). **Still open**: page kind `2`
   (trophy and cup art, `FUN_81052fb4`: find where `DAT_816c76fc..` are loaded
   and the ten named events' images), the weapon callout of the rules page
   (`FUN_810626ce`), the elite-pass row of the objective page, the record row
   of the leaderboard (needs a per-event result store), and the placement of
   the forced craft's two quads (`FUN_81061808`). Capture a card with
   `--until card:N:EVENT NAME` (unlocked events only). The map draws the
   whole timed class (53 events, 13 of them laps races) with `speed_mode`.
   **Card built 2026-09-29** (branch `2048-event-card`): layout recovered from
   `CampaignEventCard_DrawPanel` and friends, the photo/emblem/glyph art found
   in `NewImages` (no XML names it), and `oag_ui::frontend::event_card` draws
   it with pointer support - see `docs/formats/2048-campaign.md`'s "The event
   card". **2026-10-05** (branch `v2048-campaign-card`): ~~page kind `2`~~
   (trophy and cup art: the nine named elite trophies and three cup finals, found
   in `FrontEnd_LoadTrophyTextures` `0x8104d1d0`), ~~the weapon callout~~
   (`FUN_810626ce`: the law, the constructors' defaults and the icon order are
   read; drawn on the rules page and the objective page's glyph row), ~~the elite
   row~~ and the trophy glyph of the objective page, ~~the restriction glyphs~~
   and forced craft on the glyph row, and ~~the forced craft's two quads~~
   (`FUN_81061808`, disassembled) are drawn - evidence in
   `docs/ghidra/functions/vita-2048-eu-v104/weapon-callout.md`. **Still open, in
   order of player impact:** (a) page kind `3` (`FUN_81052810`, probably
   unreachable) and the leaderboard's personal record row (a per-event result
   store - `oag_game::records::Store` keeps only the medal today); (b) the
   slide-in/fade animation; (c) the objective page's forced-craft scale (chosen
   0.7) and the trophy heading's `320.0` fit; (d) verify the page order and the
   callout placement against a live Vita3K frame of an event with weapons, a
   trophy event and a passed event - no reference frame exists for any of them;
   (e) the thin vertical line at the left edge of the callout icons in captures,
   unchecked: texture content or sampling.
7. **Closed 2026-09-28** - see the "Open" section's own entry above for what
   landed (`grid_craft`, `Options::grid_teams`, `apply_grid_teams`) and where
   it is measured and verified.

## From the HANDOVER.md index (moved 2026-09-25)

2026-09-21. `SP.xml` is a "mjolnir" typed-instance database, 288 instances across eight typedefs, five named in-file by their own `type=`/`typedefid=` pairs (overturns a prior guess that a 20-instance typedef was `TrackDefinition` - it is `WeaponSetDefinition`) and four classified by field shape. `oag_tables::mjolnir(::campaign)` reads it, `oag_2048::campaign` names 2048's own facts, `race::load_event` resolves an event onto `race::Options`; `--event "2048 - Event 3"` loads Park/Flash/2 laps/`single_race`. **Finding**: `Mode::Eliminator` is real and already wired for 2048, so no event kind needs refusing. **2026-09-21: the objective law and the unlock graph both closed** - `M_OBJECTIVETYPE`'s four ordinals are measured against every `(pass, elite)` pair the file's 80 fully-authored events carry (2048's own medal law is genuinely two-tier, not three), and `oag_2048::campaign::unlock_gates` plus `Store::record_campaign`/`Frontend::refresh_campaign_progress` persist a finished event and gate the map by it - see `../../docs/formats/2048-campaign.md`'s "The objective law"/"The unlock graph" sections and `crates/game/tests/vita_2048_campaign_progress_ground_truth.rs`. Open: the `GameMode_*` class binding, `WeaponType`'s bit layout, Elimination's own unidentified `BEAT_VALUE` metric, `M_X`/`M_Y`'s projection (2026-09-21: they are real fields, read into `GameModeBase+0x2c4/0x2c8`, but what turns that into the DLC tiers' own `+0x15c/0x160` cache is still unfound, see the front-end thread below), `MP.xml`
