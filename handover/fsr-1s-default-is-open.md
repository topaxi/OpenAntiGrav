# FSR 1's default is open

`oag_render::post::fsr1` transliterates AMD's MIT `ffx_fsr1.h`; the route for FSR 3.1 is settled in [ADR-0012](../docs/architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md). It cannot reach the front end because `capture::run`'s front-end path has no `Framebuffer`, and giving it one is the same work as the UI-compositing restructure ([modern features](../docs/overview/modern-features.md) has the prerequisite table). **Do not** read a menu capture taken with `--upscaler` as evidence either way: the flag changes the UPSCALER row's own text, so the two images differ for an unrelated reason - that nearly produced a false conclusion.

## Phase 0, shared: the UI composites at presentation resolution

**This section is duplicated, on purpose, in
[Dynamic resolution wants a viewport, not an allocation](dynamic-resolution-wants-a-viewport-not-an-allocation.md) - whoever picks up either thread does this
first, and it is the same piece of work both times.** Duplicated rather than
linked because a thread file is deleted the moment its work lands: if the other
file is gone, this work is very likely already done, and
[modern-features.md](../docs/overview/modern-features.md)'s prerequisite table
is the thing to check before starting rather than the missing file.
When it lands, delete this section from whichever thread is still open.

**Decided, and most of it built.**
[ADR-0036](../docs/architecture/adr/0036-ui-composites-at-presentation-resolution.md)
settles what [ADR-0013](../docs/architecture/adr/0013-anti-aliasing-architecture.md)
explicitly declined to decide: the scene resolves first, the UI draws after it,
at presentation size. Read it before touching the rest.

**What is built.** `Framebuffer` now holds a presentation-sized target between
the upscale and the surface, and `Framebuffer::resolve` is gone, split in two:

- `resolve_scene` - FXAA/SMAA, FSR 1 and the blit, into that target,
  **ungraded**.
- `composite` - the target onto the surface, **graded**, last.

The UI draws between them. Two elements have moved across:

- **The performance overlay**, onto the surface *after* `composite`, so it is
  deliberately outside the grade - an instrument, not picture content. ADR-0036's
  one standing exception.
- **The HUD and the scoreboard**, into the presentation target *before*
  `composite`, so they are sharp at any render scale, no upscaler ever sees
  them, and they are still inside brightness and gamma. `RaceStage::render` is
  the scene alone now; `RaceStage::draw_hud` is the other half, called from
  `Session::frame` and from `race/capture.rs`'s `--presented` path.

Two tests in `crates/game/src/upscale/tests.rs` pin it:
`the_ui_composites_over_the_blit_at_presentation_resolution` for the geometry
and `the_hud_is_graded_with_the_scene_and_the_performance_overlay_is_not` for
the grade, the exception, and double-grading. Both were checked against the old
arrangement rather than assumed - drawn into the scaled target, a
one-surface-pixel fill covers no sample centre and vanishes from the frame
completely.

**Correcting ADR-0036 while it is fresh:** its consequence "every race
`--screenshot` changes bytes when the HUD moves" is **wrong**. Only
`--presented` does. The plain capture path builds no `Framebuffer` and runs no
resolve, so its HUD was already at native size and is untouched. The ADR is
immutable and stays as written; this is the correction of record.

**Roughly half a day left**, and it is one slice:

- **The menus, the front end and the loading screen.** They still draw into the
  offscreen target from their stages, so they are still rasterised at the render
  scale. The front end is the awkward one: it draws its movie and its UI through
  one `Renderer::render` over one draw list, with the movie a `Draw::Video`
  entry in it, and ADR-0036 puts a movie on the *scene* side of the seam - so
  this means splitting that call.
- **`crates/game/src/capture.rs`'s front-end path still has no `Framebuffer`
  at all**, which is the whole reason FSR 1 has never reached the front end.
  `race/capture.rs` is done and is not a template for it: that one had a
  `Framebuffer` already.

What it delivers, in the words `docs/overview/modern-features.md` uses: the
prerequisite table's last row, *a scene without UI in it*, stops being
**Absent**. It has **not** stopped yet - the HUD is out of the scaled target
but the menus and the front end are not, and FSR 3.1 needs all of it. The row
moves in the change that moves them. Update it there.

## Open

- FSR1 cannot reach the front end because `capture::run`'s front-end path has no `Framebuffer`
- A menu capture taken with `--upscaler` is not valid evidence either way (the flag itself changes the UPSCALER row's text)
- Whether the default should move to `fsr1` at all. It has been compared once, at 50 % on one frame of one track, and FSR 1 won clearly - but the doubt is about content that frame did not contain: the menus and the HUD are 480x272-era paletted raster and glyphs off a coverage atlas, and a sharpener rings on those. The comparison that settles it is only possible *after* Phase 0, because before it there is no menu frame an upscaler has ever touched.

## Next Steps

- Finish Phase 0 above. What is left is the menus and the front end, and giving `capture::run`'s front-end path a `Framebuffer` - which is this thread's own blocker, not incidental to it. Shared with [dynamic-resolution-wants-a-viewport-not-an-allocation.md](dynamic-resolution-wants-a-viewport-not-an-allocation.md).
- Then re-run the default comparison on menu and HUD content, which is the sample the default flip is actually held to.
