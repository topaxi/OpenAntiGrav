# Pulse's Zone announcer is a sequence; a wider set of cues is still sampled instead of played

2026-09-29. The maintainer heard "clear" alone at zone 5 (other milestones: "zone" alone, or a bare number). **Cause found and fixed**: each `zone_N` cue is a timeline - child `ZONE`, the number, child `CLEAR` on authored delays, every word a left/right pair of one waveform at pan angles 30/330 - and `load_named_cue` flattened it to six leaves that `Announcer::pick` drew from at random. `oag_formats::sblk::timeline` now walks it and `oag_game::audio::sfx::compose` lays it down as one stereo `Bus::Speech` voice. Evidence, delay word, and the live-measured 258.4 Hz master tick: [psp-audio.md](../../docs/formats/psp-audio.md#a-cue-can-be-a-sequence-and-zone_n-is-one) and [sound.md](../../docs/ghidra/functions/psp-pulse-usa/sound.md#the-master-tick-and-the-delay-word-2026-09-29). Pulse has no class announcer, so the "voice stolen on the same tick" idea does not apply to it.

## Open

- **`Banks::pick` still samples where the disc plays.** A survey of every playing cue on the five PSP/PS2 discs and HD: 2,168 with fewer than two key-ons of their own, 385 with a `0x19` alternate group, **910 with several key-ons of one waveform and no `0x19`** (left/right pairs, harmless to sample), **683 with several distinct waveforms and no `0x19`** (layers or sequences, played as one random layer). Pulse's `~SHIELD` (0.501 s and 1.087 s) is in the last group. Which of the 683 are audible in play, and which are layers rather than sequences, is not established. The fix here was deliberately not applied globally.
- **Not confirmed live in the original**: the order and timing of the three words. The delay-word reading is from three decompiled functions and the tick rate is measured, but no breakpoint on `Scream_KeyOnVoice` while crossing zone 6 has been taken. The phase of the tick against the cue start (up to 3.9 ms) is taken as zero.
- **HD keeps the flat pick.** Its tick is not measured, and its `c_CLEAR` (cue 21) is a goto and a marker with no waveform, so HD's bank carries no audible "clear" at all - carried by a cross-bank cue, or absent. Unread.
- **The composite's volume**: per grain `scale * 2 * cue_volume^2 * sound_volume^2`, with a child taking its record's volume and its parent's scale. A child is 0.5 dB louder than the root key-on; read from `Scream_OpPlayChild`'s decompile, confidence 75, uncaptured.
- **Rear-half pan angles** (`a' >= 180` in `Scream_PanVolumePair`) are refused by `oag_audio::spatial::pan_of_angle`, so a cue with one falls back to the flat pick.

## Next Steps

1. Take the live check: break on `Scream_KeyOnVoice` (`0x0899456c`) in PPSSPP, cross zone 6, record the descriptor address and the tick counter at `0x08ac35e0` per hit. Expect ZONE at 0, the number at 105, CLEAR at 255 ticks, each doubled 10 ticks later.
2. Run `Bank::cue_timeline` over the 683 cues and split them into "complete timeline" versus "unread opcodes"; wire `compose` into `Banks` for the complete ones a title actually fires.
3. Measure HD's tick (RPCS3 debugger, `scripts/rpcs3_debugger.py`) and read `c_CLEAR`'s goto, then let HD's zone lines use the same path.
