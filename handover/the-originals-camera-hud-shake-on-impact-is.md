# The camera shake on impact is located, on PS2 - the PSP side is now a narrower, different question

A hard wall hit in the original visibly shakes the camera/HUD; `oag_render` has no shake at all. On the PSP, the once-suspected function calls no camera API (`contact-response.md` carries the retraction) and `FUN_088418e0`'s reactions - sound, hull damage, shield flash - do not shake anything either.

**2026-09-03: found on the PS2 binary, in full - `Camera_ArmShake` (`0x0013eec0`) and its apply side inside `FUN_0013e280`.** `crates/game/src/race/camera.rs`'s FOV work led into `docs/ghidra/functions/ps2-pulse-eu/camera.md`, which led into reading `FUN_0013e280` in full, which turned out to open with exactly the shake-decay block this thread predicted. Full account, addresses and confidence: [`docs/ghidra/functions/ps2-pulse-eu/collision-shake.md`](../docs/ghidra/functions/ps2-pulse-eu/collision-shake.md). In short: the collision-response path (the PS2 counterpart of `Ship_DispatchCollisionFx`) computes the *exact same* `min(|impulse| * 0.0125, 1.0)` severity this thread already named as the likely input, spawns the hull spark (as the PSP does), and - where the PSP's path was verified to stop - also arms a camera shake scaled by that same severity, gated to the local player and to which side of the craft was hit.

**This narrows rather than closes the thread.** The PSP's collision-response function was checked specifically for a camera-API call and found to have none - that finding stands; what wasn't checked is whether the PSP's own per-frame camera-update function has an internal shake-apply block of this shape that nothing calls into from `Ship_DispatchCollisionFx` at all (a different wiring, not a different absence). So there are now two distinct open questions where there used to be one: is the PS2's shake a genuine platform addition, or does the PSP have the same mechanism reached a different way; and, separately, whether/how `oag_render` should implement it - a design decision, not a reading, and PS2-specific evidence alone may not be enough to justify it project-wide.

## Open

- Whether the PSP disc has the same shake mechanism, reached differently, or genuinely lacks it - not yet checked from the PSP's own per-frame camera-update function's own body, only from `Ship_DispatchCollisionFx`'s.
- Whether `oag_render`/`crates/game` should implement this shake at all, and for which title(s) - unimplemented either way, and a design call once the PSP question above is answered.
- `Camera_ArmShake`'s offset vector (`camera+0x40`) consumer inside `FUN_0013e280` was not traced past the accumulator calls that write it - unconfirmed whether it reaches the eye position the same function later uploads.

## Next Steps

- Read the PSP's own per-frame camera-update function (the counterpart of PS2's `FUN_0013e280`) for an internal shake-decay block of the same shape, independent of whatever calls into `Ship_DispatchCollisionFx`.
- Once that's settled either way, decide whether to implement the shake in `oag_render` and for which title(s) it's evidenced.
