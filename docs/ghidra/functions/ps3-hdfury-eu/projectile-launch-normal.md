# A projectile's launch normal is the located track's up, not the craft's

Read 2026-11-12 (`hd-rocket-init-normal`), static, over the decrypted `EBOOT.elf`
(`ps3-hdfury-eu`, TOC `r2 = 0x008ad4d8`). Ghidra truncates most of these functions at
Cell-only AltiVec loads (`lvlx`/`lvrx`) and misses two of the five stores entirely, so the
instructions below come from `llvm-objdump -d --triple=powerpc64-unknown-linux-gnu
--mattr=+altivec` over the file. Ghidra's "instructions with `0x7850`" search is not a
complete census on this binary.

**The law.** Every HD projectile that follows the track is born carrying
`0 - craft+0x7850`, and `craft+0x7850` is the `down` axis of the track spline, blended at the
point the craft was located on this tick. The carried normal is therefore **the located track's
up**, the same law Pulse has
([psp-pulse-usa/rocket-visuals.md](../psp-pulse-usa/rocket-visuals.md), "The launch normal is
the track's, not the craft's", confidence 88), and **not** the craft's own up, which HD keeps at
`craft+0x200` ([physics.md](physics.md), `Craft_UpdatePitch`). **Confidence 86**, from the
evidence below.

| Address | Name | Confidence | Role |
| --- | --- | --- | --- |
| `0x00124d90` | `Rocket_Init` | 85 | `(rocket, launch_matrix)`; copies the craft's track cursor, stores the carried normal at `rocket+0x100` |
| `0x00159a30` | `Shuriken_Init` | 80 | the same for the Shuriken, normal at `+0x150` |
| `0x00123378` | `PulseBullet_Init` | 55 | a third bullet class's init, by-catch, normal at `+0x180` |
| `0x000a97f0` | `Track_UpdateCursor` | 78 | `(track, out_record, ..., cursor, ...)`, `v2` = the point: locate, then fill the record |
| `0x000a8198` | `Track_SampleNearestPoint` | 78 | walks the cursor's sample index, Newton-refines the spline parameter, tail-calls the evaluate |
| `0x000a7cc8` | `Track_EvaluateSpline` | 80 | `(track, out, cursor, ..., t)`: a four-sample cubic blend of the sample records into `out` |

`Missile_Init` (`0x0011c5d0`, [missile-ribbon.md](missile-ribbon.md)), `Plasma_Init`
(`0x00120fe8`) and `Plasma_Launch` (`0x0011f810`, both [plasma.md](plasma.md)) are named on
their own pages; their stores are read here.

## The five stores

Each loads the craft, loads the vector at `craft + 0x7850` (`li rN, 0x7850` / `lvx`), subtracts
it from a zeroed register (`vxor vZ,vZ,vZ` ... `vsubfp vD, vZ, v0`), and stores the result into
the projectile:

| Init | Craft pointer from | `lvx` of `+0x7850` | `vsubfp` | Store | Field |
| --- | --- | --- | --- | --- | --- |
| `Rocket_Init` `0x00124d90` | `rocket+0x138` (`r8`, written `0x00124e68` from the craft table at `rocket+0x44`) | `0x00124f28` | `0x00124f2c` | `0x00124f38` `stvx v12, r22, 0x100` | `rocket+0x100` |
| `Missile_Init` `0x0011c5d0` | `missile+0x180` (`r5`, `0x0011c8ac`) | `0x0011c900` | `0x0011c904` | `0x0011c908` `stvx v13, r28, 0xd0` | `missile+0xd0` |
| `Plasma_Init` `0x00120fe8` | `r5`, the call's craft, kept at `plasma+0x6c` (`0x001210e8`) | `0x001210c8` | `0x001210d4` | `0x001210f0` (`r11 = 0x110`) | `plasma+0x110` |
| `Plasma_Launch` `0x0011f810` | `plasma+0x6c` (`0x0011f9b8`) | `0x0011f9d0` | `0x0011f9d4` | `0x0011f9d8` `stvx v12, r27, 0x110` | `plasma+0x110`, again at the release |
| `Shuriken_Init` `0x00159a30` | `shuriken+0x164` (`r8`, `0x00159cb8`; written `0x00159b60` from the craft table) | `0x00159d30` | `0x00159d34` | `0x00159d40` `stvx v13, r31, 0x150` | `shuriken+0x150` |

The Rocket, Missile, Plasma and Shuriken inits also copy the craft's four-word track cursor
`craft+0x7820..+0x782c` into the projectile (Rocket `+0x128..+0x134` at `0x00124df4..0x00124e60`,
Missile `+0x170..+0x17c`), the cursor its own track probe walks from. Plasma re-takes the normal
at the release, as on Pulse, where the two Plasma captures sat 0.29 to 0.86 degrees from the
release-time sample against 0.04 for the others.

**The Rocket's flight probes along the stored field.** `Rocket_Update` (`0x00123fb0`,
[rocket-trail.md](rocket-trail.md)): at `0x00124988..0x001249dc` it forms
`next - (rocket+0x100) * 6.0` (`r30 = rocket + 0x100`; `6.0` is the word at `0x008aa174`,
`0x40c00000`) and passes it as the segment end to the track query `0x0007be58` with the
rocket's cursor `rocket+0x128` (`r9`); on a hit it writes the hit normal back to `+0x100`
(`0x00124a58`). So `+0x100` is the carried normal, the same offset Pulse's `Rocket_Init` writes.

**Callers.** `Rocket_Init` is called from `RocketManager` (`bl` at `0x0014a290`, `0x0014b374`;
`RocketManager.cpp` constructors `0x0014a3d0`/`0x0014aea8`) and its OPD descriptor is
`0x00876258`. `Shuriken_Init` is called from `ShurikenManager` (`0x0014ccd4`,
`0x0014d398`; `ShurikenManager.cpp` constructors `0x0014cdf8`/`0x0014d6b8`), with
`Shuriken.cpp`'s constructors at `0x00158d18`/`0x00158e98`. `scripts/ps3-toc.py map` gives the
`.cpp` attributions; `Shuriken.cpp` existing on HD at all is new here
([weapon-lineage.md](../../../gameplay/weapon-lineage.md) had "no manager located").

**`0x00123378` is not the Rocket's.** It is slot `+0x3c` of the vtable at `0x00863d98`
(OPD `0x008761d8`), whose slot `+0x14` is `0x00122098`; the Rocket's vtable is `0x00863e38`
(the constructor `0x00125330` stores it via TOC `0x008aa11c`), slot `+0x14` being
`Rocket_SubmitLight` (`0x00123de8`). `0x00863d98` is loaded by `PulseBullet.cpp`'s constructors
(`0x001235a8` at `0x00123614`, `0x001238d0` at `0x0012393c`) and by the destructor
`0x00121b20`, hence `PulseBullet_Init` at 55. It stores the same `0 - craft+0x7850` at `+0x180`.
Which player-facing weapon fires a `PulseBullet` is not read.

## What `craft + 0x7850` is

`Craft_UpdateLapProgress` (`0x000e4b28`, the per-tick progress update) calls
`Track_UpdateCursor` (`bl` at `0x000e4b54`) with `r3` the track (`craft+0x6f64`),
`r4 = craft + 0x7830` (the output record), `r8 = craft + 0x7820` (the cursor), and `v2` the
craft's position (`*(craft+0x6944) + 0x200`, also saved to `craft+0x6ff0`). `Track_UpdateCursor`
runs the locate `0x000a8b20` on a copy of the cursor; when it finds a sample, it writes the
cursor back (`0x000a98d0..0x000a98dc`) and calls `Track_SampleNearestPoint(track, out, cursor,
..., point)` (`0x000a98e0`). That function steps the cursor's sample index (`+0x8`, records
`0x70` bytes apart, `+0xc` the record pointer), runs three Newton-shaped steps on the cubic's
parameter against the point (`0x000a83a4..0x000a8750`), clamps it to `[0, 1]` (`fsel` at
`0x000a8788`, `0x000a878c`) and tail-calls `Track_EvaluateSpline` (`b 0x000a7cc8`).
`Track_EvaluateSpline` weights the first sample's four 16-byte vectors `+0x00/+0x10/+0x20/+0x30`
by `(1 - t)^3 / 6` (the `1/6` is `0x3e2aaaab`, TOC `-0x5758` = `0x008a7d80`) into `out+0x00/+0x10/+0x20/+0x30` (`0x000a7db8..0x000a7e34`) and accumulates
the next three samples through `0x000a5860`.

The sample record is the `.vex` track control point, `0x70` bytes in HD's version `0x106`:
`+0x00` position, `+0x10` tangent, `+0x20` **down**, `+0x30` lateral (`oag_vex::track`, measured
against every shipped circuit). So the record at `craft+0x7830` is
`{position, tangent, down, lateral}` at the craft's located point, and:

- `craft+0x7840` (record `+0x10`) is the tangent. Seen live as the track's direction
  ([eliminator-flip.md](eliminator-flip.md), `fw.tr` `0.95` flying forward, `-0.95` after a flip;
  confidence 90).
- `craft+0x7850` (record `+0x20`) is the down. Seen live **not** to be the craft's up: in the
  2026-10-20 player-input captures over Ubermall's long jump, `dot(craft+0x200, -craft+0x7850)`
  falls below `0.9` while the nose is held up and the pitch gate closes on exactly those rows
  ([airborne-ground-truth.md](../../../physics/airborne-ground-truth.md), "The pitch gate closes
  live, at 0.9, and keeps the bias"; confidence 90). A field equal to the craft's up would read
  `1.0` throughout.

By-catch: `Craft_UpdateLapProgress` also stores `craft+0x7810` (`stw r7, 0x7810(r31)` at
`0x000e4bd0`), the lap counter [gantry-clock.md](gantry-clock.md) found no writer for.

**Why 86.** Five independent call sites store the same expression, the field's writer is read
through to the sample record's documented `down`, the Rocket's update is read probing along the
stored field, and the field is seen live to diverge from the craft's up. The projectile's own
`+0x100` was not read live on HD; Pulse's equivalent was, seven times
(`psp-pulse-usa/rocket-visuals.md`). Under the rubric that is the 85-94 band: decompilation
unambiguous with every site consistent, short of an HD runtime read of the stored value itself.

## Omega and 2048

**Omega: checked, applies.** Base `eboot.bin` (`/omega/eboot-ps4-omega-eu.bin`), static.
`Plasma_Init` (`0x0135cad0`, patch `0x0138ab50`) copies the craft cursor `craft+0x8000`/`+0x8010`
(`0x0135cae4`, `0x0135caf3`) and stores `-(craft+0x8040)` at `+0x150` (`0x0135cbb9`, `0x0135cbca`);
`Plasma_Launch` (`0x0135d5d0`) re-reads `+0x8040` at `0x0135d74e`. The same writer chain exists with
eight-byte pointers: `0x01313510` locates through `0x01380140` into the cursor at `craft+0x8000`
(written back at `0x013136e4..0x013136f0`) and calls `0x01381300` with `rsi = craft + 0x8020` (the
record); `0x01381300` walks `0x60`-byte samples (version `0x107`) and jumps to `0x0137fcd0`, which
writes `out+0x20 = w * sample+0x20` (`0x0137fde7..0x0137fdec`) and copies the flags byte at
`+0x5d`, the `0x107` layout. So `craft+0x8040` is record `+0x20`, the down, and Omega's
[plasma.md](../ps4-omega-eu/plasma.md) "`-craft->up`" reads the located track's up. Confidence
**80**: static, the writer read through; no memory readout on Omega. Omega's Rocket, Missile and
Shuriken inits were not located.

**2048: checked, applies.** `vita-2048-eu-v104`, the function referencing `WO_PLASMA_CHARGING`
(`0x811fcb40`, its `Plasma_Init` by the `'PLHE'`/`'PLCG'` spawns, the `1.0` charge and the `-1.2`
seed it shares with HD and Omega) copies the cursor `craft+0x6e60..+0x6e6c` into `+0x130` and
stores `FloatVectorNeg(craft+0x6e90)` at `+0x110`. `+0x6e90` is cursor `+0x30`, HD's own
`+0x7820`/`+0x7850` relation with four-byte pointers. Confidence **75**: one init, the writer not
traced.

## Turning it on in this engine (2026-11-12)

`Weapons::launch_normal_is_track_up` set to `true` on HD, Omega and 2048 is the law above, and
the field it gates (`Ship::track_up`) has no reader but `oag_weapons::Craft::launch_normal`. With
it on, the three runs that went red when Pulse's law was first tried on HD go red again; the
diagnosis is that each is a chaotic divergence at a margin, not a wrong normal:

- **The Rocket's flight barely moves.** `rocket_floor_trace` on Anulpha Pass, off / on: swept
  into geometry 149 / 148, probed a wall 8 / 6, struck a hull 14 / 15, expired 7 / 5, median
  lifetime 49 / 48 ticks.
- **Omega, Amphiseum reversed, Flash, seed 1.** The first volley leaves at tick 909 in both runs
  with the eight craft's track up 0.27 to 7.53 degrees from their own; its last rocket ends one
  tick earlier (932 against 933), the craft part at tick 1209, and slot 5 respawns at 5601, with
  no weapon hit, 4,700 ticks downstream.
- **HD Novice Eliminator, seeds 1 to 12, ticks to 200 points.** Off: every seed ends, leader laps
  sorted `2 3 6 6 8 8 10 11 11 14 18 18`, seed 1 at 51,417 ticks (857 s). On: eleven end,
  `4 7 7 7 7 10 10 13 13 17 20`, median 10 (the test's window is 6 to 12), and seed 1 does not
  reach 200 inside 60,000 ticks, its eight scores spread 117 to 187. Shield drops of 5 or more
  (hits and walls) per minute average 13.0 off and 10.1 on, with per-seed spreads of 2.5 to 23.5
  and 3.6 to 15.2.

The tests that fail are `hd_eliminator_hud_ground_truth::a_finished_eliminator_leaves_a_board_ranked_on_points`,
`::a_novice_score_eliminator_reaches_its_target_at_about_the_originals_pace` and
`omega_original_ai_ground_truth::the_field_finishes_amphiseum_reversed`; all three pass with
the field off, as does the lone-craft regression.

### Landed (2026-11-12, `hd-launch-normal-flip`)

`Weapons::launch_normal_is_track_up` is `measured(true)` on HD, Omega and 2048 (Omega and 2048:
their `Plasma_Init` read, their Rocket, Missile and Shuriken inits take HD's). Pinned by
`projectile_cage_ground_truth::an_hd_plasma_bolt_rides_the_located_tracks_up`: the person's craft
posed 6 degrees off HD sample 100's up, a Plasma press, the charging bolt carries the sample's
`-down`; red with HD's flag reverted. The three tests were re-based on distributions, not on
the one draw each had:

- **Omega, Amphiseum reversed.** Omega racing is static only (no memory readout; shadPS4's race
  frames are untrusted), so no original number exists to re-measure. Over the five
  `perturb::GATE` scales times the nine `perturb::BOUND_OFFSETS` (45 runs of `fly`'s bounds),
  **8 of 45 miss with the craft's up and 8 of 45 with the track's up**; with the four other
  `perturb::SCALES` added, 8 of 49 and 11 of 49 (16 % and 22 %). The unperturbed red was a draw
  from that. The test is judged on `GATE`, at least two of five hold (the Tower row's rule in
  `vita_2048_original_ai_ground_truth`): at a 22 % miss rate fewer than two hold about 1 % of the
  time. With the track's up, three of five hold. What slot 5 does in the unperturbed run with the
  track's up: from tick 5452 it loses 92 to 76 shield in 1-point steps with no wall contact, no
  beam, no disruption and no roll (a hull scrape in the integrator, by elimination), slows from 156
  to 71, leaves the track at 5524 and is put back at 5602. The runs part at tick 1209, so the
  flag-off run has no comparable moment.
- **The field, noise method.** Distinct failing tests among the 31 `*_original_ai_ground_truth`
  tests per `OAG_PERTURB` scale (`0, 1e-3, -1e-3, 1e-4, -1e-4, 1e-5, -1e-5, 1e-6, -1e-6`): craft's
  up `0 0 0 0 0 1 0 0 1` (median 0, max 1), track's up `1 0 2 0 3 1 2 1 0` (median 1, max 3).
  Comparable to the bar the Turbo's running laws were judged on (median 1, max 4), not the same
  measure: that was 49 tests under an airborne pitch-torque perturbation, this is 31 under the
  airborne-gravity one `perturb` applies. The failures are
  Amphiseum and Ubermall, plus `hd_original_ai_ground_truth::at_the_events_own_skill_the_reference_racer_reads_63`
  once at `-1e-4`. **Ubermall as Phantoms** is green unperturbed but misses 10 of 49 runs (the
  45 `GATE x BOUND_OFFSETS` plus the four other scales) with the track's up against 3 of 49 with
  the craft's: open, not re-based.
- **HD Novice pace.** The original's number is one race (Vineta K, nine laps, run 4 of
  `scripts/rpcs3-hd-eliminator-score.py`; [eliminator-score.md](eliminator-score.md)); no second
  RPCS3 race was run. Seeds 1 to 12, laps the leading opponent completed: `4 7 7 7 7 10 10 13 13
  17 20` and seed 1 still running at 60,000 ticks on 19, median 10. The test now counts an
  unended race at the laps it had, the rule
  [race-modes.md](../../../gameplay/race-modes.md#the-pace-of-a-score-eliminator-2026-11-09)
  measured its table by, and asserts the median cannot depend on it: fewer than half unended,
  and each already at or past the median.
- **The finished board.** Its claim is the board's order, not a pace; seed 1 and the 30,000-tick
  limit were ours. Elite (the default) seed 1 ends at 40,745 ticks with the track's up. The check
  now runs on every race the pace tests end: twelve Novice and twelve Elite races. Elite with the
  track's up: every seed ends, laps `4 5 6 7 7 8 10 11 11 12 16 18`, median 9 (6 with the craft's
  up; the test's floor is 4).
