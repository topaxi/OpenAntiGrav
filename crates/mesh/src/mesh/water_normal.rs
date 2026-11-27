//! The wave normal map `enhanced` water is drawn over, generated here.
//!
//! No disc is read: the map is value noise on a wrapping integer lattice (four octaves),
//! turned into slopes by wrapped central differences, so it tiles exactly and a seed gives
//! the same bytes on every platform (integer hashing and IEEE `+ - * /` and `sqrt` only, no
//! transcendental). Laid out as HD's `dc_waternormalmap` is read by the sea's taps: the
//! normal's `x` in alpha and `y` in green as `2b - 1`, `z` rebuilt by the shader
//! (`water.wesl`). Red carries `z` and blue is filled, neither is read. The shape, the
//! octaves and the slope strength are this project's own, **chosen, not measured**
//! (`docs/rendering/water.md`, "Enhanced").

use super::{ModelTexture, Texels};

/// The map's side in texels; the lattice wraps at it.
pub const SIZE: u32 = 128;

/// Lattice cells per side of each octave, and its amplitude. The cell counts divide [`SIZE`],
/// so a texel sits at a whole fraction of a cell and the wrap is a whole number of cells.
const OCTAVES: [(u32, f32); 4] = [(4, 1.0), (8, 0.55), (16, 0.3), (32, 0.15)];

/// How steeply the height field tilts the normal: the slope `dh / dx` per texel is scaled by
/// this before `z` is `1`. Chosen so the steepest facets lean about thirty degrees.
const SLOPE: f32 = 6.0;

/// A lattice value in `0..1`, hashed from its cell, octave and seed. Integer only, so it
/// agrees on every platform.
fn lattice(seed: u32, octave: u32, x: u32, y: u32) -> f32 {
    let mut h = seed ^ 0x9e37_79b9u32.wrapping_mul(octave + 1);
    h = h.wrapping_add(x.wrapping_mul(0x85eb_ca6b));
    h ^= h >> 15;
    h = h.wrapping_add(y.wrapping_mul(0xc2b2_ae35));
    h ^= h >> 13;
    h = h.wrapping_mul(0x27d4_eb2f);
    h ^= h >> 16;
    // 24 bits fit an `f32` exactly.
    (h >> 8) as f32 / 16_777_216.0
}

/// The height at texel `(x, y)`, with both coordinates taken modulo [`SIZE`].
fn height(seed: u32, x: u32, y: u32) -> f32 {
    let (x, y) = (x % SIZE, y % SIZE);
    let mut total = 0.0;
    for (octave, (cells, amplitude)) in OCTAVES.iter().enumerate() {
        let span = SIZE / cells;
        let (cx, cy) = (x / span, y / span);
        let fx = (x % span) as f32 / span as f32;
        let fy = (y % span) as f32 / span as f32;
        let fade = |t: f32| t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
        let (sx, sy) = (fade(fx), fade(fy));
        let at =
            |dx: u32, dy: u32| lattice(seed, octave as u32, (cx + dx) % cells, (cy + dy) % cells);
        let top = at(0, 0) + (at(1, 0) - at(0, 0)) * sx;
        let bottom = at(0, 1) + (at(1, 1) - at(0, 1)) * sx;
        total += amplitude * (top + (bottom - top) * sy);
    }
    total
}

fn byte(unit: f32) -> u8 {
    ((unit * 0.5 + 0.5) * 255.0 + 0.5).clamp(0.0, 255.0) as u8
}

/// The map's RGBA8 texels, `SIZE` by `SIZE`, row-major.
#[must_use]
pub fn bytes(seed: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = height(seed, x + 1, y) - height(seed, x + SIZE - 1, y);
            let dy = height(seed, x, y + 1) - height(seed, x, y + SIZE - 1);
            let (nx, ny) = (-dx * SLOPE * 0.5, -dy * SLOPE * 0.5);
            let length = (nx * nx + ny * ny + 1.0).sqrt();
            let (nx, ny, nz) = (nx / length, ny / length, 1.0 / length);
            out.extend_from_slice(&[byte(nz), byte(ny), 255, byte(nx)]);
        }
    }
    out
}

/// The map as a texture a model's third binding can hold.
#[must_use]
pub fn texture(seed: u32) -> ModelTexture {
    ModelTexture {
        label: "water: generated normal map".to_string(),
        width: SIZE,
        height: SIZE,
        texels: Texels::Rgba8(bytes(seed)),
        mip_count: None,
        lod_code: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FNV-1a over the bytes: one number for the whole map.
    fn fingerprint(bytes: &[u8]) -> u32 {
        bytes.iter().fold(0x811c_9dc5u32, |h, &b| {
            (h ^ u32::from(b)).wrapping_mul(0x0100_0193)
        })
    }

    /// The generator is a pure function of its seed: the same seed twice is the same bytes, and
    /// the bytes are pinned so a change to the lattice, the octaves or the slope shows up.
    #[test]
    fn a_seed_gives_the_same_bytes_every_time() {
        let first = bytes(7);
        assert_eq!(first, bytes(7));
        assert_eq!(first.len(), (SIZE * SIZE * 4) as usize);
        assert_eq!(
            fingerprint(&first),
            PINNED_SEED_7,
            "{:#x}",
            fingerprint(&first)
        );
        assert_ne!(fingerprint(&first), fingerprint(&bytes(8)));
    }

    const PINNED_SEED_7: u32 = 0x5ed0_7fea;

    fn at(bytes: &[u8], x: u32, y: u32) -> [u8; 4] {
        let i = ((y % SIZE) * SIZE + x % SIZE) as usize * 4;
        [bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]
    }

    fn step(a: [u8; 4], b: [u8; 4]) -> u32 {
        (u32::from(a[1].abs_diff(b[1]))).max(u32::from(a[3].abs_diff(b[3])))
    }

    /// The map tiles: the step across the wrap (last column to the first, last row to the
    /// first) is no larger than the largest step between neighbours inside it. A map that
    /// stopped wrapping would show a seam as a step far past the interior's.
    #[test]
    fn the_map_tiles_seamlessly() {
        let map = bytes(7);
        let mut interior = 0;
        let mut across = 0;
        for k in 0..SIZE {
            for j in 0..SIZE - 1 {
                interior = interior
                    .max(step(at(&map, j, k), at(&map, j + 1, k)))
                    .max(step(at(&map, k, j), at(&map, k, j + 1)));
            }
            across = across
                .max(step(at(&map, SIZE - 1, k), at(&map, 0, k)))
                .max(step(at(&map, k, SIZE - 1), at(&map, k, 0)));
        }
        assert!(interior > 0, "a flat map has nothing to compare");
        assert!(
            across <= interior,
            "seam {across} against interior {interior}"
        );
    }

    /// Every texel decodes to a unit-ish normal that leans: facing mostly up (`z` above a
    /// half), and the whole map neither flat nor a single slope.
    #[test]
    fn the_normals_lean_but_face_up() {
        let map = bytes(7);
        let (mut lean, mut flat) = (0u32, 0u32);
        for texel in map.as_chunks::<4>().0 {
            let z = f32::from(texel[0]) / 255.0 * 2.0 - 1.0;
            assert!(z > 0.5, "z {z}");
            if texel[1].abs_diff(128) > 8 || texel[3].abs_diff(128) > 8 {
                lean += 1;
            } else {
                flat += 1;
            }
        }
        assert!(lean > SIZE * SIZE / 8, "lean {lean}");
        assert!(flat > 0 || lean > 0);
    }

    /// The `x` slope is in alpha and `y` in green, as the sea's taps decode them: the mean of
    /// each is the flat byte, and neither channel is constant.
    #[test]
    fn the_slopes_are_in_green_and_alpha() {
        let map = bytes(7);
        for channel in [1usize, 3] {
            let values: Vec<f32> = map
                .as_chunks::<4>()
                .0
                .iter()
                .map(|t| f32::from(t[channel]))
                .collect();
            let mean = values.iter().sum::<f32>() / values.len() as f32;
            assert!((mean - 127.5).abs() < 6.0, "channel {channel} mean {mean}");
            let (lo, hi) = values
                .iter()
                .fold((255.0f32, 0.0f32), |(a, b), &v| (a.min(v), b.max(v)));
            assert!(hi - lo > 30.0, "channel {channel} spans {lo}..{hi}");
        }
    }

    #[test]
    fn the_texture_is_the_generated_bytes() {
        let texture = texture(7);
        assert_eq!((texture.width, texture.height), (SIZE, SIZE));
        let Texels::Rgba8(texels) = &texture.texels else {
            panic!("RGBA8 expected");
        };
        assert_eq!(texels, &bytes(7));
    }
}
