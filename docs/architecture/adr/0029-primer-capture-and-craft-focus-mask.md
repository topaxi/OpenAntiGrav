# ADR-0029: A capture renders a primer frame, and the craft are masked out of the camera blur

## Status

Accepted. Supersedes three items of
[ADR-0028](0028-camera-motion-blur-first.md): the capture passing `Off`, the
5 % stretch cap, and rival over-blur being accepted as-is.

## Context

ADR-0028 shipped camera-reprojection motion blur and was verified only by a
synthetic device test - no picture of the effect on a real track existed,
because `race::capture` renders exactly one frame and the blur is the delta
between two distinct cameras. The first real-race session then produced two
observations, both predicted by
[the design page](../../rendering/motion-blur.md) and both worth acting on
inside the cheap tier rather than waiting for the velocity buffer:

- **The player's craft blurred.** It is the reference object - near-still on
  screen - but reprojection assigns its pixels the *world's* velocity at its
  depth, which is large because it is close.
- **The world read as barely blurring while nearby craft streaked.** Both
  halves are camera-reprojection physics: forward motion's optical flow is
  near zero at the vanishing point the player looks at, grows toward the
  screen edges - where the 5 % stretch cap was truncating it - and scales
  with 1/depth, which is exactly why a *close* rival gets a large computed
  velocity. The perceived effect was inverted: sharp world, smeared rivals.

## Decision

**Three changes, none of which waits for per-object velocity:**

- **`race::capture` holds the last tick back and renders a primer frame
  first** - the scene at the tick-before-last camera, drawn into the same
  target and wholly overwritten. Camera blur carries no pixels across frames,
  so the primer's only product is the pass observing that camera; the real
  frame then shows the same smear a player sees, and `--motion-blur`
  (overriding `[graphics] motion_blur` for one run) makes A/B captures one
  flag apart. The primer submits in its own encoder: two renders in one
  submission would interleave their `write_buffer` uploads, since queue
  writes land before the submission's commands.
- **Every drawn craft becomes a focus sphere the blur skips.** The craft are
  the one class of moving object - the objects reprojection is *wrong* about
  - whose bounds the CPU already knows: centre, hull radius and depth
  project into `[centre uv, radius px, far-side NDC depth]` per slot, and
  the shader zeroes the gather inside, with a soft ring. The depth bound is
  what keeps the road blurring right up to a craft's silhouette, and the
  player's sphere is skipped in the cockpit view along with its hull.
- **The stretch cap rises from 5 % to 8 % of viewport height, with thirteen
  taps instead of nine.** The cap exists to bound a camera cut, not to style
  the picture, and at 5 % it was truncating the legitimate near-field
  streaks that carry the sense of speed; more taps keep the longer reach
  from visibly stepping.

## Alternatives considered

**Wait for the velocity-buffer tier, which fixes craft blur exactly.**
Rejected for the same reason ADR-0028 shipped the cheap tier at all: the
mask is an afternoon against a week and a half, uses only data already on
the CPU, and the velocity tier replaces it wholesale rather than fighting
it. The mask is also honest about what it is - a list of known moving
objects - where a velocity buffer *measures* motion.

**Mask by depth alone (skip blur on anything close).** Rejected: the road
directly under the camera is as close as the craft and is the strongest
speed cue in the frame; a depth cutoff would sharpen exactly the pixels
that should streak hardest.

**A `--blur-frames N` knob on the capture.** Rejected: camera blur needs
exactly one previous camera, so N buys nothing today. When TAA's history
buffer needs real warm-up frames, that is its own change with its own
reason.

## Consequences

- A capture with the blur on simulates the same tick count but renders
  twice; every other capture is unchanged, and the deferred tick reuses the
  loop's own input logic (`advance_one_tick`) so script and pulse inputs
  cannot drift between the two paths.
- The mask is a sphere, so a craft yawed side-on sharpens a disc somewhat
  wider than its hull; the soft ring makes the excess read as focus
  falloff, not a cutout.
- A weapon or debris is not masked and still takes the world's blur;
  rockets are fast enough that the smear reads as intended anyway.
- A camera cut can now smear up to 8 % of the frame height for its one
  frame, up from 5 % - still bounded, and the reset wiring ADR-0028 left
  open remains open.
- The first real screenshots of the effect exist and are reproducible:
  `--race --autopilot --ticks 900 --motion-blur off|medium|high --screenshot`
  produces identical telemetry with only the smear differing.
