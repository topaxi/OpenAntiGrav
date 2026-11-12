//! `--pose-hold`: the player's start pose, put back on the frame a capture writes.
//!
//! A comparison pair poses ours at the original's pose read out of RAM at race clock `T`, then
//! runs the countdown and `T` seconds so the clock, the GO board and the record panel match. A
//! craft resting after GO turns in the original as in ours (bank-to-yaw, see
//! `docs/physics/grid-state.md`), and the original's pose has already turned for those `T`
//! seconds, so running them again turned ours twice. Holding the pose puts ship 0's position and
//! orientation back on the frame that is written, and leaves everything the clock drives alone.

use oag_core::math::{Quat, Vec3};
use oag_raceplay::Race;

/// Ship 0's position and orientation as the race started.
#[derive(Debug, Clone, Copy)]
pub struct StartPose {
    position: Vec3,
    orientation: Quat,
}

impl StartPose {
    /// Ship 0's pose now; taken straight after `Race::start`, it is the resolved `--pose`.
    pub fn of(race: &Race) -> Self {
        let body = &race.sim.world.ships[0].physics.body;
        Self {
            position: body.position,
            orientation: body.orientation,
        }
    }

    /// Puts ship 0 back on this pose, its engine ribbon with it. Position and orientation only:
    /// the craft's own state (its velocity, its hover, the exhaust's ramps) is the clock's and
    /// stays as it ran. The ribbon moves rigidly with the hull ([`Race::place_player`]), so it
    /// hangs behind the nozzle as it did when it was laid down.
    pub fn restore(&self, race: &mut Race) {
        race.place_player(self.position, self.orientation);
    }
}
