# ADR-0039: Camera jitter post-multiplies onto the view-projection, after the frustum and the motion snapshot

## Status

Accepted, 2026-09-02. Completes the renderer-side prerequisite list
[modern features](../../overview/modern-features.md) keeps for FSR 3.1. Builds
on [ADR-0030](0030-velocity-buffer-motion-blur.md), whose velocity attachment
is the thing the ordering here exists to protect.

## Context

A temporal upscaler reconstructs detail by sampling a still scene at a different
point inside each pixel every frame. That offset is the renderer's job to apply;
FSR 3.1 is *told* what the offset was and undoes it itself.

This project had four renderer-side prerequisites for that upscaler, and jitter
is the last one. Nothing consumes it yet, and **on its own it makes the picture
worse**: with a spatial resolve or none, all it does is move every edge a
fraction of a pixel a frame, which is a shimmer with no reconstruction behind
it. It is built ahead of its consumer anyway, because the thing that is easy to
get wrong about jitter is not the sequence - it is *where the offset is applied*,
and that is much cheaper to establish now than to retrofit against an upscaler
that is already ghosting and could be ghosting for either reason.

Two things must not see the offset:

- **The culling frustum.** `race::Scene::render` builds a `Frustum` from the
  same matrix it rasterises with. A plane that wobbles by a sub-pixel each frame
  lets a bound sitting exactly on the screen edge fall in and out of the visible
  set at the sequence's period - geometry flickering along the border, once
  every sixteen frames, only ever with jitter on.
- **The velocity buffer.** Every drawable carries a `prev_mvp`
  ([ADR-0030](0030-velocity-buffer-motion-blur.md)) and the shader writes the
  difference between the current and previous clip positions. An offset that
  reaches one and not the other, or reaches both at different phases, puts the
  jitter into every pixel's motion vector - which is the one thing a temporal
  upscaler must not be handed, since it applies the offset itself and would
  double-count.

## Decision

**The offset is a clip-space translation post-multiplied onto the
view-projection**, and onto the previous tick's matrix with the same phase:

```
let frustum = cull.then(|| Frustum::from_view_projection(view_projection));
// everything above reads the unjittered camera, everything below the jittered
let jitter = oag_post::jitter::matrix(frame, (viewport.2, viewport.3));
let (view_projection, prev_vp) = (jitter * view_projection, jitter * prev_vp);
```

Five parts, each load-bearing:

1. **A matrix, not a shader change.** A translation's contribution is scaled by
   the vector's `w`, so `T * VP` is exactly `clip.x += x * clip.w` - a constant
   offset in NDC after the perspective divide, and therefore a constant offset
   in pixels. `z` and `w` come through untouched, so the depth test, the fog's
   view distance and the Zone glow's own `clip.z` read all keep meaning what
   they meant. No `.wgsl` file changes at all.

2. **The same phase on both matrices**, which is what makes the offset cancel
   out of the velocity target exactly. Both clip positions shift by the same NDC
   amount; the shader measures their difference; the jitter drops out. This is
   why part 1 is sufficient - without the cancellation the velocity write would
   have needed its own un-jittered pair of matrices in the uniform block, on
   every draw.

3. **Applied by shadowing, not at each use site.** `Scene::render` uses
   `view_projection` at about a dozen places. A site missed would be wrong only
   with jitter on, which is off by default, so no test and no capture would
   catch it. Rebinding the name once means every site downstream picks it up and
   none can be missed. `prev_vp * previous` further down stays correct because a
   matrix product is associative.

4. **Positioned after the frustum and after `MotionState::advance`.** This is
   the decision proper: the ordering *is* the guarantee, enforced by line
   position rather than by anyone remembering it. Culling reads the matrix from
   a line earlier; the snapshot a future frame will measure against is stored
   unjittered.

5. **Sixteen Halton(2,3) phases, and that count is ours.** A temporal upscaler
   conventionally derives its phase count from the presentation-to-render
   resolution ratio. That formula belongs to the FSR 3.1 port that will consume
   this - writing a guess at it here would dress an invention as an upstream
   constant, which [ADR-0012](0012-wgsl-upscalers-not-native-fidelityfx.md)'s
   transliteration property exists to prevent. The sequence is 1-indexed so no
   frame lands on the pixel centre, and centred on it so a jittered render does
   not sit a consistent half-pixel off an unjittered one.

**The surface is `--camera-jitter` and nothing else** - no `[graphics]` key, no
menu row. A settings key implies a choice a player should be making, and there
is no version of this that a player wants until something reconstructs from it.
A value persisted now would also still be set when the real gate ("is a temporal
upscaler selected") arrives to replace it. `--zone-spectrum-test` is the
precedent.

## Alternatives rejected

- **Jitter the projection matrix at its source**, inside `Race::projection`.
  Simpler to write and wrong: `Frustum::from_view_projection` and
  `MotionState::advance` both read the result, so culling and the velocity
  buffer would both be perturbed, and the reticle's own projection
  (`race::weapons`, its own FOV and far plane) would be jittered too although it
  is HUD content drawn at presentation resolution.
- **Un-jitter in the shader**, keeping a separate un-offset pair of matrices for
  the velocity write. This is what a renderer that jitters at the source has to
  do. It adds 64 bytes to every draw's uniform block - a layout mirrored by five
  `.wgsl` declarations and by the asset viewer's own buffer sizing - to solve a
  problem that applying the same phase to both matrices does not have.
- **A random offset instead of a low-discrepancy sequence.** Halton covers the
  pixel evenly at every prefix length, so a scene that stops moving after four
  frames has been sampled at four well-spread points rather than four that
  happen to cluster.

## Consequences

- **The last FSR 3.1 prerequisite row is filled**, and the four are now
  independent of each other: decoupled render resolution, readable depth,
  per-draw motion vectors, and jitter. What remains for FSR 3.1 is the port
  itself.
- **`--camera-jitter` is a strictly worse picture today** and is documented as
  such on the flag. It is a correctness affordance, not a quality one.
- **`RaceStage::warm_up` costs a phase.** It draws one discarded frame through
  `Scene::render`, and the counter advances on every render whether or not
  jitter is on, so the first frame a player sees is phase one rather than phase
  zero. Deterministic, and stated here so a capture that shifts is not a
  mystery.
- **The counter is per *frame*, not per tick**, unlike the motion snapshot
  beside it. Above 60 Hz the same tick is drawn several times and each redraw
  must sample somewhere new, or a stationary scene stops accumulating.
- **Nothing outside the race's 3D scene moves.** The HUD, the scoreboard and
  the reticle take a viewport rectangle or their own projection, never
  `Scene::render`'s matrix, and since
  [ADR-0038](0038-a-stage-with-no-scene-draws-at-presentation-resolution.md) no
  UI stage enters the offscreen target at all.
- **Verified end to end, not only in the unit tests.** Two `--race --ticks 300
  --screenshot` captures differing only by the flag: 674,047 of 1,175,040 pixels
  differ, mean absolute channel delta 2.95 over those, peak 190 at
  high-contrast edges - the signature of a sub-pixel shift rather than a whole
  one. `crates/post/src/jitter/tests.rs` carries the nine unit tests,
  including the velocity cancellation, which was checked non-vacuous by giving
  the previous matrix a different phase.
