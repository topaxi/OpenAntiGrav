// The half of a screen filter this project writes: the vertex stage, the
// bindings, the uniform and the entry point. A preset's own file is appended
// after this and supplies `screen_filter`, so every preset - built-in or a player's
// own - is compiled against exactly this text. See `screen.rs` for the
// contract in prose and `docs/rendering/screen-filters.md` for it in full.
//
// Three vertices and no vertex buffer, the same triangle the blit draws: it is
// bigger than the screen so the clipped part costs nothing and there is no
// diagonal seam.

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    var out: VertexOutput;
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    out.uv = uv;
    out.position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    return out;
}

// Everything a filter is told about the frame it is drawing, and the sixteen
// tunables its header declared. Mirrored field for field by `screen::Uniform`
// in Rust, which is what fills it.
struct Screen {
    // The size of the frame in pixels - the presentation target's, which is
    // the window's or the capture's own size. What a resolution-independent
    // grid is derived from: a filter that wants "one LCD row per output row"
    // uses this and nothing else.
    output_size: vec2<f32>,
    // `1.0 / output_size`, so a tap one pixel away is `uv + texel`.
    texel: vec2<f32>,
    // The grid the running title was authored in: 480x272 on a PSP disc,
    // 640x448 on the PS2's, 1920x1080 on Wipeout HD's. What a filter that wants
    // the *original panel's* structure - 272 rows however large the window is -
    // derives its grid from instead.
    native_size: vec2<f32>,
    // How many frames this filter has drawn, as a float so it can be used in
    // arithmetic directly. A field-alternating filter reads its parity.
    frame: f32,
    // Seconds since the filter was built. For anything that animates.
    time: f32,
    // The player's SCREEN FILTER STRENGTH row, 0..1. Applied by `fs_main`
    // below, not by the filter, so every preset honours it identically.
    strength: f32,
    // Three scalars rather than a `vec3<f32>`, which would be 16-aligned and
    // push `params` out to 64: the Rust mirror is `[f32; 3]` at 36.
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
    // The tunables, in the order the header declared them. Read through the
    // `param_<name>()` accessors generated from that header, or by index
    // through `param(i)`.
    params: array<vec4<f32>, 4>,
}

// The frame to filter, at presentation size, with everything in it - the
// scene, the HUD, the menus. A display filter is a simulation of the display,
// and the display showed all of it.
@group(0) @binding(0) var frame: texture_2d<f32>;
// Bilinear, clamp to edge.
@group(0) @binding(1) var frame_sampler: sampler;
@group(0) @binding(2) var<uniform> screen: Screen;
// What this filter itself produced last frame, for persistence, phosphor decay
// and field alternation. Black on the first frame and after a resize.
@group(0) @binding(3) var previous: texture_2d<f32>;
// Nearest, clamp to edge - for a filter that wants the frame's own pixels
// rather than a blend of them.
@group(0) @binding(4) var nearest_sampler: sampler;

// A tunable by index, for a filter that walks them. `param_<name>()` is the
// readable spelling and is generated per preset.
fn param(index: u32) -> f32 {
    return screen.params[index / 4u][index % 4u];
}

// The frame at `uv`, bilinear.
fn frame_at(uv: vec2<f32>) -> vec3<f32> {
    return textureSample(frame, frame_sampler, uv).rgb;
}

// The frame at `uv`, nearest.
fn frame_pixel(uv: vec2<f32>) -> vec3<f32> {
    return textureSample(frame, nearest_sampler, uv).rgb;
}

// Last frame's output at `uv`, bilinear.
fn previous_at(uv: vec2<f32>) -> vec3<f32> {
    return textureSample(previous, frame_sampler, uv).rgb;
}

// Rec. 601 luma, which is what a display-era filter wants: these are
// gamma-encoded values and the weights every CRT and LCD shader of the period
// used on them.
fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let untouched = frame_at(in.uv);
    let filtered = screen_filter(in.uv, in.position.xy);
    return vec4<f32>(mix(untouched, filtered, screen.strength), 1.0);
}
