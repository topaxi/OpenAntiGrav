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
| `Line` | The line to drive, as world-space points, closed. Built by the caller; on a real track that is `pos - HOVER_LIFT * down + racing_line * lateral` per spline sample, so it is **the disc's own line**, and it is index-parallel to the sample table so a craft locates itself once per tick rather than twice. |
| `Tuning` | The controller's constants. **Ours** - none of the disc's own per-class values appear in the tree, per [ADR-0006](../architecture/adr/0006-no-copyrighted-content.md). |
| `Driver` | One `u32`: where the craft last found itself. `Copy`, and it lives on `Ship` inside the world snapshot, so a replay reproduces the drive. |

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
- **The airbrakes are symmetric or off.** A single airbrake yaws the craft toward
  the side it is held, so tying one to the steering command closes a *second*
  feedback loop around a plant that already oscillates. Cornering on the
  airbrakes is real and is deferred. The recovered `<Controller>` has no airbrake
  term either, which is consistent with cornering on steering and a conservative
  speed target - and is **not** evidence for it.

Throttle is bang-bang against `sqrt(lateral_accel / curvature)`, the ordinary
cornering limit, taken over the **sharpest** bend within a braking window rather
than at one point ahead - a corner has to be seen before it is entered.

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

### What an opponent still does not get

Each is separate work, and each is listed so nobody assumes otherwise: speed pads
and weapon pads (the sweep is built around the player's position), a lap counter
of its own (`oag_race::RaceState` is single-ship), a respawn when it falls off,
anything at all happening when it is eliminated, any weapon, and a livery that is
not the player's. **Nor any of the skill
work above** - no skill vector, no mistakes, no difficulty selection, no `[ai]`
config block. Those are the second half.

### What is verified, and where

| Claim | Where | Runs in CI |
| --- | --- | --- |
| A craft driven round an oval tracks its line instead of weaving | `oag-ai`'s `tests/closed_loop.rs`, with the **real force law** on a flat plane | yes |
| The controller steers back toward its line, brakes for a corner, is damped by its own turn rate | `oag-ai`'s own tests, against a synthetic circle and straight | yes |
| Curvature reads `1/radius` on a circle | `line::tests::curvature_approximates_one_over_the_radius` | yes |
| Every opponent is stepped with controls a driver chose | `race::tests::the_opponents_are_driven_rather_than_parked` | yes |
| A grid of eight leaves the line, goes the right way, and is still on the track after ten seconds | `race_ground_truth::the_ai_drives_the_field_along_the_track` | **no** - needs a disc image |
| Two runs of one race stay identical, drivers included | `race_ground_truth::a_driven_field_replays_identically` | **no** - needs a disc image |

`race::tests`' craft is built on `Handling::ZERO` with an empty collision world,
so no force law runs there at all and those tests assert only that the controls
reached the physics. `closed_loop.rs` does run the real force law, but on an
infinite flat plane with invented handling - it says the controller is stable
against a plant of the right *shape*, which is the property that was broken.
**Whether the field actually drives a circuit is the ground-truth pair, and
nothing else.**

The wall-to-wall weave was reported from the running game, not caught by a test.
`closed_loop.rs` was then written, watched failing at **84.1 units** of peak
error from the line, and is at **7.3** after the rewrite. That is evidence the
structural fix works; it is not a substitute for looking at the screen.

Nothing anywhere asserts a lap time. There is no measurement of the original's
opponents to compare against, and the speed law is this project's own.

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
