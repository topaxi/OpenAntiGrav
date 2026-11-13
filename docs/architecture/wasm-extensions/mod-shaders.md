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
//! fill = "over"              # or "backdrop": where a screen fill through it draws; "over" with none
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
    between: f32,          // how far short of this tick the frame is drawn, ticks: -1..0
}
fn mod_texture(uv: vec2<f32>) -> vec4<f32>      // the header's PNG, opaque white with none
fn engine_texture(uv: vec2<f32>) -> vec4<f32>   // the engine's picture for this view, gamma-authored and opaque; transparent black with none
fn engine_texture_bound() -> bool               // whether this view has one
fn engine_uv(world: vec3<f32>) -> vec2<f32>     // world to engine_texture, for a surface at the entrance
fn craft_frame() -> mat4x4<f32>                 // the view's own craft in eye space: right, up, back, place
fn eye_ray(screen_uv: vec2<f32>) -> vec3<f32>   // the ray through that point of the view, eye space, z -1
```

`craft_frame` and `eye_ray` are what a shader needs to cut geometry of its own round the craft:
eye space has the eye at the origin looking down `-z`, so a point `d` units ahead through a texel
is `eye_ray(input.screen_uv) * d`, and the craft's frame is rigid (no scale, lean or shake). For a
craft in a transit the frame is the line it is flown along ("A craft's own transit" below).
`fx/wormhole.wgsl` of `oag.slipstream` draws a tunnel round the craft with the two.

`time` runs with the frames. A scalar a module sets is stepped once a tick, so one that moves
(a count of ticks, a radius) is read as `param_x() + rate * input.between`, `rate` being what it
gains a tick: `between` is the draw alpha less one, `0` on the tick itself and in a `--screenshot`,
and the frame between two ticks then shows the value between them, as the camera and the crafts
are drawn (`oag_raceplay::between_ticks`). `fx/wormhole.wgsl` of `oag.slipstream` reads its tick
count this way.

Both textures sample at level 0 with a repeating linear sampler, so they may be read anywhere,
inside an `if` too. `mod_fragment` returns straight alpha in gamma space, like every colour a mod
hands the engine: the engine converts it to whatever the title's scene target holds
(`oag_shaders::target`), so one file looks alike on Pulse's 8-bit target and HD's float target.
A value is clamped to 0..1 on an 8-bit target; on a linear float target (Omega's, 2048's) only
below 0, and above 1 it is kept up to the half float's ceiling (65504) for the bloom and the
tonemap, as the frame's own draws are. HD's scene stores what the console's packer wrote,
encoded and clamped to 1 (`SceneStore::Encoded`, `oag_shaders::target::target_out`), so nothing
on HD holds a value above 1 and a shader's output there stops at white as the rest of the frame
does (`mod_shader.wesl`, `scene_rgb`).
It writes colour only: no depth, no glow mask, no velocity, as a particle does.

`uv` by surface: an effect's quad, its corners; a beam, `x` along it from `slot` to `to` and `y`
across; a fill, the view.

### Where it draws

| Surface | Named by | `colour` | When |
| --- | --- | --- | --- |
| An effect's particles | `shader = "<name>"` on an emitter of its TOML ([`mod-effect.md`](../../formats/mod-effect.md)) | the particle's | Depth tested, with the effects |
| A screen fill | `screen_fill_shaded(&craft, rgba, shader)` (`screen-fill` with `arg` the place plus one) | `rgba` | After the flash, over the world, under the HUD. The slot's view alone |
| A backdrop | the same call, through a shader whose header says `fill = "backdrop"` | `rgba` | After the circuit and the shadows, before the crafts. See below |
| A beam | `draw_beam(&craft, to, rgba, half_width, Some(shader))` (`draw-beam`) | `rgba` | Depth tested, with the effects. Nothing while either craft is out of the world |

**A backdrop stands in for the world behind the view's own craft.** Where it is at least half
there (alpha 0.5) it is drawn before the crafts and writes a depth nearer than anything else in the
pass (`BACKDROP_DEPTH`), so every layer drawn after it, the other crafts, the weapons and the
effects, is hidden where it stands; the view's own craft is drawn in a slice of the depth range
nearer still (`Draw::Depth`, `NEAR_SLICE`), so it flies in front of the backdrop. The texels under
half alpha, a glow or something fading out, are drawn where an ordinary fill is, over everything,
and hide nothing. With a backdrop up, a craft in a transit is drawn for its own view although it
is out of the world ("A craft's own transit"). Proof: `oag_fx`'s
`a_header_says_a_fill_through_the_shader_is_a_backdrop`,
`tests::portal_view::a_craft_in_a_transit_is_flown_and_drawn_for_its_own_view_under_a_backdrop`.
Open: a flown craft has its hull and its flame shapes, not the boost plume, the particle exhaust
or the trail, which belong to a craft in the world.

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
which `oag_fx::mod_shader`'s check finds in every function, and WGSL has no recursion. Repeating a
step, such as the octaves of a noise, is written out or put in a helper called several times.

**Covered: helpers that multiply.** A file with no loop is not bounded by its length: a helper that
calls the next one twice, thirty deep, is a billion calls in two kilobytes. So the check counts each
function's *work* through its calls: its own expressions and statements in naga's IR, plus each
callee's work once per call site, both arms of a branch counted. A function over 32,768
(`check::MAX_WORK`) is refused by name. The heaviest shader shipped, `oag.slipstream`'s tunnel, is
1,291; a noise of 115 called for eight octaves from three places is under 3,000. Proof:
`mod_shader::tests::helpers_that_multiply_through_their_calls_are_refused` (the doubling chain is
refused, sixty calls in a row are not).

**Not covered:**

- **A long shader over a big surface.** The bound is on work a pixel, not on the frame, and it
  counts a texture read as it counts an add: a shader at the bound made of texture reads over a 4K
  fill is a slideshow, possibly long enough for a watchdog on a weak GPU. Nothing measures a
  shader's cost on the device.
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

**The exit is a progress or a trigger.** `reply.portal_view_to_trigger(&craft, (entrance, trigger),
..)` names the exit by a track trigger's index, the numbering of `trigger-enter`: the same point a
`transit` to `Distance::Trigger` comes out at (`transit::Exit::at`, shared by both). A progress is
always on the ring; a trigger standing nearer a fork route's line than the ring's is on that route
(`Course::locate_exit`), so the view is framed from the route's own deck, facing along it, which is
what the craft lands on. The wire carries the trigger as the sign bit of the exit word and the index
below it, so a progress exit is the same bytes it always was (`ext-abi`
`a_portal_views_exit_is_a_progress_or_a_trigger`). The request resolves the exit once, when the
command lands; a progress off the lap, or a trigger the track lacks or that is off the course, asks
for nothing. The entrance is still a ring progress (a firer already on a route is open).
Proof: `tests::portal_view::a_portal_view_to_a_forks_route_is_framed_from_the_route` (Pulse PSP EU
`05_Track`, a trigger on route 0; with the exit forced back onto the ring the frame stands 61.4 units
from the route's point, the test's bound is 5). **2048 and Omega: checked, applies, not wired
separately**: their forks are in the same `Course` (`branch_progress_ground_truth` covers every 2048
route's `route_point`), the resolution and frame code are title-blind, and no portal-view test runs on
a 2048 or Omega fork.

```wgsl
let view = engine_texture(engine_uv(input.world)).rgb;   // stays on the surface under both updates
```

`engine_uv(world)` is the camera that drew the texture, carried back through the pair, so a point of
the entrance's surface lands on the texel it covers on the screen. With `update` continuous that is
`screen_uv`; under a snapshot only `engine_uv` stays on the surface, so use it. `engine_texture` reads
what the scene target holds, turned back into gamma-authored colour (HD stores encoded linear, 2048
gamma 2), so a shader that returns it writes the scene's own value through `fs_main`, and is opaque:
the stored alpha is the bloom's glow mask. A value above 1 survives the round trip on Omega's and
2048's float scene (the portal's 960x540 target is the scene's own format, so it stores one;
`a_highlight_in_the_engine_texture_survives_a_pass_through`); it was lost in `fs_main`'s clamp,
not in the target or the sampling. On HD there is none to keep. Seen on Omega at tick 725
(Talon's Junction on HD is byte-identical): the portal's whites were already at the tonemap's
white, so the change is in the bloom halo and the adaptive exposure, about 1 to 3 levels of 255
across the frame, darker by a level or so as the exposure adapts to the brighter average.

**The camera.** `virtual = exit * rotation_180 * entrance^-1 * main`, the turn about the portal's
local up, with each portal's `+z` out of the face a craft sees: back along the road at the entrance,
forward at the exit, up the spline sample's own (banking and all: the same `Race::road_frame` as
`fire-mod-effect-ahead`). A view in a vertical loop is upside down because the road is: Pulse's
`16_Track` turns over between progress 1750 and 2180. What stands behind the exit's plane is clipped with an oblique
near plane (`portal::oblique_near`, the 0..1 depth form); the frustum and the sky keep the plain
matrix, because the swing tilts the far plane and the sky rides on the eye, behind that plane.

**Nothing of the viewer's screen is in it.** The second scene is the world at the exit: it draws
no mod surface or fill, no lens mist, and none of the viewer's own screen wash (a detonation's
flash, a flat `screen-fill`) or cockpit hit static, which are drawn once, over the view the
portal's surface is in (`Race::screen_wash`, `Race::cockpit_hit_strips`;
`tests::craft_flash::the_view_through_a_portal_wears_no_wash`).

**A craft's own transit.** A craft in a `transit` is frozen where it went in and comes out on
the road's middle, facing down it. Three things follow from one pose, its *seat*: the pose it has
on its exit tick carried back through the pair of frames at the craft's own lap progress and at the
transit's exit (`oag_raceplay::transit_camera`, the player's camera and a split's seats).

- **Its camera flies on.** The camera's target eases from the frozen craft onto the seat, over 12
  ticks and with its spring four times as fast, and keeps going along the seat's heading at the
  speed the craft went in at. The world does not stop at the entrance: the camera goes through
  where the craft did. On the exit tick it is seated behind the craft, with no spring to cross
  the gap.
- **Its pair is seen from the seat.** A `portal-view` of a slot whose craft is in a transit is
  framed by that camera carried back by the distance flown (`Race::pair_view`). From
  `Craft::progress()` of the craft to the transit's exit it shows, once the camera is on its
  line (16 ticks in), the picture the camera has on the exit tick: measured on Pulse PSP EU
  `16_Track` (2026-11-10, exit at tick 774), the two eyes stand 0.12 units apart and look
  0.0002 apart; before the seat, 3.5 units and 17 degrees. A screen fill reads it as
  `engine_texture(input.screen_uv)` under `Continuous`, or under a `Once` taken from then on.
- **It is drawn for its own view under a backdrop**, where the camera follows it
  (`Race::ship_drawn`), and for no other view: it is out of the world. `craft_frame` for it is
  the line it is flown along, the seat flown on, which the drawn hull eases onto over those first
  ticks.

Proof: `tests::portal_view::a_view_through_a_transits_own_pair_ends_on_the_camera_the_craft_comes_out_with`
and `a_craft_in_a_transit_is_flown_and_drawn_for_its_own_view_under_a_backdrop`.
**2048 and Omega: checked, applies, wired by the same code**: the seat reads the `Course` and the
spline alone; a strip of frames of each read (2026-11-10), no test of its own.

**`update`:** `ViewUpdate::Continuous` draws every frame the portal is in the view. `ViewUpdate::Once`
draws one snapshot, the first frame it is, and again when `generation` changes. The transform,
clipping and scene are the same; only the rate differs. No draw at all, `engine_texture_bound()`
false, while the entrance's face is turned from the camera or its sphere (`radius`, whole units) is
outside the view, for a progress off the lap, or where the main view's own visible set leaves out
everything the surface's sphere (`radius` about the entrance's centre) reaches: a wall or terrain in
between. On a section partition (Pulse) that is every authored `section_id` of a spline sample
inside the sphere (an unnamed section counts as visible); on a chunk partition (HD, 2048, Omega) it is
every non-moving draw of the circuit whose bounds the sphere meets, tested against the seat's chunk
set (`ChunkSet::allows`, the set the scene pass is masked with; a draw with no chunk index is never
excluded). A sphere that meets no draw, a track with no partition, and a view that does not cull by
partition keep drawing. The test is per seat (a split's seat that does not see it falls to the next
one), and a `Once` snapshot is not spent on a skipped frame: it is taken on the first frame the
sphere is visible
(`tests::portal_view::a_portal_behind_a_wall_draws_nothing`;
`a_portal_whose_radius_reaches_a_visible_section_still_draws`; on chunks
`a_portal_among_hidden_chunks_draws_nothing`, `a_portal_whose_radius_reaches_a_visible_chunk_still_draws`,
`a_portal_reaching_an_unplaced_draw_still_draws`, `a_portal_reaching_no_draw_still_draws`). A moving
draw is left out of the reach: its bounds are where it stood at time zero. Checked on a disc with
`oag.slipstream` fired at tick 700 (the portal draws in view): HD (Talon's Junction, 621 cells over
983 chunks) and Omega (882 cells over 2659 chunks); 2048 loads the same `from_hd_pvs` path as Omega and
**was not run** (checked, applies; only a PFS-encrypted `.pkg` is held, which is not read in place).

**What it costs.** One scene a frame, or one for a snapshot; a race that asks for nothing draws no
pass and builds no target (`tests::portal_view`, counted in passes). The view is a scene pass of its
own at 960x540, in a submission of its own ahead of the view that shows it, with no shadows, post
chain, jitter or PVS: both PVS sets are keyed to the craft and camera on this side of the portal, so
the frustum alone culls it (the main view's sets only decide whether to draw it at all) and a long view draws more of the circuit than the main one. No mod
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
no frame; since the burst is gated behind `graphics.trail_hits` (off by default) the pool holds
the surface alone, and with the setting on 31 slots are the burst and the 32nd the held surface
(measured at tick 480, Omega EU, `oag.window`). 2048: **checked, differs**: before the slot was held its surface reached 0.148 of the view at
tick 1230 of `PCSF00007`, so its pool does not fill there.

Evidence, 960x544, `mod_shader_frame_ground_truth`, inside the surface, mean difference of a channel
out of 255, 2026-11-04:

| | Surface, share of the view | Against no mod | Against a procedural surface | Against the reference camera | Texels moved outside it |
| --- | --- | --- | --- | --- | --- |
| Pulse PSP EU, tick 548 | 0.196 | 88.5 | 88.6 | 10.2 | 0.00036 |
| Pulse, motion blur high | 0.198 | 86.6 | 87.3 | 18.6 | 0.00398 |
| HD, tick 1022 | 0.107 | 53.5 | 71.8 | 6.1 | 0.00000 |

The reference is the same tick with no mod from `--camera-pose` at the view's logged camera: unclipped
and through the whole post chain at full size, so not zero. It proves the texture is that camera's
picture, and a wrong camera agrees with itself there: that the camera is right is
`tests::portal_view::the_view_stands_behind_the_exit_and_looks_down_the_road`, against the road's own
numbers.

"Outside" is past 88 texels of the surface, against a race with no mod, which draws no secondary view
(a run with the procedural shader still draws it, so it cannot show a leak). The rim and its bloom fall
off inside that margin. The blur run's 0.00398 is one patch at the craft's plume, 625 texels at most 70
of 765, and the same 625 with a guest that sends no `portal-view`: a mod's, not the view's. Writing
the main view's camera history from the secondary view moves 0.22, which is what that run guards.

## A mod's quality row

A mod's own choice row reaches its guest as `Observation::option(n)` (`oag_ext_abi::OPTIONS_AT`, four
bytes after the aiming matrix): the index of the pick in the row's `values`, rows in the order of the
mod's `menu.toml` (`oag_game::mod_extensions::options_of`, `Source::options`, a `Picks`). It is a drawn choice: a
guest must not let it steer the rules: the row is declared `scope = "view"` (`hooks.md`, "Picks: sim and
view"), so the simulation call reads the row's default (High) on every device and only a second,
outputs-only call reads this device's pick. `oag.slipstream` reads it as Low (no view, the shader's own
surface), Medium (`ViewUpdate::Once`, one generation a portal) or High (`Continuous`) and sends the same
transit, fill and sounds at all three (`tests::extension_slipstream`). The guest's own read
(`Quality::of(obs.option(0))`) did not move: the `generation` it sends is state, not a function of the
pick. A server's guest reads High and no device draws there. Frames on Pulse PSP EU `16_Track`, tick 534 after
a fire at 520, low / medium / high differ and High shows the road through the window
(`tests::extension_picks`).

## Builds

- **Web:** the library checks for `wasm32-unknown-unknown`, `portal-view` included
  (`cargo check --target wasm32-unknown-unknown -p oag-game --no-default-features --features web`);
  inactive, as every module is there, and not run in a browser. [`web.md`](../../tools/web.md),
  "What is missing".
- **Android:** `oag-mobile`, `oag-fx` and `oag-gpu` check for `aarch64-linux-android`. Modules there
  are as the rest of the app; not run on a device.
- **`portal-view` on 2048 and Omega: checked, applies, wired by the same code**: a frame of each read, no
  test of its own.
- **2048 and Omega: checked, applies, not wired separately.** The stage is title-blind and
  `oag.tether` runs on both: a frame with the mod against one without, 960x544, 2026-11-04, share of
  texels that differ, the view's middle and a corner: 2048 (tick 800) 0.098 and 0.93, Omega
  (tick 700) 0.58 and 0.97. Omega's middle is too high to be the beam alone and the cause was not
  looked for. Neither has a test of its own: `mod_shader_frame_ground_truth` holds Pulse and HD.
