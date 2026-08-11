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
| Weapon selection and firing | **not looked at** - `WeaponAIstats.xml` is the hook | - |
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
   `AiStats_LoadAll` does *not* load, so it has its own loader. It pairs with
   the `WEAPON AI %d` string. Not looked at at all.

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

**What an opponent does with what it draws is a policy, and nothing about it is
recovered.** The original decides this in `Data\XML\WeaponAIstats.xml`, a sixth
AI file that `AiStats_LoadAll` does not even load - so it has its own loader,
which has not been looked for. Until that is read, this is invention, and it is
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
on `16_Track` still leaves the track after a Turbo, and because nothing respawns
an opponent it is gone for the race. The speed target and the lookahead ceiling
(`look_max`, 90 units - a third of a second at 270) are the next places to look.

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

The one gap left is per-team **liveries**: the whole field still wears the
player's hull, so there is one model-space nozzle rather than eight, and every
plume is the player's team's. When per-team models land the locator moves with
the model.

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

Each is separate work, and each is listed so nobody assumes otherwise: a lap
*time* of its own (the standing counts laps; only the player has a clock), a
respawn when it falls off, anything at all happening when it is eliminated, a
weapon aimed at anybody, and a livery that is not the player's. The exhaust left
this list on 2026-08-11 - see [The field burns](#the-field-burns).

**And it does not know the other craft are there.** A personality spreads the
field across the corridor, which is what stopped the single file, but nothing
avoids, overtakes or defends: a craft closing on a slower one holds its own line
straight through it and the two bounce. That is the next piece of this work, and
it is the one the corridor was widened for. **Nor any of the skill work
above** - no mistake injection with a recovery behaviour, no reaction latency,
no difficulty selection, no adaptation, no `[ai]` config block. The personality
covers three of that vector's six axes.
The player's *own* position is on screen now; an opponent's is not -
`PosTag0`-`PosTag7`, the floating name tags, are runtime-anchored to a rival's
projected screen position and nothing computes that.

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
