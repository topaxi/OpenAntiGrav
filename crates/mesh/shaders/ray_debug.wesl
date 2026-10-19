enable wgpu_ray_query;

// The ray tier's debug picture: one primary ray per pixel against the acceleration
// structures, coloured by what it hit. Never part of a frame a player sees - it is
// the way to check by eye (and by `cmp` against a raster capture at the same pose)
// that the structures sit where the raster geometry does.

struct Params {
    // The camera's inverse view-projection, column-major, `0..1` depth.
    inverse_view_projection: mat4x4<f32>,
    // The longest ray, in world units.
    far: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var scene: acceleration_structure;

fn unproject(ndc: vec2<f32>, depth: f32) -> vec3<f32> {
    let p = params.inverse_view_projection * vec4<f32>(ndc, depth, 1.0);
    return p.xyz / p.w;
}

// Instance custom data names the structure: 0 the circuit's static geometry, 1 a craft
// hull, 2 and up a moving scenery node.
fn kind_colour(kind: u32) -> vec3<f32> {
    if kind == 0u {
        return vec3<f32>(0.35, 0.6, 1.0);
    }
    if kind == 1u {
        return vec3<f32>(1.0, 0.55, 0.2);
    }
    return vec3<f32>(0.4, 1.0, 0.4);
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(output);
    if id.x >= size.x || id.y >= size.y {
        return;
    }
    // Row 0 is the top of the picture, and NDC y is up.
    let uv = (vec2<f32>(id.xy) + vec2<f32>(0.5)) / vec2<f32>(size);
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    let near = unproject(ndc, 0.0);
    let far = unproject(ndc, 1.0);
    let direction = normalize(far - near);

    var query: ray_query;
    rayQueryInitialize(&query, scene, RayDesc(0u, 0xFFu, 0.01, params.far, near, direction));
    while rayQueryProceed(&query) {}

    var colour = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    let hit = rayQueryGetCommittedIntersection(&query);
    if hit.kind != RAY_QUERY_INTERSECTION_NONE {
        // Nearer is brighter, so a structure that sits in the wrong place reads at once.
        let shade = 1.0 / (1.0 + hit.t * 0.01);
        colour = vec4<f32>(kind_colour(hit.instance_custom_data) * shade, 1.0);
    }
    textureStore(output, id.xy, colour);
}
