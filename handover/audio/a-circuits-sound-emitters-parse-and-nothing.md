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

**2026-09-08: Next Steps 1 and 3 both closed this pass; Step 2 (`soundcone`'s
init) stays out of scope, unattempted - it needs the Ghidra bridge, which is
`quake`'s.**

**Step 1: Pure and the PS2 both author these nodes.** Full method and evidence
on
[track-sound-emitters.md](../../docs/ghidra/functions/psp-pulse-usa/track-sound-emitters.md#census)'s
now-answered "PS2 and Pure" bullet - not requoted here per this thread's own
rule. Short version: PS2 (`pulse-ps2-eu`) shares Pulse's exact class IDs and
node counts, checked two ways (`01_Track` 86/0/0, `07_Track` 43/9/0, both
exact matches to the PSP census). Pure (`pure-psp-usa`) renumbers its class
space (per `pure-status.md`) so a direct ID sweep reads zero, but an
ID-blind, size-blind text scan of three circuits' `track.vex` files (Vineta
K, Sebenco Climb, Chenghou Project) finds 48/56/106 nodes named
`woSound1`..`woSoundN`, each carrying a bank/cue pair that resolves against
Pure's own per-circuit `SBlk` banks (`VINETTA`, `SEBCLIM`, `CHENGOU`) by the
same `~`-prefixed cue-name convention Pulse uses. **The follow-on this
bullet used to name - finding Pure's circuit banks, hard because it ships no
`trackstartup.xml` - turns out to be the easy part**: the banks are named
the same way Pulse's are, one `oag-wad sounds` call away. What is still
unfound is Pure's own class ID for `woSound` (and whether it splits into a
cone/speaker equivalent the way Pulse's does - no `woSoundCone` or
`woSpeaker` node name turned up in the same scan), which needs the same
table-index technique `pure-status.md` used for the collision classes, and
that needs the Ghidra bridge.

**Step 3: no invented headroom - chosen, not measured.** Rendered two fresh
60 s (2-lap) captures on `pulse-psp-eu.chd`, 8-craft grid, `--race
--opponents --autopilot --ticks 3600`, all volumes at their default 100:
whole mix (music + SFX + ambience) clips **0.074%** (4,235/5,760,000
samples), RMS -13.0 dBFS; SFX-alone (music muted at the settings level, the
same isolation technique `the-race-mix-saturates-the-per-voice-sas.md` used)
clips **0.043%**, RMS -14.5 dBFS - close to that thread's own live-measured
original figure, **0.044%** (930/2,123,332, "isolated transients, not a mix
sitting on the rail"). Verified this was a real grid, not just the flag's
"drawing" effect its own `--help` text warns about: `--opponents` moved the
whole-mix RMS by a reproducible 0.33 dB (bit-identical across two separate
runs of each side), where `settings.toml`'s `mode` alone did not move it at
all (`single_race` without `--opponents` reproduced the solo run's RMS to
the last printed digit) - see the scratch report for the four-way check.

**This is a large, recent improvement over the 2.3-2.8% this thread and its
sibling were both citing, and the likely cause is attributable rather than
measured here**: `b46c6659` (landed 2026-09-07, after this thread's own last
saturation numbers and after the sibling thread's own last whole-mix clip
reading) ported `Scream_PanVolumePair`, the SFX pipeline stage the sibling
thread found carries a live-measured mean **-11.7 dB**. Nobody had
re-measured the whole mix's clip rate since that port landed until this
pass, and this pass did not itself capture a before/after `PanVolumePair`
pair (that would mean reverting `.rs` and pulling in the full gate for a
side measurement) - so read the attribution as "the timing and the size both
fit," not as a controlled before/after. Read as: **the saturation this
thread and its sibling both opened with is now substantially smaller, by
work neither thread did** - `PanVolumePair` was the sibling thread's own
subject, not this one's.

Given that, the decision is **not to add an invented limiter or gain
reduction**, chosen for two reasons: the port's clip rate is now the same
order of magnitude as the live-measured original's own (not a match, but not
the 50x gap the 2.3-2.8% figures were), and `oag_audio::mixer::Mixer::render`'s
own doc comment already argues the existing hard clamp is the port of a
recovered hardware behaviour (`Audio_Init_q` sets every group and the master
to unity, `sceAudioOutputPannedBlocking` gets `master << 5` unclamped) rather
than a limiter invented to make a loud mix comfortable - adding a soft
headroom knob now would move the port further from that recovered behaviour
for a gap that has mostly closed on its own. **What would change this
answer**: a future capture, after `the-race-mix-saturates-the-per-voice-sas.md`'s
own open residual (the per-voice SAS volume's unread absolute scale) is
chased down, showing the gap has reopened or a specific scenario (a full
eight-craft pileup, sustained weapon spam) still saturates chronically rather
than in the isolated bursts these two captures show (clip counts arrived in
uneven bursts of 2 to ~1,200 samples across the 60 s window, not a steady
rail-riding count - the same "isolated transient" character the sibling
thread's live original capture has, not continuous overload). No gain
changed in `crates/audio` or `crates/game/src/audio` this pass.

Captures: `/home/topaxi/oag-scratch/race_ambience_8craft.wav` (whole mix),
`/home/topaxi/oag-scratch/race_sfx_only_8craft.wav` (music muted).

**Gate**: no `.rs` file changed. `just check-docs` passes.

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
- ~~**Whether Pure and the PS2 author the same node.**~~ **Resolved
  2026-09-08: both do.** See the top-of-file entry and
  [track-sound-emitters.md](../../docs/ghidra/functions/psp-pulse-usa/track-sound-emitters.md#census).
  Kept here, struck rather than deleted, so the "unknown either way" framing
  is not read as still current. **What is genuinely still open**: Pure's own
  class ID for its `woSound` nodes, and whether it splits into a cone/speaker
  equivalent - both need the Ghidra bridge.
- **`emitter+0x3c`.** Unchanged: written from payload `+0x0c` by `VexSound_Init`
  and absent from `positional-audio.md`'s three-way-pinned emitter table, so
  nothing observed reads it back. It is `25.0` on every cone and equal to the
  radius on every `sound`.

**2026-09-08: opcode `0x14` is decoded - a bare no-op, corroborated on
`ps3-hdfury-eu` at the same slot (`Scream_OpNop14`/`Scream_DoGrainNop14`,
confidence 82, [sound.md](../../docs/ghidra/functions/psp-pulse-usa/sound.md#opcode-0x14-is-a-no-op-corroborated-on-hd-2026-09-08)).**
It does not bind a waveform, so the **9** nodes carrying it (`moather~birds`,
`dekonst~CRANE` and both `talonsj~SETREG` cues) are correctly silent, not
blocked on a missing handler -
[track-sound-emitters.md](../../docs/ghidra/functions/psp-pulse-usa/track-sound-emitters.md#a-second-larger-set-resolves-and-still-cannot-be-played-38-nodes-on-eight-circuits)
corrects its prior confidence-88 hypothesis. **The other 29 (every `~SetReg*`
cue, opcode `0x1e`) are unaffected by this and stay unread** - "set register"
is still only a name-based guess, not a decoded handler. Nothing to wire
follows from `0x14` itself - there was no waveform to bind - so the former
Next Step #1 is dropped rather than closed with an implementation. The
discriminating check on this (do the table's neighbouring slots hold distinct
handlers) found `0x15` is the same no-op shape and `0x16` scans for it -
neither is one of the 38 silent nodes' opcodes, so it doesn't change this
thread's count, but it is recorded on `sound.md` since it came from the same
table read.

## Next Steps

1. ~~**Sweep `pure-psp-usa` and `pulse-ps2-eu` for the three classes.**~~ Done
   2026-09-08 - see the top-of-file entry. The follow-on it named (finding
   Pure's circuit banks) turned out to be trivial; what remains is Pure's own
   class ID for `woSound`, which needs the Ghidra bridge.
2. **Find `soundcone`'s init**, which is the only thing standing between 134
   authored directional emitters and being played. Everything else they need is
   in place: the payload decodes, the emitter record has the half-angle at
   `+0x40` and the enable byte at `+0x4c`, and `SoundEmitter_ComputeVolumeAndAngle`
   already multiplies the cone falloff in. Needs the Ghidra bridge; out of
   scope for a pass without it.
3. ~~**Decide whether the mix wants headroom.**~~ Decided 2026-09-08, chosen
   rather than measured: no invented limiter, for now - see the top-of-file
   entry for the fresh clip-rate numbers and what would revisit this.
4. **Find Pure's own `woSound` class ID** (new, 2026-09-08, split out of the
   old Next Step 1 now that the node-level question is answered). Same
   table-index technique `pure-status.md` used for the collision classes, or
   Pure's own registration function if it is immediate-coded the way Pulse's
   three are - either way needs the Ghidra bridge. Whether Pure distinguishes
   a `soundcone`/`speaker` equivalent at all is the sharper form of the
   question: no `woSoundCone` or `woSpeaker` node name turned up in the three
   circuits checked, so that may be a Pulse-only split.
