//! How hard the field is to beat.
//!
//! # It degrades a competent driver rather than boosting a weak one
//!
//! The top level is the tuning **measured** against the real hulls on a real
//! circuit (`Tuning::lateral_accel`, `Tuning::max_turn_rate`) and every level
//! below takes something away. A baseline tuned for a novice and multiplied
//! upward has no measurement behind its top end. `docs/gameplay/ai.md` states
//! the rule as a six-axis degradation vector; four of those axes exist here.
//!
//! # What a level may not do
//!
//! **Nothing here reads the player.** The original schedules opponent thrust
//! against the player's position and gap (`PosBalancing`, `RubberBanding`, both
//! recovered) and this project [refuses to port
//! that](../../docs/gameplay/ai.md#what-we-build-instead). A difficulty is how
//! good the opponents are *before* the lights; an easy field that waits for you
//! is rigged, and the player can feel it.

use crate::pilot::Pilot;
use crate::{Span, Tuning};

/// How often a driver at the *easiest* level misses a braking point, per tick
/// of braking: about once a second spent slowing.
///
/// **Here and not in `Tuning`, whose default is zero**: erring is a
/// degradation, so a caller that chose no difficulty (closed-loop harness,
/// replay, test) gets the competent driver.
const MISTAKE_BASE: f32 = 1.0 / 90.0;

/// How good the opponents are. Four levels, named by this project, not the
/// original: [ADR-0006](../../../docs/architecture/adr/0006-no-copyrighted-content.md).
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

    /// A level by name, or `None`, so a caller reading a config file can decide
    /// whether an unrecognised value is worth a message (`oag-game`'s settings
    /// path falls back and says so; a test would rather fail).
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
    /// **The biggest single lever, with a measurement behind it**: at 55, a
    /// third of `Tuning::lateral_accel`, an opponent spent 45 per cent of a
    /// real race off the throttle. This scale walks back down the swept curve.
    #[must_use]
    pub fn grip_believed(self) -> f32 {
        match self {
            // Novice is calibrated on a measurement: 0.3 of 180 is about 54,
            // `lateral_accel` before it was swept, reported from play as "my
            // craft is faster than the AI". The one point with a play report.
            Self::Novice => 0.30,
            // **Evenly spaced in corner speed, not grip**: the target goes as
            // the square root, so these are 0.55, 0.69, 0.84 and 1.0 of the
            // measured corner speed.
            Self::Skilled => 0.48,
            Self::Elite => 0.70,
            Self::Ace => 1.0,
        }
    }

    /// How much of the turn rate the geometry may ask for. Lowering it makes a
    /// weaker driver run wide where a better one tucks back in (the Talon's
    /// Junction play report, here on purpose).
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
    /// **Zero at novice, deliberately**: a slow opponent that still shoots you
    /// in the back is annoying, and a novice wants to be left alone to learn
    /// the circuit.
    #[must_use]
    pub fn aggression(self) -> f32 {
        match self {
            Self::Novice => 0.0,
            Self::Skilled => 0.45,
            Self::Elite => 0.8,
            Self::Ace => 1.0,
        }
    }

    /// How much of a pilot's appetite for **barrel rolls** survives.
    ///
    /// **A second axis, not a reuse of [`Self::aggression`]**: that is what a
    /// driver does to *other craft* (`Driver::stew`'s distinction), a roll is
    /// what it asks of its own, and they want different bottoms (a novice that
    /// never shoots is a kindness, one that can never roll lacks a mechanic).
    /// `Ace` is the pilot's own value. Ours, a play-feel scale.
    #[must_use]
    pub fn roll_appetite(self) -> f32 {
        match self {
            Self::Novice => 0.10,
            Self::Skilled => 0.35,
            Self::Elite => 0.70,
            Self::Ace => 1.0,
        }
    }

    /// How much more of its pool a level keeps back from a roll, and how much
    /// longer a jump it wants first, as one multiplier on both: the same
    /// instinct, so two scales would always move together. It rises as the level
    /// falls; `Ace` is `1.0`, so the pilot's numbers reach it untouched.
    #[must_use]
    pub fn roll_caution(self) -> f32 {
        match self {
            Self::Novice => 2.4,
            Self::Skilled => 1.7,
            Self::Elite => 1.3,
            Self::Ace => 1.0,
        }
    }

    /// How long a driver takes to notice a craft arriving beside, in front of
    /// or behind it, in ticks. **Zero at Ace, like [`Self::mistakes`]**, or "the
    /// measured ceiling, with nothing given back" stops being true.
    ///
    /// The scale is a human's, the one axis modelling a person rather than a
    /// car: a simple visual reaction is about a quarter of a second, fifteen
    /// ticks, which is Skilled. Elite is quicker because it is already looking;
    /// Novice is four tenths, still working out where the circuit goes.
    ///
    /// **A delay, not a blindness**: it costs the early part of a reaction (the
    /// lift for a craft ahead, the cover for one behind, the shot at one
    /// crossing the cone). `driver/reflex.rs` is what a channel does meanwhile.
    #[must_use]
    pub fn reaction_ticks(self) -> u16 {
        match self {
            Self::Novice => 24,
            Self::Skilled => 15,
            Self::Elite => 6,
            Self::Ace => 0,
        }
    }

    /// The share of a speed plan's verified pace this level holds: the level's
    /// handicap on the straights, where the plan asks for everything.
    ///
    /// **Calibrated, not measured off the original**, so the
    /// `difficulty_ground_truth` ladder keeps the shape it had on the corner
    /// model, where the levels' leaders covered 5,885, 6,534, 6,806 and 6,807
    /// units a minute (0.865, 0.96, 1.00, 1.00 of the Ace's). With the plan's
    /// corner margin alone they covered 0.95, 0.99, 1.00, 1.00. See
    /// `docs/gameplay/ai.md`, "The speed plan".
    #[must_use]
    pub fn pace_share(self) -> f32 {
        match self {
            Self::Novice => 0.88,
            Self::Skilled => 0.96,
            Self::Elite | Self::Ace => 1.0,
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
            pace_share: self.pace_share(),
            ..*measured
        }
    }

    /// A continuous position on this ladder, for a caller with a non-integer
    /// skill to place.
    ///
    /// `1.0` is [`Self::Novice`], `2.0` [`Self::Skilled`], `3.0` [`Self::Elite`],
    /// the campaign's Easy/Medium/Hard (Wipeout HD's debug text equates them:
    /// `docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md`'s `HARD ≡ ELITE`,
    /// corroborated by `docs/ui/campaign-screens.md`'s RPCS3 capture). A value
    /// between two integers blends the neighbouring levels' axes linearly, so a
    /// cell's authored position on the disc's `SkillScaleValue` curve
    /// (`oag_tables::track_stats::resolve_skill_scale`) is not discarded; see
    /// `docs/gameplay/ai.md`'s campaign section.
    ///
    /// **`Ace` is unreachable here**, on purpose: the campaign names no fourth
    /// setting, and `Ace` stays reachable only by name (`--autopilot-skill ace`,
    /// the RACE page's Ace choice).
    ///
    /// Clamped to `1.0..=3.0`: an authored curve can run slightly outside, and
    /// there is no fifth level to extrapolate into.
    #[must_use]
    pub fn tune_at_scale(scale: f32, measured: &Tuning) -> Tuning {
        let (lo, hi, t) = Self::straddle(scale);
        let lo = lo.tune(measured);
        let hi = hi.tune(measured);
        Tuning {
            lateral_accel: lerp(lo.lateral_accel, hi.lateral_accel, t),
            max_turn_rate: lerp(lo.max_turn_rate, hi.max_turn_rate, t),
            mistake_rate: lerp(lo.mistake_rate, hi.mistake_rate, t),
            reaction_ticks: lerp(
                f32::from(lo.reaction_ticks),
                f32::from(hi.reaction_ticks),
                t,
            )
            .round() as u16,
            pace_share: lerp(lo.pace_share, hi.pace_share, t),
            ..*measured
        }
    }

    /// The two named levels `scale` sits between, and how far across - see
    /// [`Self::tune_at_scale`]. `t` is always in `0.0..=1.0`.
    fn straddle(scale: f32) -> (Self, Self, f32) {
        let scale = scale.clamp(1.0, 3.0);
        if scale <= 2.0 {
            (Self::Novice, Self::Skilled, scale - 1.0)
        } else {
            (Self::Skilled, Self::Elite, scale - 2.0)
        }
    }

    /// This level's version of a pilot. Only the axes deciding how a driver
    /// treats *other craft* are scaled; its own line (bias, wander, inside) is
    /// left alone so a shy pilot at novice is still recognisably itself.
    #[must_use]
    pub fn temper(self, pilot: &Pilot) -> Pilot {
        let scale = self.aggression();
        let scaled = |span: Span| Span::new(span.low * scale, span.high * scale);
        // The roll axes degrade the ways their meanings run: less often, off
        // more shield, after a longer jump. A floor is a fraction of the pool and
        // clamps at one, a pilot that never rolls.
        let appetite = self.roll_appetite();
        let caution = self.roll_caution();
        let dimmed = |span: Span| Span::new(span.low * appetite, span.high * appetite);
        let raised = |span: Span| {
            Span::new(
                (span.low * caution).min(1.0),
                (span.high * caution).min(1.0),
            )
        };
        let delayed = |span: Span| Span::new(span.low * caution, span.high * caution);
        Pilot {
            trigger: scaled(pilot.trigger),
            ram: scaled(pilot.ram),
            defence: scaled(pilot.defence),
            roll_chance: dimmed(pilot.roll_chance),
            roll_floor: raised(pilot.roll_floor),
            roll_airtime: delayed(pilot.roll_airtime),
            ..*pilot
        }
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

impl std::fmt::Display for Difficulty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// So `--autopilot-skill` can be `Option<Difficulty>` directly: clap rejects an
/// unrecognised token at parse time and names the valid ones, where
/// [`Difficulty::from_name`] is for `oag_game`'s settings path (warn and fall
/// back).
impl std::str::FromStr for Difficulty {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_name(&s.to_ascii_lowercase()).ok_or_else(|| {
            let names: Vec<_> = Self::ALL.iter().map(|(name, _)| *name).collect();
            format!("{s:?} is not a difficulty; try {}", names.join(", "))
        })
    }
}

#[cfg(test)]
mod tests;
