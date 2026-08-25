# The PS2 boost plume draws, and the PS2 file is not the PSP file wearing the same name

2026-08-23, **resolved**; it answers [The PS2's engine flare plays and reads as nothing on screen](the-ps2s-engine-flare-plays-and-reads-as.md), which recorded it as unchecked. **Pruned 2026-08-24** - the full account is in `crates/game/tests/boost_plume_ground_truth.rs` (the measured table), [ps2-texture.md](../docs/formats/ps2-texture.md#how-a-model-finds-its-texture-set-directory-position-not-a-name) and [batch-draw-state.md](../docs/ghidra/functions/ps2-pulse-eu/batch-draw-state.md); what stays here is what a reader still has to know. `Exhaust::plume_visible` was never the problem - it is simulation-side and goes true on both. `shipboost.vex` is a *differently authored model* on the two discs and was being built through the PSP's path, which gave two bugs: it drew **untextured** (the PS2 file embeds no pixels; `livery::ps2_skin` now takes the hull's directory-position branch, 24 of 24 exact) and **misplaced** (its meshes hang off `Anim Transform` anchors and drew collapsed at the craft's origin until `race::scene::frame` wrote the node table). PSP frames byte-identical either side. **The clock is the reading, confidence 65, and the thing to re-open**: the anchors' 67-key z-scale flicker rides `Exhaust::plume_timer` rather than the race clock, fitted approximately (`PLUME_SECONDS` is 1.5 and PSP-recovered) with no PS2 executable read behind it. **Two traps.** Do not retune `exhaust::BLEND` or the vertex-alpha handling to suit the new textures - they were calibrated on `pulse_boost2_ADD`, and a wrong-looking PS2 plume is a PCSX2 capture nobody has taken, not a coefficient. And the sequence is the lesson: the plume looked solid, `Gfx_BuildBatchStateList` (`0x001e9088`, 88) appeared to agree, and **a PCSX2 capture refuted it** - the original's are soft and violet with the hull visible through them. `Drawable::draw_additive` is the override that resulted; the dispatch that would settle it is unfollowed.

## Open

- The 67-key z-scale flicker riding `Exhaust::plume_timer` instead of the race clock is only an approximate fit (confidence 65), with no PS2 executable read behind it.
- The dispatch that would settle the plume-timer question is unfollowed.

## Next Steps

- Follow the dispatch to confirm whether the anchors' flicker really rides `Exhaust::plume_timer` rather than the race clock.
- Do not retune `exhaust::BLEND` or vertex-alpha to suit the new textures without a fresh PCSX2 capture.
