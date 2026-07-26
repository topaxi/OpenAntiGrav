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

- [ ] WAD container: names or hash mapping, compression, alignment
- [ ] Textures: PSP swizzled formats, PS2 equivalents
- [ ] Models: ship and scenery geometry, materials
- [ ] Tracks: geometry, spline, collision, sections
- [ ] Audio: the music and effect banks
- [ ] `oag-view`, a standalone asset viewer

**Exit criterion:** a Pulse track and a ship model render in `oag-view` from an
unmodified disc image, on both the PSP and PS2 asset paths.

**Where to start:** the [WAD directory format](../formats/wad.md) is already
mostly readable and is the gate for everything else.

---

## M2 - Binary understanding

The PSP `BOOT.BIN` is an unencrypted ELF, so this can start immediately.

- [x] Load `BOOT.BIN` (PSP) into Ghidra. Auto-analysis finds 10,646 functions.
- [ ] **Install the Allegrex processor module.** Stock Ghidra mis-decodes all
      26,032 VFPU instructions; see [Allegrex and the VFPU](../psp/allegrex-vfpu.md)
- [ ] Load `SCES_547.48` (PS2) into Ghidra
- [ ] Main loop and frame structure
- [ ] **Confirm the simulation tick rate.** Defaulted to 60 Hz; see
      [`TickRate`](../architecture/determinism.md#the-tick-rate-is-a-decision-not-a-measurement)
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
| Is the tick rate really 60 Hz? Defaulted, not measured. | M4 tuning | [determinism](../architecture/determinism.md) |
| How are WAD entries named? Is the first field a hash? | M1 | [WAD format](../formats/wad.md) |
| What is the original PRNG? | M5 (AI, pickups) | [`oag-core::rng`](../../crates/core/src/rng.rs) |
| Is the physics fixed-point or float? | M4 | [ADR-0002](../architecture/adr/0002-determinism-model.md) |
| Why does the US PSP disc carry a directory named for the *European* serial? | nothing yet | [PSP disc layout](../psp/pulse-disc-layout.md) |
