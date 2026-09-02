# ADR-0037: Dynamic resolution varies a viewport, not an allocation

## Status

Accepted. Builds on
[ADR-0036](0036-ui-composites-at-presentation-resolution.md) and
[ADR-0038](0038-a-stage-with-no-scene-draws-at-presentation-resolution.md),
which together made the scaled target hold nothing but the race's 3D scene.
Numbered after both because 0037 was left free when 0038 landed first.

## Context

`[graphics] render_scale` already decouples the resolution a race is drawn at
from the resolution it is presented at: `upscale::target_size` turns the aspect
rectangle and the scale into a pixel size, and `Framebuffer::resize` makes the
offscreen colour target that size. `Framebuffer::resize` returning `true` is
what tells the race's depth attachment - and, under MSAA, the multisampled
attachments - to be rebuilt to match, because a depth attachment whose size does
not match the colour one is a validation error rather than a bad picture.

That is correct for a value a player moves from a menu row, which is what the
render scale is: a scale change costs one reallocation of every size-matched
attachment, and it happens when somebody nudges a slider.

Dynamic resolution scaling changes who moves the value and how often. A
controller that steps the scale to hold a frame budget moves it several times a
second, and under the present shape every one of those steps would rebuild a
colour texture, two texture views, a bind group, a depth attachment and, under
MSAA, the multisampled colour and depth attachments - spending GPU memory
bandwidth and driver time to save GPU time. Reallocating is also not free of
*stalls*: a texture the previous frame may still be reading cannot simply be
dropped, so the allocator's recycling behaviour becomes part of the frame budget
the controller is trying to hold.

There is a second problem the same shape causes. `Framebuffer` has one `size`,
and every consumer of it reads that one field for two different questions -
"how big is the texture" and "how much of it was drawn this frame". Today those
questions have the same answer, so nothing distinguishes them, and a controller
added on top would have to guess which consumer meant which.

## Decision

**The scene target is allocated once at the ceiling and the frame is drawn into
a sub-rectangle of it.** `Framebuffer` carries two sizes rather than one:

- an **allocation** - the texture's real dimensions, `target_size(rect,
  render_scale, limit)`, moved only when the window, the aspect or the
  `render_scale` *ceiling* moves;
- a **render extent** - this frame's pixels, a sub-rectangle anchored at the
  origin, always `<= allocation` on both axes, moved as often as a controller
  likes and costing nothing to move.

`Framebuffer::resize` keeps its meaning of "make the allocation this" and keeps
returning whether it reallocated; the size-matched attachments follow the
allocation and so stop being rebuilt on a scale change. A new
`Framebuffer::set_extent` moves the render extent, allocates nothing, and is the
only thing a controller ever calls.

Three properties fall out of the split and are load-bearing:

1. **The extent is what a stage is handed as its viewport.** The `inside`
   rectangle passed to every stage, the rectangle `RaceStage::warm_up` draws
   with, and therefore the rectangle
   [ADR-0039](0039-camera-jitter-post-multiplies-onto-the-view-projection.md)'s
   jitter is normalised against, are all the extent. The allocation is what
   attachment builders are handed.
2. **The blit reads only the extent.** `Framebuffer::present` scales the
   fullscreen triangle's UVs by `extent / allocation` and clamps the sample
   coordinate half a texel inside the extent's edge, or the linear sampler pulls
   in whatever was left outside the rectangle - the classic dynamic-resolution
   artefact, a bright or stale fringe down the right and bottom edges.
3. **Both are exactly `1.0` when `extent == allocation`.** The UV scale and the
   clamp are the identity in that case, by construction rather than by
   arithmetic that happens to land near one, so a build with no controller in it
   produces byte-identical frames to one without the split.

Nothing in this ADR adds a controller, a cost signal or a setting. It is the
structural precondition for all three.

## Alternatives considered

**Reallocate on every scale step.** The straightforward shape, and the one the
code already has. Rejected because the reallocation is the thing being paid to
avoid a cost, several times a second, and because it makes the controller's own
behaviour depend on the driver's allocator - the least predictable part of the
frame.

**Allocate a small set of quantized targets and switch between them.** A middle
path: a handful of textures at, say, 50/60/70/80/90/100 % of the ceiling, each
allocated once, with the controller choosing one. Rejected on memory - the set
costs more than the single ceiling-sized target it replaces - and because it
still reallocates the *depth* attachment unless that is duplicated too, at which
point it is the ceiling allocation with extra bookkeeping.

**Keep one `size` and have the controller drive `render_scale` itself.**
Rejected outright, and it is worth recording why: `render_scale` is a value in
the player's settings file. A controller writing to it would make a player's
chosen quality drift downward every time they played on a loaded machine, and
the drift would persist across sessions. The controller reads `render_scale` as
its ceiling and emits a separate runtime value; the two are different kinds of
thing and are kept in different places.

## Consequences

- **The memory cost is against a hypothetical, not against today.** The
  allocation is `target_size(rect, render_scale, limit)` - exactly what is
  allocated now - so this decision costs nothing over the status quo. What it
  costs is against a dynamic-resolution implementation that *did* shrink its
  target: at a 50 % runtime scale on a 1080p window the colour target is still
  1920x1080 rather than 960x540, and so are the depth and MSAA attachments. That
  is the price of not reallocating, and on a memory-tight target - the Steam
  Deck is named in the first tier of [goals.md](../../overview/goals.md) - it is
  the number to measure before shipping a controller. It has not been measured.
- **The clear is not viewport-restricted, so a scaled frame still pays a
  full-size clear.** wgpu has no render-area parameter: `LoadOp::Clear` clears
  the whole attachment, and `set_viewport`/`set_scissor_rect` bind draws rather
  than the load op. So the fixed cost of every scene pass stays at ceiling size
  whatever the extent is, which bounds what dynamic resolution can save at low
  scales. Measuring that bound is Phase 2's business and it belongs in the
  controller's budget, not in a surprise.
- **Two sizes is a thing to get wrong, and the failure is a validation error
  rather than a bad picture.** An extent left larger than the allocation sets a
  viewport past the attachment. The invariant is enforced where it can be -
  `set_extent` clamps, and any reallocation resets the extent to the allocation
  so a window resize cannot leave a stale larger value behind - but every new
  consumer of either size has to decide which one it means, and the type cannot
  decide for it. The audit that resolves every existing consumer is on
  [dynamic-resolution.md](../../rendering/dynamic-resolution.md).
- **The post-process and upscale passes are not viewport-aware yet, and that is
  deliberately deferred.** FXAA, SMAA and FSR 1 are handed the extent as their
  input size while reading a view of the ceiling-sized texture, which is correct
  only while the two are equal. FSR 1's port
  ([ADR-0012](0012-wgsl-upscalers-not-native-fidelityfx.md)) folded upstream's
  separate viewport and resource arguments into one because they *were* one
  here; un-folding them is a restoration of `ffx_fsr1.h`'s own parameters and
  moves the port closer to upstream, not further. Until that lands, nothing
  outside a test may set an extent below the allocation.
- **The trade is asymmetric in the controller's favour, which is the point.**
  Moving the extent is a uniform write and two `set_viewport` calls; moving the
  allocation is six texture creations. A controller can now step every frame if
  its policy says to.
