# Leaving the track: where craft do it in this engine, and what was ruled out

2026-09-29. Started from a maintainer report, made by feel and unmeasured:
**it is easier to fall off the track here than in the original (Pulse PSP)**,
no specific circuit, with "flying too high?" and "collision less forgiving?" as
the guesses. This page is the measurement pass. **Nothing was tuned and no
physics changed**; the one candidate fix that was tried broke the regression
gate and was reverted (see [the sunk craft](#the-one-divergence-the-original-recovers-a-craft-that-has-sunk-into-the-floor-and-ours-does-not)).

Every number below is reproducible from `crates/game/tests/falloff_survey_ground_truth.rs`
and three files in `crates/trace/tests/` (all `#[ignore]`d, `OAG_SWEEP`-gated where
they sweep, printing rather than asserting). Original-side comparisons exist for
**Talon's Junction (`16_Track`) captures only**, plus a handful of live
`place` experiments on Moa Therma (`03_Track`); every other circuit is ours alone.

## Three of the four rescue triggers are this project's inventions

A "fall-off" in our engine is one of four things firing, and only the first is the
original's: an authored `Reset` volume (`RespawnCause::ResetZone`), or one of three
dwells this project invented - `LostCircuit` (opponent 8 half-widths from its own
driver's sample for 90 ticks), `Stalled`, and the player's `OffTrack` (2
half-widths from the nearest sample for 45 ticks). See `oag_race::recovery`. The
cause of the most recent respawn is now recorded per craft
(`Race::last_respawn_cause_of`, bookkeeping outside `state_hash`), and a fifth
value, `Destroyed`, separates a wreck from a fall - without it the `Wrecked` rows
below read as falls. `Race::respawn_given_up_of` exposes the five-in-a-row give-up.

## The survey

One craft alone, 18,000 ticks, all 24 circuit-directions, all four tiers, `VENOM`.
Two populations, labelled separately because they hit different triggers: the lone
opponent (slot 1) and slot 0 flown by the same driver (the player's rescue path).
Each event is classified at the **first tick of departure**, not at the respawn tick
(a rescue fires 45-90 ticks after the craft left; `ai.md` records two conclusions
reached backwards from the rescue that were both wrong).

1,098 rescue events in total. **About 980 are Novice on `13_Track`/`29_Track`**,
the known `grip_believed` jump pathology (`ai.md`, "The jump-clearing failure"),
and are left out of the headline. The other 117:

| where | what | count (tiers x populations) |
| --- | --- | ---: |
| `17_Track` (`01_Track` reversed), spline index ~900-910, section 11 | authored `Reset` volume touched in mid corridor at 16-28 u/s, **every lap, every tier, both populations** | 65 |
| `01_Track`, spline index ~31, section 18 | the same fall, on the forward direction; the crossing is made slowly (~30 u/s) and the craft drops through the hole under samples 31-42 into a `Reset` volume | 21 (Skilled 11, Novice 8, Elite 1, Ace 1) |
| grid slot 1 on `01_Track` and `17_Track` | **airborne from tick 0**, see below | 8 |
| `Wrecked` (shield ran out; `07`, `23r`, `05`) | not a fall, separated by the new `Destroyed` cause | 11 |
| `05_Track` index ~1210-1265 (dive/jump) | invented `LostCircuit`/`OffTrack` | 4 |
| `05_Track` index ~1650 | invented `Stalled`, beached at a wall | 5 |
| `06_Track` | invented `LostCircuit`, Novice | 2 |

**The regression gate does not see the largest row.** `a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`
runs the twelve forward circuits only, and `17_Track` is `01_Track` reversed. It
also does not see the second row at Ace: an Ace craft crosses the `01_Track` hole at
speed and clears it; Skilled and slower do not.

**Ace is clean almost everywhere.** That is the known result and it is the reason
this pass had to look past the AI: the maintainer is a human, and a clean driver
cannot show whether a small error becomes a fall.

### The gate's own `01_Track` respawn (index 794) is not a fall-off

`01_Track` and its reverse spawn **the odd grid column (slots 1, 3, 5, 7) at -1.45 to
-1.59 half-widths**, past the corridor edge. Those craft are airborne from tick 0 and
are put back by the `LostCircuit` dwell at tick ~250 (a 4-second fall, 195 u/s).
Every other circuit's eight slots sit inside the corridor (worst margins, 0-based slot: `04_Track`
slot 2 at 0.96, `20_Track` reversed slot 2 at 0.95, `13_Track` slot 1 at 1.01 - on the
edge, not off it). The gate's lone craft is slot 1, so its one `01_Track` respawn is this.
It is **not player-facing** (slot 0 is on the even column and every slot-0 spawn is
inside the corridor). It is in `crates/game/src/race/spawn.rs` (`grid_poses` carries
the authored node's offset from the centreline through the walk and adds the
20-unit odd-column stagger on top of it). The stagger and pitch were measured off the
original on `16_Track` only, and `Race_ComputeGridLayout` (`0x0882b3b0`) may scale or
clamp the column by track width. **Fixed 2026-09-29** (`grid_stagger` lane): the original lays every slot at the AI
corridor's midpoint plus or minus 10, node's side first, and this project carried the
node's offset and staggered one fixed way. See
[grid.md](../ghidra/functions/psp-pulse-usa/grid.md#the-stagger-is-about-the-corridor-midpoint-not-the-node-2026-09-29).
The gate's `01_Track` respawn at 794 is gone; the slot-1 start moved by about 1.8
laterally on every circuit, and with it `05_Track` (0 to 3 respawns, ticks
4144/11104/13324: `LostCircuit`, `LostCircuit`, `Destroyed`, all far from the grid)
and `07_Track` (index 687 to one `Destroyed` at 1137) - the hazards the table above
already lists, reached by a different trajectory, not new ones.

## Discriminators, each answered yes or no

| | question | answer |
| --- | --- | --- |
| (a) | Do we ride higher than the original, at rest or at speed? | **No.** At rest the front probe height is 2.888 on both (0.000 apart). At speed on the straight (`talons-junction-standing-start.csv`, 105+ u/s, no pads) the two are 0.015 apart. On the whole lap ours sits *lower* (0.1-0.28 front, 0.06-0.15 rear over 20-60 ticks after a reseed) - the wrong sign for the guess, and not attributed: that replay has no pad locator, so it also drifts 9 u/s slower over a window, and it seeds no landing timer. `steady_state_vertical_ground_truth.rs` walks the recorded poses with the derived timers and finds a vertical residual of +0.6 to +0.75 u/s^2 in **every** speed bin including rest (about 0.01 units of ride height at the spring gain) - flat in speed, so there is no speed-dependent vertical force missing. |
| (b) | Do we fly higher or longer over a crest? | **No**, on the two crests the captures hold: 14 vs 14 ticks airborne and 13 vs 13, apex over the surface 8.54 vs 8.74 (ratio 0.98) and 8.29 vs 8.23 (1.01), seeded once two ticks before liftoff and never reseeded. Scope: two crests, one circuit. |
| (c) | Is there containment at the edge that we lack? | **Open, and it is the key measurement.** Statically, leaks follow missing walls: `edge_profile` finds 100% walled sides on `16`, `32`, `21`, `22`, `01` (both directions), and 82-89% on `04`, `05` (right side 82%), `07`, `10`, `14`, `23`, `26`, `30`; the shove sweep (below) leaks 0/492 on `16_Track` and 12-21% on the second group. The correlation is loose (`03_Track` has 10% open edge on one side and leaks 2.5%), which is why (c) stays open. The PSP disc ships **no `Cage Collision` node at all** and none is reachable from any race circuit on either pressing (`collision.md`), so there is no authored containment the loader could be dropping; every triangle decoded reaches the collision world (`ai.md`). What the original does at an open edge is not measured: see the next measurement. |
| (d) | Do we push through walls, or kick harder? | **No.** Of 11,772 shove trials, **3** cross a `Wall` triangle between consecutive positions (all on `04_Track`, none of which leaves the track). The scrape loss (3.5% per frame floor) reproduces on the standing-start capture (`oag-trace.md`), and our hull meets a wall 3-5 ticks *later* than the original's, which is the forgiving direction. Walls hold; what leaks is an edge with no wall. |

### The shove sweep

`shove_sweep`: the player's craft on the racing line at 40 sampled points per circuit,
turned 10, 25 or 45 degrees to either side, at 110 or 190 u/s, throttle held and
nothing steered, 200 ticks. It is a stress test of containment, **not a driver
model**. 11,772 trials; `left` = respawned or beyond 1.3 half-widths for 30+ ticks
(the half-widths are the AI corridor's, not the collision mesh's, so 196 of the 910
that left never respawned and may be drivable runoff).

| circuit | left / trials | | circuit | left / trials |
| --- | ---: | --- | --- | ---: |
| `16_Track` | 0 / 492 | | `20_Track` reversed | 92 / 492 |
| `06_Track` | 3 / 492 | | `26_Track` reversed | 78 / 492 |
| `32_Track` reversed | 3 / 492 | | `23_Track` reversed | 80 / 492 |
| `01_Track` | 14 / 492 | | `10_Track` | 77 / 492 |
| `04_Track` | 102 / 492 | | `07_Track` | 72 / 492 |
| `05_Track` | 68 / 492 | | `30_Track` reversed | 60 / 492 |

Full 24-row table in the scratch report; 591 of the 910 leaks touched no wall at all
(an open edge), 319 touched one first. Leakage is independent of angle and speed to
first order: it is the edge, not the approach.

## The one divergence: the original recovers a craft that has sunk into the floor, and ours does not

Found while trying to get an original-side shove on `03_Track` (Moa Therma), the
open-edge spot the sweep flagged (spline index 1780, 12 of 12 trials leave, no wall
touched). Placing our craft there with `psp-drive.py place` put it in a pose whose
centre is 0.48 above a `Floor` face (the spline runs 2.4 below the floor there), the
probes are under the surface, and:

- **ours** free-falls through the floor for the rest of the run (the downward hover
  ray starts below the surface and finds nothing; `hover::sweep` needs a segment that
  crosses a face);
- **the original** is pushed up 9.3 and back at 74 u/s within 60 ticks.

Made clean by a control pair on `03_Track` index 200 (level `Floor`, speed 0, 60-tick
settle, same pose fed to both): the un-sunk pose settles at y 5.11 in the original and
hovers in ours; the same pose **sunk 3.6 units** settles at **y 4.64 in the original**
(speed 0.5, back at rest height) and falls to y -155 in ours.

`hover::sweep`'s own doc says "Nothing in the original does this". The original does
recover this state, by some mechanism. The ray in `Ship_CastHoverProbes` starts at the
probe in the original as in ours (`engine.md`), so it is not a lifted ray.

A candidate was tried: letting the hull's ten-ray star respond to `Floor` and
`MagFloor` as well (`wall::responds` returning true). It recovers the sunk craft **to
y 4.65 by tick 5** (the original's 4.64) and then follows the un-sunk trajectory. It
**also fails the gate**: `01_Track` loses its clean lap (lost at `[794, 71, 72, 72, 75]`),
so the real mechanism is narrower than "the hull responds to every floor". Reverted,
not shipped. The address to start from: `Collision_BoxAgainstMesh` (`0x08815cd4`) and
`Collision_StepNarrowphase` (`0x088159c0`) for whether and how the ship's box collider
is filtered by surface type, `contact-response.md` (a floor contact takes friction
`0.0`, which says floor contacts reach the resolver), and `Ship_HoverTwoPoint`
(`0x0884a658`) for a recovery at `h < 1.0` with a probe already below the face. That is
deep Ghidra work and is the follow-up this pass recommends.

Whether this is what a human feels is **unmeasured**. It fits the report only if craft
ever get a probe under a floor face in normal play (a steep landing, a crest lip) and the
original holds them where ours drops them.

## What was tried on the original and did not give a clean answer

`psp-drive.py place` teleports a craft with a heading and a speed, and `psp-trace.py`
records it. A **shove** by that route (a craft placed at an angle to a wall at 110 u/s)
is not usable: on both `16_Track` (control) and `03_Track` the original's craft is
already upside down (`up.y` -0.9) by the first recorded tick, so what follows is a
tumble and not a wall test. A placement **at rest with a 60-tick settle** is clean, and
is how the sunk-craft result above was obtained. Only `16_Track`, `03_Track` and
`18_Track` (Metropia reversed) are reachable from the front end (`race-setup.md`).

## Ruled out

- **Flying too high**: (a), (b).
- **Walls that let craft through**: (d).
- **Authored containment we drop**: no `Cage` on the PSP (and none reachable on the PS2);
  the plugin's `collisionCageEnabled` attribute is present on 11 of the 24 `PI_Track`
  entries, both walled and leaky circuits, and is read by `FUN_088c3f7c` (`0x088c40a8`)
  at the definition parse; what it gates was not followed.
- **The force law at speed**: the vertical residual is flat in speed.

## Next measurements

1. The sunk craft: which original mechanism recovers it (Ghidra, addresses above),
   then the narrowest port that also keeps `01_Track` clean. *Opus follow-up.*
2. An original-side approach on an open edge, driven rather than teleported: scripted
   steering out of the line at `03_Track` spline index 1780 or `18_Track` index 73/2044,
   through `just scripted-emu`, replayed through `oag-trace run` for the pair.
3. ~~The `01_Track` grid~~: done 2026-09-29 by rule, not capture - `01_Track` is not
   reachable from the front end; Metropia reversed was read instead (`grid.md`).
4. A perturbed-input survey (a lateral shove on a live lap) once (1) or (2) says what the
   original does; the shove sweep above is the cheap static form of it.
