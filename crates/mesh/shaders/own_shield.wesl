// Our own HD/Fury-style shield: a procedural shell over the craft's hull,
// shaded without any texture. See `docs/rendering/own-shield.md`.
//
// Off by default and on only for the model `oag_livery` builds from a Pulse or
// Pure hull when the player's `graphics.own_shield` is set
// (`oag_mesh::mesh::Model::own_shield`). The override is constant-folded
// away with the setting off, so no other model's shader changes.

import package::bindings::{scene, uniforms};
import package::types::{VertexOutput};

override own_shield_shading: f32 = 0.0;

const HEX_SCALE: f32 = 0.9;
const SQRT3: f32 = 1.7320508;

fn hash21(p: vec2<f32>) -> f32 {
    var q = fract(p * vec2<f32>(0.1031, 0.1030));
    q = q + dot(q, q.yx + vec2<f32>(33.33));
    return fract((q.x + q.y) * q.x);
}

// The hexagon grid at `p`: `x` is the distance to the nearest cell edge in
// units of the cell (0 on an edge, about 0.5 at the centre) and `yz` the
// cell's identity.
fn hex_cell(p: vec2<f32>) -> vec3<f32> {
    let r = vec2<f32>(1.0, SQRT3);
    let h = r * 0.5;
    let a = p - r * floor(p / r) - h;
    let b = (p - h) - r * floor((p - h) / r) - h;
    var g = b;
    if dot(a, a) < dot(b, b) {
        g = a;
    }
    let q = abs(g);
    let edge = max(max(q.x, dot(q, vec2<f32>(0.5, 0.8660254))), 0.0);
    return vec3<f32>(0.5 - edge, p - g);
}

// One projection's contribution: edge lines and a per-cell shimmer.
fn hex_layer(p: vec2<f32>, clock: f32) -> vec2<f32> {
    let cell = hex_cell(p * HEX_SCALE);
    let line = 1.0 - smoothstep(0.0, 0.07, cell.x);
    let seed = hash21(cell.yz);
    let shimmer = pow(0.5 + 0.5 * sin(clock * (0.8 + 1.6 * seed) + seed * 40.0), 6.0);
    return vec2<f32>(line, shimmer);
}

// The width of the bright line along the shell's silhouette, in world units.
const EDGE_WIDTH: f32 = 0.035;

// `in.colour` carries the shield's animated colour in `rgb` and its flickering
// alpha in `a`, written every frame as the Pulse shell's own is.
//
// The interpolated normal is the shell's smooth one (`oag_livery::own_shield`);
// the facets come from the flat normal of the triangle under the fragment.
fn own_shield(in: VertexOutput) -> vec4<f32> {
    let n = normalize(in.normal);
    let to_eye = normalize(scene.fog.camera - in.world);
    let facing = clamp(abs(dot(n, to_eye)), 0.0, 1.0);
    let fresnel = pow(1.0 - facing, 3.0);
    let dx = dpdx(in.world);
    let dy = dpdy(in.world);
    let flat = normalize(cross(dx, dy));
    let facet = pow(1.0 - clamp(abs(dot(flat, to_eye)), 0.0, 1.0), 1.5);
    let clock = scene.time.x;

    // The silhouette: where the smooth normal turns edge-on to the eye within
    // `EDGE_WIDTH` of the outline. `facing / fwidth(facing)` is the distance to
    // that turn in pixels, and a pixel's own world size at this depth converts
    // the width, so a wing seen at a grazing angle, where `facing` is small but
    // steady, is not lit up whole.
    let pixel = max(
        length(dx - to_eye * dot(dx, to_eye)),
        length(dy - to_eye * dot(dy, to_eye)),
    );
    let reach = max(EDGE_WIDTH / max(pixel, 1e-6), 1.5);
    let outline = 1.0 - smoothstep(0.4 * reach, reach, facing / max(fwidth(facing), 1e-5));

    // The craft's own frame, so the pattern rides the hull rather than
    // sliding through it as the craft moves: the model matrix is a rotation and
    // a uniform scale.
    let c0 = uniforms.model[0].xyz;
    let c1 = uniforms.model[1].xyz;
    let c2 = uniforms.model[2].xyz;
    let d = in.world - uniforms.model[3].xyz;
    let k = 1.0 / max(dot(c0, c0), 1e-6);
    let local = vec3<f32>(dot(d, c0), dot(d, c1), dot(d, c2)) * k;
    let ln = vec3<f32>(dot(n, c0), dot(n, c1), dot(n, c2));
    var w = pow(abs(ln) + vec3<f32>(1e-3), vec3<f32>(4.0));
    w = w / (w.x + w.y + w.z);
    let hex = hex_layer(local.zy, clock) * w.x
        + hex_layer(local.xz, clock) * w.y
        + hex_layer(local.xy, clock) * w.z;

    // A bright band travelling nose to tail.
    let sweep = fract(clock * 0.35);
    let along = fract(local.z * 0.045 + 0.5) - sweep;
    let band = exp(-along * along * 260.0);

    let lattice = hex.x * (0.2 + 0.5 * fresnel + 0.8 * band) + hex.y * 0.2 * (0.3 + fresnel);
    let glow = 0.10 + 0.45 * facet + 0.5 * fresnel + lattice + 0.5 * band * fresnel + 2.2 * outline;
    let rgb = in.colour.rgb * glow + in.colour.rgb * vec3<f32>(0.2, 0.5, 0.5) * outline;
    let cover = clamp(0.10 + 0.3 * facet + 0.4 * fresnel + 0.25 * hex.x + outline, 0.0, 0.95);
    return vec4<f32>(rgb, in.colour.a * cover);
}
