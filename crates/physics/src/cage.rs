//! The collision cage: the track's invisible tube, a second contact source beside the hull.
//!
//! Pulse keeps every craft inside an elliptical tube that runs along the AI track data, in
//! addition to the collision mesh. `FUN_08831aa4` (reached through `FUN_0883191c` from the
//! ship entity update `FUN_088418e0` at `0x08842384`, after `Body_StepWorld`) locates the
//! craft's nearest control point, builds that point's cross-section, tests the box's eight
//! corners against it and resolves every corner outside it through `Body_ResolveContact`
//! with **friction 0**. Measured live (`docs/ghidra/functions/psp-pulse-usa/collision-cage.md`):
//! the hit our hull misses on `talons-junction-clean-lap.csv` between ticks 1583 and 1584 is
//! a cage contact, its `dv` purely along the cage normal.
//!
//! This module is the geometry and the push. Which control point a craft is over, and the
//! sibling path at a fork, are the caller's (`oag_raceplay`), as the track is.

use oag_core::math::{Quat, Vec3};

use crate::Surface;
use crate::params::Handling;
use crate::ship::{Body, ShipState};
use crate::wall::{self, WallContact};

/// A located control point further than this from the box centre makes no cage at all,
/// and sends the craft to the reset state: `200.0` (`0x43480000`) at `FUN_08831aa4`.
pub const MAX_POINT_DISTANCE: f32 = 200.0;

/// A corner further than this from its boundary point stops the pass, unresolved, and sends
/// the craft to the reset state: `15.0` (`0x41700000`, `DAT_08a7abd8`).
pub const MAX_DEPTH: f32 = 15.0;

/// A contact whose inward normal has a world `y` above this sends the craft to the reset
/// state after it is resolved: the craft is under the tube's floor. `0.5` at `FUN_08831aa4`.
pub const FLOOR_NORMAL_Y: f32 = 0.5;

/// The authored offset of one cage byte: `s * |s| * 250`, `s = byte / 127`
/// (`0.007874016`, `fabsf` at `0x0897e140`).
#[must_use]
pub fn offset(byte: i8) -> f32 {
    let s = f32::from(byte) * 0.007_874_016;
    s * s.abs() * 250.0
}

/// One control point's cross-section, `FUN_08832464`.
///
/// Four extreme points, then a frame: `lateral` from the left extreme to the right one,
/// `vertical` the cross of that with the point's tangent, and `along` re-orthogonalised.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Section {
    /// The left extreme, `pos + lateral * (o0 - 25) + down * (o1 - 2.5)` (`FUN_08831104`).
    pub left: Vec3,
    /// The right extreme, `pos + lateral * (o2 + 25) + down * (o3 - 2.5)` (`FUN_08831288`).
    pub right: Vec3,
    /// The top extreme, `pos + lateral * o4 + down * (o5 - 27)` (`FUN_0883140c`).
    pub top: Vec3,
    /// The bottom extreme, `pos + lateral * o6 + down * (o7 + 33)` (`FUN_0883158c`).
    pub bottom: Vec3,
    /// Unit, from [`Self::left`] to [`Self::right`].
    pub lateral: Vec3,
    /// Unit, `lateral x tangent`.
    pub vertical: Vec3,
    /// Unit, `vertical x lateral`: the tube's free axis.
    pub along: Vec3,
}

impl Section {
    /// The section of a control point: its **lifted** position (`pos - 3 * down`, what the
    /// running game holds), its frame and its eight cage bytes (`SplinePoint::cage`).
    #[must_use]
    pub fn new(position: Vec3, tangent: Vec3, down: Vec3, lateral: Vec3, bytes: [i8; 8]) -> Self {
        let o = bytes.map(offset);
        // `-5.5 + 3.0` and the like are the binary's own two literals, summed in order.
        let left = position + lateral * (o[0] + -25.0) + down * (o[1] + -5.5 + 3.0);
        let right = position + lateral * (o[2] + 25.0) + down * (o[3] + -5.5 + 3.0);
        let top = position + lateral * o[4] + down * (o[5] + -30.0 + 3.0);
        let bottom = position + lateral * o[6] + down * (o[7] + 30.0 + 3.0);
        let across = right - left;
        let vertical = across.cross(tangent);
        let along = vertical.cross(across);
        Self {
            left,
            right,
            top,
            bottom,
            lateral: unit(across),
            vertical: unit(vertical),
            along: unit(along),
        }
    }

    /// `FUN_088327b8`: whether `point` is outside, and the boundary point and inward normal
    /// it is pushed along.
    ///
    /// Four quarter-ellipses in the `(lateral, vertical)` plane: the vertical centre is the
    /// left extreme's height, the lateral centre the top's or bottom's lateral position
    /// (whichever half `point` is in), and each half-axis reaches the extreme on that side.
    /// The boundary point is the radial projection in the ellipse's normalised space, at
    /// `point`'s own position along the tube.
    #[must_use]
    pub fn test(&self, point: Vec3) -> (bool, Vec3, Vec3) {
        let l = point.dot(self.lateral);
        let v = point.dot(self.vertical);
        let u = point.dot(self.along);
        let centre_v = self.left.dot(self.vertical);
        let (centre_l, extreme_v) = if centre_v < v {
            (self.top.dot(self.lateral), self.top.dot(self.vertical))
        } else {
            (
                self.bottom.dot(self.lateral),
                self.bottom.dot(self.vertical),
            )
        };
        let half_v = extreme_v - centre_v;
        let extreme_l = if l < centre_l {
            self.left.dot(self.lateral)
        } else {
            self.right.dot(self.lateral)
        };
        let half_l = extreme_l - centre_l;
        let x = (l - centre_l) / half_l;
        let y = (v - centre_v) / half_v;
        let r2 = x * x + y * y;
        let inverse = 1.0 / r2.sqrt();
        let (xn, yn) = (inverse * x, inverse * y);
        let normal = unit(
            self.lateral * (-(xn * half_v.abs() * half_l) / half_l.abs())
                + self.vertical * (-(yn * half_l.abs() * half_v) / half_v.abs()),
        );
        let boundary = self.lateral * (centre_l + half_l * xn)
            + self.vertical * (centre_v + half_v * yn)
            + self.along * u;
        (r2 > 1.0, boundary, normal)
    }
}

/// `vrsq`-free normalise with the original's zero guard (`vcmp`/`vcmovt` to `1.0`).
fn unit(v: Vec3) -> Vec3 {
    let length = v.length();
    if length == 0.0 { v } else { v / length }
}

/// Where the cage looks: the control point under the craft, and the sibling path's at a
/// fork.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Located {
    /// The located control point's lifted position, for [`MAX_POINT_DISTANCE`].
    pub point: Vec3,
    /// Its cross-section.
    pub section: Section,
    /// The sibling path's nearest section, when the point's path forks or merges
    /// (`AiTrack_LocateOnSiblingPath`, `0x0887d4d4`).
    pub sibling: Option<Section>,
}

/// One corner the cage resolved: where, and how hard, for the contact ring's spark dispatch.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CageContact {
    /// The boundary point the contact was built at.
    pub point: Vec3,
    /// `|p|`, the length of the impulse applied (`Body_RecordContact`'s record).
    pub impulse: f32,
}

/// What the cage did this tick.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CageResponse {
    /// Corners resolved.
    pub contacts: u32,
    /// The first [`Self::contacts`] entries: each resolved corner, in the box's order.
    pub resolved: [CageContact; 8],
    /// The original sends the craft to its reset state (`Ship_SetState(3)` at `0x08842394`):
    /// the point is past [`MAX_POINT_DISTANCE`], a corner past [`MAX_DEPTH`], or a resolved
    /// normal points up past [`FLOOR_NORMAL_Y`].
    pub reset: bool,
    /// Summed velocity change.
    pub velocity_delta: Vec3,
    /// Summed angular velocity change.
    pub angular_velocity_delta: Vec3,
    /// Summed translation.
    pub escape: Vec3,
}

/// The box's eight corners **in the original's order**, from the pose its collider was
/// synced to.
///
/// `Collider_BoxSamplePoints` (`0x08818a00`) writes `(centre - up) + row0 + forward` first,
/// and the body's row 0 is its **left** (`body.right()` is `-row0`). So the order is lower
/// left front, lower left back, lower right front, lower right back, then the same four
/// upper. Read off the live collider at `0x0885001c` on 2,000 ticks, every one the same.
#[must_use]
pub fn corners(position: Vec3, orientation: Quat, handling: &Handling) -> [Vec3; 8] {
    let dimensions = &handling.dimensions;
    let scale = crate::hover::TARGET_GLOBAL_SCALE;
    let up = orientation * Vec3::Y * (dimensions.height * scale * 0.5);
    let forward = orientation * Vec3::NEG_Z * (dimensions.length * scale * 0.5);
    let left = orientation * Vec3::NEG_X * (dimensions.width * scale * 0.5);
    let lower = position - up;
    let upper = position + up;
    [
        lower + left + forward,
        lower + left - forward,
        lower - left + forward,
        lower - left - forward,
        upper + left + forward,
        upper + left - forward,
        upper - left + forward,
        upper - left - forward,
    ]
}

/// `FUN_08831aa4`'s contact pass on one craft.
///
/// `centre`/`orientation` are the pose the box collider was synced to before the hull's
/// contacts (`Body_SyncBoxCollider`, after `Body_Integrate`): the corners and the locate
/// both read it, while the impulses go to `state.body` as the hull left it.
///
/// Per corner, in order: outside the section and, at a fork, outside the sibling's too
/// (inside either is no contact); the nearer of the two boundary points when both; then
/// `Body_ResolveContact` on a contact at that boundary point, inward normal, depth the
/// distance from the corner, friction `0`.
pub fn resolve(
    state: &mut ShipState,
    handling: &Handling,
    centre: Vec3,
    orientation: Quat,
    located: Option<&Located>,
) -> CageResponse {
    let mut response = CageResponse::default();
    let Some(located) = located else {
        return response;
    };
    if (centre - located.point).length() > MAX_POINT_DISTANCE {
        response.reset = true;
        return response;
    }
    for corner in corners(centre, orientation, handling) {
        let (outside, mut boundary, mut normal) = located.section.test(corner);
        if !outside {
            continue;
        }
        if let Some(sibling) = &located.sibling {
            let (outside_sibling, other, other_normal) = sibling.test(corner);
            if !outside_sibling {
                continue;
            }
            if (other - corner).length_squared() < (boundary - corner).length_squared() {
                boundary = other;
                normal = other_normal;
            }
        }
        let depth = (boundary - corner).length();
        if depth > MAX_DEPTH {
            response.reset = true;
            return response;
        }
        let applied = apply(&mut state.body, boundary, normal, depth);
        response.resolved[response.contacts as usize] = CageContact {
            point: boundary,
            impulse: applied.3,
        };
        response.contacts += 1;
        response.velocity_delta += applied.0;
        response.angular_velocity_delta += applied.1;
        response.escape += applied.2;
        if normal.y > FLOOR_NORMAL_Y {
            response.reset = true;
        }
    }
    response
}

/// One cage contact through the hull's own resolver: `Body_ResolveContact` is the same
/// function, called at `0x08831ff0` with a contact whose `+0x34` friction is `0`.
fn apply(body: &mut Body, point: Vec3, normal: Vec3, depth: f32) -> (Vec3, Vec3, Vec3, f32) {
    let contact = WallContact {
        point,
        normal,
        depth,
        surface: Surface::Wall,
        collider: u32::MAX,
    };
    wall::resolve_cage_contact(body, &contact)
}

#[cfg(test)]
mod tests;
