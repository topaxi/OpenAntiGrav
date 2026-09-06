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
[input-bindings.md](../docs/ghidra/functions/psp-pulse-usa/input-bindings.md#the-roll-is-drawn-0x87c-eases-into-0x880-which-rolls-the-ship-about-its-nose).
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
[camera.md](../docs/ghidra/functions/psp-pulse-usa/camera.md)). So the cockpit
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

- **The sign of the roll is not determined, and it now ships as a chosen
  value rather than a measured one.** The VFPU register-name row/column
  ambiguity reaches exactly the `sin`/`-sin` placement and nothing else - the
  *axis* survives it, because both literal `1.0`s sit on the diagonal, which
  is transpose-invariant. So "about the nose" is measured and "clockwise or
  anticlockwise for a left-right-left gesture" is not. `headtilt`'s own
  unresolved sign made the same case for holding back entirely; this one
  shipped anyway, deliberately, because a wrong roll direction is a one-line
  fix in a single named constant and the maintainer chose to play-test it
  live rather than wait on a capture. See Next Steps.
- **`craft+0x854 * 0.5`, the angle's other term, must not be transcribed.** It
  is `FUN_0883fab4`'s steering lean, which camera.md scores at confidence **0**
  for what it physically means. The roll term stands alone and is complete
  without it. This *does* mean the reimplementation's total roll angle will
  differ from the original's by whatever the lean contributes - an honest,
  documented gap rather than a guessed constant.
- **`vmmul.q`'s operand order** - `R * body` or `body * R` - was not
  established. The two agree while the body matrix is orthonormal and diverge
  when it is not.
- **`[0x002ace1c]`**, the scalar the original applies to rows 0-2 of the composed
  matrix before submitting it, is unidentified. camera.md's `g_craft_scale`
  discussion is the first place to look.
- Whether the roll should also reach anything else - a trail, an exhaust, a
  sound - is untouched. Nothing was found, but only `+0x87c` and `+0x880` were
  swept.

## Next Steps

**Steps 2-4 landed 2026-09-06.** The drawing half of this thread is done and on
screen; only the sign remains open, and it is now a play-test-and-flip loop
rather than a blocker.

1. **Settle the sign with a memory-read capture, not "watch the ship roll".**
   The original plan was to eyeball a scripted `scripts/psp-drive.py` d-pad
   alternation; the better version reads the display matrix straight out of
   PPSSPP's memory during that same scripted roll, the way the sideshift
   capture in `docs/reverse-engineering/ppsspp-debugger.md` did - headless,
   no pixels needed, and it settles the sign outright rather than by eye. This
   is queued separately and does not block anything below; whichever way it
   comes back, flipping the answer costs one line (see step 2).
2. ~~Port the ease as a pure function in `oag-render`~~ **Done**:
   `oag_render::roll::ease` in `crates/render/src/roll.rs`, with the pinned
   unit tests plus a continuity check at the `+/-0.5` seam.
3. ~~Apply the rotation to the ship's transform~~ **Done**:
   `model_matrix_of` in `crates/game/src/race/drawable.rs` composes it into
   `body.orientation`, about the local `-Z` axis `oag_physics::ship::Body::
   forward` already uses.
4. ~~Roll the internal camera's up vector~~ **Done**, alongside
   `<InternalCamera headtilt>` rather than instead of it:
   `oag_render::camera::internal::view` in `crates/render/src/camera/
   internal.rs` takes `roll_phase` and rotates the up vector about the view's
   forward axis before the look-at is built.
5. **The sign is chosen, not measured, and lives in exactly one place:**
   `ROLL_DIRECTION` in `crates/render/src/roll.rs`. It is set to `1.0` so
   that a `[1, 2, 1]` gesture (tap LEFT first, which ramps `roll_phase`
   toward `-1.0`) drops the left wing - the mapping a player's own gesture
   would suggest, and the reasoning is written on the constant itself. If a
   play-test or the capture in step 1 says it reads backwards, flip that one
   constant to `-1.0` and nothing else changes: both call sites read it
   through `oag_render::roll::angle`, and
   `roll::tests::opposite_phases_rotate_oppositely_by_equal_magnitude` and
   `camera::internal::tests::opposite_roll_phases_produce_different_and_symmetric_views`
   are sign-agnostic, so neither test needs touching either.
6. Identify `[0x002ace1c]` and `vmmul.q`'s order if either turns out to matter
   once the picture is on screen; neither blocked steps 2-4. `vmmul.q`'s
   ambiguity in particular does not reach this port: `model_matrix_of`
   composes the roll into `body.orientation` (a local-space rotation about an
   orthonormal quaternion) before the scale is applied, which is the case the
   thread's own "Open" section already says the two operand orders agree on.
