//! The engine's `paraboloidReflectionTex`: the circuit's sky as a dual hemisphere map.
//!
//! Wipeout HD's glass, water and floor programs declare one engine-supplied picture,
//! `paraboloidReflectionTex` (`0x9edd3243`), and address it by a reflection vector
//! (`shade.wesl`'s `env_probe_uv`). It is a **single 512x256 `A8R8G8B8` render target at
//! VRAM `0xc4065380`, bound on units 1 to 3 of hundreds of draws of one frame**, and its
//! contents are the circuit's own `sky.gtf` resampled: read back from RPCS3 on cold boots
//! of Moa Therma, Anulpha Pass, Metropia and Talon's Junction, each equals the box-filtered
//! bytes of that circuit's cubemap to within 0.014 rms (`docs/rendering/hd-track-tone.md`,
//! "The reflection map is the sky").
//!
//! The law, as measured and not as the program's lookup suggests:
//!
//! - two discs, `x < 0` in the left half and `x > 0` in the right, each `256` texels across;
//! - in a disc, `a` runs across it `-1..1` and `b = 1 - 2v` up it; the direction is
//!   `(+-sqrt(1 - a^2 - b^2), b, z)`, with `z = a` on the left and `z = -a` on the right;
//! - the picture is the cube's raw bytes averaged over the texel (a 4x4 box here), **no gamma
//!   step, no `Sky colour` tint and no `Sky rotation`**: Talon's Junction (tint 128, rotation
//!   17) matches at rotation 0 with correlation 0.98 and at 17 degrees with 0.55.
//!
//! The lookup law reads the same `z` the other way round (`u = 0.25 - z / 4` on the left), so the
//! composite a glass pane shows is `cube(d.x, d.y, -d.z)`: the sky mirrored in `z`. Both halves
//! are measured; neither is chosen. Outside the discs the original's corners hold the
//! generator's own overshoot (a chevron of stretched rays); no program reads there, so this
//! writes the nearest disc-edge direction instead, **chosen, not measured**.
//!
//! The cube is this module's input as the six decoded faces a [`super::sky_cube`] model carries,
//! so nothing is decoded twice and nothing here knows a title.

use super::{Model, Texels};

/// The map's size in texels: the render target's own.
pub const SIZE: (u32, u32) = (512, 256);

/// Sub-samples per texel and axis. The measured map fits a 4x4 box best (0.0104 rms against
/// 0.0118 for 2x2 on Moa Therma).
const SUBSAMPLES: u32 = 4;

/// The six faces' texture axes `(s, t)` as functions of a direction, GL's cube addressing and
/// the table `sky_cube` draws the faces from: `+X, -X, +Y, -Y, +Z, -Z`.
fn face_and_uv(d: [f32; 3]) -> (usize, f32, f32) {
    let [x, y, z] = d;
    let (ax, ay, az) = (x.abs(), y.abs(), z.abs());
    let (face, sc, tc, major) = if ax >= ay && ax >= az {
        if x > 0.0 {
            (0, -z, -y, ax)
        } else {
            (1, z, -y, ax)
        }
    } else if ay >= az {
        if y > 0.0 {
            (2, x, z, ay)
        } else {
            (3, x, -z, ay)
        }
    } else if z > 0.0 {
        (4, x, -y, az)
    } else {
        (5, -x, -y, az)
    };
    (face, (sc / major + 1.0) / 2.0, (tc / major + 1.0) / 2.0)
}

/// The direction the map's texel coordinate `(u, v)` (each `0..1`) holds, per the module's law.
fn direction(u: f32, v: f32) -> [f32; 3] {
    let left = u < 0.5;
    let mut a = if left {
        (u - 0.25) * 4.0
    } else {
        (u - 0.75) * 4.0
    };
    let mut b = 1.0 - 2.0 * v;
    let r2 = a * a + b * b;
    if r2 > 1.0 {
        let scale = r2.sqrt().recip();
        a *= scale;
        b *= scale;
    }
    let along = (1.0 - a * a - b * b).max(0.0).sqrt();
    [
        if left { -along } else { along },
        b,
        if left { a } else { -a },
    ]
}

/// Builds the map from a cube's six faces, each `face_size` square `RGBA8`, in the order
/// `+X, -X, +Y, -Y, +Z, -Z`. `SIZE.0 * SIZE.1 * 4` bytes, alpha 255.
#[must_use]
pub fn build(faces: [&[u8]; 6], face_size: u32) -> Vec<u8> {
    build_level(faces, face_size, SIZE)
}

/// Bytes of a map whose levels `0..levels` are `SIZE` halved per level: what
/// [`crate::mesh_render::upload_sky_probe`] reads a chain's level count back from.
#[must_use]
pub fn chain_bytes(levels: u32) -> usize {
    (0..levels)
        .map(|k| ((SIZE.0 >> k) * (SIZE.1 >> k) * 4) as usize)
        .sum()
}

/// Omega's sky cube array as a map whose level `k` is cube `k`, each cube one more blur step
/// (`skyCube.gnf`, `docs/ghidra/functions/ps4-omega-eu/panel-displacement.md`): the program mixes
/// cube `floor(lod)` with the next by `fract(lod)`, which two level reads at `floor(lod)` and
/// `floor(lod) + 1` give back. Each cube as its six square `RGBA8` faces, cube `k`'s
/// `face_size >> k` a side (each read at the mip that matches its level of the map).
#[must_use]
pub fn from_cubes(cubes: &[[Vec<[u8; 4]>; 6]], face_size: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(chain_bytes(cubes.len() as u32));
    for (level, faces) in cubes.iter().enumerate() {
        let faces: [&[u8]; 6] = std::array::from_fn(|f| faces[f].as_flattened());
        let side = (face_size >> level).max(1);
        out.extend(build_level(faces, side, (SIZE.0 >> level, SIZE.1 >> level)));
    }
    out
}

fn build_level(faces: [&[u8]; 6], face_size: u32, size: (u32, u32)) -> Vec<u8> {
    let n = face_size as usize;
    let mut out = Vec::with_capacity((size.0 * size.1 * 4) as usize);
    let samples = (SUBSAMPLES * SUBSAMPLES) as f32;
    for j in 0..size.1 {
        for i in 0..size.0 {
            let mut sum = [0.0f32; 3];
            for sj in 0..SUBSAMPLES {
                for si in 0..SUBSAMPLES {
                    let u = (i as f32 + (si as f32 + 0.5) / SUBSAMPLES as f32) / size.0 as f32;
                    let v = (j as f32 + (sj as f32 + 0.5) / SUBSAMPLES as f32) / size.1 as f32;
                    let (face, fu, fv) = face_and_uv(direction(u, v));
                    let px = ((fu * n as f32) as usize).min(n - 1);
                    let py = ((fv * n as f32) as usize).min(n - 1);
                    let at = (py * n + px) * 4;
                    for (channel, total) in sum.iter_mut().enumerate() {
                        *total += f32::from(faces[face][at + channel]);
                    }
                }
            }
            for total in sum {
                out.push((total / samples + 0.5) as u8);
            }
            out.push(255);
        }
    }
    out
}

/// The map of a sky model: the [`Model::sky_reflection`] its loader found, else the one built from
/// the six square `RGBA8` faces of a [`super::sky_cube::build`] model. `None` for any other model
/// (an uploaded or compressed face), which then has no map to bind. A mesh sky (Pulse's and
/// Pure's) has no map of its own: [`from_mesh`] builds one where a surface wants it.
#[must_use]
pub fn from_sky(sky: &Model) -> Option<Vec<u8>> {
    if let Some(map) = &sky.sky_reflection {
        return Some(map.to_vec());
    }
    let mut faces: Vec<&[u8]> = Vec::with_capacity(6);
    let mut size = None;
    for texture in &sky.textures {
        let texture = texture.as_ref()?;
        if texture.width != texture.height || *size.get_or_insert(texture.width) != texture.width {
            return None;
        }
        let Texels::Rgba8(bytes) = &texture.texels else {
            return None;
        };
        if bytes.len() != (texture.width * texture.height * 4) as usize {
            return None;
        }
        faces.push(bytes);
    }
    let faces: [&[u8]; 6] = faces.try_into().ok()?;
    Some(build(faces, size?))
}

/// The base level of a texture as `RGBA8`, with its size.
fn base_level(texture: &super::ModelTexture) -> Option<(&[u8], usize, usize)> {
    let bytes: &[u8] = match &texture.texels {
        Texels::Rgba8(bytes) => bytes,
        Texels::Chain(levels) => levels.first()?,
        _ => return None,
    };
    let (w, h) = (texture.width as usize, texture.height as usize);
    (w > 0 && h > 0 && bytes.len() >= w * h * 4).then_some((bytes, w, h))
}

/// A sky **mesh** (Pulse's and Pure's `Skycube`: a dome of a few hundred textured triangles
/// around the origin) as the map, by casting the ray of each texel from the origin into the
/// triangles and reading the picture there, texel times vertex colour, in the gamma the sky is
/// drawn in. A texel no triangle covers takes the mean of the ones that were hit. `None` for a
/// model with no readable triangle. Chosen, not measured: the original has no reflection map on
/// these titles (`docs/rendering/water.md`, "Enhanced").
#[must_use]
pub fn from_mesh(sky: &Model) -> Option<Vec<u8>> {
    struct Triangle {
        a: [f32; 3],
        e1: [f32; 3],
        e2: [f32; 3],
        uv: [[f32; 2]; 3],
        colour: [[f32; 3]; 3],
        texture: usize,
    }
    let mut triangles = Vec::new();
    for draw in &sky.draws {
        let Some(texture) = draw.texture else {
            continue;
        };
        if sky
            .textures
            .get(texture)
            .and_then(Option::as_ref)
            .and_then(|t| base_level(t))
            .is_none()
        {
            continue;
        }
        for tri in sky.indices[draw.range.start as usize..draw.range.end as usize].chunks_exact(3) {
            let v = |k: usize| &sky.vertices[tri[k] as usize];
            let (a, b, c) = (v(0), v(1), v(2));
            let sub = |p: [f32; 3], q: [f32; 3]| [p[0] - q[0], p[1] - q[1], p[2] - q[2]];
            let colour = |x: &super::GpuVertex| [x.colour[0], x.colour[1], x.colour[2]];
            triangles.push(Triangle {
                a: a.position,
                e1: sub(b.position, a.position),
                e2: sub(c.position, a.position),
                uv: [a.texcoord, b.texcoord, c.texcoord],
                colour: [colour(a), colour(b), colour(c)],
                texture,
            });
        }
    }
    if triangles.is_empty() {
        return None;
    }
    let cross = |a: [f32; 3], b: [f32; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let mut out = vec![0u8; (SIZE.0 * SIZE.1 * 4) as usize];
    let mut missed = Vec::new();
    let mut sum = [0.0f64; 3];
    let mut hits = 0.0f64;
    for j in 0..SIZE.1 {
        for i in 0..SIZE.0 {
            let u = (i as f32 + 0.5) / SIZE.0 as f32;
            let v = (j as f32 + 0.5) / SIZE.1 as f32;
            let d = direction(u, v);
            // The lookup reads the map at `direction`'s mirror in `z` (module doc).
            let ray = [d[0], d[1], -d[2]];
            let mut nearest = f32::MAX;
            let mut colour = None;
            for t in &triangles {
                let p = cross(ray, t.e2);
                let det = dot(t.e1, p);
                if det.abs() < 1.0e-9 {
                    continue;
                }
                let inv = 1.0 / det;
                let origin = [-t.a[0], -t.a[1], -t.a[2]];
                let s = dot(origin, p) * inv;
                if !(0.0..=1.0).contains(&s) {
                    continue;
                }
                let q = cross(origin, t.e1);
                let w = dot(ray, q) * inv;
                if w < 0.0 || s + w > 1.0 {
                    continue;
                }
                let along = dot(t.e2, q) * inv;
                if along <= 0.0 || along >= nearest {
                    continue;
                }
                nearest = along;
                colour = Some((t, 1.0 - s - w, s, w));
            }
            let at = ((j * SIZE.0 + i) * 4) as usize;
            let Some((t, b0, b1, b2)) = colour else {
                missed.push(at);
                continue;
            };
            let Some((bytes, tw, th)) = sky.textures[t.texture].as_deref().and_then(base_level)
            else {
                missed.push(at);
                continue;
            };
            let tu = b0 * t.uv[0][0] + b1 * t.uv[1][0] + b2 * t.uv[2][0];
            let tv = b0 * t.uv[0][1] + b1 * t.uv[1][1] + b2 * t.uv[2][1];
            let px = ((tu - tu.floor()) * tw as f32) as usize % tw;
            let py = ((tv - tv.floor()) * th as f32) as usize % th;
            let texel = &bytes[(py * tw + px) * 4..][..3];
            for k in 0..3 {
                let light = b0 * t.colour[0][k] + b1 * t.colour[1][k] + b2 * t.colour[2][k];
                let byte = (f32::from(texel[k]) * light.clamp(0.0, 1.0) + 0.5) as u8;
                out[at + k] = byte;
                sum[k] += f64::from(byte);
            }
            out[at + 3] = 255;
            hits += 1.0;
        }
    }
    let mean: [u8; 3] = std::array::from_fn(|k| (sum[k] / hits.max(1.0)) as u8);
    for at in missed {
        out[at..at + 3].copy_from_slice(&mean);
        out[at + 3] = 255;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A cube whose every face is one flat colour, the face index in red.
    fn flat_faces() -> Vec<Vec<u8>> {
        (0..6u8)
            .map(|face| [10 * (face + 1), 0, 0, 255].repeat(4))
            .collect()
    }

    fn map() -> Vec<u8> {
        let faces = flat_faces();
        build(std::array::from_fn(|i| faces[i].as_slice()), 2)
    }

    fn red_at(map: &[u8], u: f32, v: f32) -> u8 {
        let x = ((u * SIZE.0 as f32) as usize).min(SIZE.0 as usize - 1);
        let y = ((v * SIZE.1 as f32) as usize).min(SIZE.1 as usize - 1);
        map[(y * SIZE.0 as usize + x) * 4]
    }

    /// The two disc centres look straight down `-X` and `+X`, the top of a disc straight up.
    #[test]
    fn the_discs_look_along_x_and_up() {
        let map = map();
        assert_eq!(map.len(), (SIZE.0 * SIZE.1 * 4) as usize);
        assert_eq!(red_at(&map, 0.25, 0.5), 20, "left centre is -X");
        assert_eq!(red_at(&map, 0.75, 0.5), 10, "right centre is +X");
        assert_eq!(red_at(&map, 0.25, 0.03), 30, "left top is +Y");
        assert_eq!(red_at(&map, 0.25, 0.97), 40, "left bottom is -Y");
    }

    /// The measured `z` sign: `z = a` on the left, `z = -a` on the right, so both sides of the
    /// seam at `u = 0.5` look along `+Z` and both outer edges along `-Z`.
    #[test]
    fn z_runs_the_measured_way_round() {
        let map = map();
        assert_eq!(red_at(&map, 0.495, 0.5), 50, "left disc, right edge: +Z");
        assert_eq!(red_at(&map, 0.005, 0.5), 60, "left disc, left edge: -Z");
        assert_eq!(red_at(&map, 0.505, 0.5), 50, "right disc, left edge: +Z");
        assert_eq!(red_at(&map, 0.995, 0.5), 60, "right disc, right edge: -Z");
    }
}
