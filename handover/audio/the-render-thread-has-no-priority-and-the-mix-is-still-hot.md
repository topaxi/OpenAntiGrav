---
status: measure
titles: [all]
updated: 2026-11-03
summary: "Limiter and per-title master trim landed; open: underruns under load (no thread priority), Pure still limits 0.5-0.9 % of frames"
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
2. **Pulse and Pure still sum hot before the trim.** The trim (Pulse -6 dB,
   Pure -3 dB, `Title::master_trim_db`) took Pulse from limiting 23-29 % of
   frames to under 0.2 %; Pure still limits 0.5-0.9 % of frames below 0.94.
   Cutting the cause (eight `~ENGINE` voices at unity, `Scream_PanVolumePair`
   constant `a3`) would let it idle. The trim is set when a race arms
   (`oag_game::sound::hold_race_song`); menus keep the last race's value.
   Measure: the `mix:` line `--dump-audio` prints.
3. **The pan test still needs the limiter off.** The quake pair passes with the
   limiter on (Pulse trim). `sfx_ground_truth::the_whole_grid_is_audible...`
   subtracts two renders and still fails with the limiter on after the trim
   (the limiter acts on the grid's headphone-free mix at -6 dB): keep its
   `set_limiting(false)` or compare in a way that survives different gains.
4. **Limiter activity is not on the perf overlay.** `Mixer::peak`, `limited`,
   `deepest_limit` and `limit_depth_frames` exist; `Health` still reports only
   starved and clipped.

## Next Steps

Item 1 (a Deck run), then 2.
