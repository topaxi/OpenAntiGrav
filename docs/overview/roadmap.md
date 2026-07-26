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
- [~] [`.vex` scene format](../formats/vex.md): node tree read; track nodes and
      geometry decoding outstanding
- [x] `.vex` mesh batches decoded into portable vertex buffers, with textures
- [x] [Track data](../formats/track.md): spline graph, racing line, PVS sections
      recovered from the binary; **not yet validated against a real track file**
- [ ] Parse a real track and render it
- [ ] Audio: the music and effect banks
- [ ] `oag-view`, a standalone asset viewer

**Exit criterion:** a Pulse track and a ship model render in `oag-view` from an
unmodified disc image, on both the PSP and PS2 asset paths.

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
- [ ] Confirm the image base against PPSSPP's module load address
- [ ] Load `SCES_547.48` (PS2) into Ghidra
- [x] [Main loop and frame pacing](../psp/frame-pacing.md); state machine outline
- [x] Simulation timestep resolved: the original uses **variable** delta, not a
      fixed step. See [ADR-0007](../architecture/adr/0007-fixed-timestep-vs-original.md)
- [x] WAD subsystem: hash, lookup, mount, read, decompressors
- [x] [Input](../ghidra/functions/psp-pulse/input.md), [collision](../ghidra/functions/psp-pulse/collision.md),
      [video](../ghidra/functions/psp-pulse/frontend-video.md), [physics model](../physics/README.md)
- [x] Physics is float, not fixed-point; integrator is 3 sub-steps of dt/3
- [ ] Memory management and the heap layout
- [ ] Resource loading: how a WAD entry becomes a live object
- [ ] Game state machine
- [ ] The original PRNG
- [ ] Memory maps for both platforms

**Exit criterion:** engine lifecycle and memory map documented; at least 50
functions documented to the standard in
[`ghidra/function-template.md`](../ghidra/function-template.md).

---

## M3 - Verification harness

The instrument everything after this is measured with.

- [ ] Scripted input playback into PPSSPP
- [ ] Save-state driven fixed-start scenarios
- [ ] Trace capture: position, orientation, velocity, timers, per tick
- [ ] `oag-trace`: compare a Rust run against a captured trace, with tolerances
- [ ] The same for PCSX2

**Exit criterion:** one command diffs any subsystem against the original and
reports where and by how much it diverges.

---

## M4 - Playable core

- [ ] wgpu renderer MVP: track geometry, ships, a free camera
- [ ] Input
- [ ] Chase camera
- [ ] Ship physics: thrust, steering, airbrakes, pitch, the air cushion
- [ ] Collision against track and walls

**Exit criterion:** a single-ship time trial that passes trace comparison for
the full lap, and that feels right to someone who knows the original.

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
| What does the per-vertex collision scalar mean? | M4 | [collision](../ghidra/functions/psp-pulse/collision.md) |
| What is the original PRNG? | M5 (AI, pickups) | [`oag-core::rng`](../../crates/core/src/rng.rs) |
| What are the coordinate conventions? Handedness, units, angles. | M4 | [physics](../physics/README.md) |
| Why does the US PSP disc carry a directory named for the *European* serial? | nothing yet | [PSP disc layout](../psp/pulse-disc-layout.md) |
