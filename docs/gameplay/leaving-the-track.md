# Leaving the track: where craft do it in this engine, and what was ruled out

2026-09-29. Started from a maintainer report, made by feel and unmeasured:
**it is easier to fall off the track here than in the original (Pulse PSP)**,
no specific circuit, with "flying too high?" and "collision less forgiving?" as
the guesses. This page is the measurement pass. **Nothing was tuned.** The one
divergence it found, a craft sunk into the floor, was read out of the binary
afterwards and, with the original's own rescue for a beached craft measured
live, **ported** (see [the sunk craft](#the-one-divergence-the-original-recovers-a-craft-that-has-sunk-into-the-floor-and-ours-does-not)
and [what landed](#what-landed-2026-09-29-branch-sunk-craft-2)).

Every number below is reproducible from `crates/game/tests/falloff_survey_ground_truth.rs`
and three files in `crates/trace/tests/` (all `#[ignore]`d, `OAG_SWEEP`-gated where
they sweep, printing rather than asserting). Original-side comparisons exist for
**Talon's Junction (`16_Track`) captures only**, plus a handful of live
`place` experiments on Moa Therma (`03_Track`); every other circuit is ours alone.

## Which rescue triggers are the original's

A "fall-off" in our engine is one of five things firing. Two are the original's:
an authored `Reset` volume (`RespawnCause::ResetZone`) and, since 2026-09-29, four
seconds with no hover probe touching anything (`RespawnCause::Airborne`,
`FUN_088418e0` at `0x08841d30`). Three are dwells this project invented -
`LostCircuit` (opponent 8 half-widths from its own driver's sample for 90 ticks),
`Stalled`, and the player's `OffTrack` (2 half-widths from the nearest sample for
45 ticks). See `oag_race::recovery`. **The original has a distance trigger too**,
read but not ported: more than 200 units from the craft's tracked spline position
for half a second (`0x08841cec`), which is where our invented dwells stand; and a
fourth caller of the same reset state whose test (`FUN_0883191c`) is not read.
All four are on [shield.md](../ghidra/functions/psp-pulse-usa/shield.md#state-3-is-the-reset-and-four-things-enter-it-2026-09-29).
(This section used to say three of four triggers were inventions: the airborne
and distance triggers were not known.) The
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
inside the corridor). It is in `crates/raceplay/src/spawn.rs` (`grid_poses` carries
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

**Held back first (2026-09-29, morning).** (1) alone was on branch
`sunk-craft-floors`, (2) and (3) a prototype on `sunk-craft-parked`, because
(1) stranded a craft - the player included - on its flank below the drop under
`01_Track`'s samples 31-42, with no input that righted it and no rescue this
project knew of. (2) without (3) strips wall contacts the original covers with
pass 1. What resolved it is measured below: the original has a rescue for
exactly that pose, and its AI never reaches the lip slowly enough to need it.

### The lip under `01_Track`'s line, measured in the original (2026-09-29)

**It is not a hole in the floor.** Casting straight down across the corridor
at every sample 20-59 (`01_Track`, this project's loader):

- samples 20-31 run on an **upper deck**, `Floor` at y 24.3 across the whole
  corridor, the line three units above it;
- from sample 26 the line pitches down about 60 degrees (the samples' own up
  vector reaches `y 0.46`) and runs **through the air** to a **lower floor** at
  y 1-2, which it meets around sample 42 - so a craft following the line leaves
  the deck at its lip and drops about 23 units;
- under the drop the disc authors a four-triangle `Reset` collider (collider
  136) sloping from y 20 down to y 2, covering the corridor from about 30 left
  of the line to about 10 right of it: a craft that falls short and steep
  touches it and is reset;
- right of the line a strip of lower floor at y 6-7 runs under the deck and
  past the drop, outside that `Reset` slope. That strip is where the craft
  that fell was held on its flank.

`06_Track`'s "hole" at 1196-1200 is a different thing: a path join where the
line switches from an upper floor at y 12.6 to a lower one at y -11.7, with the
upper floor ending around sample 1199; no `Reset` there. The original's
behaviour there was not measured.

**Every circuit is reachable in PPSSPP** with the dev-unlock byte
(`*(u32 *)0x08b31774 + 0x45f`, [ppsspp-debugger.md](../reverse-engineering/ppsspp-debugger.md#every-circuit-not-three-the-dev-unlock-byte-2026-09-29)),
and Basilico Black is `01_Track`. Two measurements, both driven or coasted in
the running original, neither teleported mid-flight:

1. **The original's field flies the lip.** A Single Race, eight craft, logged
   free-running at about 10 Hz for three minutes (the racer table at
   `0x08b34420`): **21 of 21 AI crossings** reached the lip at **69-111 u/s**,
   left the deck ballistically - up to 20 units off the line, which dives
   under them - and landed upright about 35 frames later at samples 69-87,
   5-19 units off the line. None fell steeply, beached, or was reset.
2. **A slow craft in the original.** The player's craft in a Time Trial,
   placed with the four-write method on the flat upper deck at sample 10
   (upright, at rest height, facing along the line) and then left to coast
   off the lip, no input, 400 ticks:

   | speed at placement | what the original did |
   | --- | --- |
   | 45 u/s | tumbles to `up.y 0.81` in the air, lands upright on the lower floor at sample 57-62, comes to rest there |
   | 30 u/s | tumbles to `up.y 0.17` - nearly on its flank - lands, and **rights itself** by tick 200; comes to rest upright at sample 65 |
   | 20 u/s | lands **upside down** on the lower floor at sample 57; its airborne clock runs from tick 133; at tick 372 it passes 4.0 s and the craft is **reset**: relocated upright at sample 50, five up, 12.5 off the line, moving at 53.5 u/s |

So the original **does** have a rescue for a beached craft, and it applies to
the player: a craft whose hover probes touch nothing for four seconds is put
through the same reset state a `Reset` contact enters (`FUN_088418e0` at
`0x08841d30`, confidence 90; the full reading, and the other two triggers
into that state, are on
[shield.md](../ghidra/functions/psp-pulse-usa/shield.md#state-3-is-the-reset-and-four-things-enter-it-2026-09-29)).
An upside-down or flank-down craft's probes point away from the floor, so it
counts as airborne.

### What landed (2026-09-29, branch `sunk-craft-2`)

- **(1)+(2)+(3) together**, unconditionally: floor contacts, the projection
  gate, and pass 1 (`crates/physics/src/wall.rs`, `wall/clip.rs`). Pass 1
  replaces the swept centre ray this crate cast after the move as its own
  tunnelling guard. The physics determinism reference was re-recorded for pass
  1 alone (removing its one call reproduces the old hashes bit for bit).
- **The airborne reset**, ported: `oag_race::recovery::AIRBORNE_RESET_SECONDS`
  (4.0) and `RespawnCause::Airborne`, for the player and for opponents, through
  the same `Race::respawn` a `Reset` contact uses. Three recorded departures:
  our respawn puts the craft at rest on the racing line (the original: corridor
  midpoint, five up, launched at `+0x78c * 0.25 + 50`), charges the player no
  shield (the original: up to 5), and the original's third trigger into the
  same state - more than 200 units from its tracked spline position for 0.5 s
  (`0x08841cec`) - is not ported; the invented `LostCircuit`/`OffTrack` dwells
  stand where it would.
- **The AI no longer brakes for a bend over a gap** - chosen, not measured
  (maintainer decision: opponents obey the player's physics and drive smarter).
  The racing line carries a mask of samples with no track under them, and a
  curvature reading whose chord touches one reads straight. Read in 3D, the
  lip's pitch was a 0.075 rad/unit "corner", and our Ace braked from 127 u/s to
  about 20 and crawled off the lip every lap; it now crosses at 76-81 u/s and
  lands upright near sample 75. A first version that removed the pitch
  component from *every* reading took every crest faster and turned three
  `ai_clean_lap_gate` rows from a clean lap to a wreck; it was replaced.

Measured on the landed tree, against `main` at `091bfe46`:

- **Same-pose trials** in ours (`01_Track`, the original's placement at sample
  10, coasting): 45 u/s rests upright at (-164.2, 6.13, -418.0) against the
  original's (-163.6, 6.13, -418.2); 30 u/s tumbles to `up.y 0.15`, rights
  itself and rests at (-165.0, 6.12, -418.3) against (-166.9, 6.12, -419.1);
  20 u/s lands upside down and is reset by the airborne clock at tick **371**
  against the original's **372**. Only the relocation differs (ours: at rest on
  the line at sample 58; the original's: sample 50, 12.5 off the line, moving
  at 53.5 u/s).
- **Regression gate** (`a_lone_craft_gets_round...`), all twelve clean both
  times. `01_Track`: `clean lap 43.6s laps 4 respawns 0` to `clean lap 37.1s
  laps 4 respawns 0 of 2836 lost at []`; `05_Track` 3 respawns `[524, 108, 620]`
  to 1 `[407]`; `07_Track` `[1137]` to `[715]`.
- **Survey** (`VENOM`, lone AI, Ace and Novice, 24 circuit-directions, 18,000
  ticks, `--release`): **589 rescue events to 9**. `13_Track` Novice 343 to 0 and
  `29_Track` reversed Novice 211 to 0 (the jump pathology - the hull is held by
  the far lip and the driver no longer brakes over the gap), `17_Track` 17 `Reset`
  hits to 0, `01_Track` 3 to 0, `06_Track` 2 to 0. Left: `05_Track` Novice
  beached at a wall at samples 1605-1660 (2 `Stalled`, 2 wrecked), five wrecks
  elsewhere, one `05_Track` Ace `Reset`. Every run completes four laps. The
  player population (autopilot on slot 0): one event; `05_Track` Novice and Ace
  stay on lap 1, grinding the same wall at 1-3 u/s until the shield runs out -
  **as on `main`**, where Novice and Skilled are eliminated there too.
- **Shield**, clean-lap board (`VENOM`, lone Ace), end-of-run pool / per-lap
  charge: `01_Track` 30.08 / 41.9 3.8 4.1 to 59.90 / 2.1 5.0 4.8; `05_Track`
  91.18 / 2.1 33.8 0.6 to 66.08 / 1.9 1.4 2.0; `07_Track` 80.48 / 19.9 19.8 19.9
  to 52.74 / 22.7 25.9 25.2 (worse: more wall contact); `14_Track` 93.43 to 85.96
  and `10_Track` 79.49 to 94.84 end-of-run, per-lap unchanged; the other seven
  within 0.5.
- **`ai_clean_lap_gate`** regenerated: four rows improve (`01_Track` FLASH to
  `CleanLap`, `05_Track` RAPIER and VENOM and `14_Track` RAPIER from `Died`);
  `13_Track` PHANTOM still dies but no longer records a clean lap first (the hull
  port alone does this). Wall-contact ticks fall 4,251 in total, but 19 rows rise
  past tolerance, most on `13_Track` (RAPIER 515 to 1,453) and `07_Track`.
- **`off_track_rescue_ground_truth`** re-pointed at a placed shove off
  `04_Track`'s open edge: no tier on any forward circuit sends the player's
  autopilot off through `OffTrack` any more.

### What (1) alone did, measured on `sunk-craft-floors` (superseded)

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
script was a throwaway script, not kept; folding a `--rewrites` and a
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

1. ~~The sunk craft~~: landed 2026-09-29 with the airborne reset (above). Open
   from it: the respawn pose and speed, and the player's shield charge, of the
   original's reset state; the 200-unit distance trigger; `06_Track`'s join at
   1196-1200 measured in the original (Vertica is reachable now).
2. An original-side approach on an open edge, driven rather than teleported: scripted
   steering out of the line at `03_Track` spline index 1780 or `18_Track` index 73/2044,
   through `just scripted-emu`, replayed through `oag-trace run` for the pair.
3. ~~The `01_Track` grid~~: done 2026-09-29 by rule, not capture; Metropia
   reversed was read instead (`grid.md`). `01_Track` *is* reachable now through
   the dev-unlock byte, and the Single Race log above holds the original's whole
   `01_Track` grid at tick 0 (`orig01-single-hold.csv`,
   not committed) - a left-side grid capture waiting to be compared.
4. A perturbed-input survey (a lateral shove on a live lap) once (1) or (2) says what the
   original does; the shove sweep above is the cheap static form of it.
