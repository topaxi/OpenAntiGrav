// Draws a track chunk into a craft's sun-occlusion map: position through the
// craft's own sun-view projection, and the chunk's baked sun-occlusion mask
// out - nothing else.
//
// This is Wipeout HD's `SunOcclusionLightmap` / `SunOcclusionVertex`
// technique, read off `track_surface.rcsmaterial` blocks 3 and 4:
//
//     SunOcclusionLightmap: TEX H1.w, f[TC0] unit0 ; MOV H0.xyz, H1.wwww
//     SunOcclusionVertex:   MOV H0.xyz, f[TC0].wwww
//
// The two carriers are the same two `mesh.wgsl` multiplies as the road's own
// gate - the lightmap's alpha and the colour set's fourth byte - so a hull
// sampling this map darkens at exactly the texel the road under it does. See
// `docs/ghidra/functions/ps3-hdfury-eu/ship-sun-occlusion.md`.

struct Caster {
    // The craft's sun-view view-projection times the track's model matrix.
    mvp: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> caster: Caster;

// The track drawable's own material bind group, in `mesh_render`'s "albedo"
// layout - the albedo is bound but never read here; the lightmap and the
// shared sampler are what this pass is for.
@group(1) @binding(0) var albedo: texture_2d<f32>;
@group(1) @binding(1) var albedo_sampler: sampler;
@group(1) @binding(2) var lightmap: texture_2d<f32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) lightmap_texcoord: vec2<f32>,
    @location(1) sun_mask: f32,
    @location(2) @interpolate(flat) slots: u32,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(6) lightmap_texcoord: vec2<f32>,
    @location(8) sun_mask: f32,
    @location(9) slots: u32,
) -> VertexOutput {
    var out: VertexOutput;
    out.position = caster.mvp * vec4<f32>(position, 1.0);
    out.lightmap_texcoord = lightmap_texcoord;
    out.sun_mask = sun_mask;
    out.slots = slots;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // The same gate `mesh.wgsl`'s `lit_texel` builds as `baked.a *
    // in.sun_mask`: the atlas alpha where the second texture is the lightmap
    // (`slots::SECOND_IS_LIGHTMAP`), the placeholder's `1.0` otherwise, times
    // the colour set's fourth byte.
    let atlas = textureSample(lightmap, albedo_sampler, in.lightmap_texcoord);
    let baked_alpha = select(1.0, atlas.a, (in.slots & 1u) != 0u);
    let mask = baked_alpha * in.sun_mask;
    // **The map holds the mask sRGB-encoded.** Read off RPCS3 (Talon's Junction start, the
    // hull's unit-3 map, 256x256 `B8`): the road holds 61 where its lightmap alpha is the byte
    // 12, and the map's smallest texel is 13 where the lightmap's smallest alpha is 1 -
    // `encode(12 / 255)` and `encode(1 / 255)` to the byte. The hull's unit 3 has no gamma
    // decode (its address word is `0x60030303`), so it reads these bytes as the gate: a road at
    // alpha 12 passes a quarter of the sun, not a twentieth. The encode is the pass's own
    // (the target is written through it); only the lightmap carrier is measured, and the vertex
    // carrier goes through the same write. See `docs/rendering/shadows.md`.
    let encoded = select(1.055 * pow(mask, 1.0 / 2.4) - 0.055, mask * 12.92, mask <= 0.0031308);
    return vec4<f32>(encoded, 0.0, 0.0, 1.0);
}
