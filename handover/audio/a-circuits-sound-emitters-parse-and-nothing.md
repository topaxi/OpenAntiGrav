# A circuit's sound emitters parse, and nothing plays them

2026-09-04. The three `.vex` audio classes are decoded whole and the evidence
is on
[track-sound-emitters.md](../../docs/ghidra/functions/psp-pulse-usa/track-sound-emitters.md),
which is the durable record - **do not requote its layout table here**. This
thread's title is now stale: `8b6eaf56` (2026-09-06) landed the half that did
not - **the circuits are audible**, `oag_sound::sfx::TrackEmitters`
opens a held looping voice for every `sound` `0x3e1` node inside its radius
and stops it when the listener leaves, and a headless lap renders to a WAV.
What is left is what wiring the 1,298 authored emitters turned up, below.

What exists to build on: `oag_vex::sound_emitters::emitters(data, nodes)`
returns each node's bank label, cue name, radius curve, optional cone and world
matrix, and five `#[ignore]`d ground-truth tests in
`crates/vex/tests/sound_emitter_ground_truth.rs` hold that decode against
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

**2026-09-09: `soundcone`'s init is found and the 134 cones are wired and
audible; Pure's `woSound` class ID is found too, without needing the Ghidra
bridge.** Full evidence on
[track-sound-emitters.md](../../docs/ghidra/functions/psp-pulse-usa/track-sound-emitters.md#vexsoundcone_init-2026-09-09-which-angle-reaches-0x40-is-read)
and
[pure-status.md](../../docs/formats/pure-status.md#a-circuits-own-ambience-wosound-is-class-0x393-found-without-ghidra) -
**not requoted here**, per this thread's own rule. Short version:
`VexSoundCone_Init` (`0x08925ff4`, EU `0x08925ad0`, confidence 90) is a
four-line wrapper around `VexSound_Init` that writes the node's `+0x00` angle
to the emitter's `+0x40` half-angle - settling which of the two authored
angles is the one `oag_audio::spatial` now attenuates by, no guess between
them. Tracing that also found *why* `+0x42` reads `0` on every cone
(confidence 85): `VexSound_Update`, the only function that ever resamples the
radius curve, has exactly two static call sites in the executable and both are
gated on the payload's own `+0x3c`, which is `0` on all 1,298 authored nodes -
so the curve never actually runs, for a `sound` or a `soundcone`, and `+0x10`
is the one radius any node ever plays. `oag_sound::sfx::track` now
places both `TrackEmitters::omni` and `TrackEmitters::directional` (the new
name for what used to be a bare `cones: usize` count) through
`oag_audio::spatial::Emitter::cone`, sourcing the radius from `+0x10` rather
than the (provably dead) curve. Pure's own `woSound` class turned out not to
need Ghidra at all: `oag_vex::vex::Node` already carries the class ID and
scene name together, and a `woSoundN` name belongs to a `Transform` parent
whose one child, `woSoundNShape`, is class `0x393` on all 129 nodes sampled
across three circuits - and none of them shows a cone-split signature,
corroborating the prior pass's name-based negative on the payload's own
bytes.

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

*Corroborated 2026-09-15 on the real device path rather than the null
backend: `--tap-audio` of windowed `--race --autopilot` runs clipped 0.127 %
on Pulse (30 s, 16_Track) and 0.127 % on HD/Fury (40 s) - same order as the
0.074 % above, and both taken with a race start's silence in the window.*

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
changed in `crates/audio` or `crates/sound/src` this pass.

Captures: `/home/topaxi/oag-scratch/race_ambience_8craft.wav` (whole mix),
`/home/topaxi/oag-scratch/race_sfx_only_8craft.wav` (music muted).

**Gate, 2026-09-08 pass**: no `.rs` file changed. `just check-docs` passes.

**Gate, 2026-09-09 pass**: `.rs` files changed in `crates/vex/src/sound_emitters.rs`,
`crates/audio/src/spatial.rs` (+`lib.rs`), `crates/sound/src/sfx.rs` and
`crates/sound/src/sfx/track.rs`, plus their tests. Ran `just fmt`,
`just lint` (one `clippy::neg_cmp_op_on_partial_ord` fix), `just test`
(3368 passed, 0 failed), `just check-docs`, `just check-deps`,
`just check-determinism`, `just check-size`, `just check-names`,
`just check-strings` - all green - and
`OAG_REQUIRE_GAME_DATA=1 just test-data`, watched to completion:
**4071 tests run: 4071 passed (26 slow), 0 skipped**, exit code 0, including
`race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`
(106.259 s) and every `sound_emitter_ground_truth`/`track_audio_ground_truth`
test. `the_densest_circuit_on_the_disc_fits_in_the_pool_too` (`14_Track`, the
circuit with 9 of the 12 circuits' cones) now reports "97 omnidirectional
emitter(s) and 9 cone(s)" and "92 of 106 emitter(s) resolved to a cue" -
106, not 97, is the give-away that cones are in the resolve pass now. A
60 s, 8-craft headless render of `14_Track` (`/home/topaxi/oag-scratch/soundcone-14track.wav`)
plays without a crash or a silent buffer (99.98% non-zero samples).

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
- ~~**Which cone angle is which.**~~ **Resolved 2026-09-09**, see the top-of-file
  entry and `track-sound-emitters.md`'s own `VexSoundCone_Init` section.
- **A cone inside its radius but outside its angle still holds a silent
  voice.** New from this pass, not previously possible to observe: the angle
  term is a multiplier that can reach zero without the emitter's own
  radius-only "out of range" latch ever tripping, so `Ambience::tick` opens
  and holds a voice for it anyway - real cost against the 32-voice pool, only
  measured for `01_Track` (no cones) and `14_Track` (peak still 8, so not yet
  a problem there) so far. See `crates/sound/src/sfx/track.rs`'s own
  header for the reasoning; not fixed here because nothing read says the
  original does anything different for this case, and no invented gate should
  fill that gap.
- ~~**Whether Pure and the PS2 author the same node.**~~ **Resolved
  2026-09-08: both do.** See the top-of-file entry and
  [track-sound-emitters.md](../../docs/ghidra/functions/psp-pulse-usa/track-sound-emitters.md#census).
  Kept here, struck rather than deleted, so the "unknown either way" framing
  is not read as still current. ~~**What is genuinely still open**: Pure's own
  class ID for its `woSound` nodes, and whether it splits into a cone/speaker
  equivalent - both need the Ghidra bridge.~~ **Resolved 2026-09-09, and it
  turned out not to need the Ghidra bridge** - see the top-of-file entry and
  `pure-status.md`'s own section. Pure's own field layout past the class ID
  (the bank string sits at a different payload offset than Pulse's) is
  unread and is genuinely new open ground, not a repeat of this bullet.
- **`emitter+0x3c`.** Unchanged: written from payload `+0x0c` by `VexSound_Init`
  and absent from `positional-audio.md`'s three-way-pinned emitter table, so
  nothing observed reads it back. It is `25.0` on every cone and equal to the
  radius on every `sound`.
- **`emitter+0x44`.** New, 2026-09-09, same standing as `+0x3c` immediately
  above: `VexSoundCone_Init` writes the node's `+0x04` angle (`40` degrees on
  every cone) here, and nothing found so far reads it back.

**2026-09-08: opcode `0x14` is decoded - a bare no-op, corroborated on
`ps3-hdfury-eu` at the same slot (`Scream_OpNop14`/`Scream_DoGrainNop14`,
confidence 82, [sound.md](../../docs/ghidra/functions/psp-pulse-usa/sound.md#opcode-0x14-is-a-no-op-corroborated-on-hd-2026-09-08)).**
It does not bind a waveform, so the **9** nodes carrying it (`moather~birds`,
`dekonst~CRANE` and both `talonsj~SETREG` cues) are correctly silent, not
blocked on a missing handler -
[track-sound-emitters.md](../../docs/ghidra/functions/psp-pulse-usa/track-sound-emitters.md#a-second-larger-set-resolves-and-still-cannot-be-played-38-nodes-on-eight-circuits)
corrects its prior confidence-88 hypothesis. ~~The other 29 (every `~SetReg*`
cue, opcode `0x1e`) stay unread~~ **Superseded: `0x1e` is `Scream_OpSetRegister` (confidence 88, `sound.md`), a register write that starts no sound, and the cues are control-only.** Nothing to wire
follows from `0x14` itself - there was no waveform to bind - so the former
Next Step #1 is dropped rather than closed with an implementation. The
discriminating check on this (do the table's neighbouring slots hold distinct
handlers) found `0x15` is the same no-op shape and `0x16` scans for it -
neither is one of the 38 silent nodes' opcodes, so it doesn't change this
thread's count, but it is recorded on `sound.md` since it came from the same
table read.

## Next Steps

1. ~~**Sweep `pure-psp-usa` and `pulse-ps2-eu` for the three classes.**~~ Done
   2026-09-08 - see the top-of-file entry.
2. ~~**Find `soundcone`'s init**~~ Done 2026-09-09 - `VexSoundCone_Init`,
   134 cones wired and audible. See the top-of-file entry.
3. ~~**Decide whether the mix wants headroom.**~~ Decided 2026-09-08, chosen
   rather than measured: no invented limiter, for now - see the top-of-file
   entry for the fresh clip-rate numbers and what would revisit this.
4. ~~**Find Pure's own `woSound` class ID**~~ Done 2026-09-09, and without the
   Ghidra bridge the earlier framing of this step expected to need - `0x393`.
   See the top-of-file entry.
5. **Whether a cone's off-angle-but-in-radius voice is worth budgeting for**
   (new, 2026-09-09). `Ambience::tick` opens one anyway - see the "Open"
   bullet above - and neither circuit measured here (`01_Track`, no cones;
   `14_Track`, 9 of them, still peaking at 8 in-range) has shown it mattering
   yet. The other ten circuits, and especially `07_Track` reversed (32 cones,
   the densest cone count on the disc) are unmeasured.
6. **Derive Pure's own payload field layout for `woSound`** (new, 2026-09-09,
   split out of the old Next Step 4 now that the class ID is answered). The
   bank string sits at `+0x08` here, not Pulse's `+0x14`, so nothing about the
   rest of the 80-byte shape - the radius, the curve, the cone-enable
   equivalent if one exists - was re-derived; this pass read only enough to
   confirm the class and the no-cone-split negative. Needs the Ghidra bridge
   if Pure's own `VexSound_Init`-equivalent is to be read directly, or more
   `just view --payload` sampling if a byte-offset invariant (equal fields,
   fixed constants) can pin it the way this page's own Pulse table did.

## From the HANDOVER.md index (moved 2026-09-25)

**both `sound` and `soundcone` are audible now, and both without an emulator.** `oag_sound::sfx::TrackEmitters` opens a held voice for every in-range `sound`/`soundcone` node, off `oag_audio::spatial::Emitter::cone`'s law. 2026-09-09 closed the two open reversals: `VexSoundCone_Init` (`0x08925ff4`, confidence 90) settles which of a cone's two authored angles is its half-angle, and tracing it found that `VexSound_Update`'s radius curve has exactly two static call sites, both gated on a byte that is `0` on all 1,298 authored nodes - so the curve never actually runs and `+0x10` is the one radius any node ever plays, which is also why a cone's own curve key reads `0` and is safe to ignore. Pure's own `woSound` class (`0x393`) was found the same pass, without the Ghidra bridge the thread expected it to need: a scene node's own class ID and name are enough, no exporter table required. Earlier finds still stand: PS2 shares Pulse's class IDs and counts exactly; a circuit's own sound bank is named by its `trackstartup.xml` and lives beside it in the circuit directory; opcode `0x14` is a decoded no-op, corroborated on `ps3-hdfury-eu`; the mix's clip rate is down sharply since `Scream_PanVolumePair` landed, and no limiter was invented to chase the rest. Open: a cone in radius but outside its angle still holds a silent voice rather than being budgeted away, unmeasured past two circuits; Pure's own payload field layout past the class ID.

2026-10-06: a cue whose timeline walk is complete, starts no grain and passed at least one `0x14`/`0x1e` is now logged `control only` at debug, not `play nothing` at WARN (`sfx::banks::ControlOnlyCue`), so `talonsj~SETREG_01/_02` no longer warn on Pulse. Cues with an opcode the walk does not read still warn. See `docs/formats/pulse-absent-effects.md`.
