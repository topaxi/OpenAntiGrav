# `pair::respond` builds `vn` from the wrong angular-velocity convention

2026-09-10. Found while closing the wall path's point-velocity question with a
negative result (`handover/gameplay/craft-collision-feel-sticking-vs-spin.md`).
**This is a confirmed defect, not an open question** - the algebra and the
measurement are both in hand - but fixing it alone would be building on a
premise that the same measurement puts in doubt, so it is handed over whole
rather than half-done.

## The defect

`Body_ResolveContactPair` builds each body's point velocity as
`v + cross(r, R^T omega)`, where `omega` is `body+0x150` and `R` the basis rows
at `body+0x00..0x30`. `crates/physics/src/pair.rs`'s `contact_velocity`
reproduces that literally, feeding this crate's `Body::angular_velocity`
through `orientation * (-omega.x, omega.y, -omega.z)`.

**`body+0x150` is negated body-local, not world-space.** Fitted against the
rotation the recorded basis actually performs, over four captures
(`scripts/omega-column-reading-fit.py`), `NegatedLocal` scores a 2 % residual
against a 1.0-1.6 rad/s signal where a world-space reading scores 200 %. So

```text
+0x150      = -R * omega_true
R^T(+0x150) = -omega_true
v_p         = v + cross(r, -omega_true) = v + omega_true x r      <- textbook
```

and `Body::angular_velocity` is `omega_true` (the `w_game = -w_physics`
contract `crates/physics/src/integrate.rs` states as a whole-crate rule). The
half-turn `contact_velocity` applies is only `R^T(+0x150)` when the field holds
the original's raw bytes. In the running simulation it does not, so `vn`'s
angular contribution comes out with the wrong sign - the same mistake, in the
same shape, that `wall.rs` was briefly given and that cost `07_Track` its clean
lap.

**Why the pinned test passes**: `pair/tests.rs`'s `captured()` fixture puts the
raw `+0x150` triple straight into `angular_velocity`, so the test checks the
arithmetic against the original's own bytes - correctly - while building a
`Body` the simulation would never produce.

**Scope**: only `vn`'s angular contribution, and therefore `j`'s magnitude and
the `SEPARATING_LIMIT` gate. The pair path's headline finding is untouched - the
impulse is applied at each body's own centre, so a craft-to-craft hit still
imparts no angular velocity.

## Open

- **The `+0x80` reading is the premise to re-derive first.** The claim that
  `+0x150` is world-space rested on `+0x150 == inverse(I_world) * +0x160`, with
  `+0x80` the world inverse inertia and `+0x40` the body-space one. The fit
  above contradicts the conclusion, so either that identity or the block's
  convention is misread - and the *denominator* findings on **both** contact
  paths (the body-space diagonal at `+0x40..0x70` applied to a world-space
  `r x n`, no rotation) live next to the same block. Nothing measured so far
  disturbs those findings, but they are now the neighbours of a known
  misreading, and correcting `vn` while the tensor blocks are unre-derived is
  how a second confident wrong answer gets committed.
- Whether `Ship_HoverTwoPoint` builds a point velocity of its own is unread.
  `crates/physics/src/hover.rs:786` calls `Body::velocity_at` for the hover
  probe; if the original has an equivalent expression there it is a third site
  and should be checked under the same reading.

## Next Steps

1. Re-derive the `+0x40` / `+0x80` / `+0xc0` tensor blocks against the
   negated-body-local reading of `+0x150`, in Ghidra plus the same trace fit.
   The tumbling body in the 2026-09-10 pair capture is the discriminating
   sample; `scripts/psp-pair-capture.py` records all four blocks per contact
   already, so the raw material exists without a new capture.
2. Then correct `pair::contact_velocity` to the textbook form and the
   `captured()` fixture's `angular_velocity` to `-R^T(raw)` (the true world
   rate) **in one change**. This is self-checking: `j` must still come out
   `43.16971` and `0.18440`, because `omega_ours == -R^T omega_raw` makes
   `omega_ours x r` identically `cross(r, R^T omega_raw)`. If either number
   moves, the frame mapping in the fixture is wrong and that is the thing to
   fix, not the tolerance.
3. Re-run `race_ground_truth` (expect no change - a lone craft never touches
   another craft) and the weapons-live pair-tick metric
   `craft_sticking_ground_truth` prints, which is the one that can legitimately
   move.
