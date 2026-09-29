# Leaving the track: where craft do it in this engine, and what was ruled out

2026-09-29. Started from a maintainer report, made by feel and unmeasured:
**it is easier to fall off the track here than in the original (Pulse PSP)**,
no specific circuit, with "flying too high?" and "collision less forgiving?" as
the guesses. This page is the measurement pass. **Nothing was tuned and no
physics is changed on `main`.** The one divergence it found, a craft sunk into
the floor, was read out of the binary afterwards, ported on a branch, and held
back because the port strands a craft below two holes in the AI's racing line
(see [the sunk craft](#the-one-divergence-the-original-recovers-a-craft-that-has-sunk-into-the-floor-and-ours-does-not)).

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
clamp the column by track width. **Not fixed here**: it changes the number the gate
quotes.

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
open-edge spot the sweep flagged (spline index 1780). A craft whose centre is
above a floor but whose hull is through it - probes under the surface, so the
downward hover rays find nothing - **fell through the floor in ours** (y -155)
and was **pushed back to rest height in the original**.

### What the original does (2026-09-29, read and measured)

Three mechanisms, all read out of the binary; the addresses and confidences are
on [collision.md](../ghidra/functions/psp-pulse-usa/collision.md#every-mesh-surface-reaches-the-hull-narrowphase-2026-09-29):

1. **The hull's ten-ray star makes contacts against every mesh surface**, floors
   included (`Collision_BoxAgainstMesh` `0x08815cd4` reads no surface type;
   confidence 88). A floor contact translates the body out by its depth and
   applies the restitution impulse, but **never damages**: `FUN_088418e0` charges
   only a positive-friction contact (`0x08842648`, confidence 90), and a floor's
   friction is the `-1.0` sentinel.
2. **`Collision_AddContact` keeps a contact only when the sample point's
   perpendicular projection lands in the same triangle** the centre segment
   crossed (`0x08816864` into `Collision_SegmentTriangle` `0x08818bdc`,
   confidence 90).
3. **`Body_StepWorld`'s pass 1** clips the body back along its velocity by 0.9 of
   the overshoot when a segment from the centre to the box face, plus this frame's
   travel, meets any mesh surface (`0x0884f70c`, confidence 85).

Measured by placing the same pose in both engines (the clean placement method is
below), coasting, 200 ticks:

| case | original | ours (main) | (1) only, `sunk-craft-floors` | prototype (1+2+3) |
| --- | --- | --- | --- | --- |
| `16_Track` idx 200, 3.6 into the floor | +1.81 on the first tick (two corners), then climbs; rest y -39.60 | falls through | +3.54 in one tick, rest -39.61 | tick for tick (first tick -41.26 vs -41.28, tick 30 -39.17 vs -39.16) |
| `03_Track` idx 200, 3.6 into the floor | pass 1 lifts +0.757 (predicted 0.752), probe escapes +1.82, rest y 4.67 | falls through | +3.75 in one tick, rest 4.66 | rest 4.663; the gate alone (no pass 1) falls through |
| `16_Track` idx 200, on its flank on the floor | **stays on its flank** and slides; tick 199 (323.57, -47.49, -140.96) | 1.6 lower than the original by tick 60, tick 199 (323.31, -45.15, -143.92) | tick 199 (323.63, -47.50, -140.95) | tick 199 (323.42, -47.51, -141.05) |
| unsunk controls, both circuits | hovers still | hovers | hovers | hovers |

**Nothing of it is merged.** (1) alone is on branch `sunk-craft-floors`; (2) and
(3) are a prototype on `sunk-craft-parked`. The reasons, measured:

- **(1) alone strands a craft, the player included.** `01_Track`'s racing line
  drops through the hole under samples 31-42 and `06_Track`'s through the one
  under 1196-1200 (`race_ground_truth.rs`'s "nothing under the line" table). A
  craft falling through rolls onto its flank on the way down and lands on the
  floor below. Before, it sank on through that floor (into a `Reset` volume on
  `01_Track`); with (1) it is held there on its flank - which is what the original
  does to a flank-down craft, measured above - and nothing rights it. An opponent
  is freed after 20 s by this project's invented `Stalled` dwell. **The player has
  no such dwell and stays there**: the player's autopilot on `01_Track` completes
  1 lap in 18,000 ticks instead of 4, airborne 15,946 of them, and a scripted
  escape attempt from the beached pose (900 ticks each of throttle, throttle with
  full left or right, both airbrakes, and left-right-left / right-left-right roll
  taps with and without throttle) never rights it; holding a steer only scoots it
  along its flank at 4-13 u/s. That is a softlock on a selectable circuit.
- **(2) without (3)** strips wall contacts that the original covers with pass 1
  (`05_Track`'s lone Ace: 60 wall-contact ticks to 2,475, airborne 782 ticks to
  7,365) and leaves `03_Track`'s sunk craft falling.
- **(1)+(2)+(3)** matches every placement trial but costs `01_Track` its clean lap
  in the regression gate, for the same beaching.

So the prerequisite for any of it is the two holes under the line: an AI line
that no longer drops into them, or whatever the original does there (it is not
known whether the original's craft rolls onto its flank at that lip). Whether to
add a player-side rescue is a design decision for the maintainer, not a port -
the original has none.

### What (1) did, measured on `sunk-craft-floors`

- **Regression gate**, all twelve clean. `01_Track`: before
  `respawns 1 lost at [794]`, with (1) `respawns 3 lost at [794, 72, 71]`. 794 is
  the grid-slot spawn fall; 72 and 71 are the `Stalled` dwell after about 20 s
  beached. `07_Track`'s one respawn moved from index 687 to 696.
- **Survey** (`VENOM`, lone AI, Ace and Novice, all 24 circuit-directions,
  18,000 ticks, `--release`): rescue events 583 to 267. `13_Track` Novice
  345 to 26 (a craft that lands short of the jump now has its hull held by the
  far floor's lip), `06_Track` Novice 2 to 0, `01_Track` Ace 1 to 3 (the two
  `Stalled`), `17_Track` Novice 2 to 4, `05_Track` Novice 3 to 4, `29_Track`
  Novice 214 to 216.
- **Shield**, clean-lap board (`VENOM`, lone Ace), end-of-run pool and per-lap
  charge: identical on ten circuits; `01_Track` end 70.62 to 73.48, per-lap
  4.0/4.2/3.3 to 3.5/4.2/12.4 (the beached lap); `10_Track` end 87.40 to 94.24;
  `06_Track` 78.81 to 77.89; `07_Track` 79.94 to 78.46.
- **`ai_clean_lap_gate`** (48 rows, all classes): `01_Track` FLASH `CleanLap` to
  `NoCleanLap` (beached every lap), `06_Track` PHANTOM contact ticks 493 to 1,586
  (beached four times in its hole), `05_Track` FLASH `Died` to `CleanLap`,
  `10_Track` VENOM 0 to 5 contact ticks, the rest within a few ticks.
- **`off_track_rescue_ground_truth`** loses its subject: the player's autopilot
  no longer leaves `05_Track` at index ~1225, and no forward circuit sends it off
  through `OffTrack` any more. Not re-pointed.

## Placing a craft in the original: the clean method

`psp-drive.py place` writes the body once at a `Ship_UpdateCraft` breakpoint and
resumes, and `psp-trace.py` attaches afterwards. **Both halves corrupt a
placement trial**, and every original-side result above this section's date
that used them is confounded:

- the emulator runs free between the two, so the first recorded tick is several
  frames after the write (the sunk `16_Track` trial's first record was already
  5 units up and upside down);
- one write leaves per-craft state derived from the old pose (the hover probes'
  cached hits among it): on an **unsunk** control at rest the first frame after
  the write kicks the craft **76 u/s upward** and rolls it.

So the survey's "a teleported shove tumbles the original's craft (`up.y` -0.9)"
is this artefact, not the original's response to a shove, and its sunk-craft
"y 4.64 after a 60-tick settle" was only confirmed by the clean method below.

**The clean method**: write the pose on four consecutive hits of the followed
craft (so the cached state re-derives at the new pose, with the velocity zeroed
each time), then log every following hit inside the same breakpoint session. The
unsunk control then hovers perfectly still (y -39.55 for 60 ticks). The scratch
script was `data/scratch/sunk-craft/place_trace.py`; folding a `--rewrites` and a
per-tick log into `psp-drive.py place` is the obvious follow-up. Even so, the
sunk `03_Track` trial showed one unexplained frame of upward velocity (+1.9) on
the first free tick; compare from the first frame whose velocity points down.

## Ruled out

- **Flying too high**: (a), (b).
- **Walls that let craft through**: (d).
- **Authored containment we drop**: no `Cage` on the PSP (and none reachable on the PS2);
  the plugin's `collisionCageEnabled` attribute is present on 11 of the 24 `PI_Track`
  entries, both walled and leaky circuits, and is read by `FUN_088c3f7c` (`0x088c40a8`)
  at the definition parse; what it gates was not followed.
- **The force law at speed**: the vertical residual is flat in speed.

## Next measurements

1. The sunk craft: land the projection gate and pass 1 together (above), once
   `01_Track`'s line stops dropping into the hole at samples 31-42.
2. An original-side approach on an open edge, driven rather than teleported: scripted
   steering out of the line at `03_Track` spline index 1780 or `18_Track` index 73/2044,
   through `just scripted-emu`, replayed through `oag-trace run` for the pair.
3. The `01_Track` grid: read the original's eight grid positions on that circuit
   (a PPSSPP memory read, as `grid.md` did for `16_Track`).
4. A perturbed-input survey (a lateral shove on a live lap) once (1) or (2) says what the
   original does; the shove sweep above is the cheap static form of it.
