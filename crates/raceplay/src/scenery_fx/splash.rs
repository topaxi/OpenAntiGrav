//! The enhanced weather's splashes: **this project's own, chosen, not
//! measured.** The original's rain and snow never touch geometry
//! (`docs/ghidra/functions/psp-pulse-usa/weather.md`: `Modifier_WrapBox` only
//! wraps and blows them); with the weather setting at `enhanced` a drop that
//! meets the track throws a ring and a few droplets, and a snowflake that
//! meets it settles and fades.
//!
//! **Presentation, not simulation.** Nothing here writes `World`. The hits are
//! found by [`crate::Race`] from the pool's drawn positions with the same
//! read-only floor cast the blob shadows use (`crate::shadow`), once a tick,
//! and nothing runs at all at `original`, so that frame stays byte-identical.
//!
//! **Art.** The ring and the dot are cells of one `OAGMASK` this project
//! draws (`assets/fallback/Data/Psys/Tex/OAGSplash.oagmask`, made by
//! `scripts/gen-fallback-splash-art.py`), served by the fallback asset layer
//! and placed on the particle sheet after every effect's own sprite. The disc
//! has no splash sprite and none is borrowed.

use oag_core::Rng;
use oag_core::math::Vec3;
use oag_fx::psys::sprite::map_to_cell;
use oag_mesh::mesh::GpuVertex;

/// The fallback layer's name for the art.
pub const ART: &str = r"Data\Psys\Tex\OAGSplash.oagmask";

/// Splashes alive at once. Past it a hit is dropped: the cost stays bounded.
pub const CAPACITY: usize = 48;

/// Droplets a rain splash throws.
const DROPLETS: usize = 3;

/// Seed for the splashes' own draws, apart from the weather's so that
/// `enhanced` never moves the original's drop stream. Any constant: render
/// state, never hashed.
const SEED: u64 = 0x5b1a_5e07;

/// Seconds a rain splash lasts, and the ring's radius at its start and end.
const RAIN_LIFE: f32 = 0.35;
const RING_RADIUS: (f32, f32) = (0.125, 0.65);

/// Every length here is a quarter of the first version's (the maintainer's play
/// report asked for much smaller drops); speeds and gravity scale with them so
/// the flight takes the same time.
///
/// A droplet's launch speed along the surface normal and across it, and the
/// gravity pulling it back, in world units a second.
const SPRAY_UP: (f32, f32) = (1.75, 3.0);
const SPRAY_OUT: f32 = 1.0;
const GRAVITY: f32 = 13.75;

/// A droplet's and a settled flake's half-size, in world units.
const DROPLET_SIZE: f32 = 0.075;
const FLAKE_SIZE: f32 = 0.105;

/// Seconds a flake stays settled, with the share of it spent fading in and out.
const SNOW_LIFE: f32 = 1.6;
const SNOW_FADE: f32 = 0.25;

/// How far off the surface a flat ring or flake is drawn, so it does not
/// z-fight with the road it lies on.
const LIFT: f32 = 0.03;

/// The ring's cell and the dot's, as the share of the placed sprite's width
/// they take: the art is two square cells side by side.
const RING_CELL: (f32, f32) = (0.0, 0.5);
const DOT_CELL: (f32, f32) = (0.5, 1.0);

#[derive(Debug, Clone, Copy)]
struct Splash {
    at: Vec3,
    normal: Vec3,
    sprays: [Vec3; DROPLETS],
    age: f32,
    snow: bool,
}

impl Splash {
    fn life(&self) -> f32 {
        if self.snow { SNOW_LIFE } else { RAIN_LIFE }
    }
}

/// The splashes alive now.
#[derive(Debug, Clone)]
pub struct Splashes {
    live: Vec<Splash>,
    rng: Rng,
}

impl Default for Splashes {
    fn default() -> Self {
        Self {
            live: Vec::with_capacity(CAPACITY),
            rng: Rng::new(SEED),
        }
    }
}

impl Splashes {
    /// How many are alive.
    #[must_use]
    pub fn len(&self) -> usize {
        self.live.len()
    }

    /// Whether none is.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.live.is_empty()
    }

    /// Where each live splash lies on the track.
    pub fn points(&self) -> impl Iterator<Item = Vec3> + '_ {
        self.live.iter().map(|splash| splash.at)
    }

    /// Drops every splash, as the setting going back to `original` does.
    pub fn clear(&mut self) {
        self.live.clear();
    }

    /// Ages every splash `dt` seconds and removes the finished.
    pub fn advance(&mut self, dt: f32) {
        for splash in &mut self.live {
            splash.age += dt;
        }
        self.live.retain(|splash| splash.age < splash.life());
    }

    /// A drop (or flake, when `snow`) met the track at `at`, whose surface faces `normal`.
    pub fn spawn(&mut self, at: Vec3, normal: Vec3, snow: bool) {
        if self.live.len() >= CAPACITY {
            return;
        }
        let normal = normal.normalize_or_zero();
        let (tangent, bitangent) = basis(normal);
        let mut sprays = [Vec3::ZERO; DROPLETS];
        if !snow {
            for spray in &mut sprays {
                let turn = self.rng.next_f32() * std::f32::consts::TAU;
                let up = SPRAY_UP.0 + (SPRAY_UP.1 - SPRAY_UP.0) * self.rng.next_f32();
                let out = SPRAY_OUT * self.rng.next_f32();
                *spray = normal * up + (tangent * turn.cos() + bitangent * turn.sin()) * out;
            }
        }
        self.live.push(Splash {
            at,
            normal,
            sprays,
            age: 0.0,
            snow,
        });
    }

    /// Appends the geometry: rings flat on the track and snow into `additive`
    /// and the droplets into `alpha_over`, camera-facing. `cell` is the art's
    /// rectangle on the particle sheet.
    pub fn extend_vertices(
        &self,
        additive: &mut Vec<GpuVertex>,
        alpha_over: &mut Vec<GpuVertex>,
        right: Vec3,
        up: Vec3,
        cell: [f32; 4],
    ) {
        let part = |range: (f32, f32)| {
            let width = cell[2] - cell[0];
            [
                cell[0] + width * range.0,
                cell[1],
                cell[0] + width * range.1,
                cell[3],
            ]
        };
        let ring = part(RING_CELL);
        let dot = part(DOT_CELL);
        for splash in &self.live {
            let t = splash.age / splash.life();
            if splash.snow {
                let fade = (t / SNOW_FADE).min((1.0 - t) / SNOW_FADE).clamp(0.0, 1.0);
                let lifted = splash.at + splash.normal * (LIFT + FLAKE_SIZE * 0.5);
                additive.extend(quad(
                    lifted,
                    right * FLAKE_SIZE,
                    up * FLAKE_SIZE,
                    [1.0; 3],
                    0.85 * fade,
                    dot,
                ));
                continue;
            }
            let fade = 1.0 - t;
            let radius = RING_RADIUS.0 + (RING_RADIUS.1 - RING_RADIUS.0) * t;
            let (tangent, bitangent) = basis(splash.normal);
            additive.extend(quad(
                splash.at + splash.normal * LIFT,
                tangent * radius,
                bitangent * radius,
                [0.8, 0.9, 1.0],
                0.55 * fade * fade,
                ring,
            ));
            let seconds = splash.age;
            for spray in &splash.sprays {
                let height = seconds * seconds * -0.5 * GRAVITY;
                let at = splash.at + *spray * seconds + splash.normal * (height + LIFT);
                // A droplet that has fallen back below the surface is gone.
                if (at - splash.at).dot(splash.normal) < 0.0 {
                    continue;
                }
                alpha_over.extend(quad(
                    at,
                    right * DROPLET_SIZE,
                    up * DROPLET_SIZE,
                    [0.85, 0.93, 1.0],
                    0.8 * fade,
                    dot,
                ));
            }
        }
    }
}

/// Two unit vectors that, with `normal`, make an orthonormal frame.
fn basis(normal: Vec3) -> (Vec3, Vec3) {
    let axis = if normal.x.abs() < 0.9 {
        Vec3::X
    } else {
        Vec3::Z
    };
    let tangent = normal.cross(axis).normalize_or_zero();
    (tangent, normal.cross(tangent))
}

/// Six vertices, two triangles, of a quad sampling `cell` of the sheet.
pub(super) fn quad(
    centre: Vec3,
    right: Vec3,
    up: Vec3,
    rgb: [f32; 3],
    alpha: f32,
    cell: [f32; 4],
) -> [GpuVertex; 6] {
    let corner = |sx: f32, sy: f32, u: f32, v: f32| GpuVertex {
        position: (centre + right * sx + up * sy).to_array(),
        normal: [0.0, 0.0, 1.0],
        colour: [rgb[0], rgb[1], rgb[2], alpha],
        texcoord: [u, v],
        lit: 0.5,
        ..bytemuck::Zeroable::zeroed()
    };
    let bl = corner(-1.0, -1.0, 0.0, 1.0);
    let br = corner(1.0, -1.0, 1.0, 1.0);
    let tl = corner(-1.0, 1.0, 0.0, 0.0);
    let tr = corner(1.0, 1.0, 1.0, 0.0);
    let mut corners = [bl, br, tl, br, tr, tl];
    map_to_cell(&mut corners, cell);
    corners
}

#[cfg(test)]
mod tests;
