// Omega's panel-map road (`Emissive::program` mode 15, `oag_mesh::mesh::rcs::psp2::panel_map`):
// the pixel program's ray march into the panel map, read off the GCN microcode
// (`docs/ghidra/functions/ps4-omega-eu/panel-displacement.md`, "The law", 88).

@if(!bindless) import package::bindings::{albedo_sampler, fifth_map};
@if(bindless) import package::bindings::{material, samplers, textures};
import package::bindings::{scene};

// The panel map (fifth binding, an `R16Float` distance field in texture heights) at `uv`, level 0.
fn panel_field(uv: vec2<f32>) -> f32 {
    @if(bindless) {
        let albedo_sampler = samplers[material.albedo_sampler];
        let fifth_map = textures[material.fifth_map];
    }
    return textureSampleLevel(fifth_map, albedo_sampler, uv, 0.0).x;
}

// Where the view ray through `uv` meets a recessed channel's wall or floor:
//
//     a, b = -V.T, -V.B          c = sqrt(1 - a^2 - b^2)
//     D, w = 0.01 Depth, 0.01 SlopeWidth
//     step = (a D / c min(1, 1/Aspect), b D / c min(1, Aspect))
//     k    = 1 / sqrt(1 + (SlopeWidth / Depth)^2)
//     t = 0; 10 times: d = k (df(uv + step t) + w (1 - t)); stop at d <= 1e-4;
//                      t += (c / D)(d + 1e-4); stop at t >= 1
//     uv' = uv + step sat(t)
//
// `shape` is `(Depth, SlopeWidth, Aspect)`. The lip blend is left out: it fires only within about
// a degree of grazing (the page, "The lip blend is inert").
fn panel_march(uv: vec2<f32>, world: vec3<f32>, n: vec3<f32>, tangent: vec4<f32>, shape: vec3<f32>) -> vec2<f32> {
    let v = normalize(scene.fog.camera - world);
    let t_axis = normalize(tangent.xyz);
    let handed = select(-1.0, 1.0, tangent.w * 2.0 - 1.0 >= 0.0);
    let b_axis = normalize(cross(n, t_axis) * handed);
    let a = -dot(v, t_axis);
    let b = -dot(v, b_axis);
    let c = sqrt(max(1.0 - a * a - b * b, 1.0e-4));
    let depth = 0.01 * shape.x;
    let wall = 0.01 * shape.y;
    let step = vec2<f32>(
        a * depth / c * min(1.0, 1.0 / shape.z),
        b * depth / c * min(1.0, shape.z),
    );
    let ratio = shape.y / shape.x;
    let k = 1.0 / sqrt(1.0 + ratio * ratio);
    var t = 0.0;
    for (var i = 0u; i < 10u; i = i + 1u) {
        let d = k * (panel_field(uv + step * t) + wall * (1.0 - t));
        if d <= 1.0e-4 {
            break;
        }
        t = t + (c / depth) * (d + 1.0e-4);
        if t >= 1.0 {
            break;
        }
    }
    return uv + step * clamp(t, 0.0, 1.0);
}
