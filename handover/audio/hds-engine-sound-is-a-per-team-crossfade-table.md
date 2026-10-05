# HD/Fury's engine sound: the level is measured; the global HD gap, the stereo pair and the distance writer are open

2026-10-05 (second pass). An HD race plays each craft's own `xfship_<team>.xfx` layers
(`crates/game/src/audio/sfx/xfade.rs`). The evidence is
[`docs/formats/hd-xfx.md`](../../docs/formats/hd-xfx.md) ("Level, measured live") and the
2026-10-05 sections of
[`docs/ghidra/functions/ps3-hdfury-eu/xfade.md`](../../docs/ghidra/functions/ps3-hdfury-eu/xfade.md);
this file is only what is left.

## Done

- The pitch unit (a SCREAM bend), `X` (the first hover probe's length), the wiring.
- **The level**: `0x400` is unity, a layer's volume word is **squared**, every engine voice reads
  `K = 0.295` against `level` (two boots, 126 hardware voices). The port now plays
  `ENGINE_BUS_RATIO * (curve * distance_factor)^2` instead of Pulse's `x^0.59`; a goteki
  craft's ramp dropped from RMS 0.243 (clipped) to 0.072 (peak 0.33), a full grid of
  engines alone peaks at 0.77. The mixer pool grows to 128 for an HD race
  (`Mixer::grow_pool`), Pulse and Pure render byte-identical WAVs.
- The capture rig: `scripts/rpcs3-hd-engine-audio-capture.py` (RPCS3 audio dump on a private null
  sink, `--park` for a GDB session that survives, `--probe` for fixed scans).

## Open

- **A global HD level gap, not the engine's.** The front-end music is 2.7x hotter here than in the
  original, ordinary SCREAM voices about 3.1x, the circuit ambience alone louder than the original's
  whole grid mix. One factor of about 3 on every HD cue would explain all three; the profile the
  original ran on had unknown music and SFX slider values (an existing save), so it may simply be
  a default. Decide HD's bus trims (`MUSIC_MASTER_TRIM` is Pulse's `0.44`) from a capture on a
  fresh save, or photograph Options > Audio first. Touches music, collisions, the countdown.
- **The per-craft distance factor is a fit** (`slot[+4] / 1024`, confidence 55): its writer was not
  found, `Ship_UpdateEngineCrossfade` does not write it. Find the store (a short at `+4` of every layer
  slot; slots are `0x80` apart from word 2 of each `0x10`-byte instance at `*(*0x008b5064 + 0x18)`) with a data watchpoint on RPCS3's
  patched build or by xrefs from the SCREAM listener update.
- **Each layer is two voices, not one**: the same waveform from two start offsets 164 bytes apart at
  azimuth `+-30` (`+-70..90` for the noise layers, drifting with throttle). The port plays one. Which of the
  cue's five or six waveforms the original picks is unread (they are alternates in the cue script).
- **`X` is held at 2.164**; reading the mean probe clearance needs a physics change.
- **Channels 1 and 2 are held at zero**; nothing that drives `ctrl[+8]`/`[+0xc]` was found.
- **Channel 3 is written in every mode.** The original skips it in modes whose id is bit 6, 13, 14 or 21 of
  `0x206040` (Zone is 6); the port has no map from those ids to its own modes.
- **Layers with `+0x16 == 0`** keep their voice at zero volume in the original; the port releases
  it below an audible floor.
- The hardware float-gain writer (`+0x28`/`+0x2c` of a slot) behind `0x0062b738` is not located;
  `K` is measured, not derived.

## Next Steps

1. Photograph HD's Options > Audio on a fresh save and capture the front-end music again; the result
   sets or clears the global factor above.
2. Data watchpoint on a layer slot's `+4` to name the distance writer and replace the fit.
3. Play the pair: a second voice of the same waveform at the other azimuth, 164 bytes into the data.
