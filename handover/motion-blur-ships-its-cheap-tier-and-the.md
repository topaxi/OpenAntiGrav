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

Verified: 8 unit/device tests in `crates/render/src/post/motion_blur/tests.rs`
including a real-GPU test pinning that a purely horizontal camera shift smears
horizontally, spends energy, and leaks nothing vertically. **Not verified: the
effect in a real race** - the capture path honestly cannot show it (one
render), and this environment opens no windows.

## Open

- **No picture of the effect on a real track exists yet.** `just play --race
  ... --screenshot` goes through `race::capture`, which ticks N times and
  renders once - no previous camera, identity blur, and that is why the
  capture passes `Off` explicitly.
- A camera cut (view switch, respawn) is not signalled to the pass; it costs
  one frame of smear bounded by the 5%-of-viewport-height cap.
  `MotionBlur::reset` exists and nothing calls it.
- MSAA 4x excludes the blur (multisampled depth); the menu row `warn_when`s
  and the scene logs once.
- The gather averages in perceptual space like every `post` pass (ADR-0020);
  a linear-light gather was deliberately not smuggled in.

## Next Steps

- **The two-render capture**: teach `race::capture` to pose and render the
  tick-before-last camera into the target first, then the final tick - then
  wire `CaptureOptions.motion_blur` through and take the first real
  screenshot of the effect (fast section of any track, `medium`). This is
  also the verification tool a tier comparison needs.
- Wire `MotionBlur::reset` to the camera-cut events: `Session`'s view cycle
  (`graphics.camera_view` apply arm and the SELECT handler) and the respawn.
- The velocity-buffer tier, when its week-and-a-half is worth spending -
  [docs/rendering/motion-blur.md](../docs/rendering/motion-blur.md) is the
  plan of record, its `Rg16Float`-at-4x probe still the first step, and it
  owes its own ADR (ADR-0028's Alternatives says which decisions are left).
