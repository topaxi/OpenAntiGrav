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
    let n = face_size as usize;
    let mut out = Vec::with_capacity((SIZE.0 * SIZE.1 * 4) as usize);
    let samples = (SUBSAMPLES * SUBSAMPLES) as f32;
    for j in 0..SIZE.1 {
        for i in 0..SIZE.0 {
            let mut sum = [0.0f32; 3];
            for sj in 0..SUBSAMPLES {
                for si in 0..SUBSAMPLES {
                    let u = (i as f32 + (si as f32 + 0.5) / SUBSAMPLES as f32) / SIZE.0 as f32;
                    let v = (j as f32 + (sj as f32 + 0.5) / SUBSAMPLES as f32) / SIZE.1 as f32;
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

/// The map of a sky model [`super::sky_cube::build`] made: six square `RGBA8` faces. `None` for any
/// other model (a Pulse sky mesh, an uploaded or compressed face), which then has no map to bind.
#[must_use]
pub fn from_sky(sky: &Model) -> Option<Vec<u8>> {
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
