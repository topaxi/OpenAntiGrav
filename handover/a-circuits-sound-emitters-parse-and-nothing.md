# A circuit's sound emitters parse, and nothing plays them

2026-09-04. The three `.vex` audio classes are decoded whole and the evidence
is on
[track-sound-emitters.md](../docs/ghidra/functions/psp-pulse-usa/track-sound-emitters.md),
which is the durable record - **do not requote its layout table here**. This
thread is the half that did not land: **1,298 authored emitters are readable and
nothing in `oag-game` plays a single one**, so every circuit is still as silent
as it was before the parse, and the fidelity win is all still in front of
whoever picks this up.

What exists to build on: `oag_formats::sound_emitters::emitters(data, nodes)`
returns each node's bank label, cue name, radius curve, optional cone and world
matrix, and five `#[ignore]`d ground-truth tests in
`crates/formats/tests/sound_emitter_ground_truth.rs` hold that decode against
`pulse-psp-usa`. `oag_audio::spatial` already has the volume/pan law these want
to feed, and `oag_audio::sblk` already reads the banks the cues live in.

**Three things about wiring this are decided by the disc rather than by
taste, and are the reason it is worth doing properly rather than quickly:**

1. **Every emitter is live from construction.** `VexSound_Init` calls
   `SoundEmitter_Init` and `Sound_Play` on the spot, so a circuit does not start
   its ambience when you come near it - it starts all of it at load and the
   `SoundEmitter_ServiceRequests` out-of-range latch is what keeps a distant one
   silent. A design that starts a cue on approach is a different game.
2. **`01_Track` alone authors 86 of them, `14_Track` 97 + 9.** Against a PSP
   voice pool that `docs/formats/psp-audio.md` sizes, that is more emitters than
   voices, so the budget question is real and is not answered anywhere yet.
   Every cue but a handful loops.
3. **The radius is a curve, not a number.** One key on every Pulse node, so a
   constant in practice - but `SoundEmitter::sample_radius` is the honest call
   and the `f32` is the shortcut, and the difference will matter the moment
   another title in the lineage animates one.

## Open

- **The voice budget.** Nothing has counted how many of a circuit's emitters are
  simultaneously in range on a real lap, and nothing has read what the original
  does when more want a voice than exist. `oag_audio`'s pool is hardware-free
  and can be sized; the original's cannot.
- **What a dangling reference should do.** Five references on the disc do not
  resolve - `fortcle~blueflashlight` (14 nodes), `basilic~groupcraft`,
  `fortcle~RED_NEON_TUN`, `techder~neon` and `outpostf~AIR_CON_FAN`. Per
  `CLAUDE.md` the answer is to play nothing and say so in the loader report, but
  whether the original silently drops them or falls back to a bank-index lookup
  is unread: `Scream_FindSoundInBank`'s name comparison has not been decompiled,
  and `techder~neon` in particular would resolve under any fallback that
  searches other banks.
- **Which cone angle is which.** `soundcone`'s own init has not been found, so
  which of the two authored angles reaches the emitter's `+0x40` half-angle is
  unread, and `+0x42` reading `0` on all 134 cones is unexplained. **A cone
  stays unwired until that is read** - it is exactly the "an effect with no
  recovered trigger stays unwired" case.
- **Whether Pure and the PS2 author the same node.** Only `pulse-psp-usa` has
  been swept. `docs/formats/pure-status.md` lists `speaker` in Pure's class
  table, so Pure at least names the classes.
- **`emitter+0x3c`.** Written from payload `+0x0c` by `VexSound_Init` and absent
  from `positional-audio.md`'s three-way-pinned emitter table, so nothing
  observed reads it back. It is `25.0` on every cone and equal to the radius on
  every `sound`.

## Next Steps

1. **Count what is audible.** Run a lap on `01_Track` with the parsed emitters
   and the recovered radius law, and print how many are inside their radius per
   tick. That is one scenario and it answers the budget question before any
   mixing code is written; it needs no new subsystem, only
   `sound_emitters::emitters` and `oag_audio::spatial`.
2. **Play the omnidirectional ones.** `sound` `0x3e1` only, looping, started at
   load, gated by the same in-range latch the original uses. Leave `soundcone`
   out - its trigger half is genuinely unread, and a cone played as a sphere is
   a stand-in for something the disc already specifies.
3. **Report the five dangling references** in the loader output rather than
   dropping them silently, so the count is visible if a later decode changes it.
4. Sweep `pure-psp-usa` and `pulse-ps2-eu` for the same three classes, which is
   one `--nodes --class` run each and would tell whether this generalises before
   anything is built on the assumption that it does.
