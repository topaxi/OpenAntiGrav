---
status: ready-to-wire
titles: [hd, 2048]
size: S (65)
difficulty: easy (60)
updated: 2026-11-04
summary: "HD and 2048 low-pass a voice behind the listener (14 kHz, 10 kHz, two binaries); `original` plays none of it; filter shape unread"
---

# HD and 2048's rear low-pass under `original`

2026-11-04, lane `omega-3d-audio-read`. Evidence, with the law and every value:
[audio-3d.md](../../docs/ghidra/functions/ps4-omega-eu/audio-3d.md), "The rear low-pass". Confidence 80 on HD, 70 on 2048, decompile
on two binaries that agree, no live run. **2048 is the weaker half:** one voice kind's filter setter is an empty stub and the other
reaches an unread request (page, "Where the result goes"), so read `FUN_8134bde4`'s flags before wiring 2048, or ship HD alone.

Our `original` is Pulse's pan on every title (`oag_audio::SpatialMode::Original`), so on HD and 2048 it is missing a
behaviour the original has. The Pulse and Pure pan stays bit-identical; Omega stays out (`RearOcclusion` is dead there, its
front/back is Wwise data).

## What a wiring needs

1. **Data, not a const.** Read `RearOcclusion` from the title's own `GlobalAudioConfig.xml`, the way `oag_tables` reads
   its other XML: HD `<Stereo max_frequency="14000.0"/>` (ours is a stereo mix, so that child; `<Surround>` is 24,000,
   which is off), 2048 `max_frequency="10000.0"`. `None` on Pulse, Pure and Omega through a `Title` field with provenance.
   Check the 2048 DLC and HD extras' copies carry the same value before assuming one.
2. **The law.** Ramp over azimuth 90..110 and 250..270 degrees (`|angle from forward| > 90`, forward inferred, see Open), `rear = r * max + (1 - r) *
   23999`, smoothed `0.05 / 0.95` per update. `Placed.front` is `cos(angle)` already, so the listener's forward from
   `spatial-audio-wire` is all the geometry it needs. Per-update smoothing is per original update, not per sample.
3. **In the mixer.** A branch under `SpatialMode::Original` on a per-voice cutoff, with Pulse's path untouched (a test that
   Pulse stays bit-identical, a test that drops the law and fails). Fix the `rear_cue` doc comment in `crates/audio/src/mixer.rs`,
   whose "the original cannot tell front from back" is true of Pulse alone, and the same line in `spatial.rs`.
4. **DistanceFilter is not needed yet, except Moa Therma:** near and far are both 23,999 on the other circuits, so the term is
   open. Moa Therma alone has `near_cutoff="0.0" far_cutoff="999.0" low_pass="0"` (HD, the 2048 DLC, Omega), which reads as a
   high-pass: skip the rear low-pass there until it is traced.

## Open

- **The filter's order and shape.** HD's result goes to `FUN_003146e8`, sub-voice `+0x48..+0x54`; the consumer is unread
  (recipe on the evidence page, about an hour). A one-pole until then is **chosen, not measured**.
- **The axis.** That the angle's `+Z` is the camera's forward is inferred from the XML comment and the band. Recipe: the writer of
  `*(0x008b4aa4) + 0x50..0x7c` on HD. If it were backward the law would flip.
- **The rate.** 0.05/0.95 runs per emitter update, and updates run on a frame counter modulo the divisor behind TOC slot
  `0x008b4af0`. Read its runtime value (1 in the file) to get a time constant on our tick.
- **The DistanceFilter default.** `GlobalAudioConfig.xml` has none, so a track with no block uses `g_sound_system + 0xf8`, unread.
- **`auto` and `rear_cue` on these titles.** With `original` carrying a real law, whether `auto`'s speaker default stays
  `rear_cue` (2.5 kHz, far stronger than the original's) is the maintainer's call; audio clarity comes first
  (`docs/ghidra/functions/psp-pulse-usa/audio-levels.md`).
- **Omega's `original`** is Pulse's pan as a stand-in (its lineage is HD) until Wwise pan, the pass-by switches and the NPC engine's
  Azimuth curve play. That belongs to the Wwise SFX thread, not this one.

## Next Steps

1. Read the consumer (an hour), or take the one-pole as chosen.
2. One `oag-wire` lane, S: steps 1-3 above, a WAV pair front and behind per title for the lead, no listening from spawned
   processes.
