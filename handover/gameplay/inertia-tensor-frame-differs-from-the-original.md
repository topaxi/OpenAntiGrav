# The engine's inverse inertia is a world-axis diagonal; ours is a body-axis one

2026-09-10. Found while re-deriving the `+0x40`/`+0x80`/`+0xc0` tensor blocks,
which was step 1 of the `pair::respond` `vn` thread. **The measurement is
finished and it is exact; the implementation question is what is open.** The
`vn` correction itself landed and needed none of this - see
`docs/ghidra/functions/psp-pulse-usa/contact-response.md`.

## What is measured

`Body_Integrate` maps `body+0x160` (angular momentum) to `body+0x150` (angular
rate) through `R (body+0x40) R^T`, where `R`'s rows are the basis rows at
`body+0x00..0x30`. Both fields hold their quantity **negated and in body
coordinates**, so the two rotations cancel out of the physics and what is left
is

```text
omega_world = (body+0x40) * L_world
```

with `body+0x40` a constant diagonal `Body_SetBoxInertia` writes once at
construction. The engine's inverse inertia is therefore **fixed in world axes**.

`scripts/trace-inertia-frame-fit.py` predicts the recorded `+0x160` column from
the recorded `+0x150` column and the recorded basis under each candidate pairing
of frames. This one explains **100.00 %** on all three Talon's Junction captures
(6,423 ticks) and its free three-parameter fit returns `(15.600, 21.600,
15.600)` - the constructor's own code-literal tensor, recovered from a column it
was never compared against. The body-local and mixed readings score 91-95 %.
The full derivation, with the instruction addresses each frame is pinned by, is
`docs/ghidra/functions/psp-pulse-usa/rigid-body.md`.

## Why nobody noticed, and why it might matter

`diag(a, b, a)` - `15.6` on right *and* forward - is invariant under yaw. On a
level craft the world-axis and body-axis readings are **identical**, which is
how the engine gets away with it and how the doc page carried the labels the
wrong way round for three weeks. They differ only while the craft is pitched or
rolled: restricted to `up_y < 0.9` on the clean lap, the body-axis reading falls
to 76.19 % explained while this one stays at 100.00 %.

`crates/physics/src/integrate.rs` applies the inertia as a body-space diagonal -
`R^T (I^-1 (x) (R tau))` - which is the physically correct treatment and not the
original's. `docs/physics/cornering-ground-truth.md` records the basis rotating
`0.231x` and `0.647x` of what the momentum column accounts for on **pitch and
roll**, and closing at `0.970x` on yaw. That is the same axis split this
difference has. **Whether the one explains the other is unmeasured** and is the
first thing to check; a shape agreeing is not a magnitude agreeing.

## Open

- Nothing about the *measurement*. It is exact on three independent captures and
  derived independently at instruction level. Do not re-derive it; read
  `rigid-body.md` and `scripts/trace-inertia-frame-fit.py`.
- **Whether to implement it.** Changing `integrate.rs` to apply the diagonal in
  world axes moves the simulation, so it will move
  `crates/physics`'s `the_simulation_matches_the_committed_reference` and can
  move `race_ground_truth`. That is a whole lane's worth of work and it is why
  the `vn` thread did not take it.
- Faithfulness is not automatically the goal here. A world-axis inertia tensor
  is a shortcut with a real cost - a craft's resistance to pitching depends on
  which way it is facing the moment it is banked - and this project has a
  precedent for calling that kind of thing **chosen, not measured** where play
  is better for it. Decide that deliberately rather than by reflex.

## Next Steps

1. Measure before implementing: run `scripts/trace-omega-identity.py` and
   `docs/physics/cornering-ground-truth.md`'s own check against a build with the
   world-axis map in `integrate.rs`, and see whether the `0.231x`/`0.647x`
   pitch and roll gaps close. That is a throwaway branch, not a commit.
2. If they close, it is a real fix and worth the reference-hash churn; land it
   with the hash regenerated **for the right reason and said out loud**, and
   re-quote `race_ground_truth` either side.
3. If they do not, write the negative result into
   `docs/physics/cornering-ground-truth.md` and leave `integrate.rs` alone -
   the divergence is then a known, deliberate one and belongs in that page
   rather than in a thread.
