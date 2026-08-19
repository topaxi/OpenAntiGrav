# Opponent AI

What drives the seven other craft. **A basic driver does, as of 2026-08-11**:
[`oag-ai`](../../crates/ai/src/lib.rs) follows the authored racing line and takes
its speed from the corner ahead, and `Mode::SingleRace` fields a grid because the
mode says so rather than because a test forced one. This page is the recovery
that made that possible, the design it is the first half of, and what is still
unbuilt.

The short version: **the original's opponents do not race.** They follow an
authored line with a damped controller, and their *speed* comes from a schedule
keyed on the player's race position and the gap to the player. Steering is a
craft; throttle is a puppet string. This project keeps the first and replaces
the second - see [what we build instead](#what-we-build-instead).

Per [ADR-0006](../architecture/adr/0006-no-copyrighted-content.md) this page
records **element and attribute names, structure, and which quantity each field
governs**. No tuning value is reproduced; read them off your own disc with the
commands in [evidence](#evidence). Where the *shape* of a table matters to an
argument below it is described in words - "the spread is a few points", "several
times larger" - because the finding is the coupling, not the number.

## Read this first: what is recovered and what is ours

| Piece | Status | Confidence |
| --- | --- | --- |
| The parser chain and the per-class struct layout | **recovered** - see [ai-stats.md](../ghidra/functions/psp-pulse-usa/ai-stats.md) | 88 |
| The steering controller's schema and its per-class split | **recovered**, from shipped data | 88 |
| The racing line and AI corridor the controller follows | **recovered** - see [track.md](../formats/track.md) | 92 |
| That AI speed is scheduled rather than driven | **recovered**, from shipped data | 85 |
| Pulse's four speed blocks, and which two read the player | **recovered**, corroborated PSP and PS2 | 90 |
| Pure's larger, earlier model - zones, fatigue, lead/tail | **recovered**, from shipped data | 85 |
| That `BaseStartThrust` is authored and never read | **recovered** | 90 |
| What the numbers *mean* - units, and what `AIThrust` multiplies | **unknown** - the consumer is not identified | - |
| The sign convention on Pure's `Position` attribute | hypothesis | 55 |
| Which of Pure's two per-class blocks the engine reads | **unknown** | - |
| Weapon selection and firing | **recovered** (2026-08-17), and unported. `WeaponAi_Update` (`0x08851550`) reconsiders four times a second and `WeaponAi_DecideFireOrAbsorb` (`0x088518b4`) rolls the authored odds against two five-entry difficulty tables. See [weapon-ai.md](../ghidra/functions/psp-pulse-usa/weapon-ai.md) | 85 |
| **Everything under [what we build instead](#what-we-build-instead)** | **ours** | - |

Nothing here has been run under a debugger. Every score above rests on
decompilation, shipped data agreement and string evidence, which the
[rubric](../reverse-engineering/confidence-rubric.md) caps below 95 by design.
The nine parser functions **are** named in Ghidra, at 88, with their evidence on
[ai-stats.md](../ghidra/functions/psp-pulse-usa/ai-stats.md) and rows in
`names.tsv`. Nothing on the *consumer* side is named, because nothing on the
consumer side has been found.

## Evidence

Two XML resources, both named as literals in the PSP executable's string table
and both readable straight off the disc:

```sh
just wad cat --expand data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad 'Data\XML\AIControlStats.xml'
just wad cat --expand data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad 'Data\XML\AIRaceStats_phantom.xml'
just wad cat --expand data/images/pure-psp-usa.chd:PSP_GAME/USRDIR/Data.wad  'Data\XML\AIRaceStats.xml'
```

Pulse splits race balancing into four entries, one per speed class
(`AIRaceStats_venom|flash|rapier|phantom.xml`); Pure ships a single
`AIRaceStats.xml` holding all five of its classes. Both ship one
`AIControlStats.xml`, keyed by class inside. The executable also carries every
tag name below as a separate string, which is what says these files are read
rather than left over.

**The PS2 build is a corroborating second binary.**
`Data\XML\AIRaceStats_phantom.xml` in `54748/WADS2.WAD` on `pulse-ps2-eu.chd`
carries the same schema *and the same values* as the PSP disc, field for field.
That is what lifts the Pulse rows above to 90.

## Steering: a lookahead controller with a cross-track term

`AIControlStats.xml` gives one `<Controller>` per speed class:

| Attribute | What it governs |
| --- | --- |
| `LookAheadSecs` | how far down the line the aim point sits, in seconds of travel |
| `SteerMul` | proportional gain on the heading error to that point |
| `SteerDamp` | damping on the same |
| `xtrackMul` | proportional gain on cross-track error - distance from the line |
| `xtrackDamp` | damping on the cross-track term |

Two error terms, each with a gain and a damping constant, summed into a steering
command: where the craft is *pointing* versus a lookahead point, and where the
craft *is* versus the line. Across the four Pulse classes the cross-track gain
**falls** and its damping **rises** as the class gets faster, which is what you
tune when the faster classes are oscillating about the line.

**This is the same controller shape as the pure-pursuit steerer already in the
tree** - `oag_trace::replay::drive_with` and `oag-trace plan`, see
[autopilot-planning](../tools/autopilot-planning.md). The lateral half of the AI
is therefore close to built; the longitudinal half is the gap.

**The line it follows is authored track data, not something to compute.** Every
`SplinePt` in the `WO Track` node carries `racing_line`, `ai_bound_left` and
`ai_bound_right` - a line plus a corridor around it, decoded and validated
across 34,261 control points on all 40 track files. See
[track.md](../formats/track.md#control-points). The resource is published at
runtime under the name `"AI track data"` - a name that occurs 21 times in the
executable's string table, once per allocating site, so it identifies the
subsystem rather than a single function.

Pure ships a **second** controller per class, `<ControllerPO>`, with an
attribute `Controller` does not have (`xtrackMax`, a clamp on the cross-track
term) and markedly higher gains. Its consumer is unidentified. Working
hypothesis, confidence **40** and deliberately not acted on: the Autopilot
pickup, which drives the *player's* craft and would want a different, more
assertive tune. `PRO_STATS_AUTOPILOT`, `Autopilot` and `autopilot input` all
appear in both string tables. Pulse's file has no `ControllerPO`.

## Speed, in Pulse: four blocks, two of them player-coupled

`AIRaceStats_<class>.xml` holds one `<RaceBalancing>`, with four blocks under
it. Two read the player's state and two do not - and **the split does not follow
the element names**, which is the single most important thing on this page.

| Block | Input | Player-coupled |
| --- | --- | --- |
| `PosBalancing` | the player's current race position | **yes** |
| `RubberBanding` | the gap to the player | **yes** |
| `StartStats` | the craft's own grid slot | no |
| `SkillScale` | the chosen difficulty | no |

### `PosBalancing`

Eight rows, `PlayerInPos1` through `PlayerInPos8`, each carrying `AIThrust` and
`SpreadDist`. The AI's baseline thrust and the field's spread are indexed by
where *the player* currently is.

The gradient is small in Pulse - a few points of thrust between the player
leading and the player last, in every class - and `SpreadDist` peaks in the
middle of the range, so the field is held tightest when the player is at either
end of it. Small or not, **this is difficulty adjustment keyed on the player,
and it lives outside the element called `RubberBanding`.** A configuration
switch that gates only the element with that name would leave this running.

### `RubberBanding`

Two rows, `WhenLeading` and `WhenBehind`, three attributes each:

| Attribute | What it governs |
| --- | --- |
| `SpeedMul` | how much speed adjustment per unit of gap |
| `MinSpeedChangeDist` | the dead band - no adjustment inside this gap |
| `MaxSpeedChange` | the clamp on the total adjustment |

The obvious reading: past a gap of `MinSpeedChangeDist`, adjust speed by
`SpeedMul` times the gap, clamped to `MaxSpeedChange`. The two rows are **not
symmetric**: the shipped `WhenBehind` multiplier is several times the
`WhenLeading` one, in all four classes and on both discs. The catch-up is the
point; the hold-back is a courtesy. The dead band and the clamp are the same for
both directions, and the clamp is larger in the two faster classes.

### `StartStats` and `SkillScale`

`GridPlace1` through `GridPlace8` carry `StartBoost` and `BaseStartThrust`, so
the launch is staggered by grid position rather than by anything the player
does. Both attributes fall monotonically toward the back of the grid, and the
two faster classes launch the whole field softer than the two slower ones.

**`BaseStartThrust` is never read.** `AiStats_ParseStartStats` compares each
attribute against the literal `BaseThrust`, exactly, and every shipped file on
both discs authors `BaseStartThrust` - so the field it would fill stays at its
default and only `StartBoost` varies by slot. Confidence 85, and the argument is
that the parser's names and the shipped names agree character for character on
all fourteen other attributes. See
[ai-stats.md](../ghidra/functions/psp-pulse-usa/ai-stats.md#basestartthrust-is-authored-and-never-read).

`SkillScale` holds the three difficulty settings the front end exposes - the
`AI n` row documented in [race-modes](race-modes.md), stored as `SkillLevel` and
`DifficultyRC`. Each `SkillScalePoint1..3` carries a `ThrustOffset`, a
`ThrustMultiplier` and a `SpreadMultiplier`. **Only the offset varies**: both
multipliers are shipped at unity for every class on both discs, which makes them
tunables that were exposed and never used. The offsets are a handful of thrust
points either side of zero, and the spread between the easiest and hardest
setting narrows as the speed class rises.

`SkillScalePoint<N>` and `AIPackSwapping<N>` turn out to be **one record**: both
families are written at the same index with the same `0x18` stride, so a
difficulty level is six fields split across two elements for authoring
convenience. Pack swapping is part of the difficulty setting, not a system
beside it.

Alongside them sit `AIPackSwapping1..3`, one per skill level, with
`numAIPositionSwaps`, `numIntermediatePositions` and `intermediateOffsetSize`.
The reading is that opponents hold assigned slots in a pack and are shuffled
between them a bounded number of times per race, through intermediate slots so
that a swap looks gradual. **That is choreography, not racing** - and the counts
rise with the difficulty setting, so "harder" partly means "the pack churns
more".

## Speed, in Pure: the model Pulse cut down

Pure's `AIRaceStats.xml` is the more interesting document, because Pulse reads
as a simplification of it and the parts that were dropped say what was not
working.

Each of Pure's five classes appears **twice**: a legacy block, and a `<NewStats>`
block that supersedes it in structure. Which one the engine reads is not known -
`<NewStats>` is the obvious guess, and the guess is worth nothing until someone
reads the parser.

What Pure has that Pulse does not:

- **`ZoningStats`** - a piecewise curve of `SpeedPercent` against `Position`,
  six points, where `Position` is an along-track offset relative to the player.
  Separate curves for the pack (`Zone1..6`), for the race leader
  (`LeadZone1..6`) and, in `<NewStats>` only, for the back marker
  (`TailZone1..6`). In `<NewStats>` the pack curve is flat - no adjustment at all
  for a mid-field craft - and every point of adjustment lives in the lead and
  tail curves instead, boosting at the far negative end and cutting at the far
  positive end. Speed as an explicit function of distance to the player is as
  direct a statement of the mechanism as the data gets.
- **`FatigueStats`** - `FatigueStartTime`, `FatigueDecayTime`,
  `FatigueMultiplier`, `RestoreRate`, and a window with `FatigueZoneFront` and
  `FatigueZoneBack`. An opponent that sits inside a window around the player for
  the start time begins to tire and falls away over the decay time, recovering
  at the restore rate. Shipped identically in every class, in both blocks. This
  is an anti-camping rule: it stops one craft welding itself to the player's
  tail for a whole lap.
- **`MasterSpeedTweak`**, a global `min`/`max` pair, shipped degenerate - both
  ends equal, so it scales nothing.
- **`GridSpread distance`** and, in `<NewStats>`, `ShipSpacingVariation` with a
  `min`, a `max` and a `trailingFieldMultiplier` - how far the field is strung
  out, and by how much that varies toward the back.
- **`StartStats` with a `NormalRaceSpeedMax`/`NormalRaceSpeedMin` band** per grid
  slot in the legacy block, rather than a single number, so each craft draws a
  cruise speed inside a range. `<NewStats>` replaces the pair with Pulse's later
  `BaseStartThrust` plus a `StartBoostSpeed`.

And the size of the difference in the one block they share: **Pure's
`PosBalancing` gradient across the eight player positions is several times
steeper than Pulse's**, in the same attribute, with the same meaning. Between
2005 and 2007 the player-position coupling was cut hard, the zone curves and the
fatigue rule were dropped, and a flat gap-based `RubberBanding` block took their
place. Read charitably, that is a studio deciding the visible elastic was worse
than the alternative and reaching for a smaller, blunter version of it.

Pure's `GridPlace8` row is anomalous in every class and in both blocks - it
breaks the monotone gradient the other seven follow. Unexplained; the plausible
readings are the player's own slot or an unused eighth entry.

## What is not known

None of these block writing our own AI. All of them block *reimplementing the
original's numbers*, which is a thing this page deliberately does not propose
doing.

1. **Units.** `AIThrust` - per cent of what? The class's own top thrust, a shared
   constant, the player's current thrust? `MaxSpeedChange` and
   `MinSpeedChangeDist` - track units, spline distance, per cent? **The parser
   is now read in full and it does not answer this**: it stores each attribute
   at a known offset and never uses it. The consumer is a separate function and
   is not identified - the call graph does not resolve in this region, for the
   reason [ai-stats.md](../ghidra/functions/psp-pulse-usa/ai-stats.md#read-this-first-this-regions-addresses-are-unrelocated)
   records.
2. **Sign convention** on Pure's `Position`. The reading above takes negative as
   "behind the player", which makes the lead curve's negative end a catch-up
   boost and its positive end a wait-up. Self-consistent, and unverified - hence
   55.
3. **Which of Pure's two per-class blocks is live.**
4. **`ControllerPO`'s consumer** - see the Autopilot hypothesis above. Pulse's
   parser has no branch for the element at all, which is consistent with it
   being Pure-only.
5. **Weapon AI.** `Data\XML\WeaponAIstats.xml` is a sixth AI file that
   `AiStats_LoadAll` does *not* load. **Its loader is found** (2026-08-17):
   `WeaponAiStats_Load` (`0x08851d88`), called from `AiStats_LoadAll`'s own caller
   on the very next line. The schema is three floats a weapon -
   `useAgainstPlayer`, `useAgainstAI`, `absorb` - and the shipped values are
   nearly uniform, which briefly led to a wrong conclusion that nothing read it -
   **retracted**: `FUN_088518b4` reads it about 400 times a second and compares
   the values against a normalised random draw, so near-uniform weights are
   probabilities rather than dead data. This *is* the fire-or-absorb decision.
   What is open is naming the reader and its caller. See
   [ai-stats.md](../ghidra/functions/psp-pulse-usa/ai-stats.md#the-sixth-ai-file-weaponaistatsxml).

**Where the next session starts:** the consumer, not the parser. The parser is
done - nine functions, named, with the full struct layout on
[ai-stats.md](../ghidra/functions/psp-pulse-usa/ai-stats.md). What is needed is
whoever reads `+0x68` and `+0xa8` off a class record, and the ordinary route to
it (callers of the loader) is blocked by that page's unrelocated-address
problem. A live read under PPSSPP is the cheaper option, and it would settle the
units at the same time.

## What we build instead

Everything below is **ours**. It borrows the original's structure - authored
line, authored corridor, per-class controller - and replaces the speed schedule,
because the speed schedule is the part that makes the opponents furniture.

### The principle

**An AI craft's ceiling is the same physics the player has.** It gets faster by
driving better, never by being handed thrust. That single rule is what makes
"harder" mean something, and what makes a fair race possible at all.

### The shape

1. **Offline, per track**: from the authored racing line and `oag_physics`,
   derive a speed profile - the speed a craft can actually hold through each
   control point, with lookahead braking so it slows *before* the corner rather
   than in it. A solve, not a tuning table, and deterministic, so it can be baked
   or computed at load.
2. **Online, per tick**: the lateral controller above tracks the line; a
   longitudinal controller tracks the profile. Local avoidance handles traffic
   and incoming fire.
3. **Skill is degradation of a competent driver**, expressed as a vector rather
   than a throttle number:
   - braking margin - how early or late relative to the optimum
   - fraction of the available grip budget used through a corner
   - lateral noise on the line, and how much of the corridor it uses
   - reaction latency to pads, weapons and traffic
   - explicit mistake injection, with a recovery behaviour rather than a
     teleport back to the line
   - weapon competence: selection, timing, and how well it aims

   Every one of these is something a human does badly and then does better,
   which is what makes the ceiling legible to a player.

### Rubberbanding, and the config

Default **off**, and the switch gates **both** player-coupled blocks - the gap
term and the player-position term. Gating only the one named `RubberBanding`
reproduces the original's most-complained-about behaviour under a flag that
claims to have turned it off. That is the practical payoff of the recovery
above, and the reason this page exists before the crate does.

```toml
[ai]
difficulty = "auto"          # or 0.0 .. 1.0
rubberbanding = false        # may the AI read the player's gap or position at all
adaptive = "between_races"   # "off" | "within_races" | "between_races" | "both"
mistakes = true
```

`rubberbanding` and `adaptive` are different axes, and both are needed:

- `rubberbanding` answers **what the AI may read**. False means the player's gap
  and the player's race position are not inputs to anything, ever.
- `adaptive` answers **when a skill change may land**. Its input is always the
  player's *absolute pace* - lap and sector times against the track's own
  optimum from step 1 - never the gap. A player who sets a fast lap gets faster
  opponents whether they are first or last, and that is exactly the property
  that distinguishes this from rubberbanding.

Five rules keep `within_races` honest. They are constraints on the
implementation, not preferences:

1. **Adapt on lap boundaries only.** Continuous adjustment is indistinguishable
   from elastic to the player, whatever it reads.
2. **Freeze on the final lap**, so the adaptation cannot decide the finish.
3. **Apply it to the whole field equally.** Adjusting only the craft near the
   player is gap coupling wearing a different name.
4. **Clamp the per-lap step** - on the order of 0.1 on the skill vector - so the
   pack does not visibly transform mid-race.
5. **Derive it from simulation state only** - lap times out of `oag-race`, never
   the wall clock - so a replay reproduces the same adaptation and
   [determinism](../architecture/determinism.md) holds.

Default `between_races` rather than `both`: within-race adaptation is defensible
but it still costs the player the sense of having earned a gap, so it is opt-in.

**Learning opponents are out of scope.** Systems in the GT Sophy and Drivatar
family exist and work, and both need a training pipeline and a policy budget
that make no sense next to a 60 Hz deterministic simulation whose whole point is
being comparable tick-for-tick against a 2007 handheld game. The skill vector
above is where the effort goes.

One thing worth taking from Pure rather than Pulse: **the fatigue rule**. It is
not player-coupled the way `PosBalancing` is - it caps how long any one craft
may hold station in a window, whoever is in it - and it addresses a real problem
that a field of optimal drivers has, which is that identical drivers stop
producing overtakes. It is on the table as an implementation of the mistake
injection line above, not as a port of Pure's numbers.

## What is built

`oag-ai` is two types and one function. It depends on `oag-core` and
`oag-physics` and nothing else - no asset types, so the controller is tested
against a synthetic circle with no disc image anywhere.

| Piece | What it is |
| --- | --- |
| `Line` | The line to drive, as world-space points, closed, and the **corridor** around it. Built by the caller; on a real track the line is `pos - HOVER_LIFT * down + racing_line * lateral` per spline sample and the corridor is `ai_bound_left`/`ai_bound_right` rebased onto it, so both are **the disc's own**, and it is index-parallel to the sample table so a craft locates itself once per tick rather than twice. |
| `Frame` | One point's worth of corridor: the lateral axis, and how far left and right of the line a craft may go. |
| `Tuning` | The controller's constants, shared by the whole field. **Ours** - none of the disc's own per-class values appear in the tree, per [ADR-0006](../architecture/adr/0006-no-copyrighted-content.md). |
| `Personality` | What makes one driver drive unlike the next: departures from that shared tuning, derived from a seed. See [the field is not one driver eight times](#the-field-is-not-one-driver-eight-times). |
| `Driver` | Three `u32`s: where the craft last found itself, its seed, and how many ticks it has driven. `Copy` and `Eq`, and it lives on `Ship` inside the world snapshot, so a replay reproduces the drive. |

### The plant decides the controller's shape

`oag_physics::engine::steering` feeds the **torque** accumulator, so the steering
input commands yaw *acceleration* - not a rate, and not a heading. On top of that
`controls::ramp_steering` moves the steering state toward its target at a finite
rate, so the input lags.

A controller that sets steering in proportion to how far off the line it is is
therefore proportional feedback around a double integrator with a lag. It
overshoots, corrects, overshoots the other way, and keeps doing it. **The first
version of this crate did exactly that and drove wall to wall.** No gain fixes
it; the loop has to be closed one derivative in.

So the law, per tick: locate the craft in a window around its last index; aim a
speed-scaled lookahead down the line; convert the aim point's offset into the
**curvature** of the arc that reaches it (`k = 2e/L²`, with `L` the *measured*
distance to the aim point, not the requested lookahead); multiply by speed to get
a **target turn rate**; and close the loop on the difference between that and the
turn rate the craft actually has, about its **own** up axis - world up is wrong
the moment the track rolls.

Proportional feedback on a rate is derivative feedback on a heading, which is the
damping the plant needs. Three things fell out of the same fix:

- **No separate cross-track term.** The aim point is on the line and the craft is
  not, so the vector between them already carries the offset. The first version
  added a second cross-track term on top, double-counting the same error.
- **The aim point is interpolated onto its segment**, not snapped to the nearest
  sample. A real track's samples are ~2.5 units apart, and a snapped aim point
  steps by that much each time the walk crosses one - a step in the commanded
  turn rate, injected into a high-gain loop.
- **The airbrakes are symmetric until the steering loop runs out of lock.** A
  single airbrake yaws the craft toward the side it is held, so tying one to the
  steering *command* closes a second feedback loop around a plant that already
  oscillates. What the driver does instead is in
  [the airbrakes](#airbrakes-and-what-a-differential-one-actually-does) below.
  The recovered `<Controller>` has no airbrake term at all, which is consistent
  with cornering on steering and a conservative speed target - and is **not**
  evidence for it.

The speed target is `sqrt(lateral_accel / curvature)`, the ordinary cornering
limit, taken over the **sharpest** bend within a braking window rather than at
one point ahead - a corner has to be seen before it is entered.

**The angle that curvature is built from goes through `oag_core::math::acos`,
not `f32::acos`** (2026-08-15). `Line::curvature` called the platform's own from
the day it was written, which `docs/architecture/determinism.md` forbids:
IEEE-754 does not require correct rounding for a transcendental, so two targets
may differ in the last bit, and a bit in a speed target is a bit in the world
hash. Nothing failed for months because **no cross-platform gate had ever run a
driver** - the physics gate drives a scripted input. `oag_ai::probe` and
`crates/ai/tests/determinism.rs` are that gate now, on all three targets in
release and debug, and its scenario asserts its own curvature *spread* so a
circuit that flattened could not pass by testing nothing. The whole disc-backed
`oag-game` suite - every AI ground-truth test on this page among it - was re-run
after the swap, 643 of 643 green, so no bound quoted here moved. The *sweeps*
above (mean speed, time off throttle) were not re-measured: they are manual
runs, not assertions, and a one-ULP angle would not be visible in a figure
reported to three significant digits.

**`lateral_accel` was measured rather than guessed, after it was guessed once.**
It sat at 55 until 2026-08-11, when it was reported from play as "my craft is
faster than the AI, first place within a few seconds". It was, and the reason
was not the top speed: an opponent reached 283 against the player's 169. It was
that the AI spent **45 per cent of a real race off the throttle entirely**,
braking for corners it could hold flat. Swept on `16_Track`, a minute a run,
seven craft:

| `lateral_accel` | mean speed | off throttle | furthest off the line | laps |
| --- | --- | --- | --- | --- |
| 55 | 90 | 45% | 28 | five of seven onto lap 2 |
| 130 | 113 | 16% | 32 | six of seven |
| **180** | **116** | **8%** | 36 | **six of seven** |
| 220 | 99 | 4% | 46 | four of seven |

**The curve turns over**, which is what makes 180 a measurement and not a
preference: past it a craft is not cornering faster, it is sliding wide, and
both the mean speed and the lap count fall while the distance off the line
climbs. Nothing wrecked at any value tried, so the ceiling is where the craft
stops being quick rather than where it crashes.

**And the synthetic harness cannot see any of this.** `oag-ai`'s
`closed_loop.rs` drives an invented hull - `grip_ground` 40 and `accelcap` 60
against the disc's scaled 10 and 17 - which is not commensurate with a real one,
and at 180 that craft slides 81 units off its line. So the fixture carries its
own tuning beside its own hull, and what it measures stays what it was written
to measure: **controller stability, not the speed target**. The speed target is
a real-data question and `the_ai_drives_the_field_along_the_track` is where it
is answered. `the_default_tuning_is_not_this_ones` pins the split.

### Airbrakes, and what a differential one actually does

Reading the force law before designing against it changed what the feature is.
From `oag_physics::airbrake::evaluate`, with `imbalance = L - R` and the left
side held:

| term | effect |
| --- | --- |
| `local_angular.y = speed * turn * imbalance * 0.001` | nose yaws **toward** the braked side |
| `world += right * speed * amount * imbalance` | body pushed **away** from it |
| `world += forward * speed * slide * 0.001` | **forward**, so it adds a little speed |
| grip `max(L, R) * (0.01 - slidegrip) - 1.0` | lateral grip cut as hard as by both sides |

Three of those four are the wrong sign for what "trail braking" usually means,
and a fifth fact settles it: `ShipState::brake`, which is the only thing that
actually slows the craft, rises only while **both** inputs are positive. So a
differential airbrake is **not a brake**. It is yaw authority bought with grip,
and the driver spends it only where the steering loop has run out of authority
of its own - `Tuning::trail_saturation`, plus a deadband that keeps it out of
the small-signal regime and an overspeed gate that keeps it off corner exit,
where the grip is wanted for accelerating and the forward slide term is largest.

The design deliberately does not lean on the forward term: `params::Airbrake`'s
own doc records that whether `drag` accelerates or decelerates is unresolved, and
a feature justified by yaw authority survives that being settled either way.

**A second finding, which made the symmetric brake proportional.**
`oag_physics::controls::update` gates the brake ramp on a strict boolean over the
raw command and its rate never reads the level, so commanding `0.3` on both sides
and commanding `1.0` decelerate *identically*. What the level changes is
`max(L, R)`, which is what the grip coefficient reads. A proportional brake is
therefore a dial on **how much cornering grip the deceleration is bought with**,
not on how hard it stops - which is a better knob than the one that was being
reached for, and is why a driver only just over its target now keeps the grip it
is about to need. The `> 0.0` gate is faithful for the PSP's digital shoulder
buttons and degenerate for the analog axis the input snapshot admits, so
`Tuning::brake_floor` keeps the driver out of the epsilon-command exploit rather
than the recovered law being "fixed".

**Measured**, over 1,800 ticks on the closed-loop oval, differential against
symmetric:

| corners, with 400-to-700-unit straights | peak error from the line |
| --- | --- |
| 120 units - the craft is not grip-limited | 7.5 against 7.4 |
| **60 units - the craft understeers** | **19.2 against 21.4** |
| 45 units | 55.2 against 56.3 |
| 35 units | 98.5 against 98.8 |
| 25 units | 163.3 against 164.6 |

So it earns its place exactly where the argument says it should, and is within
noise where the craft could already make the corner. That small negative on the
wide oval is recorded rather than tuned away.

**And the gain is not monotonic in how tight the corner is**, which is the part
worth knowing before tightening the fixture to "test it harder". A 98-unit peak
error on a 35-unit corner is a craft missing the turn and rejoining rather than
one cornering badly, and a craft that far outside the corridor has no grip left
for extra yaw to spend. The benefit peaks where the craft is **marginally** past
the steering's authority.

`Race::step_opponents` gives each craft **its own** `Environment`, from the
sample its own driver is standing on. Sharing the player's would fly seven craft
against the player's piece of track, which is what the old "they do not move"
comment in `race.rs` was warning about.

**A wrecked opponent stops driving.** A single race runs with `Damage` on, so an
opponent grinding a wall empties its pool exactly as the player does. It is then
stepped with released controls rather than skipped, because the destroyed
sequence runs inside the step and has to finish. Nothing reads its `Eliminated`
state afterwards, so it coasts, settles and is passed - the explosion, the
respawn and the elimination bookkeeping are all unbuilt.

### The field is not one driver eight times

**Reported from play, 2026-08-11: the opponents followed the line so exactly
that they drove in single file.** That is what a field sharing one controller and
one line has to do - there is one best place to be and nothing to separate two
craft that both compute it - and no amount of tuning fixes it, because the
tuning is the thing they share.

So each craft carries a **seed**, and a personality is drawn from it: a set of
departures from the shared `Tuning`. **All of it is ours**; the original varies
its field by scheduling thrust against the player's position, which is the part
[this page refuses to port](#what-we-build-instead).

Since 2026-08-11 the seed is drawn out of a **pilot** rather than out of one
fixed set of ranges - see [pilots](#pilots-and-the-personalities-drawn-inside-them)
below. The axes are the same either way:

| Axis | What it varies | Balanced range |
| --- | --- | --- |
| `line_bias` | which side of the line this craft holds, and how far, as a fraction of the room on that side | 0.25-0.85 either way |
| `wander` | how much of that room it spends drifting about the bias | 0.10-0.30 |
| `wander_rate` | how fast the drift moves | one step per 2.5-7 s |
| `look` | multiplier on the lookahead - how far down the road it aims | 0.85-1.15 |
| `commitment` | multiplier on the grip it assumes through a corner | 0.93-1.05 |
| `patience` | multiplier on how early it starts braking | 0.85-1.20 |
| `trail` | multiplier on how readily it rotates the craft on the airbrakes | 0.7-1.1 |
| `width` | multiplier on how much of the corridor it uses at all | 0.9-1.1 |
| `inside` | bias toward the inside of the corner ahead, **signed by the corner** rather than fixed to a side | 0.0-0.15 |

Two of those do the visible work. **The bias** puts eight craft on eight parts of
the track instead of one line, and **the commitment** gives them eight different
corner speeds, which is what opens a gap and closes it again over a lap - corner
speed goes as the square root of it, so a few per cent of grip is a couple of per
cent of speed. The rest is texture. **The ranges are ours and were picked to
spread the field around a controller that is known to be stable**, not tuned
against anything: the shared `Tuning` stays the centre of the distribution, so
what `closed_loop.rs` measured still describes the middle of the field.

Four properties the implementation has, each for a reason:

- **The room comes off the disc.** `ai_bound_left` and `ai_bound_right` are
  authored per control point, so how wide the field runs is the *track's*
  property and narrows where the artists narrowed it. The bounds are rebased
  onto the racing line when the line is built - the disc stores all three as
  offsets from the sample's own centre, and a driver only ever asks how far it
  may stray from the line it is driving. `Tuning::corridor_use` is **0.6** of
  whichever side is being leant toward, so the corridor's own edge stays a
  backstop rather than a target.
- **The drift moves the aim point, not the steering command.** Noise on the
  command is noise inside a rate loop with a gain of five and nothing filtering
  it. Noise on the point being aimed at is a driver choosing a slightly
  different line, and the controller underneath is unchanged.
- **The corridor is interpolated onto the same segment fraction the aim point
  is.** A bound snapped to the nearest sample steps by ~2.5 units every time the
  walk crosses one - the same trap the aim point itself was already interpolated
  to avoid, one level further out.
- **No `sin`.** The drift is integer-hashed value noise with a smoothstep
  between whole steps (`crates/ai/src/noise.rs`). IEEE-754 does not require
  correct rounding for transcendentals, so a sine resolves to the platform's
  libm and the simulation stops being bit-identical across the three operating
  systems CI runs. See [determinism](../architecture/determinism.md).

**Nothing here reads a clock, the world's generator, or another craft.** The
personality is a pure function of the seed, the drift is a pure function of the
seed and the driver's own tick count, and the seed is a pure function of the
race's seed and the craft's slot - so the same race fields the same eight
characters on every replay, and drawing a personality from `World::rng` (which
would move every later pickup roll) was deliberately not done.

**Until 2026-08-11 the field had no idea another craft existed.** What it has
now is [a view of its rivals](#what-a-driver-can-see-of-the-grid); what it still
has not got is an overtaking line, or anything that touches another craft on
purpose.

### Pilots, and the personalities drawn inside them

A personality is what one craft got. A **pilot** is the distribution it came out
of - so four aggressive craft are four different aggressive drivers rather than
one aggressive driver four times, which is the argument that put a personality on
each craft in the first place, applied one level up. Four ship, all invented:

| Pilot | Drives like |
| --- | --- |
| `balanced` | the field as it drove before pilots existed - the same spans, exactly |
| `aggressive` | brakes latest, commits hardest, least wander, tightest inside line, rotates on the airbrakes |
| `passive` | looks furthest ahead, brakes earliest, gives up corner speed for a tidy line |
| `shy` | runs widest and slowest, and will yield once it can see who is behind it |

Which pilot a slot draws comes from the race seed through **a stream of its
own**, deliberately not the one `Driver::for_slot` uses: sharing it would tie a
craft's pilot to its personality seed, so the aggressive slot would always be the
one that also drew a high commitment and a field of eight would be four
characters wearing eight names.

**The draw order is frozen, and this is the rule to read before touching
`Pilot`.** Draws one to seven are the ones that shipped before pilots existed, in
that order, for ever; a new axis appends after them and never goes between. Two
things rest on it. `Pilot::BALANCED` holds the spans the old code used, so
`Personality::from_pilot(&BALANCED, seed)` reproduces the pre-pilot personality
**bit for bit for every seed** - which is what made introducing pilots a change
to no behaviour and no world hash, and it is pinned by a table of `f32::to_bits`
literals captured before the refactor rather than regenerated after it. And every
pilot consumes the *same* draws in the same order, so an axis a pilot holds fixed
still spends its draw (`Span::fixed`, `Lean`) - skipping it would silently
re-roll every later axis, and only for that pilot.

### What a driver can see of the grid

`oag_ai::Field` is three optional rivals - the nearest **ahead**, the nearest
**behind**, and one **alongside** - plus this craft's own race position.
`Race::field_for` builds it; the driver is handed it the same way it is handed
its line, because `oag-ai` depends on `oag-core` and `oag-physics` and on
nothing that knows what a race is.

Three things about the shape of it:

- **Two geometries, because they disagree.** `gap` and `offset` are measured
  along the track, out of `oag_race::Standing::distance`, so they count laps and
  stay meaningful round a bend - that is what a decision about yielding wants.
  `range` and `cos_bearing` are the straight-line geometry, which is what a
  projectile flies through. The bearing is a **cosine and never an angle**; the
  transcendental is forbidden.
- **`Field::EMPTY` when the track has no closed ring.** A guessed gap would put
  a craft half a lap away in the mirror and have a driver defend against it.
- **Alongside is exclusive.** A craft level with this one is not something to
  defend against or lift for; it is something not to touch.

Three axes spend it:

| Axis | What it does |
| --- | --- |
| `courtesy` | move **away** from the side a craft behind is on |
| `defence` | move **toward** it, covering the line |
| `caution` | lift off - never brake - for a craft close ahead |

**Two budgets, because the two halves fail differently.** A block is capped
tightly: provocation can push `defence` to nearly twice the whole aim budget on
its own, and an uncapped version was reported from play as opponents turning
almost ninety degrees into the player, hitting them, and then hitting a wall - a
block that costs the blocker more than the blocked. A yield gets a looser rein,
because moving *away* from somebody can never swerve into them, but not the whole
corridor either: a craft that hands over the entire track is not being
courteous, it reads as having given up. Enough room to be passed in.

That distinction was learned twice. The first version of the close-range gate
switched off the whole term below fourteen units, and since a packed grid is
permanently inside fourteen units, craft stopped getting out of each other's way
at the only range where it matters. They bumped instead, and the field lost
about an eighth of its pace. Only the **blocking** half stops at close range.

**Courtesy and defence are one signed number, not two terms.** They are opposite
signs of the same quantity, and as two separately gated terms they fight and the
craft jitters between them; as `defence - courtesy` a pilot simply sits somewhere
on the axis. It is continuous in the gap, so it needs no slew limiter and no
state on the driver - it fades in as a rival closes and out as it drops away.

It will not act mid-corner. Blocking where the corridor is a couple of metres
wide and both craft are at the grip limit is how two craft end up in the
scenery, so the same normalised `bend` the inside line uses gates it to zero.
And a rival dead astern has an offset whose sign is rounding noise, so inside a
deadband the side comes from the corner instead - yielding toward its outside,
where an overtaker least wants to be.

`caution` **lifts and never brakes**, because braking hard mid-corner spends the
grip that was holding the corner: the cure would put the craft in the scenery
rather than into the back of a rival. It is here rather than in a later stage
because courtesy and defence both sometimes move craft *toward* each other, and
nothing else in this controller reacts to a closing gap at all.

**A trap this cost, worth knowing before writing another fixture.**
`Body::right` is `orientation * X`, and the disc's own `sample.lateral` points
the same way - but every synthetic corridor in the crate was built as
`Y.cross(along)`, which is the driver's **left**. With symmetric bounds nothing
notices, and nothing did, until a term needed the sign of an offset and yielded
the wrong way. The fixtures now use `along.cross(Y)`.

**And the built-ins have to disagree, not merely declare.** Zeroing `courtesy`,
`defence` and `caution` on all four pilots was tried and every test in the crate
still passed, because the unit tests build a `Personality` by hand and never go
near a pilot. Two tests came out of that: one comparing two variants of one
pilot that differ *only* in the social pair, and one asserting the shipped table
actually spreads across it.

`shy`'s yielding is what those axes are for.

### Being provoked, and shoving

Two more axes spend the same view. `ram` is how readily a driver throws the
craft sideways at a rival level with it, through `ShipControls::sideshift` -
which the physics has implemented in full, so this is a decision rather than a
mechanism. `provocation_ticks` is how long being overtaken stings for.

`Driver` grows two integers for it, and stays `Copy + Eq` because it lives in
the world snapshot: `place`, the position it held last tick, and `provocation`,
a countdown. **A place that got worse is a craft that was just passed**, which
is the whole detector; `0` means nothing has placed this craft yet and is
deliberately not read as first, or the whole grid would be provoked on the tick
the standings first resolve. A countdown rather than a level with a decay rate,
because a rate would be an `f32` on a type that must stay `Eq` - so how
provokable a pilot is becomes how many ticks an overtake adds, which is the same
knob from the other end.

**Provocation scales covering and ramming. It does not touch `commitment`.**
That is the tempting one and the wrong one: the axis is documented as the one
where over what the hull can hold is a driver in the wall, and an angry AI that
drives into the scenery reads as a bug rather than as character. Being cross
changes what a driver does to other craft, not what it asks of its own.

The decision is rolled against `noise::roll`, which is **independent per tick**
where `wobble` is smooth on purpose - a gate driven off a drift fires in long
runs rather than at a rate. It takes a stream index so ramming and firing are
not the same coin landing twice, and like everything else here it draws from the
craft's own seed rather than from `World::rng`, which would move every later
pickup roll.

Four conditions gate a shove, and the last is the one that is easy to forget:
the physics' own `shift_lockout` must be clear (**not** a second cooldown on the
driver, which would be a second source of truth drifting out of step with the
first), no shift already running, a rival actually alongside, and **corridor room
on the side being shifted toward**. A ram that puts the rammer into the wall is
not aggression, it is a bug with a personality.

**What a ram does not do**, and this is worth saying plainly:
`Race::resolve_craft_pairs` discards the `PairContact` it computes and nothing
arms `stun_timer`, so a ram **shoves and nothing else** - no stun, no damage, no
score, no spark. Its payoff is purely positional. The natural follow-on is three
lines: bump the *victim's* provocation off the contact that is already being
computed, which would make the feature social in both directions.

### The field only got round two circuits in twelve

**Found 2026-08-11, and it had been true the whole time.** `Driver::drive`
locates a craft with a 48-sample *window* around its last index - deliberately,
so a circuit that passes over itself cannot make a craft latch onto a stacked
section - and `Driver::index` started at **zero**. A grid sitting at sample
2,500 therefore never found itself, and steered at whatever piece of track it
believed it was on.

It went unnoticed because every test on this page runs the default circuit, and
the default circuit's start line happens to sit near sample zero. Measured
across the disc's twelve circuits, one craft alone for a minute each:

| | before | after |
| --- | --- | --- |
| circuits where the craft never got going (300-900 units travelled) | **six**, one of them grinding itself to destruction | none |
| circuits starting more than 280 units off their own racing line | eight | none |
| circuits where a lone craft completes a lap | two | **five** |

The fix is one line - seed the index with a search over the whole line at spawn,
where the cost is paid once - and
`race_ground_truth::every_craft_starts_on_its_line_on_every_circuit` is the test
that was missing.

### Nothing recovered an opponent that left the circuit

**Found 2026-08-12, chasing the seven circuits the fix above did not reach.** On
those the craft was covering seven to eleven thousand units a minute at racing
speed while sitting thousands of units from the sample its driver believed it
was on. The obvious reading - that the racing line was built wrong, since
`Spline::from_track` concatenates every path in file order - is **wrong**, and a
probe killed it in one run: every circuit on the disc carries two or three
paths, the working ones included, and a healthy craft crosses path boundaries
with the index following it and a residual of three to nine units throughout.

What the probe actually showed is a craft leaving the geometry and *continuing
in a straight line for the rest of the race* - eight thousand units in fifty
seconds, off-track distance and residual growing together. Two mechanisms were
missing, not one:

1. **`Race::respawn` was the player's alone.** The code said so in a comment.
   Nothing on the grid but slot 0 could be put back.
2. **A `Reset` volume cannot catch a craft receding into open space.** With
   per-slot respawn wired in, the twelve circuits produced **zero** respawns
   between them: there is no authored geometry out there to touch.

So there is a second, **invented** trigger next to the reset volumes -
`RESCUE_HALF_WIDTHS`, eight times the track's widest half-width, held for
`RESCUE_TICKS` (a second and a half) - keyed on the distance from the craft to
the sample *its own driver* believes it is on. That measures the thing that went
wrong rather than a proxy for it, and it costs one distance per craft per tick
instead of a search of the whole sample table. It is ours under
[ADR-0006](../architecture/adr/0006-no-copyrighted-content.md); nothing in the
RE tree describes what the original does here.

| | before | after |
| --- | --- | --- |
| circuits a lone craft completes laps on | five | **nine** |
| circuits with a **clean** lap (no recovery) | five | five |

**The clean-lap number deliberately did not move**, and that gap is the finding.
Four circuits now complete every lap and never manage one without being
recovered. A lap the craft had to be rescued during is not a lap it drove, so
the benchmark counts them separately and the respawn count is the
driving-quality metric: on an empty circuit at the top difficulty a competent
driver should never need recovering at all. The rescue is a safety net for a
race with seven other craft shoving, not a fix for a driver that flies into the
scenery.

Three circuits - 05, 14 and 07 - still never registered a second lap at this
point, and 07 did it *without being rescued once*. All three are the circuits
whose spline carries three paths rather than two, which was the lead the next
section followed: two of the three were an AI line built out of a path the lap
never drives, and the third turned out to be a beached craft.

`a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round` carries both
ratchets so neither number can quietly fall, and
`an_opponent_that_flies_off_the_circuit_is_put_back_on_it` pins the rescue
without needing the disc.

### And nothing recovered the player either

**Found 2026-08-17, from a hand-driven lap.** A player took a Turbo onto a crest
on Vertica reversed (`14_Track`), left the circuit at about 240 units per second
and fell for the remaining eight seconds of the log - distance to the spline 59,
147, 227, 355, 521, 666, 802, 954, height down to -866 - with no recovery of any
kind. The race did not end and could not continue.

The two mechanisms above were per-slot by then, but only one of them applied to
slot 0: the player had the authored `Reset` volumes and nothing else.
`Race::lost_off_the_circuit` cannot be reused for the player, because it measures
the craft against the sample its own *driver* believes it is on and nobody
steers the player's craft - `driver.index` sits wherever it was left.

**The `Reset` volumes were assumed to be the player's net, and they are not.**
Counted for the first time here, across ten circuits in both directions:

| circuits | `Reset` colliders |
| --- | --- |
| `06_Track`, `14_Track`, `16_Track` | **0** |
| `01`, `04`, `07`, `09`, `13` | 1 |
| `03` | 3 |
| `05` | 7 |

Three circuits author **none at all** - including `16_Track`, the default, and
`14_Track`, the one the failure was reported on, and `06_Track`, the circuit this
page already records a human falling through. On those, a craft that leaves has
nothing to touch by construction, and no amount of work on the reset path could
have recovered it.

So slot 0 gets a trigger of its own, `Race::lost_off_the_track`: the **true**
distance to the nearest spline sample, past `PLAYER_RESCUE_HALF_WIDTHS` (two
half-widths) for `PLAYER_RESCUE_TICKS` (three quarters of a second). Both are
ours, both were measured before they were chosen, and both differ from the
opponents' pair because the quantity is different - a true distance rather than a
believed one. One autopiloted craft alone on ten circuits at ace, 6,000 ticks
each, peaked at **0.74 half-widths** on the worst circuit and 13.7 units on
`13_Track`, the one with the authored jump: a circuit's racing line runs *through*
its jump, so a craft in the air over one is near the spline rather than far from
it. Two half-widths is between 2.7x and 5.8x the worst healthy excursion.

Where the craft is put back is **latched**, not reconstructed: `Race::last_on_track`
is the last sample the craft was within the threshold of. By the time the dwell
expires the craft is hundreds of units below the circuit, where the nearest
sample can belong to a different part of it - which is
[the same trap](#wrong-answer-one-the-rescue-index-reconstructed-backwards) the
opponents' side already paid for once.

Measured after: thrown off `14_Track` reversed at 240 units per second, the craft
peaks 119.5 units out and is back within 4.0 units of the spline ten seconds
later. On `05_Track`, where a craft leaves unprompted, the peak falls from
**7,983 units to 193.7**.

**What the player deliberately does not get is the stall rescue.** Being
teleported off a wall one is scraping along, while holding the throttle, is a
thing done *to* a player rather than for them, and a human who is wedged can see
it and back off - which is exactly what an opponent cannot do. The consequence is
visible on `05_Track`: the craft is recovered when it leaves and still does not
lap, because that circuit's line runs above its own collision surface and the
craft beaches. That is the open thread the section below records, not a gap in
this one.

`crates/game/tests/off_track_rescue_ground_truth.rs` carries all of it against
the disc; `race/tests/respawn.rs` pins the trigger, the control and the latch
without needing one.

### Where a craft comes off: two wrong answers and the measurement that settled it

**Read this section as a record of method, not just of a finding.** The
conclusion changed twice, both times because the measurement was of the wrong
quantity, and the two wrong answers are more instructive than the right one. The
answer is at the bottom: craft come off **at the path boundary**, which is where
the first pass said they did, for a reason it had not established.

#### Wrong answer one: the rescue index, reconstructed backwards

Recording each rescue against the driver's index gave one tight cluster per
circuit - 06 at 1520-1522 on all six of its rescues. The rescue fires
`RESCUE_TICKS` after the craft leaves, so the index at the rescue is not the
index it left at; subtracting an estimated 325-sample flight put 06's cluster on
its path boundary at 1196, and that arithmetic became "craft come off at the
seam". **The arithmetic was not sound.** The flight offset is not a constant: it
is however far the index keeps advancing while the craft is airborne, which
depends on the craft. A right answer reached this way is still a guess.

#### Wrong answer two: distance from the line

Watching the craft leave, rather than reconstructing it - a lone Ace, every
circuit, recording the driver index the first time the craft is more than **two
max half-widths** off the line point its own driver is standing on:

| circuit | path boundary | left the corridor at | laps cleanly |
| --- | --- | --- | --- |
| 16 | 1752 | never | yes |
| 03 | 1748 | never | yes |
| 04 | 1684 | never | yes |
| 13 | 1524 | never | yes |
| 10 | 132 | 336-381 | yes, and is rescued 5× |
| 06 | 1196 | 1290-1291 | no |
| 02 | 1160 | 1274-1357 | no |
| 09 | 964 | 1106-1123 | no |
| 01 | 1764 | 114-118 | no |
| 05 | 1016, 2048 | 195-283 | no |
| 14 | 892, 1784 | 823 | no |
| 07 | 992, 1984 | **never** | no |

This says nothing leaves at a boundary: 06 94 samples past its own, 02 114 past,
09 142 past, 01 at 118 with its only boundary at 1764. **It is also wrong, and
for the same reason as the first.** A threshold on distance is crossed when the
craft has *finished* leaving, and a craft that has already been falling for a
second and a half has carried its index a long way with it. The number is real;
it just does not date the event.

One fact from this pass survives and is worth keeping: every path ends in
near-coincident samples - steps of 0.6, 0.3, 0.1 into the boundary against a
1.50-unit median - on all twelve circuits including the four that never put a
wheel wrong. Degenerate tails alone are harmless.

#### The one ordering fault that is real: a path the lap never drives

The same probe found something else, and it is a different bug wearing the same
clothes. Three circuits carry **three** paths where the rest carry two, and on
all three the lap ring walks only two of them:

| circuit | paths in file order | paths the ring walks | step from path 0 into path 1 |
| --- | --- | --- | --- |
| 05 | 0, 1, 2 | 0, 2 | 816 units, `cos -0.36` |
| 14 | 0, 1, 2 | 0, 2 | 1,249 units, `cos -0.89` |
| 07 | 0, 1, 2 | 0, 2 | 1,160 units, `cos -0.97` |

Path 1 is the other branch of a split. `Spline::from_track` concatenates every
path in file order, so the AI line spliced a stretch of track the lap never
drives into the middle of the lap - and pointing the wrong way. A driver
reaching the end of path 0 finds its next thousand samples a kilometre away, its
48-sample window cannot follow, and the index sticks. **These three circuits are
exactly the three that never register a second lap**, and 07 does it without
being rescued once: it is not flying off, it is stuck at sample 991 going
nowhere.

The fix is to walk the lap ring rather than the file, `Race::ai_order`: the AI
line is the ring's paths in travel order, and a path the ring never walks is not
in it. `Course::path_order` supplies the sequence; the mapping scans the sample
table per path rather than assuming `Course` and `Spline` resample identically,
because they carry a `STEPS_PER_SEGMENT` each and a mapping resting on those
being equal would break silently if either moved.

**Two index spaces exist from here on**, and conflating them is the bug this
introduces if it is done carelessly. `driver.index` indexes the racing line;
`Spline::nearest` returns a sample index. `Race::ai_sample` is the only bridge,
and it wraps, because the caller asking for the sample after this one is asking
a lap question - `ai_order[i + 1]`, never `ai_order[i] + 1`, which at the end of
a path is somewhere else entirely. `Race::respawn` speaks the sample table's
space for both its callers, since the player reaches it from `Spline::nearest`.

What it was worth, against the same solo benchmark:

| | before | after |
| --- | --- | --- |
| circuits completing a lap | 9 | 11 |
| circuits managing a *clean* lap | 5 | 6 |
| 07 | 1 lap, never rescued, stuck at 991 | 4 laps, clean 48.5s |
| 14 | 1 lap | 4 laps, still no clean one |
| 05 | 1 lap | 1 lap |
| the other nine | - | **identical**, to the lap time, the respawn count and every loss index |

That last row is the check that matters: on those nine the mapping is the
identity permutation, so anything moving there would have meant the bridge was
wrong. Nothing moved.

**05 did not improve, and it is not a line fault.** Its line is now the ring's
2,976 samples against a 2,976-point course, so the ordering is right. Traced
tick by tick, the craft drives into the scenery around index 590, ends up
grounded doing two units per second, decays to zero, and sits there for the
remaining seventy seconds of the run. It is 70 units off its line and the rescue
triggers at 336, so nothing recovers it. **A craft that has stopped making
progress has no recovery at all** - the rescue asks "is it far from its line",
which a beached craft is not. That is a missing mechanism rather than a tuning
question, and it is the next one.

##### Built 2026-08-17: a second dwell, on speed rather than distance

`Race::stalled` counts consecutive ticks on which a craft is **stopped, asking to
move, and has not finished**, and puts it back after `STALL_TICKS`. All three
terms are load-bearing and each excludes a craft that is stationary for a good
reason.

**The threshold was measured before it was chosen, and the separation is total
rather than a margin.** A lone opponent on each of the twelve circuits at each of
the four difficulties, counting the longest unbroken run below several speeds
with the full gate applied:

| cell | longest run under 1 unit/s | longest run under 5 units/s |
| --- | --- | --- |
| novice `05_Track` | **1,215** | 3,634 |
| novice `07_Track` | **266** | 698 |
| skilled `07_Track` | **0** | 267 |
| every other cell | **0** | 3-11 |

The two beachings are the only cells that spend *any* consecutive time below one
unit per second, so `STALL_SPEED` is 1.0 and `STALL_TICKS` is 120 with room on
both sides. The five-unit column is what rules out the obvious wider threshold:
it would also catch skilled `07_Track`, a craft crawling through a slow section
and recovering by itself in four and a half seconds, and rescuing that one would
cost a clean lap on a circuit that manages one today. The three-to-eleven-tick
runs in the last row are the standing start.

The **thrust** term was measured too rather than assumed: the throttle reads a
full 100 on every tick of both real beachings, so the gate holds continuously
through the thing it has to catch. And the **finished** term is not theoretical -
a craft that has taken the flag coasts to a halt and stays there, measured at
2,242 consecutive ticks on `05_Track` at ace, which is the longest apparent stall
on the whole disc and is not a stall at all.

**What it is worth, and what it is not.** novice `05_Track` goes from 1 lap to 2
and its longest motionless spell from 1,215 ticks to the 119 the threshold
allows; novice `07_Track` gains a recovery it needed. **The Ace benchmark is
byte-identical** - same lap times, same two respawns, same loss indices - because
no healthy craft ever meets the condition. But it does not fix `05_Track`: every
place the rescue fires is inside or immediately before a documented run of line
samples with nothing under them (05 at driver indices 116, 634 and 159 against
the measured run at 161-211; 07 at 2373 against 2364-2376), so the craft is put
back and comes off again in the same place. **The line running above the surface
is the cause and it is still open**; this bounds the symptom.

One limitation this buys, stated rather than discovered later: an opponent that
ends up physically on the split's other branch now has no line beneath it at
all. That is strictly better than stalling at the seam, and the rescue net
catches it, but it is a gap.

#### The measurement that settled it: watch contact, not distance

**Found 2026-08-12, and it moves this out of the AI entirely.** Tracing 06
through its departure showed the craft is already 22 units *below* its own line
at index 1240 and falling - `grounded` at zero, thrust at zero, speed climbing
109 to 151 while it drops. Of the 23 units it is off the line, 22 are vertical
and 6 lateral against a 70-unit half-width. It is not cornering wide. It is
falling straight down through where the track should be.

So the useful detector is not distance from the line but **loss of contact**:
the driver index at the last grounded tick before a flight of a second or more.
That reads much earlier than the corridor test - 06 at 1230 rather than 1290 -
and every failing circuit shows the same signature, a fall of 500 to 780 units:

| circuit | boundary | last contact | falls |
| --- | --- | --- | --- |
| 16, 03, 04 | - | never leaves the ground | - |
| 13 | 1523 | 20 | flies 60 ticks, **lands**, laps cleanly |
| 01 | 1763 | 78 | 510-550 |
| 05 | 1015 | 215 | 500-660 |
| 07 | 991 | 268 | 680 |
| 10 | 131 | 137 then 345 | first flight lands; the second falls 500-690 |
| 14 | 891 | 762 | 750-775 |
| 09 | 963 | 1026 | 570-600 |
| 06 | 1195 | 1230 | 600-710 |
| 02 | 1159 | 1232 | 625-720 |

This reads 35 to 73 samples past a boundary rather than 94 to 142 - closer, and
still not it, because sixty ticks of no contact is itself a delay. The version
that finally dates the event is the tick trace below, which reads the *first*
tick of lost contact and puts 06 at 1195 against a boundary at 1195.

`the_racing_line_has_track_under_it_where_it_is_known_to` then casts down the
surface normal at every sample of the line, one probe reach either way, which is
what a craft's own antigravity probes hold onto. **Nine of the twelve circuits
have a stretch of racing line with nothing under it**, each one a single
contiguous run, and the three with none - 16, 03 and 04 - are three of the
circuits that never lose a craft. A second cast sixty reaches deep splits the
result in two:

- **Nothing down there at all**: 13's 31 samples and 40 of 10's 72. That is an
  authored **jump**, and 13 flies it, lands and laps cleanly, which is what a
  jump is supposed to look like.
- **A surface, one to sixty ride-heights below**: 09's 55, 02's 70, 14's 82,
  05's 134, 01's 12, 07's 13. Not a gap in the track - a racing line running
  *above* the track.

**Three of those runs start within eight samples of a path boundary** (09 at 967
against 963, 02 at 1167 against 1159, 06 at 1196 against 1195), and that is the
lead the tick trace below cashes in.

Reading what is actually down there settles the first split. On a healthy stretch the cast finds `Floor` at **half a ride height
below the line**, everywhere, on every circuit. On 13's 20-48 it finds nothing
within sixty reaches - an authored jump. On 09's 967-1021 it finds a `Wall` at
the first sample and then `Floor` sinking to 4.8 ride heights and climbing back
over sixty samples: a dip the line flies straight over. **But 06's notch is a
single sample at 1196 with `Floor` at 0.5 either side of it**, and 06 loses its
craft at 1230, where the floor is exactly where it should be. So the unsupported
stretches are real and previously unmeasured, and they are *not* where the craft
comes off.

**Where it does come off is the middle of the track.** At the tick contact is
lost, on every failing circuit, the craft is comfortably inside its own
corridor - 06 at +0.3 in a corridor running -23.3 to 0.0, 09 at +0.7 in -16.0 to
10.8, 02 at +2.9 in -11.9 to 12.1 - at ordinary racing speed, with `Floor` half a
ride height beneath it. Then it is airborne for three hundred ticks and seven
hundred units down. The floor half a ride height beneath it is the last floor
there is: one tick later it is gone.

So it is neither cornering wide nor driving off an edge. **A craft stops being
held by a piece of circuit it is driving straight down the middle of, and that is
a physics or collision question rather than an AI one.**

#### What the tick trace says: the floor drops away at the path boundary

Casting straight down from the craft itself, every tick, through 06's departure -
and the same shape appears on 02 and 09:

```
i1195  moved 1.16  grounded 0.50  floor below   4.0
i1195  moved 1.17  grounded 0.50  floor below  12.3
i1195  moved 1.18  grounded 0.50  floor below  25.5
i1195  moved 1.18  grounded 0.50  floor below  27.9
i1195  moved 1.21  grounded 0.00  floor below  27.1
```

**The floor goes from four units under the craft to twenty-eight over five units
of travel.** 02 goes 3.8 to 39.4 in one tick, 09 goes 4.6 to 29.5. All three are
at the path boundary: 06's is 1195/1196, 02's 1159, 09's 963.

Then the craft glides - descending a fifth of a unit a tick, nowhere near fast -
and closes on that lower surface, 27.9 down to 16.0 by index 1218. **And then at
1244 the cast returns nothing at all.** It has flown past the end of whatever was
down there, and from that point it accelerates into open air: 2.7 units a tick,
speed 163, for the rest of the race.

Three things that follows from this, and one that does not:

- **It is not falling *through* a floor.** Every cast agrees the surface is
  genuinely absent, before and after the craft passes.
- **It is not a jump the physics fails to complete.** The hole at 1196-1200 is
  five samples, about seven units - a tenth of a second at the speed the craft
  is doing. It descends 0.2 units crossing it. There is nothing to clear.
- **It is not a missing magnetic floor.** `Surface::MagFloor` is decoded,
  hoverable, and `oag_physics::maglock` gates on it. It is also heavily used:
  1,730 of 03_Track's line samples sit over MagFloor and 03 laps perfectly
  cleanly, as does 16 with 346. Meanwhile 04, 10, 13 and 14 have **no** MagFloor
  collider at all, and 04 is clean while the other three are not. The surface
  kind does not sort the circuits.
- **What is not explained**: the track is back at its normal height five samples
  later, and the craft only sank 0.2 units - so why it never regains contact.
  One lead: at its last contact the craft sits at lateral `+0.3` in a corridor
  running `-23.3` to `0.0`. 06's authored AI corridor there is entirely to the
  *left* of the line, and the craft is marginally outside its right bound.

#### The decoder drops nothing, and the spline is above the track

Two measurements settle it, and the second retracts the retraction above.

**Every triangle in the file reaches the collision world.** 06_Track decodes
three nodes - Floor 3,178 triangles, Wall 1,836, MagFloor 520 - for 5,534, and
the world holds 5,534. 09_Track: 5,833 and 5,833. Nothing is dropped, so the
missing floor is not a mesh this repository failed to load. (That rules out a
dropped mesh, not a misplaced one: matching counts do not prove the vertices
land where the original puts them.)

**The craft comes off at the path boundary after all.** Printing the line's own
height through 06's join:

| sample | y |
| --- | --- |
| 1190 | 15.3 |
| 1194 | 14.2 |
| **1196** | **1.8** |
| 1198 | -1.5 |
| 1200 | -5.0 |
| 1204 | -7.7 |

The line **drops 12.4 units between 1194 and 1196** and keeps diving. That is the
same discontinuity the first pass on this page called out and the second pass
retracted: `across 14.78, cos 0.79`. The retraction was wrong, and it was wrong
because both corrected measurements - "94 to 142 samples past the boundary", then
"35 to 73 past" - were thresholds crossed *long after* the event, by a craft
whose index had gone on advancing. Watching contact itself puts 06 at 1195/1196
against a boundary at 1195, 02 at 1163-1167 against 1159, 09 at 963-964 against
963. **On the nose.**

09 has no jump - it descends smoothly at about 1.4 units per 1.5-unit sample,
which is a 43-degree dive - and it comes off just as reliably. What it shares
with 06 is the other half: through the dive the collision floor is **up to 26
units below the line**, `Floor` at 4.8 ride heights where a healthy stretch reads
0.5, recovering over sixty samples.

So the shape of it: **the authored spline runs above the track surface through a
steep descent, and a craft cannot be pulled down as fast as the line falls
away.** It separates at the top of the drop, glides, and by the time it has sunk
to where the surface was, it has flown past the end of it. The path boundary
matters where there is one because a 12-unit step is a drop the craft meets in a
single sample.

#### The reconstruction is right, and the craft does pitch

Two candidates were open: a spline that genuinely leaves its track, or sample
positions reconstructed wrongly on steep ground. **The second is dead.** `down`
is a unit vector everywhere (0.9905 to 1.0000 across all twelve circuits) and
`Sample::pos - HOVER_LIFT * down` lands *exactly* on the collision surface -
median 0.0, ninetieth percentile 0.4 or better, on every circuit. The
reconstruction is correct. It is the top one per cent of samples that stand 20
to 50 units clear, and 04's worst is 0.2 against 14's 50.5.

**Nor is the craft flying level off a slope.** Its nose tracks the surface
closely while it has contact - on 09, track diving `+0.387` against a nose at
`+0.381`. What it cannot do is keep up with a dive that *steepens*: over ten
samples 09 goes `+0.387` to `+0.514` while the nose reaches only `+0.426`, and
the craft leaves at the point the gap opens.

And **04_Track does exactly the same thing and is fine.** It lifts off on its own
dive - `grounded` 1.00 to 0.50 to 0.00 over twenty samples, nose lagging the
slope the whole way - and lands, because the surface stays under it. So going
airborne on a dive is normal. Failing to come back is the bug.

#### It is not how fast the craft arrives

The obvious AI-side fix - slow into the dive so the pitch response keeps up - is
ruled out by measurement. The same benchmark at all four difficulties:

| difficulty | clean | round | respawns | 09 | 02 | 01 | 06 | 14 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Novice | 5 | 11 | 254 | 5 | 5 | 6 | 4 | 4 |
| Skilled | 7 | 11 | 30 | 5 | 6 | 7 | 6 | 5 |
| Elite | 6 | 12 | 40 | 6 | 7 | 8 | 6 | 5 |
| Ace | 6 | 11 | 41 | 6 | 7 | 8 | 6 | 6 |

**A novice falls off the same places just as often as an ace**, at a fraction of
the speed. Nothing a driver does with the throttle reaches this.

The novice total hides the other half of the same coin: **226 of those 254
respawns are 13_Track**, the circuit with the authored jump. A slow craft does
not clear it, falls in, is recovered, and does it again. So the geometry demands
a *minimum* speed in one place and is indifferent to speed in the others.

#### The player falls off too, which settles it

**Reported from a hand-driven lap of Vertica (`06_Track`) on 2026-08-12, and it
is the single most important fact on this page.** A human at the controls falls
through the same drop:

```
tick  1920  grounded 1.0  height   3.94  at [-308.8,  20.8, 184.4]
tick  1980  grounded 0.0  height  14.07  at [-288.7,   8.1, 306.5]
tick  2040  grounded 0.0  height -57.18  at [-248.9, -58.4, 409.4]
```

Nearest line index at those three positions: 1146, **1230**, 1306. The AI loses
contact at 1195-1230. **Same drop, same place, no AI involved.** Every
opponent-side reading on this page was downstream of this.

The three casts say what happens. At 1920 the floor is 4.0 under the craft -
normal. At 1980 it is **14.0 under the craft**, and the probes reach
`ride_height`, which is **5.5**. The surface is still there; the craft simply
cannot see it. `grounded` goes to zero, and with it the spring, the damper and
`DOWNFORCE_SCALE`, all of which are gated on it. By 2040 there is nothing under
the craft at all: it has flown beyond the geometry while falling.

Under the *line* the cast reads `Floor at 8.5` at all three indices - the
healthy value. The track is fine. The craft is off it.

So the mechanism, exactly: **a step down bigger than the probe reach takes the
surface out of view in a single tick, and once the craft is airborne it travels
forward faster than gravity brings it down.** At 125 units/s a craft covers 2.1
units a tick, so any surface falling away faster than 5.5 units per 2.1 of
travel - about 69 degrees - is lost outright, and 06 adds a 12.4-unit spline
step on top of a 66-degree ramp.

That is a hair's breadth from what an instantaneous downward ray can do, and it
is what "collision detection missing" means here: the probe is a point ray at
one instant, not a sweep from where the craft was to where it is.

#### What is left, and what an alternative would have to do

The remaining question is genuinely a reverse-engineering one: **what holds the
original's craft onto a steepening dive**, given ours is faithful to a force law
scored at confidence 91 in which the suspension is compression-only, the probes
reach exactly `ride_height`, and `DOWNFORCE_SCALE` scales by `grounded` - so an
airborne craft has no downforce at all, by construction and on purpose.

Worth stating because it is the tempting fix and it is the wrong one: nothing
here should be solved by pulling a craft down onto the spline. The craft *should*
fly off a crest; that is what the model is for, and 04 demonstrates it working.
An alternative has to make the craft **reconnect**, not stop it leaving - and the
measurement says reconnection fails because on those stretches there is no
surface within a probe reach for fifty to a hundred and thirty samples, not
because the craft is in the air.

What is settled either way: **tuning the driver cannot fix this**, and a grip
figure fitted against it would be fitted against whatever this turns out to be.

#### One of the five went away, and not from the AI

Reading `Antigrav::rebound_jump_time` - parsed since the handling loader landed
and never read by the force law - took **Basilico (01_Track) from eight
respawns and no clean lap to three and a 36.9s one**, and clean circuits from
six to seven. 09 dropped from six respawns to five and 05 from two to one.

The parameter arms the `landing_rebound` response, and it arms it *in the air*:
`Ship_UpdateCraft` (`0x08849df0`) keeps an airborne clock and
`Ship_HoverTwoPoint` zeroes the landing clock only once that clock has passed
the parameter, so a hop shorter than it lands on the ordinary `rebound`. This
crate had been resetting on every touchdown edge, which meant a craft flickering
in and out of contact was under `landing_rebound` almost permanently. On a
circuit whose losses were marginal that was the difference between recovering
and not. See `docs/ghidra/functions/psp-pulse-usa/engine.md`.

It changes nothing about the four that remain: those craft leave contact where
there is nothing within a probe reach to come back to, and a damping coefficient
does not reach that.

#### And then two more, from how the probes are cast at speed

`Ship_CastHoverProbes` branches at its head on `craft+0x2ec <= 50.0` - the
cached `|dot(velocity, forward)|`, recovered at 199/199 against a capture. Below
that speed it casts two independent rays. **Above it, it casts one**, and if the
front ray hits, the rear probe's entire hit record is manufactured from it: the
point translated by the world-space vector between the two probes, then pushed
along `up` by `dot(normal, forward) * 6.0` to follow the slope. The hit *flag*
is copied with it.

That last detail is the mechanism. **At speed, a front probe in contact
guarantees a rear probe in contact**, so `grounded` cannot read `0.5` - the
craft cannot shed half its suspension because one probe overran a lip. This
crate cast two independent rays at every speed and did exactly that; the
symptom is visible in a hand-driven capture as `grounded 1.0` then `0.5` then
`0.0` over two seconds.

Every fall on this page happens between 115 and 160 units per second, which is
to say entirely inside the branch this crate did not have.

| | before | after |
| --- | --- | --- |
| circuits managing a clean lap | 7 | **9** |
| 05 de Konstruct | never a second lap | clean 36.8s, **zero** recoveries |
| 09 The Amphiseum | no clean lap, 6 recoveries | clean 47.4s, 3 recoveries |
| 01 Basilico | 36.9s, 3 recoveries | 34.3s, 2 recoveries |
| still no clean lap | 02, 05, 09, 14, 06 | 02 Metropia, 14 Fort Gale, 06 Vertica |

All twelve circuits now complete every lap. Vertica, the circuit the whole
investigation started on, is still one of the three - its losses moved from
index 1518 to 1506 and its recoveries from six to seven, so the fast path
changed where it comes off without stopping it.

The `6.0` is transcribed and not derived: the exact slope correction over the
9-unit probe spacing would be `-9 * dot(n, forward) / dot(n, up)`, and the
original uses a flat constant. Kept literal.

#### And the last three, with a mechanism the original does not have

Metropia, Fort Gale and Vertica survived all of the above, and a hand-driven
craft still fell through two of them. Logging the vertical velocity through Fort
Gale's drop said why, and it is not something a faithful port can fix:

```
air 1   vy  -76.09 (-1.27/tick)  line dy   -8.1  below  -  above   2.0
air 2   vy  -77.24 (-1.29/tick)  line dy   -8.9  below  -  above   3.4
air 44  vy -119.01 (-1.98/tick)  line dy  -81.6  below  -  above  79.8
```

**On the first airborne tick the craft is already under the track.** Nothing
below it, and the surface two units over its head - a gap that only grows. It
rides the descent at 139 units/s, which at that gradient is **76 units/s
straight down while still in contact**; the ramp ends, and it crosses to the
underside in the single tick between one contact test and the next.

The contact test casts *down* from the hull along `-up`. Once the surface is
above the hull, no ray of any length finds it and penetration escape has no
downward hit to measure. Gravity here is about 80 units/s², so the craft is at
119 units/s down after a second and 160 after two, while the track it left
levels off overhead.

So `oag_physics::hover::sweep` asks the question the discrete test cannot: *did
the hull pass through a surface during this tick?* Each probe's motion segment,
from where it was before the integrator ran to where it is now, is cast against
the world; a crossing of a hoverable face from the front puts the craft back on
the surface with its inward velocity removed. **It is ours, not recovered**, and
it is the only mechanism in `oag-physics` that is.

It runs unconditionally, and the first attempt did not. Gating it on
`state.grounded == 0.0` looked obviously right and was exactly wrong: `grounded`
is set by `forces::evaluate` *before* the integrator moves the craft, so on the
one tick that matters it still reports the contact the craft had on the way in.
The gate skipped the only case it existed for, and the benchmark moved by one
recovery on one circuit - which is what a fix that never fires looks like.

Ungated:

| | before | after |
| --- | --- | --- |
| circuits managing a clean lap | 9 | **12** |
| circuits needing no recovery at all | 5 | **11** |
| total recoveries across the disc | 22 | **2** |

Both remaining recoveries are 01's, one of them on the standing-start lap.
Neither determinism reference moved: no probe scenario ever crosses a surface,
so the sweep never fires in one.

### What this whole thread was actually about

Five sections of this page were written chasing an AI fault that did not exist.
The opponents were fine. What was wrong was an AI line built out of a path the
lap never drives, a landing response armed on every flicker of contact, a
missing speed-gated probe path, and a contact test that could only look one way.
The AI's own tuning has not been touched since, and the twelve-circuit benchmark
went from two clean laps to twelve.

The order in which those were found is not the order they should have been
looked for in. **A player driving the same section by hand fell through it too,
and that single fact - available at any point - would have ruled out the AI on
day one.** It is worth trying before a measurement campaign, not after.

**Grip stays at 180 until the corner departures are understood.** A sweep across
all twelve circuits says 220 and 260 are quicker (38.6s and 38.4s mean clean lap
against 39.8s) and reach six clean circuits rather than five - but the respawn
count barely moves across the whole range (29 at grip 120, 35 at 180, 33 at
260). A number that does not respond to grip is not answering a grip question,
and fitting grip against it would be fitting to whatever is actually throwing
craft off those corners. `sweep_grip` is the harness; re-run it after.

### A solo lap is the benchmark

Pace should be judged on **one craft, alone**, before it is judged in traffic.
With seven craft out there every number mixes how well a craft drives with how
much the traffic cost it, and those move in *opposite* directions when
aggression changes - which is how a field measurement came to report that
raising `lateral_accel` past 180 made things worse when a solo lap says it
keeps helping (42.3s at 180, 40.7s at 220, 40.1s at 260). The field number was
measuring interference.

`solo_lap_ticks` is that benchmark and `a_solo_craft_laps_faster_at_a_harder_setting`
is its regression: on Talon's Junction a novice laps in 53.8s and an ace in
42.3s.

### Difficulty

Four levels - `novice`, `skilled`, `elite`, `ace` - chosen on the pre-race page
or as `[ai] difficulty` in `settings.toml`. It greys to N/A in the three modes
that field no opponents.

**A level degrades a competent driver; it never boosts a weak one.** The top
level is the tuning that was measured against real hulls on a real circuit, and
each level below takes something away. The other direction is the one that goes
wrong: a baseline tuned for a novice and multiplied upward has no measurement
behind its top end, so the hardest setting would be the least tested one.

**And nothing in it reads the player.** The original schedules opponent thrust
against the player's race position and the gap to them, and this page
[refuses to port that](#what-we-build-instead). A level decides how good the
opponents are before the lights, and then they race. An easy field that is easy
because it waits for you is a rigged one, and it can be felt.

Three things move, and one falls:

| | novice | skilled | elite | ace |
| --- | --- | --- | --- | --- |
| grip it believes it has | 0.30 | 0.48 | 0.70 | 1.00 |
| turn rate it may ask for | 0.60 | 0.80 | 0.95 | 1.00 |
| appetite for weapons, ramming, blocking | **0.00** | 0.45 | 0.80 | 1.00 |
| how often it misses a braking point | 1.00 | 0.50 | 0.15 | **0.00** |

The grip scale is **even in corner speed rather than in grip**, because the
target goes as the square root: those four are 0.55, 0.69, 0.84 and 1.00 of the
measured corner speed. Novice's 0.30 is not a fraction picked for shape - it is
about 54, which is what `lateral_accel` was before it was swept, and therefore
the one point on the scale with a play report behind it.

Aggression is **zero at novice** deliberately. A slow opponent that still shoots
you in the back is not an easy race, it is an annoying one.

Mistakes are the axis `docs` previously called blocked on the expensive half.
They are not: a mistake here is holding the throttle through a braking point,
and the *recovery* needs no code at all, because the controller follows a line
and getting back to it is the thing it was already doing. `Driver::mistake` is
an integer countdown, and seed zero never errs so every exact assertion keeps
measuring the plain line-follower.

Measured on `16_Track`, a minute a run, seven craft, mean speed:

| novice | skilled | elite | ace |
| --- | --- | --- | --- |
| 93 | 109 | 119 | 120 |

Nothing wrecked at any level. **`elite` and `ace` are within one per cent**,
which says the ceiling is no longer the tuning: at that point the field is at
full throttle 84 per cent of the time and what is left on the table is
elsewhere - the field takes only about 30 per cent of the track's speed
pads, which looked like the next thing worth fixing. **It was not** - see
[pads](#pads-built-measured-removed) below.

### Pads: built, measured, removed

The field crosses only about **30 per cent** of a real circuit's speed pads,
which looked like the largest thing left on the table once the pace tuning was
at its ceiling. So pad-seeking was built - a channel on the driver's view giving
the next pad's distance along the line and offset across it, resolved once at
the start by projecting each pad's centre onto the nearest spline sample - and
then it was **removed on the measurement**. It is written up here because the
next person to notice that statistic will have the same idea.

Four measurements, all the same direction. Against a control that does not chase
at all, on `16_Track`: at a greed of 0.2 the field took 108 pads against the
control's 105 and completed the same 22 laps; at 0.4, 105 pads and 21 laps; at
0.8, 101 pads and 19 laps. Chasing pads **earns nothing and then costs laps**.
Worse, with it on the four difficulty levels came out 94 / 91 / 103 / 114 - both
slower across the board and *out of order*, so the setting had stopped being
monotonic.

The reason is visible in the resolved offsets: **the artists put the racing line
through the pads worth having.** A pad far enough off the line to need a detour
is one the track is offering as a *choice*, and at these speeds the detour is
the worse half of it. Nothing about that is specific to this AI - it is a
property of how the circuits are authored.

Two things worth keeping from the exercise. Pads are authored **in pairs
straddling the line** (`281@13` beside `281@-1` on `16_Track`), so any future
"nearest pad" search picking by index alone can send a craft at the far one of a
pair - a longer trip for the same boost, and it can end up between the two
having taken neither. And a weapon pad is a different question from a speed pad:
what it is worth is not lap time but what the driver intends to fire, so if it
is ever tried again it should ride `trigger` rather than the difficulty. It was
tried, and a trigger-happy field collected 17 weapons against a timid field's
19 - noise, and confounded besides, since a pilot that fires more empties its
slot sooner and can pick up again.

### Pilots you can author

**Every `.toml` file in `$XDG_CONFIG_HOME/oag/pilots/`** - usually
`~/.config/oag/pilots/` - is loaded at the start of a race, and **the filename
becomes the pilot's name**: `winston.toml` is a pilot called `winston`. There is
no list to register one in and no limit on how many are kept.
[`assets/ai/example-pilot.toml`](../../assets/ai/example-pilot.toml) is a
documented one to copy under any name.

A file named after one of the built-ins - `aggressive.toml`, `balanced.toml`,
`passive.toml`, `shy.toml` - **replaces** it rather than adding another pilot,
which is how a player retunes one of the four without editing the tree.

Each axis is a `[low, high]` range and each craft draws inside it. One number is
not offered as shorthand: a pilot whose every axis is fixed is one driver
wearing eight hulls, and making that the easy thing to write would be odd.
Anything a file leaves out comes from `balanced`.

`crates/game/src/pilots.rs` owns it, and **`oag-ai` gains nothing** - the loader
lives in the composition root, which already owns `toml`, `serde` and `dirs` for
`settings.rs`. `include_str!` plus serde would put a TOML parser inside a
gameplay crate to save one indirection, which is what rule 1 of the
[workspace layout](../architecture/workspace-layout.md) exists to prevent. The
built-in four stay Rust constants.

Two deliberate departures from `settings.toml`: **unknown keys are an error**,
because a typo'd axis that silently does nothing is the exact frustration this
removes and there is no migration history to protect; and **nothing is rewritten
on load**, because these are hand-authored files with the author's own comments
in them.

**`commitment`'s ceiling is the most important number in the module.** It is the
only thing between a hand-written file and an opponent that corners faster than
the physics allows - which is not a faster driver, it is a driver in the wall,
on someone else's machine, in a race they did not author. Every axis is bounds
checked, and a reversed range is rejected outright, because a reversed span
still *draws* and lands outside the range it appears to state with nothing
complaining.

#### And the determinism problem it creates

A pilot read from the player's config directory makes race behaviour depend on a
file that **differs between machines by design**, which `Ship::handling` - the
existing unhashed tunable - does not: that comes off the disc and is the same
everywhere.

So `Driver` carries a `pilot: u32` digest and **it is hashed**. Left out, two
machines running "the same race" with different `winston.toml` files would
produce identical world hashes for different races: a gate claiming an agreement
it does not have, which is worse than no gate. In it, they visibly disagree.

Three things to accept with that:

- It is a digest of the **resolved numbers, never the name** - two files both
  called `winston` saying different things must not agree.
- Thirty-two bits cannot be inverted, so it says *these differ* and not *which
  file differs*. The composition root prints every roster entry's name and
  digest at the start of a race, and which pilot each grid slot drew, so a bug
  report carries it.
- **Adding a pilot reshuffles which one every slot draws**, because
  `pilot_for_slot` indexes into the roster. That is a *different* field, not a
  corrupted one - and the digest is what makes the difference visible instead of
  silent, which is the whole argument for the field.

The committed determinism constants stay machine-independent only while nothing
in that scenario flies a pilot. It does not, so the digest is `0` throughout,
and a tripwire in `the_run_visits_the_paths_it_claims_to_cover` asserts it -
because if an edit ever gave that scenario an opponent, the constants would
quietly start depending on `~/.config/oag/pilots/`.

### Shooting at somebody

**An opponent fires as of 2026-08-11**, and the delete-this-paragraph moment is
worth marking: the firing *mechanism* was complete long before - rockets fly and
collide with craft - and what was missing was only ever a driver willing to pick
a target. So this stage is a policy, not a system.

`Driver::wants_to_fire` returns the slot it would shoot at, and five gates stand
in front of it:

1. There is a craft ahead at all.
2. It is inside `WEAPON_RANGE` **and outside `WEAPON_MIN_RANGE`**. The near
   bound is the one that is easy to miss: `projectile::blast` damages every
   craft in radius *including the firer*, so point-blank is a rocket fired at
   yourself.
3. It is inside a cone, as a **cosine and never an angle** - the transcendental
   is forbidden, and the caller measured the dot product already.
4. **The road between here and there is straight enough**, by the same
   `max_curvature` the Turbo gate uses. A rocket round a corner is a rocket in a
   wall, and reusing that notion of "is this a straight" keeps the two decisions
   consistent instead of inventing a second one.
5. The trigger roll.

**`trigger` is a rate, not a probability**, and the arithmetic is worth stating
because the shape invites the wrong reading. It is rolled every tick, so what it
sets is the *expected delay* between a target entering the cone and a rocket
leaving: at full trigger about a fifth of a second, at a fifth of it about a
second and a half. Read as "a five per cent chance" it looks far too small; read
as a delay it is a driver taking a moment to line up.

The target slot comes back even though a Rocket is unguided and never uses it,
because the decision is the part worth testing and a guided weapon will want it.

**Two things that would have been silent bugs.** The player's firing path passes
owner `0` because the player *is* slot 0; an opponent doing the same would put
rockets in the air owned by the player, which `projectile::step` flies straight
through the player before detonating them on whoever actually fired. And the
curvature gate cannot be validated synthetically at all - a straight fixture
always passes it, so a threshold set too tight would kill the feature on real
geometry and no test in CI would notice. That one is answered by a disc-backed
count instead: **159 rockets from six of the seven opponents over two minutes of
a real race.** The seventh is whoever is leading, which has nobody ahead to shoot
at.

### Pads, and what an opponent does with a pickup

**Both classes of pad now cross every craft**, not only the player's. Each racer
keeps its own broadphase row and its own "which pad was I on last tick" edge; a
weapon pad's *refresh timer* stays shared, because it belongs to the pad, so a
craft that takes a pickup leaves the pad inert for whoever arrives next. An
opponent draws from the `ai` column of the shipped `<Pickupodds>` table rather
than the `human` one - a distinction the disc's own data makes. `front` and
`back` remain unreachable: they need race *positions*.

**What an opponent does with what it draws is a policy, and the original's is now
located but not ported.** `Data\XML\WeaponAIstats.xml` holds three values a weapon
- `useAgainstPlayer`, `useAgainstAI`, `absorb` - and `FUN_088518b4` reads them
every frame, compares them against a normalised random draw and writes the fire
request. So it is a **weighted random choice gated on an along-track range test**,
not the flat rule below. Porting it needs the reader named and its other inputs
read. Until then this is still invention, and it is
kept small enough to be obviously provisional: **Turbo is fired at once but only
where the driver is not braking, the line stays clear for as far as the boost
carries, and there is nobody close enough ahead to arrive in the back of**; a
**Rocket is aimed** - see [shooting at somebody](#shooting-at-somebody); and
**everything else is absorbed**, which pays energy into the pool.

**The second half of that Turbo rule was added 2026-08-11, and the measurement is
worth keeping.** "Not braking" is the driver's own braking horizon, and that
horizon is built from the speed the craft is doing *now*: a craft at 126 looks
about 160 units ahead, fires, and arrives at the next corner at 270 having never
looked at it. On `16_Track` one craft in a field of seven did exactly that and
left the circuit - **and it never touched a wall on the way**, so nothing in the
damage model saw a problem. `Driver::allows_speed` is the fix: it asks the same
cornering limit the throttle brakes against, for the *boosted* speed, over the
distance the boost covers. `TURBO_SPEED_RATIO` (2.2, in `race.rs`) is the
assumed speed-up, ours and a rule of thumb - erring high is the safe direction,
because it makes a craft keep the pickup rather than spend it into a wall.

**It does not close the hole**: with the gate in, one craft in 28 craft-minutes
on `16_Track` still leaves the track after a Turbo. It is no longer gone for the
race - that measurement predates the rescues, and both halves of the grid are
recovered now (`Race::lost_off_the_circuit` for an opponent,
`Race::lost_off_the_track` for the player, who does the same thing with a Turbo
and was the last one left falling). What the gate is still for is the craft not
leaving in the first place: a recovered lap is not a lap it drove. The speed
target and the lookahead ceiling (`look_max`, 90 units - a third of a second at
270) are the next places to look.

### The field burns

**Every craft has its own exhaust as of 2026-08-11.** There used to be one
`Exhaust` and it belonged to slot 0, so an opponent got the speed pad's *force*
and nothing on screen: seven craft drove the whole circuit dark.

`Race::exhaust` is now `[Exhaust; MAX_SHIPS]`, advanced per craft in
`Race::advance_exhausts` off *that* craft's thrust and *that* craft's speed - so
an opponent lifting for a corner dims while the leader on the straight does not.
Each craft also lays its own ribbon from its own `engine_flare` locator, carried
through its own pose, and arms its own boost plume: at the speed pad
(`ExhaustFlare_OnSpeedupPad`'s site, which the original calls from inside the
per-craft update), and on a Turbo it decides to spend.

Three things are worth stating about how it is put together, because each is a
decision rather than a detail:

- **One generator per craft, not one drawn from eight times.** Slot 0 keeps
  `EXHAUST_SEED` unshifted and an opponent takes `EXHAUST_SEED + slot`. A single
  shared stream would make the player's flicker depend on how many opponents the
  mode fields, so every exhaust number pinned against a single-craft capture
  would have moved the day a grid appeared behind it. Adjacent seeds are safe
  because `Rng::new` runs a SplitMix64 expansion before the first draw. Pinned
  by `the_players_flicker_does_not_depend_on_the_field_behind_it` and
  `the_field_does_not_flicker_in_lockstep`.
- **Still render-only state.** The array sits on `Race`, not in `World`, for
  exactly the reason one `Exhaust` did: it must not enter a snapshot a replay or
  a determinism hash reads. Eight of them change nothing about that.
- **The shared vertex buffers had to grow.** `oag_render::exhaust` uploads every
  flare and every ribbon through one pipeline, and `Pipeline::upload` clamps with
  `min` - so an undersized buffer would have dropped the last craft's ribbon with
  nothing in the logs. `MAX_TRAILS` is the grid, `MAX_TRAIL_VERTICES` is eight
  ribbons, and `MAX_SPRITES` now carries its own arithmetic: eight flares plus a
  projectile and a blast flash per projectile slot, 40 of 48.

**Liveries landed 2026-08-15** (`crates/game/src/livery.rs`), and with them
the per-slot nozzle this section used to want: `Setup::nozzles` is one locator
per slot, read off that slot's own hull, so each plume samples its own team's
UV keyframes rather than the player's. `Race::force_boost_state` stays slot
0's alone, deliberately - a pose comparison is against one captured craft, and
that is a capture-only synthetic warmup, not a field-wide mechanic.

### The field is placed

**Every craft counts its own laps as of 2026-08-11**, and the field can be
ordered. `oag_race::Standing` is the small per-craft thing - lap, distance round
the circuit, finish tick - that sits on every `Ship` and therefore inside the
world snapshot, so a replay reproduces the finishing order. `Race::places()`
returns all eight positions and `Race::player_place()` the player's.

**The lap rule is not written twice.** A lap is a wrap of the distance-along, and
the two-half `LapGate` is what stops a craft rocking over the line and scoring
one; both live in `oag_race::state` and `Standing` reaches them through
`wrapped_forward` and `LapGate::advanced`. The player's displayed lap is then
*assigned from* their standing rather than counted a second time. Two lap
counters that disagreed would put a craft in a position it is not in.

`RaceState` stays what it was: the player's clock, best lap, Zone counters and
finish condition. Those are the player's alone and are not per craft.

Ordering is finishers first (by when they finished), then racers by distance
covered, then **by slot index**. The last rule is not cosmetic - eight craft on
the grid are at exactly the same distance before anyone has a fix, and a
comparison that left that to chance would feed an arbitrary result into
simulation state.

**The place reaches the HUD as of 2026-08-11**, which is what makes any of this
visible: `Race::readout()` fills `place` from `player_place()` and
`Arcade_HUD.xml`'s `Position` group draws it. Two things had to be settled to get
it on screen, both on [hud.md](../ui/hud.md): the layout authors `Position` and
`TotalTime` at the *same* anchor, so one of them has to yield; and the field size
was being drawn without the place, so a single race showed `POS` beside a bare
`8`.

**And one real bug fell out of running it**, worth reading before trusting a
first-lap position: `Standing::distance` measured lap 1 from the start *line*,
while the grid straddles that line - the authored `Start Position` on `16_Track`
sits ~138 units before it and the eight slots run forward from there. So a craft
still on the grid read as almost a full circuit *ahead* of one that had crossed
and driven on. Measured on a real load: the parked player read `progress 4956` of
5,094 units against seven craft on 1,500, and was placed **first** while last.
The fix is one rule in `Standing::distance` - a craft whose lap gate has never
seen the near half has not reached the line yet, so its distance is negative -
and the same run now places that player 8th of 8.

### What an opponent still does not get

**This section has been wrong more often than any other on this page**, because
each stage that landed left it saying otherwise. Everything below was checked
against the code on 2026-08-17; check it again before trusting it.

What is still missing, and it is now a short list: **reaction latency**;
**adaptation between races**, which was blocked on per-opponent lap times and is
not any more; and **anything at all happening when a craft is eliminated**. The
player's *own* position is on screen; an opponent's is not - `PosTag0`-`PosTag7`,
the floating name tags, are runtime-anchored to a rival's projected screen
position and nothing computes that.

**What left this list, with what to look at instead of re-deriving it:**

| Left | When | Where it lives |
| --- | --- | --- |
| An exhaust of its own | 2026-08-11 | [The field burns](#the-field-burns) |
| A respawn when it falls off | 2026-08-12 | `Race::lost_off_the_circuit` |
| A weapon aimed at somebody | 2026-08-12 | `Driver::wants_to_fire` |
| Knowing the other craft are there | 2026-08-12 | `oag_ai::Field`, and the `courtesy`/`defence`/`caution` axes |
| Mistake injection, with a recovery behaviour | 2026-08-12 | `Driver::blunder`, `Driver::mistake`; the rate comes from `Difficulty::tune`, **not** from a config key |
| Difficulty selection | 2026-08-12 | `oag_ai::Difficulty`, and one `[ai] difficulty` key in `settings.rs` |
| A livery that is not the player's | 2026-08-15 | `crates/game/src/livery.rs` |
| **A lap time of its own** | **2026-08-17** | `oag_race::Standing::best_lap_ticks` |
| **Being recovered when it stops** | **2026-08-17** | `Race::stalled`, `race::STALL_SPEED` |

**Two of those rows are narrower than they look.** The `[ai]` section holds
`difficulty` and nothing else - `rubberbanding`, `adaptive` and `mistakes` are
specified on this page and deliberately absent from `settings.rs` rather than
present and ignored, so the *block* is not built, one key is. And "difficulty
selection" means the scale is built and measured end to end on real geometry
(`difficulty_ground_truth.rs`, four levels ordered across five seeds); the
**pre-race menu row** for it is still in the believed-done-never-verified state
`HANDOVER.md` records.

**The lap clock is the one that unblocks something else.** A `Standing` now times
its own laps and keeps its own best, starting the clock at the craft's first
crossing of the line rather than at the standing start - which matters because
the grid is laid out behind the line and every slot is a different distance back,
so lap 1 timed from the start would be a different error for each craft. Slot 0
therefore carries two clocks, its `Standing`'s and `RaceState`'s, and
`the_standings_clock_agrees_with_the_players` is what stops them drifting apart.

**Being recovered when it stops** closed the hole the rescue left: it asks
whether a craft is *far from* its line, which catches one flying into open space
and misses one beached against the scenery at zero speed. A second dwell counter
watches for a craft that is stopped while asking to move and has not finished,
and puts it back after two seconds. See
[Where a craft comes off](#where-a-craft-comes-off-two-wrong-answers-and-the-measurement-that-settled-it)
for the measurement the two constants come from, and note what it does **not**
fix: `05_Track`'s racing line runs above its own collision surface for 134
samples, which is why a craft beaches there in the first place, and that is still
open.

**Craft-to-craft collision landed 2026-08-11**, and it was never an AI gap - the
engine had no two-body path for anybody. `Body_ResolveContactPair`
(`0x0884ef30`) is now read and reimplemented in `oag_physics::pair`; the recovery
is on
[contact-response.md](../ghidra/functions/psp-pulse-usa/contact-response.md#body_resolvecontactpair-0x0884ef30-the-two-body-path).

The finding worth carrying: **a craft bounces off another craft with `e = 0.1`
and off the track with `e = 0.4`**, in the same build. The pair resolver hardcodes
its restitution where the one-body path reads the per-body field, it has a
separating-velocity gate the one-body path does not, and it applies **no
friction** at all. Track contact was already right and is untouched.

**The detection is ours, and there is nothing to recover.** `0x08815ccc`, the
box-against-box narrowphase a pair of craft would dispatch to, turns out to be
`jr ra; nop` - it reports nothing, ever. So craft-to-craft contact never comes out
of Pulse's narrowphase at all, and what does feed the pair resolver two craft has
not been found. `pair::overlap` tests an oriented box against an oriented box over
the hull's own `<Misc width height length>`, which is what the shape kind says a
craft is.

It began as a sphere of half the hull's diagonal and that was **far too big** -
on a 4 x 2 x 8 hull the sphere reaches 4.58 units where the flank is 2 away, so
craft shoved each other while visibly apart. Reported from play; the box test was
the first fix and it was still too big, which is the second finding:

**`<Misc width height length>` is a bounding box, not the hull.** A real Pulse
craft measures `5.5 x 3.5 x 13`, and half that length - `6.5` - lands within a
whisker of the `6.45` bounding radius `oag-view` reports for the shipped Feisar
mesh. So the authored box *bounds* the model rather than tracing it, and a
Wipeout hull tapers hard toward the nose: at full size a craft collides along its
whole length at the width of its widest point, and two craft passing bump where
the models visibly miss.

`pair::HULL_SCALE` is the knob, and it is **ours with nothing behind it** - there
is no original to match, so only play sets it. The tests are written against the
constant rather than against literals, so retuning it moves one number and not a
test's meaning.

Still not armed: the `stun_timer` and its gate exist and nothing sets them,
because what posts the pending impulse at `entity->0x4c + 0x110` is still unread -
the two calls at the tail of the pair resolver are the candidates and they do not
rebase onto a function start, so they were left alone rather than guessed at.

### What is verified, and where

| Claim | Where | Runs in CI |
| --- | --- | --- |
| A craft driven round an oval tracks its line instead of weaving | `oag-ai`'s `tests/closed_loop.rs`, with the **real force law** on a flat plane | yes |
| The controller steers back toward its line, brakes for a corner, is damped by its own turn rate | `oag-ai`'s own tests, against a synthetic circle and straight | yes |
| Curvature reads `1/radius` on a circle | `line::tests::curvature_approximates_one_over_the_radius` | yes |
| Every opponent is stepped with controls a driver chose | `race::tests::the_opponents_are_driven_rather_than_parked` | yes |
| A grid of eight leaves the line, goes the right way, and is still on the track after ten seconds | `race_ground_truth::the_ai_drives_the_field_along_the_track` | **no** - needs a disc image. **Run and passing as of 2026-08-11.** |
| Two runs of one race stay identical, drivers included | `race_ground_truth::a_driven_field_replays_identically` | **no** - needs a disc image. **Run and passing as of 2026-08-11.** |
| The field is placed, and being further round earns a better place | `race_ground_truth::the_field_is_placed_by_how_far_round_it_is` | **no** - needs a disc image. **Run and passing as of 2026-08-11.** |
| Seven seeded drivers hold seven different parts of the corridor, and the field strings out | `oag-ai`'s `closed_loop::a_field_of_seeded_drivers_does_not_drive_one_line` and `..._strings_out`, on the oval with an invented corridor | yes |
| A seeded driver stays inside the corridor, and the same seed drives the same race | `oag-ai`'s `closed_loop::a_seeded_driver_stays_inside_the_corridor`, `the_same_seed_drives_the_same_race` | yes |
| The drift is smooth, bounded, and different per seed | `noise::tests`, five of them | yes |
| The field spreads across the **disc's own** corridor, both sides of the line | `race_ground_truth::the_field_spreads_across_the_ai_corridor` | **no** - needs a disc image. **Run and passing as of 2026-08-11**: seven craft spread **16.6 units** across a corridor with 10.5 to spare either side of the line. |
| The differential holds a corner the steering alone cannot, and does not ring | `closed_loop::a_differential_holds_a_corner_the_steering_alone_cannot` and `..._does_not_ring`, on a 60-unit oval with the airbrake fixture | yes |
| And stays out of the way where the craft could already make the corner | `closed_loop::a_differential_barely_touches_a_corner_the_craft_can_already_make` | yes |
| The differential brakes the side the nose is turning toward, is dead inside its deadband, waits for the loop to run out of lock, and never acts on corner exit | four tests in `driver::tests` | yes |
| A yaw request never cancels the brake it is layered on, and survives a craft already braking flat out | `driver::tests::a_differential_never_cancels_the_brake_it_is_layered_on` and `..._survives_a_craft_already_braking_flat_out` | yes |
| Braking starts at the floor rather than an epsilon, and climbs with the overspeed | `driver::tests::a_brake_climbs_with_the_overspeed_and_never_starts_below_the_floor`, `a_driver_never_brakes_below_the_floor` | yes |
| **The default closed-loop fixture has no airbrakes at all** | `closed_loop::the_default_fixture_has_no_airbrakes_and_the_regression_bounds_know_it` | yes |
| And the airbrake fixture is not inert either | `closed_loop::the_airbrake_fixture_actually_slows_and_yaws_a_craft` | yes |
| **The balanced pilot reproduces the pre-pilot personality bit for bit** | `pilot::tests::the_balanced_pilot_reproduces_the_personality_that_shipped_before_pilots_existed`, against literals captured at `fd35d8f` | yes |
| Every pilot consumes the same draws in the same order, and a fixed span or lean still spends its draw | three tests in `pilot::tests` | yes |
| Seed zero is the plain line-follower whatever the pilot, and no pilot asks for more grip than the hull has | `pilot::tests::seed_zero_is_the_plain_line_follower_whatever_the_pilot`, `..._no_pilot_asks_for_more_grip_than_the_hull_has` | yes |
| Which pilot a slot draws does not track its personality seed | `pilot::tests::the_pilot_a_slot_draws_does_not_track_its_personality_seed`, over 400 races | yes |
| All four pilots get round the oval on **both** handling fixtures, and the aggressive one out-runs the shy one from every seed | `closed_loop::every_built_in_pilot_gets_round_the_oval`, `an_aggressive_pilot_gets_further_round_than_a_shy_one` | yes |
| An inside line leans into a corner and does nothing on a straight | `driver::tests::an_inside_line_leans_into_the_corner_and_not_on_a_straight` | yes |
| A shy driver moves away from the craft behind it and an aggressive one covers it | two tests in `driver::tests` | yes |
| Equal courtesy and defence cancel instead of fighting | `driver::tests::equal_courtesy_and_defence_cancel_instead_of_fighting` | yes |
| Neither yielding nor blocking happens mid-corner, and the gate scales rather than switching | `driver::tests::neither_yielding_nor_blocking_happens_mid_corner` | yes |
| A rival beyond awareness range, or squarely astern on a straight, is not reacted to | two tests in `driver::tests` | yes |
| A driver lifts for a craft it is closing on, and one with no caution does not | `driver::tests::a_driver_lifts_for_a_craft_it_is_closing_on` | yes |
| Each difficulty's **leader** covers more ground than the one below it, on real geometry | `race_ground_truth::every_difficulty_is_quicker_than_the_one_below_it` | **no** - needs a disc image. **Run and passing 2026-08-11**: 6075 / 6911 / 7302 / 7857, nothing wrecked. |
| **A driver that sees nobody drives exactly the line it did before stage 3** | `driver::tests::a_driver_that_sees_nobody_drives_exactly_the_line_it_did_before`, over all four pilots | yes |
| Two craft that can see each other do not converge, and a yielding leader gives way where a covering one does not | `closed_loop::two_craft_that_can_see_each_other_do_not_converge`, `a_yielding_leader_gives_way_where_a_covering_one_does_not` | yes |
| The field a driver sees excludes itself, orders rivals correctly, and is empty on a track with no ring | four tests in `race::tests` | yes |
| The built-in pilots disagree about yielding rather than merely declaring it | `pilot::tests::the_built_in_pilots_disagree_about_yielding` | yes |
| A driver that loses a place is provoked and calms down again, and one that gains a place is not | three tests in `driver::tests` | yes |
| **A craft placed for the first time is not treated as having been overtaken** | `driver::tests::a_driver_that_has_never_been_placed_is_not_provoked_by_its_first_placing` | yes |
| Provocation is capped however often a driver is passed, and a provoked driver covers harder | two tests in `driver::tests` | yes |
| A ram goes toward the craft alongside, waits for the physics' own lockout, and never goes toward a corridor edge it has no room for | five tests in `driver::tests` | yes |
| `Driver` stays `Copy + Eq`, which is what keeps it in the world snapshot | `driver::tests::a_driver_stays_copy_and_eq`, a compile-time guard | yes |
| A roll is uniform, per-tick independent, and its streams disagree | five tests in `noise::tests` | yes |
| A driver fires at a craft ahead and inside its cone, and not at one beside it, out of range, at point-blank, or round a corner | six tests in `driver::tests` | yes |
| A trigger-happy driver fires sooner than a cautious one, and a provoked one sooner than it did calm | two tests in `driver::tests` | yes |
| **An opponent's rocket is owned by the slot that fired it** | `race::tests::an_opponents_rocket_is_owned_by_the_slot_that_fired_it` | yes |
| **The aiming gates actually open on real geometry** | `race_ground_truth::an_opponent_fires_at_a_craft_ahead_on_a_real_circuit` | **no** - needs a disc image. **Run and passing 2026-08-11**: 159 rockets from six of seven opponents in two minutes. |
| A pilot file round-trips, and one with an impossible commitment, a reversed range, an unknown key or a bad lean is rejected **by name** | six tests in `pilots::tests` | yes |
| The roster is sorted by name rather than by the filesystem, and a file may replace a built-in | two tests in `pilots::tests` | yes |
| Two pilots differing in one number digest differently, and no built-in digests to zero | two tests in `pilots::tests` | yes |
| **The suite never reads the developer's own pilot directory** | structural: every test goes through `parse` or `load_from` with a path it made itself | yes |
| **The determinism scenario flies no pilot**, so its constants cannot become machine-dependent | tripwire in `determinism::the_run_visits_the_paths_it_claims_to_cover` | yes |
| A branch the lap never drives is not in the AI line, and the lap's own path order is not the file's | `race::tests::a_branch_the_lap_never_drives_is_not_in_the_ai_line`, `course::tests::an_alternate_branch_is_left_off_the_ring` | yes |
| **A track whose lap drives every path maps a line index onto itself**, which is what says the two index spaces agree where they should | `race::tests::a_track_whose_lap_drives_every_path_maps_a_line_index_onto_itself`, `..._a_track_with_no_ring_still_gets_every_sample_in_file_order` | yes |
| A driver's index reads the sample the lap puts there, and one past the end wraps to the start | `race::tests::an_ai_index_reads_the_sample_the_lap_puts_there` | yes |
| Eleven of twelve circuits complete a lap and six manage a clean one | `race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round` | **no** - needs a disc image. **Run and passing 2026-08-12.** |
| **Three circuits have track under every sample of their racing line, and nine do not** | `race_ground_truth::the_racing_line_has_track_under_it_where_it_is_known_to` | **no** - needs a disc image. **Run and passing 2026-08-12**: 16, 03 and 04 at zero; the other nine between 5 and 134 samples. |

`race::tests`' craft is built on `Handling::ZERO` with an empty collision world,
so no force law runs there at all and those tests assert only that the controls
reached the physics. `closed_loop.rs` does run the real force law, but on an
infinite flat plane with invented handling - it says the controller is stable
against a plant of the right *shape*, which is the property that was broken.

**And `closed_loop.rs`'s default fixture has no airbrakes**: `handling()` ends
`..Handling::ZERO` and sets neither `airbrake` nor `brakes`, so the ramped
airbrake states never leave zero, `ShipState::brake` never rises, and every
`imbalance` term is zero. Braking there is `thrust = 0` and nothing else, and
every number that file quotes - the 84.1 against the 7.3 included - was measured
on a craft whose airbrakes do nothing. Anything about airbrakes needs
`handling_with_airbrakes()`, which is a second fixture on purpose: extending the
first would silently re-baseline the regression the file exists for.
**Whether the field actually drives a circuit is the ground-truth pair, and
nothing else.**

The wall-to-wall weave was reported from the running game, not caught by a test.
`closed_loop.rs` was then written, watched failing at **84.1 units** of peak
error from the line, and is at **7.3** after the rewrite. That is evidence the
structural fix works; it is not a substitute for looking at the screen.

**The two disc-backed tests were run for the first time on 2026-08-11 and both
pass**, on `16_Track` off a real image. Until then they had only ever been
compiled. Every claim about a full grid actually getting round a circuit rests on
that run and on nothing earlier.

Nothing anywhere asserts a lap time. There is no measurement of the original's
opponents to compare against, and the speed law is this project's own.

**The single-file report was from play too, and the numbers behind the fix were
measured on real data rather than asserted.** Driving `16_Track` for a minute,
four race seeds, seven opponents each: with one shared personality every craft
peaked at **13.0-13.7 units** from the line - seven craft doing the same thing to
within a decimetre - and with seeded personalities they peak at **11-22**, on
both sides of the line. The same runs are where the Turbo departure above was
found, and where the claim that it is now one craft in 28 rather than zero comes
from.

## Where this sits

The prerequisites were all done before this landed: the
[racing line and corridor](../formats/track.md), the
[grid](../ghidra/functions/psp-pulse-usa/grid.md) that seats eight craft within
2.4 units of the original's own positions, and a controller of the right shape
already in `oag-trace` (see [autopilot-planning](../tools/autopilot-planning.md);
it is a planning aid and says of itself that it is no claim about the original).

`race::Options::opponents` remains, and now means only "field a grid from a mode
that would not ask for one" - which is what the grid's own ground-truth test
wants. See the M5 `AI` item on the [roadmap](../overview/roadmap.md).
