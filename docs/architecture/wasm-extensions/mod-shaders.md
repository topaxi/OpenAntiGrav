# A mod's own shaders

A mod ships fragment shaders of its own, and the engine draws them on the surfaces a mod can name.
Render side only: `oag_fx::mod_shader` (compile and refusals with no device, then the `Stage`),
`oag_raceplay::extension_shaders` (load, per-view surfaces), `Scene::attach_mod_shaders`. Everything
is an output: never hashed, never read by the simulation, absent from `oag-server`. A race with no
mod builds no stage and draws nothing extra. The example is `oag.tether` in the mod examples.

## A shader file

`[shaders]` in `oag-mod.toml` names files, `name = "fx/name.wgsl"`; `.wgsl` or `.wesl`. A module
names one by its place in **name order**, an effect file by its name.

```wgsl
//! blend = "alpha"            # or "add"; "alpha" with none
//! texture = "fx/noise.png"   # optional, read by mod_texture
//!
//! [[param]]                  # up to 4: fn param_<name>() -> f32
//! name = "speed"
//! default = 2.0

fn mod_fragment(input: ModInput) -> vec4<f32> {
    let wave = sin(input.uv.x * 20.0 - input.time * param_speed());
    return vec4<f32>(input.colour.rgb, input.colour.a * (0.5 + 0.5 * wave));
}
```

The header is the screen filter's (`oag_gpu::shader_file`, [ADR-0053](../adr/0053-screen-filters-are-loadable-wgsl-after-the-composite.md)).
The file goes in front of the engine's text as one module, so a line in naga's report is a line of
the file. **`.wesl` is read as WGSL**: `import` and conditional compilation are not available in
a file a mod ships, because linking at run time would put the `wesl` crate in every build for a
feature with no user yet. naga's report refuses an `import` by name.

### What the engine provides

```wgsl
struct ModInput {
    uv: vec2<f32>,         // 0..1 across the surface, y down
    screen_uv: vec2<f32>,  // 0..1 across the view, y down
    world: vec3<f32>,      // zero on a screen fill
    colour: vec4<f32>,     // straight alpha, gamma-authored
    time: f32,             // seconds of race time
}
fn mod_texture(uv: vec2<f32>) -> vec4<f32>      // the header's PNG, opaque white with none
fn engine_texture(uv: vec2<f32>) -> vec4<f32>   // the engine's picture for this view, gamma-authored and opaque; transparent black with none
fn engine_texture_bound() -> bool               // whether this view has one
fn engine_uv(world: vec3<f32>) -> vec2<f32>     // world to engine_texture, for a surface at the entrance
```

Both textures sample at level 0 with a repeating linear sampler, so they may be read anywhere,
inside an `if` too. `mod_fragment` returns straight alpha in gamma space, like every colour a mod
hands the engine: the engine clamps it and converts it to whatever the title's scene target holds
(`oag_shaders::target`), so one file looks alike on Pulse's 8-bit target and HD's float target.
It writes colour only: no depth, no glow mask, no velocity, as a particle does.

`uv` by surface: an effect's quad, its corners; a beam, `x` along it from `slot` to `to` and `y`
across; a fill, the view.

### Where it draws

| Surface | Named by | `colour` | When |
| --- | --- | --- | --- |
| An effect's particles | `shader = "<name>"` on an emitter of its TOML ([`mod-effect.md`](../../formats/mod-effect.md)) | the particle's | Depth tested, with the effects |
| A screen fill | `screen_fill_shaded(&craft, rgba, shader)` (`screen-fill` with `arg` the place plus one) | `rgba` | After the flash, over the world, under the HUD. The slot's view alone |
| A beam | `draw_beam(&craft, to, rgba, half_width, Some(shader))` (`draw-beam`) | `rgba` | Depth tested, with the effects. Nothing while either craft is out of the world |

`draw-beam` with `None` draws the flat colour, alpha over. `set_shader_param(&craft, shader, index,
value)` sets a scalar for the view of that craft's slot this tick; unset it is the header's default.
Every one of these is re-sent each tick it should hold.

The ribbon is `oag_fx::mod_shader::ribbon`, straight and facing the eye, not `oag_fx::beam::build`
or `leach_strip`: those carry a recovered weapon's wobble, trail and state.

## What is refused

`compile` runs on the load thread, needs no device, and never panics. A refused file keeps its
place, the loader report quotes why (naga's annotated report for a module that does not build) and
it **draws nothing**: no flat colour, no stand-in. A mod's other shaders are untouched.

- A file over 64 KiB, a bad header, more than 4 params.
- No `fn mod_fragment(`, or anything naga refuses, under no optional capability.
- **Any loop.** See below.
- A binding, an entry point or an override with no default of the mod's own: the pipeline layout
  and the entry points are the engine's, and these would fail on the device, not in naga.
- An array over 64 KiB.
- What the device still refuses at pipeline creation is caught in an error scope, logged, and the
  shader draws nothing. **Not on the web**, where the scope resolves through a promise the page's
  thread may not block on, so the browser logs it and the pipeline is an invalid one.

## A shader that hangs the GPU

A fragment shader runs for every pixel it covers, and a screen fill covers them all. A loop that
never ends holds the GPU until the driver's watchdog resets the device (`TDR` on Windows, a GPU reset
on Linux), which ends the race and can take the desktop's compositor with it.

**Covered:** every loop is refused. WGSL's `for`, `while` and `loop` all parse to one statement,
which `oag_fx::mod_shader`'s check finds in every function, and WGSL has no recursion. So a shader's
run time on one pixel is bounded by its length, which is bounded by the 64 KiB ceiling. Repeating a
step, such as the octaves of a noise, is written out or put in a helper called several times.

**Not covered:**

- **A long shader over a big surface.** The bound is on instructions a pixel, not on the frame: a
  64 KiB shader of texture reads over a 4K fill is a slideshow, possibly long enough for a watchdog
  on a weak GPU. Nothing measures a shader's cost.
- **A bounded loop is refused too.** A `for` to a constant would be safe; proving one means
  recognising the counter's shape in naga's IR and folding the bounds, and is left open.
- **A driver's own compiler** taking long or crashing on a legal shader. wgpu's validation sits in
  front of it and the mod can do nothing the engine's own text cannot.
- **Overdraw.** 512 particles of a large quad each run the shader; the live-cap is the only limit.

Mods are the player's own choice, in the trust model of
[`README.md`](README.md) ("Trust"): this stops an accident, not a determined author.

## The engine texture: `portal-view`

`portal-view{slot, entrance, exit, radius, update, generation}` (command 36, SDK
`reply.portal_view(&craft, (entrance, exit), radius, update, generation)`) asks for a second camera
on the same race: what `slot`'s camera would see through a pair of portals at two lap progresses. The
engine draws the scene from there into one reused 960x540 texture of the scene's own format, and every
shader of the race reads it. Re-sent each tick it should hold, never hashed, absent from `oag-server`.
`oag.window` in the mod examples is the example. Code: `oag_render::camera::portal` (the maths),
`oag_raceplay::portal_view` (the frames, off the road), `Scene::draw_portal_view`
(`scene/portal_view.rs`).

```wgsl
let view = engine_texture(engine_uv(input.world)).rgb;   // stays on the surface under both updates
```

`engine_uv(world)` is the camera that drew the texture, carried back through the pair, so a point of
the entrance's surface lands on the texel it covers on the screen. With `update` continuous that is
`screen_uv`; under a snapshot only `engine_uv` stays on the surface, so use it. `engine_texture` reads
what the scene target holds, turned back into gamma-authored colour (HD stores encoded linear, 2048
gamma 2), so a shader that returns it writes the scene's own value through `fs_main`, and is opaque:
the stored alpha is the bloom's glow mask. A value above 1 on a float target clamps there.

**The camera.** `virtual = exit * rotation_180 * entrance^-1 * main`, the turn about the portal's
local up, with each portal's `+z` out of the face a craft sees: back along the road at the entrance,
forward at the exit, up the spline sample's own (banking and all: the same `Race::road_frame` as
`fire-mod-effect-ahead`). A view in a vertical loop is upside down because the road is: Pulse's
`16_Track` turns over between progress 1750 and 2180. What stands behind the exit's plane is clipped with an oblique
near plane (`portal::oblique_near`, the 0..1 depth form); the frustum and the sky keep the plain
matrix, because the swing tilts the far plane and the sky rides on the eye, behind that plane.

**`update`:** `ViewUpdate::Continuous` draws every frame the portal is in the view. `ViewUpdate::Once`
draws one snapshot, the first frame it is, and again when `generation` changes. The transform,
clipping and scene are the same; only the rate differs. No draw at all, `engine_texture_bound()`
false, while the entrance's face is turned from the camera or its sphere (`radius`, whole units) is
outside the view, or for a progress off the lap. A wall in between does not count: a portal behind
one still costs its scene.

**What it costs.** One scene a frame, or one for a snapshot; a race that asks for nothing draws no
pass and builds no target (`tests::portal_view`, counted in passes). The view is a scene pass of its
own at 960x540, in a submission of its own ahead of the view that shows it, with no shadows, post
chain, jitter or PVS: both PVS sets are keyed to the craft and camera on this side of the portal, so
the frustum alone culls it and a long view draws more of the circuit than the main one. No mod
shader draws in it: that would bind the texture to its own pass, and a portal would show itself. The
scene blocks, a uniform region and a camera history are held for it in every race.

**Split screen and windows: one extra scene a frame at most, never one per seat.** The first seat of
the frame that asks and has the portal in view gets the texture. Every other view of that frame,
a later window of a multi-window race included, reads `engine_texture_bound()` as false and draws
the shader's fallback. So a portal in front of two seats shows the first the view and the second
its fallback; a shader must have one.

**The surface.** A mod's effect placed by `fire-mod-effect-ahead` holds its particle slot until it
ends (`Stage::play_held`, at most 4): a burst recycles the instance with the fewest particles, the
one quad a surface is. On Omega's grid all 32 slots were `WO_TRAIL_HITSHIP` and the surface drew on
no frame.

Evidence, 960x544, `mod_shader_frame_ground_truth`, inside the surface, mean difference of a channel
out of 255, 2026-11-04:

| | Surface, share of the view | Against no mod | Against a procedural surface | Against the reference camera | Texels moved outside it |
| --- | --- | --- | --- | --- | --- |
| Pulse PSP EU, tick 548 | 0.196 | 88.5 | 88.6 | 10.2 | 0.00000 |
| Pulse, motion blur high | 0.198 | 86.6 | 87.3 | 18.6 | 0.00000 |
| HD, tick 1022 | 0.107 | 53.5 | 71.8 | 6.1 | 0.00000 |

The reference is the same tick with no mod from `--camera-pose` at the view's logged camera: unclipped
and through the whole post chain at full size, so not zero. It proves the texture is that camera's
picture, and a wrong camera agrees with itself there: that the camera is right is
`tests::portal_view::the_view_stands_behind_the_exit_and_looks_down_the_road`, against the road's own
numbers.

## Builds

- **Web:** the library checks for `wasm32-unknown-unknown`, `portal-view` included
  (`cargo check --target wasm32-unknown-unknown -p oag-game --no-default-features --features web`);
  inactive, as every module is there, and not run in a browser. [`web.md`](../../tools/web.md),
  "What is missing".
- **Android:** `oag-mobile`, `oag-fx` and `oag-gpu` check for `aarch64-linux-android`. Modules there
  are as the rest of the app; not run on a device.
- **2048 and Omega: checked, applies, not wired separately.** The stage is title-blind and
  `oag.tether` runs on both: a frame with the mod against one without, 960x544, 2026-11-04, share of
  texels that differ, the view's middle and a corner: 2048 (tick 800) 0.098 and 0.93, Omega
  (tick 700) 0.58 and 0.97. Omega's middle is too high to be the beam alone and the cause was not
  looked for. Neither has a test of its own: `mod_shader_frame_ground_truth` holds Pulse and HD.
