# Motion blur ships its cheap tier, and the temporal ledger has one row down

Camera-reprojection motion blur is built (`oag_render::post::motion_blur`),
behind the live `[graphics] motion_blur` strength row `off | low | medium |
high` that [docs/rendering/motion-blur.md](../docs/rendering/motion-blur.md)
designed for the *expensive* tier - the "cheap tier first, same row" path the
design sanctioned.
[ADR-0028](../docs/architecture/adr/0028-camera-motion-blur-first.md) records
the step. Read the design page before touching any of this: it pre-answers
most questions, and the shipped tier deliberately diverges from it in only two
places (skip under MSAA rather than sample-0 reads; `race::capture` passes
`Off` because one render has no previous camera).

What this bought TSAA/TAA beyond the effect itself: previous-frame camera
matrices with a tick-keyed update rule (`Camera::observe` - repeats of an
unmoved simulation keep the held previous, a tick with an unmoved camera
expires it), depth-based per-pixel reprojection in WGSL, and a race depth
buffer that is now `StoreOp::Store` + `TEXTURE_BINDING`. Still absent for a
`Taa` row, per ADR-0013: per-draw motion vectors, sub-pixel jitter, a history
buffer.

Verified: 9 unit/device tests in `crates/render/src/post/motion_blur/tests.rs`
- the real-GPU test pins that a horizontal camera shift smears horizontally,
spends energy, leaks nothing vertically, and leaves a focus-masked block
untouched - **and real-track captures now exist**, taken through the primer
capture below (`--race --autopilot --ticks 900 --motion-blur off|medium|high
--screenshot ...`: identical telemetry, only the smear differs).

The first live session confirmed the design page's predicted artifacts and
[ADR-0029](../docs/architecture/adr/0029-primer-capture-and-craft-focus-mask.md)
records the responses: `race::capture` defers its last tick and renders a
discarded primer frame so a screenshot shows the real smear (`--motion-blur`
overrides the settings file for one run); every drawn craft is masked out of
the gather as a depth-tested focus sphere (the craft are what camera
reprojection is wrong about, and their bounds are already on the CPU); and
the stretch cap rose to 8 % with 13 taps, because 5 % was truncating the
near-field streaks that read as speed.

## Open

- A camera cut (view switch, respawn) is not signalled to the pass; it costs
  one frame of smear bounded by the 8%-of-viewport-height cap.
  `MotionBlur::reset` exists and nothing calls it.
- MSAA 4x excludes the blur (multisampled depth); the menu row `warn_when`s
  and the scene logs once.
- The gather averages in perceptual space like every `post` pass (ADR-0020);
  a linear-light gather was deliberately not smuggled in.
- The focus mask is a sphere: a side-on craft sharpens a disc wider than its
  hull, read as focus falloff through the soft ring. Weapons and debris are
  not masked. Both are accepted until the velocity tier measures real
  motion.

## Next Steps

- Wire `MotionBlur::reset` to the camera-cut events: `Session`'s view cycle
  (`graphics.camera_view` apply arm and the SELECT handler) and the respawn.
- The velocity-buffer tier, when its week-and-a-half is worth spending -
  [docs/rendering/motion-blur.md](../docs/rendering/motion-blur.md) is the
  plan of record, its `Rg16Float`-at-4x probe still the first step, and it
  owes its own ADR (ADR-0028's Alternatives says which decisions are left).
  It replaces the focus mask with measured per-draw motion.
