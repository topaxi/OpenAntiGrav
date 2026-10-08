# Race modes

Four modes are implemented. Three are single-ship - **time trial**, **speed lap**
and **Zone** - and came first because none of them needs opponents, weapons or a
grid: they are the part of the race layer that can be finished rather than
stubbed. The fourth, **single race**, is the first with weapons, and it exists
because pickups have nowhere else to happen; see below.

**The disc names three more.** `Tournament`, `Elimination` and `Head2Head` sit
alongside the four above in `Single Player`'s own `Mode` list; all three are
now implemented (below). Above all seven of them sits a
`RACE CAMPAIGN` main-menu entry - a persistent grid of events with medals
and unlocks, not a single race repeated - whose two screens now draw and
launch a cell in any of the six modes this engine runs; see
`docs/ui/campaign-screens.md` and `docs/architecture/persistence.md`.
[`race-setup.md`'s Race Campaign
section](../formats/race-setup.md#the-race-campaign-the-discs-own-campaign-grid-shape-yes-content-no)
has the 2026-09-08 scoping pass: the campaign's **screen flow and grid shape**
are fully authored (a two-level hex grid, `Grid Selection` then `Cell
Selection`, sharing its 32-cell widget with the player's own custom-grid
editor), its **medal-target widgets** are authored down to gold/silver/bronze
colour, and its **unlock gating** is the same named `Grid0`..`Grid10`
mechanism this page's circuit/craft-variant unlocks already use - but **which
track, mode and medal times actually populate a given cell was not found
authored anywhere this pass could read**, and is flagged there as a Ghidra
question for whoever picks it up next. English text for `Tournament`,
`Elimination` and `Head2Head` is recovered in full on that page, including
three previously unrecorded Eliminator mechanics (no pickup absorption,
scaled weapon damage, a kill-count ending rather than a lap count) and all
eight tournament placement strings.

**Neither is invented - both are measured on the running original and now
enforced in code, not just true by construction.** `Mode::has_opponents` and
`Mode::weapons_enabled` return `false` for the three single-ship modes: the Custom Race screen
greys `AI DIFFICULTY` to `N/A` and `WEAPONS` to `Off` for time trial, speed lap
and Zone alike, screenshotted for all seven of the original's race types on
2026-08-10 - see
[shield.md](../ghidra/functions/psp-pulse-usa/shield.md#g_weapons_enableds-per-mode-default-spelled-out-and-checked-against-every-race-type).
Two consequences in `crates/raceplay/src/lib.rs`:

- **The grid never spawns opponents for any mode yet.** For the three
  single-ship modes that is the original's own answer; for a single race it is
  not - `MSC_EVENT_SR` promises "a full grid of opponents" and the AI
  `DIFFICULTY` row is selectable. Nothing drives an opponent here, and seven
  parked hulls wearing the player's livery is a worse race than an empty track,
  so `Mode::has_opponents` answers `false` for it too and says why.
  The grid *arithmetic* is right and measured - [grid.md](../ghidra/functions/psp-pulse-usa/grid.md)
  took it against a real *Single Race*, the "one reachable race type with a full
  field" - it is only unused. `Options::opponents` is the escape hatch that
  measurement's own ground-truth test and `--opponents` on `just play` use to
  place all eight without a mode asking for them.
- **`Weapon Pad`s decode but are drawn and armed only in a single race.** The original does the
  equivalent at a different layer - `World_CollectNodeLists` clears the
  node's own visibility bits rather than skipping the decode - documented on
  [pads.md](../ghidra/functions/psp-pulse-usa/pads.md#a-weapons-off-race-neither-draws-weapon-pads-nor-can-trigger-them).
  `Scene::new` makes the same "decoded, never uploaded" choice for the same
  reason: the geometry decode is asset-pipeline correctness, mode-independent,
  and `the_weapon_pads_are_drawn_where_they_trigger` checks exactly that on
  every mode's own default settings.

  **On the original, this is now cross-title.** The user, playing HD directly,
  reports time trial, speed lap and Zone draw no weapon pads there either
  (2026-08-31, reported from play, not yet screenshotted against the disc the
  way the PPSSPP pass was) - so the rule is not a Pulse peculiarity.

  **In this build, `Scene::new`'s gate never reached HD's own pads, and that
  was a bug this report caught (fixed 2026-08-31).** `Mode::weapons_enabled`
  is title-agnostic, but the PS3 path never fed it anything to gate: HD's
  circuits load through `oag_mesh::mesh::rcs::build_scene`, which used to
  draw weapon-pad geometry as part of the track's own world-space chunk pass
  rather than through a separate `Weapon Pad` node the way the PSP-shaped path
  does, so `oag_raceplay::load` set `weapon_pad_model` to `None` on that
  path and the mode gate had nothing to act on - HD weapon pads drew in every
  mode. `build_scene` now splits `Weapon Pad` chunks into their own model
  (`crates/mesh/src/mesh/rcs/pads.rs`), and `geometry::track_model`
  (`crates/raceplay/src/load/geometry.rs`) hands that model to the same
  `weapon_pad_model` slot the PSP-shaped path already fills, so the existing
  gate covers HD for free. Pinned on the real disc by
  `crates/render/tests/hd_weapon_pad_split_ground_truth.rs`. `Speedup Pad`
  chunks are untouched - nothing gates them by mode, on this disc or the
  original's.

The rules live in `crates/race`; how a lap is decided at all is
[lap counting](lap-counting.md), and it is a convention rather than a recovery.

Selected on the RACE menu page, which opens on the time trial, or with
`--mode time_trial|speed_lap|zone|single_race|eliminator` on the command line. `--race` skips the menus,
so the flag is the only way in on that path.

## What each mode is

| | Time trial | Speed lap | Zone | Single race | Eliminator |
| --- | --- | --- | --- | --- | --- |
| Laps | 3/4/4/5, per class | unlimited | unlimited | 3/4/4/5, per class | unlimited |
| Throttle | the player's | the player's | the mode's | the player's | the player's |
| Ends by itself | after the class's own last lap | never | **not implemented** - see below | after the class's own last lap | on a kill count - see below |
| Weapons | off | off | off | **on** | **on, scaled up** |
| Free turbo a lap | **yes** | **yes** | no | no | no |
| HUD layout | `TimeTrial_HUD.xml` | `TimeTrial_HUD.xml` | `Zone_HUD.xml` | `Arcade_HUD.xml` | `Elimination_HUD.xml` |
| Environment | the circuit's | the circuit's | **its own**, see below | the circuit's | the circuit's |

Time trial and speed lap sharing a layout is the disc's arrangement, not a
shortcut: there is no `SpeedLap_HUD.xml`, which is why
[the HUD page](../ui/hud.md) counts five layouts for six modes.

## Time trial

**Per speed class, and confirmed live at all four rungs, 2026-09-09: 3 Venom, 4
Flash, 4 Rapier, 5 Phantom.** A Custom Race launched once per class under
PPSSPP - `pulse-psp-usa.chd`, Talon's Junction White, no campaign cell in play
at all - read `Lap 1 of 3`/`4`/`4`/`5` in that order, matching
[single race](#single-race)'s own 3/4/4/5 census exactly. That settles the
question this page used to leave open: the census was not campaign-only, and
the flat `3` this crate used to carry for every class was right for Venom and
wrong for the other three, the same shape [single race](#single-race)'s own fix
already had. `Mode::TIME_TRIAL_LAPS_BY_CLASS`, confidence **90**, level with
[single race](#single-race)'s own table: that one rests on a bigger census
(236 cells against this one's 47) plus a single live point, this one on a
smaller census plus a live point at every rung it has - the stronger half of
the same trade.

A run that drives past the last lap starts a fresh attempt with the best time
cleared rather than ending outright - see
[the PPSSPP debugger notes](../reverse-engineering/ppsspp-debugger.md) - which
this crate still approximates as an ending; see
[what happens when a race ends](#what-happens-when-a-race-ends).

The count is *configuration* in the original rather than a constant - the
race-setup format string carries `laps="%d"`.

Weapons and AI fall out of the mode rather than being set: selecting TIME TRIAL on
the original's Custom Race screen flips WEAPONS to OFF and AI DIFFICULTY to N/A on
its own. The RACE page's WEAPONS row does the same here (greyed, showing OFF); AI DIFFICULTY
is greyed but still shows its stored value rather than N/A.

**Not implemented:** the fresh-attempt-behind-you behaviour. The run stops at the
end of the class's own last lap.

## Speed lap

Chasing one fast lap; it never ends on its own and escape leaves.
`Mode::laps_target` returns `None`.

**The census's `7` is real outside the campaign too, and it still does not end
the race - both halves checked live, 2026-09-09, rather than assumed. Confidence
75.** A Custom Race speed lap - no campaign cell in play - reads `Lap 1 of 7`
on Venom under PPSSPP, the same `Lap X of Y` widget Time Trial uses, so the
`42`-cell campaign census (all reading `7`) was never a campaign peculiarity.
But Speed Lap's own in-race pause menu carries a seventh row, `END SESSION`,
that neither Time Trial's nor [single race](#single-race)'s pause menu has -
both are otherwise identical, six rows each, and both are modes that *do* end
on their own lap count. That is the falsifier this claim needs, checked rather
than assumed safe to skip: three modes screenshotted, and the one extra row
sits exactly on the mode whose own text says it never ends. A pause menu that
offers a dedicated way to deliberately conclude an open run is a mode that
does not conclude one on its own - agreeing with `MSC_EVENT_SL`'s text: *"never
ends, escape leaves."* 75 rather than higher because this is a correlated UI
signal standing in for a lap-8 crossing nobody watched directly: driving far
enough past lap 7 to watch the race actually end or not was tried and
abandoned - open-loop script replay could not complete even one lap of Talon's
Junction in nine minutes of real time under PPSSPP, the same drift this
project's own verification-protocol notes already name as fatal past one lap.
`Zone`'s `0` is left as before: read as the attribute's "not applicable"
spelling for a mode that counts zones instead, untested either way.

**This mode is not documented anywhere in the original's data that has been read.**
It appears in no mode enumeration in this repository's notes and has no HUD layout
of its own. It is here because it is a trivially correct subset of the time trial,
and if a recovered speed lap turns out to differ, this is the one to re-check.

## Zone

Recovered, and recovered late: the evidence is
[zone-mode.md](../ghidra/functions/psp-pulse-usa/zone-mode.md), which reads
`Zone_Update` (`0x0882f5cc`) end to end. The identification itself is now
confidence **84** rather than the 50 it sat at - the mode factory picks the Zone
constructor with the byte-identical selector `Ship_UpdateEngine` uses for its
four-corner branch.

What runs:

- **The zone number steps every 10.0 seconds of accumulated frame time**, and on
  nothing else - not distance, not laps, not score. Confidence **84**. At the
  fixed 60 Hz of [ADR-0007](../architecture/adr/0007-fixed-timestep-vs-original.md)
  that is every 600 ticks.
- **The accumulator resets to zero rather than subtracting the step**, so each
  zone discards the frame overshoot and the zone clock runs fractionally slow.
  Confidence **82**. This is behaviour, and there is a test asserting it so nobody
  tidies it away as a rounding artefact.
- **Thrust is `start + increment * zone`**, replacing the throttle entirely: a
  Zone craft accelerates with nothing held down. Uncapped - the ordinary engine
  branch clamps against `0.5 * speed + accelcap` and this one does not.
- **Score** rises by one every tick, plus 500 for a zone completed without
  touching anything, which also restores shield.

### The colour grade escalates too, on the two HD-lineage titles

Wipeout HD/Fury and 2048 ship a `.effectSettings` table naming one full
palette per speed class, laid over whichever circuit the race runs on -
[effectsettings.md](../formats/effectsettings.md) has the format and
`oag_raceplay::zone_grade` the application. **What selects a stage during a
race is recovered on 2048 and not on HD.**

2048's Zone HUD widget (`Hud_UpdateZoneSpeedClassWidget`, `0x81197d6c`) walks
a seventeen-record table of descending **zone-number** thresholds
(`g_zone_speed_class_thresholds`, `0x8151faf8`) and writes the matched
record's index into the per-craft field `Zone_UpdateStage` (`0x81044cfc`)
clamps and shows. So the grade steps at zones `2, 9, 17, 33`, then every five
to `90` - **every few zones, not every zone** - and a race opens on stage `1`,
never on the table's own `Start` row. Confidence **78**; full evidence on
[zone-environment-fallback.md](../ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md).

HD/Fury's own Zone stage index comes from a per-craft field
(`craftArray[n]->+0x640`) whose writer has not been found, so an HD Zone race
loads its table and rests on stage `0`. Its *Detonator* mode, by contrast, is
fully recovered: `Detonator_UpdateRace` (`0x00067b40`) starts a counter at `1`,
steps it by one per event and ends the race at `15`, which is exactly the
stage count `detonatormode.effectsettings` names.

**A caveat that belongs with the timing above, not with the ladder**: the zone
number this engine feeds that ladder still steps on Pulse's own recovered
10-second rule, which has never been checked against 2048's or HD's
executables. The ladder is 2048's; the clock driving it is not.

### The ship model is fixed, the livery is not

`Ship_LoadModel` loads `<Team>\Zone.vex` instead of `<Team>\Ship.vex` under the
same selector, and every team's `Zone.vex` decodes to the same hull - see
[zone-mode.md](../ghidra/functions/psp-pulse-usa/zone-mode.md#the-ship-model-is-not-the-players-own-hull).
Confidence 84. `oag_livery::entry::ship_entry_name` picks the model this way, and
the menu greys the TEAM row while MODE is Zone so a player is not offered a
choice that no longer changes the shape drawn - only the colour it is drawn in.

**That is Pulse's arrangement, and the other two titles do not share it.**
`oag_title::ZoneCraft` is the axis, and it splits the corpus exactly the way
`ZoneCircuit` does - which is the interesting part, because it means each title
made *one* decision that shows up twice:

| | Zone's circuit | Zone's hull |
| --- | --- | --- |
| Pulse | the race circuit's directory, `zone_`-prefixed | the player's team, `Zone.vex` |
| Pure | `Data\Zone\NN_Zone\track.vex` | `Data\Ships\Zone_01\Ship.vex` |
| HD / Fury | `/data/environments/zone_N/track.vex` | `/data/ships/zone/ship.vex` |
| 2048 | the race circuit itself, unchanged | the player's own ship, unchanged |

Pulse hangs Zone off the entities a race already has; Pure and HD give Zone
entities of its own, and there the player's team stops reaching the hull at all.
Pulse's is the only one with a recovered *selector* behind it (`case 6`,
confidence 84); the other two are name resolution at 94, and neither title's
executable has been read. HD's was already half-recovered before anything wanted
it - `zone` is one of the four `oag_hd::names::MODE_SHIPS` read off the manifest.

**2048 is a fourth shape on both axes at once, not a variant of the other
three, and its source is play rather than the disc.** `oag_2048::race` used to
guess both by analogy: a Pure/HD-shaped Zone ship directory (`hdships\Zone`,
under 2048's HD-derived ship tree rather than its native one, so it never
resolved and failed every Zone race), and a Pure/HD-shaped Zone circuit - one
of the four `zone_1`..`zone_4` environments `dlc2.psarc` ships, which
`docs/formats/track.md` had already shown are Wipeout HD's own dedicated Zone
circuits reshipped verbatim. Playing the game settled both instead: 2048 has
no dedicated Zone craft and no dedicated Zone environment - the player flies
whichever of the twenty native ships they picked on whichever circuit they
picked, exactly as in any other mode. That agrees with a fact this title's own
data already carried unread: every circuit in the base package ships a
`ZoneMode2048.effectSettings` beside its `track.vex`, which only makes sense
if Zone runs on the circuit that file sits next to. The four ported `zone_N`
environments are real disc content, added by the same DLC that adds eight
ordinary HD-ported race circuits; they are simply not what either axis
answers here. Confidence 90 on both, the same bar the rest of `oag_2048::race`
holds itself to for an unread executable - see `oag_title::ZoneCraft::PlayerShip`
and `oag_title::ZoneCircuit::SameCircuit`.

**Correction, 2026-10-06 (craft only): the hull is shared, not the player's.**
The v1.04 executable's ship-model loader names `Data\art\published\hdships\Zone\Ship.vex`
for game mode 6 without reading the craft, and Omega's loader does the same
(`ZoneCraft::OwnShipAt`, confidence 75, one source tree read twice; evidence in
[vita-2048-eu-v104/zone-craft.md](../ghidra/functions/vita-2048-eu-v104/zone-craft.md)
and [ps4-omega-eu/zone-craft.md](../ghidra/functions/ps4-omega-eu/zone-craft.md)).
What the player picks is a livery on that hull. The circuit half above stands.

### Every title ships one Zone handling block; Pulse's executable never reads it

**The shipped answer on Pulse is the player's own team and class**, read 2026-10-02 from the executable
([zone-rest.md](../ghidra/functions/psp-pulse-usa/zone-rest.md), confidence **82**). All three titles carry a Zone-mode
craft directory - `Data\Ships\Zone_01` on both PSP titles, `/data/ships/zone` on HD - whose `handlingstats.xml` opens
`<Stats team="ZoneMode">` and authors **no `<Class>` block at all**. On Pulse nothing reaches it: neither `Zone_01` nor
`ZoneMode` is a string in `BOOT.BIN`, the definition lists no team of `type="Zone"` (only a `PI_TeamModel name="Zone"`
per team, the `Zone.vex` hull), and `Ship_LoadHandlingStats`, `Ship_LoadModel` and `Ship_InitCraft` all go through the
player's own team directory and `craft+0x70 = stats + class * 0x80 + 0x94`, with no Zone test. So this engine flying the
player's team numbers **is** the original's behaviour on Pulse, and the file is a leftover from Pure.

What Zone does replace, inside that handling: the engine (the auto-speed law), the brakes (disabled) and the hover
variant (`Ship_HoverFourCorner`, whose bank-to-yaw gain is `50.0` against the two-point law's `30.0`; ported).

**Pure and HD are unread.** Pure's definition marks `Zone_01` as `type="Zone"`, so a Pure Zone race presumably flies
that block; neither that executable nor HD's was read. Watch the near-miss on Pulse: `Data\Ships\Zone` is
`<Stats team="Zone">` **with** classes and is the unlockable Zone *livery*, a raceable team; `Zone_01` is the leftover.

### Zone flies its own environment, and the disc authors every bit of it

**The look is loaded, not computed.** A Zone circuit is a whole separate `.vex`
with its own meshes, its own light rig, its own `fogCube` and its own
`Skycube` - nothing here tints, desaturates or otherwise invents it, which is
[`CLAUDE.md`](../../CLAUDE.md)'s rule about not authoring a stand-in for what the
data already carries. The sky is the clearest single marker: of the 40 skies on
the PSP disc, the twelve with one material are exactly the Zone variants against
five or six for every race circuit ([skycube](../formats/skycube.md)).

**Where that environment lives is a title fact**, and it is one of the few axes
measured on all three titles rather than two:

| | Zone's environment |
| --- | --- |
| Pulse (PSP and PS2) | the race circuit's own directory, one extra `zone_`-prefixed file |
| Pure | `Data\Zone\NN_Zone\track.vex` - four circuits of its own, `type="Zone"` |
| HD / Fury | `/data/environments/zone_N/track.vex` - four circuits of its own |

Two shapes across three titles, which is what makes `oag_title::ZoneCircuit` a
type with two variants and no third "unknown" state. Pulse derives the Zone name
from the race one; Pure and HD cannot, because their discs author no mapping
between the two sets - asking what Vineta K's Zone variant is has no answer in
the data. Evidence and per-claim confidence: [track](../formats/track.md), and
the sweep is `crates/game/tests/zone_ground_truth.rs`.

**Sixteen of Pulse's twenty-four circuits can be raced in Zone**, declared by
`availableInZone="true"` and confirmed by probing all 24 by name - the attribute
predicts the file 24 times out of 24. A circuit without it carries no Zone
environment, so this is load-correctness rather than menu data, and `race::load`
says so by name when a Zone race asks for one that has none.

**The Race Box's Zone list is the sixteen** (`Session::tracks_for(Mode::Zone)`),
as the original's `TrackSelection_PopulateList` does: its `Mode == 6` byte `+0x16e`
is `availableInZone` (read 2026-10-02, [zone-start](../ghidra/functions/psp-pulse-usa/zone-start.md)). `catalogue::tracks_of_kind` and `Track::available_in_zone` are the two
listings such a filter would read; nothing dispatches between them, because
which one applies is a title fact and `oag_title::ZoneCircuit` is where that
lives.

**And the Zone HUD draws its labels with no values behind them.** A screenshot of
a Zone race shows `Lap`, `Score` and `Zone` positioned exactly where
`Zone_HUD.xml` puts them and reading blank, which looks like this change broke
something and is instead the pre-existing gap [hud.md](../ui/hud.md) records: a
mode's code substitutes string keys into widgets the layout positioned, that
substitution rule is unread, and Zone is one of the layouts explicitly scoped out
there. [zone-mode.md](../ghidra/functions/psp-pulse-usa/zone-mode.md) has the
same hole from the RE side - what raises `IG_HUD_PERF_ZONE` and what drives the
`Zone_Bar_*` widgets is inside the widget system, not the mode. The environment
swap does not touch any of it.

**Zone's racing line is authored separately from the race one**, which was worth
checking while the start line was a constant fitted on `16_Track`'s *race*
file. Across the sixteen: fourteen have identical control-point counts and one
environment does not - `10_Track` at 844 race / 848 zone, and its reversed twin
`26_Track` at 847 / 852. The line is now derived from each file's own spline
(`Course::START_LINE_ADVANCE`, [lap-counting.md](lap-counting.md)), so the
difference no longer inherits anything. Nothing about Zone lap timing is verified
against the original either way.

### Zone's numbers are not in this repository

`start`, `increment` and `recharge` are read at runtime from
`<Handling><Global><Zone/></Global>` in `Data\XML\HandlingStats.xml` on the
player's own disc, exactly as the original reads them - see
[handling stats](../formats/handling-stats.md). Nothing is invented and nothing is
committed, which is what
[ADR-0006](../architecture/adr/0006-no-copyrighted-content.md) requires.

A source without that file races Zone with no auto-speed, and the load report
says so rather than substituting numbers.

### Two approximations, both deliberate

- **The gate on the speed law.** The original applies it only when
  `(flags & 1) && !(flags & 2)` on the undecoded `craft+0x1c0`, and writes `0.0`
  when the test fails. Bit 0 is known to be the ground-contact bit - it is the
  same bit that gates the brakes - so groundedness stands in for it. Bit 1 is the
  grid state (read live 2026-10-02, [zone-start](../ghidra/functions/psp-pulse-usa/zone-start.md)),
  so `on_grid` stands in for it: a Zone craft stands through the countdown.
- **The shield recharge is live as of 2026-08-10.** It is implemented, it clamps
  to the ship's maximum the way `Ship_SetShield` does, and the pool now depletes:
  wall contact spends `|p| * 0.05 * 0.7` of it through `oag_physics::damage`. So
  a clean zone pays back something a dirty run actually lost. See
  [shield](../ghidra/functions/psp-pulse-usa/shield.md).

### Zone has no ending yet, and now we know what it should be

**A Zone run ends when the ship is destroyed.** The disc says so: `ER_ZONE_DEST`
reads `"Ship destroyed on zone"`, and `MSC_EVENT_ZONE` describes the mode as
*"survive for as long as possible before crashing out"*. Confidence **80**.

That is a change of footing rather than of behaviour.
[`zone-mode.md`](../ghidra/functions/psp-pulse-usa/zone-mode.md) first recorded only
that a run ends on bit 12 of `entity+0x860`, with 25 confidence that the bit meant
shield depletion - a guess, and the reason the ending was left out. The string
table corroborates it.

**Implemented 2026-08-10, and the chain is the original's own.** The pool
empties, `Ship_Damage` (`0x088439ac`) puts the craft into state 4, the explosion
runs for half a second, and state 5 sets **bit 12 of `entity+0x860`** - the bit
`Zone_UpdateRacing` ends a run on, and the one
[`zone-mode.md`](../ghidra/functions/psp-pulse-usa/zone-mode.md) recorded as set
by nothing findable. `oag_physics::damage::CraftState` is the state machine and
`oag_race::RaceState::eliminate` is the ending.

Zone is the only one of *the three modes above* that can reach it, because it is
the only one of them that races with damage on - see below. **A single race
reaches it too**, and its own ending is settled the same way: see
[A destroyed craft is out of a single race](#a-destroyed-craft-is-out-of-a-single-race-not-respawned).

**What is deliberately absent is the presentation.** The original plays
`_BLOWUP`, hides the HUD and swings the camera into its mode 5 on the way
through state 4; this engine has none of those, so a wrecked run simply stops.
That is a rendering and UI gap rather than a rules one, and `race.rs` says so at
the point where it would go.

`MSC_EVENT_ZONE` also ends *"Clear the target number of zones to win the event"*,
so a Zone event has a target zone count. That is progression data - M7 - and is
not modelled. **The number itself is now known**: the 2026-09-08 campaign
scoping pass found a 24-entry per-track record family in `FEData.wad` (see
[race-setup.md](../formats/race-setup.md#fedatawad-authors-real-per-track-numbers---race-times-lap-times-ai-difficulty-and-two-mode-independent-constants))
where every one of the 24 carries an identical `<Targets Zone="25"
Elimination="10">` - so the zone target is **25**, flat across every circuit,
and Eliminator's kill target (the count `ER_YOU_ELIM`'s ending refers to) is
**10**, also flat. Confidence 90, a clean census over all 24 files. Not
wired into `oag_race` - this is the number, not the implementation.

**Refined the same day in Ghidra, and the two numbers are not equal in
standing.** `Eliminator_UpdateKillTarget` (`0x0882ce18`) reads the kill target
from **the campaign cell's own gold target** when one is in play, and only falls
back to a global (`DAT_08b30fb0`) otherwise; the campaign's 22 `Elimination`
cells all author `10`/`7`/`5` for gold/silver/bronze, so `10` is both the
`FEData` constant and the campaign's own gold threshold, and the race ends when
**any** ship reaches it (`entity + 0x8d8` is the kill count). Zone is the
opposite: the campaign's 16 `Zone` cells author gold targets of **18 to 24, per
track**, never 25 - so `FEData`'s `Zone="25"` is a default for some other path,
not the campaign's target. See
[race-campaign.md](../ghidra/functions/psp-pulse-usa/race-campaign.md).

### The string table also settles why shield does not matter in the other two

`MAN_P3_PG1_ENER`: *"The Time Trial and Speed Lap events will recover your ship
energy automatically."* `MSC_EVENT_TT` and `MSC_EVENT_SL` each repeat it. So a
depleting shield was never going to end those two modes, and the missing pool
cost them nothing.

**That sentence turned out to be a description of a mechanism, not a design
note.** `Race_ReadSetupOptions` (`0x08896b84`) clears `g_damage_enabled` for
game modes `{5, 7, 10}`, and clearing it is what makes the pool recover: the
original adds `dt * 4` a tick and floors the result at 20
(`oag_physics::damage::regenerate`). Modes 5 and 10 also have weapons cleared
and **share one constructor** in `Race_CreateModeObject` (`0x0882112c`), which
is the time-trial/speed-lap relationship this page already describes. So the
manual string and the switch statement are two independent records of the same
rule, and `oag_gameplay::damage_rules` is the port of it - a time trial's pool
recovers automatically because the original's does, not because this project
decided it should.

Both also grant *"a free turbo pickup once per lap"*, and **it is implemented as
of 2026-08-11** - on the lap edge, into an empty slot only, including lap 1's
own edge (fixed 2026-08-19; the field started every race one turbo short
until then), which since 2026-10-04 is the first line crossing, read from
`TimeTrial_UpdateRacing` (`0x0882ddd8`) - see [pickups.md](pickups.md). The manual string is
no longer the only record of it: `TimeTrial_HUD.xml` authors a
`PickupBackground` and exactly one weapon icon, `TurboIcon`, where `Zone_HUD.xml`
authors none and `Arcade_HUD.xml` authors all thirteen. Two shipped files
agreeing puts it at confidence 85. What a fired turbo does is the recovered `1.2`
engine multiplier and **not** `Engine::turbo`, which is a different term under
different flags and is still unapplied - see [pickups](pickups.md).

**Also out of scope:** the `Zone_Bar_*` widgets and the `IG_HUD_PERF_ZONE` /
`IG_HUD_NEW_ZONE_RECORD` banners. Their writers were not found, and the
zone-specific graphics were scoped out of this work.

## Single race

**The mode weapons happen in, and the reason it exists.** The original clears
`g_weapons_enabled` for all three modes above and then hides every `Weapon Pad`
and empties the trigger list, so no mode this engine had could hand out a pickup
at all - not "a crossing does nothing", but "there is nothing to cross". A
pickup system therefore needed the mode that has weapons, and this is it.
See [pickups](pickups.md).

What it is: a three-lap race with weapons on, damage on, and `Arcade_HUD.xml` -
the layout whose pickup widgets have been parsed since the HUD work and drawn by
nothing until now.

**Three departures from the original, all of them absences.**

- **No opponents.** `MSC_EVENT_SR` promises "a full grid of opponents" and the
  original's `AI DIFFICULTY` row is selectable. That is the AI item; until it
  lands the field is empty rather than parked, for the reason above.
- ~~**No `WEAPONS` row.**~~ **There is one now (2026-09-30).** The original calls weapons "optional" here
  and leaves the row editable; this build's RACE page offers the disc's own `On`/`Off` in a single
  race (`race.weapons`, default `On`), greyed and showing the mode's own answer elsewhere -
  [`race-setup.md`](../formats/race-setup.md). Off drops the weapon pads and clears the damage
  rules' weapons flag together (`Options::weapons_override`).
- ~~**The lap count is ours.**~~ **Not any more, and it is no longer flat.**
  The lap count is **per speed class, not per mode**:

  | Class | Laps |
  | --- | ---: |
  | Venom | 3 |
  | Flash | 4 |
  | Rapier | 4 |
  | Phantom | 5 |

  Confidence **90**, a flat census over the 236 authored `PI_Cell` records in
  `Data\Plugins\grids\grid_00.xml` .. `grid_15.xml`: every cell carries a
  `laps` attribute and across all 236 it is exactly these four values, with no
  exception. See
  [race-campaign.md](../ghidra/functions/psp-pulse-usa/race-campaign.md).

  **The plumbing now exists.** `Mode::laps_target` takes the speed class and
  indexes `Mode::SINGLE_RACE_LAPS_BY_CLASS`; `RaceState::new` takes it too, so
  no construction site can default it silently. The class reaches `oag_race` as
  `oag_tables::handling::SpeedClass` - a crate it already depends on - and the
  disc's own spelling of the class name is resolved onto that enum one layer
  out, in `oag_raceplay::Race::start`, which is the layer that already carries
  the name. Wipeout Pure's `VECTOR` rung has no measured lap count of its own
  (no Pure campaign census has been read), so it falls back to Venom's `3` with
  a warning in the load report; that fallback is **chosen, not measured**, and
  carries no confidence score.

  **This table is the fallback, not the authority, and the retirement clause
  fired 2026-09-14.** A campaign launch now takes the lap count from its own
  cell: `race::Options::laps_override`/`Setup::laps_override` carry
  `oag_tables::race_campaign::Cell::laps` straight through to
  `oag_race::RaceState::laps_target`, overriding whatever this table would
  have answered - `Session::launch_campaign_cell`
  (`crates/game/src/main/session/campaign.rs`) is the one caller. **Only for
  `Time Trial` and `Single Race`**, the two modes this table's own
  `laps_target` already answers `Some` for - `Speed Lap` and `Zone` both
  author a `laps` attribute too (`7` and `0`) that is display convention,
  not an ending, and applying it the same way would end a race the original
  measurably does not; see `race::Options::laps_override`'s own doc. This
  table stays necessary regardless: a Custom Race started outside the
  campaign still has no cell to read one from.

  **A Phantom race's fifth lap has no split row and is not recorded.**
  `MAX_RECORDED_LAPS` is `4` because HD/Fury's `HUD_lap_times.xml` composes
  exactly four rows and no shipped layout on any of the four titles authors a
  fifth - see [hd-hud.md](../formats/hd-hud.md). The array was deliberately not
  widened: five rows would invent a row no measured title has, and would change
  `World`'s size and so the committed state hash, to store a number nothing can
  display. The fifth lap is still counted and still ends the race; only its
  split time is dropped.

  The earlier reading, kept because it is what a future per-class pass on
  another title should start from: the speed-class help text (`MSC_LOAD_VENOM`
  etc., see
  [race-setup.md](../formats/race-setup.md#the-full-mode-list-the-disc-authors-seven-against-this-projects-four))
  hedges "most events" at confidence 78 and says the same thing in prose -
  Venom mostly 3, Rapier and Phantom usually 4 and 5. The census is the hard
  version of it.

### A destroyed craft is out of a single race, not respawned

**A craft destroyed mid-race takes "Ship destroyed" as its result instead of a
finishing place. It does not come back.** Confidence **75**, and the evidence is
the disc's own text plus the executable's field list - the same method
[`ER_ZONE_DEST`](#zone-has-no-ending-yet-and-now-we-know-what-it-should-be) was
settled by, one notch lower because the field-list ordering is read rather than
traced to the widget that consumes it.

The results screen's field list sits at `0x08a82dfc` in `psp-pulse-usa`'s
`BOOT.BIN`, in layout order rather than alphabetically, and reads:

```
ER_RACING  tablehighlight  BigTopText  ER_RES  Line1 .. Line8
ER_SHIP_DES  ER_1STP ER_2NDP ER_3RDP ER_4THP ER_5THP ER_6THP ER_7THP ER_8THP
ER_TT_COM  ER_SL_COM  ER_ZONE_COM  "%s %s"  ER_ELIM_COM  ER_END_TOUR
```

`Line1`..`Line8` are the eight scoreboard rows, and the run that follows them is
that row's **status**, one of three: `ER_RACING` *"Racing"*, `ER_SHIP_DES`
*"Ship destroyed"*, or `ER_1STP`..`ER_8THP` *"1st place"*..*"8th place"*. A
status only means something if the alternatives exclude each other, so a craft
that was destroyed did not go on to take a place - if destruction were followed
by a respawn, the row would end the race either still `Racing` or holding a
place, and `ER_SHIP_DES` would never appear on it.

`ER_SHIP_DES` is `"Ship destroyed"` in English (`"Schiff zerstört"`,
`"Nave destruida"`, `"Nave distrutta"`), read out of the language tables in
`Data.wad` beside `ER_ZONE_DEST`'s `"Ship destroyed on zone"`. The two strings
being distinct is itself part of the argument: Zone gets its own wording because
Zone is a different mode reaching the same state, not because destruction means
something different there.

**Respawn belongs to Eliminator, and the strings say that too.** `ER_DEATHS`
reads *"Deaths:"* and `ER_YOU_ELIM` *"You have been eliminated!"*, both beside
`ER_ELIM_COM` *"Eliminator complete - "*. A mode that counts deaths is a mode
you come back in; a single race has no such row.

**So `oag_race::RaceState::eliminate` ending a single race is right**, and as
of 2026-09-16 the reader of the bit is found: `ArcadeRace_UpdateRacing`
(`0x0882c5c4`) ends the race the moment the *player's* destroyed bit is set,
confidence 82 ([shield.md](../ghidra/functions/psp-pulse-usa/shield.md#who-ends-a-single-race-on-the-destroyed-bit-and-who-comes-back)).
**An opponent is a different matter, and it is out for good too** (corrected
2026-10-02, measured live, confidence 85): nothing watches its bit, so it runs
the original's own state 5 (`1.5` s) and state 6 (`0.8` s for an AI craft) and
then stays in state 6 with its wreck at rest - `Ship_UpdateRespawn` is state
8's update, the Eliminator's, and a write watchpoint on the craft's state saw
no writer after 5 to 6 in 13 s ([shield.md](../ghidra/functions/psp-pulse-usa/shield.md#state-6-measured-on-ppsspp-a-wrecked-ai-craft-stays-down-2026-10-02-pulse-state6)).
State 6's timer plays *"contender eliminated"* at its zero crossing; that
announcer line is not wired. This page, and the port, used to return the
opponent to the track with a full pool after those two dwells
(2026-09-16 to 2026-10-02).

## Eliminator

Race for kills, not position: a full grid, weapons on and scaled up, no lap
target, and a destroyed craft respawns rather than sitting out. Implemented
2026-09-08, from `MSC_EVENT_ELIM`'s own text and the campaign RE pass the same
day settled the ending on:

> *"Eliminator: race for kills, not position, against a full grid of trigger
> happy contenders in a weapons-heavy environment. Weapons do more damage,
> and you cannot absorb pickups, but you regain health after each lap. The
> lap count is not fixed, and the race will end when the kill count is
> reached."*

Four mechanics come straight out of that sentence, each with its own recovery
status:

- **A full grid.** `Mode::has_opponents` answers `true`, the same reading
  `Mode::SingleRace` already has and for the same reason - the text promises
  one outright.
- **Weapons on, and scaled up by a whole second table.** `Mode::weapons_enabled`
  answers `true`, and `race::load` opens `Data\XML\WeaponStats_Elimination.xml`
  instead of the ordinary `WeaponStats_Race.xml` for this mode alone -
  `oag_title::weapons::Weapons::elimination`, a field that existed and named
  this exact file before anything read it. Confidence 90: `DAT_08b32428`
  selects the file (not the speed class) and was measured `0` in a single race
  and `1` in Eliminator, both at Venom - see
  [`missile.md`](../ghidra/functions/psp-pulse-usa/missile.md). This is what
  "weapons do more damage" *is* in the recovered engine: a second, real table,
  not an invented multiplier. A title with no second table (Pure) falls back
  to the ordinary one and says so in the load report.
- **Pads refresh an order of magnitude faster.** `<WeaponPad
  elimination_refresh_time>` is `0.05` seconds against the ordinary
  `refresh_time`'s `0.55` on both shipped PSP discs -
  `oag_tables::handling::global::WeaponPad`'s own doc comment already named
  Eliminator by name before this mode existed to read it. `race::load` selects
  it the same way it selects the weapon table above, falling back to the
  ordinary figure for a title with none. See [pickups](pickups.md#the-refresh-timer-is-a-debounce-not-a-respawn).
- **No energy from an absorb; the absorb is a one-second Shield instead
  (2026-10-03, read statically, confidence 80).** `Mode::pickups_absorb` answers
  `false` for Eliminator alone. Until 2026-10-03 both absorb paths then kept
  the pickup, which is not what the original does: `Ship_AbsorbHeldPickup`
  (`0x08844ec4`), in `g_game_mode` 8 or `0x12` with a weapon held, calls
  `0x088612e8` (writes `5` into both held-id copies, `craft+0x1bc` and
  `+0x1c0`), then `Weapon_RequestFire` (`0x08862d9c`), whose jump-table entry 5
  is `ori 0x20`: fire bit `0x20`, `Shield_Fire`
  ([shield-pickup.md](../ghidra/functions/psp-pulse-usa/shield-pickup.md)). Every
  per-weapon arm of the absorb table then skips `Ship_AddShield` on the same
  flag, `0x08862bc0` writes `-1` into the slot, and the tail skips
  `Ship_PlayAbsorbFeedback`. `WeaponStats_Elimination.xml` authors the Shield's
  `time` as **1 s** (5 s in `WeaponStats_Race.xml`) and its pad odds as zero in
  every column, so this is the only shield the mode has. Ported for the player
  (`Race::eliminator_absorb`); an opponent keeps its weapon, see "Held weapons are
  not a lever" below. Not confirmed live.
- **Health regenerates on a completed lap, in place of absorption - a fifth
  of the maximum, measured.** `Race::eliminator_lap_health_refill` runs on
  the same `outcome.lap_completed` edge the free Time Trial/Speed Lap turbo
  already reads, and as of 2026-09-16 the amount is the original's:
  `Ship_RefillLapShield` (`0x0883de30`) adds 20 % of the skill-indexed
  maximum through `Ship_AddShield`'s clamp and plays the absorb feedback,
  called by `Eliminator_UpdateKillTarget` on the player's craft when its
  crossing count goes up while racing. This used to be a full refill, chosen
  off the sentence alone. See
  [shield.md](../ghidra/functions/psp-pulse-usa/shield.md#ship_addshields-four-callers-and-ship_refilllapshield-0x0883de30).
  One stated departure: it refills every craft's lap here, where the original
  reads the player's slot alone.

### A destroyed craft respawns and is counted, not eliminated from the race

Where a single race or Zone reads `CraftState::Eliminated` as the end
(`RaceState::eliminate`), Eliminator reads it as a **death**:
`Race::tick_eliminator` runs once a tick, over the whole field, and for a
craft that has finished its `Eliminated` sequence it increments
`Standing::deaths`, credits a kill if one is owed (below), refills the shield
pool and puts the craft back on the track through the same recovery
`Race::respawn` already does for an off-track/`Reset` rescue - confidence 40,
inherited from that method's own doc comment, since where an Eliminator
respawn actually happens was not found in the executable either.
`ER_DEATHS` ("Deaths:") is this count's own results-screen row, and a mode
that authors one is a mode you come back into - the reasoning the disc's own
strings settled before any of this landed.

**A 1.5-second dwell before the respawn is measured, separately from the
explosion.** `oag_physics::CraftState::Eliminated`'s own doc comment records
the original moving on "after 1.5 s, into a respawn or the Eliminator's kill
bookkeeping" - `eliminator::DESTROYED_DWELL` is that figure, counted
from the tick the craft reaches `Eliminated` (i.e. *after*
`DESTROYED_DURATION`'s own half-second explosion has already run);
the Eliminator then waits state 8's `1.0` s (the player) or `0.8` s (anyone else, measured live) before the
respawn (`eliminator::eliminator_respawn_delay`, 2026-10-02, `shield.md`).

### Kill attribution is chosen, not recovered

**Nothing in the read executable was traced for how the original decides
*whose* weapon a kill belongs to** - only that the count exists, at
`entity+0x8d8`. `Race::last_damager` is this engine's own rule: the most
recent craft to land a *direct* weapon hit on the one that died is credited,
cleared the instant a wall or another cause deals damage instead, so a stale
hit from long before is never credited for an unrelated death. A splash-only
hit (inside a blast's radius but not the craft it directly struck) is not
credited, and neither is a wall-only death - both count the victim's own
death and nobody's kill. This is a design choice, stated as one, and it is
the piece of Eliminator most likely to need revisiting if the original's own
mechanism is ever read.

### The ending is measured, and the number it uses today is not the flat one first reported

`Eliminator_UpdateKillTarget` (`0x0882ce18`, confidence 80) is a real,
*measured* ending: the mode's own update reads a kill target and ends the
race the instant **any** craft's kill count reaches it - `RaceMode_SetState`
to state 3 - not only the player's. See
[`race-campaign.md`](../ghidra/functions/psp-pulse-usa/race-campaign.md).
`RaceState::eliminator_finished(target, kills)` takes both numbers from its
caller rather than deciding either itself, and `kills` is the field's own
highest `Standing::kills`, exactly matching "any craft".

**The target itself is per-campaign-cell, not a flat number - a correction to
this page's own earlier reading.** A first pass read `FEData.wad`'s
identical `<Targets Elimination="10">` across all 24 per-track records as a
flat census; the campaign RE pass the same day found the real source is
`Data\Plugins\grids\grid_00..15.xml`'s `PI_Cell` records, each with its own
gold-medal figure, and the values actually used are **10, 7 or 5** - never
one flat figure. `FEData.wad`'s `10` was one sample of three read as the
whole population. **A campaign Eliminator launch now fills this in from the
cell's own gold target, 2026-09-14** -
`Session::launch_campaign_cell` (`crates/game/src/main/session/campaign.rs`)
sets `Options::eliminator_kill_target` to `cell.gold`, so the campaign's own
`10`/`7`/`5` reach `Race::start` per race rather than one flat figure. A
Custom Race outside the campaign still has no cell to read, so
`Options::eliminator_kill_target: None` still falls back to
`oag_race::Mode::ELIMINATOR_KILL_TARGET_DEFAULT`, **5** as of 2026-09-30 (the race box's
first `KILLS` entry, see below) - the only figure a `--mode eliminator` run with no campaign
cell in play can have.

### The kill target of a non-campaign Eliminator is the race box's `KILLS` row, and it starts on 5 (2026-09-30)

`Hud_BindWidgets` (`0x0881fbec`) formats `KillsText` as `"%s (%d)"` of `IG_HUD_KILLS` and
the kill target: the campaign cell's gold (`cell+0xa0`) when one is in play, and
`DAT_08b30fb0` otherwise. **`DAT_08b30fb0` has one writer**, `RaceBox_ApplySetupGlobals`
(`0x088e5ff4`, the race box's commit), which `atoi`s the selected entry of the `Eliminations`
list - `5`, `10`, `15`, `20`, `25` - into it. A fresh profile's Racebox, with `RACE TYPE
ELIMINATOR` chosen and the row untouched, shows **`KILLS 5`**, and a Racebox Eliminator ends at
exactly 5 kills. Confidence 90; the widget identification (`+0x120` is `Eliminations`) is 78.
Evidence page: [eliminator-kill-target.md](../ghidra/functions/psp-pulse-usa/eliminator-kill-target.md).

The XML authors `Default="10"` on that list and the running screen does not honour it (why is
not determined). `oag_race::Mode::ELIMINATOR_KILL_TARGET_DEFAULT` is therefore **5**, the
list's first entry; a campaign cell's gold (10, 7 or 5) still wins. This build's RACE page
now has the `KILLS` row (2026-09-30): its values are read off the disc's own `Eliminations`
list, it is greyed outside an Eliminator, and the pick is the Eliminator's kill target
(`oag_game::settings::Race::eliminator_kill_target`). RACE REMIX has no such row and keeps the
default.

### Who is credited with a kill, and how this build's Eliminator reaches five (2026-10-02)

`Ship_Damage` (`0x088439ac`): on the blow that empties the shield, in mode 8, **when the damage
source is a weapon**, the victim's recorded last attacker gets `+0x8d8` (kills) raised, unless
it is the victim. A wall finishing a craft off credits nobody; a wall scrape earlier does not
erase the attacker. This build used to clear the attacker on any wall contact and credit
only a direct hit, never splash: a third of deaths credited nobody. Now credited on the fatal
weapon blow, splash included (`oag_raceplay::eliminator`).

**A parked-player Eliminator on `16_Track` now finishes at the original's target of 5 on 22 of 24
seeds in six game-minutes** (three fixed sets of eight, the last two never swept on; the finish
test pins 5, 9, 13 and 16, all of which finish), in **about 100 to 345 s, median about 230 s,
against the original's 85 s**. Before 2026-10-02 it was 1 of 8 and about 11 kills in five
minutes. Two changes, both in Eliminator only (Single Race and Time Trial are byte-identical, the
lone-craft gate is unchanged):

1. **A destroyed opponent is put back near its wreck, not at the driver's stale index.**
   `Driver::drive` does not run while a craft is down, so the index stops at the death and the
   wreck coasts on, measured at about 250 units (100 samples). A wreck that crossed the start line
   in that time was respawned *behind* it, which `Standing::update` reads as a backward wrap: one
   lap lost, the gate reset to `NeedsNearHalf`, and so the next forward crossing earned nothing.
   The craft was a lap down for the rest of the race (seed 5, slot 2, from tick 3025: distance
   -258 beside a pack at 5,000), invisible to every other craft's `Field` and outside every pack
   rule. This was most of the "string out": 1 of 8 seeds finished, 5 of 8 after this alone, same
   settings, and 11 of 24 over all three sets. The search is windowed around the stale index
   (`RESPAWN_SEARCH_WINDOW`), so a circuit that passes over itself cannot put a wreck on the
   other level; it gave the same result as a whole-ring search on `16_Track`. Guarded by
   `eliminator_finish_ground_truth::a_respawned_craft_keeps_its_place_in_the_race`.
2. **The leader tether is tighter** (`eliminator_pack_scale`, **chosen, not measured**; no
   confidence score): `PACK_REACH` 0, `PACK_EASE_SPAN` 30, `PACK_MIN_THRUST` 0.1
   (`PACK_LOST` stays 1,500; 300 gives the same 22 of 24). A craft with nobody ahead lifts as its nearest follower falls back. Only ever a
   throttle *reduction*, so the AI keeps the player's physics; **whether catch-up by slowing the
   front is acceptable is the maintainer's call**. Fix alone, 11 of 24; fix plus this tether, 22
   of 24; with the easing off (`PACK_MIN_THRUST` 1.0) 0 of 8. What the leader does under it: below half
   its top speed for 9 to 18 % of ticks and for at most 2.5 to 5.1 s at a stretch over six
   seeds (cornering and crash recovery included; not separated from the tether).
   **Tried and dropped: wider forward-weapon gates** (cone 0.8, curvature 0.01 in place of 0.94
   and 1/400). With the old tether they raised kills but cut finishes (3 of 8 against 5 of 8); with
   the strong tether they added 2 finishes in 24. Not worth a new public `Tuning` field.
   **A held-Turbo chase was not tried**: Turbo (and speed pads) are lawful speed advantages, and
   Turbo is currently fired the moment it is picked up.

**Why the earlier easing looked insensitive to its parameters.** The scale was logged per tick
per slot: at `PACK_REACH` 60 a leader with any follower inside 60 units ran at full throttle,
which a tight pack nearly always has, so the easing acted on 73 of 1,680 samples; and every craft
the respawn fault had put a lap down read thousands of units behind, past `PACK_LOST`, so it was
not in the pack at all. Reach 60 against 0 is 1 against 3 fields in eight finishing; the
knob was sensitive, the sample just did not show it.

**Beam and Quake kills are credited too (2026-10-02, `pulse-elim-credit`).**
`LeachBeam_Drain` and the Quake wave block write the shooter into the victim's `+0x13c` as the
Cannon does (evidence in `eliminator-kill-target.md`), so `Ship_Damage` credits them; this build
had set neither `last_damager` nor `last_weapon_hit` for either, so a kill by beam or wave
credited nobody. Fixed, and the credit now needs the blow that *emptied the shield* (the shield
is read right after the hit), so a wall that finishes a craft after a hit that left it standing
credits nobody, as in the original. Eliminator only; no other mode's state moves. Parked
player, `16_Track`, seeds 1 to 24 (a fresh sweep, not the earlier three sets), six game-minutes:

| | before | after |
| --- | --- | --- |
| finishes | 22 of 24 | **24 of 24** |
| median time to five | about 210 s | **about 118 s** (65 to 325 s) |
| kills against deaths (seed 1) | 9 against 21 | 18 against 16 |

Deaths with no credit are now close to none in the sweep (kills run level with deaths on every
seed). The original's 85 s is still ahead of the median by about a third.

Still open:

- ~~**Time to five is about 1.4 times the original's.**~~ **Now 0.9 times it (2026-10-03)**:
  with opponents firing on the original's own law ([below](#firing-on-the-originals-law-2026-10-03)),
  seeds 1 to 240, **median 75 s**, 17 to 154 s, 13.6 kills a minute. Since 2026-10-04 opponents also fire the Repulser, and the 240-seed figures predate it; re-measured on the four gated seeds only: 13/16/5/9 went from 71.4/105.8/64.3/53.2 s to 15.7/65.6/70.5/73.1 s (seed 13: one Repulser finished five craft already at 8-15 of 95 shield). History: re-measured 2026-10-03 on main at
  `64521c88` (later than the 147 s median of 2026-10-02): parked player, `16_Track`, six
  game-minutes, seeds 1 to 24: 24 of 24 finish, **median 135 s**, 33 to 196 s; seeds 1 to 240:
  240 of 240, **median 117 s**, mean 122 s, 13 to 225 s, the field scoring **8.1 kills a minute**.
  The original: 85 s. With empty-slot opponents steering for weapon pads (below), **median 108 s**,
  mean 108 s, 17 to 232 s, **9.1 kills a minute**.
- **Wall deaths are not a lever (2026-10-03).** Every opponent death in that 24-seed sweep was
  logged with the shield lost to walls and to weapons over the 2 s before it (the wall's share
  read off `wall_shield_charged_of`, the rest of the pool's drop taken as weapon): of **393
  opponent deaths, 392 were finished by a weapon blow and 1 by a wall** (seed 14, tick 1085,
  after a 94-point weapon hit that left 1 point). Per seed, kills credited run level with
  deaths; the only shortfalls are that wall death and a death in a race's last half second
  (13 to 29 ticks before the finish on seeds 3, 13, 14 and 19), whose credit would land after
  the race has ended on the fifth kill. The 2 s before a death average
  37.7 points of weapon loss against 0.08 of wall. The older figure ("11 of 30 deaths with no
  weapon hit", seed 5) predates the beam and quake credit and no longer holds.
- **Held weapons are not a lever either (2026-10-03).** Over seeds 1 to 48 opponents hold a
  weapon for about 45 % of their craft-ticks (wrecked ticks included in the denominator): Missile 19 % of that, Leech Beam 14 %, Shuriken 14 %,
  Cannon 13 %, Plasma 13 %, Rocket 12 %, Mine 7 %, Bomb 5 %, Quake 3 %. Two outlets were built
  and swept over seeds 1 to 240 against the same binary with them off:

  | Outlet | Median | Mean | Kills a minute | Median shift, 95 % bootstrap |
  | --- | --- | --- | --- | --- |
  | none | 117 s | 122 s | 8.11 | - |
  | the original's own rule (below) | 112 s | 119 s | 8.09 | -4.8 s [-15.7, +4.6] |
  | that, plus a held Cannon, Leech Beam, Mine or Bomb absorbed after 10 s (chosen) | 115 s | 122 s | 8.02 | -2.3 s [-14.5, +8.4] |

  The first halved the craft-ticks spent on a Mine or Bomb, the second halved those on a Cannon
  or Leech Beam, and the field's kill rate did not move with either, so neither shipped. **The
  original's rule** is `WeaponAi_DecideFireOrAbsorb` read for mode 8 (see
  [weapon-ai.md](../ghidra/functions/psp-pulse-usa/weapon-ai.md#mode-8-what-the-original-does-with-a-held-weapon-read-2026-10-03)):
  a Mine or Bomb is laid with no target at all, and anything is absorbed (into the one-second
  Shield above) at `0.001` a quarter-second decision, so the original discards next to nothing
  either.
- **A held-Turbo chase cannot happen in this mode.** `WeaponStats_Elimination.xml` gives the
  Turbo zero pad odds in every column (`human`, `front`, `back`, `ai`), and no craft held or ran a
  Turbo in any sweep. Speed pads remain the only lawful speed advantage.
- ~~**What is left is the fire decision.**~~ **Ported 2026-10-03** - see
  [below](#firing-on-the-originals-law-2026-10-03). It was the lever: the kill rate went from 9.1
  to 13.6 a minute.
- ~~**A backward wrap costs a lap in every mode.**~~ **Fixed 2026-10-02.** The original's lap target
  is never lowered by a reverse crossing (`Craft_UpdateLapProgress`, static, confidence 92), so
  `Standing` and `RaceState` keep a `reversed` deficit instead; see `lap-counting.md`. The
  no-lap half of `an_immediate_re_crossing_after_a_backward_wrap_earns_no_lap` stands, its lap
  value changed. No committed golden moved and the 24-seed Eliminator sweep is identical (no
  craft ends a sweep with a deficit). Not observed live with a reversed craft.
- **The player's Eliminator respawn** reads `last_on_track`, latched each tick the craft is near
  the spline, so a player wreck that coasts over the line may lose a lap the same way. Not
  checked, nobody drives in the finish test.

The original's decision is `WeaponAi_DecideFireOrAbsorb` ([weapon-ai.md](../ghidra/functions/psp-pulse-usa/weapon-ai.md));
its fire half is ported (below), its absorb half is not. Guarded by
`crates/game/tests/eliminator_finish_ground_truth.rs`.

### Steering for weapon pads (2026-10-03)

**Ours, chosen, not measured.** In an Eliminator an opponent whose weapon slot is empty steers for
the next weapon pad on its line instead of driving the authored line over it. Nothing read of the
original says its AI does this: the weapon-pad list (`world+0x10c`, count `+0x1d4`) has three
documented readers, `World_CollectNodeLists`, `WeaponPads_TestCraft` and the weapons-off reset
([pads.md](../ghidra/functions/psp-pulse-usa/pads.md)), and none is AI code. That is a reading of
the documented callers, not an exhaustive cross-reference. It obeys the player's physics: the
driver is handed a place in the road, never thrust, speed or shield.

How it works. `Race::pad_for` (`crates/raceplay/src/field/pad_seek.rs`) projects each pad's
centre onto the AI line once at the start, and each tick hands an empty-slot craft the next armed
pad within 150 units, as `oag_ai::Field::pad` (distance ahead, offset across the line). Of a pair
straddling the line it picks the one nearer across to where the craft already is. `Driver::drift`
blends its finished lateral offset toward the pad's offset on a smoothstep, total inside 40 units
(`crates/ai/src/driver/pads.rs`), and drops the pull while it is dodging a laid charge. The
corridor clamp still bounds it. `PadSeeking::for_mode` turns it on for an Eliminator and off for
every other mode, and a driver handed no pad computes the same bits as before, so no golden hash
moved. A held weapon blocks a pickup, so a full-slot craft keeps the line. The player's own slot
is never handed a pad, so an autopilot driving the player does not detour.

**A pad off the lap's road is skipped.** `05_Track`, `14_Track` and `07_Track` author pads on
the branch of a split their lap ring does not drive, and `07_Track` one on a deck stacked over
the line. Projected onto the line, those come out 34 to 153 units across a corridor of 0 to 17,
or up to 64 units off its lateral axis, and steering at one would pin a craft to the corridor's
edge every lap. `line_positions` drops a pad whose centre overhangs the corridor by more than
5 units (about the pad's half-width) or sits more than 8 units off the line's lateral axis.
Checked on all twelve circuits: every pad a lap can reach overhangs by at most 2.0 units
(`16_Track`'s far pad) and sits within 3.3, and every skipped one overhangs by 10.2 or more or
sits 28 or more off the axis.

The pickup gap it closes. With the player parked on `16_Track`, opponents spend 54 % of their
racing ticks with an empty slot and wait a median 9.7 s from emptying to the next pickup. The
circuit's nine pads sit up to 19.8 units off the line (two pairs straddle it).

Sweep: parked player, VENOM, kill target 5, six game-minutes, one binary with the steering
switched per arm (`Race::set_pad_seeking`). Shield lost to walls is the sum over the run of
`wall_shield_charged_of`, per craft-minute of racing. It is not a per-lap figure and not an
end-of-run shield.

| `16_Track`, seeds 1 to 240 | off | empty slot (ships) | every craft |
| --- | --- | --- | --- |
| finishes | 240 | 240 | 240 |
| median time to five | 117 s | **108 s** | 107 s |
| mean | 122 s | **108 s** | 105 s |
| min to max | 13 to 225 s | 17 to 232 s | 17 to 200 s |
| quartiles | 94 / 117 / 149 s | 85 / 108 / 135 s | 88 / 107 / 123 s |
| kills a minute (field) | 8.11 | **9.08** | 9.41 |
| pickups per craft-minute | 3.07 | 3.60 | 3.75 |
| share of racing ticks with an empty slot | 0.54 | 0.45 | 0.44 |
| median wait for a refill | 9.7 s | 7.6 s | 7.2 s |
| wall-contact ticks per craft-minute | 10.24 | 10.70 | 11.83 |
| shield lost to walls per craft-minute | 1.75 | 1.86 | 2.05 |
| paired median shift against off, 95 % bootstrap | - | -6.3 s [-22.1, -2.7] | -12.3 s [-20.5, -5.0] |
| seeds faster than off | - | 141 of 240 | 148 of 240 |

Every craft against empty slot only: -3.4 s [-11.4, +3.9], within noise, and it costs more wall
contact. Empty slot only ships.

Shield, opponents only, seeds 1 to 240: **per-lap**, the shield a craft carries as it crosses
the line, averaged over every crossing, is 56.4 off and 56.3 with the steering; **end-of-run**,
the field's mean shield on the finish tick, is 45.1 off and 45.2 with it. A respawn resets the
shield, so neither is a survivor's figure.

Two other circuits, seeds 1 to 48, off against empty slot:

| | median | mean | kills a minute | wall-contact ticks per craft-minute | shield to walls per craft-minute | paired shift |
| --- | --- | --- | --- | --- | --- | --- |
| `01_Track` | 128 to 112 s | 133 to 115 s | 7.54 to 8.47 | 9.59 to 11.25 | 1.97 to 2.27 | -13.7 s [-37.8, -2.6] |
| `09_Track` | 133 to 116 s | 135 to 124 s | 7.25 to 8.64 | 5.83 to 9.51 | 1.00 to 1.84 | -13.4 s [-43.1, +12.1] |
| `14_Track` | 151 to 133 s | 146 to 134 s | 6.32 to 6.97 | 6.49 to 7.37 | 1.29 to 1.36 | -5.3 s [-42.7, +13.3] |

The two split circuits whose pads the lap mostly does not drive are where it matters most,
because there the field is starved of pickups (a median 26 to 31 s between them, off). An
unfinished run counts as 360 s here:

| seeds 1 to 48 | finishes | median | kills a minute | median refill | wall-contact ticks per craft-minute | shield to walls per craft-minute |
| --- | --- | --- | --- | --- | --- | --- |
| `05_Track` off | 37 | 289 s | 3.44 | 26.0 s | 7.64 | 1.62 |
| `05_Track` with | **46** | **216 s** | 4.28 | 13.9 s | 7.69 | 1.40 |
| `07_Track` off | 24 | over 360 s | 2.83 | 31.3 s | 16.90 | 2.29 |
| `07_Track` with | **48** | **207 s** | 4.60 | 18.6 s | 18.95 | 2.76 |

Per-lap shield at the line and end-of-run shield at the finish tick, off to with: `05_Track`
58.3 to 59.3 and 49.9 to 46.7; `07_Track` 58.6 to 55.1 and 49.3 to 45.6; `14_Track` 57.2 to
53.4 and 46.5 to 46.3.

Tuning tried and not shipped, seeds 1 to 240 on `16_Track`, paired against the shipped 150/40:
a 220-unit pull total inside 100 (+2.9 s [-5.6, +10.5]), and an aim extrapolated through the pad
once it is nearer than the steering lookahead (capped at three times; with 150/40, -0.9 s
[-8.1, +3.8]; with 220/100, -1.6 s [-9.6, +1.5]). All within noise. The far pad of `16_Track`
(19.8 units left, outside the 17.8-unit corridor) is still mostly missed: the craft swings about
10 units toward it and arrives short, because the steering aims a lookahead past the pad.

The gap to the original's 85 s that remained here was the fire decision, ported in the next
section. Guarded by
`crates/game/tests/eliminator_pads_ground_truth.rs` (pickups per empty craft-minute over seeds 1
to 6, one game-minute each: 7.20 with the steering, 5.48 without).

### Firing on the original's law (2026-10-03)

**Read, then ported.** An opponent now decides when to fire its Rocket, Missile, Plasma,
Shuriken, LeachBeam or Quake on `WeaponAi_DecideFireOrAbsorb`'s fire half (`oag_ai::weapon_ai`),
in every mode, on the odds out of the title's `WeaponAIstats.xml`. The read, its confidence and
what was chosen around it are on
[weapon-ai.md](../ghidra/functions/psp-pulse-usa/weapon-ai.md#the-fire-half-at-instruction-level-read-2026-10-03).
In short: a roll every tick at the rate table's entry for how close the nearest craft ahead and
behind are, times the weapon's authored odds, times five in an Eliminator; nothing before 0.8 s
held; and an aimed weapon also needs a craft inside an 8.5-degree cone along the shot's predicted
path, any range out to 20 s of flight. **The difference from the rule it replaces is the range
and the road**: `Driver::wants_to_fire` wanted a noticed craft inside 200 units and straight road
between, so a held weapon waited 10 to 12 s for that to happen. The AI still obeys the player's
physics; only when it presses the button changed. The Cannon (its fire byte reaches nothing in
the original), Mine, Bomb and Turbo keep this project's own rules. A race whose title names no
`WeaponAIstats.xml` keeps the old rule and says so in the loader report; none does now.

**Which titles ship the table (searched 2026-10-03, census redone below).** All five. Pulse and
Pure carry `Data\XML\WeaponAIstats.xml`. HD/Fury's `DATA00.PSARC` carries
`/data/xml/weaponaistats.xml` (1,369 bytes): the thirteen element names with HD's own values, plus
an `AllWeapons` row and an `EliminatorAIStats` row. 2048's `data.psarc` carries `WeaponAIStats.xml`
and `WeaponAIStats2048.xml`, both 1,369 bytes and byte-identical to each other and to HD's
`DATA00` copy (md5 `21e1062b`); `oag_2048::TITLE` names the suffixed one. Omega's `data00.psarc`
carries both spellings too; `oag_omega::TITLE` names the unsuffixed one, which has **no**
Eliminator row (see the next section). So the maintainer's rule (an unmeasured title runs Pulse's
law) needed no restated table: **every title runs `oag_ai::weapon_ai` on its own odds, inherited
from Pulse and unmeasured on HD, Omega and 2048**, with no confidence score for those three. HD's
and Omega's decision code is unread.
`crates/game/tests/fire_law_inherit_ground_truth.rs` pins that a race on each of the three loads
the table and runs `FireLaw::Original`. A title with no table would still have to run Pulse's
odds; that case does not exist, so no cross-title read or restated constant was built for it.

#### HD's `EliminatorAIStats` row (2026-10-03)

**Census** (`scripts/psarc.py list` over every `.psarc` under `data/extracted/ps3`, `ps4` and
`vita`: 7 HD archives, 9 Omega, 9 for 2048's two pressings with patches and DLC, 25 in all;
Omega's per-copy sizes were not listed, `psarc.py` reads a PS4 `data00` as one entry):

| Where | Copy | Bytes | `AllWeapons` | `EliminatorAIStats` |
| --- | --- | --- | --- | --- |
| HD `DATA00.PSARC` | `weaponaistats.xml` | 1,369 | yes | **yes** |
| HD `DATA02.PSARC` (the base game's) | `weaponaistats.xml` | 1,046 | yes | no |
| 2048 EU `PCSF00007` `data.psarc` | `WeaponAIStats.xml`, `WeaponAIStats2048.xml` | 1,369 each | yes | yes, both |
| 2048 US `PCSA00015` `data.psarc` | the same two | 1,369 each | yes | yes, both |
| Omega `data00.psarc` | `weaponaistats.xml` (named by `oag_omega::TITLE`) | n/a | yes | **no** |
| Omega `data00.psarc` | `WeaponAIStats2048.xml` | n/a | yes | yes |

Every other archive (HD `DATA01`, `03` to `06`, Omega `data01` to `04` and the patch's `data05`,
`07`, `08`, `09`, the 2048 patches and DLC) holds none. `HD DATA00` and `DATA02` both being listed
corrects the earlier "DATA00 only"; which one HD's runtime reads is not established
(`Archives::read_name` takes `DATA00`'s). The row is Fury-era: the base game's copy has none.
`oag_tables::weapons::ai` now parses both rows (`WeaponAiStats::all_weapons`,
`WeaponAiStats::eliminator`), refuses a short row by name and ignores attributes it does not
know. `crates/game/tests/eliminator_ai_stats_ground_truth.rs` decodes every copy above and pins
relations rather than literals (ADR-0006): the three copies that carry the row are equal, the
`easy`/`medium`/`hard` flip, use and score scales rise, the absorb scale falls, and Pulse's and
Pure's files carry neither row.

**The row, as authored** (values are the file's; this page quotes them as analysis, not as data
the repository ships): `normalFlip` and `infrontFlip`, then flip, absorb, use and score scales
for each of easy, medium and hard.

**What HD's loader does with it** (`docs/ghidra/functions/ps3-hdfury-eu/weapon-ai-stats.md`,
confidence 85 for the offsets, read off the instructions): the parse stores the fourteen floats
at `+176 .. +228` of a 232-byte object embedded in the RaceManager at `+6240`, in the order
`normalFlip`, `infrontFlip`, flip e/m/h, use e/m/h, absorb e/m/h, score e/m/h. The constructor
defaults the two flips to `1.0` and every scale to `0.0`, so a file without the row (`DATA02`'s,
Omega's unsuffixed one) leaves every scale at zero. `AllWeapons` is the 14th three-float row,
stored after the thirteen weapons. **The consumer of those fields was not found** within the
hour: no direct `lfs` of `RaceManager+6416..6468` exists, so the reads go through a computed
pointer.

**Against Pulse's hard-coded Eliminator terms** (`weapon-ai.md`, "Mode 8"):

| Pulse (mode 8) | HD's row | Verdict | Confidence |
| --- | --- | --- | --- |
| `use *= 5.0`, flat | `*UseScale` 10.5 / 12.5 / 15.5 by tier | A tiered scale in the same slot is plausible; the numbers are not 5, and a different base rate could still make the product agree. Unproven | 25 |
| absorb rate `0.001` | `*AbsorbScale` 3.6 / 2.6 / 0.6, falling with difficulty | Opposite shape: Pulse's is one tiny constant for every skill. Conflicts on its face | 20 |
| `+0x34` forced `0` (a skill index) | none | Not a flip; no counterpart | n/a |
| none | `normalFlip`, `infrontFlip`, `*FlipScale` | New. Plausibly the chance a craft that is ahead, or in front of the player, flips a pickup into a different weapon; **unread, a hypothesis** | below 50 |
| none | `*ScoreScale` | New | below 50 |

No mapping reaches 70, so **nothing is wired**: HD, Omega and 2048 stay on Pulse's hard-coded
mode-8 terms (the maintainer's inherit rule), Pulse and Pure are untouched, and no golden moved.
`WeaponAiStats::eliminator` has no caller.

The file-selection trap, for whoever wires it: Omega's named file has no row while its
`WeaponAIStats2048.xml` has one, so "use the row where the file carries it" would split Omega from
2048 on a spelling. Do not switch Omega's named file without evidence of which copy Omega loads.

Parked player, `16_Track`, Venom, seeds 1 to 240, paired against the old rule on one binary:

| | old rule | original's law | original's law, decided 4 times a second |
| --- | --- | --- | --- |
| finishes | 240 of 240 | 240 of 240 | 240 of 240 |
| time to five, median | 108 s | **75 s** | 93 s |
| time to five, range | 17 to 232 s | 17 to 154 s | 25 to 167 s |
| paired median shift, 95 % bootstrap | - | -29.5 s [-34.9, -22.7] | -15.5 s [-20.1, -8.0] |
| field kills a minute | 9.08 | 13.63 | 10.70 |
| held share (craft-ticks racing with a weapon held) | 0.555 | 0.299 | 0.492 |
| forward weapon held before it goes | 10.4 to 12.0 s | 1.5 to 2.7 s | 7.0 to 9.4 s |
| shield per lap, at the line | 56.3 | 60.1 | 58.1 |
| shield at the finish tick, end of run | 45.2 | 43.7 | 44.8 |

The four-a-second column is a sensitivity check on a prototype of the same law, not shipped: the read says every call (confidence
65, see weapon-ai.md), and the cadence alone moves the median by 18 s. **75 s is now faster than
the original's 85 s.** That figure's provenance is a single number; the result was not tuned
toward it. The held share above divides by racing craft-ticks; the "about 45 %" further up
divided by all craft-ticks, wrecked included, and is the same data.

Single Race (seeds 1 to 48, parked player, six minutes, `16_Track`): the rate table's skill-0
entry is zero, so an opponent with nobody inside 100 ahead never fires there. Forward-weapon
spends per craft-minute moved from 0.097 to 0.122 (Rocket), 0.076 to 0.093 (Missile), 0.032 to
0.040 (LeachBeam) and 0.026 to 0.036 (Plasma), and a forward weapon is held 27 to 57 s rather than
73 to 115 s. Shield per lap at the line 67.4 to 70.4, at the end of the run 49.7 to 52.4.

Guarded by `crates/game/tests/opponent_fire_ground_truth.rs` (the law is the one a Pulse race
runs, and on seeds 1 and 2 a forward weapon is held less than half as long on it as on the old
rule) and the `oag_ai::weapon_ai` unit tests.

### What is deliberately out of scope

- **Per-cell AI-skill resolution.** `AI_ResolveSkillScale`'s own
  `skill`/`skillEasy`/`skillHard` interpolation is not implemented - a
  campaign launch still uses the ordinary `[ai] difficulty` setting rather
  than the cell's own figure. `Cell_EvaluateMedal`'s own gold/silver/bronze
  (direction flipped for Zone and Elimination) and the lap-count/kill-target
  resolution above it are wired, 2026-09-14 - see
  `docs/architecture/persistence.md`'s "where a career system attaches".
- **Presentation.** The same "the sound is built, the HUD hide and the camera
  swing are not" gap [Zone's own ending](#zone-has-no-ending-yet-and-now-we-know-what-it-should-be)
  already carries applies here too.
- **`ER_YOU_ELIM`'s exact wording** ("You have been eliminated!") is not
  drawn anywhere; nothing in this pass settled whether it belongs to the
  craft that reached the target or to the one that did not, and this engine's
  own ending does not need to answer that to work.
- **Opponent aggression in Eliminator is a design axis, not a fidelity
  one** - the owner's own standing rule, *"AI doesn't have to be faithful,
  but challenging"*. Since 2026-10-03 an opponent fires its forward weapons on
  the original's own law (below) rather than on `oag_ai::Driver::wants_to_fire`;
  how hard an opponent hunts another craft beyond that is future tuning, not a
  recovery question.

HUD layout: `Elimination_HUD.xml`, wired the same way every other mode's
layout is (`oag_title::HudLayouts::elimination`) - see [the HUD
page](../ui/hud.md#eliminator).

## Tournament

**Built.** `oag_game::campaign::race_mode_for_cell` maps a campaign cell's
`Tournament` onto [`oag_race::Mode::Tournament`], and a campaign cell in
that mode now launches its first leg, carries points across a leg boundary
and ends on a medal-eligible final standing. The law is recovered at high
confidence in
[`tournament.md`](../ghidra/functions/psp-pulse-usa/tournament.md); this
section is what implements it.

A tournament is a series of single races (up to twelve legs, four authored
on the PSP disc's own campaign cells - `Data\Plugins\grids\grid_01.xml`
onward) scored as one event:

- **A leg races exactly like an ordinary single race.** No separate
  `Tournament_HUD.xml` exists - `docs/formats/race-setup.md`'s own reading
  of `docs/ui/hud.md`'s layout census finds `Arcade_HUD.xml` cited as "the
  single-race and tournament layout" - so [`oag_race::Mode::Tournament`]
  mirrors [`oag_race::Mode::SingleRace`] for every per-leg rule
  (opponents, weapons, pickup absorption, the per-class lap table) and
  draws the same HUD layout. See that variant's own doc comment.
- **A leg earns points by finishing position, off a fixed table - not the
  campaign's own gold/silver/bronze medal points.** 1st through 8th score
  **8, 6, 5, 4, 3, 2, 1, 0**; a craft that did not finish the leg scores
  **0** regardless of where it stopped. No fastest-lap or elimination
  bonus. Confidence 88, a direct table read (`g_tournament_points_by_position`,
  `0x08ab0ba4`) - implemented in [`oag_race::tournament`], with the running
  per-slot totals and the standings rank in the same module.
  **Chosen, not measured:** the original zeroes points for a craft whose
  own race-state byte reads "destroyed"/"retired" (state `7`), a byte this
  engine does not carry; not having finished the leg when it ended is the
  closest honest stand-in, and is what `Progress::record_leg`
  (`crates/raceplay/src/tournament.rs`) actually reads off the board.
- **A leg advances when the player picks `Race Again`'s mid-tournament
  sibling, `ER_NEXT_RACE`**, offered on `EndRace Menu` for every leg but
  the last (`endrace-screens.md`) - `Session::advance_tournament_leg`
  (`crates/game/src/main/session/tournament.rs`) is
  `Tournament_AdvanceLeg`'s effect, reached through this engine's existing
  relaunch machinery rather than a parallel path.
- **The campaign medal a finished tournament earns is a threshold on the
  *final standings rank* by total accumulated points across every leg -
  not the last leg's own finishing position, and not the raw point
  total.** The same `Cell_EvaluateMedal` threshold compare
  ([`race-campaign.md`](../ghidra/functions/psp-pulse-usa/race-campaign.md))
  that turns a `Race` cell's position into gold/silver/bronze runs against
  that rank - `RaceStage::tournament_final_rank` is set only on the last
  leg, once it has finished, and `RaceStage::campaign_medal`'s Tournament
  arm reads it instead of a leg's own placing.
- **A tie in total points is broken by grid-slot order, not a secondary
  criterion** - the standings sort never swaps two crafts with an equal
  total, so whichever started ahead stays ahead.
  [`oag_race::tournament::Standings::ranks`] reproduces this with a stable
  sort, which keeps an exact tie in its original (slot) order the same
  way.
- **A Custom Race `TOURNAMENT` (Racebox's `Tournament C`) exercises the
  identical law**, not an adjacent one - but this engine has no such
  screen yet (a twelve-slot leg picker), so the only reachable launch path
  today is a campaign cell's own `tournament_tracks`. See
  [`Mode::Tournament`][`oag_race::Mode::Tournament`]'s own doc for why the
  variant is deliberately absent from `oag_race::Mode::ALL`, the RACE
  page's own mode list.

**What is deliberately out of scope, chosen rather than read:**

- **Save/resume between legs** (`Tournament_SaveProgress`/
  `Tournament_LoadProgress`, the mechanism behind `MSC_EVENT_TOURN`'s own
  "you can also save your tournament progress between races" and the
  `MSC_MSG_AUTOSAVE3` dialog) is not implemented. `Session::tournament`
  is discarded the moment its `EndRace Menu` is left by any option,
  whether the tournament finished or was abandoned mid-way - there is
  nothing to resume.
- **The authored `EndRace Results` standings table**
  (`EndRaceResults_PopulateTournamentTable`'s own `PRO_POS`/`ER_TEAM`/
  `ER_POINTS` columns, `Line1` = `ER_RACE_STAN`, `BigTopText` =
  `ER_END_TOUR` on the last leg) is not drawn. A leg's `EndRace Results`
  and `EndRace Rewards` reuse the same screens every other campaign race
  already draws - the leg's own placing and, on the last leg, the medal
  earned. The copy from `Race_BuildEndRaceResult`'s own sorted scratch
  rows into that table's fields was never located in the decompile either
  (`tournament.md`'s own "what is not determined"), so there is no traced
  mechanism this could reproduce yet.
- **`DAT_08b30fa0`'s write site, `DAT_08b31158+0xdc`'s exact meaning, the
  leg-name-hash to `PI_Track` resolution and `DAT_08b34320`'s reset
  condition** all stay open - `tournament.md`'s own list, unchanged by this
  pass; none of them was forced by this implementation. `Head2Head`, the
  other item this list used to carry, is now its own section below.

[`oag_race::Mode::Tournament`]: ../../crates/race/src/mode.rs
[`oag_race::tournament`]: ../../crates/race/src/tournament.rs
[`oag_race::tournament::Standings::ranks`]: ../../crates/race/src/tournament.rs

## Head2Head

**Built.** `oag_game::campaign::race_mode_for_cell` maps a campaign cell's
`Head2Head` onto [`oag_race::Mode::Head2Head`], and a campaign cell in that
mode now launches: a two-craft field, weapons locked off, the cell's own
laps, and the campaign's ordinary win-or-nothing medal. The law is recovered
in full in
[`head2head.md`](../ghidra/functions/psp-pulse-usa/head2head.md); this
section is what implements it.

- **The field is the player plus exactly one AI opponent, not seven** -
  measured off `AICount="1"` on all 23 authored cells and the per-mode
  grid-size table's own row for mode `9`, not designed.
  [`Mode::opponent_count`][`oag_race::Mode::opponent_count`] carries this;
  every other opponent-fielding mode still fields seven.
- **Weapons are locked off, not merely defaulted off** - `shield.md`'s own
  `g_weapons_enabled` switch puts mode `9` in both its default-off set and
  its no-override set, and all 23 cells carry `Weapons="off"` with no
  exception. [`oag_race::Mode::weapons_enabled`] answers `false`.
- **Laps follow the same per-class census `Single Race` uses** - every
  authored cell's own `laps` is `4` (Flash, Rapier) or `5` (Phantom); no
  Venom-class `Head2Head` cell exists, so that rung falls back to the same
  table's `3`, on this crate's usual chosen-not-measured footing for an
  unauthored combination.
- **The medal is win or nothing, flat across all 23 cells**: gold target
  `1`, silver and bronze both `0` - the same `evaluate_medal` compare every
  other position-scored mode uses, unmodified.
- **Which team the AI opponent flies is not measured**, the same open
  question `crates/livery/src/lib.rs`'s `teams_for_slots` already carries
  for `Single Race`'s own seven opponents - no cell authors an opponent
  identity (`ship="None"` on all 23, like every other mode), and the code
  path that does carry a real per-entrant team id has no confirmed write
  site. This build answers it the same way: `teams_for_slots`'s own cyclic
  assignment, chosen rather than measured.
- **The sole opponent's grid slot is extrapolated, not independently
  measured for a two-craft field** - composing `grid.md`'s own compaction
  rule ("a short grid packs to the back") with "the local player is forced
  to the back" puts the opponent at slot 7, immediately ahead of the
  player, but no live capture of an actual two-craft grid confirms it.
- **The HUD's own gap readout - `MSC_EVENT_HTH`'s "track the distance
  between you and your opponent" - is drawn**: `crates/hud/src/head_to_head.rs`
  swaps the Position readout for the "1ST"/"2ND" rows, the bar and the
  "+/-NNNm" gap whenever the live mode is `Head2Head`, and a campaign cell was
  walked live on 2026-10-08 (`head2head.md`). Several widget-struct field
  meanings stay at the confidences that page gives.
- **Quitting the race returns to `Cell Selection`**, like every campaign mode
  ([campaign-quit.md](../ghidra/functions/psp-pulse-usa/campaign-quit.md)).
- **Deliberately absent from [`Mode::ALL`]**, like `Tournament` - reachable
  only through a campaign cell. Unlike `Tournament` this is not a leg-list
  problem; a field of one opponent has no row on the RACE page's own mode
  list to ask for it from.

[`oag_race::Mode::Head2Head`]: ../../crates/race/src/mode.rs
[`oag_race::Mode::opponent_count`]: ../../crates/race/src/mode.rs
[`oag_race::Mode::weapons_enabled`]: ../../crates/race/src/mode.rs
[`Mode::ALL`]: ../../crates/race/src/mode.rs

## What happens when a race ends

**The flag is the player's own last crossing.** `RaceState::finished` is set by
the lap counter when the player wraps past `laps_target`, which is the same rule
that counts every other lap - so a time trial and a single race both end after
their speed class's own count (3/4/4/5, see [time trial](#time-trial) and
[single race](#single-race)), and a speed lap and a Zone run have no target and
never end this way. An
opponent crossing ahead takes a `Standing::finish_tick` and stops being ordered
by distance; it does not end the event.

Three things happen on that tick, and they are in three layers on purpose:

1. **`oag-race` decides it.** Nothing was changed here - the condition and the
   ordering (`places`: finishers by when they finished, then racers by distance,
   then by slot) both predate this.
2. **`Race::capture_results` takes a snapshot**, once, at the end of that tick.
   The field is still moving when the player crosses, so a table recomputed a
   second later would show a different result; the board is fixed at the flag
   and never revised. `Race::tick` itself is unchanged and goes on stepping if
   something keeps calling it, which is what keeps the trace harness and the
   headless capture honest.
3. **The composition root stops stepping it - unless the player crossed the line.**
   `Session::frame` stops calling `tick` once `Race::finished` answers yes, the HUD
   is replaced by the results table, and X or escape hands the window back to the
   menus. That is the same place the camera cycle is handled, and for the same
   reason: a race that stopped itself inside `tick` would change what every
   fixed-tick-count caller produces. **Since 2026-10-01 a finish by the line keeps
   being stepped** (`Race::runs_on_after_the_line`): the original keeps the race
   running behind the panels with the player's craft flown by the AI, see
   [after-the-finish.md](after-the-finish.md). A wreck, an Eliminator target and a
   Zone run still stand still, and a Single Race wreck runs on too (2026-10-02, the maintainer's "hold, then results").

**The table is ours and is labelled as one.** The original ends a race in a
sequence of screens whose names are recovered - `"Race End Photo"`,
`"Race End Save"`, `"Race End Records"`, `"Race End Proceed"`,
`"Race End Alone"`, and `"EndRace_Results"` from `Zone_UpdateResults`. **`Race End Photo`
has been read and is reproduced for a finish by the line and a Single Race wreck** (2026-10-02,
[after-the-finish.md](after-the-finish.md)); the others have not, so none of them is. What is drawn instead is a plain list of positions in the built-in
grid, the same kind of stand-in [our own menus](../ui/menus-original.md) are.
See `crates/game/src/scoreboard.rs`, which carries the argument and what would
retire it. A row names its grid slot rather than a team, because the team a slot
flies is an id held by the renderer's liveries and is not reachable from the race.

**Reaching the end without driving:** `--autopilot` flies the player's craft with
an opponent's driver. A verification aid, not a mode - the flag falls on the
player's crossing, so nothing that holds the throttle in a straight line ever
gets there, and without it no test could assert on a finished race and no
screenshot could show one:

```sh
cargo run -p oag-game -- --race --mode single_race --autopilot \
    --ticks 9000 --screenshot /tmp/scoreboard.png <image>
```

The capture stops on the tick the race ends and says so, so a `--ticks` past the
flag lands on the board rather than overshooting it. Measured on
`pulse-psp-usa.chd`, default circuit, Venom, elite AI: the race ends around tick
7,500, a little over two minutes.

## The countdown is measured

`ReadyText`, `GoText`, `CountdownTime` and the `<Mode3D>` models all parse; what
was missing was runtime evidence of what they are actually timed against. Three
live PPSSPP captures settle it - `pulse-psp-usa.chd`, Time Trial, Talon's
Junction White, `cross` held continuously from before the track description
screen through the whole countdown and into the launch, two runs entered via
`psp-drive.py restart` and one via a genuine front-end menu walk
(`psp-drive.py menu`). All three agree to the tick:

- The front end reports `InGameTrackDescriptionScreen` for ticks 0-60, then
  `InGame` from tick 61 - a real, pollable state name, not the "nothing to
  poll" this page and `ppsspp-debugger.md` used to say. **Corrected
  2026-10-01: those 60 ticks are the lock on the pre-race flyby**, not load time
  and not a dialog: the screen is the circuit's camera animation
  ([race-intro.md](race-intro.md)), nothing can end it before its 60-tick
  counter reaches zero, and thrust held through it (Cross, button 5) ends it
  the tick the counter does - which is why it dismissed at the same tick on
  every run. Left alone, it plays to its `AnimEnd` (25 s on `16_Track`) and
  the 272 ticks below follow.
- From tick 61, the craft sits stationary - `throttleState` (`craft+0x2b8`)
  reads a flat `0.0` regardless of the input held, and position holds within
  jitter (< 0.1 units total drift, `grounded` staying `1.0`) rather than
  creeping under any partial thrust.
- At tick 333, `throttleState` steps straight from `0.0` to the full held
  value (`100.0`) - no ramp, no separate launch state.
- So the countdown itself, dialog-dismiss to green, is **272 ticks**, dt-summed
  to **4.5377 s** - identical (to five figures) on all three runs.

**Holding thrust through the countdown is not a false start.** This page and
four places in `scripts/` used to say Pulse stalls the engine for it; that was
unmeasured belief, not a finding, and it does not hold up: `craft+0x290` (the
collision stun timer) and `craft+0x2e0` (`Ship_UpdateEngine`'s other early-return
gate, see
[engine.md](../ghidra/functions/psp-pulse-usa/engine.md#the-engine-has-an-early-return-that-produces-no-thrust-at-all))
both stay flat at `0.0` across the entire 1200-tick capture, on every run. That
also **refutes** `engine.md`'s own standing hypothesis that `craft+0x2e0` gates a
race start - it does not arm during one. Whatever pins `throttleState` to zero
for those 272 ticks is a separate write this capture does not locate; the
early-return path documented in `engine.md` is confirmed *not* to be it.

**What this does not cover, yet:** Single Race / a full grid, Zone mode's own
timing (partially mapped statically - see
[zone-mode.md](../ghidra/functions/psp-pulse-usa/zone-mode.md)), Pure, HD/Fury
and 2048. Treat the 272-tick figure as Pulse Time Trial's own number until one of
those is checked against it, not as a cross-title constant.

`crates/race` implements the shape this measured - an input gate rather than a
physics hold or a stall state - at a fixed tick count; see `crates/race/src/`
for the current implementation and whether it has grown per-mode or per-title
values since this was written.

## The race clock starts at the release

**Measured 2026-09-29, Pulse (PSP), Time Trial, Talon's Junction - four
captures, the same ticks each time.** The maintainer's report from play was that
the timer ran during the countdown and that "GO" showed well before the craft
answered. Both are real, and this is what the original does instead. Method: a
breakpoint in `Ship_UpdateCraft` counts ticks, and each tick reads `throttleState`
(`craft+0x2b8`), the racer's lap clock (`racer+0x920`), the race manager's race
time (`manager+0x2b8`) and the front-end state name; the frames come from
one screenshot per tick with the CPU held at the breakpoint (the presented frame
is within a tick or two of the row, per
[`ppsspp-debugger.md`](../reverse-engineering/ppsspp-debugger.md#screenshots-at-the-breakpoint-and-the-clipboards-one-shot-lag)).
Counting from the first `InGame` tick after the track description screen, so the
thrust release is tick 272:

| what | tick | evidence |
| --- | --- | --- |
| `throttleState` steps `0.0` -> `100.0` | **272** | the earlier three captures, and these four |
| lap clock `racer+0x920` and race time `manager+0x2b8` leave `0.0` | **272**, first value one `dt` (0.0166 s) | flat `0.000000` for ticks 0-271 on all four runs |
| HUD `CurrentTime` | reads `0.00.0` for the whole countdown, `0.00.1` by tick ~284 | screenshots, ticks 68-288 |
| gantry `3` first reads white | 132 | 1-tick screenshot series, white-pixel count in the digit's third of the board |
| gantry `2` / `1` first read white | 178 / 222 | same - 46 and 44 ticks apart, the asset's own 45 |
| the board turns green and shows `GO` | **273** | the same series: a step from 611 to 38,234 green pixels between two consecutive ticks |
| Time Trial's top-right slot (a record to beat, `1.57.0`) starts counting down | 274-275 | screenshots |

So **the original's clocks are zero through the whole countdown and start on the
release tick**, and `GO` is drawn on that same tick, **not** the second before it
that this page's gantry section had assumed. Confidence 90 for the clock start
(read from memory on four runs, no inference), 85 for `GO` on the release (a
screenshot series, one tick of presentation lag in it).

`craft+0x920` is also what a Time Trial's `TotalTime` reads (plus the completed
laps' splits, `race-progress.md`), so **every time the player is shown or scored
excludes the countdown**. Zone is the one mode read only statically: its state
machine leaves the countdown into a state that resets the dwell timer, the dirty
flag and the score (`zone-mode.md`, state 1), so nothing accrues on the grid -
confidence 65, not measured live.

What `oag_race` does with it:

- [`oag_race::race_clock_ticks`](../../crates/race/src/state.rs) is
  `tick - COUNTDOWN_TICKS`, saturating. The HUD's `TotalTime`, the campaign medal
  pace, the results board's times, a finish time handed to the records and a
  campaign medal all read it; the raw `World::tick` is still what the thrust gate
  and the animations ride.
- `RaceState::lap_ticks` floors the lap's start at the release. The stored
  `lap_start_tick` is untouched, so no state hash moves for it.
- Zone's score and dwell timer are held through the countdown.
- **Lap 1's clock still restarts at the first line crossing**, the documented
  divergence in [lap counting](lap-counting.md), because our spawn is further
  behind the line than the original's. `CurrentTime` therefore runs from the
  release to the first crossing, resets there, and runs on - the reset is that
  divergence, not the countdown.

**The start gantry rides its own clock, 92 ticks in.** The board's timeline is not
zero at the race start: the authored `u` step at frame 181 that hands the board
to `GO` lands on the green step above, tick 273, which puts frame 0 at **tick 92**
(`272 - 180`); the softer digit windows bracket that at 85-96. The countdown's
`GO` is therefore the timeline's own handover frame landing on the release, and
the "about a second of `GO` before the craft can move" the gantry docs recorded
was our clock starting at tick 0, not the original's behaviour.
`crates/raceplay/src/gantry.rs::CLOCK_START_TICK`, **Pulse only**: every other
title runs Pulse's rule on its own `GO` edge (HD: frame 203, tick 70; inherited
from Pulse, unmeasured - `docs/rendering/start-gantry.md`). The
cockpit-view overlay
(`Cockpit_321GO.vex`, only drawn where no gantry is) was **not** measured in the
original and still runs off `world.tick / 60`.

**The countdown voice is two cues and lands one tick before the release.**
Measured 2026-09-29 by breaking on `Scream_StartSound` and logging the cycle
counter (Time Trial twice, a single race, Eliminator): the original plays
`ready` at tick 90 and `go` at tick 270 on this table's axis - **180.0 ticks
apart, and `go` between the `Ship_UpdateCraft` entries numbered 270 and 271, two
entries before `throttleState` first reads 100**. The craft update in between is
the first one the countdown state no longer gates, so `go` is the state change
itself: `RaceMode_UpdateCountdown` plays it in the call that runs
`Race_StartRacing`. Nothing else is started between the two - the gantry's
`3`, `2`, `1` (132, 178, 222) have no cues of their own; what `ready`'s words say
is not identified. Confidence 95 for the ticks and the absence of other
cues (four captures, agreeing to 0.05 tick in the three anchored ones), 80 for the
trigger; **Zone was not captured** and is 75, from sharing the same two
functions and the constant that sets the 180 (`RaceManager_Construct`, all modes). `ready` sits one to two ticks before the gantry timeline's derived
start (92), most likely the same event through a screenshot's lag. The bank is the
mode's speech bank; see
[countdown-voice.md](../ghidra/functions/psp-pulse-usa/countdown-voice.md).
`oag_raceplay::countdown` raises them at `World::tick` 91 and 271
(`COUNTDOWN_TICKS - 1`, the last gated tick).

**Launch speed is not the problem.** Ours reads 28 km/h thirteen ticks after the
release (tick 285), the original 26 km/h at tick 284 - so the "delay after GO" was
the board, not the physics.

**Saved records from before this change include the countdown.** A personal-best
total time stored by an older build carries the extra ~4.5 s, so the first run
after the change beats an old record by about that much. No migration is written.

**Still open:** steering, braking and the airbrakes through the countdown
(unmeasured, left live in ours); any other mode or title's countdown length;
whether the `go` voice line fires at the release like the board (not measured -
it needs audio; ours has no countdown voice yet, and its trigger belongs on
[`oag_race::COUNTDOWN_TICKS`](../../crates/race/src/state.rs) when it is wired).

## See also

- [lap counting](lap-counting.md)
- [zone-mode.md](../ghidra/functions/psp-pulse-usa/zone-mode.md) - the Ghidra evidence
- [the HUD](../ui/hud.md) - which widgets have a source
- `crates/race/`
