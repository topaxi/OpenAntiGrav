# ADR-0030: The velocity buffer lands - measured per-draw motion replaces camera reprojection and the focus mask

## Status

Accepted. Supersedes [ADR-0028](0028-camera-motion-blur-first.md)'s
camera-reprojection technique and [ADR-0029](0029-primer-capture-and-craft-focus-mask.md)'s
focus mask (that ADR's primer capture and its raised stretch cap both stand).
This is the ADR [`docs/rendering/motion-blur.md`](../../rendering/motion-blur.md)
says its velocity tier owes when built.

## Context

The cheap tier shipped first as sanctioned, and its two rounds of live
feedback were exactly the design page's predicted failure modes: computed
camera velocity is wrong about everything that moves. The focus mask patched
the craft, but a mask is a list of exemptions, not a measurement - it
sharpened a rival sliding past (whose *relative* motion should smear), knew
nothing about airbrake flaps or debris, and had to be maintained. The design
chose the velocity buffer for a second reason that stands apart from the
blur: per-pixel motion vectors from every draw are a prerequisite FSR 3.1
and TAA share, and the blur is just their first consumer.

The design's precondition is settled: `Rg16Float` is multisample-renderable
at 4x on the development adapter, pinned as a live test
(`rg16float_is_multisample_renderable_at_4x_on_this_adapter`) with
`Rgba16Float` as the documented fallback should another adapter fail it.

## Decision

**Every race draw writes its measured screen-space motion into a second
colour attachment, and the blur becomes a reconstruction filter over it.**

- **The velocity target is `Rg16Float`, at the scene's own sample count,
  always written in the game path** - the setting gates the blur passes, not
  the buffer, so the buffer is a dependable artefact for FSR 3.1/TAA rather
  than a sometimes-there one. The value is `(cur_ndc - prev_ndc) * (0.5,
  -0.5)`: a uv delta, y flipped, computed per fragment because per-vertex
  division does not survive perspective interpolation. A device test reads
  the target back and pins the y flip - the classic invisible bug.
- **`Uniforms` gains one premultiplied `prev_mvp`** rather than a previous
  camera/model pair: fog and lighting need world position, velocity needs
  only clip position, and symmetry would cost 64 bytes per draw for nothing.
  The other pipelines mirroring the layout over-allocate harmlessly.
- **Depth-writing pipelines write velocity; blended ones carry the target
  write-masked empty.** The invariant: velocity and depth describe the same
  surface at every pixel. Camera-facing particle quads have no
  frame-to-frame vertex correspondence at all (the recovered exhaust has no
  history buffer), so their pixels keep the background's motion - the
  design's accepted artefact. The sky is the exception that proves the
  rule: it writes no depth but is the only thing at its pixels, and its
  `prev_mvp` uses the previous camera's *translation* so a horizon pans
  rather than approaches.
- **Previous transforms are tick-keyed** (`race::scene::frame::MotionState`):
  a snapshot of the camera, per-slot craft matrices and rocket matrices,
  promoted only when `race.world.tick` advances, so frames outrunning 60 Hz
  keep measuring one tick's travel and a paused world measures zero with no
  special case. A drawable with no previous entry (first frame, fresh
  rocket) measures against its current pose - zero object velocity, never a
  smear from stale state. The capture's primer render (ADR-0029) seeds this
  cache, which is why a `--screenshot` still shows the real smear.
- **The reconstruction is McGuire et al. (I3D 2012), written from the
  published description** per ADR-0012: prepare (velocity+depth folded to
  one texture, sample 0 under MSAA), tile-max at the reach cap's tile size,
  neighbour-max, then the depth- and velocity-weighted gather with per-pixel
  jitter, and the copy home. **The MSAA gate is gone**: sample-0 reads were
  always the design's answer, and only the prepare pass ever sees a
  multisampled binding.
- **The blur runs where it always did** - after bloom, before the HUD - and
  the strength row is untouched: the tier swap the row was designed for,
  with no settings migration.

## Alternatives considered

**Keep the focus mask alongside the buffer for extra craft sharpness.**
Rejected: the buffer measures the craft as still when they are still, and
where they are *not* still the mask was wrong and the buffer is right. Two
mechanisms disagreeing about the same pixels is how artefacts get
unattributable.

**Resolve the multisampled velocity instead of reading sample 0.** Rejected,
as the design already had: averaging velocity across a silhouette edge
produces a vector describing neither surface, which defeats the exact
classification the reconstruction's weights exist for.

**Per-frame previous transforms.** Rejected for the reason the design calls
non-obvious: above 60 fps consecutive frames re-render one tick, and a
per-frame previous reads those as zero motion - the blur would pulse at
exactly the frame rates where it is smoothest today.

## Consequences

- **Fifteen more pipelines per scene** (the velocity twins and masked
  targets ride existing ones - the real growth is the blur's six small
  fullscreen pipelines and four scratch targets) and 4 bytes/pixel/sample of
  velocity. Not profiled beyond "no visible cost in a capture run"; a
  frame-time number belongs in the docs once measured on real hardware.
- **Airbrake flaps carry their craft's velocity, not their own swing** - the
  CPU deforms their vertices, which `prev_mvp` cannot see. The design costs
  the analytic fix (two extra draws per craft with a previous swing angle);
  deferred, and the flaps are small and fast.
- **`Anim Transform` scenery blurs by camera motion only**: the node
  matrices in `prev_mvp`'s path are this frame's. A moving trackside object
  smears as if static - accepted and documented in `mesh.wgsl`.
- **A mid-list rocket despawn misassigns previous matrices for one frame**,
  bounded by the reach cap.
- **A still surface beside a moving region can dim by one count** at a
  hard edge: the gather's quarter-pixel cone floor gives sub-pixel taps a
  sliver of weight. Measured, asserted at exactly that bound in the device
  test, invisible in practice.
- **The temporal ledger after this ADR**: readable depth - done; per-draw
  motion vectors - **done**; sub-pixel camera jitter and a history buffer -
  still absent, so ADR-0013's "no `Taa` row" stands, two prerequisites
  lighter.
