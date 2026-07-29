# Roadmap

Eight milestones. Each has an exit criterion that is a demonstrable fact, not a
judgement call, so it is always clear whether a milestone is done.

The ordering follows the natural dependency chain, with one deliberate
departure: the verification harness (M3) comes *before* the physics work (M4).
Building ship handling before it can be measured against the original means
building it twice, once by feel and once properly.

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
      masters). Per-sound boundaries within a bank and track names are still
      open.
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
      [`docs/ghidra/functions/ps2-pulse/`](../ghidra/functions/ps2-pulse/README.md)
- [x] [Main loop and frame pacing](../psp/frame-pacing.md); state machine outline
- [x] Simulation timestep resolved: the original uses **variable** delta, not a
      fixed step. See [ADR-0007](../architecture/adr/0007-fixed-timestep-vs-original.md)
- [x] WAD subsystem: hash, lookup, mount, read, decompressors
- [x] [Input](../ghidra/functions/psp-pulse/input.md), [collision](../ghidra/functions/psp-pulse/collision.md),
      [video](../ghidra/functions/psp-pulse/frontend-video.md), [physics model](../physics/README.md)
- [x] Physics is float, not fixed-point; integrator is 3 sub-steps of dt/3
- [x] [Engine, brakes, steering and pitch](../ghidra/functions/psp-pulse/engine.md):
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

The function count is met: [names.tsv](../ghidra/functions/psp-pulse/names.tsv)
carries **120** documented symbols, each refused by
`scripts/apply-ghidra-names.py` unless its address and name are still on an
evidence page. The memory map is what remains.

---

## M3 - Verification harness

The instrument everything after this is measured with.

- [x] Scripted input playback into PPSSPP: `input.buttons.send`/`press` through
      the websocket debugger, verified by watching `craft+0x2b8` read 100 while
      thrust is held
- [ ] Save-state driven fixed-start scenarios
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
[engine](../ghidra/functions/psp-pulse/engine.md) derived from the frame
ordering. See also [frame pacing](../psp/frame-pacing.md).

---

## M4 - Playable core

- [x] wgpu renderer MVP: `oag-render` draws track geometry and ships, and
      `oag-game` puts a ship on a real track with a chase camera, headless or in a
      window - reached from the front end's `Launch Game` in the same window, or
      directly with `--race`. See [oag-game](../tools/oag-game.md).
- [x] Input: `oag-input` maps devices onto the abstract button layer and produces
      the `InputSnapshot` the simulation consumes
- [x] Chase camera: `oag_render::camera::chase`, its seven parameters taken from
      `<ExternalCameraFar>` rather than invented
- [x] Ship physics **implemented, none of it verified**: thrust, steering, brakes,
      airbrakes, pitch, the air cushion, grip, drag and the passive torques, from
      [physics](../physics/README.md) and
      [engine](../ghidra/functions/psp-pulse/engine.md). Every magnitude is
      transcribed static analysis; only two cross-product signs are settled, and
      those by arithmetic rather than by measurement.
- [x] Collision against track and walls: the [triangle soup](../formats/collision.md)
      decoded and validated on both discs, queried by a segment-triangle
      narrowphase behind an AABB reject. The sweep-and-prune broadphase is not
      implemented, and [the origin its coordinate packing subtracts](../ghidra/functions/psp-pulse/collision.md)
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
| Single-sided rejection instead of flipping the normal (`Collision_BoxAgainstMesh`, `0x08815cd4`) | 617.5 units travelled to 618.6 - the size 99.6 % predicts |

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

**So what blocks M4 now is that no capture can decide the first of its two
numbers.** The criterion above asks how many ticks we track the original before
coming apart, and that question has no answer past tick **171** on anything in
`data/traces/`: the lap recapture is wall-free on 32.7 % of its ticks and first
touches a wall there, after which the comparison measures the original's own
scrapes rather than our force law. **A clean lap capture is therefore load-bearing
for the milestone**, not a convenience - and it has to be re-derived from a fresh
`just autopilot` run, because the committed lap script is a closed-loop recording
that no longer flies clean open-loop. The second number, the reseeded per-window
error, is answerable today and is the table above.

The other live gap is smaller and sharper: **our hull begins responding to a wall
about six ticks before the original does.** Measured against the original's own
recorded poses rather than a replay - so it is contact geometry, not accumulated
trajectory error - and reproduced on both captures, on the same lower front
corner probe. See
[`oag-trace.md`](../tools/oag-trace.md#our-hull-meets-that-wall-six-ticks-early-and-it-is-the-contact-geometry);
the next step is reading `Collider_BoxSamplePoints` (`0x08818a00`).

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

- [ ] Race rules, grid, lap timing, positions
- [ ] HUD
- [ ] Weapons and pickups
- [ ] Shield and energy
- [ ] AI
- [ ] Audio

**Exit criterion:** an eight-ship race that is indistinguishable from the
original to a player, and whose per-tick trace stays within tolerance.

---

## M6 - Shell and polish

- [ ] Menus and the front end
- [ ] Save data
- [ ] Replay
- [ ] PS2 asset path at parity with PSP
- [ ] Modern features: ultrawide, unlocked frame rate, HDR, VRR, dynamic
      resolution, upscaling (FSR 3.1, with FSR4 via the driver), Steam Input,
      optional ahead-of-time FMV upscaling - licensing status and the
      architectural prerequisites to decide early are recorded in
      [modern features](modern-features.md)

**Exit criterion:** Pulse is feature complete, start to finish, on both asset
paths.

---

## M7 - Beyond Pulse

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
| How is a ship assigned a grid slot? | M5 | [track data](../formats/track.md) |
| What does the per-vertex collision scalar mean? **Not answerable from assets**: all 602,086 are exactly `1.0`, so only the consumer can say. | M4 | [collision](../ghidra/functions/psp-pulse/collision.md) |
| How does the sweep-and-prune packing hold coordinates beyond +/-1024, when real tracks reach 1,554? **Narrowed by survey**: all 16 environments measured; 7 reach past ±1024, but nothing on either disc *spans* more than 2,048 (widest 2026.6057, `10_Track`, 98.96% of the window). So the geometry is not at fault and the packed input cannot be raw world space - the open part is reading `Sap_Init` (`0x0882f8f4`) for the base it must subtract. | M4 | [collision](../ghidra/functions/psp-pulse/collision.md) |
| What is the original PRNG? | M5 (AI, pickups) | [`oag-core::rng`](../../crates/core/src/rng.rs) |
| What are the coordinate conventions? **Handedness answered**: `cross(row0, row1) = row2` exactly on 200/200 ticks, so the basis is positively oriented under ordinary component arithmetic - and turning left rotates forward toward `+row0`, so **row 0 is left, not right**. Units and angles still open. | M4 | [engine](../ghidra/functions/psp-pulse/engine.md) |
| ~~What calls `Ship_UpdateCraft`?~~ **Answered**: `0x0884ff70`, a virtual call through slot `0x70` of the vtable at `object+0x38`, inside a per-entity update loop beginning at `0x0884f70c`. | M4 | [engine](../ghidra/functions/psp-pulse/engine.md) |
| ~~Do the angular accumulators hold torque or angular acceleration?~~ **Answered: torque.** `body+0x120`/`+0x130` integrate into `body+0x160` with no inertia division - `+0x160` is body-frame angular *momentum*, and the inertia is applied once in the `L -> omega` map (`omega = I_world^-1 * L`, rebuilt per sub-step). `body+0x40` is the body-space inverse inertia tensor, and its writer is now read too - `Body_SetBoxInertia` (`0x0884e1ac`), a solid box with the literal dimensions `(12, 8, 12)` and the constructor's mass `0.9`, giving `I = (15.6, 21.6, 15.6)` against the captures' fitted `~(15, 21.2, 15)`. `YAW_DRIVE_CALIBRATION` is retired for the recovered `oag_physics::forces::YAW_INVERSE_INERTIA`. | M4 | [rigid-body](../ghidra/functions/psp-pulse/rigid-body.md) |
| Why does the US PSP disc carry a directory named for the *European* serial? | nothing yet | [PSP disc layout](../psp/pulse-disc-layout.md) |
