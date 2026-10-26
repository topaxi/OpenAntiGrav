//! Wipeout Pure's sun lens flare: ten additive quads on a 2x2 atlas, laid along the
//! line from the sun's screen position through the screen centre.
//!
//! The law is `docs/rendering/lens-flare.md` (measured live on PPSSPP, Vineta K and
//! Modesto Heights; `docs/ghidra/functions/psp-pure-usa/lensflare.md` for the
//! functions). This module is the wgpu-free half: [`Placement::new`] turns a sun
//! direction, a camera and a viewport into ten rings. [`pipeline`] draws them.
//!
//! **Authored**: the sun direction, its colour, the atlas. **The executable's**: the ring
//! count, spacing, sizes, cells, alpha and the occlusion test.
//!
//! **Chosen, not measured**
//! - The two random factors (the frame's size jitter `j`, each ring's alpha factor `r`)
//!   take the values the original uses when its engine flag `DAT_002883fc` is set,
//!   `j = 1.0` and `r = 0.7`: both sit inside the measured bands (`0.98..1.02`,
//!   `0.5..0.9`), the picture is reproducible, and the flag's setter and the generator
//!   are unread. See [`Random`].
//! - The flare is drawn in its own pass over the resolved scene, after the scene pass
//!   and before the post chain and the HUD (the original draws it inside the scene's
//!   queue at key `0x4e000000`, so mist and the screen flash, keys `0x4f`, land after
//!   it there and before it here).
//! - Quads are square on the viewport drawn, whatever its aspect (the original's are on 480 x 272).
//! - A sun ring-0 whose half-size would be negative (the sun beyond the corner of the
//!   screen) is clamped to zero.
//! - Occlusion is the 16 depth samples of the original's 4 x 4 grid at one pixel pitch of
//!   2 px, scaled by `viewport height / 272`, read from this frame's depth in the same
//!   frame (the original may read the previous frame's).

use oag_core::math::{Mat4, Vec3, Vec4};

pub mod pipeline;

#[cfg(test)]
mod tests;

/// Rings in the chain.
pub const RINGS: usize = 10;

/// The atlas cell of each ring: `u0 = (c & 1) / 2`, `v0 = (c >> 1) / 2`, each half wide.
pub const CELLS: [u8; RINGS] = [0, 1, 1, 1, 2, 2, 3, 3, 3, 3];

/// The draw's own screen space is `x` in `[-16/9, 16/9]`, `y` in `[-1, 1]`.
const ASPECT: f32 = 16.0 / 9.0;

/// `_DAT_00034114`: the half-diagonal the ring layout is normalised by.
const REACH: f32 = 5.0 / 3.0;

/// The visibility cone, radians.
const CONE: f32 = 1.0;

/// The output the original samples depth on, in pixels (`480 x 272`).
const ORIGINAL_HEIGHT: f32 = 272.0;

/// The size cycle of rings `1..`: `[1.0, 2.0, 0.5, 1.2][(k - 1) & 3]`.
const SIZE_CYCLE: [f32; 4] = [1.0, 2.0, 0.5, 1.2];

/// One circuit's flare as authored: the sun's direction, its colour, and the atlas.
#[derive(Debug, Clone, PartialEq)]
pub struct Setup {
    /// A unit vector from the camera to the sun, world space.
    pub direction: [f32; 3],
    /// `ColourR`, `ColourG`, `ColourB`.
    pub colour: [f32; 3],
    /// The circuit's own `LensflareComponents.tga`.
    pub atlas: Atlas,
}

/// The atlas as RGBA8888, one entry per authored mip level, base first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Atlas {
    /// Base width, pixels.
    pub width: u32,
    /// Base height, pixels.
    pub height: u32,
    /// Level `n` is `max(width >> n, 1)` by `max(height >> n, 1)`.
    pub levels: Vec<Vec<u8>>,
}

/// The two random factors; see the module doc.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Random {
    /// One per frame, `0.98..1.02`.
    pub jitter: f32,
    /// One per ring, `0.5..0.9`.
    pub ring: f32,
}

impl Random {
    /// The engine flag's values: `j = 1.0`, `r = 0.7`.
    pub const FLAGGED: Self = Self {
        jitter: 1.0,
        ring: 0.7,
    };
}

/// One quad of the chain, in the target's normalised device coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ring {
    /// Quad centre, `x` right and `y` up.
    pub centre: [f32; 2],
    /// Half the quad's width and height in NDC (the quad is square on screen).
    pub half: [f32; 2],
    /// The vertex colour, each channel truncated to a byte and back. Alpha excludes the
    /// occlusion fraction, which the draw multiplies in.
    pub rgba: [f32; 4],
    /// The atlas cell.
    pub cell: u8,
}

/// Everything one view's draw needs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// Ring 0 first.
    pub rings: [Ring; RINGS],
    /// The sun's pixel in the depth attachment (y down), where the 16 samples are centred.
    pub sun_pixel: [f32; 2],
    /// The viewport rectangle in the attachment: `x, y, width, height`.
    pub viewport: [f32; 4],
    /// The spacing multiplier for the 16-sample grid, `viewport height / 272`.
    pub pixel_scale: f32,
}

fn truncate_byte(c: f32) -> f32 {
    f32::from((c.clamp(0.0, 1.0) * 255.0) as u8) / 255.0
}

/// `rgb` as hue in sextants `[0, 6)`, saturation, value.
fn to_hsv([r, g, b]: [f32; 3]) -> [f32; 3] {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let hue = if delta <= 0.0 {
        0.0
    } else if max == r {
        ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };
    let saturation = if max <= 0.0 { 0.0 } else { delta / max };
    [hue, saturation, max]
}

fn from_hsv([hue, saturation, value]: [f32; 3]) -> [f32; 3] {
    let h = hue.rem_euclid(6.0);
    let sector = h.floor();
    let f = h - sector;
    let p = value * (1.0 - saturation);
    let q = value * (1.0 - saturation * f);
    let t = value * (1.0 - saturation * (1.0 - f));
    match sector as u32 {
        0 => [value, t, p],
        1 => [q, value, p],
        2 => [p, value, t],
        3 => [p, q, value],
        4 => [t, p, value],
        _ => [value, p, q],
    }
}

/// The ten ring colours: ring 0 is the authored colour; ring `k` is ring `k - 1`'s with
/// its hue moved by `-0.01 k` sextants (`-0.005 k (k + 1)` in total).
#[must_use]
pub fn ring_colours(colour: [f32; 3]) -> [[f32; 3]; RINGS] {
    let mut out = [colour; RINGS];
    for k in 1..RINGS {
        let mut hsv = to_hsv(out[k - 1]);
        hsv[0] = (hsv[0] - 0.01 * k as f32).rem_euclid(6.0);
        out[k] = from_hsv(hsv);
    }
    out
}

/// `1 - angle` between the sun and the camera's look direction, or `None` outside the
/// one-radian cone or behind the camera. `cos` is `dot(direction, look)`.
#[must_use]
pub fn intensity(cos: f32) -> Option<f32> {
    if cos <= 0.0 {
        return None;
    }
    let angle = cos.clamp(-0.999, 0.999).acos();
    (angle <= CONE).then_some(1.0 - angle)
}

/// The ring layout for the sun at `s` (the draw's screen space, see [`ASPECT`]) with
/// `alpha0 = occlusion * intensity` excluded: alphas are returned for occlusion 1.
#[must_use]
pub fn rings(s: [f32; 2], intensity: f32, colour: [f32; 3], random: Random) -> [Ring; RINGS] {
    let length = (s[0] * s[0] + s[1] * s[1]).sqrt();
    let t = length / REACH;
    let fade = if t >= 0.2 { 1.0 } else { 1.0 - (0.2 - t) * 5.0 };
    let colours = ring_colours(colour);
    let to_ndc = |c: [f32; 2], half: f32| {
        let half = half.max(0.0);
        Ring {
            centre: [c[0] / ASPECT, c[1]],
            half: [half / ASPECT, half],
            rgba: [0.0; 4],
            cell: 0,
        }
    };
    std::array::from_fn(|k| {
        let (scale, half, alpha) = if k == 0 {
            (1.0, (REACH - length) * 1.5 + 0.8, intensity)
        } else {
            let n = (k - 1) as i32;
            (
                1.0 - 0.2 * 1.5_f32.powi(n),
                0.2 * t * random.jitter * 1.45_f32.powi(n) * SIZE_CYCLE[(k - 1) & 3],
                intensity * fade * random.ring,
            )
        };
        let mut ring = to_ndc([s[0] * scale, s[1] * scale], half);
        let c = colours[k];
        ring.rgba = [
            truncate_byte(c[0]),
            truncate_byte(c[1]),
            truncate_byte(c[2]),
            truncate_byte(alpha),
        ];
        ring.cell = CELLS[k];
        ring
    })
}

impl Placement {
    /// The flare for a sun at world `direction` seen through `view` and `projection` on
    /// `viewport` (`x, y, width, height` in the attachment), or `None` when it draws
    /// nothing: the sun is behind the camera, outside the cone, or beyond the sampling
    /// margin.
    ///
    /// `view` and `projection` are the unjittered matrices, the same pair the scene is
    /// drawn with.
    #[must_use]
    pub fn new(
        direction: [f32; 3],
        colour: [f32; 3],
        view: Mat4,
        projection: Mat4,
        viewport: [f32; 4],
        random: Random,
    ) -> Option<Self> {
        let [x, y, w, h] = viewport;
        if w < 1.0 || h < 1.0 {
            return None;
        }
        let eye = view.transform_vector3(Vec3::from(direction));
        // The cosine is the sun's component along the camera's third axis as the matrix
        // holds it, not renormalised: the original dots the row it reads, and the live
        // camera's rows are a few parts in a thousand off unit length (0.8649 here, against
        // 0.8848 renormalised, is the measured `0.4736`).
        let intensity = intensity(-eye.z)?;
        let clip = projection * Vec4::new(eye.x, eye.y, eye.z, 0.0);
        if clip.w <= 0.0 {
            return None;
        }
        let ndc = [clip.x / clip.w, clip.y / clip.w];
        let scale = h / ORIGINAL_HEIGHT;
        let sun_pixel = [x + (ndc[0] * 0.5 + 0.5) * w, y + (0.5 - ndc[1] * 0.5) * h];
        let margin = 4.0 * scale;
        let inside = sun_pixel[0] > x - margin
            && sun_pixel[0] < x + w + margin
            && sun_pixel[1] > y - margin
            && sun_pixel[1] < y + h + margin;
        if !inside {
            return None;
        }
        let s = [ndc[0] * ASPECT, ndc[1]];
        let mut rings = rings(s, intensity, colour, random);
        // The layout is in the original's 16:9 screen space, which maps to NDC as `1 / (16/9)`
        // across; the squares of the original are squares on the viewport actually drawn, so a
        // split view (about 3.5:1) or a Deck (16:10) takes its own `h / w`. Chosen, not measured.
        for ring in &mut rings {
            ring.half[0] = ring.half[1] * h / w;
        }
        Some(Self {
            rings,
            sun_pixel,
            viewport,
            pixel_scale: scale,
        })
    }
}
