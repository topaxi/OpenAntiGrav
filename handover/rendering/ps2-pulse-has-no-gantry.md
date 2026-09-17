# PS2 Pulse draws no start gantry, while PSP Pulse and HD/Fury do

2026-09-17. Reported from play by the user: "the gantry is not
implemented/rendered/animated" on the PS2 Pulse source, "but is in psp pulse
and HD/Fury (potentially expected gap)". `oag_render::gantry` and
`crates/game/src/race/gantry.rs` carry no PS2 branch (no `ps2` mention in
either), so the absence is real; whether it is *expected* is the first
question.

## Open

- Does the PS2 disc author the gantry at all, and where? On the PSP it is a
  track node class the race animates; the PS2 `.vex` carries VIF packets in
  place of PSP display lists (`oag_vex`'s PS2 payloads) and may place the
  same node class or none. `oag-wad list`/`oag-view --track` on
  `data/images/pulse-ps2-eu.chd`'s `WADS2.WAD` against the PSP is the
  five-minute check.
- If it is authored: what the PS2 loader skips (a class id not mapped on the
  PS2 version of the class table, a mesh payload not decoded) - the loader
  report should say.
- If it is not authored on the PS2 disc: record it as an expected gap on
  `docs/formats/ps2-status.md` (or wherever the PS2 per-subsystem matrix
  lives) so nobody re-derives it.

## Next Steps

1. Answer "authored or not" from the disc, record it.
2. If authored, draw and animate it through the PSP path's own recovered
   timing (`gantry.rs`) - the PS2 game is Pulse, so the timing should not
   need a second read; verify against a PCSX2 capture only if it looks off.
