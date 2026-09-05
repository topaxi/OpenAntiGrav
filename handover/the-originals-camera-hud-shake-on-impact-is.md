# The camera shake on impact is implemented; reproducing its two-axis rotation is not

A hard wall hit in the original visibly shakes the camera/HUD; `oag_render` has no shake at all.

**2026-09-03: found and corroborated on both discs.** PS2's `Camera_ArmShake` (`0x0013eec0`) and its apply side inside `FUN_0013e280` were found first, from the collision-response path - full account in [`docs/ghidra/functions/ps2-pulse-eu/collision-shake.md`](../docs/ghidra/functions/ps2-pulse-eu/collision-shake.md). A maintainer's own memory of playing the PSP/Pure builds ("this happens there too") sent the search back to the PSP disc, where the mechanism turns out to already have been sitting in plain sight, mislabelled: `docs/ghidra/functions/psp-pulse-usa/contact-response.md`'s own retraction ("nothing in this function's body calls a camera API") was checking the wrong call - the "audio cue" it left unread at confidence 40 is `Camera_ArmShake` (now `0x08878750`, confidence 88), field-for-field the same function as the PS2's, called from the same collision-response path (`Ship_DispatchCollisionFx`) with the same severity clamp, the same `[0.2, 0.8]` phase randomiser, and the same front/behind mode branch. That correction is written up in place on `contact-response.md` rather than as a separate note, per this project's own convention for a retraction that itself needed retracting.

**So this is settled, not open: both PSP and PS2 arm and apply the same shake, from the same collision event, at the same severity.**

**2026-09-03, later the same day: the apply side is a rotation, not a translation - and the envelope/scale constants are read, not guessed.** `Camera_ArmShake`'s writes and `FUN_0013e280`'s reads were re-checked field-for-field (full accounting in
[`docs/ghidra/functions/ps2-pulse-eu/collision-shake.md`](../docs/ghidra/functions/ps2-pulse-eu/collision-shake.md)):

- The falloff is a **three-key piecewise-linear envelope**: `magnitude * 0.25` at progress 0, `magnitude * 0.125` at progress 0.3, `0.0` at progress 1.0 - authored by `Camera_ArmShake` itself, not a separate table.
- The two arm-call scale constants are read directly from both binaries' `.data`: **magnitude scale `0.3`, duration `0.6` seconds flat** (`DAT_0027e8dc`/`DAT_0027e8e0` on PS2, `DAT_08ab0dfc`/`DAT_08ab0e00` on PSP - identical bytes).
- **The "offset vector" this thread previously flagged as unconfirmed is not an offset at all.** `param_1 + 0x40` is the first row of the camera's own 3x4 basis matrix. `FUN_0025cbb0` (PS2) and `func_0x002676b4` (PSP, `0x08a6b6b4`) both build a Rodrigues axis-angle rotation matrix from the shake's oscillator output and `vmmul_t` it into that basis - confirmed independently on both binaries. **The shake rotates the camera's orientation; it does not translate the eye.**

**2026-09-03, implemented.** `oag_render::camera::shake::Shake` reproduces the
confirmed envelope, timing, severity scale and per-mode combination exactly;
wired into `oag-game`'s `Race` at the same wall-contact site the hull sparks
already fire from, applied as a post-rotation on the view matrix. The
rotation axis is the one unconfirmed piece and is implemented as this
module's own flagged choice (the camera's local right) rather than a
reading - see the module's own doc comment for the full accounting of what
is confirmed and what is not. `just` passes clean with the implementation in.

**2026-09-05: the rotation axis is settled, and it is not a fixed axis.** `FUN_0025ca48`'s `in_a1_qw` traces (by disassembly alone - no p-code needed) to a fresh `lq a1` off one of the camera's own three live basis rows (`+0x40`/`+0x50`/`+0x60`) immediately before every call, picked by `shake_mode`, with **two sequential rotations per frame** (row1 then row2 for `Ahead`/`Elsewhere`, row1 then row0 for the third mode) rather than one rotation about one axis. Cross-checked independently on the PSP's `Camera_SubmitScene`, whose decompile shows the same axis as an explicit pointer argument at the same three offsets - confidence 92 (PS2) / 95 (cross-checked). Full accounting in [`docs/ghidra/functions/ps2-pulse-eu/collision-shake.md`](../docs/ghidra/functions/ps2-pulse-eu/collision-shake.md)'s "Settled 2026-09-05" section.

This falsifies the premise this thread's own "Next Steps" item bet on: the consequence was scoped to "`AXIS` and `Shake::rotation` are the only things that would need to change" if the axis turned out not to be a fixed local-right. It isn't fixed at all, and a correct fix is bigger than that - the axes are the camera's own current orientation state, not a constant, so reproducing this needs the camera's live basis threaded into whatever plays the shake (an API/signature change), not a swapped constant. Making the two-item edit anyway would produce a plausible-looking axis choice that still isn't what the disc does - exactly the stand-in this project's own rule warns against - so it was not made. `oag_render::camera::shake`'s module doc comment, `AXIS`'s doc comment and `Shake::rotation`'s doc comment are corrected to stop presenting the current single-fixed-axis, single-rotation shape as an unconfirmed reading and say plainly that it's a known simplification with the axis question already answered. No logic changed.

## Open

- Threading the camera's live basis into `Shake`'s rotation (or into whatever calls it) so two sequential rotations about the camera's own current row1/row2 (or row1/row0) can be reproduced - a signature/API change, not scoped here.
- What each basis row physically represents (world right/up/forward directly, vs. the transposed "columns are world axes" reading `positional-audio.md` established for the active camera elsewhere) - confidence ~55, not needed to answer the axis-selection question above.
- Whether the shake's basis-row writes are read back (and so accumulate) before or after the per-frame copy-out/orthogonalise pass in `FUN_0013e280` - order suggests they're read before, not traced further.
- Whether Pure, HD/Fury and 2048 carry the same mechanism - a maintainer's guess that it continues into newer titles, not yet checked on any of them.
- Whether the shake reads right on screen against the real games - nobody has yet compared this implementation's motion against a capture or a play session, only against the recovered arithmetic.

## Next Steps

- Design and implement the API change needed to pass the camera's live basis into the shake's rotation, then reproduce the two-sequential-rotations-per-mode shape exactly, rather than the current single-axis approximation.
- Check HD/Fury and 2048 for the same collision-response shape (a `min(|impulse| * k, 1)` severity feeding both a spark trigger and a camera-shake arm) if/when either title's collision path is read for other reasons - not worth a dedicated pass on its own yet.
- Play-test or capture-compare the landed implementation against the real games at some point, the way `crates/game/tests/chase_camera_ground_truth.rs` did for the chase camera - nothing here has been checked against a running original yet, only against its decompiled arithmetic.
