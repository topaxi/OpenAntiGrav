# The barrel roll has a drawing job, and the original's own recipe for it

`oag_physics::ShipState::roll_phase` ramps every tick a pilot rolls and
**nothing outside `oag-physics` reads it**, so a barrel roll is currently a
handling change with no picture. The original does draw it, and the recipe was
recovered on 2026-09-06 at confidence **88**. This thread is the drawing half,
split off from the mechanic's own thread
([the-barrel-roll-is-read-and-unimplemented.md](the-barrel-roll-is-read-and-unimplemented.md))
because the RE and the render work are different jobs and the RE half was its
own session.

Nothing here needs inventing. **Do not author a stand-in roll animation** - the
shape is measured and written down; if a piece of it turns out to be missing,
draw nothing for that piece and say so.

## What the original does

Full evidence, with every address and constant, is in
[input-bindings.md](../../docs/ghidra/functions/psp-pulse-usa/input-bindings.md#the-roll-is-drawn-0x87c-eases-into-0x880-which-rolls-the-ship-about-its-nose).
The short form is two stages.

**Stage 1 - the ease.** `FUN_088418e0` reads the linear phase `craft+0x87c` and
stores an eased copy to `craft+0x880` (`0x08841f88`):

```
ease(p) = 0                          p == 0
        =  2p^2                      0    <  p <= 0.5
        =  1 - 2(1-p)^2              0.5  <  p
        = -2p^2                     -0.5 <=  p <  0
        = -1 + 2(-1-p)^2                    p < -0.5
```

The symmetric quadratic ease-in-out. Odd in `p`, continuous at `+/-0.5` where it
takes `+/-0.5`. This is why the roll does not snap in and out of level.

**Stage 2a - the ship.** The same function turns the eased value into an angle
and rotates the craft's display matrix by it:

```
angle = craft[0x854] * 0.5 + craft[0x880] * 6.28
```

wrapped into `(-pi, pi]`, then a rotation **about the craft's nose** - the third
row of the rigid body's 4x4 at `craft+0x794`, the forward axis - composed with
that body matrix, scaled, and submitted to the ship's scene node.

**Stage 2b - the camera.** `FUN_088455ec`, the internal rig, does
`up = Rot(craft[0x880] * 6.28) * up` (already on
[camera.md](../../docs/ghidra/functions/psp-pulse-usa/camera.md)). So the cockpit
view rolls with the ship rather than staying level.

Two numbers to carry across as-is: `6.28` is a **code literal**, not authored
data, so a completed roll is `359.8` degrees and not quite `2*pi`; and the ease
is exact arithmetic, not an approximation of a smoothstep.

## Where it goes

`ShipState::roll_phase` is already the right input and needs no change -
`oag-physics` stays renderer-blind per this project's core principle, and the
ease belongs on the **render** side of the boundary, as a pure function of a
value the simulation already publishes.

- `crates/render` is exempt from the determinism rules (a pixel is not compared
  across machines), so the ease and the rotation may use whatever is fastest.
- The ship's transform is where the visible half lands; the internal camera's up
  vector is `oag_render::camera::internal`, which already exists and already
  carries `<InternalCamera headtilt>` parsed-but-unapplied for a *different*
  reason (an unknown sign). The roll's sign is unknown too - see below - so the
  same caution applies and the two should be decided together, not separately.

## Open

- **The internal camera's own site was not independently captured.**
  `FUN_088455ec`'s `up = Rot(craft[0x880] * 6.28) * up` was measured only by
  inference from the ship's own capture below (the same VFPU sign is assumed
  to be one binary-wide fact rather than a per-call-site choice), not by its
  own breakpoint. A capture identical in shape to the one below, breaking
  inside `FUN_088455ec` instead of `FUN_088418e0`, would turn that inference
  into a second measurement.
- **`craft+0x854 * 0.5`, the angle's other term, must not be transcribed.** It
  is `FUN_0883fab4`'s steering lean, which camera.md scores at confidence **0**
  for what it physically means. The roll term stands alone and is complete
  without it. This *does* mean the reimplementation's total roll angle will
  differ from the original's by whatever the lean contributes - an honest,
  documented gap rather than a guessed constant.
- Whether the roll should also reach anything else - a trail, an exhaust, a
  sound - is untouched. Nothing was found, but only `+0x87c` and `+0x880` were
  swept.

## Next Steps

**Steps 2-5 are done.** The drawing half of this thread is fully landed,
including the sign; only the internal camera's own corroboration (an
optional second measurement) and the two items above remain open, and
neither blocks anything.

1. ~~Settle the sign with a memory-read capture~~ **Done, 2026-09-08.** A
   ground d-pad `[2,1,2]`/`[1,2,1]` gesture produced no roll at all
   (`entity+0x87c` stayed `0.0`, `grounded=1.0` - this port's own airborne
   gate, `crates/physics/src/barrel_roll.rs`), so the capture wrote
   `entity+0x880` directly at a breakpoint instead and read back the
   original's own display matrix - full method, the six-sample sweep and the
   numbers are in
   [input-bindings.md](../../docs/ghidra/functions/psp-pulse-usa/input-bindings.md#settled-on-the-running-game-2026-09-08-the-sign-confidence-92).
   Confidence **92**. The same capture also closed `vmmul.q`'s operand order
   (`R * body`, since the captured row 0/1 mix only `body`'s rows, never its
   columns) and identified `[0x002ace1c]` as `0.75` = `g_craft_scale`, the
   `3/4` factor camera.md already had a name for.
2. ~~Port the ease as a pure function in `oag-render`~~ **Done**:
   `oag_render::roll::ease` in `crates/render/src/roll.rs`, with the pinned
   unit tests plus a continuity check at the `+/-0.5` seam.
3. ~~Apply the rotation to the ship's transform~~ **Done**:
   `model_matrix_of` in `crates/raceplay/src/drawable.rs` composes it into
   `body.orientation`, about the local `-Z` axis `oag_physics::ship::Body::
   forward` already uses.
4. ~~Roll the internal camera's up vector~~ **Done**, alongside
   `<InternalCamera headtilt>` rather than instead of it:
   `oag_render::camera::internal::view` in `crates/render/src/camera/
   internal.rs` takes `roll_phase` and rotates the up vector about the view's
   forward axis before the look-at is built.
5. ~~The sign is chosen, not measured~~ **Now measured**: `ROLL_DIRECTION` in
   `crates/render/src/roll.rs` is `-1.0`, flipped from the `+1.0` it shipped
   as - a `[1, 2, 1]` gesture (tap LEFT first, which ramps `roll_phase`
   toward `-1.0`) turns out to drop the **right** wing, not the left the
   constant originally assumed a player's own gesture would suggest.
   `crates/raceplay/src/drawable.rs`'s
   `roll_direction_matches_the_original_captured_display_matrix` pins it
   against the captured body and phase directly, through this crate's own
   rotation composition rather than hand algebra, so a future regression on
   this constant fails a test rather than waiting for a play-test to notice.

## From the HANDOVER.md index (moved 2026-09-25)

drawn and on screen since 2026-09-06: a symmetric quadratic ease on `ShipState::roll_phase`, then `eased * 6.28` radians about the craft's **nose** on its display matrix, and the same angle on the internal camera's up vector, all at confidence 88. **The sign is now measured too (2026-09-08, confidence 92)**: a memory-read capture against a real, booted `pulse-psp-usa.chd` settled the VFPU `sin`/`-sin` ambiguity the axis survived but the sign didn't, and closed `vmmul.q`'s operand order and `[0x002ace1c] = 0.75 = g_craft_scale` in the same pass - `oag_render::roll::ROLL_DIRECTION` is `-1.0`, flipped from the `+1.0` a player's own gesture would have suggested. Only the internal camera's own site (`FUN_088455ec`) was not independently captured; it inherits the ship's measured sign as a labelled inference. `craft+0x854`'s lean term shares the angle and must not be transcribed - confidence 0 upstream. **Also found this pass**: every offset in this section is on the **entity**, not the craft `Ship_UpdateCraft` itself takes - the third time this exact trap has bitten the project
