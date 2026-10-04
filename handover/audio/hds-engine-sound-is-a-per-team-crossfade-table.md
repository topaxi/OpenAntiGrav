# HD/Fury's engine sound: the crossfade plays; the hover term and the loudness are unchecked

2026-10-05 (rewritten; first written 2026-09-23). An HD race now plays each craft's
own `xfship_<team>.xfx` layers (`crates/game/src/audio/sfx/xfade.rs`, wired in
`audio/sfx.rs`'s craft loop, tables loaded by `Banks::load_xfade`). The evidence is
[`docs/formats/hd-xfx.md`](../../docs/formats/hd-xfx.md) and the 2026-10-05 section
of [`docs/ghidra/functions/ps3-hdfury-eu/xfade.md`](../../docs/ghidra/functions/ps3-hdfury-eu/xfade.md);
this file is only what is left.

## Done

- The pitch unit: a layer's pitch word is a SCREAM bend, linear in semitones over the
  cue descriptor's bend range (`Scream_SetVoiceBend`, `Scream_UpdateVoiceBend`), the
  law `audio/sfx/layers.rs` already plays on Pulse.
- `X` (`body[+0x260]`): the first queued hover probe's length, pushed by
  `Physics_QueueRayProbe_q`/`Physics_QueueSegmentProbe_q`, equal to the mean probe
  clearance `body[+0x354]` plus about 1.12 live.
- Wired for every grid slot whose team has a table, Pulse and Pure untouched (their
  `~ENGINE` cue gates the load; a Pulse and a Pure race render byte-identical WAVs with
  and without the HD path).

## Open

- **`X` is held at 2.164** (the grid value). This simulation keeps `HoverProbe::height`
  and the probe reach only as per-step locals, so reading them needs a physics change.
  Channel 0 is off by up to about 12 of 511 while the craft rides high.
- **Channels 1 and 2 are held at zero**: they were zero in all twelve live samples and
  nothing that drives `ctrl[+8]`/`[+0xc]` was found.
- **Gain unity.** `0x400` is taken as 1.0 against the cue's own volume; the volume
  word's scale in `0x0062b9e8` was not followed. With it the eight-craft grid roughly
  doubles a goteki race's RMS and touches full scale (a Pulse-style 0.85 opponent scale
  is unmeasured on HD). An RPCS3 audio capture of the same team would settle both the
  unity and the loudness.
- **Layers with `+0x16 == 0`** (ag_systems, assegai, egx `~n..`) keep their voice
  running at zero volume in the original; the port releases it below an audible floor.
- Whether an opponent's channel 3 stays unwritten in the original is the read of
  `ship+0x628c == 0`, not a live observation.

## Next Steps

1. Record one team's engine on RPCS3 (muted emulator, so a loopback or the emulator's own
   audio dump) over a standing start and a climb, and compare the layer balance and the
   note against `data/scratch/hd-engine-wire/xfade-goteki-ramp.wav`.
2. If `X` matters by ear, keep the mean probe clearance on `ShipState` (a physics change)
   and feed `0.5 * speed_field + 5 * (clearance + 1.12)`.
3. Follow `0x0062b9e8`'s per-engine dispatch to the volume scale to settle the unity.
