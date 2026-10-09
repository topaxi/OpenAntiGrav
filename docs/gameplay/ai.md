`05_Track` VENOM and FLASH and `14_Track` RAPIER are `CleanLap` again. One row
reads worse than before the airbrake fix, `07_Track` FLASH `Died` to
`Eliminated` (contact 854 to 869), and it is **a window artefact**: both trees
wreck once mid-run and the new one wrecks a second time at tick 17834, 166 ticks
before the 18,000-tick window closes, while a respawn takes 167. `06_Track` and
`09_Track` lap 1-2 s quicker on every class they were `CleanLap` on. `13_Track`
RAPIER and PHANTOM took more contact (1129 to 1555, 641 to 1347), already `Died`
either way; that is not explained by the change, which drops only three samples
above 0.005 on that circuit's driven line.

**Checked for real corners being discounted.** The world-Y flattening would
misread a banked or past-vertical corner as a valley, so every sample the change
took from above 0.01 to under half was listed on all twelve circuits (lone Ace,
VENOM, the samples its driver actually reads). Every one has a chord with a
climb of at least 0.06 (about 3.5 degrees); none is level ground, and the
largest (`02_Track`, 0.034 over 60 samples at a 59-to-37 degree drop;
`06_Track`, 0.028 over a steepening climb) are the foot of a slope. No evidence
of a dropped banked turn, on those twelve lines. It is a world-Y approximation:
a section that banks past vertical would need the line's own frame.

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
| Which side of a fork an opponent takes: a fair coin per craft per fork, `rand() & 0x100`, held to the merge | **recovered** (2026-10-05), same law in Pulse, 2048, HD and Omega - see [branch choice](#branch-choice-at-a-fork) | 84 |
| Weapon selection and firing | **recovered** (2026-08-17); **the fire half is ported** for the forward weapons (2026-10-03, `oag_ai::weapon_ai`). `WeaponAi_DecideFireOrAbsorb` (`0x088518b4`) rolls the authored odds against two five-entry rate tables on every call, and an aimed weapon also needs a craft in its predicted path. The absorb half is not ported. See [weapon-ai.md](../ghidra/functions/psp-pulse-usa/weapon-ai.md#the-fire-half-at-instruction-level-read-2026-10-03) | 85 |
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

**The direction above is questioned by the code (2026-10-08, confidence 70).**
Read in HD's `AI_ComputeOpponentThrust`, which is Pulse's line for line, the
`WhenLeading` term is *added* when the player leads every opponent by more than
the dead band, and the `WhenBehind` term is *subtracted* when every opponent
leads the player. So the larger multiplier eases the field off for a player
who is behind, and the smaller one is the catch-up. It rests on the progress
field growing forward; see
[ps3-hdfury-eu/ai-stats.md](../ghidra/functions/ps3-hdfury-eu/ai-stats.md#the-thrust-law-pulses-with-four-changes).
HD, 2048 and Omega author these rows with Pulse's own values.

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

**The consumer is found, `AI_ComputeOpponentThrust`
(`0x08855904`) - see
[ai-stats.md](../ghidra/functions/psp-pulse-usa/ai-stats.md#the-skillscale-consumer-ai_computeopponentthrust)
- and it confirms the table above rather than moving it.** `SkillScale`'s
`ThrustOffset` is added inside the identical per-tick computation as
`PosBalancing`'s `SpreadDist` and `RubberBanding`'s leading/behind term, so
this project's own refusal to port either still covers `SkillScale`'s raw
number too: it cannot be pulled out of that function without carrying the
player-coupled terms along with it. What *is* now wired, one level up, is
the campaign's own contribution to `AI_ResolveSkillScale`'s **return
value** - a position on the track's `SkillScaleValue` curve
(`oag_tables::track_stats::resolve_skill_scale`), which a campaign cell's
own `skillEasy`/`skill`/`skillHard` places on. `oag_ai::Difficulty::tune_at_scale`
is this project's own (**chosen, not measured**) reading of what that
position should mean for the four axes below - continuous, so a cell's own
authored position is not quantized to the nearest named tier, and bounded
to `Novice..Elite` since the campaign's own three-rung vocabulary never
names a fourth, harder setting either
(`docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md`'s `HARD ≡ ELITE`
finding). Measured on the real disc, same seed, opponents alone on the
circuit: `16_Track`'s `grid0_2_1` best-laps 51.3s at Easy and 43.0s at
Hard - `crates/game/tests/campaign_skill_scale_ground_truth.rs`.

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
   - reaction latency to weapons and traffic - built, see
     [Reaction latency](#reaction-latency-a-driver-takes-time-to-notice).
     **Pads are not in it**: the one pad reaction, an Eliminator opponent with an
     empty slot steering for a weapon pad, passes through with no latency, as a
     laid charge does
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

The speed target is the **smaller of two limits**, taken over the **sharpest**
bend within a braking window rather than at one point ahead - a corner has to be
seen before it is entered.

1. `sqrt(lateral_accel / curvature)`, the ordinary cornering limit: how fast the
   craft can go round before it slides.
2. `max_turn_rate / curvature`, the **kinematic** one, added 2026-09-06. A craft
   at speed `v` on a line of curvature `k` has to yaw at `v * k` to stay on it,
   so a hull that cannot rotate faster than `w` cannot hold that line above
   `w / k` whatever grip it has. It binds above
   `k = max_turn_rate^2 / lateral_accel`, about 0.0125 at the current defaults -
   20 per cent of `07_Track`'s samples and 26 per cent of `06_Track`'s.

**The second was missing and it cost a circuit.** `07_Track`'s tightest arc has
curvature 0.047, radius 21, admitting 33 units/s; the grip term alone said 74 and
the craft arrived at 94. It left the line pointing up to 26 degrees wrong with
slip flat at -8.4 degrees and `grounded` at 1.00 on every tick - so not a slide -
and shed **34-35 shield a lap** grinding down the wall outside it, destroyed on
lap 3 while still banking a clean 49.8s lap. `lateral_accel` cannot reach that
corner from either end: dropping it from 260 to 90 moved the loss from 24.08 to
24.11.

### The estimator has a resolution, and it is `Tuning::curvature_span`

Curvature is measured over a chord triple, and every caller used to pass **half
its own lookahead** as the chord - so the span grew with speed, and at 80
units/s the triple covered ~72 units of track. A corner shorter than that is
averaged with the straights either side of it. `07`'s ~50-unit arc read
**0.0155-0.0186** where its local value is 0.047, understating it 2.5-3x and
putting its own peak thirty samples early, so the yaw limit above was being
applied to a curvature the driver could not see.

`Tuning::curvature_span` caps the chord at **11 units**, measured by
`sweep_curvature_span` in `crates/game/tests/ai_span_sweep.rs`; the field's own
docs carry the table and the criterion. Shield retained is the metric rather
than lap time, because the failure is a craft that laps cleanly *while*
grinding down a wall.

Together the two changes take the twelve-circuit board from **562.04 to 705.68**
shield retained (summed end-of-run, lone Ace, 18,000 ticks) for 1.7s of mean
clean lap, with no circuit losing a clean lap and no respawn added. `07` sheds
28-30 a lap instead of 33-35 and reaches lap 4 instead of being destroyed on lap
3. **The corner is still untakeable at the speed the driver picks** - what is
left is the wall response and the sustained-contact charge, neither of which is
the AI's.

**The span was swept twice, and the first sweep picked a number that broke two
tests.** A cap of 10 is the best solo row inside the geometric band and it turns
`lap_times_ground_truth::every_opponent_that_laps_has_a_lap_time` and
`opponent_weapons_ground_truth::a_field_racing_with_real_pads_does_not_mine_itself_to_death`
red - a single craft wedging in traffic, which twelve *lone*-craft circuits
cannot see and which `just` does not run. The sweep now reports a field board
beside the solo one, and the trap is worth stating on its own: **a green `just`
says nothing about the disc-backed suite.**

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

### The correction converges too slowly across a wide S-bend, and it is `Tuning::look_speed`

`13_Track` lost 88.48 of its 95.00-shield pool across 18,000 ticks to four
recurring wall sites, every lap, on a circuit with no corner its yaw-rate or
grip limits ever bind on - `head_err` stayed 4-20 degrees and `grounded` was
`1.00` throughout, ruling both out the same way they were ruled out on `07`.
Widening the trace 150-200 samples ahead of each site (rather than only
through the contact) showed the same shape at all four: the craft's own
lateral offset from the line swings 25-30 units across a wide S-bend, its own
AI corridor goes negative 23-57 samples *before* wall contact, and the aim
point the driver's own steering was chasing stayed centred and legal
throughout - `racing_line` reads `0.00` at every sampled index in all four
windows, and the craft's actual offset already exceeds the largest offset
`Frame::clamp` would ever let `Driver::drift` request. So this is not a
line-selection fault: it is the *response* that lags, not the target.

**`Tuning::rate_gain` was swept first, on the strength of its own doc comment
naming it the likeliest lever, and a direct probe ruled it out before the
sweep even ran.** Reconstructing `Driver::steering`'s own arithmetic
(`wanted`, `actual`, and the implied `rate_gain * rate_error` before its
`-1..=1` clamp) against the real simulated state through all four widened
windows shows the command is **already at full lock** for roughly half of
every window and for the samples nearest every contact - there is no headroom
left for a larger gain to spend. The sweep confirms it the expensive way:
`rate_gain` at `10.0` and `20.0` both raise `13`'s own end shield, but both
also break the field - `opponent_weapons_ground_truth`'s own floors (mean
0.70, worst 0.45) fail at `10.0` (worst opponent shield `0.00`, a craft
destroyed) and fail harder at `20.0`, which also drops a lapped opponent from
seven of seven to six.

**`Tuning::look_speed` is the lever that was left**, and the mechanism is
geometric rather than a raw gain: pure pursuit's correction is
`curvature = 2 * offset / distance^2`, and `distance` grows with the
lookahead - so a longer lookahead answers the same lateral offset with a
*gentler* request, exactly backwards from what a craft already outside its
corridor needs during a fast transition. Lowered from `0.35` to **`0.30`**,
swept the same way `Tuning::curvature_span` was - a lone Ace, twelve forward
circuits, 18,000 ticks, against the two committed field fixtures so a value
clean on all twelve *lone* circuits cannot hide a craft wedged in traffic.
The full table is in "Tuning sweep tables" below; the summary is
that `13`'s end-of-run shield rises from `6.52` to `39.07`, the twelve-circuit
solo total rises from `705.68` to `863.10`, and the field floors - which fail
outright at `0.22` and `0.20`, two adjacent points further down the same
slope - **improve** over the old default at `0.30` rather than merely holding,
which is why `0.30` was chosen over the lower values that scored higher on
`13` alone.
**Superseded 2026-09-07** for the field columns: after
`oag_physics::pair::overlap`'s correction the field floors no longer separate
`0.22` or `0.20` from `0.30`, so the reason for `0.30` over the lower values is
gone and it stays shipped unmoved. See "Tuning sweep tables".

**`07_Track` is unchanged by this**, at `0.00` end shield in every row swept.
Its per-lap loss falls (28-30 down to 22-25 at `0.30`) but the craft still
ends the run destroyed, exactly as it did before. That is `07`'s own
wall-response question - whether contact damage should charge every tick of a
sustained low-speed graze at all - and it is untouched here: `CONTACT_DAMAGE_SCALE`
is confidence 94, measured live against the running original at `0.035000`
on 25 of 25 calls, and nothing in this section's evidence bears on whether
that scale is right for a *sustained* graze rather than for the discrete
impacts it was measured against. Left open.

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
of its own - `Tuning::trail_saturation`, a deadband that keeps it out of the
small-signal regime, a curvature floor and exit-decay check that keep it off
straights and corner exit, and a check against the craft's own weapon-hit and
collision timers. See "The corner-entry gate was wrong" below for what that
replaced and why.

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

### The corner-entry gate was wrong, and what replaced it

Reported from play: the AI and the autopilot did not appear to use the
airbrakes at all, and clipped tight corners. Measured headlessly - `VENOM`/Ace,
one craft solo, two laps of Talon's Junction (`oag_pulse::race::DEFAULT_TRACK`,
5,336 ticks), a temporary `eprintln!` inside `Driver::drive` - and confirmed:
`driver::pace::trail`'s gate, `speed < target`, is not an overspeed check on
corner exit only, the way its own reasoning above assumed. `target` is a
*modelled* limit from the corner ahead, and a craft can be saturated on the way
**in** - full lock, large turn-rate error - while still measurably below it.
On the corner that cost the most: `command` pinned at `±1.0`, `rate_error`
0.5-0.6 rad/s, for over ten consecutive ticks while speed collapsed from 111 to
39.5 units/s and `target` sat at 160-180 throughout. `speed < target` held on
every one of those ticks, so the differential was zero on all of them - the one
corner on the lap that needed it most.

**The obvious fix does not survive contact with a real track.** Replacing the
condition with `!target.is_finite()` - off only on a genuine straight, where
`corner_target` returns infinity - passed every test in `tests/closed_loop.rs`,
including the stability guard, because that file's `oval_of` fixture builds its
straights from exactly collinear points and so `target` really is infinite
there. **A real racing line's sampled points are never exactly collinear**:
over the full two-lap Talon's Junction trace, `target` was infinite on **zero**
of 5,336 ticks, against 303 on the closed-loop oval in the same length of run.
So the replacement gated out nothing on disc geometry - it was equivalent to
removing the gate outright - and on the real seven-opponent field
(`opponent_weapons_ground_truth::a_field_racing_with_real_pads_does_not_mine_itself_to_death`,
a real minute of racing with the disc's own pads) that ground one opponent's
shield to zero, against a comfortable `full * 0.45` clear with the gate
unmodified. The attempt was reverted in full.

**What actually separates corner entry from corner exit, measured on the same
lap, is not `speed` or `target` at all - it is curvature's own trend.** Two
real saturated stretches, both holding `speed` a similar fraction below
`target` throughout: one flat-to-rising in curvature (still working the
corner), the other falling steadily (the corner opening up on exit, curvature
0.0274 down to 0.0117 over 63 ticks as the craft pulled away). `speed`/`target`
cannot tell these apart; curvature's trend can. `Driver::peak_curvature` tracks
the high-water mark since the line last went straight, resetting only when
curvature drops to a floor small enough to be chord noise rather than a bend -
0.0003-0.0005 on Talon's Junction's own straights, against 0.0056 and up
everywhere the driver was genuinely saturated on a real one. `0.001` sits in
that gap. `trail` now reads exit as curvature having fallen to `0.7` of that
peak, a threshold chosen well inside the measured decline and well outside the
~5e-5 tick-to-tick noise the chord estimate carries mid-corner.

**A second, unrelated failure surfaced by the same instrumentation, run over
the seven-craft field test with the slowdown and stun timers logged
alongside**: a weapon hit **halves a craft's speed in one tick**
(`ShipState::slowdown_timer` arms the tick after), which the pure-pursuit loop
reads as exactly the shape of a genuine corner - `command` and `rate_error`
both saturate correcting the sudden mismatch between heading and velocity - on
track geometry that is nearly straight throughout (curvature ~0.0026-0.0028 the
whole episode, well under any real corner). `oag_physics::forces::evaluate`
already skips lateral grip entirely while the timer runs, so a differential
spent there buys nothing at all; `trail` now gates on `slowdown_timer` and
`stun_timer` (the latter currently unarmed by anything in this codebase - see
`ShipState::stun_timer`'s own doc - gated anyway, for when it is).

**Consequence for the closed-loop suite**: `tests/closed_loop.rs`'s `oval_of`
straights are exactly collinear and so cannot exercise the curvature floor the
way disc geometry does - the same blind spot that let `!target.is_finite()`
through. `tests/closed_loop_real_geometry.rs` adds a straight built with a
small lateral wobble (0.001 units of curvature, inside the measured
0.0003-0.0005 band) so three consecutive points are never exactly collinear,
and asserts the differential still never fires on it.

Net effect on the disc-backed suites: `opponent_weapons_ground_truth` and
`race_ground_truth`'s solo and field benchmarks were re-run against the change
(`just test-data`); see their own test bodies for the current numbers. The AI
determinism reference (`crates/ai/tests/determinism.rs`) moved on all three
rows, `Solo` included this time - unlike the `social` axis entries above, the
differential is reached by a lone craft on the scenario's own corners, so a
change here was always going to show there. `oag_gameplay`'s own reference
moved too, but only mechanically: neither of its scenarios ever calls
`Driver::drive`, so `Driver::peak_curvature` joining the hash is one more
`u32` a tick and no behaviour, the same shape `roll_decided` and `reflex`
moved it for before.

### The gate was right, the magnitude was not - `--race --autopilot` found it

Reported from play, the same day: with the gate above fixed, `--race
--autopilot` (Pulse PSP EU, VENOM, Elite - the CLI's own defaults) still showed
no visible airbrake through Talon's Junction's own U-turn, and the craft "takes
a normal turn" through the one corner the differential most exists for.
Instrumented the exact code path `--autopilot` runs
(`Race::autopilot_controls`, which shares `ship.driver.drive` with every AI
opponent) rather than the synthetic ovals: the gate **was** opening - 19 ticks
of the corner engaged it - but the differential it computed peaked at **7.67**
of the airbrake's `0..=100` range. Barely there against a symmetric brake
already near its own ceiling.

The cause was arithmetic, not logic: `Tuning::trail_saturation` (`0.85`)
implies a turn-rate-error threshold of `0.85 / rate_gain` = `0.17` rad/s at the
defaults, and `Tuning::trail_deadband` sat at `0.15` - `0.02` rad/s of headroom
between "the gate is allowed to open" and "the magnitude computed from it is
still zero". A driver saturated exactly at the gate's own threshold, which real
corners spend most of their time doing rather than pinned at absolute full
lock, computed almost nothing.

Fitted against the same real corner, `Tuning::trail_saturation` swept alongside
wider `trail_deadband`/`trail_gain` values, cross-checked on the real
seven-opponent field test that caught the original gate's own regression:

| `trail_saturation` | engaged ticks | mean magnitude | field energy (mean-of-means) | depleted craft |
| --- | --- | --- | --- | --- |
| `0.85` (old) | 19 | 4.6 | 0.67 | 0 |
| `0.6` | 58 | 23.9 | 0.58 | 3 |
| **`0.7`** (chosen) | **41** | **25.2** | **0.67** | **1** |

`0.6` engages for the longest stretch of the corner and no stronger a
differential than `0.7` - both `trail_deadband` (lowered to `0.05`) and
`trail_gain` (raised to `3.0`) hold the *magnitude* wherever the gate is open,
regardless of which of the three saturation values is doing the gating. What
`0.6` costs, that `0.7` does not, is real: a full seven-opponent field over a
minute of racing with the disc's own weapon pads spends measurably more grip
fighting more of every corner, and the energy floor
`opponent_weapons_ground_truth` guards reads that as three depleted craft
instead of one and a mean-of-means back down near its own `0.5` floor rather
than matching the pre-retune number. `0.7` is the point on this three-value
sweep that keeps more than double the old engagement and the old field-energy
margin at once. Zero respawns through the U-turn at every point swept,
including `0.6` - the corner was never missed on this run at any of the three,
only how much of the field's own grip it cost.

See `Tuning::trail_saturation`'s own doc for the exact constants, and
`crates/ai/tests/determinism.rs`'s history for why all three rows moved a
second time the same day.

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
now is [a view of its rivals](#what-a-driver-can-see-of-the-grid), plus
blocking, yielding, a lift off a craft ahead, mine avoidance, and - since
2026-08-24 - a deliberate shove: see
[being provoked, and shoving](#being-provoked-and-shoving). **Corrected
2026-09-07**: this paragraph read "anything that touches another craft on
purpose" as still missing well after `Driver::ram` landed; it does not touch
`Driver::social`'s own paragraph below, which was already accurate. What
still is not built is a distinct overtaking *line* - a driver does not choose
a different path to pass, only a lean off the one it already drives - and a
ram may only ever target the player, never another AI craft (`ram.rs`'s
`PLAYER_SLOT`, a deliberate gate against an AI-on-AI pile-up, not an
oversight).

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

Draws **17, 18 and 19** are the three roll axes below, appended 2026-09-06 in
that order and fixed there.
`every_built_in_pilot_still_draws_what_it_drew_before_the_roll_axes_were_appended`
(`crates/ai/src/pilot/tests.rs`) is the guard, and it covers all four built-ins
across all fifteen earlier axes rather than one pilot across seven - because
draws eight to sixteen had nothing pinning them, and those are exactly the ones
a mid-order insert would move.

### Our AI barrel-rolls on purpose, and the original's never did

**A deliberate, authorized deviation, not a fidelity gap.** The original reads
the barrel roll's tap history out of the human player's pad block and nothing
else - recovered at confidence 85, three independent legs, on
[input-bindings.md](../ghidra/functions/psp-pulse-usa/input-bindings.md). Its
opponents therefore never roll. Ours do, on a maintainer's directive of
2026-09-06: an `Ace` rolls whenever the energy budget allows, and lower tiers
roll less.

Four gates, outermost first. The first two are recovered and the deviation sits
**on top of** them rather than through them:

| # | Gate | Where | Provenance |
| --- | --- | --- | --- |
| 1 | airborne | `oag_physics::barrel_roll::advance_gesture` | recovered, 88 |
| 2 | `cost < shield` | `oag_physics::barrel_roll::arm` | recovered, 90 |
| 3 | the energy budget | `ShipControls::roll_shield_floor` | **invented** |
| 4 | difficulty propensity | `Difficulty::roll_appetite`/`roll_caution` | **invented** |

Three pilot axes carry it, so a player retunes them in a file rather than in a
rebuild:

| Axis | What it does |
| --- | --- |
| `roll_chance` | whether it commits, **once per airborne window** - a probability, not a rate |
| `roll_floor` | the fraction of the shield pool it will not spend below |
| `roll_airtime` | how many **seconds** a flight has to have lasted first |

Two decisions worth stating, because each is the sort a later reader would
otherwise assume was an oversight:

- **Provocation does not feed it.** The grudge scales what a driver does to
  *other craft*, never what it asks of its own, and a roll is entirely the
  latter - it spends the driver's own shield for the driver's own turbo, aimed
  at nobody. Wiring it would make an angry AI *faster*, which is the failure
  `provocation`'s own cap exists to prevent, and it would close exactly the
  rubber-banding loop [this project refuses to port](#what-we-build-instead):
  overtake an AI, it gets provoked, it rolls, it boosts, it repasses you.
- **The driver asks for the roll directly** rather than synthesising the
  three-tap gesture on its own steering axis. `ShipControls::roll_request` is
  the request, on the same pattern `ShipControls::sideshift` already sets: a
  caller that has decided sets it, and a real pad leaves it empty and arms
  through the gesture. Routing an invented intent back through the recovered
  input path would make the two indistinguishable in a year's time.

`Difficulty` degrades it the way it degrades everything else - `Ace` is the
pilot's value untouched, and `Elite`, `Skilled` and `Novice` take a lower
chance, a *raised* floor and a *longer* minimum airborne time.

#### What it costs on the disc, measured

One craft alone, 18,000 ticks, every forward circuit, each cell **arms /
shield spent** out of a 95-unit pool. `crates/game/tests/ai_roll_ground_truth.rs`,
`OAG_SWEEP=1`. **Re-measured 2026-09-07**, after `Tuning::curvature_span` and
`Tuning::look_speed` moved (see the S-bend section above) - the original
2026-09-06 table is superseded, not merely stale, since both knobs change how
long a craft spends airborne and therefore how often it clears
`roll_airtime` at all:

| Pilot | Novice | Skilled | Elite | Ace |
| --- | --- | --- | --- | --- |
| `aggressive` | 4 / 30.4 | 12 / 91.2 | 18 / 136.8 | 40 / 304.0 |
| `balanced` | 1 / 7.6 | 7 / 53.2 | 6 / 45.6 | 22 / 167.2 |
| `passive` | 0 / 0.0 | 1 / 7.6 | 0 / 0.0 | 5 / 38.0 |
| `shy` | 0 / 0.0 | 0 / 0.0 | 2 / 15.2 | 1 / 7.6 |

Summed over all four pilots, the ordering the maintainer specified holds
cleanly: **5 -> 20 -> 26 -> 68** rolls, novice to ace, monotonic. **Per pilot it
does not always hold**, and the counts explain why rather than a design fault:
`balanced` (7 skilled vs 6 elite), `passive` (1 skilled vs 0 elite) and `shy`
(2 elite vs 1 ace) each show a one-roll inversion in the middle tiers. Every
inversion sits on totals in the low single digits - a once-per-flight coin
toss over twelve circuits is a small sample, and each tier draws its own
`Personality` scalar from the same `Pilot` span per race, so a different
circuit-tier pairing draws a different point in the range even though the
range itself is fixed. `aggressive`, the only character with double-digit
counts at every tier, is monotonic at every step - which is the tell that this
is sampling noise on a rare event and not the propensity axes failing to
degrade.

The lowest any craft finishes on, on a circuit it laps cleanly, stays close to
where it was: **09_Track remains the worst case**, though `aggressive` at Ace
is now less extreme (34.2 left against the old 21.7, `Tuning::look_speed`
having changed the driving under it). `05_Track` and `07_Track` finish on
nothing at every tier *including the ones that armed no rolls at all*, so that
is the circuits' own doing rather than this mechanic's - unchanged from
before. **A separate, pre-existing pathology surfaced in this re-run and is
not this mechanic's either**: `13_Track` at `novice` respawns a lone craft
**221 to 349 times** for every one of the four pilots, finishing the run on
lap 1-3 rather than 4, all with zero rolls armed - a Novice-tuned craft simply
cannot get through `13_Track`'s authored jump at `grip_believed` 0.30. That is
`Difficulty::grip_believed`'s territory, not the roll axes', and is flagged
here rather than chased, since chasing it is outside this pass's scope.
Chased in [a follow-up below](#the-13_track-novice-pathology-chased).

**Two things the original measurement changed, and they still hold.** The
invented `0.20` shield floor that `oag_physics` carried as a bare constant is
still a number no built-in uses, and `shy` still rolls a handful of times
across the disc rather than never, because never is the original's behaviour
and not this one's.

The harness forces the pilot rather than letting the grid draw it, and that is
not a detail: `pilot_for_slot` hands slot 1 the same character on every circuit,
so an unforced lone-craft benchmark measures one of the four and reads as the
field's. **This is also why a lone-craft table alone cannot answer whether the
mechanic fires in a real race** - see the full-grid measurement below.

#### Does it fire on a full grid, not just in isolation?

Everything above isolates one craft. `crates/game/tests/ai_roll_ground_truth.rs`'s
`a_full_grid_of_*_arms_no_fewer_rolls_than_*` pair-and-seed tests and
`an_ace_pilot_rolls_more_than_a_novice_pilot_in_the_same_race` run `09_Track` -
the worst-case circuit above - with nothing switched off: seven opponents,
each drawing its own pilot, weapons and traffic live.

**The airborne gate is the bottleneck, not the propensity axes.** Across all
28 craft-tier combinations measured (seven opponents at each of four
difficulty tiers), every jump on `09_Track` lands in a narrow band -
**0.47 to 0.62 seconds**, one outlier at 5.07s from a respawn-relaunch rather
than a driven jump. `Difficulty::roll_caution` raises `roll_airtime` for
anything below Ace by 1.3x (Elite) to 2.4x (Novice), so a `BALANCED` pilot's
raw 0.45-0.70s threshold becomes 0.59-0.91s at Elite and 1.08-1.68s at
Novice - **above every ordinary jump this circuit produces**. Novice, Skilled
and Elite armed **zero** rolls between them, all 21 craft, across the whole
run; only Ace - whose threshold is the pilot's own, untouched - reached one
craft that armed 4. That is the propensity axes working exactly as specified
(a lower tier needs a longer jump before it will even consider rolling) run
into a circuit that mostly does not offer one: on `09_Track` specifically,
only Ace is structurally reachable at all for most pilots, which is a fact
about this circuit's jumps rather than a bug in the tempering.

**"Ace against the lower tiers" cannot be built as a physically mixed grid** -
`race::Options::difficulty` is race-wide, so `Race::start` derives one shared
`Tuning` (grip, turn allowance, mistakes, reaction) for the whole field; there
is no per-slot difficulty to ask for. The honest substitute
`an_ace_pilot_rolls_more_than_a_novice_pilot_in_the_same_race` runs instead:
`Tuning` fixed at Ace for everybody (so nobody is slow because it cannot
corner), each opponent's *pilot* separately tempered to a named tier
(`BALANCED`, two of each tier). On `09_Track` this measured **zero arms at
every tier**, Ace included - consistent with the uniform-grid finding above,
since `BALANCED`'s own raw airtime (0.45-0.70s) already sits at the edge of
what this circuit's ~0.5-0.6s jumps clear, and 0 vs 0 says nothing about
ordering either way. **This is the airborne-window finding again, not a new
one**: on a circuit whose jumps run short, `BALANCED` and every tier below Ace
compete for a threshold the circuit rarely clears, and small per-race sample
counts (12-14 jumps a race) make an all-zero outcome unremarkable rather than
alarming.

Shield economics on the full grid, `09_Track`, uniform tiers, end-of-run
(**not per-lap** - see the per-lap figures in the section above, a different
unit): 21 of 28 craft finish with more than half their 95-unit pool; the worst
single craft (`elite`, one opponent) is destroyed outright (`0.0`, lap 3 of
4), which is wall contact and lap-time pressure rather than the roll mechanic
- it never armed one. No craft anywhere in the full-grid runs rolled itself
below the 10-shield floor `no_tier_rolls_itself_down_to_nothing` guards.

### The `13_Track` Novice pathology, chased

Flagged and left for later two sections up: a lone Novice craft respawns
**221 to 349 times** on `13_Track` and never gets past lap 1-3 of 4. Followed
up 2026-09-07, one circuit and one tier at a time, with
`crates/game/tests/novice_respawn_ground_truth.rs`.

**It is one location, not many.** Bucketing every respawn's driver-line-index
by 25-sample ranges, all 347 of 347 respawns in a fresh 18,000-tick run fired
with the driver at index 25-49 - inside `13_Track`'s own authored jump
(indices 19-49, from `the_racing_line_has_track_under_it_where_it_is_known_to`
in `race_ground_truth.rs`). A craft respawning 221-349 times scattered across
a circuit would be a driving-quality question; one respawning that many times
in one place is a single failure repeating, and the airborne-window log says
which one: liftoff at index 22, a 0.88s flight that comes down short at index
44, and then every subsequent window is `liftoff 46, landing 46, duration
~0.75s` - the craft is respawned back into the gap, falls straight through it
again, and repeats for the rest of the run. It never reaches index 50, so it
never completes the jump even once.

**It is the jump, confirmed by a grip sweep rather than by reading the
geometry.** Holding every other Novice axis fixed (`turn_allowed` 0.60,
`mistake_rate`, `reaction_ticks`) and sweeping only `lateral_accel`'s scale -
the same one `Difficulty::grip_believed` sets - on `13_Track` alone:

| `grip_believed` | respawns | laps completed | shield, end-of-run |
| --- | --- | --- | --- |
| 0.30 (Novice) | 347 | 1 | 0.00/95.00 |
| 0.40 | 350 | 1 | 0.00/95.00 |
| **0.48 (Skilled)** | **0** | **4** | 0.00/95.00 |
| 0.60 | 0 | 4 | 2.91/95.00 |
| 0.70 (Elite) | 0 | 4 | 0.00/95.00 |
| 0.85 | 0 | 4 | 0.00/95.00 |
| 1.00 (Ace) | 0 | 4 | 0.00/95.00 |

The threshold sits between 0.40 and 0.48, sharply: nothing in between was
swept, but the jump is either uncleared or clean, with no middle ground seen
at the six points measured. Since `turn_allowed` never moved in this sweep,
the failure is a raw-speed one - a Novice-grip craft's approach to this jump
is too slow to clear it, not a steering or braking artifact from the gap's
geometry confusing the pursuit controller.

**The shield column is end-of-run, not per-lap - per-lap shield was not
measured in this pass**, and it qualifies the headline: six of the seven rows
finish the run destroyed. Clearing the jump stops the respawn loop; it does
not mean the rest of the circuit goes unpunished, the same wall-contact
pressure the S-bend section already measured at Ace. And **`1.00` here is not
a real Ace run** - only `lateral_accel`'s scale moved, so `turn_allowed` and
`mistake_rate` are still Novice's (0.60, `MISTAKE_BASE`). That is why this row
reads `0.00` while the S-bend section's real Ace ends `13_Track` at `39.07`:
different tuning under the same grip number, not a contradiction between the
two tables.

**It is not `Tuning::look_speed`.** Built as `Difficulty::Novice.tune()` would
from a `measured` `Tuning` with `look_speed` set to the pre-2026-09-07 value
of 0.35 and to the current 0.30, both give the same failure: 346 respawns at
0.35, 347 at 0.30. That single-respawn difference is a real effect of a
different trajectory in a deterministic sim, not stochastic noise - but
unchanged either way means "against a healthy 0", not "identical", and 346 is
exactly as broken as 347. The pathology predates the S-bend fix and is
untouched by it either direction.

**It is specific to this one tier-circuit pairing**, not a general Novice
weakness. A fresh sweep, four tiers by all twelve circuits, one lone craft
each, respawns per run:

| circuit | novice | skilled | elite | ace |
| --- | --- | --- | --- | --- |
| 16_Track | 0 | 0 | 0 | 0 |
| 03_Track | 0 | 0 | 0 | 0 |
| 02_Track | 0 | 0 | 0 | 0 |
| 10_Track | 0 | 0 | 0 | 0 |
| 05_Track | 1 | 2 | 3 | 0 |
| 04_Track | 0 | 0 | 0 | 0 |
| 09_Track | 0 | 0 | 0 | 0 |
| 14_Track | 0 | 0 | 0 | 0 |
| 01_Track | 2 | 7 | 1 | 1 |
| **13_Track** | **347** | 0 | 0 | 0 |
| 06_Track | 1 | 0 | 0 | 0 |
| 07_Track | 0 | 0 | 0 | 0 |

Every other cell in the table is single digits - `01_Track`'s own known
respawns (the regression gate's own index-794 one is the `ace` cell here) and
`05_Track`/`06_Track`'s occasional ones, none of them new. `13_Track` at
`novice` is a three-order-of-magnitude outlier sitting alone in an otherwise
unremarkable table, which is what "one authored jump, one tier too slow to
clear it" looks like from above.

**The pilot is unforced in this table** - `run()` never calls
`set_ai_pilot`, so each cell is whichever character `pilot_for_slot` deals
slot 1, the same trap `ai_roll_ground_truth.rs`'s own docs name ("an unforced
lone-craft benchmark measures one of the four and reads as the field's"). It
does not change the conclusion here: the barrel-roll pass that first flagged
this already saw 221-349 respawns on `13_Track` at Novice across *all four*
forced pilots, and `grip_believed` is a shared `Tuning` axis no pilot axis
touches - but a single-pilot table is a narrower claim than "every character",
and it is recorded as one.

**No axis was moved.** Raising `Difficulty::grip_believed(Novice)` from 0.30
to something past 0.48 would clear this jump, but that constant is not free
to spend on one circuit: it is the same one calibrated against the 2026-08-11
play report ("my craft is faster than the AI, first place within a few
seconds"), and 0.30 *is* that report's own number - see
[Difficulty](#difficulty). Moving it to Skilled's value would not be a Novice
fix, it would be deleting the Novice tier's defining trait everywhere else on
the disc for the sake of one circuit's one jump - a number-justified change
serving one row of a twelve-row table, exactly what this project's own rule
warns against. Nothing here justifies it *mechanistically* either: the sweep
shows a threshold, not a reason a threshold that low is wrong in general.

**A separate mechanism question, outside `oag-ai` and left for whoever owns
`crates/raceplay/src/` - measured in part, inferred for the rest, and
labelled accordingly.** `Race::respawn` zeroes velocity and teleports the
craft back onto the racing line nearest where it was lost - which, for a
craft that falls straight through an authored gap, reads as a sample *inside*
the gap. From there, with no speed, it falls straight through again. That
part is read off `crates/raceplay/src/respawn.rs`'s own doc comments, not
observed directly.

What **is** measured: 18,000 ticks over 347 respawns average **51.9 ticks per
respawn cycle**. `RESPAWN_COOLDOWN_TICKS` is 30, so on that average roughly 22
ticks of every cycle run with the cooldown already expired. `respawn()`
zeroes `lost_ticks` on every teleport and `RESCUE_TICKS` (the dwell
`lost_off_the_circuit` requires before it fires) is 90 - nearly double the
51.9-tick average cycle - so the "lost" trigger cannot mathematically
complete its own dwell inside one average cycle here; whatever ends each
cycle, it is something other than that dwell timing out.

The rest is read off the source and **not observed in this pass**:
`respawns_in_a_row` and `respawn_disabled` are private to `Race`, and nothing
in this crate's own tests reaches them without leaving the `oag-ai` lane.
Reading `field.rs`'s tick loop, `respawns_in_a_row` is zeroed the moment the
cooldown reaches 0 and none of `lost_off_the_circuit`/`stalled`/
`reset_zone_touched` are true *that tick* - and since the measured 51.9-tick
average cycle is barely past the 30-tick cooldown when the craft is still
airborne (the 0.75s/45-tick airborne windows are a different interval, timed
from loss of contact rather than from the respawn itself, but consistent with
the fall still being in progress once the cooldown has expired), the zeroing
condition plausibly holds every cycle, which would mean `RESPAWN_GIVE_UP` (5
in a row) never accumulates and every respawn reads as a fresh, non-consecutive
one. **This is a hypothesis from reading the code, not a measurement** - chosen,
not measured, no confidence score - and whoever owns that file should confirm
it before treating it as fact. Fixing the *symptom's cost* (347 down to
something like `01_Track`'s single-digit norm, if the hypothesis holds) is a
`crates/raceplay/src/` question, not this crate's; fixing the *underlying*
jump-clearing failure, if it is worth fixing at all rather than accepted as
"Novice does not clear this jump", is a question for whoever owns that call,
since it trades against the grip calibration above.

### The jump-clearing failure: the mechanism, a real bug that turned out not to be it, and why this is where the chase stops

Followed up further, same day, with `crates/game/tests/novice_jump_ground_truth.rs`.
Three things came out of it: what `grip_believed`'s cliff is actually made of,
a genuine bug the chase turned up that does **not** explain it (measured, not
assumed - see below), and, once both were in hand, a precise answer to
whether anything in this crate can close the remaining gap without moving
`grip_believed`. It cannot, and this section says why rather than leaving that
implicit.

**`grip_believed` is a correlated lever, not a direct one - it works through a
real corner, not through the gap itself.** `13_Track`'s racing line carries a
genuine bend immediately before liftoff: curvature rises to a peak of `0.0125`
at index 13, six indices before the gap starts at 19. That is well inside
[`corner_target`](../../crates/ai/src/driver/pace.rs)'s ordinary territory, so
`grip_believed` throttles the approach exactly the way it throttles every other
corner on the disc - it has no idea a jump follows. Measured directly (not
inferred from the respawn count): a lone Novice craft's actual forward speed at
liftoff (driver-line index 22),

| `grip_believed` | speed at index 22 (units/s) | speed at index 26, still airborne |
| --- | --- | --- |
| 0.30 (Novice) | 87.85 | 87.60 |
| 0.48 (Skilled) | 108.38 | 107.65 |

a 23 per cent difference that reproduces the grip sweep's cliff without
touching the jump at all. **The cliff itself is not a discontinuous grip
effect** - the corner's own target speed is the same smooth `sqrt` function of
`grip_believed` it is everywhere else. What is discontinuous is the *landing*:
the flight this liftoff speed buys lasts about the same span of time whichever
speed it starts from (0.88 s measured at 0.30), so the horizontal distance
covered is continuous in speed while "lands past the gap" versus "lands inside
it and falls through" is a step function of that distance against the gap's
own fixed length. A continuous cause with a binary consequence is exactly what
a sharp cliff looks like from outside - and it is a genuinely sharp one,
**measured, not estimated**, with a fine sweep at `0.01` steps
(`novice_13_track_grip_fine_sweep`) between the coarse sweep's two bracketing
points:

| `grip_believed` | respawns | laps | liftoff speed | first landing index |
| --- | --- | --- | --- | --- |
| 0.41 | 1 | 1 | 101.01 | 42 |
| 0.42 | 1 | 1 | 103.07 | 45 |
| **0.43** | **0** | **4** | **103.76** | **47** |
| 0.44 | 0 | 4 | 104.66 | 47 |
| 0.45 | 0 | 4 | 106.53 | 47 |
| 0.46 | 0 | 4 | 106.41 | 47 |
| 0.47 | 0 | 4 | 107.31 | 47 |

The threshold sits between **0.42 and 0.43**, not the coarse sweep's `0.40`
and `0.48` - a liftoff-speed gap of under one unit per second (`103.07` versus
`103.76`) decides it. Every clean row lands at the same index, 47: past the
threshold, extra grip buys the craft nothing more on this jump, which is the
first sign of a second limit below - see the ceiling test.

**This is not a reason to raise `grip_believed`.** The corner producing the
87.85-vs-108.38 split is ordinary cornering physics, correctly scaled by the
axis that is calibrated against play - see the sweep two sections up. Moving
it would still be spending a constant that governs every corner on every
circuit to fix one gap on one of them.

**The ceiling test: is the corner even capable of the speed this jump needs,
independent of grip?** `Tuning::max_turn_rate` is scaled by `turn_allowed`
too (`0.6` at Novice), and the coarse sweep held it fixed while only
`lateral_accel` moved - so a sweep that never sees a plateau cannot rule out
the yaw term becoming the binding constraint at some grip past what was
tried. `novice_13_track_grip_ceiling` answers it directly: `lateral_accel`
scaled far past anything the fine sweep needed (`1.0`, `5.0`, `20.0`,
`100.0`, all with `turn_allowed` still Novice's own `0.6`),

| `grip_believed` | liftoff speed | first landing index |
| --- | --- | --- |
| 0.48 | 108.38 | 47 |
| 1.00 | 111.21 | 47 |
| 5.00 | 111.21 | 47 |
| 20.00 | 111.21 | 47 |
| 100.00 | 111.21 | 47 |

Speed plateaus at `111.21` from `1.0` on - the yaw term has taken over, and no
further grip buys another unit of speed through this corner while
`turn_allowed` stays at Novice's `0.6`. But `111.21` clears the jump with
7.5 units/s to spare over the `103.76` the fine sweep found sufficient, so
**the yaw ceiling is not what is stopping Novice.** The corner is
geometrically capable, at Novice's own turn-rate degradation, of every speed
this jump needs. What is missing is entirely the **34-unit gap** between
Novice's calibrated liftoff speed (`87.85`, at `grip_believed = 0.30`) and the
`103.76` the threshold needs - a gap `grip_believed` alone closes, and nothing
else in this corner's physics does.

**A genuine bug turned up in the same pass, found first and ruled out
second - reported in that order because that is the order it was learned
in.** `ShipState::time_airborne` was read nowhere in `Driver::drive`. Proven,
not inferred - a unit test (`a_craft_mid_flight_never_brakes_for_the_corner_ahead`,
written before the fix below) built one fixture at `time_airborne = 0.0` and a
second identical in every other respect at `time_airborne = 0.3`, on a corner
tight enough to brake hard on the ground, and the two produced **byte-identical
controls**. The driver braked mid-flight exactly as hard as it braked on the
track, for a corner it had no lateral grip left to use the brake on -
[`pace::throttle`](../../crates/ai/src/driver/pace.rs)'s own doc says what the
symmetric brake buys: "how much cornering grip the deceleration is bought
with". Off the ground there is no cornering grip to buy, so every tick of that
brake was a pure loss of the forward speed a landing needs, for nothing - in
general. **The fix**: when `state.time_airborne > 0.0`, hold full thrust and
skip the corner-based brake entirely, the same way the existing
mistake-injection branch already does for a driver "sailing through" a missed
braking point. Chosen, not measured, no confidence score - this is the
driver's own behaviour, not a recovered one, and defensible on mechanism
alone: a brake that buys cornering grip is not a sensible command to issue
when there is no cornering grip in play.

**It does not touch `13_Track`'s own pathology, and that is measured rather
than assumed.** Before and after this exact fix, same seed, same tuning,
same 18,000 ticks: `347` respawns both times, the *same* first liftoff
(index 22) and the *same* first landing (index 44, 0.88 s flight). The
symmetric brake this fix removes was never actually engaging during this
particular flight - the look-ahead window during those ticks never reached
a curvature the reduced Novice speed was already under, so there was nothing
for the fix to remove here. The bug is real and the fix is kept (it is a
correctness fix independent of this pathology, and the regression gate below
confirms it costs nothing), but it answers a different question than the one
this section is chasing.

**Tier x circuit table, before and after the airborne fix** (respawns per
run, four tiers by twelve circuits - the coarse grid, pilot unforced, same
caveat as before):

| circuit | before (n/s/e/a) | after (n/s/e/a) |
| --- | --- | --- |
| 16,03,02,10,04,09,14,07_Track | 0/0/0/0 | 0/0/0/0 |
| 05_Track | 1/2/3/0 | 1/**3**/3/0 |
| 01_Track | 2/7/1/1 | **3**/**5**/1/1 |
| 13_Track | 347/0/0/0 | 347/0/0/0 |
| 06_Track | 1/0/0/0 | 1/0/0/0 |

Two single-digit cells moved (`05_Track` skilled `2->3`, `01_Track` novice
`2->3` and skilled `7->5`) - a real behaviour change from removing a bug that
*was* live somewhere on those two circuits, even though it is inert on
`13_Track`. Nothing moved outside the single digits the pre-existing table
already had, and the tier that matters for the regression gate - Ace - is
unchanged on every circuit, `01_Track` included.

**The regression gate,
`race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`,
before and after, quoted verbatim:** all twelve circuits clean lap both times;
`01_Track`'s one respawn stays at index **794** both times; `13_Track` clean
laps at `37.5s` both times (`0` respawns for the Ace tier this gate runs, same
as always - the pathology is Novice-only).

### Why the chase stops here: no lever left in `oag-ai` that is not `grip_believed` itself

Three things are now measured, not assumed: the mechanism (a real corner,
ordinary physics), the precise threshold (`grip_believed` between `0.42` and
`0.43`, not the coarse `0.40`/`0.48`), and that the corner is geometrically
capable of the needed speed at Novice's own turn-rate degradation (the
ceiling test, `111.21` against a `103.76` requirement). What that leaves is a
plain gap: Novice's calibrated liftoff speed through this corner is `87.85`;
clearing the jump needs `103.76`. Nothing between those two numbers is a
controller bug - the corner is driven correctly, just slower than this one
jump wants, because that is what `grip_believed(Novice) = 0.30` is *for*.

**A jump-aware rule was considered and not built, and the reason is
concrete rather than a preference.** [Question 4 of the brief](#the-13_track-novice-pathology-chased)
asks whether a driver that "knows it is approaching an authored jump" would
be defensible. It would be, in principle - but `oag_ai::Line` carries no
signal to build it from. Checked directly, not assumed: the racing line's
own height barely changes across the whole gap (`20.36` to `20.64` world
units over indices 19 to 49, out of a full circuit that climbs and descends
far more than that everywhere else) and the corridor stays a normal width the
entire way across (`left`/`right` within a unit of the samples either side).
There is nothing in the geometry `oag-ai` can see that marks this stretch as
a jump rather than an ordinary straight, and the collision-based test that
*does* know (`the_racing_line_has_track_under_it_where_it_is_known_to`) lives
outside this crate's dependency graph by design - `oag-ai` depends on
`oag-core` and `oag-physics` and nothing that can run a raycast against
loaded track collision. Wiring that signal through would mean adding a
producer in `crates/raceplay/src/spline.rs`, which is this pass's another
lane, not this crate's. A `Line` field with nothing to populate it is a dead
API, not a fix.

**So: accepted as "Novice does not clear this jump", per the brief's own
allowance for that outcome.** `Difficulty::grip_believed` is untouched at
every level - Novice keeps every other circuit's identity exactly as
calibrated. The `crates/raceplay/src/` respawn-cooldown hypothesis flagged
above is also untouched - it was never load-bearing for either finding here,
and whoever owns that file should still confirm or refute it on its own
terms; fixing it would very likely take `347` down toward the single-digit
norm every other cell in the table already shows, without touching why the
first landing falls short.

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

#### The clearance gate asked the wrong question, and the wrong number

**Reported from play 2026-08-24 as "the AI steers into me and puts its nose in
the wall".** The mechanism was already right - a ram has been a
`ShipControls::sideshift` since it landed, never a steering command, and nothing
in this controller ever aims a craft at another one. What was wrong was the gate
in front of it, in two ways at once.

**It measured the room from the line rather than from the craft.**
`Frame::room` answers "how far may the *line* go this way", and a craft is
hardly ever on its line - `drift` puts it off, a corner puts it off, the last
shove it took puts it off. Measured on `16_Track` before the fix: a shove fired
with 3.13 units of corridor to its left while the craft was already 11.67 units
past that edge.

**And it asked for three units, when one shift covers about eight.** A sideshift
is `handling.airbrake.sideshift` as a force for
`oag_physics::airbrake::SIDESHIFT_DURATION`, and on Pulse that is `450` against
a mass of `1` - the most violent lateral event in the game. Measured over six
seeded elite races on `16_Track`, a craft that throws one reaches a **median 8.2
units** across the corridor before its own grip and steering claw it back, p90
12-15, with a tail past 20 when the shove connects and the contact throws it
further.

How often the rammer ended up outside the corridor edge it shifted toward (all
four rows measured before the ram was narrowed to the player, below):

| room measured from | clearance | shifts | ended outside |
| --- | --- | --- | --- |
| the line | 3.0 | 208 | 54 (26%) |
| the line | 12.0 | 123 | 25 (20%) |
| the craft | 3.0 | 223 | 56 (25%) |
| **the craft** | **12.0** | **138** | **11 (8%)** |

**Neither half does much alone**, which is why both landed together: at three
units, where the room is measured from hardly ever changes the verdict, and
asking twelve of the *line* still lets a craft already ten units out shift
further out. Past 12 the sweep keeps improving - 16 gives 4 of 72, 20 gives 0 of
20 - but by making a ram rare rather than by making it safe.

#### And a ram may only target the player, because a clump of AI spirals

**Reported from play the same day**, once shoves started landing: opponents
shoving *each other* in a clump does not settle. The shove provokes, and
`Driver::stew` turns provocation into appetite for the next one, so three or
four craft running together feed each other until the group is off the racing
line - a positive feedback loop between `stew` and `ram` with nothing damping
it. From outside it reads as a pile-up that will not stop rather than as racing.

`ram` now returns `None` unless `Field::alongside` names **slot zero**, the same
player-slot convention `Driver::for_slot` already runs on. Two other dampers
were available and both are worse. A cooldown longer than the physics'
`SIDESHIFT_LOCKOUT` would be a second source of truth beside a timer already in
the snapshot, which is exactly what gating on `shift_lockout` exists to avoid. A
"not while several rivals are near" gate needs a count `Field` does not carry,
and would let the loop run right up to its own threshold. Aggression toward the
player is the part that was wanted, and `Field::alongside` already carries the
slot, so it costs nothing.

**The cost, recorded rather than hidden**: the field no longer shoves itself, so
an opponent battle mid-pack is quieter than it was, and a ram fires about a
fifth as often - 27 shifts across six three-minute races against 138. Wire it
back the day something damps the loop. The honest damper is the victim's
provocation being *spent* by the contact rather than only added to, and
`Race::resolve_craft_pairs` already computes the contact that would do it.

**Two remainders, recorded rather than chased.** The gate reads the corridor at
the craft's *current* index, and the shift plays out over a second in which the
craft covers a hundred units downtrack into a corridor that may have narrowed.
And a shove that connects hands the rammer whatever `oag_physics::pair::resolve`
gives it, which is not the driver's to gate - those are most of the 8% that
remain. Also: `RAM_CLEARANCE` is measured against **one hull**. The reach scales
with `handling.airbrake.sideshift / mass`, a physics quantity `oag-ai` cannot
see - `Context` carries a line, a tuning, a pilot and a field, and no handling.

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

**2026-09-29: the driver reads a bend over one of these runs as straight.**
Chosen, not measured (maintainer decision: opponents obey the player's physics
and drive smarter). `oag_ai::Line::with_unsupported` carries the same cast as a
per-sample mask, and `Line::curvature` returns zero for a chord touching a masked
sample: over a gap the craft is flying, not steering. It was `01_Track`'s lip
(samples 31-42) that asked for it - read in 3D its 60-degree pitch was a
0.075 rad/unit corner and the Ace crawled off it at 20 u/s every lap, where the
original's field flies it at 69-111. See
[leaving-the-track.md](leaving-the-track.md#what-landed-2026-09-29-branch-sunk-craft-2).

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
guarantees a rear probe in contact**, so the craft cannot shed half its
suspension because one probe overran a lip. This crate cast two independent rays
at every speed and did exactly that; the symptom is visible in a hand-driven
capture as `grounded 1.0` then `0.5` then `0.0` over two seconds.

**The guarantee is one-directional, and this paragraph used to overstate it.**
It said `grounded` "cannot read `0.5`" at speed. It can, and the original does:
all **23** of the `0.5` ticks in `data/traces/talons-junction-clean-lap.csv` are
above the threshold, with `speed_cached` - which *is* `craft+0x2ec`, the branch's
own input - between **76.4 and 111.6**. The copy only runs when the front ray
*hits*; when it misses, the fast path falls through to casting the rear for real,
which is what `oag_physics::hover::probe_pair` does and what the original is
doing on every one of those 23 ticks (front miss, rear hit). Measured by
`crates/trace/tests/hover_contact_ground_truth.rs`, which reproduces the
recording's whole `grounded` column - 2,976 of 2,976 ticks, `0.5` runs included -
by asking our probes about the original's own poses. The branch, the flag copy
and the `6.0` are unchanged; only the sentence was wrong.

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

That single-track, single-class regression is now a board-wide one:
`crates/game/tests/ai_clean_lap_gate.rs` freezes all 12 circuits x 4 speed
classes as 48 individual tests, each checking status (clean lap / no clean
lap / eliminated), clean-lap time and wall-contact ticks against a committed
baseline. It runs under `just test-data`, not `just` - see the file's own
doc comment for what a green board does and does not prove, and for why a
row can be "clean" while still grinding a wall every lap.

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

**Weapon pads in an Eliminator are the exception, and they ship (2026-10-03).**
There a pickup is worth kills rather than lap time, and an opponent with an empty
slot waits a median 9.7 s for its next one on `16_Track`. Steering an empty-slot
craft at the next weapon pad cut the median time to five kills from 117 s to
108 s over 240 seeds. The channel is `Field::pad`, gated on the slot being empty
and the mode being Eliminator, so no other race moves. Chosen, not measured: see
[race-modes.md](race-modes.md#steering-for-weapon-pads-2026-10-03).

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

`crates/raceplay/src/pilots.rs` owns it, and **`oag-ai` gains nothing** - the loader
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

**Since 2026-10-03 this is the fallback.** Wherever the title's
`WeaponAIstats.xml` is read (every title: Pulse, Pure, HD, Omega and 2048), an opponent fires its forward
weapons on the original's own law, `oag_ai::weapon_ai` - see
[race-modes.md](race-modes.md#firing-on-the-originals-law-2026-10-03). What
follows is the rule a race without that file still runs - the sweeps, through
`set_fire_law` - and its history.

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

**What an opponent does with what it draws is a policy, and the original's fire
half is ported for the forward weapons since 2026-10-03** (`oag_ai::weapon_ai`,
[race-modes.md](race-modes.md#firing-on-the-originals-law-2026-10-03)); what follows
is the history and the rules that are still ours. `Data\XML\WeaponAIstats.xml` holds three values a weapon
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
leaving in the first place: a recovered lap is not a lap it drove.

**`look_max` is ruled out, measured rather than reasoned.** The named suspect
was the lookahead ceiling (90 units - a third of a second at 270) capping the
ordinary steering/braking loop's horizon after a boost, separately from the
`allows_speed` gate's own unclamped one. `crates/game/examples/turbo_lookahead_sweep.rs`
tracks, for every Turbo an opponent fires, how far it strays from the spline
over the boost plus a short coast, at `look_max` 90 (today's value), 100, 110,
120 and 200 (the value past which the clamp cannot bind at any speed measured
on this circuit - see below). 32 seeds, seven opponents, a minute each -
224 craft-minutes, eight times the original measurement:

| look_max | Turbos fired | mean excursion | worst excursion | escapes |
| --- | --- | --- | --- | --- |
| 90 | 104 | 14.5 | 47.4 | 0 |
| 100 | 97 | 13.6 | 28.9 | 0 |
| 110 | 99 | 15.1 | 43.5 | 0 |
| 120 | 101 | 15.2 | 47.1 | 0 |
| 200 | 100 | 15.3 | 47.1 | 0 |

Flat across every setting, and **zero escapes at any of them** against a
456-unit rescue threshold on this circuit - the worst excursion measured,
47.4, is nowhere near it. Two things follow. First, `look_max` cannot be the
lever: raising it from 90 to 200 changes nothing about how far a boosted
craft strays. Second, the specific failure this section opened with did not
reproduce once, in a sample eight times the size of the one that found it
originally - the opponent respawn (reverted 2026-10-02: a wrecked opponent
stays down, which only removes a craft the gap was measured on) and the stall
rescue, both landed after this measurement was first taken, most plausibly
already cover what `look_max` was suspected of; only the stall rescue still does. The residual gap this section describes is not shown to
exist any more on `16_Track`; it has not been re-measured on other circuits,
and nothing here rules out a rarer event this sample size still missed.

Confirmed separately, from the same sweep: the arithmetic
(`look_min + look_speed * speed > look_max`) says the clamp can only bind
above 200 u/s at `look_max` 90, and the sweep's own max observed forward
speed - 321-322 u/s, consistent across every setting - means it *does*
bind occasionally even at `look_max` 120. `look_max` is exclusively a
boost-phase parameter: no craft in this sample ever drove fast enough
outside a Turbo to feel it.

**Widened to two more circuits, 2026-09-07 - the negative result above was
one circuit only, and the thread said so.** `turbo_lookahead_sweep` now also
sweeps `07_Track` and `09_Track`, 32 seeds each, at the shipped `look_max`
(90) only - the block above already settled whether `look_max` itself is the
lever, so this does not re-sweep it. Both are chosen, not drawn at random:
`07_Track` carries the disc's tightest measured arc (curvature 0.047, radius
21, admitting 33 units/s, where an *unboosted* craft already overspeeds it at
94 - see "The second was missing and it cost a circuit" above), and `09_Track`
is independently flagged twice elsewhere on this page as the worst case for
**end-of-run** shield. `13_Track` is deliberately
excluded despite being on the twelve-circuit list: its own authored-jump
pathology (see
["The `13_Track` Novice pathology, chased"](#the-13_track-novice-pathology-chased))
would confound a Turbo-excursion count with an unrelated failure.

| circuit | Turbos fired | mean excursion | worst excursion | rescue threshold | escapes |
| --- | --- | --- | --- | --- | --- |
| 16_Track | 55 | 13.6 | 36.3 | 456 | 0 |
| 07_Track | 28 | 12.0 | 42.4 | 393 | 0 |
| 09_Track | 51 | 11.1 | 22.1 | 414 | 0 |

**Zero escapes on all three**, including `07_Track`'s own tightest corner.
The escape this section opened with still does not reproduce - not just on
`16_Track`, but on the circuit picked specifically because it already
overspeeds a corner unboosted. Each circuit is 32 seeds times seven
opponents times a minute, the same 224 craft-minutes the original `16_Track`
sweep measured, so the two new circuits add **448 craft-minutes and 79
Turbos** (28 on `07_Track`, 51 on `09_Track`) on top of it - 672
craft-minutes and 134 Turbos total across all three, same result throughout:
the residual gap this section describes is not shown to exist any more.

One number moved between the two sweeps and is worth flagging rather than
silently reusing: this session's own re-run of `16_Track` at `look_max` 90
fired 55 Turbos (mean 13.6, worst 36.3, max speed 313.6) against the
2026-09-02 sweep's 104 (mean 14.5, worst 47.4, max speed 321-322) - fewer
than half. That is not a change made in this session; `crates/ai` moved
underneath the harness between the two runs (`look_speed` was lowered on
2026-09-06 to fix `13_Track`'s convergence lag, among other changes), which
plausibly changed how the driving reaches the Turbo trigger at all. It is
not a regression in what this table is checking: `race_ground_truth::
a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round` still passes
all twelve clean with `01_Track` at its usual single respawn (index 794),
and no committed hash moved - this sweep only reads.

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
- **The shared vertex buffers had to grow.** `oag_fx::exhaust` uploads every
  flare and every ribbon through one pipeline, and `Pipeline::upload` clamps with
  `min` - so an undersized buffer would have dropped the last craft's ribbon with
  nothing in the logs. `MAX_TRAILS` is the grid, `MAX_TRAIL_VERTICES` is eight
  ribbons, and `MAX_SPRITES` now carries its own arithmetic: eight flares plus a
  projectile and a blast flash per projectile slot, 40 of 48.

**Liveries landed 2026-08-15** (`crates/livery/src/lib.rs`), and with them
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

What is still missing, and it is now a short list:
**adaptation between races**, which was blocked on per-opponent lap times and is
not any more. ~~And **anything at all happening when a craft is eliminated**~~ -
**it came back from 2026-09-16 to 2026-10-02, which was wrong**: state 5's `1.5` s and state 6's `0.8` s, then a
full pool and a place on the line. Measured live on 2026-10-02, the original's wrecked opponent stays in state 6 for good
([shield.md](../ghidra/functions/psp-pulse-usa/shield.md#state-6-measured-on-ppsspp-a-wrecked-ai-craft-stays-down-2026-10-02-pulse-state6)),
and `Race::tick_destroyed_craft` now leaves it down outside the Eliminator. The
player's *own* position is on screen; an opponent's is not - `PosTag0`-`PosTag7`
are eight fixed-position rows meant to list the field, not the runtime-anchored
per-craft tags an earlier reading of this line took them for (corrected
2026-09-08, see [hud.md](../ui/hud.md)). The anchor is not the gap: what each
row is meant to say is - no `idstring`, no `string`, content unread.

**What left this list, with what to look at instead of re-deriving it:**

| Left | When | Where it lives |
| --- | --- | --- |
| An exhaust of its own | 2026-08-11 | [The field burns](#the-field-burns) |
| A respawn when it falls off | 2026-08-12 | `Race::lost_off_the_circuit` |
| A weapon aimed at somebody | 2026-08-12 | `Driver::wants_to_fire`; since 2026-10-03 the original's law, `oag_ai::weapon_ai`, with the former as the fallback |
| Knowing the other craft are there | 2026-08-12 | `oag_ai::Field`, and the `courtesy`/`defence`/`caution` axes |
| Mistake injection, with a recovery behaviour | 2026-08-12 | `Driver::blunder`, `Driver::mistake`; the rate comes from `Difficulty::tune`, **not** from a config key |
| Difficulty selection | 2026-08-12 | `oag_ai::Difficulty`, and one `[ai] difficulty` key in `settings.rs` |
| A livery that is not the player's | 2026-08-15 | `crates/livery/src/lib.rs` |
| **A lap time of its own** | **2026-08-17** | `oag_race::Standing::best_lap_ticks` |
| **Being recovered when it stops** | **2026-08-17** | `Race::stalled`, `race::STALL_SPEED` |
| **Reaction latency** | **2026-08-26** | [Reaction latency](#reaction-latency-a-driver-takes-time-to-notice) |
| **Coming back from a wreck in a single race** | **2026-09-16** | `Race::tick_destroyed_craft`, `race::eliminator::AI_RESPAWN_WAIT` |

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

**Corrected 2026-08-25: `0x08815ccc` was mislabelled, and the "nothing to
recover" reading it supported does not hold.** `0x08815ccc` itself is still
`jr ra; nop` - it reports nothing, ever - but it is the *mesh*-against-mesh
narrowphase, not the box-against-box one a pair of craft dispatches to; craft,
being box colliders, never reach it. What a pair of craft actually dispatches
to is `Collision_BoxAgainstBox` (`0x0881702c`), which is **not** a stub: a
decompiled fifteen-axis oriented-box SAT that writes a real contact on
overlap. Whether it ever fires for two craft during a live race depends on two
gates (`world+0x5464`, each collider's own `+0x68` byte) that are still
unconfirmed - see
[contact-response.md](../ghidra/functions/psp-pulse-usa/contact-response.md#0x08815ccc-is-a-stub-but-it-is-not-what-a-pair-of-craft-dispatches-to).
`pair::overlap` tests an oriented box against an oriented box over the hull's
own `<Misc width height length>`, which is what the shape kind says a craft
is - ours, provisionally, until that live check settles whether it is
approximating a real narrowphase or standing in for one that never runs.

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

### Reaction latency: a driver takes time to notice

Landed 2026-08-26, and it is the fourth of the six skill axes above - the last
of them that decides how a driver treats the craft around it.

**Nothing on the disc authors this and there is nothing to recover.** The
original's opponents are a thrust schedule keyed on the player's race position
and the gap to them, which this project [refuses to
port](#what-we-build-instead); the whole controller here is ours. So the number
to justify is a human's, not a hull's.

**What it does.** Until this existed a driver acted on `oag_ai::Field` the tick
the caller measured it: a rival that came alongside was covered on the frame it
got there, and one that appeared in the weapon cone was shot at on the same
frame. That is not a quick driver, it is one with no perception step at all,
and it is a large part of what makes a *slow* opponent still feel machine-like.
Each of the field's three channels - ahead, behind, alongside - is now held
back for `Tuning::reaction_ticks` after its occupant **changes**, and reads as
empty until then.

Three decisions inside that, each of which could have gone the other way:

- **The driver is told nothing, not something stale.** Holding the previous
  answer would mean carrying a copy of the field per driver - `f32`s in the
  world snapshot that would cost `Driver` its `Eq` - and it models the wrong
  thing anyway. Not having noticed is an absence, not a memory.
- **A channel *emptying* is acknowledged at once.** A driver still covering a
  rival that has dropped away reads as a bug; one late to spot an arrival reads
  as a driver. The asymmetry is deliberate and is the one place the axis is not
  symmetric in time.
- **A race place is never held back.** `Field::place` feeds the grudge in
  `Driver::stew`, and being annoyed about an overtake is a reaction to having
  *been* passed rather than a reaction time. The standings are not something
  seen out of a cockpit.

**The numbers**, in ticks at 60 Hz - a simple visual reaction is about a quarter
of a second, which is fifteen:

| level | ticks | seconds |
| --- | --- | --- |
| Novice | 24 | 0.40 |
| Skilled | 15 | 0.25 |
| Elite | 6 | 0.10 |
| **Ace** | **0** | **0** |

Zero at Ace for the same reason `Difficulty::mistakes` is zero there, and
**zero in `Tuning::default`**, so a caller that has not chosen a difficulty -
the closed-loop harness, a replay, `oag-trace` - still gets the competent
driver it always got.

**Measured, on the five-seed harness** in
`crates/game/tests/difficulty_ground_truth.rs`, leader distance averaged over
five seeds, before and after:

| level | before | after | change |
| --- | --- | --- | --- |
| Novice | 6,444 | 6,338 | -106 |
| Skilled | 7,000 | 7,058 | +58 |
| Elite | 7,502 | 7,509 | +7 |
| **Ace** | **7,614** | **7,614** | **0** |

**Ace is identical seed by seed**, not merely on the mean - `[7474, 7698, 7656,
7459, 7784]` both runs - which is the one strong result in the table: a latency
of zero has to change nothing, and it changes nothing.

**The other three rows are not measurements of this axis, and the Novice one
is the trap.** Its five per-seed deltas are `-352, -11, -283, -90, +208` -
mixed in sign, on a harness whose own history records adjacent levels sitting
44 apart on a five-seed set. A mean of -106 against a spread that wide is not a
slow-down that has been demonstrated.

**And the mechanism says leader distance is the wrong witness here anyway.**
`Difficulty::temper` scales `trigger`, `ram` and `defence` by `aggression()`,
which is **zero at Novice** - so at the level where the latency is longest, the
only behaviours it can withhold are `caution` (a lift for a craft closing
ahead) and `courtesy` (a yield for one coming up behind), and both of those
*cost* distance. Withholding them for four tenths of a second makes a Novice
hold its throttle and its line a little longer, not less. What the axis buys at
that level is how the craft reads from the cockpit - one that tailgates and
takes a moment to move over - and distance round a lap does not see it.

What the ground-truth run establishes, then, is that the ordering still holds
and Ace is untouched. That the axis reaches the controls at all is
`driver::tests::reaction_tests`, which is where it is actually asserted.

**Where it lives:** `oag_ai::Reflex` on `Driver`, three arrays of integers, so
the snapshot stays `Copy` and `Eq` and a race replays. `Driver::drive` advances
the clock and `Driver::wants_to_fire` reads it, in that order within a tick -
which is a real coupling: a caller that asks whether a driver wants to fire
without having driven it that tick is asking a driver that has noticed nothing.
`Race::step_opponents` drives every opponent a few lines before it spends its
pickup, so in a race the two are the same tick.

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
| A ram goes toward the craft alongside, waits for the physics' own lockout, and never goes toward a corridor edge it has no room for | seven tests in `driver::tests::ramming_tests` | yes |
| **And it goes at the player and never at another opponent** | `ramming_tests::a_ram_goes_at_the_player_and_never_at_another_opponent`, over all seven opponent slots | yes |
| **And the room it measures is the craft's own, not the line's** | `ramming_tests::a_ram_measures_its_room_from_the_craft_and_not_from_the_line` | yes |
| The same gate holds on real geometry, and a ram rarely throws the rammer out of the corridor | `ram_ground_truth`, two tests | **no** - needs a disc image. **Run and passing 2026-08-24**: 27 shifts over six races, none under the clearance, 2 ending outside. |
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

## The clean-Ace board: every circuit, every speed class, wall contact counted

Added 2026-09-12. Until then **every solo measurement this project had ever
taken was at `VENOM`**, and nothing in the harness measured wall contact at all
- `Solo` carried best lap, respawns, laps and `lost_at`, and no shield. The
standard the board is written against is the user's own: an Ace should lap
every circuit, in every class, alone, without touching a wall.

The harness is `crates/game/tests/ai_clean_lap_board.rs` - `#[ignore]`d,
`OAG_SWEEP`-gated, printing rather than asserting, for the reason `sweep_grip`
and `sweep_curvature_span` both give. Forty-eight rows of five simulated
minutes is 42 s of wall clock in release.

### What it counts, and why not the obvious thing

**Contacts, not inbound impacts.** `oag_physics::ShipState::wall_contact_prev`
is `WallResponse::impact`, an inbound test (`normal_speed < 0.0`), and a craft
grinding along a wall stops being inbound long before it stops being in
contact. Measured on `07_Track` at VENOM: **1,445 contact ticks against 1,378
inbound**, so the inbound reading undercounts by 5 %.

**Charged damage, not a shield delta.** The pool also moves for barrel-roll
charge and shield pads, which is exactly why `07_Track` was for a long time the
only circuit whose wall attrition could be read by hand at all. `RaceSim`
sums `oag_physics::damage::contact_damage(impulse_sum, rules)` - the quantity
physics itself charges - so the wall's share is separated structurally rather
than by subtraction. It is what the wall *charged*: a craft at zero shield is
charged the same and loses nothing, so it is always reported beside the
end-of-run pool and never instead of it.

**A contact counted is a wall by construction.** `oag_physics::wall::responds`
excludes `Floor` and `MagFloor`, so a `WallResponse` contact can only be
`Surface::Wall` or `Surface::Reset`. That is a surface-tag test and strictly
stronger than the `|n.up|` near-zero geometric proxy a by-hand reading needs.

**The counters are gated on the craft still racing.** `step_opponents`
*releases* a wrecked opponent rather than skipping it, so `oag_physics::step`
keeps integrating a hull settled against a wall, at a frozen `driver.index`.
Ungated, `10_Track` at PHANTOM read 7,157 contact ticks against 328 after a
change that *halved* what the walls charged it, and `07_Track` charged 122.05
against a pool of 95.00 - both the wreck, not the driving. `racing_ticks_of` is
the denominator, and `charged` saturates at the pool on a dead row, so it
discriminates between living rows only.

`Mode::SINGLE_RACE_LAPS_BY_CLASS` is `[3, 4, 4, 5]`, so a PHANTOM race is two
laps longer than a VENOM one at a higher speed. Eighteen thousand ticks holds
every class, but a craft that crosses its last line stops being `Racing` and
coasts, so the board carries a `fin` column: a row measured past the flag is
not comparable with one still racing.

### The team axis, and the constant it retired from one of its two jobs

`Tuning::max_turn_rate` was one global constant, `1.8`, doing two unrelated
jobs: a clamp on the turn rate pure pursuit may *request* in `Driver::steering`,
and the kinematic corner-speed limit `max_turn_rate / k` in
`pace::corner_target`. The first is a permission. The second is a claim about
what a hull can do, and it was wrong for **every craft on the disc**.

The yaw axis's steady state is `omega = steer * Turning.amount / (damping *
I_yy)`. `I_yy` is **not per-craft**: the inertia box `(12, 8, 12)` and the mass
`0.9` are code literals at a single call site in the ship-entity constructor
(see `oag_physics::forces::YAW_INVERSE_INERTIA`, which carries the
disassembly), and `Misc` `width`/`length`/`height` reach the *collider*, not the
tensor. So every craft has `I_yy = 21.6` and the only per-craft term is
`<Turning amount>`. Read off `handlingstats.xml` for all eight teams and all
four classes:

| team | `Turning.amount` | ceiling (rad/s) | against 1.8 |
| --- | ---: | ---: | ---: |
| Feisar | 1.80 | 1.6667 | -7.4 % |
| Assegai, AG Systems | 1.68 | 1.5556 | -13.6 % |
| Qirex | 1.55 | 1.4352 | -20.3 % |
| EGX, Goteki | 1.42 | 1.3148 | -27.0 % |
| Triakis, Piranha | 1.30 | 1.2037 | -33.1 % |

**`Turning.amount` is identical across the four speed classes** on all eight
teams, so team and class are independent axes. And **no craft reaches 1.8** -
the corner limit asked every one of them for a speed its hull could not rotate
at, by 7 % on the best team and 33 % on the worst. The 1.5556 row is the one
measured by hand on `07_Track`, and
[engine.md](../ghidra/functions/psp-pulse-usa/engine.md) measures the
*original* hull at 1.42-1.51 under full lock, so the arithmetic lands within a
few per cent of the original's own behaviour.

This settles the caveat the `max_turn_rate` sweep table ("Tuning sweep tables") carries -
that the curve "flattens either side of 1.8 rather than continuing to improve,
which is the shape of a constraint that has stopped binding". **The constant
was the wrong shape, not the wrong value.** That sweep measured the steering
clamp, where 1.8 really has stopped binding; the same number was silently doing
a second job where it could never bind correctly.

So `Context::yaw_ceiling` now carries the flown craft's own number,
`oag_ai::hull_yaw_ceiling` derives it, and `corner_target` takes `min(hull,
tuning.max_turn_rate)` - the permission still caps, so `Difficulty::tune`'s
ladder survives and a Novice does not corner like an Ace. `Driver::steering`'s
clamp is deliberately left on `max_turn_rate`: lowering *that* is what the 1.2
row of its sweep measured as worse.

Board-wide, over all 48 rows:

| class | contact ticks | wall-charged (capped) | end-of-run pool | respawns | eliminated | racing ticks |
| --- | --- | --- | --- | --- | --- | --- |
| VENOM | 1,252 -> **1,154** | 255.5 -> **224.4** (-12.2 %) | 838.9 -> **908.0** | 1 -> 1 | 1 -> 1 | 210,968 -> **212,844** |
| FLASH | 2,008 -> **3,305** | 476.7 -> **432.5** (-9.3 %) | 655.7 -> **684.7** | 1 -> **7** | 2 -> 2 | 205,552 -> **206,865** |
| RAPIER | 3,124 -> **2,764** | 869.6 -> **726.7** (-16.4 %) | 255.9 -> **382.9** | 4 -> **13** | 5 -> **3** | 180,175 -> **195,135** |
| PHANTOM | 4,627 -> **3,701** | 1032.2 -> **893.7** (-13.4 %) | 101.4 -> **216.0** | 17 -> **14** | 4 -> **5** | 185,171 -> **173,027** |

**The totals above cap each row's charge at its own 95-unit pool, and the
uncapped ones are different enough to matter.** The `Racing` gate stops a
*wreck* accruing; it does not stop a craft that is still racing at **zero**
shield, where `damage::subtract` clamps the loss to nothing while the counter
keeps charging. Uncapped, the boards carry 277.3 and 96.5 units of over-pool
charge and the cut reads 18.5 %; capped it is **13.5 %** (2,633.9 -> 2,277.3),
and part of even that is fewer rows being ground alive at zero rather than fewer
walls hit. Capped is the honest number.

Charged shield falls at every class, the end-of-run pool rises at every class,
contact ticks fall at three of four, and **zero-contact rows double, 2 -> 4**. Eliminations go 12 -> 11. **The price is
lap time, 0.5-3.0 s a circuit, and respawns, which rise 23 -> 35 board-wide**: a
craft that brakes earlier wedges at low speed and is rescued, where before it
crashed at speed and was charged for it. Nothing on the board or in the gate
asserts on lap time, so no measurement here separates "the AI is slower" from
"the AI is correctly slower".

`race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`
stays twelve clean with `01_Track`'s single pre-existing respawn at driver
index 794, and both field ground-truth tests stay green.

### Where the board stands

**Four of 48 rows meet the standard**: `03` at VENOM and FLASH, `02` and `10`
at VENOM, each with zero wall contact - up from two before the change.
`07_Track` is the weakest row at every class and `13_Track` the second weakest. The full per-row table, the worst
cluster on each row and the open residuals are in this thread's handover file
rather than here, because they move every time the driver does.

## The differential airbrake never fires in a hairpin, and the reason is a latch

Added 2026-09-12, from a player report: the airbrake was visible but "for
U-turns I'd still expect it to be used more heavily, more akin to how one would
*drift*". It is worse than timid.

`what_the_airbrakes_do_through_a_hairpin` dumps the **ramped** `ShipState`
airbrake values tick by tick through `07_Track`'s documented 2,150-2,199
cluster, the tightest corner on the board's worst row. Lone Ace at VENOM,
**`L = 0.0` and `R = 0.0` on every tick** while the craft accelerates 99.6 ->
138.1 units/s at full lock, `grounded` 1.00 throughout, into the wall. Not
capped - zero. So `Tuning::trail_max`, the obvious first candidate, is a ceiling
on a quantity that never leaves zero and cannot be the constraint.

### Which of `trail`'s five gates, measured

Instrumented, run, reverted. 6,000 ticks:

| gate | rejects | share |
| --- | ---: | ---: |
| `recovering` | 0 | - |
| `curvature <= trail_curvature_floor` | 0 | - |
| **`curvature < peak * trail_exit_decay`** | **4,486** | **74.8 %** |
| `steer.command < trail_saturation` | 1,287 | 21.5 % |
| `rate_error <= trail_deadband` | 0 | - |
| **fires** | **227** | **3.78 %** |

Through the hairpin four of the five pass - `cmd = 1.000`, full lock, with
`rate_error` 0.22-0.53 rad/s, exactly the state the differential exists for.
The exit gate rejects it on `k = 0.0139` against a bound of `0.0320`.

### The root cause

`pace::track_peak_curvature` resets the high-water mark when curvature falls to
`trail_curvature_floor` - "since the line last went straight". **On this disc's
geometry the line never goes straight by that definition**: the smallest
windowed curvature in 6,000 ticks is `0.00127` against a floor of `0.00100`, and
the reset fires **zero** times. The mark becomes a monotone running maximum over
the whole race, latching at `0.04566`, so the exit gate really means "this is
not the tightest corner seen so far this race" - true almost everywhere after
lap one.

### The fix works and does not pay, which is the finding

`Tuning::trail_peak_decay` makes the mark leaky. At `0.99` the exit gate's share
falls **74.8 % -> 24.2 %** and the differential fires **3.78 % -> 11.58 %**.
Board-wide it is *worse*: every leak value costs contact ticks and end-of-run
shield against the latch and buys 0.16 s of mean lap. The full sweep is on
[`Tuning::trail_peak_decay`], which ships at `1.0` - no behaviour change, with
the measurement beside the constant.

**Why**: `trail` spends the differential *reactively*, once `trail_saturation`
says the steering loop has already run out of authority, and it "cuts lateral
grip exactly as hard as holding both sides would". Spending grip without
**banking** the higher corner speed it permits is a pure loss. The speed is
banked only if `corner_target` raises its target *because* the differential is
planned - `v = omega_steer / (k - C)` from the section above - and it does not.
This is evidence for that change, not against it.

And the hairpin stays shut even with the leak, for a second reason already on
the board: the windowed curvature **falls monotonically** through it, `0.01387
-> 0.00770`, so the estimator tells the driver the corner is opening while the
craft is at full lock hitting a wall. That is the same understatement the
`curvature_span` residual names, now feeding a second consumer. The gate's logic
is not wrong here; its input is.

## A valley is not a corner

Added 2026-09-30, from the AI lane that followed the airbrake scale fix
([engine.md](../ghidra/functions/psp-pulse-usa/engine.md#the-raw-steerx-is-on-the-100-scale-too)).
That fix made `oag_physics::airbrake::evaluate`'s forward term 100x stronger,
and three `ai_clean_lap_gate` rows went `CleanLap` to `Died` (`05_Track` FLASH
and VENOM, `14_Track` RAPIER) and `difficulty_ground_truth` went red. The
working theory was that the drivers, tuned against the weak term, now arrive
too fast. **It was mostly wrong**, and what replaced it is worth keeping.

### What the airbrake fix did not do

- **The AI has no model of the airbrake drag.** `oag-ai` emits `steer_x` on
  `-1..=1` like a player's snapshot and the physics does the `x100`, so nothing
  in the crate assumed the old scale and there was nothing to update.
- **It did not make the field drive into corners faster.** Over the 48 rows
  of the board, the mean clean-lap time per row moved by 16 ticks or less on
  every circuit, and contact ticks by under 100 on nine of twelve. `05_Track`
  carried the regression (537 to 5014, four classes), `07_Track` rose by 265
  and `13_Track` fell by 854.
- **Removing the differential does not undo it.** `trail_max = 0` turns
  `05_Track` VENOM back to `CleanLap` and kills `10_Track` and `04_Track` RAPIER
  instead, with `05_Track` FLASH and `14_Track` RAPIER still `Died`.

### What `05_Track` does

The fatal spot is a crest lip at about `(-651, 68, -517)`, reached up a hill from
`(-411, -5, -537)` on a straight, **with no airbrake held**. Traced, lone Ace at
VENOM, every pass of it on the pre-fix and post-fix trees:

| forward speed at the lip (units/s, about) | result |
| --- | --- |
| 70.9 to 73.5, ten passes across both trees | clears |
| 69.0 | clips: forward speed 68 to 18 in one tick, the craft slides back down and wrecks |

So the lip needs about 70 and the driver arrives at 71-74: a knife edge that
already existed. The airbrake change only moved lap 3's arrival down by 3.

The reason the arrival is that low is the driver braking for the hill. At the
foot, `Line::curvature` read the line levelling into the climb as a bend of
`0.013` (a radius of 75) and `corner_target` answered 117 against a speed of
126, so the Ace lifted and tapped both airbrakes for 14 ticks, losing 7 units/s
before a climb that takes out 50 more on its own.

### The change

**A pitch change is a corner only where it is convex.** `Line::curvature` now
reads `hypot(yaw, crest)` (`bend_angle`): the yaw is the turn between the chords
flattened onto the ground plane (world Y up), and `crest` is how far the climb
angle *falls* from one chord to the next. The foot of a hill and the bottom of a
drop are concave - the hover spring presses the craft into the road and nothing
is asked of its yaw - so they read straight. A crest or a drop's lip still reads
as a bend, because the craft leaves the road over it. A chord steeper than sixty
degrees keeps the plain angle between chords. Every other consumer of
`curvature` (the boost gate, the rocket gate, the differential's exit gate)
reads the same number, which is why it is changed in the estimator and not at
the brake.

**Chosen, not measured**, and it revisits two earlier commits. `64da876e`
discounted every pitch change and was narrowed to gaps in `2c1a3d4f` because it
"took crests faster everywhere and killed three more rows". Keeping the convex
half is what that experiment lacked: flattening both halves on this tree
(experiment, not shipped) fixed the three rows and killed `10_Track` RAPIER and
`09_Track` PHANTOM, and the convex-only version killed neither.

### The board

`ai_clean_lap_gate`, 48 rows, lone Ace, three states on the same code but for
the airbrake line and this change:

| | pre-airbrake-fix | post-fix | valley change |
| --- | ---: | ---: | ---: |
| `Died` rows | 11 | 14 | **10** |
| contact ticks, all rows | 13340 | 17357 | 13834 |

`05_Track` VENOM and FLASH and `14_Track` RAPIER are `CleanLap` again. **One row
is worse than before the airbrake fix: `07_Track` FLASH, `Died` to `Eliminated`**
(contact 854 to 869, not traced). `06_Track` and `09_Track` lap 1-2 s quicker on
every class they were `CleanLap` on. `13_Track` RAPIER and PHANTOM took more
contact (1129 to 1555, 641 to 1347), already `Died` either way.

`race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`
stays all twelve clean, and `05_Track`'s respawns are **0**, where they were 1
before the airbrake fix and 3 after it. `07_Track` still respawns once (at tick
1949, where it was 715).

### What this does not fix

- **Ace against Elite is thin.** `difficulty_ground_truth` is green at the
  unchanged `2.0 %` tolerance, with Ace **1.85 %** under Elite on the five-seed
  leader mean (6725 against 6852; 0.66 % before the airbrake fix, 2.14 % after
  it). Ace is below Elite on four of the five seeds, and the field's own mean is
  close to a tie, so the ordering is the pace-ceiling effect the test's own
  comment records and not a driving fault this change removes.
- **The crest lip is still a knife edge.** The driver now arrives with margin,
  but a craft that arrives below about 70 on a slower class or after a
  collision clips it, and once it has, **neither rescue fires**: it climbs and
  slides back on a cycle of about 60 ticks for up to 5,000 ticks, never stopped
  for two seconds and never far from its line. That is race rules, not driving,
  and not attempted here.
- **`05_Track`'s racing line still runs above its own collision surface for 134
  samples** ([above](#the-clean-ace-board-every-circuit-every-speed-class-wall-contact-counted)),
  a separate open fault.

## The speed plan

Added 2026-10-03 (lane `pulse-ai-speedplan`), against the maintainer's
standard: *"I'd expect the AI to do perfect laps at almost full speed and
airbrake use."* **Ours, not the original's**: the original's opponents take
their speed from a schedule keyed on the player (see
[what we build instead](#what-we-build-instead)); this uses nothing a player's
craft does not have. Every constant below is **chosen, not measured**, and
carries no confidence score.

### What it is

`oag_ai::SpeedPlan` is a speed per racing-line sample, learned at race start
by driving the authored line in **our own physics** - the same
`oag_physics::step`, the same collision, the same handling, the race's own
pads, reset volumes and four-second airborne rescue. `Race::start` builds one
for the field (every opponent flies the same handling), from opponent slot 1's
grid pose. `crates/ai/src/plan.rs` and `plan/brake.rs`:

1. **Calibration.** Full throttle from the grid, then full symmetric
   airbrake from the fastest point; the deceleration actually seen is binned
   by speed (10 u/s bins, 80 % of it banked). A first guess the search
   corrects, not the arbiter.
2. **Learning, two laps from the grid.** The neutral driver
   (`Driver::default()`, seed 0, the top level's `Tuning`) steers; the plan
   decides throttle and the symmetric brake (`plan::longitudinal`); the
   driver's own differential is kept and folded on top (`plan::airbrakes`).
   A **failure** is what the race would see: a wall contact (the clean-lap
   board's own `contacts > floor_contacts`), a wreck, leaving the rescue
   distance, a reset volume, more than 4 s airborne, or a stall. On a failure
   the ceilings over the last 45 *grounded* ticks are lowered to 0.96 of
   `min(speed done, ceiling)` - harder each third retry at the same spot -
   the backward pass re-derives the braking zones, and the run rewinds to a
   checkpoint before the earliest sample whose target moved.
3. **Backward pass.** `v0^2 = v1^2 + 2 a ds` from each sample to the one
   before, `a` off the calibration, twice round the ring, never through a
   *held* sample, and nothing above 1,000 u/s.
4. **Jumps.** A wall touched after more than 0.5 s in the air with braking on
   the run-up marks the run-up *held*: ceilings lifted, nothing brakes through
   it. A stall at full throttle all along is beyond any plan: the craft is put
   back past the spot the way the race's rescue would, and on the flying lap
   the spot is recorded as unresolved.
5. **Verification.** Two laps from the grid following the finished plan,
   nothing learned. **The race only uses a plan that verified clean** - no
   wall, no rescue; otherwise its opponents keep the corner model
   (`pace::corner_target`) and the loader log says so.

### What a driver does with it

`Context::plan` carries it. With a plan, the target speed is the plan's
lowest target over the next `6 x patience` ticks of travel (the airbrake
ramp), times **`sqrt(grip_believed x commitment)`, capped at one** - the
difficulty table's own corner-speed fractions, 0.55 / 0.69 / 0.84 / 1.00 from
Novice to Ace. Novice and Skilled also hold **0.88 and 0.96 of the plan's
verified pace** everywhere (`Difficulty::pace_share`), because the plan leaves
most straights unlimited and a corner margin alone shrank the ladder to
0.95 / 0.99 / 1.00 / 1.00 of the Ace's distance. Calibrated against
`difficulty_ground_truth` on the corner model (0.865 / 0.96 / 1.00 / 1.00);
with the share it reads 6038 / 6706 / 7074 / 7160, strictly ordered.

**A pilot's line is a handicap on a plan.** `Personality::spent` blends the
driving axes - line bias, wander, inside line, lookahead, patience,
differential, width - toward neutral by `(1 - margin) / 0.452`, so a balanced
Ace drives the plan's line exactly and a Novice keeps all of its character.
Commitment stays the margin; courtesy, defence, caution, ramming, weapons,
provocation and the barrel roll are untouched, and so are the social terms
(yielding, blocking, the contact floor) and mine avoidance. Measured on the
96 lone rows: the plan with full character was clean on 15, gating only the
lateral axes on the plan's slack ahead 15-23, all lateral axes off 45, and
every driving axis spent 84; a pilot's own lookahead alone put `06_Track`
FLASH into the wall at samples 723-740 every lap, under the plan's own pace.

**And in traffic a pilot is a pilot again.** A field of Aces all on the
plan's one line at its one pace ran nose to tail:
`craft_sticking_ground_truth` went 1,558 -> 2,101 overlapped pair-ticks,
1,420 of them sustained, past the old pathology's 1,062. So the share spent
is `max(level, traffic)`, `traffic` rising from zero at a 40-unit gap to one
at contact with the nearest rival noticed. It reads 523 (275 sustained); a
lone craft has nobody and is unchanged. The cost is field wall contact,
32,440 without it and 39,804 with it (49,833 at a 120-unit range).

### What it buys

Same harness before and after, `crates/game/tests/ai_speed_plan_sweep.rs`:
24 layouts (12 forward, 12 reversed) x 4 classes, Ace, `SingleRace`, 18,000
ticks or the finish, team the title default (Assegai). **Clean** is a row
with no wall contact, rescue or death. Shield is reported twice: **per lap**
is the mean pool lost per completed lap, **end** the pool when the run stops.

| scenario | class | clean | contact ticks | respawns | destroyed | mean best clean lap | per-lap shield lost | end-of-run shield |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| lone | VENOM | 4 -> **21** of 24 | 1,332 -> 75 | 0 -> 0 | 0 -> 0 | 42.1 -> 39.3 s | 3.6 -> 0.8 | 84.1 -> 92.6 |
| lone | FLASH | 2 -> **23** | 2,688 -> 72 | 1 -> 0 | 2 -> 0 | 38.3 -> 34.7 s | 6.5 -> 1.0 | 70.6 -> 91.1 |
| lone | RAPIER | 1 -> **21** | 3,295 -> 368 | 6 -> 0 | 2 -> 0 | 34.7 -> 31.2 s | 10.5 -> 1.9 | 55.9 -> 87.4 |
| lone | PHANTOM | 0 -> **19** | 4,475 -> 716 | 7 -> 1 | 4 -> 1 | 32.5 -> 28.3 s | 13.8 -> 2.2 | 34.4 -> 84.2 |
| field (7 craft) | all | 85 -> **151** of 672 | 113,203 -> 39,804 | 60 -> 41 | 101 -> **28** | 2.3-4.5 s faster per class | 13.5 -> 8.6 | 46.7 -> 62.1 |

Lone totals: clean 7 -> **84 of 96**, contact 11,790 -> 1,231, respawns
14 -> 1, destroyed 8 -> 1. Of the twelve lone rows still not clean, nine are
layouts whose plan did not verify (below) and so drive the corner model; the
other three touch 1, 6 and 21 ticks (`17_Track` VENOM, `09_Track` PHANTOM,
`25_Track` PHANTOM). `ai_clean_lap_gate` went `Eliminated` 11 -> 1 and
contact 8,828 -> 1,243 over its 48 rows; `10_Track` FLASH is the one row that
laps slower, 34.2 -> 36.0 s, trading 66 contact ticks for none. The
parked-player Eliminator finishes 24 of 24 seeds (`examples/eliminator_seed_sweep.rs`)
(147 s before; 33-196 s, median 139.0 s, with the traffic rule below; the
original's own is 85 s, and the gap is still the AI not cheating). No original lap times or AI lap times are documented anywhere in
this tree to compare the solo laps against; the Eliminator's 85 s is the only
original timing there is.

### Plans that do not verify

`crates/game/tests/speed_plan_ground_truth.rs` pins the exact set, at the
title-default team: **91 of 96** layout-class plans verify clean (87 until
`05_Track` forward's four did, [below](#de-konstruct-black-the-first-jump-is-a-magstrip)).
Not: `06_Track` VENOM and RAPIER
(the corner after the gap at 1196-1200), `14_Track` PHANTOM and `29_Track`
RAPIER and PHANTOM (rescued, in the air or off the line, at 1247 and
2402-2441 with every ceiling tried). By team: Feisar (`Turning` 1.80) 89,
Assegai 87, Piranha (1.30) 83 of 96.

### What it costs

One `oag_physics::step` for one craft against real collision is 21-25 us in
release on a loaded machine, the driver 1.3-2.6 us
(`examples/physics_step_cost.rs`). A plan is 7,000-130,000 steps: **0.1-2.0 s
per layout, median 0.18 s, mean 0.31 s** in release (what `just play` runs;
the dev profile builds the simulation crates at opt-level 2 and measures the
same), the slow ones being exactly the plans that do not verify and are then
thrown away. `05_Track` forward's four took over a second each (about 130,000
steps) until they verified; they take 9,700-29,000 steps now. Not cached: a race
start pays it once. The short-horizon rollout idea (8 craft x 9 candidates x 90
ticks = 6,480 steps) would cost about 150-175 ms per full-field replan, nine
to ten ticks' worth.

### Still open

- The five unverified plans.
- A reset-volume respawn loop seen once in the field: `29_Track` VENOM, one
  craft put back at sample 45 every 46 ticks, 61 times - the rescue pose
  lands in a reset volume. Race rules, not the plan.
- The plan is built for slot 1's handling and grid pose; a field of mixed
  teams would need one per handling.
- Lap 1 is learned from a standing start the race also has, but not with the
  race's countdown launch boost.
- **de Konstruct Black's first jump is a plan test case now**: Black is
  `05_Track` forward, its plans verify at all four classes
  ([below](#de-konstruct-black-the-first-jump-is-a-magstrip)), and
  `ai_dekonstruct_symptoms_ground_truth` measures the lone Ace and the field on
  both layouts against them.
- The cross-platform gate for the plan is
  `crates/ai/tests/determinism.rs::the_speed_plan_matches_the_committed_reference`,
  on the probe's invented circuit; a disc-backed plan is pinned only as
  verified-or-not (`speed_plan_ground_truth.rs`), not bit for bit.

## Branch choice at a fork

**Recovered 2026-10-05**, `Ai_ChooseBranch` (`0x08854920`), evidence on
[ai-branch-choice.md](../ghidra/functions/psp-pulse-usa/ai-branch-choice.md):
every opponent flips a fair coin at every fork it reaches, independently, on
entering the path before the fork, and drives the chosen side until it is past
the merge. The excluded path is what its lookahead and its lap progress walk
honour, so a craft on either side reads progress on one scale. Nothing else -
position, class, difficulty, pilot, corridor - enters the choice in Pulse. A
craft knocked onto the other side off the track takes that side instead.

2048 (`0x8119ae90`), HD (`FUN_000fe818`) and Omega (`FUN_012c0480`) flip the
same coin. 2048 adds an authored per-circuit override after it (circuit record
`+0x150`/`+0x154`/`+0x158`, source not found) and a different re-commit; neither
is ported. See [the 2048 page](../ghidra/functions/vita-2048-eu-v104/ai-branch-choice.md).

**Ported 2026-10-05**, for every title, since all four flip the same coin:

- **The coin** is `oag_ai::branch::coin`: bit 8 of a draw off `oag_core::Rng`
  seeded from the driver's own seed and a per-driver visit counter, so it moves
  no other roll in the race (49.6 % alternates on the first draw over 14,000
  seeded craft, 50.0 % after). The draw source is ours; the fairness, the timing
  (on entering the pre-fork path) and the hold to the merge are the original's.
- **The geometry** is `oag_race::course::Route`: every way from a ring fork back
  to the ring, including 2048's multi-path alternates and forks nested inside an
  alternate (`square`, `mall`, `subway`), which `Course::branches` - the
  Repulser's narrower view - drops. A nested route is picked by rolling its
  coins in order (`oag_ai::branch::choose`), so two forks deep is one in four.
- **The line** is ours: one AI line per route, the ring from the route's merge
  round to its split and then the route's own samples, built by the same
  `racing_line` the ring's is (`oag_raceplay`'s `routes` module). A driver's
  index is into whichever line it is on, and every consumer that pairs it with
  a line, a sample or a plan asks `Race::line_of` / `ai_sample_for` / `plan_of`.
- **Chosen, not measured**, each forced by a measured failure on `05_Track` or
  `07_Track` (`cargo run -p oag-game --example fork_trace`):
  - the driver holds its draw (`Branching::pending`) and moves onto the route's
    line only 320 samples short of the split, so the shared stretch is driven
    on the ring's line and plan;
  - each route has its own speed plan, and **a route whose plan does not lap
    clean and contact-free is not offered to the coin** (its share stays on the
    ring): `07_Track`'s centre ramp parks our craft at route sample 196 without
    a wall touch, so 07's route is never taken;
  - across the stretch where a route is still within 8 units of the ring, its
    corridor is held to 1 unit either side of its line: a Novice on the far
    side of path 1's corridor rode the ring's take-off ramp and met the divider
    every lap;
  - the re-commit (below) never fires in the air, scores the craft's own line
    over its search window, and needs a margin of 1: as first ported it flipped
    craft between lines every tick over 05's jump, 328 times in one run.
- **The re-commit** is otherwise the original's Pulse rule: off the track and
  moving, a craft scored better on a sibling (`|across| * 10 + along`) switches
  to it.
- **Lap progress** on a route is read off the ring span it replaces - see
  [lap counting](lap-counting.md#where-we-differ).

Which routes are driven (Ace, VENOM, 2026-10-05): Pulse 05 and 14 yes, 07 no;
Omega's copy of `altima` both.
2048: every route on `square`, `park`, `tower`, `mall`, `bridge`, `arena`,
`subway` and `altima`; none on `cathedral` (both verify with failures) or `sol`
(no route laps, and the ring's own plan does not verify there either).

Measured on the result: a full field on `05_Track` over a race decided 17 times
for the ring and 24 for the route; on 2048's `altima` over three laps, 13/7 at
one fork and 6/16 at the other, every running craft within a lap of the rest
(`ai_fork_split_ground_truth.rs`). `race_ground_truth`'s lone-craft board stays
all twelve clean with no respawns, best laps as before except 05 (34.8 ->
34.6 s). de Konstruct Black's destroyed-craft board moved 5 -> 6 forward, no
loss on the route (`ai_dekonstruct_black_ground_truth.rs`).

**Not ported**: 2048's per-circuit override (circuit record `+0x150`), its
per-craft construction coin and its own re-commit; pad-seeking while on a route
(the pads sit on ring indices).

## Each title's craft laws (2026-10-09)

The AI obeys the player's physics, so it has to plan with the craft it actually flies. Wipeout
HD builds its inertia with mass `1.0` where Pulse passes `0.9` (`I_yy` 24 against 21.6), clamps
its steering ramp at the target, and hovers on four probes
([craft-inertia.md](../ghidra/functions/ps3-hdfury-eu/craft-inertia.md),
[hover-four-point.md](../ghidra/functions/ps3-hdfury-eu/hover-four-point.md)). The race seats
those as `oag_title::RaceDefaults::craft_laws` and `hover_rig`; a title with `None` (Pulse,
Pure, Omega, 2048) flies Pulse's (`oag_raceplay::launch_hover::craft_inertia(None)` and
`physics_rig(None)`, and `steer_ramp_clamped` is `is_some_and`, so `false`).

### Census: what the AI assumed

| where | assumed | now |
| --- | --- | --- |
| `crates/ai/src/driver/pace.rs`, `hull_yaw_ceiling` | `I_yy = 1 / YAW_INVERSE_INERTIA`, Pulse's 21.6, for every craft | `body.inertia.y`, the tensor the race seated the craft with |
| `crates/ai/src/plan.rs` (`SpeedPlan::build_within`), `plan/probe.rs` (`drive`) | the ceiling above | `craft.start.body` |
| `crates/raceplay/src/field.rs`, the opponent and autopilot calls | the ceiling above | that ship's own `physics.body` |
| `crates/raceplay/src/start.rs`, the speed plan built in `Race::start` | **the plan's craft had Pulse's two-probe hover and cycling steering ramp on HD**: both were only written by the first tick's `set_hover_caps`, after the plan was built | rig and clamp seated with the inertia, before the plan |
| `crates/gameplay/src/spawn.rs`, `Ship::place_at` | reset the rig and clamp to Pulse's, so the plan's rescues and route plans started at an index flew Pulse's laws | kept, as mass and inertia already were |

Nothing in `oag-ai` models the steering ramp in closed form: the ceiling is a steady state the
ramp does not change, and the plan steps the real ramp through `oag_physics`. The determinism
probe (`crates/ai/src/probe.rs`) builds its craft from `ShipState::default()`, Pulse's laws, on
purpose: it is an invented scenario and its hash did not move. Out of this lane and unchanged:
`oag-trace`'s two spawns (`crates/trace/src/main.rs`, `box_inertia()`) and
`Setup::headless`'s `None`s, which has no title.

### Pulse is bit for bit

`ship_inertia_at(0.9).y` is the same expression as `1 / YAW_INVERSE_INERTIA`, and
`a_pulse_craft_reads_pulses_ceiling_bit_for_bit` pins the bits. A Pulse craft's rig and clamp
are the defaults `place_at` now keeps. `race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`
prints the same twelve lines before and after (all twelve clean, `01_Track` 0 respawns).

### What moved on HD, and which change moved it

Ablated: with the plan-state fix alone the HD board below is identical to the full change, so
**the ceiling changes nothing on HD today**. Every HD circuit's speed plan verifies, and a
followed plan overrides the corner model the ceiling feeds. The ceiling still binds wherever no
plan is followed: a route whose plan did not verify, and `corner_target`'s fallback. HD's Feisar
now reads `1.5` rad/s, the yaw rate RPCS3 measures
([hd-handling-ground-truth.md](../physics/hd-handling-ground-truth.md)), where it read `1.667`.

All movement comes from the plan simulating HD's own hover and steering. Measured by
`crates/game/tests/hd_ai_craft_laws_ground_truth.rs` (HD/Fury EU, Venom, Ace, Single Race with
weapons off, 12,000 ticks). Shield starts at 95. The only shield an opponent loses off a wall is
a barrel roll's cost, 14.25 each (`roll_cost` 15 % of 95, `oag_physics::barrel_roll::arm`).

**One opponent alone**, flying laps 2 and 3, wall-contact ticks over the run, end-of-run shield
(no respawns on any circuit, before or after):

| circuit | laps before | laps after | contacts before | after | shield before | after |
| --- | --- | --- | ---: | ---: | ---: | ---: |
| Talon's Junction | 41.18 / 41.22 | 41.07 / 41.08 | 0 | 6 | 95.0 | 94.6 |
| Vineta K | 34.27 / 34.28 | same | 0 | 0 | 95.0 | 95.0 |
| Ubermall | 37.17 / 37.22 | 36.97 / 37.00 | 15 | 0 | 93.8 | 95.0 |
| Amphiseum | 47.72 / 47.63 | 47.77 / 47.73 | 0 | 0 | 95.0 | 95.0 |
| Modesto Heights | 37.13 / 36.52 | 37.00 / 36.98 | 0 | 0 | 66.5 (2 rolls) | 80.8 (1 roll) |
| Tech de Ra | 40.23 / 40.23 | 40.22 / 40.22 | 0 | 0 | 95.0 | 95.0 |
| 02 | 36.38 / 36.48 | 36.25 / 36.32 | 0 | 0 | 95.0 | 95.0 |
| 03 | 40.98 / 40.87 | same | 0 | 0 | 95.0 | 95.0 |
| Chenghou Project | 41.43 / 42.08 | 42.08 / 42.10 | 0 | 0 | 80.8 (1 roll) | 66.5 (2 rolls) |
| Sebenco Climb | 45.67 / 45.67 | 45.05 / 45.13 | 11 | 0 | 94.5 | 95.0 |
| Sol 2 | 36.67 / 36.67 | 36.38 / 36.38 | 0 | 0 | 95.0 | 95.0 |
| Anulpha Pass | 37.45 / 36.42 | same | 0 | 0 | 95.0 | 95.0 |

Per lap on Talon's Junction after: 2, 1 and 2 contact ticks, shield at lap end 94.9, 94.8, 94.6
(before: 0 every lap, 95.0). Board total 26 contact ticks before, 6 after.

**The full field** (seven opponents), five seeds each, summed over seeds: wall-contact ticks,
shield the walls charged, and the mean flying lap. No respawns in any run.

| circuit | contacts before | after | wall shield before | after | mean flying lap before | after |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Talon's Junction | 642 | 788 | 55.8 | 67.4 | 40.70 | 40.69 |
| Vineta K | 137 | 137 | 16.0 | 16.0 | 34.60 | 34.60 |
| Ubermall | 828 | 777 | 95.7 | 92.9 | 37.39 | 37.11 |
| Sebenco Climb | 1,102 | 895 | 74.8 | 47.6 | 45.63 | 45.00 |

Per seed on Talon's Junction, contacts before / after: 105 / 159, 163 / 168, 126 / 138,
123 / 134, 125 / 189: worse on every seed, so not noise. Ubermall and Sebenco improve on most
seeds and on the sums, and lap 0.3-0.6 s faster. Across the four fields contacts fall from 2,709
to 2,597.

**So the fix cuts HD wall contact where the old plan was wrong about the craft (Ubermall,
Sebenco), and costs some on Talon's Junction**, where the plan built on Pulse's hover happened
to fly a line with more margin. Talon's lone contacts are 0.4 shield over 200 s. Why the correct
plan scrapes there is open; the lone craft's contact ticks are where to start.

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

## de Konstruct Black: the field dies where one craft does not (2026-10-03)

**Which file.** **Black is the forward `05_Track`** (`track.vex`) and White the
reversed one, `21_Track` (`track_reversed.vex`). That is what the disc's own
string table says (`21_Track` reads "de Konstruct White", `05_Track` "de
Konstruct Black") and it was **measured live on PPSSPP, 2026-10-03**: the Track
Select entry titled Black started the craft at (-39.86, -16.40, -193.97), ours on
the forward `05_Track` is (-39.57, -16.40, -191.02); the entry titled White
started at (21.98, -17.48, -63.71), ours on `21_Track` is (20.05, -17.40, -67.61).
See [track.md](../formats/track.md#white-and-black). The maintainer's report ("the
AI really struggles with de Konstruct Black") was therefore about the forward
`05_Track`, which is the worse field measured below.

**The report, measured** (`crates/game/tests/ai_dekonstruct_black_board.rs`,
`OAG_SWEEP=1`: seven Aces plus the parked player slot, three seeds, 21 craft per
cell, 18,000 ticks, weapons as the mode ships them). Craft **destroyed** of 21:

| class | Black before | Black after | White (reversed, control) |
| --- | ---: | ---: | ---: |
| VENOM | 19 | 15 | 1 |
| FLASH | 19 | 13 | 2 |
| RAPIER | 13 | 11 | 3 |
| PHANTOM | 13 | 10 | 6 |

The lone Ace (slot 1) never died on Black at VENOM and died once at FLASH, which
is why the twelve-circuit gate was green all along. Across every circuit in both
directions (two seeds, VENOM and RAPIER) forward `05` is the worst VENOM field
(13 of 14 destroyed, 37,048 wall-contact ticks against 07's 6,116 and 18 respawns
against 0), and its reversed twin is among the best.

**Mechanisms found, in order of how much they explain:**

1. **The first jump's crest is read as a corner.** `05_Track`'s line has a crest
   bend (curvature 0.0126, a speed target of 124 against a craft doing 127) 20-30
   samples *before* the first unsupported sample, so the driver coasts off the
   ramp. A lone Ace at the pole arrives at 118.9; the field arrives at 105-119 and
   a craft leaving the ground a few units a second short clips the far lip at 204-206
   and loses 70 of its speed. Fixed by `Line::curvature` reading **yaw only** on the
   75 units before a gap of 40+ unsupported samples, and `Driver::drive` skipping
   the caution lift there. Takeoff speeds at VENOM went from 105-119 to 118-134.
   **Chosen, not measured.** The first version marked every gap and silenced real
   corners before `07_Track`'s gap (399 to 708 contact ticks), and the 5- and
   12-sample lips on `06` and `01` took 20-40 more contact ticks for a faster lap.
2. **A wedge at 586-600.** After a bad landing a craft swings across the road and
   ends up 17-29 units left of the line (the corridor is +-10) creeping at 1-3 units
   a second with the throttle full, and the next arrival piles onto it. The stall
   rescue needs under 1 unit a second for 120 ticks, so a creeper waits 800+ ticks.
   **Not fixed.** What did not work, each measured against the field's destroyed
   count and the gate: a speed floor on the yaw demand (10-60 cut 27 to 4-9 over 6,000 ticks but only
   about 4 over 18,000, and cost contact on 07, 10, 16 and 09 and regressed 04
   RAPIER), a lookahead that grows with cross-track error (25-33 against 4), a
   speed cap off the line, steering held straight in the air (the lone Ace then
   needed 4 respawns), a wider stall trigger (the pile forms faster than 300 ticks).
3. **Not causes**, each switched off in turn: craft-to-craft contact, the speed
   pads, weapons (Black's *reversed* twin dies only from them), the personality
   axes (a plain line follower in slots 2, 4, 6 and 7 alone on the circuit dies at
   600-650 as well - slot 1 is the pole, the one grid position that arrives fast
   enough).

**Where it stands.** `ai_dekonstruct_black_ground_truth.rs` bounds the field's
destroyed craft per (class, seed) on both layouts: 64 before, 49 after across the
twelve forward cells, 12 and 12 reversed. `ai_clean_lap_gate`'s four `05` rows moved
(FLASH `Eliminated` to `CleanLap`, 1,700 contact ticks to 140; laps 26-35 ticks
quicker; PHANTOM contact 203 to 232); every other row reproduced exactly, and the
twelve-circuit gate stays all-twelve-clean with `01` and `06` unchanged.
Roughly half the field still dies on Black: the wedge and a second crest at
idx 399-484 (a craft at 250 units a second after a pad flies it and lands wide) are
open.
**Superseded the same day**: the wedge and the field's deaths were one fall into
a pit under the upper road, caused by the line riding a magstrip up the first
jump's ramp; see [below](#de-konstruct-black-the-first-jump-is-a-magstrip).

## de Konstruct Black: the first jump is a magstrip

Added 2026-10-03 (lane `pulse-ai-05`). Ours throughout: every rule below is
**chosen, not measured**, and carries no confidence score; the measurements
are of our own physics on the disc's own collision.

### The walls at 424-637 are a pit, not the circuit

`05_Track` forward's speed plan failed at all four classes, and the search
reported walls at line samples 424-637 that the craft "touches at 15 u/s too".
A per-tick probe of the plan's own drive (`oag_ai::plan::probe`, printed by
`crates/game/tests/ai_plan_probe.rs`) showed the craft **50-77 units below the
line** from sample 200 on. Casting straight down from the line finds the road
3 units under it everywhere from 216, and a second floor 26-81 units further
down from 216 to 576; casting up from the craft finds road 25-75 units overhead.
So a craft that falls short of the first jump (samples 162-210 have no surface
under the line) lands in a pit under the upper road, and the driver's windowed
locator keeps it on upper-line indices while it meets the pit's own walls. The
pit ends at a wall at 586-614, where the craft creeps along it at 1-3 u/s with
the throttle full, 33-50 units from its line point: **the wedge of the section
above is the same fall**. No authored `Reset` volume reaches the pit floor (the
nearest sheets sit 15-30 units above it), and at full throttle the plan's craft
fell short on every lap.

### Why it fell short: the run-up is a magstrip

The race's lone Ace cleared the same jump on the corner model. The difference
was where it drove: six units right of the authored line, where the ramp from
sample 64 to 154 is `Floor Collision`. The line itself runs up the left two
thirds, which are `Mag Floor Collision`. `oag_physics::maglock` projects the
surface-normal component out of a craft's velocity while it is locked, so over
the ramp's convex top a craft on the strip leaves along the road rather than
with the climb it carried. Measured at full throttle, VENOM: **78 u/s of climb
at takeoff on the line, 87 off the strip**; the first lands 35 units under the
lip, the second on the upper road. Moving the line six units right at sample
110 cleared it with no other change, and the speed pad at 2845 (which the
race's Ace happened to cross, 16.6 units right of the line) does not by itself:
its boost is spent 260 samples before the ramp.

### The changes

1. **The line leaves a magstrip on a takeoff run-up** (`race/takeoff_line.rs`,
   `oag_ai::line_shift::toward`). On every sample `Line::is_takeoff` marks
   whose line point is over `Mag Floor`, the line moves to the nearest corridor
   offset whose surface, and 2.5 units either side of it, is plain `Floor`,
   easing in and out over 120 units; a run-up with no such room is left alone.
   It reads only the collision. **Only `05_Track` forward has such a run-up**
   (32 samples move); every other layout's line is byte-identical.
2. **The corridor stops short of the strip** on every sample the move reached.
   In a field the character a pilot gets back in traffic
   ([above](#what-a-driver-does-with-it)) put craft back on the strip at -5 to
   -8 units; field takeoffs that fell into the pit, VENOM, three seeds, went
   18 -> 7, RAPIER 4 -> 3.
3. **`oag_race::recovery::BENEATH_LINE`**: an opponent on the ground, on a
   supported sample, more than 15 units below its own line point along the
   sample's up is lost, on the distance trigger's own `RESCUE_TICKS` dwell.
   The original's rescues are `Reset` volumes and the four-second airborne
   clock; what it does with a craft in this pit is not known. Measured before
   it was chosen, all 24 layouts, seed 1, 18,000 ticks: a lone Ace at VENOM and
   PHANTOM never spends a tick grounded-beneath; a field of seven trips it 4
   times in 48 races, each a craft knocked onto a lower road (`07_Track` 657,
   `23_Track` 246, 714, 715). Leaving out unsupported samples is load-bearing:
   `25_Track` reversed's drop at 690-740 put a craft that landed early 77
   ticks under the line.
4. **A lower level holds at least 0.96 of the plan's pace within 450 units of
   a takeoff run-up** (`RUN_UP_SHARE`, `RUN_UP_REACH`). A Novice at 0.88 of the
   plan's 128 u/s hit the far lip on every lap at VENOM; the field's takeoffs
   fell short at 119-120 and cleared from 120.4. A floor and not a lift to
   one: at one a Skilled craft landed at full pace at PHANTOM and could not
   brake to its own corner margin after the landing. **It reaches every layout
   with a gap of 40+ samples** (02, 05, 09, 10, 14, 18, 25, 26, 30) and only
   the Novice level (Skilled already holds 0.96). Novice lone on those nine at
   four classes, floor off -> on: dead stops 7 -> 1, rescues 10 -> 1,
   destroyed 3 -> 4; worse rows are `10_Track` PHANTOM (one dead stop, lap
   +64 ticks), `25_Track` PHANTOM and RAPIER (one destroyed each) and
   `26_Track` PHANTOM (one rescue).

### What it bought

| | before | after |
| --- | --- | --- |
| `05_Track` forward plans that verify | 0 of 4 | **4 of 4** (9,700-29,000 steps, were ~130,000) |
| Black field destroyed of 21, VENOM / FLASH / RAPIER / PHANTOM | 15 / 13 / 11 / 10 | **0 / 0 / 1 / 3** |
| Black field wall-contact ticks, VENOM | 46,616 | 565 |
| Black field end-of-run shield, VENOM / PHANTOM | 18.4 / 14.5 | 50.7 / 35.9 |
| lone Ace best lap, ticks, VENOM / FLASH / RAPIER / PHANTOM | 2,354 / 2,136 / 1,884 / 1,737 | **2,091 / 1,833 / 1,554 / 1,329** |
| lone Ace wall contact ticks, VENOM / PHANTOM | 68 / 232 | 0 / 0 |
| field dead stops (seed 1), VENOM / FLASH / RAPIER / PHANTOM | 8 / 3 / 3 / 9 | 0 / 0 / 1 / 0 |

On the speed-plan sweep (`ai_speed_plan_sweep.rs`, 24 layouts x 4 classes,
same harness as [above](#what-it-buys)): lone Ace clean **84 -> 87 of 96**,
contact 1,231 -> 949, respawns 1 -> 1, destroyed 1 -> 1; the field (672
craft-rows) clean 151 -> 158, contact 39,804 -> 17,073, destroyed 28 -> 18,
respawns 41 -> 41, per-lap shield lost (field, VENOM, forward) 11.7 -> 8.7,
end-of-run shield 64.6 -> 68.6. Every reversed lone row is unchanged; the
reversed field moved only through the beneath rescue (PHANTOM destroyed 6 -> 4).
Of the nine lone rows still not clean, five are the unverified plans, three are
the verified rows that already touched, and one is `05_Track` PHANTOM's single
rescue at the 428 pad. `race_ground_truth`'s twelve-circuit gate stays twelve
clean; `05_Track` laps 39.2 -> 34.8 s, every other row identical. The
Eliminator finishes 24 of 24 seeds, median 139.03 s (was 139.0).

White (the reversed `21_Track`) is unchanged by all four: its line has no such
run-up, and no White cell moved on the board. The board is
`ai_dekonstruct_black_board.rs` (`OAG_SWEEP=1`), three seeds; the symptoms are
`ai_dekonstruct_symptoms_ground_truth.rs`.

### The maintainer's report, measured on both layouts

The report ("the AI really struggles with de Konstruct Black - unnecessary slow
driving, and hitting a wall to full stop") is about **this** layout: Black is
`05_Track` forward on the disc's own titles, live on PPSSPP
([track.md](../formats/track.md#white-and-black)). Both symptoms were real
here: with no verified plan the lone Ace drove the corner model 11-23 % slower
than the plan now laps, and the field hit the first jump's lip and the pit's
dead end at full stop (8 / 3 / 3 / 9 dead stops by class, seed 1).

`ai_dekonstruct_symptoms_ground_truth.rs` counts, per craft, **dead stops** (a
wall-contact tick on which forward speed fell from 60+ to 15 or less within 20
ticks), **trough ticks** (on a clean flying lap, under 75 % of the speed the
plan's own verification lap did there) and the best clean lap against the
plan's. A lone Ace on either layout, every class: no dead stop, no trough tick,
best lap 0.0-0.7 % over the plan's. The field: one dead stop in eight cells
(`05_Track` RAPIER at the lip, 207). A Novice at VENOM clears the first jump.

At the lower levels there is no dead stop on either layout now; what slow
driving is left is the level's own handicap: on White (`21_Track`) a lone
Novice or Skilled at PHANTOM spends 1,725 and 1,171 ticks a run under 75 % of
the plan's pace, from `plan_margin`'s corner fractions (0.55 and 0.69) and
`pace_share`. That is a difficulty question, not a fault.

### Still open

- **The 428 pad on Black's upper road** pushes left on a crest: a lone Ace at
  RAPIER flies 43 units left at 157 u/s and touches a `Reset` sheet at 509-511,
  three times in 18,000 ticks - **all after the flag** (it finishes at tick
  6,570; the race keeps running under AI), so not in a race a player sees, but
  the same pad in traffic is untested.
- The field's remaining pit falls are craft arriving at the ramp at 91-108 u/s
  after contact on lap 1; the beneath rescue now puts them back.
- What the original does with a craft in this pit (a PPSSPP capture) was not
  measured.

## Tuning sweep tables

The evidence behind `oag_ai::Tuning`'s swept constants, moved here from the
field doc comments in `crates/ai/src/driver/tuning.rs`. Every number is this
project's own measurement of this project's own controller (chosen, not
measured against the original).

### `look_speed`

Swept by `sweep_look_speed` and `sweep_rate_gain` in
`crates/game/tests/ai_look_sweep.rs`: a lone Ace, twelve forward circuits,
18,000 ticks each, against the two committed field fixtures (tuned through
`Difficulty::tune` as `Race::start` does). `rate_gain` was swept first and
ruled out: at `10.0` and `20.0` the loop is already saturated at full lock, and
`opponent_weapons_ground_truth`'s floors fail (worst opponent shield `0.00`).

| value | 13 end shield | 07 end shield | solo total | field mean | field worst |
| --- | --- | --- | --- | --- | --- |
| 0.35 (old) | 6.52 | 0.00 | 705.68 | 0.90 | 0.70 |
| 0.33 | 17.86 | 0.00 | 774.28 | 0.86 | 0.75 |
| 0.32 | 24.17 | 0.00 | 788.97 | 0.83 | 0.54 |
| **0.30 (shipped)** | **39.07** | **0.00** | **863.10** | **0.73** | **0.00 (fails 0.45)** |
| 0.25 | 59.50 | 0.00 | 932.20 | 0.92 | 0.67 |
| 0.22 | 67.55 | 0.00 | 937.78 | 0.66 (fails 0.70) | 0.20 (fails 0.45) |
| 0.20 | 63.45 | 0.00 | 934.48 | 0.68 (fails 0.70) | 0.00 (fails 0.45) |
| 0.15 | 58.02 | 0.00 | 914.41 | 0.85 | 0.50 |

Re-swept 2026-09-07 after `oag_physics::pair::overlap`'s correction. The solo
columns (`13`, `07`, total) reproduce to the digit: a lone craft never touches
the narrowphase. The two field columns are a single seed (`SEED = 1`) and moved:
at `0.30` the corrected contact tie-break seats an opponent inside a Plasma
blast, so `worst` reads `0.00`. Over 16 seeds
(`crates/game/tests/weapon_floor_sweep.rs`, `sweep_worst_shield_over_seeds` and
`sweep_lap_completion_over_seeds`):

| value | mean-of-means shield | mean-of-worsts | seeds with a depleted craft | seeds with short opponent laps |
| --- | --- | --- | --- | --- |
| 0.30 | 0.68 | 0.32 | 2/16 | 2/16 |
| 0.22 | 0.72 | 0.38 | 2/16 | 2/16 |
| 0.20 | 0.71 | 0.30 | 3/16 | 3/16 |

No field column separates `0.22` or `0.20` from the shipped `0.30` any more, so
the original reason for `0.30` (distance from the field-floor cliff) is gone.
On solo evidence alone `0.22` is the best row. **The constant was not moved**:
the field board was the guard against a value that scores well solo and wrecks
a race, and nothing has replaced it for a tuning regression.
`opponent_weapons_ground_truth` is now deliberately coarse and is not an AI
tuning guard. `07_Track` ends at `0.00` shield at every value (per-lap loss
falls from 28-30 to 22-25 at `0.30`); that is the damage-charging model's
wall-grind, not convergence.

### `max_turn_rate`

Was `1.2`, which bound a real corner on Talon's Junction: the craft sat below
its speed target and never braked, yet drifted from 18 units inside the line to
30 outside. A 57-unit lookahead at 30 units off asks for about 2 rad/s, and the
clamp held it to 1.2 while the craft achieved 1.08. Swept on `16_Track`, a
minute a run, seven craft:

| value | worst excursion | mean off line | mean speed |
| --- | --- | --- | --- |
| 1.2 | 36 | 7.5 | 114 |
| 1.6 | 27 | 6.0 | 122 |
| **1.8** | **24** | **6.0** | **122** |
| 2.2 | 29 | 6.1 | 122 |

It flattens either side of 1.8: a constraint that has stopped binding. This is
a permission only. Until 2026-09-12 it was also the kinematic corner limit in
`pace::corner_target`, wrong for every craft (hull ceilings are 1.204 to 1.667
rad/s, `oag_physics::forces::YAW_INVERSE_INERTIA` and `<Turning amount>`); that
job is now `pace::hull_yaw_ceiling`'s. The flat table is evidence about the
permission, not that 1.8 was ever the right corner limit. See "The clean-Ace
board".

### `lateral_accel`

Was `55.0` until 2026-08-11, reported from play as "my craft is faster than the
AI": an opponent spent 45 per cent of a real race off the throttle. Swept on
`16_Track`, a minute a run, seven craft:

| value | mean speed | off throttle | furthest off the line |
| --- | --- | --- | --- |
| 55 | 90 | 45% | 28 |
| 130 | 113 | 16% | 32 |
| **180** | **116** | **8%** | 36 |
| 220 | 99 | 4% | 46 |

Re-swept at 260 on 2026-08-12. The first sweep ran on a build where craft fell
through the track (the respawn count did not respond to grip). With the hover
fixed (`oag_physics::hover::sweep`, `FAST_PROBE_SPEED`) all twelve circuits lap
cleanly. A lone Ace, every circuit, five minutes each:

| value | clean laps | recoveries | mean clean lap |
| --- | --- | --- | --- |
| 120 | 12 | 1 | 43.5s |
| 180 | 12 | 2 | 40.3s |
| **260** | **12** | **2** | **39.2s** |
| 340 | 12 | 8 | 39.1s |
| 440 | 12 | 5 | 39.2s |
| 560 | 11 | 6 | 38.1s |
| 700 | 11 | 9 | 38.0s |

260 is the knee: 340 buys 0.1 s and quadruples the recoveries. Lap time keeps
falling past 560 only because a recovered craft's lap does not count. Harness:
`sweep_grip` in `race_ground_truth.rs`, `#[ignore]`d and gated on `OAG_SWEEP`.

### `curvature_span`

`Line::max_curvature` is called with half the lookahead as its span, so at
80 units/s `Line::curvature`'s chord triple covers about 72 units. A shorter
corner is averaged with the straights either side: `07_Track`'s tightest arc
(about 50 units) reads 0.0155-0.0186 against a local 0.047, 2.5-3x under, with
its peak thirty samples early.

`sweep_curvature_span` in `crates/game/tests/ai_span_sweep.rs` reports two
boards. **Solo**: a lone Ace over twelve forward circuits, 18,000 ticks each,
shield retained rather than lap time (`07_Track` banks 49.8 s while shedding
34-35 shield on a wall). **Field**: the two committed field fixtures, with
`untimed` (opponents past lap 2 with no lap time, must be zero) and opponent
shield after a minute (floors 0.70 mean, 0.45 worst).

| span | solo total | resp | clean | mean lap | 07 laps | untimed | field mean | field worst |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 4 | 689.31 | 3 | 11 | 43.6s | 4 | - | - | - |
| 5 | 738.77 | 1 | 12 | 43.3s | 4 | 0 | 0.90 | 0.63 |
| 6 | 726.01 | 1 | 12 | 42.8s | 4 | 0 | 0.78 | **0.29** |
| 7 | 715.32 | 1 | 12 | 42.7s | 4 | 0 | 0.78 | **0.25** |
| 8 | 709.29 | 7 | 11 | 43.5s | 4 | **1** | 0.91 | 0.78 |
| 9 | 709.51 | 1 | 12 | 42.7s | 4 | 0 | 0.83 | **0.35** |
| 10 | 725.91 | 1 | 12 | 42.8s | 4 | **1** | 0.84 | **0.41** |
| **11** | **705.68** | **1** | **12** | **42.7s** | **4** | **0** | **0.88** | **0.64** |
| 12 | 704.18 | 1 | 12 | 42.6s | 4 | 0 | 0.90 | 0.63 |
| 13 | 675.59 | 1 | 12 | 42.5s | 4 | **1** | 0.91 | 0.81 |
| 14 | 688.94 | 1 | 12 | 42.4s | 4 | 0 | 0.83 | 0.45 |
| 15 | 683.72 | 1 | 12 | 42.4s | 3 | 0 | 0.87 | 0.69 |
| 16 | 669.70 | 1 | 12 | 42.2s | 3 | 0 | 0.84 | 0.57 |
| 18 | 649.39 | 1 | 12 | 42.2s | 3 | 0 | 0.93 | 0.78 |
| 20 | 660.42 | 1 | 12 | 42.2s | 3 | 0 | 0.88 | 0.77 |
| 25 | 620.39 | 1 | 12 | 42.0s | 3 | 0 | 0.86 | 0.76 |
| 32 | 620.28 | 1 | 12 | 42.2s | 3 | - | - | - |
| none | 613.52 | 1 | 12 | 42.2s | 3 | 0 | 0.83 | 0.50 |

Ten was chosen first on the solo board alone and turns two green field tests
red: a span can be clean on twelve solo circuits and still wedge a craft shoved
by seven others, which the ordinary `just` gate does not run. The criterion,
in order:

1. Hard gates: twelve clean circuits, solo respawns at most the baseline's 1,
   `13_Track` end shield above its 2.90 baseline, both field tests green. That
   admits `none`, 5, 11, 12, 15, 16, 18, 20, 25.
2. A band from the line's geometry. Below about 7 all three chords can fall in
   one path segment (every circuit has two seams shaped `0.30, 0.11, ~7.0`
   units, samples 0.89-1.77 apart), so sample spacing reads as curvature. Above
   about 15 the triple (`3 * span`) cannot resolve `07`'s 50-unit arc and `07`
   dies on lap 3 again. That leaves 11 and 12.
3. Maximise solo total: 11 over 12 by 1.5 shield on a board of twelve, which is
   a tie-break and nothing more.

11 and 12 pass the field gate because a craft happened not to wedge. The field
worst clusters (`{6,7}` 0.25-0.29, `{9,10}` 0.35-0.41, `{5,11,12}` 0.63-0.64):
one craft switching basins, and the basin is not a property of the span. `None`
is kept so the uncapped row stays re-runnable.

Recorded 2026-09-12: every row above evaluated `pace::corner_target`'s
kinematic limit at `max_turn_rate`'s 1.8, now the flown craft's own hull
ceiling, so the sweep measured a different function from the shipped one.
Nothing is broken (both field tests were green at eleven, checked), but the
choice wants re-establishing with the estimator change the Outpost 7 thread
names.

### `curvature_chord`

Separating the chord from the walk step is the only untried axis on the
estimator: the span's value was swept three times, and it was never known
whether a short chord costs resolution or sampling density. Swept 2026-09-12 on
the current tree with `sweep_curvature_span`'s field columns (`worst` is
`opponent_weapons_ground_truth`'s floor, `0.45`):

| chord | step | solo total | respawns | clean | mean lap | field worst |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| **11** | **11** | **907.97** | **1** | **12** | 43.6s | 0.57 |
| 4 | 4 | 860.75 | 3 | 11 | 44.4s | 0.80 |
| 6 | 6 | 830.42 | 2 | 11 | 43.6s | 0.74 |
| 8 | 8 | 827.14 | 11 | 10 | 44.6s | **0.38** |
| 4 | 11 | 844.01 | 5 | 11 | 44.7s | 0.54 |
| 6 | 11 | 815.53 | 10 | 10 | 44.4s | 0.59 |
| 8 | 11 | 798.39 | 10 | 10 | 44.4s | 0.63 |

Every shortening is worse than `11`, coupled or decoupled, so the short chord's
cost is resolution, not sampling density. Decoupling repairs one thing (chord 8
at step 11 lifts the field floor from `0.38` to `0.63`) and buys nothing solo.
This retires the chord-length theory of the understatement. The defect is real
(the windowed reading is 1.66x/1.85x under the true apex and reports a hairpin
as opening at full lock into a wall), but neither knob fixes it. What is left
is a different estimator. See `Line::max_curvature_stepped`.

### `trail_deadband`, `trail_gain`, `trail_saturation`

All three moved on 2026-09-11 (`trail_deadband` `0.15` to `0.05`, `trail_gain`
`1.2` to `3.0`, `trail_saturation` `0.85` to `0.7`). At the old values the
deadband sat only `0.02` rad/s below the turn-rate error that `0.85 /
rate_gain` (`0.17`) implies, so almost nothing was left to turn into a command:
on a live `--race --autopilot` run of Talon's Junction's U-turn the
differential peaked at 7.67 of the airbrake's `0..=100` range, 19 engaged ticks
at mean magnitude 4.6. Three `trail_saturation` points on the same run and the
real-disc regression
`opponent_weapons_ground_truth::a_field_racing_with_real_pads_does_not_mine_itself_to_death`:

| `trail_saturation` | engaged ticks | mean magnitude | mean-of-means energy | depleted |
| --- | --- | --- | --- | --- |
| `0.85` (old) | 19 | 4.6 | 0.67 | 0 |
| `0.6` | 58 | 23.9 | 0.58 | 3 |
| **`0.7`** | **41** | **25.2** | **0.67** | **1** |

`0.6` engages longer and no stronger but costs the field test's energy floor.
`trail_deadband` and `trail_gain` hold the magnitude in the 20s at every point;
only saturation decides how much of the corner reaches it. Zero respawns on
the live run at every point.

### `trail_peak_decay`

`1.0` latches `Driver::peak_curvature` for the whole race, which was a bug:
`pace::track_peak_curvature` resets it when curvature falls to
`trail_curvature_floor`, but `07_Track`'s smallest windowed curvature is
`0.00127` against a floor of `0.00100`, so the reset never fires. The mark
became a running maximum latching on the tightest corner, the exit gate then
rejected 74.8 % of ticks and the differential fired on 3.78 %. A leak makes
"exit" mean "opened up since the tightest point recently" (at `0.99` the mark
halves in about 69 ticks).

It ships at `1.0` anyway: the leak works as a mechanism and does not pay.
`sweep_trail_peak_decay` over the 48-row board, lone Ace, charge capped at each
row's pool:

| decay | contact ticks | charged | end pool | respawns | eliminated | clean laps | mean lap |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| **1.0** | **10,924** | **2,277.3** | **2,191.6** | 35 | **11** | 44/48 | 38.88s |
| 0.998 | 14,228 | 2,373.4 | 2,057.1 | 40 | 10 | 44/48 | 38.95s |
| 0.995 | 13,902 | 2,317.3 | 2,129.6 | 34 | 12 | 43/48 | 38.76s |
| 0.99 | 14,973 | 2,339.5 | 2,112.7 | 34 | 12 | 43/48 | 38.72s |
| 0.95 | 13,917 | 2,302.2 | 2,181.8 | 34 | 11 | 45/48 | 38.72s |

Every leak value costs contact ticks and end shield and buys 0.16 s of mean
lap. On `07_Track` the differential goes from 3.78 % to 11.58 % of ticks and the
board gets worse. The differential is spent reactively (once
`trail_saturation` says steering ran out of authority) and `pace::trail` "cuts
lateral grip exactly as hard as holding both sides would", so spending it
without banking the higher corner speed it permits (`v = omega_steer / (k - C)`,
which `pace::corner_target` does not do) is pure loss. `1.0` is not an
endorsement of the latch: it is the value that changes no behaviour until the
differential becomes a plan.
