# ADR-0038: A stage with no scene draws entirely at presentation resolution

## Status

Accepted. **Supersedes [ADR-0036](0036-ui-composites-at-presentation-resolution.md)'s
seam item** for stages that have no 3D scene in them. Everything else in
ADR-0036 stands: the presentation target, the grade after the composite, the
HUD inside it and the performance overlay outside it.

## Context

ADR-0036 put the seam between *authored-grid* and *sampled* content: quads,
glyphs and paletted sprites composite late at presentation size, while "a movie
is sampled picture content and stays on the scene side, where an upscaler is
the right thing to have carried it."

That sentence was written from the race's shape, where the scene side holds a
rendered 3D frame and the movie would be one more piece of picture beside it.
It was not checked against the front end. It is now, and the front end's scene
side holds **nothing but the movie quad**:

- `LauncherStage` and `LoadingStage` draw no movie at all - one UI list each,
  plus the loading screen's additive wave pass.
- `FrontendStage` and `MenuStage` each draw **one** list containing their UI
  *and* a `Draw::Video`, through a single `Renderer::render` call.

So honouring the seam there would mean splitting one draw list into two passes
whose only content on the scene side is a full-screen movie - and then paying an
upscale to carry it. At a render scale of 50 % that is 480x272 stretched to half
the rectangle and resampled up again: two resamples where the video shader's own
sampler already does it in one, and stretching to a *smaller* intermediate first
can only lose.

## Decision

**A stage with no 3D scene draws its whole list straight into the presentation
target, and `Framebuffer::resolve_scene` is skipped for that frame.**

- The frame is `stage.render(framebuffer.output(), rect)` then
  `Framebuffer::composite`. `Renderer::render` clears with `LoadOp::Clear`, and
  with the viewport set to the aspect rectangle on a surface-sized target that
  clear draws the letterbox bars exactly where `present` drew them.
- **The movie rides along at presentation size.** One resample rather than two.
- The race is unchanged: `resolve_scene` → `draw_hud` → `composite`.
- `RaceStage::warm_up` keeps drawing into the *scene* target. It warms pipelines,
  which are keyed on format and not on the size of the attachment they first
  meet.

## Alternatives considered

**Split the draw list at the video, per ADR-0036's seam.** Rejected on cost
against benefit. A `Renderer` uploads its quads and uniforms through
`queue.write_buffer`, and a queue write lands before *any* of the submission's
commands - so two passes off one `Renderer` both draw whatever the second
uploaded. `race/capture.rs` records this hazard for its primer frame, and a test
written for ADR-0036 walked into it. Splitting therefore needs a second
`Renderer` per stage: two pipelines and two font-atlas textures each, spent to
make a menu backdrop blurrier than it has to be.

**Keep the whole front end in the scaled target.** That is the arrangement
ADR-0036 exists to end.

**Give the front end a scene side by moving only the movie into the scaled
target, sharing one `Renderer` with an offset-based quad buffer.** Rejected as
the same picture for more machinery: it removes the two-renderer cost by adding
a per-frame write cursor to `Renderer`, and still pays the extra resample.

## Consequences

- **`[graphics] render_scale` no longer affects the front end at all.** This is
  the honest cost and it is also the correct behaviour - nothing in a menu is
  GPU-expensive, so scaling it saved nothing. But a player moving that row
  *inside a menu* now sees nothing change on the screen they are standing on,
  where before the menu itself visibly resampled. The row has no `warn_when`
  and arguably wants one; that is left open rather than guessed at, because the
  existing vocabulary (`disabled_by`, `warn_when`) says "this row does nothing
  *given another row*", and this is "this row does nothing *on this screen*",
  which is a fourth kind of thing to say to a player.
- **Above 100 % the UI is no longer supersampled**, for the menus as it already
  was for the HUD. Native is the right answer for glyphs off a coverage atlas
  and for paletted sprites, but at 125-200 % this is a visible change and not
  purely an improvement: a supersampled-then-downsampled glyph has softer edges
  that some will prefer.
- **The offscreen scene target is allocated and idle while the front end is
  up.** It is not resized away, because the render scale it is sized from can
  change while the menus are open and a race parked in `Session::suspended_race`
  still expects it.
- **No upscaler, and no FXAA or SMAA, will ever touch UI again.** The HUD
  composites after `resolve_scene` (ADR-0036) and the whole front end skips it,
  so the only thing those passes now see is the race's 3D scene. That clears
  the *a scene without UI in it* row in
  [modern-features.md](../../overview/modern-features.md) outright.
- **It also settles half of an open question rather than deferring it.** The
  doubt over whether `[graphics] upscaler` should default to `fsr1` was that
  "the menus and the HUD are 480x272-era paletted raster and glyphs off a
  coverage atlas, and a sharpener rings on those". Neither ever reaches a
  sharpener now, so that half is answered structurally and needs no comparison.
  What is left of the doubt is scene content alone.
- **The front-end capture path needs no `Framebuffer` for correctness.**
  `capture::run` already draws its list into the capture texture at the aspect
  rectangle, which *is* presentation resolution - so it now agrees with the
  window by construction. It would still need one to honour `--presented`'s
  grade and bars, but that is a capture-fidelity want, not the blocker it was
  recorded as.
