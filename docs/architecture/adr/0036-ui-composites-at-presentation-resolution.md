# ADR-0036: The UI composites at presentation resolution

## Status

Accepted.

**Partially implemented.** The performance overlay moved in the change that
landed this ADR; the HUD, the scoreboard, the menus, the front end and the
loading screen have not. The prerequisite row this decision exists to clear -
*a scene without UI in it*, in
[`docs/overview/modern-features.md`](../../overview/modern-features.md) - stays
**Absent** until they do.

This does not supersede [ADR-0013](0013-anti-aliasing-architecture.md). That
ADR chose which anti-aliasing techniques exist and where they sit relative to
the upscaler, and both of those are unchanged. It also *recorded* the UI's
position in the pipeline while explicitly declining to decide it - "Note this
puts HUD compositing *before* FXAA/SMAA, not after - the reverse of what a
from-scratch design would choose. It falls out of the existing render-scale
architecture, not a decision made by this ADR." This is that decision, made.

## Context

Every stage draws into one offscreen `upscale::Framebuffer` sized by
`[graphics] render_scale` against the aspect rectangle, and
`Framebuffer::resolve` then runs FXAA or SMAA, FSR 1, the grade, and the blit
onto the surface. The UI is inside that target when `resolve` runs: the HUD and
scoreboard from `RaceStage::render`, the menus and front-end screens from their
stages' own `Renderer::render`, and the performance overlay from
`Session::frame`.

That was a deliberate reading for a *fixed* render scale - `perf.rs` states it
as "the overlay should cost what the game costs, or it is measuring a frame
that does not exist" - and three separate pieces of work are now blocked on it,
all of them naming the same row:

- **FSR 3.1** must be handed a frame with no UI in it. A temporal upscaler
  reconstructs from history and motion vectors, and UI has neither.
- **FSR 1's default** cannot be judged on the content the doubt is actually
  about. The menus and the HUD are 480x272-era paletted raster and glyphs off a
  coverage atlas, which is exactly what a sharpener rings on - and the front
  end has never been through an upscaler at all, because `capture::run`'s
  front-end path has no `Framebuffer` (`crates/game/src/capture.rs`).
- **Dynamic resolution** makes the scale a per-frame quantity. A static 50 % is
  a choice a player sees once; a scale stepping several times a second makes
  coverage-atlas glyphs crawl continuously, and lands that on the one element
  being read mid-race.

`Upscaler::Off`'s own doc comment already recorded the doubt for glyph and
paletted-sprite content, and ADR-0013 noted FXAA and SMAA inherit it. Nothing
about the arrangement was wrong; what changed is that three downstream things
now need the seam that it does not have.

## Decision

**The scene resolves onto the surface first, and the UI draws after it, into
the surface, at presentation size.**

- **The authored grid scales to the surface, not to the offscreen target.**
  Everything this project draws lays out on the PSP's 480x272 grid
  (`crate::display`), mapped into `display::viewport(surface, aspect)`. That is
  the *same rectangle on screen* either way - the aspect bars are unmoved, and
  nothing changes position - so this is one substitution of which pixels a
  glyph is rasterised into, not a layout change.
- **The seam is authored-grid content versus sampled content.** Quads, glyphs
  and paletted sprites composite late, at presentation size. A movie is sampled
  picture content and stays on the scene side, where an upscaler is the right
  thing to have carried it. The front end draws both through one
  `Renderer::render` call over one draw list, with the movie as a `Draw::Video`
  entry in it, so honouring this seam there means splitting that call - which
  is why the front end is not in the first slice.
- **The grade moves after the composite when the UI does.** Grading is one pass
  for the whole picture, per `crate::upscale`'s standing argument that three
  stages grading themselves would be three places to get it wrong and three to
  forget when a fourth lands - and a player calibrating brightness has to see
  the menu they are standing on respond. So the end state is: `resolve`
  upscales into a presentation-sized target, the UI composites into it, and a
  final grade-and-blit pass writes the surface. The grade does not stay in
  `resolve`, and it does not move into each UI shader.
- **The performance overlay is the exception, permanently: it is outside the
  grade.** It is an instrument, not picture content, and a frame-time graph
  that a brightness of 20 % makes unreadable is a worse instrument. It is also
  the reason the overlay is the first slice - it is the one UI element whose
  final position needs no presentation target to be correct.
- **Capture paths are part of the decision, not a follow-up.** The window path
  and both capture paths have to composite in the same order or a
  `--screenshot` stops being a picture of what the window draws, which is
  `crate::capture`'s stated reason for existing. The two are differently
  shaped - `race/capture.rs` builds a `Framebuffer` conditionally on
  `--presented` and calls `upscale::target_size` itself; `capture.rs`'s
  front-end path has no `Framebuffer` at all - so a fix to one is not a fix to
  the other.

## Alternatives considered

**Leave it as it is and let FSR 3.1 consume the composited frame.** Rejected:
it is not a quality tradeoff but a correctness one. Reprojecting UI by the
scene's motion vectors smears it across the frame, and the history buffer then
holds a ghost of a HUD that has since changed.

**Grade inside each UI shader instead of moving the grade pass.** Rejected on
the argument `crate::upscale` already makes for why grading lives in one place.
It is cheaper - no presentation-sized target, no extra fullscreen pass - and it
is three or four shaders that have to agree, forever, with a fifth that lands
later.

**Keep the grade in `resolve` and accept an ungraded UI.** Rejected for
everything except the performance overlay. Brightness and gamma are a
calibration of the whole picture; a menu that does not respond to the row being
moved is the specific behaviour `crate::upscale`'s module doc defends, and
calibrating against a picture whose UI is exempt is calibrating against the
wrong picture.

**Draw the UI at presentation size but still into the offscreen target.**
Rejected as not a thing: the offscreen target *is* the render resolution. There
is nowhere presentation-sized to draw into before `resolve` runs.

## Consequences

- **A presentation-sized colour target is added**, once the UI moves: one more
  RGBA8 at surface size (about 8 MB at 1080p, 33 MB at 4K), plus one more
  fullscreen pass per frame for the grade. Stated rather than buried - it is
  memory paid on every machine, including the Steam Deck that
  [goals.md](../../overview/goals.md) names in the first tier, for a seam only
  three features need.
- **`perf.rs`'s "the overlay should cost what the game costs" is overturned.**
  It was right for a fixed scale and is wrong for a moving one: the overlay is
  the row a player uses to judge what a resolution controller is doing, and an
  overlay that resamples along with the scene cannot be read while it moves.
- **At a non-neutral brightness or gamma the overlay no longer matches the
  picture behind it.** That is the visible cost of the exception above, and it
  is the correct trade for an instrument.
- **The overlay is no longer touched by FXAA, SMAA or FSR 1.** Its glyphs stop
  being edge-detected and sharpened as though they were scene content, which
  ADR-0013 flagged as inherited doubt rather than as intent. Its cost also
  leaves the scene's budget, which is what a GPU-timestamp signal needs if it
  is to measure the part that scales.
- **`RaceStage::render` bundles the scene and the HUD in one call**, and
  `RaceStage::warm_up` is a second caller of it. Splitting the HUD out means
  both callers change, and the warm-up must still touch the HUD pipelines or it
  stops warming what the first real frame needs.
- **Every race `--screenshot` changes bytes when the HUD moves.**
  `RaceStage::render` records that the HUD "lands in a `--screenshot` too"
  precisely because it shares the target. This project verifies with
  byte-identical screenshot diffs, so that slice invalidates every stored race
  capture at once and should land on its own, with the diff explained rather
  than discovered.
- **ADR-0013's pipeline diagram no longer shows where the UI is.** That ADR is
  immutable and its own subject is unchanged; this page is where the UI's
  position is now recorded.
