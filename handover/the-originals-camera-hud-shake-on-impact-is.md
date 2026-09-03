# The camera shake on impact is located, on both PSP and PS2 - implementing it is what's left

A hard wall hit in the original visibly shakes the camera/HUD; `oag_render` has no shake at all.

**2026-09-03: found and corroborated on both discs.** PS2's `Camera_ArmShake` (`0x0013eec0`) and its apply side inside `FUN_0013e280` were found first, from the collision-response path - full account in [`docs/ghidra/functions/ps2-pulse-eu/collision-shake.md`](../docs/ghidra/functions/ps2-pulse-eu/collision-shake.md). A maintainer's own memory of playing the PSP/Pure builds ("this happens there too") sent the search back to the PSP disc, where the mechanism turns out to already have been sitting in plain sight, mislabelled: `docs/ghidra/functions/psp-pulse-usa/contact-response.md`'s own retraction ("nothing in this function's body calls a camera API") was checking the wrong call - the "audio cue" it left unread at confidence 40 is `Camera_ArmShake` (now `0x08878750`, confidence 88), field-for-field the same function as the PS2's, called from the same collision-response path (`Ship_DispatchCollisionFx`) with the same severity clamp, the same `[0.2, 0.8]` phase randomiser, and the same front/behind mode branch. That correction is written up in place on `contact-response.md` rather than as a separate note, per this project's own convention for a retraction that itself needed retracting.

**So this is settled, not open: both PSP and PS2 arm and apply the same shake, from the same collision event, at the same severity.** What's left is squarely implementation and design, not reading.

## Open

- `Camera_ArmShake`'s offset vector (PS2 `camera+0x40`) consumer inside `FUN_0013e280` was not traced past the accumulator calls that write it - unconfirmed whether it reaches the eye position the same function later uploads, though the equivalent PSP field almost certainly plays the same role given how closely the two match otherwise.
- Whether `oag_render`/`crates/game` should implement this shake, for which title(s), and how it composes with the existing chase-camera smoothing - a design decision, not a reading.
- Whether Pure, HD/Fury and 2048 carry the same mechanism - a maintainer's guess that it continues into newer titles, not yet checked on any of them.

## Next Steps

- Decide whether to implement the shake in `oag_render`, and for which title(s).
- If implementing, trace `+0x40`'s consumer on one binary to confirm it reaches the eye position rather than something else, before porting the maths blind.
- Check HD/Fury and 2048 for the same collision-response shape (a `min(|impulse| * k, 1)` severity feeding both a spark trigger and a camera-shake arm) if/when either title's collision path is read for other reasons - not worth a dedicated pass on its own yet.
