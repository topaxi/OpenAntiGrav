# HD/Fury's engine sound: level and authored mix are measured; the stereo pair, per-cue groups and the distance writer are open

2026-10-05 (second pass). An HD race plays each craft's own `xfship_<team>.xfx` layers
(`crates/game/src/audio/sfx/xfade.rs`). The evidence is
[`docs/formats/hd-xfx.md`](../../docs/formats/hd-xfx.md) ("Level, measured live") and the
2026-10-05 sections of
[`docs/ghidra/functions/ps3-hdfury-eu/xfade.md`](../../docs/ghidra/functions/ps3-hdfury-eu/xfade.md);
this file is only what is left.

## Done

- The pitch unit (a SCREAM bend), `X` (the first hover probe's length), the wiring.
- **The authored mix** (2026-10-05, lane `hd-mix-level`): the global gap was `GlobalAudioConfig.xml`'s
  per-state group volumes (music linear, effects squared), sliders default 80 %. See
  [`hd-xfx.md`](../../docs/formats/hd-xfx.md) "The authored mix".
- **The level**: `0x400` is unity, a layer's volume word is **squared**, every engine voice reads
  `K = 0.295` against `level` (two boots, 126 hardware voices). The port now plays
  `ENGINE_BUS_RATIO * (curve * distance_factor)^2` instead of Pulse's `x^0.59`; a goteki
  craft's ramp dropped from RMS 0.243 (clipped) to 0.072 (peak 0.33), a full grid of
  engines alone peaks at 0.77. The mixer pool grows to 128 for an HD race
  (`Mixer::grow_pool`), Pulse and Pure render byte-identical WAVs.
- The capture rig: `scripts/rpcs3-hd-engine-audio-capture.py` (RPCS3 audio dump on a private null
  sink, `--park` for a GDB session that survives, `--probe` for fixed scans).

## Open

- **Every HD cue's group.** The mix is read and the buses exist (`crates/game/src/audio/hd_mix.rs`,
  `Bus::Group(n)`); only the engine (group 7) and the circuit's emitters (group 8) are on theirs.
  Every other cue plays on the effects bus at group `1.0`, which is chosen, not measured: read the
  group each cue's voice carries (a SCREAM voice's bus field, or the bank's cue record) and move
  collisions (6, `Cue::Collision`), pads and turbo (4), HUD cues (2), weapons (3, hd-weapons lane),
  explosions (5) and the announcer (1) onto theirs. The `USER1..12` comment is the only list.
- **The original's mix beyond the group rows**: the `MasterCompressor` (ratio 0.2, -6 dB), the
  ducking templates and their `DuckerEvents`, `Auto Volume` (default on), the `PreRace`,
  critical-energy and player-dead rows, and the transition speed (`SMOOTHING` is one reading).
- **The front end's music has no side energy in the original** (stereo RMS `0.072` against mono-mean
  `0.070`) and a lot in ours (`0.13` against `0.070`): the same loudness, a different stereo image.
- **Ours is half of the original on the engine and the ambience** at the matched law, which the
  second voice of each pair predicts (below); measured ratios 1.8 to 2.3 against 1.93 predicted.
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
