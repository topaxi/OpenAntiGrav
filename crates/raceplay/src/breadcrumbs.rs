//! Where the original's reset state puts a craft back, and how fast: a ring of
//! grounded positions, and the relocation that reads it.
//!
//! A craft put through `Ship_SetState(3)` (a `Reset` contact, or four seconds
//! with no hover probe touching) runs `FUN_0883ff6c` once. Read in the PSP
//! `BOOT.BIN` (`docs/ghidra/functions/psp-pulse-usa/reset-relocation.md`):
//!
//! ```text
//! FUN_088418e0, every frame (0x08841c00):
//!   if grounded == 1.0:                      ; craft+0x94 -> +0x2b0, against 1.0f
//!       FUN_0883d9ec(craft, pose)            ; lay a crumb, below
//!       craft+0x78c = body+0x398             ; |velocity|, latched
//! FUN_0883d9ec:
//!   if !(located.flags & 1):                 ; craft+0xaf0 + 0x61
//!       if any axis of pose.pos - newest crumb is past +/-20:
//!           FUN_0883d960(craft, pose)        ; push onto an 8-slot ring
//! FUN_0883ff6c (state 3's update), once:
//!   crumb = FUN_0883ef30(craft)              ; the OLDEST crumb; head = tail + 1
//!   at = AiTrack_LocatePosition(crumb + 5 up)
//!   mid = at.pos + at.lateral * (half_width_right - half_width_left) * 0.5
//!   aim = locate(locate(at.pos + at.tangent * 40).pos + tangent * 40).pos
//!   pose = { pos: mid + 5 up, forward: normalize(aim - mid), up: -at.down }
//!   FUN_0883db20(craft+0x78c * 0.25 + 50, craft, pose)   ; velocity = forward * that
//! ```
//!
//! So a craft that falls into a gap comes back on the road it was last
//! grounded on, about six crumbs (120 units or more) behind, moving. Verified
//! live on PPSSPP: nine resets of nine on the oldest crumb at that speed, and
//! on `13_Track` row 2949 every time, where this port puts the parked field at
//! 2939 and 2950. The disc
//! decides where no crumb may be laid: `flags` bit 0 is authored around the
//! gaps of 13, 10, 14, 25, 26, 29 and 30_Track, so a craft cannot be put back
//! over the hole it fell into.
//!
//! This project used to put the craft at rest on the racing line at the
//! sample nearest where it fell (confidence 40, a placeholder), which on
//! `13_Track` is over the gap: it fell straight back in, 389 times in 9,000
//! ticks for the original driver's field with the player parked.
//!
//! Three departures, recorded rather than hidden: the locate is this
//! project's nearest spline sample rather than the original's B-spline
//! projection; a crumb is laid after this tick's step from this tick's
//! groundedness, where the original's order between `Ship_UpdateCraft` and
//! `FUN_088418e0` was not traced; and [`oag_gameplay::Ship::place_at`] zeroes
//! the angular velocity, which `FUN_0883db20` does not touch.

use oag_core::math::Vec3;
use oag_gameplay::{Pose, orientation_on_sample};
use oag_vex::track::{HOVER_LIFT, Sample};

use crate::spline::Spline;

/// How many crumbs the ring holds, counting the slot the next push writes:
/// `FUN_0883d960` wraps `craft+0x780` at `8` and `FUN_0883ef30` the same.
pub const CRUMB_SLOTS: usize = 8;

/// How far the craft must be from the newest crumb, **on any one world axis**,
/// before another is laid: `FUN_0883d9ec`'s `20.0 < d || d < -20.0` on x, y
/// and z separately.
pub const CRUMB_SPACING: f32 = 20.0;

/// The `flags` bit of the craft's located spline record that stops a crumb
/// being laid: `FUN_0883d9ec` tests `craft+0xb51 & 1`, and `0xb51` is byte
/// `+0x61` of the `SplinePt` record at `craft+0xaf0`.
pub const NO_CRUMB_FLAG: u8 = 0x01;

/// The lift on the crumb before it is located, and on the pose after: both
/// `5.0` world-`y` (`0x40a00000`), the first added to the crumb's `y`, the
/// second as the vector `(0, 5, 0)`.
pub const RESET_LIFT: f32 = 5.0;

/// The step down the tangent, taken twice, to the point the reset faces:
/// `40.0` (`0x42200000`, loaded once at `0x08840160` and used by both
/// `vscl.q` steps).
pub const RESET_AIM_STEP: f32 = 40.0;

/// The launch speed with no grounded speed latched: `+ 50.0` in
/// `FUN_0883db20`'s first argument.
pub const RESET_LAUNCH_BASE: f32 = 50.0;

/// The share of the last fully grounded speed the launch carries: `* 0.25`.
pub const RESET_LAUNCH_SHARE: f32 = 0.25;

/// One craft's crumbs: `craft+0x370 + i * 0x40` (the position row of each
/// pose), the write head `craft+0x780`, the oldest `craft+0x784`, and the
/// latched speed `craft+0x78c`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Breadcrumbs {
    positions: [Vec3; CRUMB_SLOTS],
    head: u8,
    tail: u8,
    grounded_speed: f32,
}

impl Breadcrumbs {
    /// A ring holding one crumb, where the craft was placed:
    /// `Race_PlaceGrid` pushes each craft's grid pose (`0x08827dc8`).
    #[must_use]
    pub fn seeded(position: Vec3) -> Self {
        let mut crumbs = Self::default();
        crumbs.push(position);
        crumbs
    }

    /// `FUN_0883d960`: write at the head, advance it, and when it lands on the
    /// oldest, drop the oldest. So seven crumbs survive, not eight.
    pub fn push(&mut self, position: Vec3) {
        self.positions[usize::from(self.head)] = position;
        self.head = Self::next(self.head);
        if self.head == self.tail {
            self.tail = Self::next(self.tail);
        }
    }

    /// The crumb laid last, the one the spacing is measured from. On a ring
    /// with nothing pushed it is slot 7, as the original reads `craft+0x570`.
    #[must_use]
    pub fn newest(&self) -> Vec3 {
        self.positions[(usize::from(self.head) + CRUMB_SLOTS - 1) % CRUMB_SLOTS]
    }

    /// `FUN_088418e0`'s grounded block, after this tick's step: on a tick
    /// with both probes down, lay a crumb unless the located record forbids
    /// it or the craft is within [`CRUMB_SPACING`] of the last one on every
    /// axis, and latch the speed either way.
    pub fn lay(&mut self, position: Vec3, speed: f32, grounded: f32, flags: u8) {
        if grounded != 1.0 {
            return;
        }
        if flags & NO_CRUMB_FLAG == 0 {
            let away = position - self.newest();
            if away.x.abs() > CRUMB_SPACING
                || away.y.abs() > CRUMB_SPACING
                || away.z.abs() > CRUMB_SPACING
            {
                self.push(position);
            }
        }
        self.grounded_speed = speed;
    }

    /// `FUN_0883ef30`: the oldest crumb, and every newer one forgotten (the
    /// head moves to just past it), so a second reset before the craft lays
    /// another comes back to the same place.
    pub fn take_oldest(&mut self) -> Vec3 {
        let crumb = self.positions[usize::from(self.tail)];
        self.head = Self::next(self.tail);
        crumb
    }

    /// `craft+0x78c * 0.25 + 50`, the speed a reset launches the craft at.
    #[must_use]
    pub fn launch_speed(&self) -> f32 {
        self.grounded_speed * RESET_LAUNCH_SHARE + RESET_LAUNCH_BASE
    }

    /// Every field, for [`crate::RaceSim`]'s state hash: a crumb laid on a
    /// different tick is a reset somewhere else.
    pub(crate) fn hash_into(&self, hasher: &mut oag_core::hash::StateHasher) {
        for position in self.positions {
            hasher.write_vec3(position);
        }
        hasher.write_u8(self.head);
        hasher.write_u8(self.tail);
        hasher.write_f32(self.grounded_speed);
    }

    fn next(slot: u8) -> u8 {
        (slot + 1) % CRUMB_SLOTS as u8
    }
}

/// Where and how a reset puts a craft back.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Relocation {
    /// The pose: the width midpoint five up, facing the aim point.
    pub pose: Pose,
    /// The launch velocity, along the pose's forward.
    pub velocity: Vec3,
}

/// `FUN_0883ff6c`'s relocation from `crumb`, launched at `speed`.
///
/// `None` on a spline with no samples, which leaves the caller its fallback.
#[must_use]
pub fn relocate(spline: &Spline, crumb: Vec3, speed: f32) -> Option<Relocation> {
    let at = locate(spline, crumb + Vec3::new(0.0, RESET_LIFT, 0.0))?;
    let lateral = Vec3::from_array(at.lateral);
    let mid = lifted(at) + lateral * ((at.half_width_right - at.half_width_left) * 0.5);
    let mut aim = at;
    for _ in 0..2 {
        aim = locate(
            spline,
            lifted(aim) + Vec3::from_array(aim.tangent) * RESET_AIM_STEP,
        )?;
    }
    let pose = Pose {
        position: mid + Vec3::new(0.0, RESET_LIFT, 0.0),
        orientation: orientation_on_sample(at),
    }
    .facing(lifted(aim) - mid);
    let forward = pose.orientation * Vec3::NEG_Z;
    Some(Relocation {
        pose,
        velocity: forward * speed,
    })
}

/// The spline sample nearest `position`, standing in for
/// `AiTrack_LocatePosition`'s projection.
fn locate(spline: &Spline, position: Vec3) -> Option<&Sample> {
    spline.nearest(position).map(|(_, sample, _)| sample)
}

/// A located record's position is on the lifted curve
/// (`docs/formats/track.md`, "the lift"); a [`Sample`] is on the surface line.
fn lifted(sample: &Sample) -> Vec3 {
    Vec3::from_array(sample.pos) - Vec3::from_array(sample.down) * HOVER_LIFT
}

#[cfg(test)]
mod tests;
