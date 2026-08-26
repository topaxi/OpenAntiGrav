//! How hard the field is to beat.
//!
//! # It degrades a competent driver rather than boosting a weak one
//!
//! The top level is the tuning that was **measured** against the real hulls on
//! a real circuit - see `Tuning::lateral_accel` and `Tuning::max_turn_rate`, both
//! of which carry their sweeps - and every level below it takes something away.
//! The other direction is the one that goes wrong: a baseline tuned for a novice
//! and then multiplied upward has no measurement behind its top end, so the
//! hardest setting is the least tested one, which is exactly backwards.
//!
//! `docs/gameplay/ai.md` sets out the same rule as a six-axis *degradation*
//! vector. Four of those axes exist and are what a level moves here.
//!
//! # What a level may not do
//!
//! **Nothing here reads the player.** The original schedules opponent thrust
//! against the player's race position and the gap to them - `PosBalancing` and
//! `RubberBanding`, both recovered - and this project [refuses to port
//! that](../../docs/gameplay/ai.md#what-we-build-instead). A difficulty setting
//! chooses how good the opponents are *before* the lights, and then they race.
//! An easy field that is easy because it waits for you is not an easy field, it
//! is a rigged one, and the player can feel the difference.

use crate::pilot::Pilot;
use crate::{Span, Tuning};

/// How often a driver at the *easiest* level misses a braking point, per tick
/// of braking. About once a second spent slowing down.
///
/// **It lives here and not in `Tuning`, whose default is zero.** Erring is a
/// degradation, so a caller that has not chosen a difficulty - the closed-loop
/// harness, a replay, a test - gets the competent driver rather than a quietly
/// fallible one.
const MISTAKE_BASE: f32 = 1.0 / 90.0;

/// How good the opponents are.
///
/// Four levels, and the names are this project's own rather than the original's
/// - see [ADR-0006](../../../docs/architecture/adr/0006-no-copyrighted-content.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Difficulty {
    /// Slow, forgiving, and largely uninterested in you.
    Novice,
    /// Quick enough to punish a bad lap.
    Skilled,
    /// The default: quick, and it will defend and shoot.
    #[default]
    Elite,
    /// The measured ceiling, with nothing given back.
    Ace,
}

impl Difficulty {
    /// Every level, easiest first, with the name a config file and a menu spell
    /// it by.
    pub const ALL: [(&'static str, Self); 4] = [
        ("novice", Self::Novice),
        ("skilled", Self::Skilled),
        ("elite", Self::Elite),
        ("ace", Self::Ace),
    ];

    /// The name this level is spelled by.
    #[must_use]
    pub fn name(self) -> &'static str {
        Self::ALL
            .iter()
            .find(|(_, level)| *level == self)
            .map_or("elite", |(name, _)| *name)
    }

    /// A level by name, or `None`.
    ///
    /// **`None` rather than a default**, so a caller reading a config file can
    /// decide whether an unrecognised value is worth a message. `oag-game`'s
    /// settings path falls back and says so; a test would rather fail.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .find(|(spelling, _)| *spelling == name)
            .map(|(_, level)| *level)
    }

    /// How much of the measured cornering grip this level's drivers believe
    /// they have.
    ///
    /// **The biggest single lever, and the one with a measurement behind it.**
    /// `Tuning::lateral_accel` is the fastest a craft can be driven before it
    /// stops cornering and starts sliding; at 55 - a third of it - an opponent
    /// spent 45 per cent of a real race off the throttle. So this scale is not
    /// arbitrary: it walks back down the curve that was swept.
    #[must_use]
    pub fn grip_believed(self) -> f32 {
        match self {
            // **Novice is calibrated on a measurement rather than a fraction.**
            // 0.3 of 180 is about 54, which is what `lateral_accel` was before
            // it was swept - the tuning that was reported from play as "my
            // craft is faster than the AI, first place within a few seconds".
            // That is exactly what a novice level should feel like, and it is
            // the one point on this scale with a play report behind it.
            Self::Novice => 0.30,
            // **Evenly spaced in corner speed, not in grip.** The target goes
            // as the square root of this, so a linear grip scale bunches the
            // hard end together: these four are 0.55, 0.69, 0.84 and 1.0 of the
            // measured corner speed, which is four even steps.
            Self::Skilled => 0.48,
            Self::Elite => 0.70,
            Self::Ace => 1.0,
        }
    }

    /// How much of the turn rate the geometry is allowed to ask for.
    ///
    /// Lowering it is what makes a weaker driver run wide where a better one
    /// tucks back in - the failure that was reported from play on Talon's
    /// Junction, here on purpose.
    #[must_use]
    pub fn turn_allowed(self) -> f32 {
        match self {
            Self::Novice => 0.6,
            Self::Skilled => 0.8,
            Self::Elite => 0.95,
            Self::Ace => 1.0,
        }
    }

    /// How much of a pilot's appetite for weapons, ramming and blocking
    /// survives.
    ///
    /// **Zero at novice, and that is deliberate rather than lazy.** A slow
    /// opponent that still shoots you in the back is not an easy race, it is an
    /// annoying one; the thing a novice wants is to be left alone while they
    /// learn the circuit.
    #[must_use]
    pub fn aggression(self) -> f32 {
        match self {
            Self::Novice => 0.0,
            Self::Skilled => 0.45,
            Self::Elite => 0.8,
            Self::Ace => 1.0,
        }
    }

    /// How long a driver takes to notice a craft arriving beside, in front of
    /// or behind it, in ticks.
    ///
    /// **Zero at Ace, like [`Self::mistakes`]**, or "the measured ceiling, with
    /// nothing given back" would stop being true of it.
    ///
    /// The scale is a human's, because this is the one axis that models a human
    /// rather than a car: a simple visual reaction is about a quarter of a
    /// second, which is fifteen ticks, and that is Skilled. Elite is quicker
    /// than a person because it is already looking, and Novice is four tenths
    /// of a second - somebody still working out where the circuit goes, who
    /// notices the craft alongside once it is properly there.
    ///
    /// **It is a delay, not a blindness.** What it costs is the early part of
    /// a reaction - the lift for a craft closing ahead, the cover for one
    /// coming up behind, the shot at one crossing the cone - which is exactly
    /// the part a better driver has and a worse one does not. `driver/reflex.rs`
    /// is what a channel does while the clock runs.
    #[must_use]
    pub fn reaction_ticks(self) -> u16 {
        match self {
            Self::Novice => 24,
            Self::Skilled => 15,
            Self::Elite => 6,
            Self::Ace => 0,
        }
    }

    /// How often a driver misses a braking point, as a multiplier on the base
    /// rate. Zero means never.
    #[must_use]
    pub fn mistakes(self) -> f32 {
        match self {
            Self::Novice => 1.0,
            Self::Skilled => 0.5,
            Self::Elite => 0.15,
            Self::Ace => 0.0,
        }
    }

    /// This level's tuning, from the measured one.
    #[must_use]
    pub fn tune(self, measured: &Tuning) -> Tuning {
        Tuning {
            lateral_accel: measured.lateral_accel * self.grip_believed(),
            max_turn_rate: measured.max_turn_rate * self.turn_allowed(),
            mistake_rate: MISTAKE_BASE * self.mistakes(),
            reaction_ticks: self.reaction_ticks(),
            ..*measured
        }
    }

    /// This level's version of a pilot.
    ///
    /// Only the axes that decide how a driver treats *other craft* are scaled.
    /// The ones that decide its own line - bias, wander, the inside line - are
    /// left alone, so a shy pilot at novice is still recognisably the same
    /// character rather than a generic slow one.
    #[must_use]
    pub fn temper(self, pilot: &Pilot) -> Pilot {
        let scale = self.aggression();
        let scaled = |span: Span| Span::new(span.low * scale, span.high * scale);
        Pilot {
            trigger: scaled(pilot.trigger),
            ram: scaled(pilot.ram),
            defence: scaled(pilot.defence),
            ..*pilot
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_level_round_trips_through_its_name() {
        for (name, level) in Difficulty::ALL {
            assert_eq!(Difficulty::from_name(name), Some(level));
            assert_eq!(level.name(), name);
        }
        assert_eq!(Difficulty::from_name("impossible"), None);
    }

    /// The scales have to be ordered, or a "harder" setting is not harder.
    #[test]
    fn every_axis_rises_with_the_level() {
        let levels = Difficulty::ALL.map(|(_, level)| level);
        for pair in levels.windows(2) {
            let (easier, harder) = (pair[0], pair[1]);
            assert!(harder.grip_believed() > easier.grip_believed());
            assert!(harder.turn_allowed() > easier.turn_allowed());
            assert!(harder.aggression() > easier.aggression());
            // Mistakes are the one that falls.
            assert!(harder.mistakes() < easier.mistakes());
        }
    }

    /// The top level gives back the measurement untouched, mistakes included:
    /// `Tuning::default()` does not err and neither does an ace. If it ever
    /// scaled the pace, the hardest setting would be the least tested one.
    #[test]
    fn the_top_level_gives_back_the_measured_tuning_exactly() {
        let measured = Tuning::default();
        let ace = Difficulty::Ace.tune(&measured);
        assert_eq!(ace.lateral_accel, measured.lateral_accel);
        assert_eq!(ace.max_turn_rate, measured.max_turn_rate);
        assert_eq!(
            ace, measured,
            "the top level changed something, and it is the measured tuning"
        );
        assert_eq!(
            Difficulty::Ace.temper(&Pilot::AGGRESSIVE),
            Pilot::AGGRESSIVE
        );
        assert_eq!(Difficulty::Ace.mistakes(), 0.0);
    }

    #[test]
    fn a_novice_field_is_slower_and_turns_less_hard() {
        let measured = Tuning::default();
        let easy = Difficulty::Novice.tune(&measured);
        assert!(easy.lateral_accel < measured.lateral_accel);
        assert!(easy.max_turn_rate < measured.max_turn_rate);
        // And everything else is left alone.
        assert_eq!(easy.look_min, measured.look_min);
        assert_eq!(easy.brake_floor, measured.brake_floor);
        assert_eq!(easy.trail_gain, measured.trail_gain);
        // And the novice is the one that errs.
        assert!(easy.mistake_rate > 0.0);
        assert_eq!(Difficulty::Ace.tune(&measured).mistake_rate, 0.0);
    }

    /// A slow opponent that still shoots you in the back is not an easy race.
    #[test]
    fn a_novice_field_leaves_the_player_alone() {
        let calm = Difficulty::Novice.temper(&Pilot::AGGRESSIVE);
        assert_eq!(calm.trigger.high, 0.0);
        assert_eq!(calm.ram.high, 0.0);
        assert_eq!(calm.defence.high, 0.0);
    }

    /// A pilot has to stay itself at every level, or four difficulties give one
    /// generic slow driver and one generic fast one.
    #[test]
    fn a_level_does_not_change_which_pilot_a_craft_is() {
        for (_, level) in Difficulty::ALL {
            for (name, pilot) in Pilot::BUILT_IN {
                let tempered = level.temper(&pilot);
                assert_eq!(tempered.line_bias, pilot.line_bias, "{name}");
                assert_eq!(tempered.wander, pilot.wander, "{name}");
                assert_eq!(tempered.inside, pilot.inside, "{name}");
                assert_eq!(tempered.commitment, pilot.commitment, "{name}");
                assert_eq!(tempered.courtesy, pilot.courtesy, "{name}");
                assert!(tempered.is_well_formed(), "{name} at {}", level.name());
            }
        }
    }

    /// Scaling must not push a pilot past the ceiling that keeps a craft out of
    /// the wall.
    #[test]
    fn no_level_lets_a_pilot_ask_for_more_grip_than_the_hull_has() {
        for (_, level) in Difficulty::ALL {
            for (name, pilot) in Pilot::BUILT_IN {
                assert!(
                    level.temper(&pilot).validated().is_ok(),
                    "{name} at {}",
                    level.name()
                );
            }
        }
    }
}
