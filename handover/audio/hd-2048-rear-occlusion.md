---
status: re-first
titles: [hd, 2048]
size: S (60)
difficulty: easy (60)
updated: 2026-11-04
summary: "HD rear low-pass wired under `original` (one-pole, per-tick smoothing chosen); 2048 flags unread; axis, rate, `auto` default open"
---

# HD and 2048's rear low-pass under `original`

2026-11-04, lane `hd-rear-occlusion`. **HD is wired**; the law and every value are on
[audio-3d.md](../../docs/ghidra/functions/ps4-omega-eu/audio-3d.md), "The rear low-pass" and "Wired on HD", which also records the
measured effect on HD's real voices (a trim, not a thinned mix). Pulse, Pure and Omega stay bit-identical or off; 2048 is not wired.

## What landed

- ~~Data, not a const~~: `Title::rear_occlusion` (HD `Stereo`, `None` elsewhere), the value from `GlobalAudioConfig.xml` through
  `oag_tables::audio_config` and `hd_mix::Maps`; both on-disc copies (`DATA00`, `DATA01`) carry 14,000; no HD extras pack is under `data/dlc`, so none beyond the disc was checked.
- ~~The law and the mixer branch~~: `Mixer::set_rear_occlusion`, `spatial::rear_occlusion_cutoff`; the Pulse path is the existing
  `original_ignores_where_a_voice_is_along_the_forward_axis` and `without_the_law_original_stays_bit_identical_front_and_back`.
- ~~Doc comments~~ on `rear_cue`, `Play::front`, `Listener::forward`, `SpatialMode::Original`.
- ~~Moa Therma~~: `TrackEmitters::low_pass_off`, read off the circuit's `AudioConfig.xml` (`03_track` alone).

## Open

- **2048.** Same law at 10,000 Hz, but one voice kind's filter setter is an empty stub and the other reaches `FUN_8134bde4`'s flags,
  unread. Read the flags, then set `oag_2048::TITLE.rear_occlusion` to `Some(Sourced::measured(...))` and check its
  `GlobalAudioConfig.xml` has a `Stereo` child: Omega's copy is self-closing with `max_frequency` on `<RearOcclusion>` itself, and
  `audio_config::rear_occlusion_max_frequency` would then scan to the end of the file for `<Stereo `, so 2048's own spelling needs a look. Checked, applies, not wired.
- **The filter's order and shape.** HD's result goes to `FUN_003146e8`, sub-voice `+0x48..+0x54`; the consumer is unread (about an
  hour). The one-pole is **chosen, not measured** and cuts at most 2.8 dB (Nyquist), so how hard the original bites is unknown until this
  is read; it also decides `auto`'s default below. Do this one first.
- **The axis.** That the angle's `+Z` is the camera's forward is inferred, not traced (recipe: the writer of `*(0x008b4aa4) + 0x50..0x7c`
  on HD). If it were backward the law would flip.
- **The rate.** The `0.05/0.95` smoothing runs once per tick here (**chosen**); the original runs it per emitter update on a frame
  counter modulo the divisor behind TOC slot `0x008b4af0`, 1 in the file, run-time value unread.
- **The DistanceFilter default** for a circuit with no block (`g_sound_system + 0xf8`), and Moa Therma's high-pass reading, are
  unread; the rear low-pass is simply off there.
- **Mixed library.** `Audio::set_hd_mix` keeps the first source's `GlobalAudioConfig.xml` for the session, as the group volumes
  always did; a session that read 2048's or Omega's file before racing HD would apply that file's value. Not reachable while
  only HD sets a layout and HD's own disc is the source of its race, but it is a trap.
- **`auto` and `rear_cue` on HD.** `auto`'s speaker default is still `rear_cue` (2.5 kHz, far stronger than the original's 14 kHz);
  whether it moves to `original` on HD is the maintainer's call. Audio clarity comes first
  (`docs/ghidra/functions/psp-pulse-usa/audio-levels.md`).
- **Omega's `original`** is Pulse's pan as a stand-in until Wwise pan, the pass-by switches and the NPC engine's Azimuth curve play;
  that belongs to the Wwise SFX thread.

## Next Steps

1. 2048: read `FUN_8134bde4`'s flags (the evidence page's recipe) and wire its title field, or write down why not.
2. Read the HD consumer (an hour) and replace the one-pole.
