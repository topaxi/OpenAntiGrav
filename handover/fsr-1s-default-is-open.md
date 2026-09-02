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

**Decided, and one slice of it built.**
[ADR-0036](../docs/architecture/adr/0036-ui-composites-at-presentation-resolution.md)
settles what [ADR-0013](../docs/architecture/adr/0013-anti-aliasing-architecture.md)
explicitly declined to decide: the scene resolves onto the surface first, and
the UI draws after it, into the surface, at presentation size. Read it before
touching any of the rest - it records three consequences neither thread had,
and one of them reorders the remaining work.

**The performance overlay has moved** (`crates/game/src/main/session/frame.rs`):
it draws after `Framebuffer::resolve`, onto the surface, laid out against the
aspect rectangle rather than the offscreen extent. It is the one element whose
final position needs no new plumbing, because it is deliberately outside the
grade - an instrument, not picture content.
`the_ui_composites_over_the_blit_at_presentation_resolution` in
`crates/game/src/upscale/tests.rs` pins both halves of the property, and it was
checked against the old order rather than assumed: drawn before the resolve, a
one-surface-pixel fill covers no sample centre in the scaled viewport and
disappears from the frame completely.

**Roughly a day and a half left**, and the order below is the ADR's, not the
cheapest-first one:

- **The grade, first.** It rides inside `Framebuffer::resolve`, so anything
  composited after that pass is ungraded. That is right for the overlay and
  wrong for the menus: a brightness row a player cannot see working on the menu
  they are standing on is the exact behaviour `crate::upscale`'s module doc
  defends. So nothing else can move until `resolve` upscales into a
  presentation-sized target and a final grade-and-blit pass writes the surface
  after the composite. That target is the first code to write, not the last -
  and it costs one RGBA8 at surface size plus a fullscreen pass, stated in the
  ADR rather than discovered here.
- **The HUD and the scoreboard.** `RaceStage::render` draws the scene and then
  one of the two into the same target, and `RaceStage::warm_up` is a second
  caller of it that still has to warm the HUD's pipelines. **This slice changes
  bytes in every race `--screenshot`**, which this project verifies with
  byte-identical diffs - land it alone, with the diff explained rather than
  discovered.
- **The menus, the front end and the loading screen.** The front end draws its
  movie and its UI through one `Renderer::render` over one draw list, the movie
  being a `Draw::Video` entry in it. ADR-0036 puts a movie on the scene side of
  the seam, so this slice means splitting that call.
- **Both capture paths, which are not the same shape.**
  `crates/game/src/race/capture.rs` builds a `Framebuffer` conditionally on
  `--presented` and calls `upscale::target_size` itself;
  `crates/game/src/capture.rs`'s front-end path has no `Framebuffer` at all,
  which is the whole reason FSR 1 has never reached the front end. A fix to one
  is not a fix to the other.

**The 480x272 authored grid** (`display.rs`) is what every overlay lays out in,
and compositing later means scaling that grid to the *surface* rather than to
the offscreen target - one substitution, in more places than it looks. It is
the same rectangle on screen either way, so nothing moves; only which pixels a
glyph is rasterised into changes.

What it delivers, in the words `docs/overview/modern-features.md` uses: the
prerequisite table's last row, *a scene without UI in it*, stops being
**Absent**. It has **not** stopped yet - the overlay slice does not clear that
row, and it moves in the change that moves the HUD and the menus. Update it
there.

## Open

- FSR1 cannot reach the front end because `capture::run`'s front-end path has no `Framebuffer`
- A menu capture taken with `--upscaler` is not valid evidence either way (the flag itself changes the UPSCALER row's text)
- Whether the default should move to `fsr1` at all. It has been compared once, at 50 % on one frame of one track, and FSR 1 won clearly - but the doubt is about content that frame did not contain: the menus and the HUD are 480x272-era paletted raster and glyphs off a coverage atlas, and a sharpener rings on those. The comparison that settles it is only possible *after* Phase 0, because before it there is no menu frame an upscaler has ever touched.

## Next Steps

- Finish Phase 0 above, starting with the presentation-sized target and the grade pass after the composite - nothing else can move until the grade can follow the UI. It is the prerequisite for everything else here, and it is shared with [dynamic-resolution-wants-a-viewport-not-an-allocation.md](dynamic-resolution-wants-a-viewport-not-an-allocation.md).
- Then re-run the default comparison on menu and HUD content, which is the sample the default flip is actually held to.
