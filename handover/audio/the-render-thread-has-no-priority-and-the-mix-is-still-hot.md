---
status: measure
titles: [all]
updated: 2026-11-03
summary: "Limiter landed; open: underruns under load (no thread priority), Pulse peaks 2x full scale, HRTF behind 4-9 dB hotter"
categories: [audio]
---

# The master limiter landed; what is still hot

The limiter (`oag_audio::limiter`, `docs/ghidra/functions/psp-pulse-usa/audio-levels.md`,
"Our master limiter") took Pulse's clipped samples from 0.2 to 0.8 % of a
race to zero. Open, ranked:

## Open

1. **Underruns under heavy host load, not measured.** The ring is 60 ms
   (`MIN_BUFFER`) and the render thread (`output/render.rs`) asks for no
   scheduling priority anywhere (`rg -i priority crates/audio crates/thread`
   finds nothing). Measure: `--tap-audio` with `stress-ng` inside the gate
   lock (real stream; plays on the host, so on the Deck only), read the
   "dropped" health line. A priority request would need a new dependency
   (`audio_thread_priority`).
2. **Pulse mixes at up to 2.3x full scale before the limiter.** Cutting the
   cause (eight `~ENGINE` voices at unity, `Scream_PanVolumePair` constant `a3`)
   would let the limiter idle. Measure: the `mix:` line `--dump-audio` prints.
3. **A cue added to a hot Pulse mix is partly taken back by the limiter.**
   The quake `.COLLISIONS` pair measures 2 % quieter with the limiter on
   (table in the audio-levels page); a 6 dB master trim restores +2.5 to
   +4 %. Decide: a 6 dB trim (Pulse only, per-title data; HD peaks 0.94 and the
   limiter is idle there), or a shorter hold (ripple vs punch). Measure:
   `cargo nextest run -p oag-game --run-ignored all -E 'test(a_quake_hit_is_louder)'`
   after removing its `set_limiting(false)`. Headphones HRTF was checked and
   adds no gain (struck).
4. **Limiter activity is not on the perf overlay.** `Mixer::peak`, `limited`,
   `deepest_limit` and `limit_depth_frames` exist; `Health` still reports only
   starved and clipped.

## Next Steps

Item 3 first (lead decision), then 1 (a Deck run).
