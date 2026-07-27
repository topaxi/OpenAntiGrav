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
- [ ] Audio: the music and effect banks
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
- [ ] `oag-trace`: compare a Rust run against a captured trace, with tolerances
- [ ] The same for PCSX2

**Exit criterion:** one command diffs any subsystem against the original and
reports where and by how much it diverges.

The instrument exists and records a moving ship - 200 ticks of a Time Trial at
about 22 units/s, position, orientation, velocity, dt and the whole control block
per tick. The comparison does not exist, so the exit criterion is untouched.

What the capture path has already produced is five runtime confirmations that
were previously static readings, including one that settles a claim by counting:
`craft+0x2ec` matches the **previous** frame's `|dot(velocity, forward)|` on
199 of 199 samples and the current frame's on 137, which is the one-frame
staleness [engine](../ghidra/functions/psp-pulse/engine.md) derived from the
frame ordering. See also [frame pacing](../psp/frame-pacing.md).

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
      implemented, and [an unresolved contradiction](../ghidra/functions/psp-pulse/collision.md)
      in its coordinate packing should be settled before it is.

**Exit criterion:** a single-ship time trial that passes trace comparison for
the full lap, and that feels right to someone who knows the original.

**Nothing here is close to that exit criterion**, and the ticks above say only
that code exists. There is no trace comparison, because that is M3 and M3 has not
started.

Concretely: **the ship cannot yet fly a lap.** It spawns on the racing line,
hovers, and drives forward, and then loses probe contact after about 100 ticks and
is thrown off the track by 166. That is recorded as a failing-on-purpose test,
`the_ship_does_not_stay_on_the_track_yet`, whose message says to delete it when it
starts passing. Three measurements produced while implementing this say the model
as transcribed is not yet right: the roll oscillator is
[11 % outside the specified integrator's stability limit](../physics/README.md),
the hover target exceeds the probe cast length so no resting height exists, and
two cross-product terms had to be flipped to produce the behaviour their own
evidence pages describe.

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
- [ ] Modern features: ultrawide, unlocked frame rate, HDR, VRR, dynamic resolution

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
| How does the sweep-and-prune packing hold coordinates beyond +/-1024, when real tracks reach 1,554? | M4 | [collision](../ghidra/functions/psp-pulse/collision.md) |
| What is the original PRNG? | M5 (AI, pickups) | [`oag-core::rng`](../../crates/core/src/rng.rs) |
| What are the coordinate conventions? **Handedness answered**: `cross(row0, row1) = row2` exactly on 200/200 ticks, so the basis is positively oriented under ordinary component arithmetic - and turning left rotates forward toward `+row0`, so **row 0 is left, not right**. Units and angles still open. | M4 | [engine](../ghidra/functions/psp-pulse/engine.md) |
| ~~What calls `Ship_UpdateCraft`?~~ **Answered**: `0x0884ff70`, a virtual call through slot `0x70` of the vtable at `object+0x38`, inside a per-entity update loop beginning at `0x0884f70c`. | M4 | [engine](../ghidra/functions/psp-pulse/engine.md) |
| Do the angular accumulators hold torque or angular acceleration? | M4 | [engine](../ghidra/functions/psp-pulse/engine.md) |
| Why does the US PSP disc carry a directory named for the *European* serial? | nothing yet | [PSP disc layout](../psp/pulse-disc-layout.md) |
