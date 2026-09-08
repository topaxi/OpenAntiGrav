# A circuit's sound emitters parse, and nothing plays them

2026-09-04. The three `.vex` audio classes are decoded whole and the evidence
is on
[track-sound-emitters.md](../../docs/ghidra/functions/psp-pulse-usa/track-sound-emitters.md),
which is the durable record - **do not requote its layout table here**. This
thread's title is now stale: `8b6eaf56` (2026-09-06) landed the half that did
not - **the circuits are audible**, `oag_game::audio::sfx::TrackEmitters`
opens a held looping voice for every `sound` `0x3e1` node inside its radius
and stops it when the listener leaves, and a headless lap renders to a WAV.
What is left is what wiring the 1,298 authored emitters turned up, below.

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

- **What a latched voice should do.** `SoundEmitter_ServiceRequests` leaves a
  latched request untouched - "not even reaped" - so the original's voice keeps
  playing at the last in-range gain, which is near zero. `Ambience::tick` stops
  it instead and says so in its own doc comment. What reclaims the original's
  voice is unread, and that is the gap.
- **The mix clips.** 7,932 samples over 30 s of `01_Track` before the ambience
  landed, 22,473 after - about 0.8% of the render. Every emitter plays at the
  `1.0` `VexSound_Init` passes and `Mixer::starved` stays at zero, so this is
  the sum's headroom rather than a voice budget. Nothing here invents a gain to
  hide it.
- **Which cone angle is which.** Unchanged: `soundcone`'s own init has not been
  found, so which of the two authored angles reaches the emitter's `+0x40`
  half-angle is unread, and `+0x42` reading `0` on all 134 cones is
  unexplained. **A cone stays unwired until that is read**, and 134 nodes are
  waiting on it - 32 of them on `07_Track` reversed alone.
- **Whether Pure and the PS2 author the same node.** Only `pulse-psp-usa` has
  been swept for the classes. Two things are now known either way: Pure and the
  PS2 both carry `Data\Sound\generaltrack.bnk` (`oag_title::SoundBanks::track_general`),
  and **Pure ships no `trackstartup.xml` per circuit**, so if Pure does author
  emitters its circuit bank is named some other way and would have to be found
  before the non-`gentrak` half of them could resolve.
- **`emitter+0x3c`.** Unchanged: written from payload `+0x0c` by `VexSound_Init`
  and absent from `positional-audio.md`'s three-way-pinned emitter table, so
  nothing observed reads it back. It is `25.0` on every cone and equal to the
  radius on every `sound`.

**2026-09-08: opcode `0x14` is decoded - a bare no-op, corroborated on
`ps3-hdfury-eu` at the same slot (`Scream_OpNop`/`Scream_DoGrainNop`,
confidence 82, [sound.md](../../docs/ghidra/functions/psp-pulse-usa/sound.md#opcode-0x14-is-a-no-op-corroborated-on-hd-2026-09-08)).**
It does not bind a waveform, so `moather~birds`, `dekonst~CRANE` and both
`talonsj~SETREG` cues are correctly silent, not blocked on a missing handler -
[track-sound-emitters.md](../../docs/ghidra/functions/psp-pulse-usa/track-sound-emitters.md#a-second-larger-set-resolves-and-still-cannot-be-played-38-nodes-on-eight-circuits)
corrects its prior confidence-88 hypothesis. Nothing to wire follows from this
- there was no waveform to bind - so the former Next Step #1 is dropped rather
than closed with an implementation.

## Next Steps

1. **Sweep `pure-psp-usa` and `pulse-ps2-eu` for the three classes** - one
   `--nodes --class` run each. If Pure authors them, finding where its circuit
   banks live is the follow-on, because it ships no `trackstartup.xml`.
2. **Find `soundcone`'s init**, which is the only thing standing between 134
   authored directional emitters and being played. Everything else they need is
   in place: the payload decodes, the emitter record has the half-angle at
   `+0x40` and the enable byte at `+0x4c`, and `SoundEmitter_ComputeVolumeAndAngle`
   already multiplies the cone falloff in.
3. **Decide whether the mix wants headroom.** Not an audio-emitter question -
   it was already clipping - but the ambience is what made it visible.
