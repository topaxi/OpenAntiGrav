# The loading screen draws now, and its wave is runtime-verified except for one case

The rippling band is procedural and the `loading` plugin holds no video; the decode is pinned against the real blob (which is **swizzled**, bit 0 of `+0x07`, and **fully opaque**, so both plausible wrong readings are excluded by construction). The "no caller" half is stale: `LoadingStage` (`crates/game/src/main/loading_stage.rs`) constructs `oag_render::loading::Pipeline` and calls `capture::draw_wave` on both the boot path and the pre-race path (`feat(loading): recover HD's loading screen, and put one before every race`, 2026-08-24), confirmed against the real disc via `loading_screen_ground_truth` (5/5). **The motion model is now runtime-verified too**: two breakpoint captures against `pulse-psp-usa.chd` (`scripts/psp-loading-wave-capture.py`) predicted each column's state from the previous one and matched what `Loading_DrawWave` actually computed to float-rounding precision across hundreds of consecutive columns, plus one observed call boundary matching the reset-to-zero, phase advance and envelope-table indexing all at once. Confidence 85 -> 96 on `docs/ghidra/functions/psp-pulse-usa/loading-screen.md`'s motion section.

## Open

- Neither capture happened to land on one of `g_loading_wave_envelope`'s two 99-peaks (`phase` was 1 then 2, both low entries), so the `pulse / 99.0 + 0.1` amplitude term is exercised structurally but not at its largest live value. See the doc page's own "Not determined" section.

## Next Steps

- Run `scripts/psp-loading-wave-capture.py` again, either with a much higher `--hits` budget or timed to start nearer a beat (the envelope wraps every 24 frames at 30 Hz, so roughly every 0.8 s an entry of 99 is live) - a fresh `PPSSPPHeadless` instance is required per run, never a reused/already-probed connection (see the script's own docstring on why a leftover breakpoint from an earlier connection can silently defeat a new one). This closes the residual; nothing about the arithmetic already checked would change.
