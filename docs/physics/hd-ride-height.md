# HD ride height against RPCS3: the grid and the straight match, the flyby does not

2026-10-08, lane `hd-ride-height`. The maintainer's request: in HD's pre-race fly-by the ship
"felt like it is further above the floor" than on the original. Measured, not tuned toward.
Tool: `scripts/rpcs3-height.py` (a companion of `rpcs3-trace.py`; one boot samples the player's
body, craft entry and ship before the cross tap, on the grid and down a straight).

**Verdict.** No ride-height difference in the race: on the grid and down the straight we ride
where the original rides. **In the flyby the original's craft sits 1.84 units lower than ours**
(2.16 above the floor against 4.00), so the maintainer's impression reproduces, and it is a
flyby-only difference: the original's hover target is lowered while the fly-over plays, and ours
draws the craft at its settled placement pose.

## What was measured

Racebox Time Trial, Venom, no AI, weapons off, Pilot Assist off, Feisar concept1 (`feisar_c1`,
what HD races), circuit `Data\Environments\Talons_Junction\track.rcsmodel`, grid slot 0.

The craft's own ground distance is **`craft+0x260`** (entry, the `0x600` object at
`ship+0x5fac`): body centre above the surface under it. Confidence 85 (it equals the body's
height above a fixed floor to 0.001 in every state below: floor y = -54.080 from the flyby
(`-51.916 - 2.164`) and from the grid (`-50.080 - 4.001`), and `entry+0x354..0x374`, the four
hull probes' heights, read 1.024-1.039 in the flyby and 2.875 at rest, each 1.125 under the
body's figure, the hull point's own drop).

| Moment (original, RPCS3) | boots | body y | height `+0x260` | probe height `+0x354` | spring target `+0x344` |
| --- | --- | --- | --- | --- | --- |
| Flyby, `START RACE` prompt up, 26 s (b1) and 20 s (b4) | 2 | -51.92 +/-0.03 | **2.156 / 2.159** (2.128-2.190) | 1.03 | 2.25-2.27 |
| Grid after the countdown, at rest | 2 (+2 in-race boots) | -50.080 | **4.001** | 2.875 | 4.125 |
| Straight, `hd-thrust` held 600 ticks, to 123 u/s | 2 | -49.9 .. -35 | **3.980** mean (3.862-4.004) | - | - |

| Moment (ours) | body y | height (floor -54.080) |
| --- | --- | --- |
| Flyby: the world is held at its first tick, the placement pose | -50.076 | 4.004 |
| Grid at rest (280 ticks) | -50.065 | 3.99 |
| Straight at the same positions (z within 0.4 of the original, x >= 267) | within +/-0.17 of the original's y | not read separately |

Ours is derived from the same floor, not from a raycast of its own mesh; the grid rests agree to
0.015, which is what the check rests on. The straight's early part (x 130-240) is not compared: the original drifts 7 units
sideways there and ours does not (a handling gap that `hd-handling-ground-truth.md` records), on
a banked slope, so a y at the same x is not the same floor.

Frames: `data/scratch/hd-ride-height/b1/frames/flyby-*.png` (RPCS3, shots 024-032 show the craft
close), `b3/frames/*.png` (in-race, craft at 4.0), ours `ours/fly-1650.png`, `fly-1800.png`.
The flyby cameras are not the same shot at the same tick (the original's start is not read off
the clock), so frames were compared by what they show, not overlaid.

## Where it comes from

Flyby-only, and now read (2026-10-08, lane `hd-flyby-hover`,
[hover-target.md](../ghidra/functions/ps3-hdfury-eu/hover-target.md), confidence 90). The craft
update holds `entry+0x344`, the hover target, at `0.75 * entry+0x348` while the craft is in its grid
state (`entry+0x2f8 == 0`, flag word bit 1): `+0x348` is a jitter `3.0 + rand8 * 0.0003`, so the target
reads 2.25-2.31 and the craft settles `target - 0.16` above the floor, 2.13-2.19. The state flips to
racing **at GO** (three boots: the first sample with the HUD's `GO` and race clock 0), not at the
cross tap, so the craft sits low through the flyby **and the whole countdown**. From the flip
`+0x348` is a timer (`+= dt`, 1.00 per game second) and the target is `0.75 * min(5.5, +0x348)`: a
linear ramp of `0.75` units a second that reaches 4.125 after 2.46 s. The craft follows it within 0.04
(`h = target - 0.16`): 3.20 at 1.4 s after GO, 4.00 by about 2.5 s. (The first reading here said "about
1.5 s": that was a coarse sample, three boots with a 0.1 s sampler give 2.5 s.) The four-point hull
is not the cause of anything seen here, because the grid and straight heights agree.

Ours cannot draw this by construction in the flyby (the world is held at its first tick), and
until this lane the craft sat at its settled placement pose for the whole flyby and countdown.
Ported (2026-10-08) as HD title data, `oag_title::launch_hover::LaunchHover` on `RaceDefaults`: the
clamp is written to every craft each tick by `oag_raceplay::launch_hover` from the countdown clock
(`3.03825` on the grid, `+ 1.0` per second from the tick the grid state ends), the spring reads
`min(base, clamp) * 0.75` (`oag_physics::hover::capped_target_height`) and the spawn height is the
same spring's rest height under the lowered target (`oag_gameplay::spawn::capped_spawn_height`), so
the placement pose, the flyby's held world, is already low. Pulse, Pure, Omega and 2048 carry `None`
and their paths are unchanged. **Chosen, not measured:** the clamp is the mean of the original's
`3.000 .. 3.0765` jitter (no random stream consumed), the first timer step lands on the tick the grid
ends, and rivals take the same clamp (read in the release loop, never measured: Time Trial had no AI).

Aligned on the craft's own clock (`tau` = `entry+0x348` minus its grid value 3.03, game seconds since
GO, so the sampler's wall time does not enter):

| Moment | original | ours (`hd_grid_hover_ground_truth` and a dump) |
| --- | --- | --- |
| flyby / placement | 2.13-2.19 (3 boots) | 2.158 |
| countdown | 2.13-2.19 (3 boots, state 0 throughout) | 2.144 (4 s in), 2.169 (last tick) |
| tau 0.1 | 2.15-2.20 (c2 2.144, c3 2.200 at tau 0.23) | 2.17 at tau 0 |
| tau 1.45 | 3.204 (c2) | 3.189 at 1.40, 3.373 at 1.67 |
| tau 1.8 / 1.9 | 3.421 / 3.500 (c3) | 3.627 at 2.00 (the original is about 0.1 lower at equal tau) |
| tau 2.6 and later | 4.040 (c2, tau 2.6), 4.027 (c3, tau 3.5) | 3.998 at 2.5, 4.039 at 3.0, 4.025 at 4.0 |

The ramp's middle runs about 0.1 above the original at equal tau (one tick of timer step or the
spring's lag, which ours does not model separately); the end states and the shape match.

Frames: `data/scratch/hd-flyby-hover/ours-sheet2.png` (original flyby shot beside ours), `c2/sheet.png`
(the original's countdown and GO). The chase camera follows the craft down, so the craft holds the
same place on the screen through the rise, as the original's frames do. Our livery in the chase
frames is not the dark Feisar the capture shows; not this lane's.

Other titles: Omega (HD's craft code lineage, no PS4 emulator): not checkable, `None`. 2048: not
checked, its craft is a separate build, `None`.

## Traps

- `walk_to_race`'s last press can land on the race's `START RACE` prompt and skip the flyby: two of
  four boots here were already in the race with the timer running (the craft rising from 3.07). The tool
  now refuses to press on `InGame`/`HUD`/`Team Launch Transition` and exits early when the first height
  reads above 3.
- A copied lane config carries the old pad name; see `docs/reverse-engineering/rpcs3-capture.md`.

## Other titles

Omega: not checkable (no PS4 emulator). 2048: not checked, no HD-style flyby wired. Pulse's flyby
holds the world at tick 0 too; whether Pulse's original lowers its craft there is **not measured**.
