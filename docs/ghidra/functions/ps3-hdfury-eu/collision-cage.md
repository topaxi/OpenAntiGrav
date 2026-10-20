# The collision cage: HD runs none off the track header

2026-10-20, lane `hd-chenghou-jumps`. The question: Pulse keeps every craft
inside an invisible tube along the AI track data
([collision-cage.md](../psp-pulse-usa/collision-cage.md)), turned on by bit 0 of
the track header word `+0x18`. HD's `04_chenghou_project` is the one HD circuit
whose header sets that bit, with cage bytes on all 839 points, so our race ran
Pulse's cage there, **inherited and not measured**. The maintainer reported the
craft "pulled down quite strongly on the first two jumps" on Chenghou Project
(Time Trial, Piranha).

**Answer: HD's original runs no cage there.** Our cage was the whole pull-down.
Ours now holds it off by the title (`CraftLaws::cage_from_track_header`, HD
`measured(false)`). Confidence **85** for "HD runs no cage off the track header".
The behaviour is measured live. The static read agrees but does not prove it on
its own, because Ghidra's PPC64 decode of the cage module is soft.

Program `/hdfury/EBOOT-ps3-hdfury-eu.elf`. It has two TOCs: `0x008bd3c4` (16,090
descriptors) and `0x008ad4d8` (11,035). Every function here uses `0x008ad4d8`.

## Ours: the cage is the pull-down

Scratch sweep `crates/raceplay/tests/hd_chenghou_jumps_sweep.rs`, Time Trial, Venom.
The first rows below use Piranha flown by our autopilot. The placed rows use classic
Harimau, to match the save state, with thrust held alone. Run it with:

```text
OAG_SWEEP=1 [OAG_CAGE=off] [OAG_HD_TEAM=Harimau] [OAG_PLACE=x,y,z,fx,fy,fz,ux,uy,uz,speed] \
    cargo nextest run -p oag-raceplay chenghou_jumps_per_tick --run-ignored all --no-capture
```

With the cage, one tick on each jump carries the whole push, straight down:

| Run | Tick | Cage `dv` (x, y, z) | vy before -> after | Height above spline |
| --- | --- | --- | --- | --- |
| Piranha, jump 1 | 840 | `(-4.7, -35.3, 16.4)` | `1.8 -> -35.0` | 15.7 -> 16.3 |
| Piranha, jump 2 | 941 | `(1.5, -22.7, 10.4)` | `-16.9 -> -40.9` (3 ticks) | 16.6 -> 16.3 |

Then a few smaller pushes follow on jump 1 (`-5.1`, `-0.4`, `-1.7`). With the cage
off, the same takeoffs climb to 24.6 and 24.3 above the spline instead of being
capped at about 16.5, the tube's top: `pos + down * (o5 - 27)`. Gravity per tick is
the same either way, about `-1.37` per tick (`-83 /s^2`).

## The original: no step anywhere

RPCS3 attached to the state
`hd-timetrial-chenghou-project-harimau-classic-start` (Chenghou Project,
Venom, classic Harimau). The craft was placed with `scripts/rpcs3_place.py`'s
`write_pose` at our own pose and speed about 0.4 s before each takeoff:

- A: `(-18.70, 49.99, -70.30)`, forward `(-0.1748, 0.2192, -0.9599)`, 132.5
- B: `(-33.90, 54.03, -228.37)`, forward `(0.0174, 0.2495, -0.9682)`, 112.4

Cross was then held for 2.2 game seconds and the body (`+0x190` velocity, `+0x200`
position) was read on every new position. There were two repetitions of each.
**The largest per-frame vy drop over each whole run is `-2.34`, `-2.12`, `-1.65`
and `-1.63`**, which is gravity at a 30 to 60 fps frame. The landings rise
`+60` to `+75` in one frame. Where ours took its cage step, the original keeps
falling smoothly. Compared by track position (`pz`), with vy in units per second:

| Jump | pz | Ours, cage (before) | Ours, no cage (after) | Original |
| --- | --- | --- | --- | --- |
| A | -125 | 54.18 / -40.0 | 54.53 / -9.4 | 56.54 / -4.7 |
| A | -150 | 41.73 / -60.7 | 51.14 / -24.7 | 53.74 / -22.3 |
| A | -170 | 37.13 / 18.2 (landed) | 45.43 / -37.8 | 48.48 / -36.0 |
| B | -300 | 53.73 / -41.2 | 53.92 / -27.7 | 55.94 / -25.8 |
| B | -320 | 44.19 / -54.5 | 47.68 / -40.6 | 49.45 / -40.2 |
| B | -340 | 32.07 / -67.0 | 38.24 / -53.9 | 39.28 / -54.7 |

Each cell is `py / vy`. After the change, ours' vy follows the original's to within
1 to 5 along both arcs. **Still open:** ours flies about 2 units lower, and the gap
is already there at the takeoff (`pz -100`: py 54.58 against 55.59, vy 8.8 against
13.0). That is a takeoff or ramp residual, not the cage.

## Static: where HD's cage would be, and why it does not run

| Claim | Evidence | Confidence |
| --- | --- | --- |
| HD parses the track definition's `collisionCageEnabled` attribute into byte `+0x1b7` of its track record | string `0x0078fcc0`, TOC slot `0x008aed20` = `0x1848(r2)`; `FUN_001ea258` compares it at `0x001ea86c` and on a match parses a bool into `rec + 0x1b7` (`0x001ea884`-`0x001ea898`, `bl 0x006772c8`) | 80 |
| The byte defaults to `0` | `li r31,0` at `0x001ea09c`, `stb r31,0x1b7(r22)` at `0x001ea1ac` (`FUN_001ea028`) and `0x001ec1cc` (`FUN_001ec048`); no `lbz` of `+0x1b7` anywhere, only these two stores | 75 |
| No HD definition sets it | every `*definition*.xml` in `DATA00`-`DATA06.PSARC` and the PSN pressing's `data01`-`04.psarc`: zero hits for `collisionCage` | 95 |
| HD carries Pulse's cage geometry, compiled from `CollisionCageFx.cpp` | `FUN_0007c588` (`CageEffect_Spawn`, below) reads the point's bytes `+0x68`..`+0x6f` as `s * abs(s) * 250` with `s = byte * 0.007874016` (TOC `0x008a7398`, `0x008a73a4`), with Pulse's extremes pre-folded: `25`, `2.5`, `27`, `33` (`0x008a73a8`-`0x008a73b4`) | 70 |
| The module `0x0007bf40`-`0x0007d0f0` has no caller from outside | a `bl` scan of the whole text segment and a scan of the whole file for each function's `.opd` descriptor address: no reference to `0x0007bf40`, `0x0007bf80`, `0x0007c330`, `0x0007c588`, `0x0007cb90`, `0x0007cee0` or `0x0007cf10`; its vtables (`0x00862d60`, `0x00862da0`, `0x00862de0`) are named only by TOC slots the module itself loads | 60 |
| No `200.0` or `15.0` (Pulse's reset distances) sits in the module's TOC run | `0x008a7300`-`0x008a7420` dumped | 60 |

The decompiler stops in every one of these functions (`halt_baddata`, VMX128), so
the function boundaries are soft. That is why the static read is corroboration only.
The live run is the measurement.

`CageEffect_Spawn` (`0x0007c588`) is named by its shape against Pulse's
`CageEffect_Spawn` (`0x08832b0c`). It has four slots of `0x90` bytes, its cursor at
`+0x740` wraps past 3, and it stores the point pointer at `+0x44 + 4 * slot`.
Confidence **62**: a static structural match, and it never ran live.

## Ported

`oag_title::craft_laws::CraftLaws::cage_from_track_header`. `oag_raceplay::load`
builds the cage only when the header flag is set and the track's title runs a cage
off it. A title with no `CraftLaws` (Pulse) runs it.

| Title | Value | Origin |
| --- | --- | --- |
| Pulse | on (no `CraftLaws`) | measured, [collision-cage.md](../psp-pulse-usa/collision-cage.md) |
| Pure | `true` | inherited from Pulse; its own headers clear the flag on all 16 tracks, so it runs none |
| HD | `false` | measured (this page) |
| Omega | `false` | inherited from HD |
| 2048 | `false` | inherited from HD |

The load report names the gate: `collision cage: off, Wipeout HD runs no cage off
the track header's flag`. `hd_env_probes_ground_truth`'s every-circuit walk asserts
that no HD circuit runs a cage, Chenghou by the title, and that Pulse's still does.
With HD's value flipped to `true`, that test fails on `04_chenghou_project`.

**Not ported:** HD's `collisionCageEnabled` attribute as a second source. No
definition on any disc sets it, so it changes nothing today. A mod that set it
would want the attribute read, and that is when it gets wired.

## Omega and 2048

**Checked, applies (off), wired through the same field.** Both executables keep
the strings `collisionCageEnabled`, `Cage Collision` and `Cage : %i`, but neither
has `CollisionCageFx.cpp`. No definition file in Omega's base or patch packages,
or in 2048's (base, v1.04 patch, DLC), sets the attribute. Their track points
(version `0x107`, 96 bytes) end before the cage bytes at `+0x68`, so no cage could
be built there either way.
