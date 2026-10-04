# HD/Fury's engine sound: the table is read and the drive law is nearly recovered; two things stop it playing

2026-10-04 (rewritten; first written 2026-09-23). An HD race still plays music and
SFX with no engine note: `shiphd.bnk` has no `~ENGINE` cue, so the held voice
`crates/game/src/audio/sfx/engine.rs` opens on Pulse never opens on HD. What the
`hd-engine-xfade` lane established is in
[`docs/formats/hd-xfx.md`](../../docs/formats/hd-xfx.md) and
[`docs/ghidra/functions/ps3-hdfury-eu/xfade.md`](../../docs/ghidra/functions/ps3-hdfury-eu/xfade.md);
this file is only what is left.

## Done

- `oag_formats::xfx` reads all **13** `xfship_<team>.xfx` (the old note said
  12) and accounts for every byte; feisar's 2,100 bytes is one layer plus its
  pointer. Every layer name (`~jet03 03`, `~ABResLoL`, `~afterburner`, ...) is a
  cue in `shiphd.bnk`. Channel 0's 26 triggers name no sound.
- The crossfade system's load, smooth and layer-evaluate path is named
  (`XFadeSystem_*`, ten names in `names.tsv`).
- `Ship_UpdateEngineCrossfade` (`0x000d5968`) writes the four input channels;
  channel 0 and 3 were checked live on RPCS3 (12 samples, within two counts).

## Open

- **`X`**, the second term of channel 0 (`5.0 * X`, `X = body_entry+0x260`).
  Live it is about 2.2 on the grid and 2.9-4.5 driving, and tracks
  `entry+0x354/0x364/0x368/0x36c/0x370` plus a constant 1.12, which looks like
  a ride height but is untested. Without it channel 0, which carries the main jet
  note, is `0.5 * speed_field` plus a constant-ish 11 to 22 at best.
- **The pitch unit.** A layer's pitch curve minus `0x200`, times `0x7fff`,
  shifted right 9, is clamped to `+-0x8000` and handed to
  `FUN_0031c948` -> `FUN_006796b8`. Not decoded, so no playback ratio is known and
  a played note would be a guess.
- Channels 1 and 2 (`speed_field * 0.01 * ctrl[+8]` / `[+0xc]`) stayed `0` on
  all 12 samples; what drives `ctrl[+8]`/`[+0xc]` is unmeasured (airbrake?
  `L1` did not move them).

## Next Steps

1. Name `X`: either set the RPCS3 write watchpoint
   (`just build-rpcs3-watchpoints`) on `body_entry+0x260` and read the writing
   `PC`, or read the entry's vtable slots (`0x00682b68`..., vtable `0x008745b8`).
   `scripts/rpcs3-hd-engine-xfade-probe.py` already finds the player entry and
   dumps the body, which is the start of either.
2. Decode the pitch unit below `FUN_006796b8`, or measure two samples of
   (channel value, played pitch) from the emulator.
3. Only then wire it in `crates/game/src/audio/`: a sim-side `speed_field`
   and `X`, `oag_formats::xfx` for the layers, the smoother's band edges and
   rates for channel 0, and a headless WAV over a race segment as proof. A
   Pulse-shaped `~ENGINE` stand-in on HD stays forbidden.
