//! Wipeout HD's Bomb detonation: the blast object `NormalBomb`'s constructor
//! builds (`0x00144a48` calls `0x00151ad8`, vtable `0x00864ab8`), played back
//! law for law.
//!
//! **Not Pulse's.** Pulse eases two models by `cur += (target - cur) * rate`;
//! HD drives four distinct models (eleven instances) from one per-tick update
//! (`0x001503d8`, with `0x0014fff0` for the ripple rings and `0x00151538` at
//! the start), each setting the model's clock (`AnimNode_UpdateTransformTree`,
//! which the materials' `AlphaAnim`/`V_Anim`/`Shockwave_scalar` are bound to)
//! and a placement matrix. None of the four models carries a moving
//! `Anim Transform` key (`crates/render/examples/hd_weapon_anim_keys.rs`), so
//! every size and fade below is the update's own arithmetic.
//!
//! The constants are the tunable table at `0x008c1aa4`, read out of the
//! executable's initialised data (no writer found; confidence 85), and the
//! curves `0x0014fa08` builds from it. **The numbers were checked by running
//! the executable's own instructions**, not only by reading them: a scratch
//! PowerPC interpreter drove `0x00151538`, `0x001503d8` and `0x001512f8` over
//! random cameras, positions and orientations and this module's arithmetic
//! (`weapons.md`, 2026-10-07) agrees with it to 4e-3 on rows of about ten
//! units - the residue is the executable's own polynomial `sin`/`cos`.
//!
//! # What is read, and what this substitutes
//!
//! - **Read.** The timeline (below), every size, every clock, the ripple
//!   rings' windows, curves and offsets, the camera-facing frames, and the two
//!   effects (`WO_BOMB_SMOKERING` at the start, which the ordinary detonation
//!   trigger already plays; `WO_BOMB_RAYS` once, at 0.5 s).
//! - **Read, with a quirk kept.** The fireball's tilt is a rotation about the
//!   *unnormalised* cross product of its up and the direction to the camera
//!   (`0x006ca538` uses the axis components as given), so the matrix is a
//!   rotation only where the two are perpendicular. It is reproduced as is.
//! - **The camera.** The executable reads a per-viewport camera table
//!   (`0x00987780`). Reading its code, the frame's third axis is that table's
//!   third *column* and the direction to the camera is `position + row 3`;
//!   what the table holds at run time was not read live. This module takes the
//!   column as the camera's **back** axis and the row as `-eye`, which is the
//!   reading under which the executable's own output is a camera-facing frame
//!   (the bloom disc's normal comes out as `-d`, towards the eye). Confidence
//!   65. The frames differ per viewport in the original; this draws for the
//!   one camera [`Race::camera_position`] names.
//! - **Chosen, not measured.** `up`: the original takes the bomb entity's own
//!   matrix row, which this engine's frozen-pose substitute stands in for, on
//!   the same footing as Pulse's (`bomb_blast_basis`). The two point lights the
//!   draw function issues (`0x006778c8`, range 40 and 100 at the blast's
//!   centre three units up) are not played: this engine has no point light for
//!   a weapon to place.
//!
//! # The timeline (seconds from the detonation)
//!
//! | Age | What |
//! | --- | --- |
//! | 0 .. 1.5 | Fireball, white core, bloom disc and the first ring shown. |
//! | 0 .. 0.6 | The first ring grows from 0.1 to 53.33 times its model, its clock running 0..1. |
//! | 0.5 | `WO_BOMB_RAYS`, once. |
//! | 0.75 .. 1.5 | The fireball's clock falls `1 - (age - 0.75)^2`. |
//! | 1.2 .. 1.5 | The three squash: sideways `x (1 + 0.2 t^2)`, up `x (1 - t^2)`. |
//! | 1.4 .. 2.3 | Seven ripple rings, each in its own window, stacked along the up axis. |
//! | 3.0 | The object retires. |

use super::*;

/// The blast object's lifetime: `0x001503d8` returns false once `age >` this
/// (the table's `+0x00`).
pub(super) const LIFETIME_SECONDS: f32 = 3.0;

/// The end of the first phase (`+0x64`): the first four models hide here.
const PHASE_ONE_END_SECONDS: f32 = 1.5;

/// How many ripple rings follow the first (`0x0014fff0`'s loop of seven).
pub(in crate::bomb_blast) const RIPPLES: usize = 7;

/// `WO_BOMB_RAYS` is spawned the first tick `age` exceeds this (`-0x23b4`).
const RAYS_AT_SECONDS: f32 = 0.5;

/// The fireball's size curve, `(age, value)` keys (`0x14fa08`: `2.0` at 0,
/// `6.66` at `+0x60` = 0.1, `10.0` at the phase end), linear between.
const SIZE_KEYS: [(f32, f32); 3] = [(0.0, 2.0), (0.1, 6.66), (PHASE_ONE_END_SECONDS, 10.0)];

/// The size's second term eases `cur += (target - cur) * rate` once a tick
/// from `START` (`+0x60`) towards `TARGET` (`+0x5c`) at `RATE` (`-0x23e8`).
const EASE_START: f32 = 0.1;
const EASE_TARGET: f32 = 3.33;
const EASE_RATE: f32 = 0.05;

/// The white core is the fireball scaled by this (`-0x2360`).
const WHITE_SCALE: f32 = 0.99;
/// The white core's `ColourAnim` (`+0xac`), where the fireball's is animated.
const WHITE_COLOUR_ANIM: f32 = 0.9;

/// The first ring's size curve: `0.1` to `53.33` over `0.6` s (`+0x68`).
const RING_SIZE: (f32, f32, f32) = (0.1, 53.33, 0.6);

/// The ripple rings, `[start, window, radius at start, radius at the end of
/// `start` seconds, offset along up]` per ring, from the table's lists at
/// `+0x24`, `+0x88`, `+0x08`, `+0x40` and `+0x6c`. The radius curve runs
/// `radius_from -> radius_to` over `start` seconds (its second key time is the
/// start, `0x14fa08`), of which only the window is seen.
const RIPPLE: [[f32; 5]; RIPPLES] = [
    [1.4, 0.6, 1.33, 2.0, -5.33],
    [1.42, 0.7, 3.33, 5.33, -4.0],
    [1.43, 0.8, 5.33, 8.66, -2.0],
    [1.45, 0.9, 8.0, 10.66, 0.0],
    [1.43, 0.8, 5.33, 8.66, 2.0],
    [1.42, 0.7, 3.33, 5.33, 4.0],
    [1.4, 0.6, 1.33, 2.0, 5.33],
];

/// One placed model: where, and what its materials read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Piece {
    pub(crate) matrix: Mat4,
    /// The model's clock: `AlphaAnim`, `V_Anim` and `Shockwave_scalar`.
    pub(crate) clock: f32,
    /// `ColourAnim`, for the two fireballs; `0.0` for the rest.
    pub(crate) colour: f32,
}

/// Everything one blast shows this frame.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Pieces {
    pub(crate) fireball: Option<Piece>,
    pub(crate) core: Option<Piece>,
    pub(crate) bloom: Option<Piece>,
    pub(crate) ring: Option<Piece>,
    pub(crate) ripples: [Option<Piece>; RIPPLES],
}

/// One live blast. **View state**, on the same terms as every blast here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct HdBlast {
    pub(crate) position: Vec3,
    /// The bomb's up axis (see the module doc: chosen).
    pub(crate) up: Vec3,
    pub(crate) age: f32,
    /// The fireball size's eased term.
    cur: f32,
    rays_sent: bool,
}

/// What one tick of a blast asks of the caller.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Step {
    pub(crate) retire: bool,
    /// Play `WO_BOMB_RAYS` at the blast, this tick.
    pub(crate) rays: bool,
}

impl HdBlast {
    pub(crate) fn new(position: Vec3, up: Vec3) -> Self {
        Self {
            position,
            up: up.try_normalize().unwrap_or(Vec3::Y),
            age: 0.0,
            cur: EASE_START,
            rays_sent: false,
        }
    }

    /// One tick: age, the size ease and the rays trigger. `0x001503d8`.
    ///
    /// The ease steps `trunc(dt * 59.999996)` times in the original, which is
    /// one at this engine's fixed 60 Hz.
    pub(crate) fn step(&mut self, dt: f32) -> Step {
        self.age += dt;
        if self.age > LIFETIME_SECONDS {
            return Step {
                retire: true,
                rays: false,
            };
        }
        if self.age < PHASE_ONE_END_SECONDS {
            self.cur += (EASE_TARGET - self.cur) * EASE_RATE;
        }
        let rays = self.age > RAYS_AT_SECONDS
            && self.age < PHASE_ONE_END_SECONDS
            && !std::mem::replace(&mut self.rays_sent, true);
        Step {
            retire: false,
            rays,
        }
    }

    /// The models shown at the current age, for a camera at `eye` whose back
    /// axis (towards the viewer) is `back`.
    pub(crate) fn pieces(&self, eye: Vec3, back: Vec3) -> Pieces {
        let age = self.age;
        let mut out = Pieces::default();
        let at = |x: Vec3, y: Vec3, z: Vec3, position: Vec3| {
            Mat4::from_cols(x.extend(0.0), y.extend(0.0), z.extend(0.0), position.extend(1.0))
        };
        // The bomb's own frame (`Start`'s matrix), for the rings.
        let [side, up, fwd] = bomb_frame(self.up);
        if age < PHASE_ONE_END_SECONDS {
            let ring_t = (age / RING_SIZE.2).min(1.0);
            let ring_scale = RING_SIZE.0 + (RING_SIZE.1 - RING_SIZE.0) * ring_t;
            out.ring = Some(Piece {
                matrix: at(
                    side * ring_scale,
                    up * ring_scale,
                    fwd * ring_scale,
                    self.position,
                ),
                clock: ring_t,
                colour: 0.0,
            });
            let [bx, by, bz] = camera_frame(self.up, back);
            let d = (self.position - eye).try_normalize().unwrap_or(-back);
            let axis = by.cross(d);
            let tilt = (1.0 - age / LIFETIME_SECONDS) * -std::f32::consts::FRAC_PI_2
                + std::f32::consts::FRAC_PI_4;
            let size = piecewise(&SIZE_KEYS, age) + self.cur;
            let (sx, sy) = if age > PHASE_ONE_END_SECONDS - 0.3 {
                let t = (age - (PHASE_ONE_END_SECONDS - 0.3)) * (1.0 / 0.3);
                (1.0 + 0.2 * t * t, 1.0 - t * t)
            } else {
                (1.0, 1.0)
            };
            let fire = [bx, by, bz].map(|row| rotate_row(row, axis, tilt));
            let fire_clock = if age > 0.75 {
                (1.0 - (age - 0.75) * (age - 0.75)).max(0.0)
            } else {
                1.0
            };
            let colour = if age <= 0.1 {
                1.0
            } else {
                let x = (age - 0.1) / (PHASE_ONE_END_SECONDS - 0.1) * 2.0 - 1.0;
                if x < 0.0 { -x } else { x.powf(0.25) }
            };
            let sized = |rows: [Vec3; 3], k: f32, position: Vec3| {
                at(
                    rows[0] * (size * sx * k),
                    rows[1] * (size * sy * k),
                    rows[2] * (size * k),
                    position,
                )
            };
            out.fireball = Some(Piece {
                matrix: sized(fire, 1.0, self.position),
                clock: fire_clock,
                colour,
            });
            out.core = Some(Piece {
                matrix: sized(fire, WHITE_SCALE, self.position),
                clock: fire_clock,
                colour: WHITE_COLOUR_ANIM,
            });
            // The bloom disc faces the viewer: `z = -d`, `y` the up with the
            // viewing direction removed, `x = -(up x d)`.
            let disc_y = (by - d * by.dot(d)).normalize_or_zero();
            let disc_x = -by.cross(d).normalize_or_zero();
            let bloom_clock = if age <= 0.8 {
                age / 0.8 * 0.5
            } else {
                let s = (age - 0.8) / (PHASE_ONE_END_SECONDS - 0.8) * 0.5;
                s * s + 0.5
            };
            out.bloom = Some(Piece {
                matrix: sized([disc_x, disc_y, -d], 1.0, self.position),
                clock: bloom_clock,
                colour: 0.0,
            });
        }
        for (k, [start, window, from, to, offset]) in RIPPLE.into_iter().enumerate() {
            if !(age > start && age < start + window) {
                continue;
            }
            let into = age - start;
            let u = into / window;
            let radius = from + (to - from) * (into / start);
            // The height factor starts negative (an inside-out ring) and
            // crosses zero a little before 0.4 of the window.
            let height = radius * (1.0 - 25.0 * (1.0 - u).powi(4) + 2.0);
            out.ripples[k] = Some(Piece {
                matrix: at(
                    side * radius,
                    up * height,
                    fwd * radius,
                    self.position - up * offset,
                ),
                clock: u,
                colour: 0.0,
            });
        }
        out
    }
}

/// The bomb's own orthonormal frame `[side, up, forward]`, what the executable
/// reads off the bomb entity's matrix and the first ring is scaled from. The
/// forward axis is the engine's `Z` against which `up` is orthogonalised,
/// exactly as [`bomb_blast_basis`] builds Pulse's.
fn bomb_frame(up: Vec3) -> [Vec3; 3] {
    let basis = bomb_blast_basis(Vec3::ZERO, up);
    [
        basis.x_axis.truncate(),
        basis.y_axis.truncate(),
        basis.z_axis.truncate(),
    ]
}

/// `Start`'s camera frame: `z` the camera's back axis, `y` the bomb's up with
/// `z` removed, `x = y x z` (`0x00151538`).
fn camera_frame(up: Vec3, back: Vec3) -> [Vec3; 3] {
    let z = back.normalize_or_zero();
    let y = (up - z * up.dot(z)).normalize_or_zero();
    if y == Vec3::ZERO || z == Vec3::ZERO {
        let any = z.try_normalize().unwrap_or(Vec3::Z);
        let y = any.any_orthonormal_vector();
        return [y.cross(any), y, any];
    }
    [y.cross(z), y, z]
}

/// A row vector times the rotation matrix `0x006ca538` builds from a **raw**
/// (not necessarily unit) axis `a` and angle `t`: rows
/// `(x^2 k + c, xy k - z s, xz k + y s)`, `(xy k + z s, y^2 k + c, yz k - x s)`,
/// `(xz k - y s, yz k + x s, z^2 k + c)` with `k = 1 - c`.
fn rotate_row(v: Vec3, a: Vec3, t: f32) -> Vec3 {
    let (s, c) = t.sin_cos();
    let k = 1.0 - c;
    let r0 = Vec3::new(a.x * a.x * k + c, a.x * a.y * k - a.z * s, a.x * a.z * k + a.y * s);
    let r1 = Vec3::new(a.x * a.y * k + a.z * s, a.y * a.y * k + c, a.y * a.z * k - a.x * s);
    let r2 = Vec3::new(a.x * a.z * k - a.y * s, a.y * a.z * k + a.x * s, a.z * a.z * k + c);
    r0 * v.x + r1 * v.y + r2 * v.z
}

/// Linear interpolation through `keys` (ascending in `x`), held at the ends.
fn piecewise(keys: &[(f32, f32)], x: f32) -> f32 {
    let Some(&(first_x, first)) = keys.first() else {
        return 0.0;
    };
    if x <= first_x {
        return first;
    }
    for pair in keys.windows(2) {
        let [(x0, y0), (x1, y1)] = [pair[0], pair[1]];
        if x <= x1 {
            return y0 + (y1 - y0) * (x - x0) / (x1 - x0);
        }
    }
    keys.last().map_or(0.0, |&(_, y)| y)
}

#[cfg(test)]
mod tests;
