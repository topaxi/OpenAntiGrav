# Roadmap

Nine milestones. Each has an exit criterion that is a demonstrable fact, not a
judgement call, so it is always clear whether a milestone is done.

The ordering follows the natural dependency chain, with one deliberate
departure: the verification harness (M3) comes *before* the physics work (M4).
Building ship handling before it can be measured against the original means
building it twice, once by feel and once properly.

**Renumbered 2026-07-30.** [Rendering fidelity](#m6---rendering-fidelity) became
a milestone of its own, so the two after it shifted:

| Was | Is now |
| --- | --- |
| M6 - Shell and polish | **M7** - Shell and polish |
| M7 - Beyond Pulse | **M8** - Beyond Pulse |

Documents written before that date may cite the old numbers. The ones under
[`docs/`](../README.md) have been updated;
[ADR-0009](../architecture/adr/0009-multi-game-fanout.md) has **not**, because
[ADRs are immutable](../architecture/adr/README.md) - read its "M6" as this
table's M7 and its "M7" as M8.

---

## M0 - Foundation

**Status: complete.**

Repository, licensing, CI, documentation tree, and enough tooling to see what is
actually on the discs.

- [x] Repository hygiene, dual licensing, `data/` layout
- [x] Ghidra MCP wiring
- [x] Cross-platform determinism gate in CI, before there is a simulation to guard
- [x] `oag-core`, `oag-disc`, `oag-formats`, `oag-tools`
- [x] `oag-unpack`: `info`, `container`, `list`, `extract`, `hexdump`, `sniff`
- [x] Documentation tree and ADRs 0001-0006
- [x] Disc layout documented for [PSP](../psp/pulse-disc-layout.md) and [PS2](../ps2/pulse-disc-layout.md)
- [x] [Format status table](../formats/README.md) seeded

**Exit criterion:** both Pulse discs listed and identified from their own
contents; every distinct format on them has a row in the status table.

---

## M1 - Asset archaeology

Decode the containers, then the assets inside them.

- [x] [WAD container](../formats/wad.md): layout, name hash, compression flags
- [x] [LZSS decoder](../formats/lzss.md), verified on all 6,053 compressed entries
- [x] [PSP textures](../formats/psp-texture.md): decoded and rendering
- [x] [Front-end XML](../formats/fexml.md) and [handling stats](../formats/handling-stats.md)
- [x] [`.vex` scene format](../formats/vex.md): node tree, geometry, textures and
      the `Transform` hierarchy, validated by a bounding-box check over 342,115
      vertices
- [x] `.vex` mesh batches decoded into portable vertex buffers, with textures
- [x] [Track data](../formats/track.md): spline graph, racing line, PVS sections,
      validated against all 40 track files on the disc with nothing left over
- [x] [Collision geometry](../formats/collision.md): the indexed triangle soup,
      separate from the render mesh, validated on both discs with 319 of 319
      nodes closing exactly over 602,086 vertices
- [x] Parse a real track and render it: `oag-view --track` draws the driveable
      ribbon, `--mesh` draws all 482 art meshes in place, and the two outlines
      agree
- [x] Audio: [PSP `.bnk` sound banks](../formats/psp-audio.md) (`SBlk`
      container, PS-ADPCM waveforms, confidence 94/92) and
      [PS2 music](../formats/ps2-audio.md) (48 kHz stereo PCM, channel order
      and sample rate both cross-validated against the PSP's ATRAC3plus
      masters). The twelve non-circuit bank paths are recovered from the
      executable and hash-confirmed against `Data.wad` on both the USA and EU
      discs. Per-sound boundaries within a bank, and the 24 per-circuit bank
      names, are still open.
- [x] `oag-view`, a standalone asset viewer: textures, and models/tracks with
      an interactive orbit camera

**Exit criterion:** a Pulse track and a ship model render in `oag-view` from an
unmodified disc image, on both the PSP and PS2 asset paths.

**Met.** PS2 mesh batches were the first gap: their vertex type is a VIF/GS
packet, not the PSP's GU vertex array, and decoding it closed on an exact
arithmetic invariant across all 757 PS2 `.vex` files (98,243 batches, 11.8
million vertices, confidence 94) - the PS2 `01_Track` and Feisar ship come out
the same size as their PSP counterparts (radius 831.02 vs 830.92, and 6.45 on
both, respectively). See
[PS2 vertex decoding](../formats/vex.md#ps2-the-vertex-type-still-names-the-attributes-but-the-data-is-a-vif-packet).

PS2 textures were the second gap, and are now also decoded: they are not
embedded in `.vex` at all (that block is always length zero) but standalone
GS-upload-packet entries in the PS2 WADs, PSMT8/PSMCT32-swizzled with a CSM1
palette, confidence 94 across 5,348 real textures with every declared size
closing against every other one.
`oag-view --mesh ... --textures <hash>` now draws the PS2 Feisar ship in its
own livery rather than a white silhouette. See
[PS2 texture format](../formats/ps2-texture.md). **Both asset paths now render
with visual parity, not merely geometric parity.**

Two things are still open, neither blocking the exit criterion: **how the game
finds a model's texture set** (it's an ordinarily-hashed WAD entry, and the
name that hashes to it hasn't been recovered, so `oag-view` takes it
explicitly rather than automatically) and **`PSMT4`-swizzled textures** (5 of
5,348, refused rather than guessed at).

**Where to start:** containers are done. `oag-wad` reads every archive on every
disc, decompresses, resolves names by hash and renders textures to PNG. What
remains is geometry: decoding `.vex` mesh batches into portable vertex buffers,
and the `section`/`gate` payloads that carry the track spline.

---

## M2 - Binary understanding

The PSP `BOOT.BIN` is an unencrypted ELF, so this can start immediately.

- [x] Allegrex processor module built and installed; VFPU decoding verified.
      See [Allegrex and the VFPU](../psp/allegrex-vfpu.md)
- [x] `BOOT.BIN` (PSP) loaded at base `0x08804000`, analysed: 10,683 functions
- [x] Confirm the image base against PPSSPP's module load address. See
      [Allegrex and the VFPU](../psp/allegrex-vfpu.md#on-the-value)
- [x] Load `SCES_547.48` (PS2) into Ghidra: 5,234 functions, `r5900:LE:32:default`,
      image base `0x00100000`, 28 overlay spaces. See
      [`docs/ghidra/functions/ps2-pulse-eu/`](../ghidra/functions/ps2-pulse-eu/README.md)
- [x] [Main loop and frame pacing](../psp/frame-pacing.md); state machine outline
- [x] Simulation timestep resolved: the original uses **variable** delta, not a
      fixed step. See [ADR-0007](../architecture/adr/0007-fixed-timestep-vs-original.md)
- [x] WAD subsystem: hash, lookup, mount, read, decompressors
- [x] [Input](../ghidra/functions/psp-pulse-usa/input.md), [collision](../ghidra/functions/psp-pulse-usa/collision.md),
      [video](../ghidra/functions/psp-pulse-usa/frontend-video.md), [physics model](../physics/README.md)
- [x] Physics is float, not fixed-point; integrator is 3 sub-steps of dt/3
- [x] [Engine, brakes, steering and pitch](../ghidra/functions/psp-pulse-usa/engine.md):
      the craft update frame, every control force term, and all 32 handling
      parameters placed with the block's byte accounting closing exactly
- [ ] Memory management and the heap layout
- [ ] Resource loading: how a WAD entry becomes a live object
- [ ] Game state machine
- [ ] The original PRNG
- [ ] Memory maps for both platforms

**Exit criterion:** engine lifecycle and memory map documented; at least 50
functions documented to the standard in
[`ghidra/function-template.md`](../ghidra/function-template.md).

The function count is met: [names.tsv](../ghidra/functions/psp-pulse-usa/names.tsv)
carries **120** documented symbols, each refused by
`scripts/apply-ghidra-names.py` unless its address and name are still on an
evidence page. The memory map is what remains.

---

## M3 - Verification harness

The instrument everything after this is measured with.

- [x] Scripted input playback into PPSSPP: `input.buttons.send`/`press` through
      the websocket debugger, verified by watching `craft+0x2b8` read 100 while
      thrust is held
- [x] Fixed-start scenarios: **not** via a PPSSPP save state - measured
      2026-07-30 at `0.031` units / `1.41` degrees between two loads, no better
      than the plain menu walk - but via heading-pinning
      (`--start-heading`, `0.0022` units / `0.0001` degrees), already
      committed. See
      [ppsspp-debugger.md](../reverse-engineering/ppsspp-debugger.md#measured-a-save-state-does-not-pin-the-pose-as-tightly-as---start-heading-does-and-is-not-adopted).
- [x] Trace capture: `scripts/psp-trace.py` records position, orientation,
      velocity, dt, groundedness and the whole control block per call of
      `Ship_UpdateCraft`, breakpoint-driven off the running game
- [x] `oag-trace`: compares a Rust run against a captured trace, with
      tolerances from [the protocol](../reverse-engineering/verification-protocol.md).
      `oag-trace show|run|compare`; reports the first divergent tick, field,
      magnitude and a bounded/growing/shrinking trend. See
      [oag-trace](../tools/oag-trace.md).
- [ ] The same for PCSX2

**Exit criterion:** one command diffs any subsystem against the original and
reports where and by how much it diverges. **Substantially met** - `oag-trace`
does exactly this for PPSSPP, on the reference scenario
(`docs/reverse-engineering/ppsspp-debugger.md#the-reference-scenario`). Not
fully met until the same exists for PCSX2/the PS2 asset path.

Its first real run **found and fixed a bug in the harness itself worth
learning from**: the reference scenario's track was misidentified (`01_Track`
assumed by directory-numbering convention; it is actually `16_Track`, settled
decisively by casting the recording's own 200 positions against every track's
collision geometry on the disc and finding only one where all 200 land). Run
against the wrong track, `compare` reported a confident, clean, entirely
wrong finding - "the ship falls through the floor at tick 0" - because it
correctly diffs whatever two runs it's given without checking they describe
the same world. See [oag-game](../tools/oag-game.md#the-suspension-was-never-the-problem-the-default-track-was)
for the full account. With the right track, the tool immediately produced a
real, correctly-isolated finding instead: see M4.

What the capture path has already produced, independent of the above: five
runtime confirmations that were previously static readings, including one
that settles a claim by counting: `craft+0x2ec` matches the **previous**
frame's `|dot(velocity, forward)|` on 199 of 199 samples and the current
frame's on 137, which is the one-frame staleness
[engine](../ghidra/functions/psp-pulse-usa/engine.md) derived from the frame
ordering. See also [frame pacing](../psp/frame-pacing.md).

---

## M4 - Playable core

- [x] wgpu renderer MVP: `oag-render` draws track geometry and ships, and
      `oag-game` puts a ship on a real track with a chase camera, headless or in a
      window - reached from the front end's `Launch Game` in the same window, or
      directly with `--race`. See [oag-game](../tools/oag-game.md).
      **MVP is meant literally, and the stopping point was deliberate**: geometry,
      textures, a camera and the exhaust, with no scenery animation, no authored
      lighting, no environment classes and none of the series' look. A renderer is
      not what a physics comparison measures, so fidelity is [M6](#m6---rendering-fidelity)
      rather than a hidden dependency of this milestone.
- [x] Input: `oag-input` maps devices onto the abstract button layer and produces
      the `InputSnapshot` the simulation consumes
- [x] Chase camera: `oag_render::camera::chase`, its seven parameters taken from
      `<ExternalCameraFar>` rather than invented
- [x] Ship physics **implemented, none of it verified**: thrust, steering, brakes,
      airbrakes, pitch, the air cushion, grip, drag and the passive torques, from
      [physics](../physics/README.md) and
      [engine](../ghidra/functions/psp-pulse-usa/engine.md). Every magnitude is
      transcribed static analysis; only two cross-product signs are settled, and
      those by arithmetic rather than by measurement.
- [x] Collision against track and walls: the [triangle soup](../formats/collision.md)
      decoded and validated on both discs, queried by a segment-triangle
      narrowphase behind an AABB reject. The sweep-and-prune broadphase is not
      implemented, and [the origin its coordinate packing subtracts](../ghidra/functions/psp-pulse-usa/collision.md)
      should be settled before it is - measurement has ruled out the geometry, so
      what is left is a code read.

**Exit criterion:** a single-ship time trial that passes trace comparison for
the full lap, and that feels right to someone who knows the original.

**What "passes for the full lap" has to mean, because the literal reading is
not decidable.** The original integrates the frame duration it actually
measured ([ADR-0007](../architecture/adr/0007-fixed-timestep-vs-original.md)),
and those durations follow host load: driving the *original itself* twice with
the same script and the same pinned start pose, `dt` agrees on about **1 %** of
ticks and the two runs of the original are **100 units apart by tick 495**. A
single-seeded three-thousand-tick trajectory comparison therefore cannot be
passed by any implementation, byte-exact ones included; it measures the
emulator's scheduler. So the criterion is read as two numbers, both from
[`oag-trace`](../tools/oag-trace.md):

- **single-seeded**, how many ticks we track the original before coming apart;
- **`--reseed N`**, the per-window error across the whole lap, judged against
  [the protocol's tolerances](../reverse-engineering/verification-protocol.md#tolerances).

The second is the one that says whether the force law is right everywhere on
the circuit; the first is the one that says whether it is right *enough* to
compound cleanly.

**Closer than it was, and the remaining gap is now one clean, isolated
question instead of three tangled ones.** Trace comparison exists (M3) and has
been run against the real reference scenario. Three things that looked like
separate physics bugs earlier this session turned out to be one measurement
artifact (see M3 above): the suspension, the roll oscillator's stability
margin, and the hover target height were all real physics questions worth the
work put into them, but the specific symptom that motivated chasing them - the
ship losing probe contact and being thrown off within ~166 ticks - was a ship
being flown over the *wrong track's* collision geometry. With the correct
track (`16_Track`, not `01_Track`), the ship **holds both hover probes in
contact for the full 120-tick well-behaved window**, at a steady height
matching what the (independently corrected) spring math predicts to within
0.6%.

**The speed-equilibrium question is resolved (2026-07-28), and the answer is
that the force law was never wrong.** The apparent factor-17 thrust surplus,
later remeasured as a missing linear resistance of `2.28 * fs`, is neither: a
standing-start capture shows the recovered force law reproducing a real
launch from `fs = 0.56` to `47.67` at **rms 0.127 units of force** on a
one-parameter fit, and the deficit in the older captures is a **3.67 %
per-frame reduction of the velocity applied outside the force accumulators**
during sustained wall contact - both old reference captures spent their
entire length scraping a wall, recorded all along in the trace's own `speed`
column (`speed/|velocity| == 1.0000` is now the cleanliness test for any
future capture). See
[the resolution](../physics/force-balance-ground-truth.md) for the
measurement and the instruction-level readings behind it.

**What's left for the exit criterion is the wall-contact response**: a craft
in sustained contact loses ~3.6 % of its speed per frame (decaying from
~4.2 % post-impact toward 3.67 %), applied post-integrate in velocity space -
corroborated on the PS2 build, whose `World_StepBodies` runs its contact
resolver after the integrator with restitution `0.1` and a deferred
per-contact friction queue. `crates/physics/src/wall.rs` must reproduce that
response, from the recovered law rather than the fitted number.

**The ship now stays on the track.** With the recovered contact response
implemented (friction `0.035` from `(0.05 + 0.02)/2`, both literals;
restitution `0.4`; the tangential term `-friction * v_t`), the ship holds the
track for the full 600 ticks, finite throughout, grounded on all 600 and with
no respawns, its worst distance from the spline 27.2 of a 114-unit envelope.
`the_ship_does_not_stay_on_the_track_yet` has been retired for
`a_ship_stays_on_the_track_for_ten_seconds`, which pins that the right way
round.

**Contact generation is no longer the gap, and the paragraph that used to say
it was had been stale for thirty-one commits.** All four of the things it
named are in `crates/physics/src/wall.rs`, each pinned by a unit test and each
measured against the whole-lap scenario on its own commit so the effect is
attributable:

| Recovered behaviour | Effect on the lap scenario |
| --- | --- |
| The ten box sample points (`Collider_BoxSamplePoints`, `0x08818a00`) | landed earlier, with the angular half |
| The angular share of the denominator and the `0.1`-scaled application | landed earlier, with the sample points |
| A contact per (sample point, triangle) pair, and no non-wall hit hiding a wall | **nothing** - both are real divergences this track's geometry never exercises |
| Single-sided rejection instead of flipping the normal (`Collision_BoxAgainstMesh`, `0x08815cd4`) | 617.5 units travelled to 618.6 - the size 99.6 % predicts. Both figures are from the spline-sample-0 spawn the race no longer uses, so the 1.1-unit difference is the finding and the absolutes do not reproduce |

That last row rests on a measurement this pass added rather than an
assumption: **2,076 of `16_Track`'s 2,084 wall triangles (99.6 %) are wound
toward the circuit**, which is the fact the original's rejection gate needs and
which nothing in `docs/` had established. See
`crates/game/tests/race_ground_truth.rs`.

~~**What actually separates M4 from its exit criterion is a ship that gets
wedged**, travelling 618 units of path in 3,146 ticks against the original's
1,045 on the same inputs.~~ **Retired, and it was never the physics.** The two
sides were not being driven by the same inputs: `psp-trace.py --script-lead 2`
never sends a script's first two ticks, and the replay applied them. Two ticks
survive a whole lap because the steering ramp oscillates around a saturated
target rather than clamping to it, so the head start became a standing 17 %
steering offset, 8 degrees of heading by tick 70, and a wall at tick 147 the
original never touches. `oag-trace run --script-lead 2` is the fix; the chain is
measured in
[`oag-trace.md`](../tools/oag-trace.md#the-first-two-ticks-of-a-script-never-reach-the-emulator).

**The lap was recaptured on 2026-07-29 and both comparisons run**, and re-scored
after the input was aligned. Read the comparison over the window where the
*capture* is clean - it first touches a wall at tick 171 and is wall-free on only
32.7 % of its ticks, so nothing later measures our physics:

| Ticks 0-170 | `--script-lead 0` | `--script-lead 2` |
| --- | ---: | ---: |
| Position, max error | 22.85 | **6.54** |
| Position, mean error | 6.12 | **2.41** |
| Speed, max error | 75.44 | **6.01** |
| Speed, mean error | 9.97 | **2.75** |

Under `--reseed 60` the position figure is **10.3** at tick 299 either way - a
60-tick window never accumulates enough to show the offset - while the worst
orientation axis goes 0.114 to **0.0824** rad. `grounded` is exact on 3,146 of
3,146 ticks.

**A fresh clean-lap capture exists (2026-07-30), and it moves the ceiling on
number 1 from 171 to 256, not further.** `oag-trace show`'s cleanliness report
(added this pass, see [oag-trace.md](../tools/oag-trace.md#show-and-the-cleanliness-report-in-it))
put a number on the old capture's 32.7 %/tick-171 figures for the first time;
a fresh `just autopilot` lap (`data/traces/talons-junction-clean-lap.csv`,
2,977 ticks) is dramatically cleaner overall - wall-free on **96.3 %** of its
ticks - but still first touches a wall at tick **256**. A second attempt
chasing the lifted centre line instead of the authored racing line
(`--column lift`) landed at tick 249, no better; per the plan this pass worked
to, two attempts with no material gain in the clean prefix is itself the
result, not a reason to keep tuning the controller.

**And 256 ticks of "wall-free" is not 256 ticks of "physics agrees" - the wall
stops being the binding constraint well before that.** Scored with
`--script-lead 2` (confirmed correct the same way the 2026-07-29 row above
did: `throttle`/`brake` are bit-exact across all 2,977 ticks only at lead 2),
single-seeded position error over the *same* ticks 0-170 window as the row
above is **10.12** max / **2.50** mean - consistent with the older capture's
6.54/2.41, a different lap on the same track section rather than a
regression - but over the fuller 0-255 clean window it is **47.76** max (tick
255) / **13.28** mean, because error is already growing fast by tick 200
(17.5 at tick 180, 44.1 at tick 240) while the ship is still nowhere near a
wall. The whole-run single-seeded figure is 1,245 units at tick 1,628 and
still growing, the same character as before: not comparable tick-for-tick
against a different capture's trajectory, and not meaningful past a few
hundred ticks regardless.

Under `--reseed 60` on this capture, position error is **28.86** at tick 1,739
(bounded, not growing) and the worst orientation axis is **0.4858** rad at
tick 1,129 (also bounded) - both markedly larger than the 2026-07-29 capture's
10.3/0.0824. **Checked, not just read, before attributing this to the lap's
own content rather than a regression**: two things changed between the two
figures, the capture *and* the physics (the collision-box scale fix, above),
so the 2026-07-29 capture was re-run under today's physics to isolate them.
Its worst reseeded orientation axis today is **0.0541** rad - a modest shift
from the 0.0824 it read before the collision fix, not the 6x jump the new
capture shows. That rules the physics change out as the cause: the new
capture's larger error tracks its own content, holding racing speed
(100-150 units/s) and cornering hard throughout where the older capture spent
most of its length slowed and wedged against a wall, and per-window error
scales with how aggressively the craft is maneuvering.

**So the honest read of number 1 is that the wall was never the whole
story.** Closing the 171-tick ceiling to 256 removed the wall as the limiting
factor earlier than expected - by tick 200-255 the force law itself, not
contact geometry, is already tens of units off on a clean line. That is a
different, harder problem than the one this pass set out to close, and it is
now the one blocking number 1's headline figure from moving materially further
without physics work, not another capture.

The other live gap **was our hull responding to a wall about six ticks before
the original does; it is now three-to-five ticks after.** Measured against the
original's own recorded poses rather than a replay - so it is contact geometry,
not accumulated trajectory error. The cause was a missing `0.75` scale on the
`<Misc>` dimensions feeding the collision box (`Ship_InitCraft`,
`crates/physics/src/wall.rs`), corrected 2026-07-30; fixing it moved the gap to
the other side of zero rather than closing it, by a different amount on each of
the two captures, which reads as an approach-angle-dependent residual rather
than a further scale error. See
[`oag-trace.md`](../tools/oag-trace.md#our-hull-met-that-wall-six-ticks-early-the-fix-overshot-into-three-to-five-ticks-late)
for the full account and what not to try next.

**What `grounded` agreeing on every tick does and does not say.** The original's
column is `1.0` on all 3,146 ticks of this capture, so the agreement means *our
ship also never leaves the ground* - which the 2026-07-28 run could not manage
(it read `0.5` against the original's `1.0`, then left the surface for good at
tick 313). That is a real improvement and it is **not** evidence that the hover
model quantises contact the same way the original does; a constant column cannot
test that. Under `--reseed` it says even less, because `grounded` is one of the
fields the seed restores, 52 times a lap.

The hypothesis that died first, before the wedge itself did: the run report's
**21 rad/s** average angular velocity looked like a craft spinning three and a
half times a second. **The original reads 22.96 rad/s on the same scenario**,
slightly higher than ours. Both sides do the same thing with that column and it
is not the cause.

One caveat on the capture, because it changes what a *clean* number would take:
the recapture fails the `speed/|velocity| == 1.0000` cleanliness test on 67.3 %
of its ticks, where the 2026-07-28 capture of the same script was 95.2 %
wall-free. The script is a closed-loop autopilot recording and replaying it
open-loop drifts into the walls - the same reproducibility negative as above,
seen from the authoring side.

---

## M5 - Full race

- [~] Race rules, grid, lap timing, positions. **Lap timing and the three
      single-ship modes are done**, in the new `oag-race` crate: time trial,
      speed lap and Zone, selectable on the RACE menu page. Lap counting is a
      wrap of a travel-ordered ring walked from the track's junction graph -
      the loop's shape is the original's own traversal at confidence 88, but
      *where the lap begins* is ours at 55, so read
      [lap counting](../gameplay/lap-counting.md) before trusting a lap time.
      Zone's ten-second step and its speed law are recovered
      ([zone-mode.md](../ghidra/functions/psp-pulse-usa/zone-mode.md)) and its
      numbers are read off the disc. **Zone now ends**: the pool empties, the
      craft blows up for half a second, and `Ship_SetState` case 5 sets the bit
      `Zone_UpdateRacing` watches - which is the bit that page recorded as set by
      nothing findable. What is absent is the *presentation* of it, the explosion
      and the camera hand-off. **The grid geometry is recovered and measured**
      ([grid.md](../ghidra/functions/psp-pulse-usa/grid.md)), but none of the
      three modes here spawns opponents onto it, because none of the three
      races with any in the original either - `AI DIFFICULTY` greys to `N/A`
      for all three on the real Custom Race screen, confirmed live for every
      race type. See [race-modes.md](../gameplay/race-modes.md). The AI item
      below is what a fourth, opponent-bearing mode is waiting on.
- [~] HUD. **The disc's own layout is parsed and drawn.** All five
      `Data\XML\*_HUD.xml` layouts decode - exact rectangles, atlas UVs, colour
      constants, font roles - and a race draws the speed and shield bars, the lap
      block, the three clocks and the wrong-way warning through them, in the
      disc's `PulseHud`/`small` fonts off its own `PulseHUD.mip`. Verified against
      a PPSSPP reference frame. What it does **not** have is anything to show for
      lap or position, because neither exists yet - the widgets are omitted
      rather than filled with invented values. **The shield bar is no longer in
      that list**: the pool depletes on wall contact as of the shield work
      below, so `ShieldBar` and `ShieldBarText` are live readouts rather than a
      constant 100 %. See
      [hud.md](../ui/hud.md) for the four named gaps and what is scoped out.
- [x] **A race draws the map.** It used to draw the *driveable ribbon* - a flat
      coloured band over the spline - with the art meshes behind an opt-in
      `--art`. That default was deliberate and was right while M4's whole job was
      verifying a force law: the ribbon is the geometry the simulation spawns on
      and queries, so ship-plus-ribbon shows directly whether the ship is where
      the physics thinks it is, where art meshes are "prettier and prove less".
      It stopped being right once there was a game to look at, because `just play`
      showed a player a debug view of a spline.
      **Inverted**: art meshes are the default and `--ribbon` is the development
      view, alongside `--collision` which was already one. 144,351 triangles on
      `16_Track`, textured, with buildings, barriers, road markings and scenery.
      The PS2 path now draws the same geometry **textured** too: the texture
      set is the archive entry directly before the track's own `.vex`, the
      same directory-position rule already solved for ships, checked
      independently against all 32 `<n>_Track`/`track_reversed` models (27
      fill every slot, the other 5 are short 1-2 slots and the load report
      says so rather than guessing at a fill). See
      [ps2-texture](../formats/ps2-texture.md).
      A **graphics-menu row** for it is still open; today it is a flag only.
      What still makes a race look unlike the original is authored lighting, sky
      and fog, and those are [M6](#m6---rendering-fidelity).
- [x] **Speed pads**, decoded, placed, drawn and boosting. `Speedup Pad` `0x3bd`
      is a `Mesh` subclass, so its geometry ships inside the track file (see
      [pads](../formats/pads.md)); the trigger is `oag_game::race`'s reimplementation
      of the original's per-racer distance cache, and the force is step 15 of
      `Ship_UpdateCraft` in `oag_physics::engine::speedup_pad`, reading
      `<GlobalClass><SpeedupPads amount time/>` off the player's own disc. Zone
      pays +100 per new pad, and `Exhaust::boost` is armed on the same edge.
      **One branch of the original is deliberately absent**, gated on an
      unidentified bit: the `craft+0x2cc` fade-in. The `speedpad_jump` tilt that
      used to sit beside it is **applied** - `craft+0x160` is the hull up axis
      and `controls->0x24 & 1` is d-pad Up, both read live, and the magnitude
      comes off the disc as `<Global><Special speedpad_jump>`. It is a
      5.71-degree tilt and 0.5 % more force, not a jump.
      The field-of-view kick is an **authored** effect, not a recovered one -
      `[graphics] boost_fov_kick` is a magnitude (0, 8, 16, 32), 0 turning it
      off, with a `BOOST FOV KICK` row on the graphics menu page.
- [x] **Per-class gravity.** `<GlobalClass><GravityMul airborne/>` fills
      `g_class_gravity_scale`, read off the disc rather than defaulted to the
      identity. Despite the attribute's name it scales the **grounded** term;
      see [handling stats](../formats/handling-stats.md). This corrected two
      physics pages that recorded the shipped table as `1.0` for every class.
- [~] Weapons and pickups. **The pads are drawn** as of 2026-08-10 - `Weapon Pad`
      `0x3be` is a `Mesh` subclass like the speedup pads, so it is the mesh
      builder pointed at another class id (738 triangles on `16_Track`), and its
      trigger volumes decode and are carried. A ground-truth test checks the two
      against each other, the geometry coming through the mesh path and the
      volumes through the payload path. **And the weapon table turned out to be
      authored data**, not a table compiled into the executable:
      `Data\XML\WeaponStats_Race.xml` carries all fourteen weapons' tunables,
      the seven disturber effects, and the pickup distribution weighted per
      speed class and separately for AI, human, front and back of the grid. See
      [weapon stats](../formats/weapon-stats.md). That makes the item a format
      decode plus behaviour rather than a recovery.
      **A pad now hands out a pickup, and Turbo is the one weapon that does
      something** (2026-08-11). See [pickups](../gameplay/pickups.md), which
      carries the split that matters here: the *trigger* is recovered at 90 -
      `WeaponPads_TestCraft` stamps `<WeaponPad refresh_time>`, a per-class
      debounce now parsed - the odds table is recovered at 92, `SQUARE` fires
      and `CIRCLE` absorbs at 90, and the `1.2` engine multiplier a fired Turbo
      arms is recovered at 85. **The grant itself, the weighted draw and the
      one-slot inventory are this project's**, because no pickup-grant call site
      exists anywhere in the executable and `craft+0x1c0`'s bits are still
      unread. The original's PRNG being unrecovered means only the
      *distribution* can ever be checked, never the sequence.
      **This needed a fourth mode**, because a weapons-off race in the original
      hides every pad and empties the trigger list - so none of the three modes
      here could exercise a pickup at all. `Mode::SingleRace` is it: weapons on,
      `Arcade_HUD.xml`, and no opponents until the AI lands.
      **And a time trial's free turbo is in**, on two independent shipped
      records rather than one - `MSC_EVENT_TT`'s *"free turbo pickup once per
      lap"* and `TimeTrial_HUD.xml` authoring a `PickupBackground` and
      `TurboIcon` alone where Zone authors none and Arcade authors all thirteen.
      **Shield and Rocket followed the same day**, taking the draw pool to
      three. Shield is the Turbo's shape - the disc's `<Shield time>` on a timer
      that makes `damage::apply_contact` refuse - and **Rocket is the first
      projectile**: `oag_gameplay::projectile` flies a fixed-size array of them
      straight-line, sweeps each tick's step against the track and against
      hulls, and spends `<Rocket damage>`/`blastforce`/`blastradius` through a
      new `damage::apply_weapon` that shares `Ship_Damage`'s recovered body with
      the contact path. A ground-truth test fires one on the disc's own geometry
      and watches it detonate. The per-class flight speeds are the file's own -
      it authors four.
      **Almost all of it is ours**, and pickups.md's table says which parts: no
      firing call site, no projectile class and no flight update exists anywhere
      in the executable, so straight-line flight, the launch offset, a sphere
      for a hull, full damage inside the radius, and one rocket per fire are all
      this project's readings on top of the disc's numbers. `spread` argues for
      a volley and is recorded as an open question rather than dismissed.
      **The determinism gate widened to match**: `oag_gameplay::hash::hash_world`
      plus `Race::state_hash` now cover the inventory, the projectiles, the pad
      refresh timers and the generator's own position, which were the known hole
      pickups.md recorded when the pickups landed. Both halves run in CI on all
      three platforms and neither needs a disc.
      **Ten weapons still hand out nothing**: Missile needs a lock and a target,
      Quake track deformation, LeachBeam a beam, and most of the rest the
      slowdown mechanic behind `<Global slowdown_limit>`, which has no consumer.
      Autopilot is the AI's own controller taking over and belongs with the AI.
      **And a rocket has almost nothing to hit**, which is the AI's fault rather
      than the weapon's: `Mode::has_opponents` is unconditionally `false`, so
      track geometry and the `--opponents` parked grid are the whole target
      list.
- [~] **Shield and energy.** The pool, its maximum and the one thing that spends
      it are recovered and implemented: `*(craft+0x1c4) + 0x88` is the pool,
      `<Misc>`'s three difficulty slots at stats-base `0x84 + skill * 4` are its
      maximum (a PS2 *writer* and a PSP *reader* agreeing, confidence 88), and
      wall contact costs `|p| * 0.05 * 0.7` of it through `Ship_Damage`
      (`0x088439ac`) - **halved whenever the race has weapons off**, which a
      time trial does. `oag_physics::damage` is the port, the pool is hashed by
      the determinism gate, and the HUD's `ShieldBar` now moves. See
      [shield](../ghidra/functions/psp-pulse-usa/shield.md).
      **Weapon damage and weapon absorb are in** as of 2026-08-11:
      `damage::apply_weapon` is `Ship_Damage`'s body given an authored amount
      rather than a scaled impulse, and `CIRCLE` pays `<Stats absorb>` through
      the recovered clamp. **The pit-lane recharge is the one left**, and the
      destroyed transition - `Ship_Damage` sets craft state 4 at zero - still has
      no state to move to, so the pool floors instead and
      `damage::Shield::depleted` is the signal to build it on. A fired Shield
      suppresses that signal for its duration, which is a thing Zone's unbuilt
      end condition will have to account for. **Measured against the running original** for the pool's location,
      the two race-option globals, the regeneration branch and the weapons-off
      halving - which comes out at 1.98 against a predicted 2.00 - and for the
      **coefficient itself**, which measures `0.035000` exactly on 25 of 25
      `Ship_Damage` calls spanning a factor of 225 in magnitude. The one claim
      left unmeasured is `entity + 0x368`, the field gating the damage path.
- [ ] AI
- [ ] Audio, including the **positional** classes a track authors:
      `sound` `0x3e1`, `soundcone` `0x3e9` and `speaker` `0x3cc`. The banks and
      waveforms decode already (M1); what is missing is placing them in the world.
      **Music switching is in**: a race plays a cycling playlist through the
      sixteen soundtrack tracks in place of the menu's loop, its position kept
      in memory across races so leaving and re-entering one resumes rather than
      restarts. Authored, not recovered - see [ps2-audio](../formats/ps2-audio.md)'s
      "Not determined" section, which this does not close.
- [ ] **The visuals these systems own.** Effects that cannot be built before the
      thing they belong to, so they sit here rather than with the rendering work
      in M6. Each is a `.vex` class the loader already enumerates - see the
      [class table](../formats/vex.md):
      - `Ship Muzzle` `0x3e2` and `cannon_flash` `0x3eb` - weapon firing
      - [x] `Ship Collision Fx` `0x3d0` - **wall/track impact sparks, currently
        an authored effect rather than a reading of this class.** The class's
        own registration path is a dead end (see below), but the actual
        trigger, `ShipCollisionFx_Trigger` (`0x089246b4`), was found and read
        in full (2026-07-31): it names the three real spark resources by
        string and computes a usable severity formula from the same contact
        impulse magnitude `contact-response.md` already recovers - see
        [contact-response.md](../ghidra/functions/psp-pulse-usa/contact-response.md#shipcollisionfx_trigger-0x089246b4-is-the-actual-spark-spawn-function).
        `oag_render::sparks` still triggers off our own physics contact data
        (`oag_physics::wall::WallResponse`) rather than this class, and has
        not yet been retuned against the newly recovered formula - see that
        module's doc comment and the open thread on `HANDOVER.md`.
      - `Cage Collision` `0x3e7` is **out of scope**: PSP-over-PS2 policy, and
        it has 0 nodes on the PSP disc (6 on PS2). Scrape effects for it are
        not built. `Data\visual_effects\cage_collision_curved.vex` is the
        authored asset for whoever picks this up on the PS2 side
      - `Quake` `0x3c7` - the quake weapon's track deformation
      - Shield hit response, whose model is `Data\Ships\<Team>\<Team>shield.vex`
      - [x] `shipboost.vex`, the boost ship state - **the additive plume a
        speed pad reveals**, recovered and drawn (2026-08-04); see
        [`exhaust.md`](../ghidra/functions/psp-pulse-usa/exhaust.md)'s "boost
        visual" section and `oag_game::race::Loaded::boost_model`. Two meshes,
        drawn once in ship space with no locator mounting - the file's own
        node tree carries no `Transform` for either mesh to mount on.
      - `shipwreck.vex`, the damage ship state, still open. **Note the naming
        trap**: both resolve through the FE team-model name, whose default is
        the literal `"ship"`, so it is `shipwreck.vex` and not
        `Assegaiwreck.vex` - guessing the team name into the template 404s

**A default run should be the production run**, and the defaults were audited
against that on 2026-07-30. The art-mesh flip above was the only thing a player
was getting the wrong side of; everything else already defaults the way a
release would want it:

| Default | Value | Why it is right |
| --- | --- | --- |
| Track model | **art meshes** | changed above; `--ribbon` is the dev view |
| Anisotropic filtering | **16x**, the maximum | a user turns filtering *down* for performance, never up, so defaulting low leaves the mip work unused |
| Render scale | 100 % | below is internal resolution, above is supersampling |
| Collision overlay | off | a wireframe diagnostic |
| Performance overlay | off | a diagnostic, not decoration |
| Menu backdrop video | on | `--no-video` is opt-in |
| HUD | on | drawn from the disc's own layout |

The one thing still off that a player would want is the graphics-menu row for the
track model, noted above. Anything added here later should be checked the same
way: a flag that only a developer wants defaults off, and a flag that changes
what a player sees defaults to whatever the original did.

**Exit criterion:** an eight-ship race that is indistinguishable from the
original to a player, and whose per-tick trace stays within tolerance.

**Grid formation is recovered, and the authored node turned out to be the back
of the grid rather than a lone slot.** A track's `Start Position` is **slot 8**:
this project's spawn from it lands 1.84 units from where the original puts its
own eighth craft, against 139.7 from where a time trial starts. The other seven
run forward from it in two staggered columns - `19.79` per slot along forward,
alternating `20.0` along row 0, all eight sharing one heading - measured off
eight craft read out of the running original while the countdown held them in
place. `oag_gameplay::spawn::grid_pose` is the port. Which ship gets which slot
is a shipped permutation table, and a short field packs to the *back*. See
[grid](../ghidra/functions/psp-pulse-usa/grid.md).

**Eight craft can take the grid**, and every slot lands within **2.40 units** of
where the original puts its own - a near-constant residual dominated by our
anchor being 1.68 out, which is what says the layout is right rather than
averaging out. `crates/game/tests/race_ground_truth.rs` pins it against the
original's own eight positions. "Can" rather than "does": none of the three
modes this crate implements races with opponents in the original either, so a
real race spawns the player alone - `race::Options::opponents` is what the
ground-truth test above sets to exercise the full grid without a fourth,
opponent-bearing mode existing yet.

**Nothing drives the seven opponents, when a build asks for them at all.** They
hold station on the grid, and they all wear the player's hull. That is the AI
item and the livery item, not the grid one.

---

## M6 - Rendering fidelity

Everything that makes a race *look* like the original, as opposed to being
shaped like it. M4 built a renderer MVP - track geometry, ships, a chase camera
and the exhaust - and deliberately stopped there, because a renderer is not what
a physics comparison measures. This is the milestone that closes the gap.

**It is its own milestone rather than a line in M7 for two reasons.** It is large
enough that burying it beside menus, save data and PS2 parity would make that
milestone's exit criterion undecidable; and it is almost entirely independent of
gameplay, so it can proceed in parallel with M5 rather than behind it. The
effects that *are* gameplay-coupled are in M5 above, on purpose.

**The work list is not invented.** The `.vex` loader enumerates ~55 authored
classes and [the class table](../formats/vex.md) groups them; what follows is
those groups, minus what M4 and M5 already cover. So "full fidelity" here means a
specific, countable set of authored things that a track or ship declares and this
engine currently ignores - not an open-ended wish for it to look nicer.

Two cautions carried over from
[`vex.md`](../formats/vex.md) and [HANDOVER](../../HANDOVER.md), because they
change how this list should be read:

- **"Has instances" and "has a handler" are independent properties.** `engine_fire`
  and `exitglow` have no registration site at all, yet `exitglow` is authored **13
  times** on `16_Track`. An unregistered class can still be authored, and a
  registered one can be authored nowhere.
- **An xref count on a class descriptor bounds *authored* instances only.** Code
  that builds an object directly never touches the descriptor - which is how the
  exhaust pass concluded racing craft have no trail, when the emulator plainly
  shows one. Look at the running game before concluding a visible thing is absent.

### Scene, animation and culling

- [ ] `Anim Transform` `0x3c0` - the authored animation channel. This is the one
      that makes scenery move, and nothing reads it yet
- [ ] `animationTrigger` `0x3dc` - what starts an animation
- [x] `LodGroup` `0x2ee` - confirmed (see `docs/formats/vex.md`, "`LodGroup`:
      authored, but never switched at runtime") that the original never
      selects a tier at runtime: both children of a `child_count == 2` group
      are always drawn, ten times over on `16_Track` alone. `oag_render::mesh::Lod`
      (`[graphics] lod` in the settings file: `both`/`single`) offers a
      load-time choice between reproducing that (default) and keeping only the
      higher-detail tier - a static deduplication, not a distance-based switch.
      A genuine live switch needs the same per-frame camera mechanism as
      frustum culling below, and is not built
- [x] **PVS culling** off the track's own `section` `0x3c9` payload. The
      payload is now [decoded](../formats/track.md) (`oag_formats::pvs`,
      validated against every track file on both discs) and drawn with:
      `oag_render::pvs` intersects each draw call's bounding sphere with the
      authored section boxes at load, and the race loop tests that mask against
      the sections visible from the craft's and the camera's own sections
      before the frustum test runs. **On by default**
      (`[graphics] pvs_culling`), having cleared the same bar frustum culling
      did - thirty captures across three tracks, several tick counts and every
      combination of the two tiers, all byte-identical to culling nothing.
      Cuts what reaches the frustum test by 22-66% depending on the track -
      about half on a median one, measured over all 40 of the PSP disc's track
      files - and by nothing at all in the worst section of nearly every one.
      See
      [ADR-0011](../architecture/adr/0011-authored-pvs-before-frustum-culling.md),
      which also records the cheaper association rule that was tried first and
      lost background scenery, and why our own batch granularity - not the
      authored data - is what caps the saving
- [ ] `MeshNode_Ghost` `0x3d4` - ghost rendering. Needs replay data, so it lands
      with replay in M7 if that comes first
- [x] **Frustum culling.** Not an authored class - a plain engine technique.
      `DrawCall` now carries its own world-space bounding sphere
      (`mesh::Bounds`), tested per frame against the camera's view frustum
      (`oag_core::math::frustum::Frustum`, unit-tested Gribb-Hartmann
      extraction) before `draw_indexed`; applied only to the track (~2,000
      draw calls), not the ship or the collision overlay, since both carry a
      non-identity model matrix the bounds are not valid against without an
      extra transform not yet written.
      **On by default** (`[graphics] frustum_culling`, config-file only, no
      menu row). Measured on `16_Track`, the test itself costs ~59
      microseconds a frame to check all ~2,000 draw calls, a real,
      unconditional cost; whether the GPU-submission saving is worth more than
      that depends on the GPU - confirmed as a real, noticeable improvement on
      a weak integrated GPU, unconfirmed on anything with more headroom to
      spare. A pixel-identical screenshot comparison with the setting on and
      off found zero visual difference, so the default is a bet on which GPU
      is more common, not a correctness question. Worth re-measuring once more
      is drawn per frame at once (weapons, other ships, effects), where the
      saving should only grow.

### Lighting and shadow

- [~] `AmbientLight` `0x12c`, `DirectionalLight` `0x131`, `PointLight` `0x132` -
      **decoded and investigated; not built as a render feature.** All three
      payloads are parsed (`crates/formats/src/lighting.rs`,
      [`lighting.md`](../formats/lighting.md)); a Ghidra pass
      ([`lighting.md`](../ghidra/functions/psp-pulse-eu/lighting.md)) found
      `PointLight` has no `Vex_RegisterClass` call site at all, and
      `AmbientLight`'s decoded colour never reaches the render path
      (live-verified against two tracks with sharply different authored
      colours) - both confirmed inert on the original, not a gap to fill.
      `DirectionalLight` is different from the other two: registered, and
      **live-verified collected into a real, correctly-populated track-load
      list** capped at exactly the hardware light count (4) - a breakpoint
      capture matched three independent per-class counts against the disc's
      own authored data exactly. **Three passes (one live capture, two
      Ghidra, one time-boxed and one broader) converged on not finding a
      downstream reader** - the class-based lookup path is exhaustively
      closed and every plausible one-hop consumer checked clean, leaving
      only a direct trace of `World_LoadTrack`'s own 1,228 instructions
      unexplored, twice judged not worth known tooling blind spots on that
      function. A genuine converged negative, not an abandoned search.
      Building a hardware-light-style GPU rig, this item's original
      assumption, is not supported by the mesh-draw evidence either way
      regardless of how that last thread ends: no mesh-draw function any
      pass reached ever enables a `GU_LIGHT0..3` slot, so even a confirmed
      `DirectionalLight` reader would consume the list some other way, not
      through the hardware slots this item originally assumed. See
      `HANDOVER.md`'s "M6 authored lighting" thread for the full trail.
- [ ] `Dynamic Point Light` `0x3c2` - the moving lights, ships included
- [ ] `Dynamic Shadow Occluder` `0x3c3` and `shadow` `0x3cb`
- [ ] `lensflare` `0x3de`
- [ ] Reconcile with the prelit path. `GpuVertex.lit` already selects per vertex
      between the light rig and baked vertex colour, and the track ribbon is
      prelit today - so this is a question of which surfaces are which, not of
      adding lighting from nothing

### Environment

- [x] `Skycube` `0x3c6` - **done at confidence 90**, and cheaper than expected:
      the payload turned out to be a `Mesh` `0x125` payload, so
      `vex::mesh_materials`/`mesh_batches` read it unchanged and
      `oag_render::mesh::build_sky` is the mesh builder pointed at a different
      class id. Every circuit authors exactly one, parented to the world node,
      with its geometry and texture ordinals inline; material counts are 1, 5 or
      6 and are **not** face counts - geometry is 474-553 triangles whichever it
      is, so what separates the groups is still open. Drawn first in the race pass, camera-centred, `depth_compare:
      Always` with no depth write, exempt from both culling tiers. Validated over
      all 40 sky nodes on the disc. **The handler is still unrecovered**, so the
      GE state is chosen rather than measured. See
      [`skycube.md`](../formats/skycube.md)
- [ ] `fogCube` `0x3d3`, and the fog *curve* separately - see the look below.
      The **payload is decoded** (a 64-byte 4x4 defining a box volume, then two
      `{rgb, 0, near, far}` sets; 36 of the 40 track files author one, and
      `06_Track` authors none) and the per-track colours corroborate the sky
      decode independently. **The runtime path is now fully recovered too** -
      `FogCube_RegisterClass`/`_Init`/`_Sample`, `Fog_FindVolume`, `Fog_Apply`
      and `Gu_Fog`, in
      [`fog.md`](../ghidra/functions/psp-pulse-usa/fog.md) - including that the two
      parameter sets are the ends of a lerp across the volume's local Z. Nothing
      draws it yet, which is why a race now has its authored sky and a hard
      horizon
- [ ] `cloudCube` `0x3d8` and `cloudGroup` `0x3d9`
- [ ] `sea` `0x3d5`, `seareflect` `0x3d7`, `seaweed` `0x3d6`
- [ ] `weatherPos` `0x3da`
- [x] Trackside animated textures, as a V-axis scroll over a banded texture -
      the same mechanism as the ship blink lights, generalised. Eight track
      textures across five circuits, `col_display7_GLOW` on all twelve. Keyed by
      an enumerated list rather than a name rule, because the format holds no
      authored rate (material `+0x0c..0x14` is zero everywhere) and the material
      `flags` word does not separate animated from static. **At confidence 65,
      behind `[graphics] animated_textures`**: the geometry and texture content
      are measured, but no capture has confirmed the original animates these
      surfaces. See [`vex.md`](../formats/vex.md) and
      [`texture-animation.md`](../ghidra/functions/psp-pulse-usa/texture-animation.md)
- [x] Confirm those eight against the running original - **done, and negative**.
      `Gu_TexOffset` never carries a non-zero offset except from
      `Trail_DrawRibbon`, on a circuit carrying two of them, and five candidate
      palettes were byte-static. The original does not animate track surfaces by
      moving texture coordinates, so the setting defaults **off**
- [ ] Find what *does* drive the ship lights' measured pulse. It is neither a UV
      offset nor a CLUT scroll; an animated per-draw colour is the untested
      candidate, and `Trail_DrawRibbon` already sets one through `FUN_0881125c`
- [ ] Continuous UV scroll, as distinct from the filmstrip above:
      `Plasma_scroll_ADD_GLOW` on `16_Track` is one tile with all rows distinct,
      so it slides rather than cycles. The exhaust's trail reads authored scroll
      rates for its own layers, and no track equivalent has been found
- [ ] The `0x2000` extra pass and its second texture index at material `+0x08` -
      it lands on exactly the `*_shinemap` textures, and nothing draws it

### Ship visual state

- [x] `Airbrake` `0x3c5` - **done**, and the file turned out to say more than
      this entry assumed. The class sits between the hinge `Transform` and the
      flap `Mesh` in every playable team's `Ship.vex`, with the two locators
      mirrored in X, so it is a real hinged subtree rather than a marker;
      `<AirbrakeGraphics>` supplies the deflection and two rates that are
      **not** the force law's `gain`/`falloff`. What is *not* recovered is the
      rotation axis: the node's payload is zero bytes and its class descriptor
      carries no handler, so it is chosen and flagged as chosen. See
      [`ship-parts.md`](../ghidra/functions/psp-pulse-usa/ship-parts.md)
- [ ] `engine_fire` `0x3e5` and `exitglow` `0x3e4` - both authored, neither
      registered; see the caution above
- [ ] Livery and team variants across all twelve teams. No longer blocked on
      name mining: the four teams whose `Ship.vex` did not resolve are
      [downloadable content](../formats/dlc-pack.md), the packs are mounted, and
      all twelve are loadable. What is left is the *variants* - each
      `PI_TeamModel` and `PI_ModelSkin` a definition declares, which nothing
      collects or draws yet

### Particles

- [ ] `ParticleSystem` `0x3c4` - the general system. `Trail` `0x3c8` is done
      (the exhaust) and is the worked example of one authored effect recovered
      end to end
- [ ] `blob` `0x3e0` and `textureBlob` `0x3df`

### The series' look

This group is different in kind from the others: it is not a class to implement
but a set of judgements, and
[`rendering/README.md`](../rendering/README.md) states the principle -
**reproduce the output, not the pipeline** - along with the exception that
matters here. Some of the original's look comes from its *limitations*, and where
such an artefact is part of the game's identity rather than an accident it gets
reproduced deliberately and documented as a choice.

- [x] The fog curve - **resolved, and it is not a curve in the ramp**. `Gu_Fog`
      (`0x08811748`) is flatly linear; what varies is its *parameters*, which
      `FogCube_Sample` re-interpolates every frame from the camera's position
      across the fog volume's local Z. Reproducing the look needs the volume and
      the lerp, not a shaping function - see
      [`fog.md`](../ghidra/functions/psp-pulse-usa/fog.md)
- [ ] Bloom and the bright-pass on the exhaust and lights - **the original's is
      fully recovered**, so this is a port rather than a look choice: a
      destination-alpha bright pass at half resolution, a separable 11-tap blur
      (`20 30 40 50 64 64 64 50 40 30 20`, gain `1.85` per axis) and an additive
      composite at `175/255`. See
      [`bloom.md`](../ghidra/functions/psp-pulse-usa/bloom.md). The prerequisite
      is a scene target with a meaningful alpha channel, which nothing writes yet.
- [ ] Colour grading
- [ ] Motion blur / speed streaking. Visible in the reference frames captured for
      the HUD, so this is observed rather than assumed
- [ ] Decide, and document as choices: dithering, and affine texture-mapping
      artefacts. Both are hardware limitations; both are arguably identity
- [ ] Retro-versus-modern as an explicit setting axis, if any of the above is
      contentious. The screen-space exhaust sprite is already costed against this

**Exit criterion:** a still frame from a race, at the original's aspect and with
the same craft, track and camera, is hard to tell apart from a PPSSPP frame of
the same place - and every deliberate departure is written down as a departure
rather than being absent.

**Why a still and not a video.** Frame-by-frame equivalence is not decidable here
for the same reason M4's lap comparison is not: the original integrates a
measured frame duration, so two runs of the original itself diverge. A still
frame at a matched pose is decidable, and the HUD work proved the method - a
screen-space comparison caught a font bug that every unit test passed. See
[`docs/ui/hud.md`](../ui/hud.md#what-a-reference-frame-settled) for the capture
recipe and its three traps.

---

## M7 - Shell and polish

- [~] Menus and the front end. **The shell exists and navigates**: our own
      definition format in `assets/ui/menu.toml`, a page tree with working
      Race, Options, Display, Graphics and Controls pages, reached from `Launch Game`
      and able to start a race. Every row is live - speed class, team and
      circuit are persisted and applied, anisotropic filtering is written to
      `settings.toml` on the keypress, and the chosen language skips the picker
      on the next run. Circuits and their names come off the player's own disc
      rather than out of the asset. **Escape backs out one level** rather than
      quitting: a race returns to the menus, a page pops, and only the front
      end and `--race` quit on it. See [menus](../architecture/menus.md).
      The rows are drawn over **the disc's own looping menu backdrop**, the
      movie `FE Screen`'s `Movie` widget names, so the menus sit on the
      background the original's do. Style, animation and the rest of the disc's
      artwork are still absent, and **rebinding** is the one row that displays
      without editing.
- [~] Modern features. The display half is in: **monitor selection** (by name,
      never by index), window mode (windowed or borderless), window size,
      aspect ratio (`psp`, `ps2` or `free`), **brightness and gamma**, and a
      **render scale** from 50 % to 200 %, the last of which is the
      internal-resolution knob below 100 and supersampling above it. Every
      stage draws into an offscreen target and one pass stretches it into the
      aspect rectangle, which is also where the letterbox bars and the
      brightness/gamma grade come from - one pass, so every stage is covered
      and a screenshot deliberately is not. A **field of view** row is in as a
      percentage of the disc's authored value rather than an angle, because
      an absolute angle would name a field the game only shows while
      stationary - the original widens it with speed. (Until 2026-08-09 the
      reason was that the unit itself was unrecovered; it is vertical degrees at
      confidence 94, so this is now a presentation choice rather than a
      blocked one.) The settings are split into `[display]` and
      `[graphics]` to match the two menu pages, with an older `[graphics]`-only
      file migrated on load. See [menus](../architecture/menus.md) and
      `oag_game::display`.
      **Frame pacing is in too**: a three-way vsync (off, on, or `smooth` - the mailbox present mode, which is triple buffering done properly), an **unlocked or limited frame rate**
      (unlimited, or up to 1000), and a performance overlay that shows the rate,
      the mean, the 99th percentile and a per-frame graph - because an average
      frame rate is exactly what hides uneven frames. `oag_game::perf`.
      **Spatial upscaling is in**: FSR 1 (EASU then RCAS), ported to WGSL from
      AMD's MIT source, behind `[graphics] upscaler` and off by default until a
      wider screenshot comparison earns the change. It runs only where it is
      magnifying. `oag_render::post::fsr1`.
      HDR, VRR, temporal upscaling and Steam Input are still open -
      [modern features](modern-features.md).
- [ ] Save data
- [ ] Replay
- [ ] PS2 asset path at parity with PSP
- [~] Modern features: ultrawide, unlocked frame rate, HDR, VRR, dynamic
      resolution, upscaling, Steam Input, optional ahead-of-time FMV upscaling -
      licensing status and the architectural prerequisites are recorded in
      [modern features](modern-features.md). **Spatial upscaling (FSR 1) is
      done**; FSR 3.1 is a WGSL port per
      [ADR-0012](../architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md)
      and still wants motion vectors, a readable depth buffer, camera jitter and
      a scene with no UI in it - the first two of which
      [motion blur](../rendering/motion-blur.md) would deliver as a side effect,
      designed and costed at about a week and a half but **not built**.
      **FSR4 is not a plan**: it ships as signed DLLs
      and its driver upgrade path does not reach a native Linux build

**Exit criterion:** Pulse is feature complete, start to finish, on both asset
paths.

---

## M8 - Beyond Pulse

- [ ] Networking
- [ ] Wipeout Pure
- [ ] Wipeout HD / Fury
- [ ] Wipeout 2048
- [ ] Omega Collection, if feasible

Release order, because each title is the closest relative of the one before it.
Pure shares the most format DNA with Pulse and is the cheapest second title;
2048 is the long-term prize.

**Exit criterion:** a second title boots and plays on the same engine.

---

## Open questions blocking later milestones

| Question | Blocks | Tracked in |
| --- | --- | --- |
| Where is lap counting? `gate` has no runtime class at all. | M5 | [track data](../formats/track.md) |
| How is a ship assigned a grid slot? **The order is answered, the geometry is not.** `Race_SpawnGrid` (`0x088247e0`) assigns slots from a shipped permutation table and packs a short grid to the back; the local player is forced to slot 8 in the AI's own view. What turns a slot index into a world pose is still unfound. | M5 | [grid](../ghidra/functions/psp-pulse-usa/grid.md) |
| What does the per-vertex collision scalar mean? **Not answerable from assets**: all 602,086 are exactly `1.0`, so only the consumer can say. | M4 | [collision](../ghidra/functions/psp-pulse-usa/collision.md) |
| How does the sweep-and-prune packing hold coordinates beyond +/-1024, when real tracks reach 1,554? **Narrowed by survey**: all 16 environments measured; 7 reach past ±1024, but nothing on either disc *spans* more than 2,048 (widest 2026.6057, `10_Track`, 98.96% of the window). So the geometry is not at fault and the packed input cannot be raw world space - the open part is reading `Sap_Init` (`0x0882f8f4`) for the base it must subtract. | M4 | [collision](../ghidra/functions/psp-pulse-usa/collision.md) |
| What is the original PRNG? | M5 (AI, pickups) | [`oag-core::rng`](../../crates/core/src/rng.rs) |
| What are the coordinate conventions? **Handedness answered**: `cross(row0, row1) = row2` exactly on 200/200 ticks, so the basis is positively oriented under ordinary component arithmetic - and turning left rotates forward toward `+row0`, so **row 0 is left, not right**. Units and angles still open. | M4 | [engine](../ghidra/functions/psp-pulse-usa/engine.md) |
| ~~What calls `Ship_UpdateCraft`?~~ **Answered**: `0x0884ff70`, a virtual call through slot `0x70` of the vtable at `object+0x38`, inside a per-entity update loop beginning at `0x0884f70c`. | M4 | [engine](../ghidra/functions/psp-pulse-usa/engine.md) |
| ~~Do the angular accumulators hold torque or angular acceleration?~~ **Answered: torque.** `body+0x120`/`+0x130` integrate into `body+0x160` with no inertia division - `+0x160` is body-frame angular *momentum*, and the inertia is applied once in the `L -> omega` map (`omega = I_world^-1 * L`, rebuilt per sub-step). `body+0x40` is the body-space inverse inertia tensor, and its writer is now read too - `Body_SetBoxInertia` (`0x0884e1ac`), a solid box with the literal dimensions `(12, 8, 12)` and the constructor's mass `0.9`, giving `I = (15.6, 21.6, 15.6)` against the captures' fitted `~(15, 21.2, 15)`. `YAW_DRIVE_CALIBRATION` is retired for the recovered `oag_physics::forces::YAW_INVERSE_INERTIA`. | M4 | [rigid-body](../ghidra/functions/psp-pulse-usa/rigid-body.md) |
| Why does the US PSP disc carry a directory named for the *European* serial? | nothing yet | [PSP disc layout](../psp/pulse-disc-layout.md) |
