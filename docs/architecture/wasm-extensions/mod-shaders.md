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
fn engine_texture(uv: vec2<f32>) -> vec4<f32>   // what the engine binds; transparent black with nothing
fn engine_texture_bound() -> bool               // whether it has
fn engine_uv(world: vec3<f32>) -> vec2<f32>     // world to engine_texture, through its camera
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

## The engine texture

The slot is for a view the engine draws for a mod, a portal first. `Stage::set_engine_texture(device,
Some((view, view_projection)))` binds any filterable colour texture to every shader's slot,
`Stage::set_engine_view_projection` moves its camera per frame. A shader samples it at
`screen_uv`, or at `engine_uv(input.world)` to project through that camera. Until something binds
one, `engine_texture_bound()` is false and the slot is a 1x1 transparent black, so a shader can
draw its own fallback.

## Builds

- **Web:** the library checks for `wasm32-unknown-unknown`; inactive, as every module is there, and
  not run in a browser. [`web.md`](../../tools/web.md), "What is missing".
- **Android:** `oag-fx` and `oag-gpu` check for `aarch64-linux-android`. Modules there are as the
  rest of the app; not run on a device.
- **2048 and Omega: checked, applies, not wired separately.** The stage is title-blind and
  `oag.tether` runs on both: a frame with the mod against one without, 960x544, 2026-11-04, share of
  texels that differ, the view's middle and a corner: 2048 (tick 800) 0.098 and 0.93, Omega
  (tick 700) 0.58 and 0.97. Omega's middle is too high to be the beam alone and the cause was not
  looked for. Neither has a test of its own: `mod_shader_frame_ground_truth` holds Pulse and HD.
