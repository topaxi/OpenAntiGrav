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

/// When a posed capture puts ship 0 back on its start pose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Hold {
    /// Never: the craft flies from its pose.
    #[default]
    Off,
    /// `--pose-hold`: on the frame that is written, after the ticks have run.
    LastFrame,
    /// `--pose-hold-ticks`: after every tick as well, at rest. A weapon fired from a resting
    /// craft then leaves along the pose's own heading instead of the heading the craft has
    /// turned to by the shot (`docs/ghidra/functions/ps3-hdfury-eu/plasma.md`, "The pairs").
    EveryTick,
}

impl Hold {
    /// The mode `--pose-hold` and `--pose-hold-ticks` ask for.
    #[must_use]
    pub fn from_flags(last_frame: bool, every_tick: bool) -> Self {
        match (last_frame, every_tick) {
            (_, true) => Self::EveryTick,
            (true, false) => Self::LastFrame,
            (false, false) => Self::Off,
        }
    }

    /// Ship 0's pose as the race started, when this mode puts it back at all.
    #[must_use]
    pub fn start(self, race: &Race) -> Option<StartPose> {
        (self != Self::Off).then(|| StartPose::of(race))
    }
}

/// Ship 0's position and orientation as the race started.
#[derive(Debug, Clone, Copy)]
pub struct StartPose {
    position: Vec3,
    orientation: Quat,
}

impl StartPose {
    /// Ship 0's pose now; taken straight after `Race::start`, it is the resolved `--pose`.
    pub fn of(race: &Race) -> Self {
        let body = &race.core.sim.world.ships[0].physics.body;
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

    /// [`Self::restore`], and the body brought to rest: a craft put back every tick would
    /// otherwise keep the velocity its hover gathered while it was moved away.
    pub fn restore_at_rest(&self, race: &mut Race) {
        self.restore(race);
        let body = &mut race.core.sim.world.ships[0].physics.body;
        body.linear_velocity = Vec3::ZERO;
        body.angular_velocity = Vec3::ZERO;
    }
}

#[cfg(test)]
mod tests {
    use super::Hold;

    #[test]
    fn the_every_tick_flag_wins_and_neither_flag_holds_nothing() {
        assert_eq!(Hold::from_flags(false, false), Hold::Off);
        assert_eq!(Hold::from_flags(true, false), Hold::LastFrame);
        assert_eq!(Hold::from_flags(true, true), Hold::EveryTick);
        assert_eq!(Hold::from_flags(false, true), Hold::EveryTick);
        assert_eq!(Hold::default(), Hold::Off);
    }
}
