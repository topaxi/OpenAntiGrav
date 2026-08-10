# Race modes

Three single-ship modes are implemented: **time trial**, **speed lap** and
**Zone**. None of them needs opponents, weapons or a grid, which is why they came
first - they are the part of the race layer that can be finished rather than
stubbed.

The rules live in `crates/race`; how a lap is decided at all is
[lap counting](lap-counting.md), and it is a convention rather than a recovery.

Selected on the RACE menu page, which opens on the time trial, or with
`--mode time_trial|speed_lap|zone` on the command line. `--race` skips the menus,
so the flag is the only way in on that path.

## What each mode is

| | Time trial | Speed lap | Zone |
| --- | --- | --- | --- |
| Laps | 3 | unlimited | unlimited |
| Throttle | the player's | the player's | the mode's |
| Ends by itself | after lap 3 | never | **not implemented** - see below |
| HUD layout | `TimeTrial_HUD.xml` | `TimeTrial_HUD.xml` | `Zone_HUD.xml` |

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

### The ship model is fixed, the livery is not

`Ship_LoadModel` loads `<Team>\Zone.vex` instead of `<Team>\Ship.vex` under the
same selector, and every team's `Zone.vex` decodes to the same hull - see
[zone-mode.md](../ghidra/functions/psp-pulse-usa/zone-mode.md#the-ship-model-is-not-the-players-own-hull).
Confidence 84. `oag_game::race::ship_entry_name` picks the model this way, and
the menu greys the TEAM row while MODE is Zone so a player is not offered a
choice that no longer changes the shape drawn - only the colour it is drawn in.

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

Both also grant *"a free turbo pickup once per lap"*, which is **not**
implemented: `Engine::turbo` is read into `Handling` and never applied.

**Also out of scope:** the `Zone_Bar_*` widgets and the `IG_HUD_PERF_ZONE` /
`IG_HUD_NEW_ZONE_RECORD` banners. Their writers were not found, and the
zone-specific graphics were scoped out of this work.

## What no mode has yet

A countdown. `ReadyText`, `GoText`, `CountdownTime` and the `<Mode3D>` models all
parse and nothing drives them, and the original's false-start stall - which
silently kills the engine if you hold thrust before the lights - is unimplemented
too. A race begins the moment it loads.

## See also

- [lap counting](lap-counting.md)
- [zone-mode.md](../ghidra/functions/psp-pulse-usa/zone-mode.md) - the Ghidra evidence
- [the HUD](../ui/hud.md) - which widgets have a source
- `crates/race/`
