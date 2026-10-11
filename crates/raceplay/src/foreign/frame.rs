//! Unity's frame into this engine's: the handedness, the scale, and the frame a
//! section stands in.

use oag_core::math::Vec3;

use super::choice::WORLD_SCALE;

/// A Unity point or vector into this engine: Unity is left-handed Y-up, this
/// engine right-handed Y-up, so one horizontal axis is negated (Z; **chosen**,
/// the other horizontal axis would be as valid). Directions are mirrored only;
/// positions are scaled by [`WORLD_SCALE`] too, by [`world`].
#[must_use]
pub(super) fn mirror(v: [f32; 3]) -> Vec3 {
    Vec3::new(v[0], v[1], -v[2])
}

/// A placed Unity position in this engine's world.
#[must_use]
pub(super) fn world(v: [f32; 3]) -> Vec3 {
    mirror(v) * WORLD_SCALE
}

/// `q` (x, y, z, w) applied to `v`.
#[must_use]
pub(super) fn rotate(q: [f32; 4], v: Vec3) -> Vec3 {
    let u = Vec3::new(q[0], q[1], q[2]);
    v + 2.0 * u.cross(u.cross(v) + q[3] * v)
}

/// The frame of one section, in this engine's world.
#[derive(Debug, Clone, Copy)]
pub(super) struct Frame {
    /// On the floor, at the section's centre.
    pub pos: Vec3,
    /// Up, unit.
    pub up: Vec3,
    /// Along the track, unit and perpendicular to `up`.
    pub forward: Vec3,
    /// `forward x up`, the side `SplinePoint::lateral` points to (measured on
    /// Pulse's `16_Track`: `cross(tangent, down) . lateral = -1`).
    pub right: Vec3,
}

impl Frame {
    /// The frame of a section from its position, rotation and normal.
    ///
    /// Forward is the rotation's `(0,0,1)`, up is the authored normal, and the
    /// forward is made perpendicular to it. `None` for a degenerate normal.
    #[must_use]
    pub(super) fn of(position: [f32; 3], rotation: [f32; 4], normal: [f32; 3]) -> Option<Self> {
        let up = mirror(normal).try_normalize()?;
        let fwd = mirror(rotate(rotation, Vec3::Z).to_array());
        let forward = (fwd - up * fwd.dot(up)).try_normalize()?;
        Some(Self {
            pos: world(position),
            up,
            forward,
            right: forward.cross(up),
        })
    }
}
