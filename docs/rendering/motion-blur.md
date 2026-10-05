# Motion blur

> **Status, 2026-08-25: the tier this page chose is built.** The per-object
> velocity buffer, the `prev_mvp` uniform, the tile-max / neighbour-max /
> reconstruction chain and the always-on buffer are implemented as designed
> - `oag_post::motion_blur`, `mesh.wgsl`'s `velocity_of`,
> `mesh_render::Velocity` - under exactly the strength row and at the
> placement this page specified. It happened in the two steps the page
> sanctioned: camera reprojection shipped first as the stepping stone
> ([ADR-0028](../architecture/adr/0028-camera-motion-blur-first.md), plus
> [ADR-0029](../architecture/adr/0029-primer-capture-and-craft-focus-mask.md)'s
> live-feedback patches), and the velocity tier replaced it under the same
> row with no settings migration
> ([ADR-0030](../architecture/adr/0030-velocity-buffer-motion-blur.md) - the
> ADR this page said the tier owes). The predicted ghosting-on-rivals was
> observed live in the interim, which is as validated as a design prediction
> gets. What remains below as design rather than implementation: the
> airbrake flaps' own swing velocity, and this page's read on what the
> buffer deliberately does not solve.

**This is an invented feature, not a recovered one.** Wipeout Pulse has no
motion blur; nothing on this page came out of a disassembler, and nothing on it
carries a [confidence score](../reverse-engineering/confidence-rubric.md),
because a score would imply it was measured against the original. It belongs to
the [modern features](../overview/modern-features.md) track, alongside
ultrawide, HDR and FSR-class upscaling, and it is subject to the same rule those
are: a setting exists only when the thing behind it exists.

The file and line references below were read against the tree at the time of
writing. They are there so an implementer can start from this page instead of
re-deriving the renderer's shape, and they will drift; treat them as
signposts, not as addresses.

## Why this is worth more than it looks

Two tiers were considered, and the cheap one is genuinely tempting.

**Camera reprojection** samples the depth buffer, reconstructs each pixel's
world position, reprojects it through the previous camera and blurs along the
resulting screen-space vector. It needs no change to any mesh pipeline, and on a
racing track it is not an approximation: the track ribbon, walls, scenery and
sky do not move, so reprojecting them as static geometry is *exact*. It costs
about two days.

**A per-object velocity buffer** writes the real screen-space motion of every
draw into a second render target, then runs the published tile-max /
neighbour-max / reconstruction chain over it. It costs about a week and a half.

The entire visible difference between them is the eight ships, and the failure
mode is counter-intuitive enough to be worth stating plainly:

> Camera reprojection does not fail to blur a rival. It **over**-blurs one.
> A rival holding station beside you barely moves on screen and should be sharp,
> but the camera has travelled a long way that tick, so reprojection computes a
> large velocity for it and streaks it.

In this game that is not a corner case. A race is largely spent alongside
someone.

The second reason is the one that decides it. [Modern
features](../overview/modern-features.md) keeps a table of what FSR 3.1 still
needs, and the same list appears in [the rendering
scope](README.md) and in
[ADR-0013](../architecture/adr/0013-anti-aliasing-architecture.md) as the reason
there is no `Taa` row. A velocity buffer moves two of its five rows:

| Prerequisite | Today | With this built |
| --- | --- | --- |
| Render resolution decoupled from presentation | Done | Done |
| A depth buffer the upscaler can consume | `StoreOp::Discard`, no `TEXTURE_BINDING` | **Done** - this needs both |
| Per-pixel motion vectors from every draw | Absent | **Done** - this is that buffer |
| Camera jitter, sub-pixel per frame | Absent | Absent, unchanged |
| A scene without UI in it | Absent | Partially - see below |

**That table is the argument as it stood when this was designed, and the last
row has moved since.** The UI-free scene is **Done** as of 2026-09-02, by
[ADR-0036](../architecture/adr/0036-ui-composites-at-presentation-resolution.md)
and [ADR-0038](../architecture/adr/0038-a-stage-with-no-scene-draws-at-presentation-resolution.md)
rather than by anything here - the scene now resolves into its own presentation
target and the UI composites over it afterwards, so the seam this design only
*demonstrated* is now exposed. Camera jitter followed on the same day
([ADR-0039](../architecture/adr/0039-camera-jitter-post-multiplies-onto-the-view-projection.md)),
and it is this design's `prev_mvp` that constrained where the offset could go:
the same phase has to reach the previous tick's matrix, or the jitter lands in
every motion vector this buffer writes.

That last row deserves precision rather than a tick. This design inserts its
chain *inside* `race::Scene::render`, before the HUD is drawn, so at that point
in the frame a UI-free scene demonstrably exists. What it does not do is hand
that scene downstream as a separate artifact, which is what FSR 3.1 actually
needs. The seam is shown to exist; it is not yet exposed.

Camera reprojection moves only the depth row, and its reconstructed velocity is
**not** a substitute for the motion-vector row: reprojection ghosts on exactly
the moving objects temporal reconstruction has the most trouble with.

Note also that [frame generation is deliberately out of
scope](../overview/modern-features.md), so nothing here should be justified by
it.

## Decisions, and why

**The velocity buffer is always written in the game path.** The setting gates
only the blur passes, not the buffer. This keeps the menu row live-applying -
no `restart_required`, so the deliberate deferred-row tripwire in
`crates/game/src/menu.rs` stays untouched - and it leaves the velocity target
unconditionally available to FSR 3.1, which is half the reason for choosing the
expensive tier. The cost is one `Rg16Float` write per fragment.

**The setting is strength, not technique.** `off | low | medium | high`, mapping
to a shutter fraction of the tick. Offering `camera` and `full` as user-facing
values would ship a value with no implementation behind it, which
`crates/display/src/display.rs` and ADR-0013 both refuse: *a row for infrastructure
that is not there is worse than no row*. It also means that if the cheap tier is
ever shipped first as a stepping stone, it can be replaced underneath the same
row without a settings migration.

**Blur length keys off `race.world.tick`, not the frame.** This is the
non-obvious one. The simulation is fixed 60 Hz
([ADR-0007](../architecture/adr/0007-fixed-timestep-vs-original.md)) and there
is no render-side interpolation - `TickClock::interpolation_alpha` exists in
`crates/core/src/tick.rs` and has no callers outside its own tests. So above 60
fps, consecutive frames render *identical* simulation state. Caching the
previous transforms per frame would therefore produce zero blur on a duplicate
frame and full blur on the next: a visible amplitude pulse that gets worse the
faster the machine is. Caching per tick makes the blur a constant 1/60 s of
motion at any frame rate.

> **When state interpolation lands** - [modern
> features](../overview/modern-features.md) plans it for unlocked frame rate -
> this must be revisited. The previous transform should then be the previous
> *interpolated* camera and the shutter a fraction of the real frame delta.
> Tick-keying is correct only while every frame within a tick is identical.

**The chain runs after bloom, inside `Scene::render`.** Both halves of that are
load-bearing and both look wrong at a glance:

- *Inside `Scene::render`, not `Framebuffer::resolve_scene`.* This was written
  when the HUD and the perf overlay were drawn into the same target immediately
  after the scene, so a pass sitting alongside FXAA and SMAA in
  `oag_present::upscale` would have smeared them. Since
  [ADR-0036](../architecture/adr/0036-ui-composites-at-presentation-resolution.md)
  neither is in that target - both composite at presentation resolution - so
  the smearing argument no longer holds. The placement stands on the *other*
  reason it always had: the blur wants the scene's own depth and velocity
  attachments, which exist inside `Scene::render` and nowhere downstream of it.
- *After bloom, not before.* Physically, motion blur belongs before bloom.
  Here it must not: alpha is bloom's glow mask, stamped by the track ribbon and
  the exhaust flare, and blurring alpha corrupts the mask. The clear at the top
  of the race pass is `a: 0.0` for exactly this reason. Bloom composites
  additively into the scene view and the blur then reads the composited result.

**Neither extra attachment is resolved under MSAA.** With `msaa4x` the depth and
velocity textures are `sample_count: 4`; both are read with `textureLoad` at
sample 0. Averaging velocity across a silhouette edge produces a vector that
describes neither surface, and reading sample 0 sidesteps the question of
whether `Rg16Float` is resolvable at all. One decision covers both attachments.

**The reconstruction filter is written from the published description**, not
transliterated - McGuire et al., *A Reconstruction Filter for Plausible Motion
Blur* (I3D 2012). This is the rule
[ADR-0012](../architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md)
sets and the way FXAA was already done.

## Implementation sketch

**Attachments**, in `oag_raceplay`. ~~The depth texture gains
`TEXTURE_BINDING` and the pass's `depth_ops.store` becomes `StoreOp::Store`~~ -
**done**, by the camera tier, which reads that depth every frame. Add
an `Rg16Float` velocity target at the scene's sample count, `RENDER_ATTACHMENT |
TEXTURE_BINDING`, resized alongside depth, cleared to zero each frame with no
`resolve_target`. The race pass gains a second colour attachment.

**Shader**, in `oag_mesh::mesh_render` and `mesh.wgsl`. `Uniforms` gains one
premultiplied `prev_mvp`; `VertexOutput` gains current and previous clip
position; add a second fragment entry point returning `@location(0)` colour and
`@location(1)` velocity as the NDC delta, halved and y-flipped into UV space.

> `prev_mvp` is deliberately **not** symmetric with the existing
> `view_projection` / `model` pair. Those are separate because fog and lighting
> need world position; velocity needs only clip position. Say so in the doc
> comment, or the next reader restores the symmetry and adds 64 bytes to every
> draw for nothing.

**Pipelines.** `mesh_render::build` gains a `Velocity::None | Velocity::Write`
argument - an enum rather than a `bool`, matching how `Depth::Scene` /
`Depth::Sky` already reads in the same function. Two of its three callers want
it off - the asset viewer's `crates/view/src/orbit.rs` and
`crates/render/tests/msaa_resolve.rs`; only `oag_raceplay` wants it on - and a
missing velocity write is invisible in the final image, so the call sites need
to say which they mean rather than passing a bare `false`.

**Previous transforms.** `Scene` caches last tick's view-projection and
per-ship model matrices, updated only when `race.world.tick` changes.
`Scene::render` takes `&self`, but the struct already holds its exhaust and
spark pipelines in a `RefCell`, so this needs no signature change. Static
geometry keeps passing identity; the sky passes last tick's camera translation,
which cancels against last tick's view-projection and correctly leaves it with
rotation-only velocity.

**Post chain**, as `oag_post::motion_blur`, following the shape every
other pass in that module already has: `new(device, format) -> Result<Self>`,
targets built lazily at the first `render` because that is the first time a size
is known, `output()`, uniforms written only when they change. Four passes -
tile-max, neighbour-max, reconstruction, and a blit back into the scene view
(the pass cannot read and write one texture, and `Scene::render` holds only a
`TextureView`, so this is a blit rather than a copy). Build it lazily and treat
a build failure the way bloom does: report once, carry on. A race without motion
blur is a sharper race, not a broken one.

> **Weight the taps by depth.** Without it the background drags across
> foreground silhouettes, which is the single difference between an effect that
> reads as motion blur and one that reads as a bug.

> As built the chain is **six passes and five targets**, not four:
> `prepare` folds velocity and depth together so the rest of the chain reads
> one binding whatever the scene's sample count, and tile-max runs separably,
> one axis at a time.
> [ADR-0030](../architecture/adr/0030-velocity-buffer-motion-blur.md) counts
> five and four, which is what landed with it; the separable split came
> after. ADRs are immutable, so this line and
> `oag_post::motion_blur`'s module docs carry the current count.
>
> The warning above was earned twice. The first tier had no per-object
> velocity at all, which is the whole of ADR-0028 and ADR-0029. The second
> got the depth weighting right and then lost half of it in the *velocity*
> weighting: the gather's third term - the paper's product of two cylinders,
> which credits two surfaces for sharing a motion - was written against the
> tile neighbourhood's dominant reach instead of the centre pixel's own. That
> reach is `>= own_reach` by construction, so the substitution could only add
> weight, and the term is the one with no depth gate: a **still** surface in
> front of a fast one took a full-weight tap of whatever was behind it,
> measuring 199 of 255 at a test block's silhouette. Fixed to
> `min(tap_reach, own_reach)`, pinned by
> `a_still_surface_over_a_moving_background_keeps_its_colour`.

### Cost, measured

The number ADR-0030 says belongs here once measured on real hardware. Intel
Arc (ARL) integrated graphics, 1920x1080, `strength 0.5`, the whole chain
timed over 50 submissions:

| | ms/frame |
| --- | --- |
| Square tile-max, as ADR-0030 landed | 2.6 |
| Separable tile-max | **1.2** |
| Separable, with the reduction stubbed out entirely | 1.1 |

The square form was not doing more arithmetic - both forms touch every pixel
once - it was doing it in `ceil(w / tile) * ceil(h / tile)` fragments, which
at 1080p is 299 threads each running 7,569 serial `textureLoad`s. Splitting
the reduction into a horizontal pass at `ceil(w / tile)` by `h` and a
vertical one onto the tile grid puts the first pass's work in 24,840
fragments instead, and what remains of tile-max is now within noise of free.
This is worth knowing generally: **the reach cap doubles as the tile size, so
a generous cap makes the tiles large**, and a square reduction over a large
tile is the one shape where a fullscreen pass can starve the GPU.

#### The extent, not the allocation

Reported from play on a **Steam Deck**, 2026-09-04, with dynamic resolution
on and `motion_blur = high`: the overlay's `BLUR` row read **4-5 ms**, in the
same ballpark as the FSR 3.1 chain beside it. That is the first reading of
this chain on the first-tier target, and it is not the same machine or the
same resolution as the table above.

Part of it was work on pixels nobody sees. Since
[ADR-0037](../architecture/adr/0037-dynamic-resolution-varies-a-viewport-not-an-allocation.md)
the scene target is allocated at the render-scale ceiling and the controller
draws into a *sub-rectangle* of it, and every pass here was a fullscreen
triangle over the whole target - so the chain paid the ceiling's price at
every extent. It was invisible in the picture, which is why it survived:
`fs_reconstruct` already returned the centre tap unchanged outside the
rectangle and `fs_copy` samples a texel centre exactly, so the two rasters
produce byte-identical scenes. Each pass now sets a viewport - the tile
reductions onto the tile grid their own rectangle reduces to - and what is
left outside is the clear alone.

Two consequences worth carrying:

- **`drs::Cost` counts this chain as `scalable`**, "what falls when the
  render extent does". Until the raster fell with the extent, that was a
  claim the code did not honour, and a controller that lowers the resolution
  chasing a cost that will not move is
  [ADR-0042](../architecture/adr/0042-the-dynamic-resolution-budget-subtracts-what-it-can-measure.md)'s
  own failure mode.
- **A load operation has no sub-rectangle.** The four intermediates a later
  pass reads outside its own rectangle are still cleared whole, which is what
  keeps them zero rather than stale; the copy pass onto the caller's scene
  must *not* clear, and clearing it wiped everything outside the rectangle the
  moment the raster stopped covering it.

A second consequence of the same asymmetry, found beside it: **the tile edge
is `MAX_STRETCH` of the extent, so it moves on every controller step**, and
`resize` rebuilt all five scratch targets whenever it did - two of them
allocation-sized, for a reason that has nothing to do with either. Only the
three tile targets depend on the edge now, and
`a_moving_tile_edge_does_not_reallocate_the_full_size_targets` pins that. The
seven bind groups are still rebuilt on a step, which is the shape the
`Groups` cache exists to keep off the *per-frame* path and is left there.

The rest is the gather itself: up to 15 taps, two fetches each, on a frame
where nearly everything moves - the `dominant_len <= 0.5` early-out that
carries a still scene almost never fires in a race. Measured 2026-10-03 on HD
on an integrated GPU (Ryzen 7900's Raphael, `OAG_RENDER_GPU_BENCH` with
`--autopilot`, `high`), the gather was 14.6 of the chain's 15.9 ms at
1600x900 and the chain 59 ms at 3200x1800, the reach cap being a fraction of
the extent so the same taps stride four times the texels. Two changes, both
picture changes judged from the captures:

- **The tap count follows the smear.** One tap per 4 px of the
  neighbourhood's reach, odd, between 5 and 15 (`TAP_SPACING_PX`,
  `MIN_TAPS`): a short smear sampled fifteen times re-read the same texels.
  The chain 15.4 -> 11.1 ms at 1600x900, 59.3 -> 49.9 at 3200x1800, for a
  mean change of 0.7/255 over the frame.
- **BLUR RESOLUTION `half`** (`display::BlurResolution`, per render profile,
  `full` by default) gathers at half size each way and blends the smear back
  over the full-size frame - premultiplied by the share of the result the
  taps rather than the centre contributed, and upsampled by depth as well as
  distance so a still edge keeps its own side. The chain 4.6 ms at 1600x900
  and 19.3 at 3200x1800. The smear reads grainier, which the maintainer
  judged too much to make it the default: it is the setting for hardware
  that needs it. Two shapes failed on the way and are pinned by the tests: a
  *difference* from the half-size centre left a two-pixel block unsmeared
  (`the_half_resolution_gather_smears_a_two_pixel_block`), and a plain
  bilinear upsample bled the background across a still silhouette, 201 of
  255 (`a_still_surface_over_a_moving_background_keeps_its_colour`, now run
  at both resolutions).

**The setting** follows `AntiAliasing` exactly, because it is the fullest
worked example in the tree: an enum in `oag_display::display` with `name()`,
`ALL`, `FromStr`, `Display` and the `TryFrom<String>` / `Into<String>` serde
pair; a field on `Graphics`; a tuple in `menu_seeds`; a `kind = "choice"` row in
`assets/ui/menu.toml` whose `values` are exactly `ALL`, in order; an arm in
`Session::apply_setting`; a per-frame argument on `Scene::render` next to
`anim_seconds`; and a field on `race::CaptureOptions` so `--screenshot`
reflects it, the way `bloom` and `msaa` already do.

> All of that exists now (`display::MotionBlur`), the `CaptureOptions` field
> included: `race::capture` holds the last tick back, renders a discarded
> **primer** frame at the tick-before-last camera, then the real one - so a
> `--screenshot --motion-blur medium` shows the same smear a player sees,
> and the tier comparison this page wants has its tool. `--motion-blur`
> overrides the settings file for one run, like `--anti-aliasing`. See
> [ADR-0029](../architecture/adr/0029-primer-capture-and-craft-focus-mask.md),
> which also records the shipped tier's **craft focus mask**: the first
> real-race session confirmed this page's prediction that camera
> reprojection smears the craft (the player's own included), and the cheap
> counter is to project every drawn craft's bounding sphere and have the
> gather skip those pixels, depth-tested so the road still blurs up to each
> silhouette. The velocity tier replaces the mask with measurement.

### The tile lattice, and the jitter that hides it

Reported from play, 2026-08-31: with motion blur on, steering through a turn
shows faint **squares near the centre of the screen**. They are the tile grid.

Every pixel of a tile reads the same dominant velocity, so wherever the
scene's motion ramps across the screen - the road ahead of a craft, most of
the frame in a turn - the gather's span changes in steps rather than
smoothly. The reach cap doubles as the tile edge, so those steps are `8 %` of
the viewport height apart: **87 px at 1080p**, and the lattice is anchored to
the window rather than to the world, so the scene flows through a stationary
grid. That is what makes a small error legible - the eye locks onto
stationary structure in a moving field.

Measured by driving `post::motion_blur`'s own chain on a synthetic frame:
velocity purely vertical, its magnitude ramping along x from zero to past the
cap, depth flat, and the colour a 40-row bright block, so the smear above the
block reads the gather out directly. At 1920x1080:

| | peak phase of the profile's \|2nd difference\|, mod 87 | ratio over median |
| --- | --- | --- |
| Nearest tile lookup, `strength 0.5` | **0** - the tile edge | 2.33 |
| Nearest tile lookup, `strength 0.75` | **0** | 2.59 |
| Jittered lookup, `strength 0.5` | 84 | 1.48 |
| Jittered lookup, `strength 0.75` | 84 | 1.43 |

Before the jitter, every tile boundary from x=174 to x=1305 stepped the smear
profile up by 0.9-2.4 levels in a single pixel column - 5-13 % of its own
level, 3 to 10 times the local trend, and **always the same sign**, which is
neighbour-max raising the dominant as the lookup crosses over. After, the
mean tile-edge step is 0.8 times the local trend with mixed signs: gone.
A control period of 44 px shows no phase preference either way (1.29-1.35),
which is what says the 87 px result was the lattice and not the measure.

Confirmed in play the same day: the squares are gone, and what is left is a
faint dither along a smear's edge that is not visible in motion, only on a
frozen frame. That is the trade landing exactly where the comment says it
would.

Two things the measurement does *not* say. The steps vanish at both ends of the ramp -
below half a pixel of dominant motion the pass returns the pixel untouched,
and past the cap every tile clamps to the same value - so the blocking only
lives in the band between, which is why it reads as local rather than as a
full-screen grid. And it was never reproduced in a capture of a real race:
at the ~85 km/h a headless `--race --hold cross` reaches, the whole effect is
0.6 of 255 and the lattice does not rise above the scene's own content.

The trade the jitter makes is in
[`motion_blur.wgsl`](../../crates/post/src/motion_blur.wgsl)'s own
comment: a smear's boundary becomes stochastic where it was hard. It cannot
under-reach - the tile the lookup lands on is at most one away, and that
tile's neighbour-max already covers the pixel's own tile.

**The other two suspects, ruled out rather than assumed.** The centre tap's
`TAPS / max(own_reach, 0.5)` weight follows the smooth per-pixel reach, so it
can bend the ramp but cannot step it. And `SOFT_Z` compares nonlinear 0..1
depths against a constant, which *is* dimensionally wrong - the world
distance it tolerates grows with the square of the distance - but it
contributes nothing here: the same velocity ramp run against a flat depth and
against a perspective road (z 4..3000, near 0.1, far 5000) gives profiles
that differ by 3 levels out of 198, and a constant velocity over that road
depth gives no structure at all. Worth fixing on its own terms; not worth
blaming for this.

### Grain, and what replaced the tile wobble (2026-10-05)

Reported from play: "the motion blur feels quite grainy and not smooth".
Measured, not assumed; all of it *chosen, not measured off the original*, as
the effect is this project's own.

**Cause.** Two per-pixel `sin`-hash noises, both spatially white, both a pure
function of the pixel (no frame index), so the pattern is *static in screen
space* while the scene moves under it. (1) the +/-half-tile wobble of the tile
lookup that hid the lattice above: neighbouring pixels landed on different
tiles, so they gathered along different spans, which is noise in the *value*
of the smear and not only in where the taps fall; (2) the tap-offset dither.

**Method.** Pulse, `--race --autopilot --motion-blur high --size 1280x720
--ticks 400` and 401, headless. Reference: the same frame with wobble and
dither off and 63 taps at 1 px spacing. Metric: RMS of the 5x5 high-passed
difference (variant minus reference), over the pixels blur visibly changed
(55,693 of them, HUD rows excluded), plus the frame-to-frame correlation of
that residual between ticks 400 and 401.

| variant | hp-RMS vs reference (0-255) | frame-to-frame corr |
| --- | --- | --- |
| as shipped (wobble + dither, 15 taps) | **4.12** | 0.28 |
| wobble off | 3.24 | 0.08 |
| dither off | 3.96 | 0.23 |
| both off (15 taps, 4 px) | 2.98 | -0.09 |
| **bilinear tile lookup, dither kept** | **3.44** | 0.13 |
| bilinear tile lookup, dither off | 3.08 | 0.06 |
| half-resolution gather | 4.81 | 0.21 |

Attribution, in quadrature over the 2.98 floor (the 15-tap sampling itself):
the wobble is about 2.6, the dither about 1.3, so **the wobble was roughly
80 % of the grain energy in a real frame**. Half resolution adds about 0.7
on top of full. Raising the cap to 63 taps changes nothing here, because the
tap count follows the reach (`TAP_SPACING_PX`) and the cap is not what limits
it at 1280x720.

**What landed: a bilinear tile lookup.** The gather reads the neighbour-max
of the four nearest tile centres and interpolates, so the span is a continuous
function of the pixel: no lattice and no per-pixel noise. Pulse 1280x720,
4.12 -> 3.44 hp-RMS (77 % of the way to wobble-off's 3.24; the
remainder is the dither and the smoother span itself differing from the
nearest-lookup reference), and HD shows the same on the right-hand wall.
GPU cost, `OAG_RENDER_GPU_BENCH=150` at 1600x900 on a discrete GPU, `high`:
the chain 441 -> 427 us (four tile loads in place of two hashes and one; no
measurable cost). The synthetic ramp of the section above is now a test,
`lattice_tests::the_tile_lookup_leaves_neither_a_lattice_nor_grain_on_a_ramp`:
peak/median of the per-phase mean `|2nd difference|`, 1.62 / 1.58 at
`strength` 0.5 / 0.75 (the hash wobble measured 1.68 / 1.47; a nearest lookup
1.98 at phase 86, the tile edge, and the test fails there). Cap interpretation:
at a hard edge of a fast object the blended span now ramps over a tile width
where the nearest lookup stepped; the two neighbour-max tiles still bound it.
Opposing velocities in adjacent tiles cancel in the lerp; not seen in a
race frame, noted.

**Tried, and not worth it.** Interleaved gradient noise for the dither in
place of the `sin` hash: 3.48 vs 3.44, no change in hp-RMS or in a 3x3
low-passed residual (0.96-1.04 across hash, half amplitude and a 4x4 Bayer).
Halving the dither amplitude: 3.20, but it trades grain for banding at a
hard edge (the dither-off synthetic ramp: grain 0.08 but 2.18x peak/median
from tap staircase, against 6.1 grain with dither on). Tap spacing 3 px
(21 taps): 3.16 for roughly 1.4x the gather.

**What is left, and why it is not landed.** The dither is the remaining
grain (about 1.3 of the 3.44) and on the synthetic block-on-black ramp it is
the whole of it (mean 6 levels per pixel against 0.08 without). It can only
be removed by more taps (cost), a temporal resolve (the FSR 3 history could
accumulate it; the dither would need a frame index in the uniform) or a
post-filter on the blurred region. Ranked by quality over cost: temporal
accumulation of an animated dither > tap spacing 3 px > a depth-aware 3x3
filter on the blurred region only > full-res gather as the default. See the
handover thread `motion-blur-grain`.

## What the velocity buffer does not solve

Both of these read as solved once an MRT target exists. Neither is.

**Airbrake flaps.** `Drawable::deflect_airbrakes` builds a rigid transform from
the flap angle and applies it to the model's rest vertices on the CPU, so the
flaps move in *model* space, before the ship's own matrix, and a per-draw
`prev_model` knows nothing about them. The good news is that because the
deformation is a rigid transform, the previous flap position is analytically
recoverable from the previous angle - two `f32` per ship, not a duplicated
vertex buffer. The awkward part is that the flaps are a vertex sub-range inside
the ship's single draw, so they need two extra draws carrying their own
`prev_vp * model * prev_swing`.

**Additive particles.** The exhaust flare, its trail and the collision sparks
are camera-facing quads rebuilt from scratch every draw, with no frame-to-frame
vertex correspondence at all - the recovered behaviour is explicit that [there
is no history buffer
here](../ghidra/functions/psp-pulse-usa/exhaust.md). There is no previous
position to compute a velocity from, and no amount of MRT invents one. Give both
pipelines the second target with an empty write mask so those pixels keep the
velocity of whatever is behind them.

Be honest in the code comment about what that actually does, because it is not
the same as excluding them. Those pipelines are depth-write-off, so both the
velocity *and* the depth at a particle pixel belong to the surface behind it.
Reconstruction will therefore blur the particle along the background's velocity
while the depth weighting treats it as foreground - the very artifact that
weighting exists to prevent. This is accepted rather than fixed: the geometry is
additive, low contrast and already bloomed, so it should be imperceptible. It is
listed below as a thing to check, not to assume.

**Camera cuts.** A view switch or a respawn moves the camera further in one
tick than any surface really travelled, and every pixel measures that jump.
The reach cap bounds it to one frame of at most [`MAX_STRETCH`] of the
viewport height, which is why nothing has had to be wired yet - but nothing
*is* wired, and the pass no longer holds the state a reset would clear.
[ADR-0028](../architecture/adr/0028-camera-motion-blur-first.md) left a
`MotionBlur::reset` for whoever wired those events; the velocity tier is
stateless across frames, so that method is gone and the cache to invalidate
is `race::scene::frame::MotionState` instead. Whoever wires a cut clears the
snapshot there, and the frame after it measures zero.

**The impact shake (solved 2026-10-01).** The shake turns the view by up to six
degrees in one tick, and a velocity buffer built from the shaken views reads
that as the world flying past: the first frame of a hard hit smeared whole,
which the original does not (seen against PPSSPP, 2026-09-30). **The velocity
buffer stays true screen motion**, because the temporal upscaler reprojects its
history with the same attachment and that history has to follow the shake; it
is the blur's prepare pass that takes the shake out. A rotation about the eye
moves a pixel by an amount that depends on where it is on screen and not on how
deep it is, so one clip-space matrix describes the whole frame:
`projection * previous * current^-1 * projection^-1`, with `current` and
`previous` the view-space shake rotations of this and the last tick
(`Race::view_shake_rotation`, `Race::shake_screen_motion`; the previous one
rides in `MotionState`'s snapshot). It reaches the pass as
`motion_blur::Frame::camera_shake` and `fs_prepare` subtracts the pixel's own
shake motion from the velocity it reads; identity, with no shake in either
tick, subtracts exactly nothing. A first attempt that built the velocity from
the unshaken camera instead was dropped because it also removed the shake from
what the upscaler sees, for the whole 0.6 s decay.

Evidence: `--force-shake 120:1.0 --ticks 121` on the grid, a still craft, so
the blur has nothing to blur and a frame with it on must equal the frame with
it off. Before, the two differ by RMSE 0.07 (the whole frame smeared); after,
byte-identical, at 480x272 and 1440x816, severity 0.3 and 1.0, ticks 120 to
134. Pinned by `crates/game/tests/shake_blur_ground_truth.rs` (disc-backed,
fails if `Scene::render` drops the map), `the_blur_is_given_the_shakes_own_screen_motion`
(the matrices, from a cold shake and mid-decay) and `post::motion_blur::shake_tests`
(the shader, on a device).

[`MAX_STRETCH`]: ../../crates/post/src/motion_blur.rs

## Settle these before writing any shader

**Is `Rg16Float` multisample-renderable at 4x here?** Reading sample 0 avoids
the question of *resolvability*, but not whether the format can be a
multisampled attachment at all. Probe
`adapter.get_texture_format_features(Rg16Float)` for `MULTISAMPLE_X4` first.
This project already has a monument to assuming otherwise:
`crates/render/tests/msaa_resolve.rs` pins why there is no `Msaa2x` row,
`sample_count: 2` having failed device validation on every adapter here rather
than only some. Note that it is a live tripwire, not a historical record - it
also pins the condition under which 2x could come back, and asserts that if
*that* check ever fails, `Msaa4x` needs the same treatment. Read it before
adding a format assumption of your own. Multisampled `Rg16Float` is far more
ordinary than sample-count-2, so the probe will very likely pass - but
discovering otherwise after the shader is written is expensive. Fallbacks: `Rgba16Float`, universally
multisample-renderable at four more bytes per pixel; or gate the combination
with a `warn_when` on the menu row, for which the reconstruction row's existing
warning against the upscaler is exact precedent.

**Does bloom render correctly today?** This design inherits bloom's placement
and its colour-space handling, and "bloom works, copy it" is weaker evidence
than it sounds: `[graphics] bloom` defaults off and has no menu row, so it is
not exercised by anyone playing normally. Its pipelines are built with the sRGB
suffix removed while the view it writes is the framebuffer's sRGB view. Set it
on, take one screenshot, confirm it before the blit-back inherits the same
arrangement.

**Is the particle artifact above actually imperceptible?** Capture a boost plume
against fast-moving track and look specifically for the plume smearing along the
wrong axis. If it shows, suppressing blur where velocity and depth disagree is
the fix, and it is not budgeted below.

## Cost

About a week and a half.

| Days | Work |
| --- | --- |
| 1-2 | Attachments, `Uniforms`, the second entry point, the `build` argument, three callers |
| 3 | Previous transforms, airbrake flaps, particle write masks |
| 4-6 | Tile-max, neighbour-max, reconstruction, blit-back |
| 7 | The setting, the menu row, the settings and menu test pins |
| 7.5 | The velocity readback test - it needs a headless wgpu harness, though `msaa_resolve.rs` gives the pattern |
| 8-9 | ADR-0024, these docs, the gate, screenshot verification |

**The shader is not the long pole.** The velocity target touching every pipeline
that draws into the race pass is, and so is this repository's own overhead: an
ADR, and four tests that will fail until the option is wired end to end.

Those tests, so they are not a surprise:

- The settings pages' rows must equal their type's `ALL`, as an ordered
  sequence.
- Every setting-bearing row must appear in `menu_seeds`, and every seed must
  name a key the settings file actually has. Together these catch a half-wired
  option, which is their whole purpose.
- The `UNIFORMS_SIZE` pin in `oag_raceplay` fails the moment `Uniforms`
  grows. That is intended: it is what makes the two mirrored definitions stay
  mirrored.

Worth adding a fifth: draw one moving quad, read the velocity target back and
assert the sign and rough magnitude. A y-flip in the velocity encoding is the
most likely single bug and it is invisible in the final image.

## The owed ADRs are written

This section used to say "ADR-0024 is owed" (four ADRs landed in between).
Three now exist:
[ADR-0028](../architecture/adr/0028-camera-motion-blur-first.md) for the
stepping-stone camera tier,
[ADR-0029](../architecture/adr/0029-primer-capture-and-craft-focus-mask.md)
for the primer capture and the interim focus mask, and
[ADR-0030](../architecture/adr/0030-velocity-buffer-motion-blur.md) for this
page's own tier - the always-on buffer, the sample-0 MSAA reads, the settled
`Rg16Float` question (multisample-renderable at 4x, pinned by a live test
with `Rgba16Float` as the fallback), and both unsolved cases above.

## Ghosting: measured, not reproduced (2026-10-05)

The maintainer reported the bilinear tile lookup ("Grain, and what replaced the
tile wobble") sometimes reads as ghosting. `motion_blur/ghost_tests.rs` builds
scenes whose swept path is known (a still block over a fast background,
opposing movers, perpendicular movers) and measures the energy the chain puts
outside that path: 0.0003, 0.0000 and 0.0000 of the scene's energy.

Those numbers are identical on the pre-grain shader (`bd229d257^`), because
every region has one uniform velocity and the tile lookup then returns the same
value either way. So the scenes guard against off-path leakage but do not
isolate the lookup; the ghosting the maintainer sees needs a velocity
*gradient* scene, not yet built. The line profile also shows gaps inside the
swept band at v=0.1 (discrete tap copies) on both shaders - a tap-count
effect, not the grain change.

Tests that allocate frames: `lattice_tests::frames` once shadowed its `count`
parameter with a byte count and rendered 8 million frames (51 GB resident);
run new `oag-post` tests under `systemd-run ... -p MemoryMax=8G`.
