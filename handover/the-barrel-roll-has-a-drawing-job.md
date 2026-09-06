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

- **The sign of the roll is not determined, and applying it wrong is worse than
  not applying it.** The VFPU register-name row/column ambiguity reaches exactly
  the `sin`/`-sin` placement and nothing else - the *axis* survives it, because
  both literal `1.0`s sit on the diagonal, which is transpose-invariant. So
  "about the nose" is measured and "clockwise or anticlockwise for a
  left-right-left gesture" is not. camera.md already declined to apply
  `headtilt` on exactly this reasoning ("a lean applied with the wrong sign
  leans the horizon the wrong way through every corner, which is worse than a
  horizon that does not lean"); the same bar applies here.
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

1. **Settle the sign with one capture.** `scripts/psp-drive.py` can already
   script the d-pad alternation, and the mechanic's own thread wants a capture
   for an unrelated reason (whether the original arms on the ground) - so one
   run serves both. Watch the ship roll and write down which way. Until this
   lands, step 2 has a coin-flip in it.
2. **Port the ease as a pure function in `oag-render`**, off
   `ShipState::roll_phase`, with a unit test pinning `ease(0) = 0`,
   `ease(+/-0.5) = +/-0.5`, `ease(+/-1) = +/-1` and the odd symmetry. This part
   has no sign ambiguity in it at all and can land before step 1.
3. **Apply the rotation to the ship's transform** about its forward axis, by
   `eased * 6.28` radians, once step 1 has settled the sign.
4. **Roll the internal camera's up vector by the same angle**, alongside
   `<InternalCamera headtilt>` rather than instead of it - they are two terms on
   one vector and the original applies both.
5. Identify `[0x002ace1c]` and `vmmul.q`'s order if either turns out to matter
   once the picture is on screen; neither blocks steps 1-4.
