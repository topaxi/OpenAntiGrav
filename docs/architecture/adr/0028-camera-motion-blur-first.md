# ADR-0028: Ship camera-reprojection motion blur first, under the strength row the full design specified

## Status

Accepted.

This is the ADR [`docs/rendering/motion-blur.md`](../../rendering/motion-blur.md)
says its implementer owes (it names it "ADR-0024"; four ADRs landed between the
design and the build). It records a *narrower* first step than the design's
chosen tier, and why that is a stepping stone rather than a reversal.

## Context

The motion blur design chose the per-object velocity buffer tier - about a week
and a half - over cheap camera reprojection, mostly because the velocity buffer
clears two of FSR 3.1's five prerequisites while reprojection clears only the
depth row. That reasoning stands unchallenged.

The same design also stated, deliberately, that its setting is **strength, not
technique**, "so that if the cheap tier is ever shipped first as a stepping
stone, it can be replaced underneath the same row without a settings
migration."

This build takes that sanctioned path: the goal of the moment is the smallest
increment that starts the temporal infrastructure
[ADR-0013](0013-anti-aliasing-architecture.md) lists as missing for TAA -
motion vectors, jitter, a history buffer - and camera reprojection builds the
first of the three (previous-frame camera matrices, depth-based per-pixel
velocity) in a form TAA reuses directly, at roughly a day against a week and a
half.

## Decision

**Build `oag_render::post::motion_blur` as camera reprojection, wired exactly
where the design put the full tier, under exactly the row the design
specified.**

- **The row is the design's row**: `[graphics] motion_blur`, a `choice` of
  `off | low | medium | high`, each a shutter fraction of one tick (0, 0.25,
  0.5, 0.75). Live-applying - `Scene::render` reads it fresh each frame - so
  no `restart_required`. Replacing the technique underneath later changes no
  serialised value.
- **The placement is the design's placement**: inside `race::Scene::render`,
  after bloom (alpha is bloom's glow mask; blurring it before the composite
  would corrupt the mask), before the HUD (which the caller composites
  afterwards, so readouts stay sharp). ADR-0013's reason FXAA/SMAA cannot have
  this - they live in `Framebuffer::resolve`, after the HUD is already in the
  frame - is exactly why this pass does not live beside them.
- **The previous camera is tick-keyed**, the design's non-obvious decision,
  with one refinement the design's phrasing forces once state is actually
  held: the pass keeps the last *distinct* view-projection, expiring it when
  the tick advances without the camera moving. Frames outrunning the 60 Hz
  simulation re-render an unmoved camera, and a naive per-frame previous
  would flicker the blur off on exactly those frames; a naive held previous
  gets the opposite wrong and smears a paused race forever.
- **The depth attachment is now stored and bindable**: `StoreOp::Store` and
  `TEXTURE_BINDING` on the race's depth texture. This is the depth row of the
  FSR 3.1 prerequisite table, done.
- **Under MSAA the pass is skipped**, with a one-time log line and a
  `warn_when` on the menu row against `msaa4x`. The design's answer -
  `textureLoad` at sample 0 - was written for the velocity tier's own
  multisampled attachments and belongs to that build-out; a second shader
  variant here would be scaffolding thrown away with the gather.
- **The gather is bounded**: velocity is capped at 5 % of the viewport height,
  so a camera cut this project does not yet signal (a view switch, a respawn)
  costs one frame of bounded smear rather than a whole-frame streak.
- **Colour space**: the gather averages in the stored perceptual encoding,
  like every pass in `oag_render::post`, per
  [ADR-0020](0020-gamma-authoritative-colour-space.md)'s authority statement.
  Linear-light averaging would be the physically better lens model and is
  deliberately not smuggled in as a side effect of a blur pass.

## Alternatives considered

**Build the velocity-buffer tier now, as designed.** Rejected only as a first
step, not as a destination: the increment wanted here is the smallest one that
makes temporal reprojection real and testable, and the expensive tier's extra
week buys per-object correctness that TAA's history-validation step needs but
its reprojection step does not. The design and its costing remain the plan of
record for that tier.

**A boolean settings-file switch, no menu row** (the `bloom` /
`frustum_culling` pattern). Built first in this same change and rejected
against the design before landing: the design's strength row exists precisely
so the cheap tier can ship under it, and a boolean would be a second
serialised shape to migrate away from - the exact cost the row was designed to
avoid.

**Read multisampled depth at sample 0 so MSAA keeps the blur.** Rejected for
now: it is the right call when the velocity target forces per-sample decisions
anyway, and scaffolding here would duplicate that work in a form the gather
throws away.

**Wire the strength into `race::capture` so `--screenshot` shows it.**
Rejected as dishonest rather than as work: the capture renders exactly one
frame, so there is no previous camera and the pass could only produce the
identity. The capture passes `Off` explicitly, with a comment; a two-render
capture is recorded in the design doc's follow-ups.

## Consequences

- **A rival holding station over-blurs.** Camera reprojection computes the
  camera's travel for every pixel, including pixels showing a craft that
  barely moved on screen. The design is blunt that in this game that is not a
  corner case; it is the visible cost of the cheap tier and the standing
  argument for the velocity buffer. The strength row's `low` exists partly so
  a player who notices can keep a subtler version of the effect.
- **Additive effects blur by what is behind them.** The exhaust, trails and
  sparks write no depth, so their pixels reproject at the surface behind
  them. Same class of artefact as the above, same successor fixes it - and
  the design records that even the velocity tier only changes *which* wrong
  velocity those pixels get.
- **MSAA and motion blur exclude each other**, and only the menu row's
  warning and one log line say so at runtime.
- **One frame of bounded wrong blur on a camera cut.** Nothing signals a view
  switch or respawn to the pass yet; the cap keeps it to 5 % of the viewport
  height for one frame. `MotionBlur::reset` exists for whoever wires those
  events.
- **The temporal ledger moves**: previous-frame camera matrices exist,
  per-pixel camera velocity exists, and the depth buffer is readable. Motion
  vectors *from every draw*, sub-pixel jitter and a history buffer remain
  absent, so ADR-0013's "no `Taa` row" stands.
- **The pass is built for every non-MSAA race**, setting on or off - two small
  pipeline compiles - so the row can apply live without lazy-build plumbing
  inside a `&self` render path.
