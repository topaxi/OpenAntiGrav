# The camera shake on impact is located, on both PSP and PS2 - implementing it is what's left

A hard wall hit in the original visibly shakes the camera/HUD; `oag_render` has no shake at all.

**2026-09-03: found and corroborated on both discs.** PS2's `Camera_ArmShake` (`0x0013eec0`) and its apply side inside `FUN_0013e280` were found first, from the collision-response path - full account in [`docs/ghidra/functions/ps2-pulse-eu/collision-shake.md`](../docs/ghidra/functions/ps2-pulse-eu/collision-shake.md). A maintainer's own memory of playing the PSP/Pure builds ("this happens there too") sent the search back to the PSP disc, where the mechanism turns out to already have been sitting in plain sight, mislabelled: `docs/ghidra/functions/psp-pulse-usa/contact-response.md`'s own retraction ("nothing in this function's body calls a camera API") was checking the wrong call - the "audio cue" it left unread at confidence 40 is `Camera_ArmShake` (now `0x08878750`, confidence 88), field-for-field the same function as the PS2's, called from the same collision-response path (`Ship_DispatchCollisionFx`) with the same severity clamp, the same `[0.2, 0.8]` phase randomiser, and the same front/behind mode branch. That correction is written up in place on `contact-response.md` rather than as a separate note, per this project's own convention for a retraction that itself needed retracting.

**So this is settled, not open: both PSP and PS2 arm and apply the same shake, from the same collision event, at the same severity.**

**2026-09-03, later the same day: the apply side is a rotation, not a translation - and the envelope/scale constants are read, not guessed.** `Camera_ArmShake`'s writes and `FUN_0013e280`'s reads were re-checked field-for-field (full accounting in
[`docs/ghidra/functions/ps2-pulse-eu/collision-shake.md`](../docs/ghidra/functions/ps2-pulse-eu/collision-shake.md)):

- The falloff is a **three-key piecewise-linear envelope**: `magnitude * 0.25` at progress 0, `magnitude * 0.125` at progress 0.3, `0.0` at progress 1.0 - authored by `Camera_ArmShake` itself, not a separate table.
- The two arm-call scale constants are read directly from both binaries' `.data`: **magnitude scale `0.3`, duration `0.6` seconds flat** (`DAT_0027e8dc`/`DAT_0027e8e0` on PS2, `DAT_08ab0dfc`/`DAT_08ab0e00` on PSP - identical bytes).
- **The "offset vector" this thread previously flagged as unconfirmed is not an offset at all.** `param_1 + 0x40` is the first row of the camera's own 3x4 basis matrix. `FUN_0025cbb0` (PS2) and `func_0x002676b4` (PSP, `0x08a6b6b4`) both build a Rodrigues axis-angle rotation matrix from the shake's oscillator output and `vmmul_t` it into that basis - confirmed independently on both binaries. **The shake rotates the camera's orientation; it does not translate the eye.**

What's left is squarely implementation and design, not reading.

## Open

- The rotation *axis* itself - Ghidra drops the carrying (non-GPR) argument from the decompiled C signature at every call level (`FUN_0013e280` -> `FUN_0025cbb0` -> `FUN_0025ca48`), so which axis (or axes, per `shake_mode`) the rotation happens around needs a raw p-code/register read, not another decompile. See collision-shake.md's "Not determined".
- Whether `oag_render`/`crates/game` should implement this shake, for which title(s), and how it composes with the existing chase-camera smoothing - a design decision, not a reading.
- Whether Pure, HD/Fury and 2048 carry the same mechanism - a maintainer's guess that it continues into newer titles, not yet checked on any of them.

## Next Steps

- Decide whether to implement the shake in `oag_render`, and for which title(s). Given the axis is unresolved, an initial implementation would need to pick a reasoned, clearly-flagged axis choice (e.g. the camera's own right/up axes) rather than the exact original - document that as an approximation per this project's own rule against invented stand-ins, distinct from the fully-confirmed envelope/timing/severity maths.
- If time allows, read `FUN_0025ca48`'s `in_a1_qw` argument's register origin via p-code to settle the axis before committing to an approximation.
- Check HD/Fury and 2048 for the same collision-response shape (a `min(|impulse| * k, 1)` severity feeding both a spark trigger and a camera-shake arm) if/when either title's collision path is read for other reasons - not worth a dedicated pass on its own yet.
