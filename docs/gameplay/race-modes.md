# Race modes

Four modes are implemented. Three are single-ship - **time trial**, **speed lap**
and **Zone** - and came first because none of them needs opponents, weapons or a
grid: they are the part of the race layer that can be finished rather than
stubbed. The fourth, **single race**, is the first with weapons, and it exists
because pickups have nowhere else to happen; see below.

**Neither is invented - both are measured on the running original and now
enforced in code, not just true by construction.** `Mode::has_opponents` and
`Mode::weapons_enabled` return `false` for the three single-ship modes: the Custom Race screen
greys `AI DIFFICULTY` to `N/A` and `WEAPONS` to `Off` for time trial, speed lap
and Zone alike, screenshotted for all seven of the original's race types on
2026-08-10 - see
[shield.md](../ghidra/functions/psp-pulse-usa/shield.md#g_weapons_enableds-per-mode-default-spelled-out-and-checked-against-every-race-type).
Two consequences in `crates/game/src/race.rs`:

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
  circuits load through `oag_render::mesh::rcs::build_scene`, which used to
  draw weapon-pad geometry as part of the track's own world-space chunk pass
  rather than through a separate `Weapon Pad` node the way the PSP-shaped path
  does, so `oag_game::race::load` set `weapon_pad_model` to `None` on that
  path and the mode gate had nothing to act on - HD weapon pads drew in every
  mode. `build_scene` now splits `Weapon Pad` chunks into their own model
  (`crates/render/src/mesh/rcs/pads.rs`), and `geometry::track_model`
  (`crates/game/src/race/load/geometry.rs`) hands that model to the same
  `weapon_pad_model` slot the PSP-shaped path already fills, so the existing
  gate covers HD for free. Pinned on the real disc by
  `crates/render/tests/hd_weapon_pad_split_ground_truth.rs`. `Speedup Pad`
  chunks are untouched - nothing gates them by mode, on this disc or the
  original's.

The rules live in `crates/race`; how a lap is decided at all is
[lap counting](lap-counting.md), and it is a convention rather than a recovery.

Selected on the RACE menu page, which opens on the time trial, or with
`--mode time_trial|speed_lap|zone|single_race` on the command line. `--race` skips the menus,
so the flag is the only way in on that path.

## What each mode is

| | Time trial | Speed lap | Zone | Single race |
| --- | --- | --- | --- | --- |
| Laps | 3 | unlimited | unlimited | 3, **ours** |
| Throttle | the player's | the player's | the mode's | the player's |
| Ends by itself | after lap 3 | never | **not implemented** - see below | after lap 3 |
| Weapons | off | off | off | **on** |
| Free turbo a lap | **yes** | **yes** | no | no |
| HUD layout | `TimeTrial_HUD.xml` | `TimeTrial_HUD.xml` | `Zone_HUD.xml` | `Arcade_HUD.xml` |
| Environment | the circuit's | the circuit's | **its own**, see below | the circuit's |

Time trial and speed lap sharing a layout is the disc's arrangement, not a
shortcut: there is no `SpeedLap_HUD.xml`, which is why
[the HUD page](../ui/hud.md) counts five layouts for six modes.

## Time trial

**Three laps.** Observed rather than invented: the original's own counter reads
`Lap 1 of 3`, and a run that drives past the third lap starts a fresh attempt with
the best time cleared. See
[the PPSSPP debugger notes](../reverse-engineering/ppsspp-debugger.md).

The count is *configuration* in the original rather than a constant - the
race-setup format string carries `laps="%d"` - so three is what a time trial was
seen configured with, not a limit of the format. `Mode::TIME_TRIAL_LAPS`.

Weapons and AI fall out of the mode rather than being set: selecting TIME TRIAL on
the original's Custom Race screen flips WEAPONS to OFF and AI DIFFICULTY to N/A on
its own. Neither exists here yet, so nothing had to be turned off.

**Not implemented:** the fresh-attempt-behind-you behaviour. The run stops at the
end of lap 3.

## Speed lap

Unlimited laps, chasing one fast lap; it never ends on its own and escape leaves.
Everything about it is the time trial without the lap target, which is how it is
implemented - `Mode::laps_target` returns `None`.

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
`oag_game::race::zone_grade` the application. **What selects a stage during a
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
Confidence 84. `oag_game::race::ship_entry_name` picks the model this way, and
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

### Every title ships one Zone handling block, and this engine does not read it

**The shipped answer to "what handling does a Zone craft have" is one block
shared by every team**, not the player's own. All three titles carry a Zone-mode
craft directory - `Data\Ships\Zone_01` on both PSP titles, `/data/ships/zone` on
HD - whose `handlingstats.xml` opens `<Stats team="ZoneMode">` and authors **no
`<Class>` block at all**, where a team file authors four or five.

That fits the mode: a `<Class>` carries engine, brakes and turning, and Zone
replaces the engine with the auto-speed law, disables the brakes and flies a
four-corner hover variant - all three selected by the Zone expression rather
than read from a team.

**It is unread here**, so a Zone race flies the player's own team's numbers and
`race::load` says so on every Zone load. Closing it is a physics question -
which blocks the mode is actually meant to supply, and where turning comes from
when no class authors one - rather than a naming one, and no title's Zone
handling path has been read in any executable. Watch the near-miss on Pulse:
`Data\Ships\Zone` is `<Stats team="Zone">` **with** classes and is the
unlockable Zone *livery*, a raceable team; `Zone_01` is the mode.

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

**What is not wired: the menu still offers all 24 in Zone.** The track list is
supplied once when the menus open and is not re-supplied when MODE changes, so
picking a non-Zone circuit and then Zone gives a load error naming
`availableInZone` rather than a row that was never offered. Deliberate: making
the list mode-reactive is front-end work, and the error is honest in the
meantime. `catalogue::tracks_of_kind` and `Track::available_in_zone` are the two
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
checking because `Course::START_LINE_OFFSET` is fitted on `16_Track`'s *race*
file. Across the sixteen: fourteen have identical control-point counts and one
environment does not - `10_Track` at 844 race / 848 zone, and its reversed twin
`26_Track` at 847 / 852. So the fit carries on fourteen and is inherited rather
than re-measured on the other two. Nothing about Zone lap timing is verified
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
  same bit that gates the brakes - so groundedness stands in for it. Bit 1 is not
  decoded and is treated as always clear.
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

Zone is the only one of the three modes that can reach it, because it is the only
one that races with damage on - see below.

**What is deliberately absent is the presentation.** The original plays
`_BLOWUP`, hides the HUD and swings the camera into its mode 5 on the way
through state 4; this engine has none of those, so a wrecked run simply stops.
That is a rendering and UI gap rather than a rules one, and `race.rs` says so at
the point where it would go.

`MSC_EVENT_ZONE` also ends *"Clear the target number of zones to win the event"*,
so a Zone event has a target zone count. That is progression data - M7 - and is
not modelled.

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
own edge at the start of the race (fixed 2026-08-19; the field started every
race one turbo short until then). The manual string is
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
- **No `WEAPONS` row.** The original calls weapons "optional" here and this
  build takes the default, so a single race always has them. A missing setting,
  not a different rule.
- **The lap count is ours.** Three, carried over from the time trial. The
  race-setup format carries `laps="%d"`, so the original configures it per event
  and no single race has been watched long enough to read what a Custom Race is
  set to. `Mode::SINGLE_RACE_LAPS`, flagged as a guess where it is defined.

## What happens when a race ends

**The flag is the player's own last crossing.** `RaceState::finished` is set by
the lap counter when the player wraps past `laps_target`, which is the same rule
that counts every other lap - so a time trial and a single race both end after
lap 3, and a speed lap and a Zone run have no target and never end this way. An
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
3. **The composition root stops stepping it.** `Session::frame` stops calling
   `tick` once `Race::finished` answers yes, the HUD is replaced by the results
   table, and X or escape hands the window back to the menus. That is the same
   place the camera cycle is handled, and for the same reason: a race that
   stopped itself inside `tick` would change what every fixed-tick-count caller
   produces.

**The table is ours and is labelled as one.** The original ends a race in a
sequence of screens whose names are recovered - `"Race End Photo"`,
`"Race End Save"`, `"Race End Records"`, `"Race End Proceed"`,
`"Race End Alone"`, and `"EndRace_Results"` from `Zone_UpdateResults` - and
**none of them has had its screen, its layout or its transitions read**, so none
is reproduced. What is drawn instead is a plain list of positions in the built-in
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
  poll" this page and `ppsspp-debugger.md` used to say. It reads as load time
  plus the first frame the dialog's dismiss press is seen, not an authored
  minimum-show timer - thrust was already held before the dialog appeared on
  every run and it still dismissed at the same tick each time.
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

## See also

- [lap counting](lap-counting.md)
- [zone-mode.md](../ghidra/functions/psp-pulse-usa/zone-mode.md) - the Ghidra evidence
- [the HUD](../ui/hud.md) - which widgets have a source
- `crates/race/`
