# Motion blur

> **Status, 2026-08-25: the tier this page chose is built.** The per-object
> velocity buffer, the `prev_mvp` uniform, the tile-max / neighbour-max /
> reconstruction chain and the always-on buffer are implemented as designed
> - `oag_render::post::motion_blur`, `mesh.wgsl`'s `velocity_of`,
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
`crates/game/src/display.rs` and ADR-0013 both refuse: *a row for infrastructure
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

- *Inside `Scene::render`, not `Framebuffer::resolve`.* The HUD and the perf
  overlay are drawn into the same target immediately after the scene, so a pass
  sitting alongside FXAA and SMAA in `oag_game::upscale` would smear them.
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

**Attachments**, in `oag_game::race`. ~~The depth texture gains
`TEXTURE_BINDING` and the pass's `depth_ops.store` becomes `StoreOp::Store`~~ -
**done**, by the camera tier, which reads that depth every frame. Add
an `Rg16Float` velocity target at the scene's sample count, `RENDER_ATTACHMENT |
TEXTURE_BINDING`, resized alongside depth, cleared to zero each frame with no
`resolve_target`. The race pass gains a second colour attachment.

**Shader**, in `oag_render::mesh_render` and `mesh.wgsl`. `Uniforms` gains one
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
`crates/render/tests/msaa_resolve.rs`; only `oag_game::race` wants it on - and a
missing velocity write is invisible in the final image, so the call sites need
to say which they mean rather than passing a bare `false`.

**Previous transforms.** `Scene` caches last tick's view-projection and
per-ship model matrices, updated only when `race.world.tick` changes.
`Scene::render` takes `&self`, but the struct already holds its exhaust and
spark pipelines in a `RefCell`, so this needs no signature change. Static
geometry keeps passing identity; the sky passes last tick's camera translation,
which cancels against last tick's view-projection and correctly leaves it with
rotation-only velocity.

**Post chain**, as `oag_render::post::motion_blur`, following the shape every
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

**The setting** follows `AntiAliasing` exactly, because it is the fullest
worked example in the tree: an enum in `oag_game::display` with `name()`,
`ALL`, `FromStr`, `Display` and the `TryFrom<String>` / `Into<String>` serde
pair; a field on `Graphics`; a tuple in `menu_seeds`; a `kind = "choice"` row in
`assets/ui/menu.toml` whose `values` are exactly `ALL`, in order; an arm in
`Session::apply_setting`; a per-frame argument on `Scene::render` next to
`anim_seconds`; and a field on `race::CaptureOptions` so `--screenshot`
reflects it, the way `bloom` and `anti_aliasing` already do.

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

[`MAX_STRETCH`]: ../../crates/render/src/post/motion_blur.rs

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
with a `warn_when` on the menu row, for which the anti-aliasing row's existing
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
- The `UNIFORMS_SIZE` pin in `oag_game::race` fails the moment `Uniforms`
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
