// The side of a mod's shader the engine writes: bindings, vertex stages and the fragment entry.
// `oag_fx::mod_shader` puts the mod's file in front of this text, so naga's line numbers are
// the mod's own, and renames `mod_fragment` below out of the way; this one is what a draw with
// no shader of its own runs. `docs/architecture/wasm-extensions/mod-shaders.md` is the contract.

import oag_shaders::target::{linear_out, gamma_to_target, target_in, target_out, target_to_gamma};

// What a mod's `mod_fragment` is handed.
struct ModInput {
    // 0..1 across the surface, y down: a quad's corners, a beam's length (x) and width (y),
    // the view for a screen fill.
    uv: vec2<f32>,
    // 0..1 across the view, y down.
    screen_uv: vec2<f32>,
    // Where the fragment is in the world; zero on a screen fill.
    world: vec3<f32>,
    // Straight alpha, gamma-authored: the command's colour, or a particle's.
    colour: vec4<f32>,
    // Seconds on the race's world clock.
    time: f32,
};

struct ModFrame {
    view_projection: mat4x4<f32>,
    // The camera that drew the engine texture; identity with none.
    engine_view_projection: mat4x4<f32>,
    // A screen fill's colour.
    fill: vec4<f32>,
    params: array<vec4<f32>, 1>,
    time: f32,
    // The draw alpha minus one, as `psys.wesl` carries it.
    between: f32,
    // 1.0 where the engine texture holds a picture for this view.
    engine_bound: f32,
    pad: f32,
};

@group(0) @binding(0) var<uniform> oag_frame: ModFrame;
@group(0) @binding(1) var oag_mod_texture: texture_2d<f32>;
@group(0) @binding(2) var oag_engine_texture: texture_2d<f32>;
@group(0) @binding(3) var oag_sampler: sampler;

// The PNG the shader's header names; opaque white with none.
fn mod_texture(uv: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(oag_mod_texture, oag_sampler, uv, 0.0);
}

// The picture the engine provides, gamma-authored and opaque whatever the scene target stores
// (its alpha is the bloom's glow mask, not coverage), so a shader that returns it writes back
// what the scene holds; transparent black with no picture for this view.
fn engine_texture(uv: vec2<f32>) -> vec4<f32> {
    if !engine_texture_bound() {
        return vec4<f32>(0.0);
    }
    let stored = textureSampleLevel(oag_engine_texture, oag_sampler, uv, 0.0).rgb;
    return vec4<f32>(target_to_gamma(target_in(stored), linear_out), 1.0);
}

// Whether `engine_texture` holds a picture.
fn engine_texture_bound() -> bool {
    return oag_frame.engine_bound > 0.5;
}

// Where `world` lands in the engine texture, through the camera that drew it.
fn engine_uv(world: vec3<f32>) -> vec2<f32> {
    let clip = oag_frame.engine_view_projection * vec4<f32>(world, 1.0);
    return clip.xy / clip.w * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5);
}

fn mod_fragment(input: ModInput) -> vec4<f32> {
    // The helpers above are the contract, so nothing may strip them from this text.
    let unused = mod_texture(vec2<f32>(0.0)) * 0.0 + engine_texture(engine_uv(vec3<f32>(0.0))) * 0.0;
    return input.colour + unused * select(0.0, 1.0, engine_texture_bound());
}

struct Varying {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) colour: vec4<f32>,
    @location(2) world: vec3<f32>,
    @location(3) screen: vec4<f32>,
};

// `psys.wesl`'s `ParticleInput`: an effect's particles and a beam share the one list.
struct SurfaceVertex {
    @location(0) position: vec3<f32>,
    @location(2) colour: vec4<f32>,
    @location(3) texcoord: vec2<f32>,
    @location(5) shift_xy: vec2<f32>,
    @location(6) shift_z: f32,
};

@vertex
fn vs_surface(in: SurfaceVertex) -> Varying {
    var out: Varying;
    let world = in.position + vec3<f32>(in.shift_xy, in.shift_z) * oag_frame.between;
    out.clip = oag_frame.view_projection * vec4<f32>(world, 1.0);
    out.uv = in.texcoord;
    out.colour = in.colour;
    out.world = world;
    out.screen = out.clip;
    return out;
}

// One triangle over the viewport, as the screen flash draws.
@vertex
fn vs_fill(@builtin(vertex_index) index: u32) -> Varying {
    var out: Varying;
    let x = f32((index << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(index & 2u) * 2.0 - 1.0;
    out.clip = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>(x, -y) * 0.5 + vec2<f32>(0.5);
    out.colour = oag_frame.fill;
    out.world = vec3<f32>(0.0);
    out.screen = out.clip;
    return out;
}

@fragment
fn fs_main(in: Varying) -> @location(0) vec4<f32> {
    let screen_uv = in.screen.xy / in.screen.w * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5);
    let shaded = mod_fragment(ModInput(in.uv, screen_uv, in.world, in.colour, oag_frame.time));
    let rgb = gamma_to_target(clamp(shaded.rgb, vec3<f32>(0.0), vec3<f32>(1.0)), linear_out);
    return vec4<f32>(target_out(rgb), shaded.a);
}
