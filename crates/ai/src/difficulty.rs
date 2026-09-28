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

    /// How much of a pilot's appetite for **barrel rolls** survives.
    ///
    /// **A second axis rather than a reuse of [`Self::aggression`]**, and the
    /// distinction is the same one `Driver::stew` draws: aggression is what a
    /// driver does to *other craft*, and a roll is entirely what it asks of its
    /// own. They also want different bottoms - a novice that never shoots is a
    /// kindness, a novice that can never roll is a level with a mechanic
    /// missing from it.
    ///
    /// `Ace` is the pilot's own value, per the module docs: the top level gives
    /// back the measurement with nothing taken away, and every level below it
    /// rolls less often. Ours, and a play-feel scale.
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
    /// longer a jump it wants first, as one multiplier on both.
    ///
    /// **It rises as the level falls**, which is the same degradation the rest
    /// of this type applies read from the other end: a weaker driver is more
    /// cautious about spending, and slower to decide a jump is worth spending
    /// on. `Ace` is `1.0`, so the pilot's own numbers reach it untouched.
    ///
    /// One multiplier for two axes because they are the same instinct, and two
    /// scales would be two knobs that always moved together.
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

    /// A continuous position on this ladder, for a caller that has a
    /// non-integer skill to place rather than a named level to pick.
    ///
    /// `1.0` reads as [`Self::Novice`], `2.0` as [`Self::Skilled`], `3.0` as
    /// [`Self::Elite`] - the same three-rung vocabulary the built-in
    /// campaign's own Easy/Medium/Hard already carries: Wipeout HD's own
    /// debug text equates them rung for rung
    /// (`docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md`'s `HARD ≡
    /// ELITE` section, corroborated at the UI layer by
    /// `docs/ui/campaign-screens.md`'s RPCS3 capture). A value between two
    /// integers blends the neighbouring levels' axes linearly rather than
    /// snapping to the nearer one, so a campaign cell's own authored
    /// position on the disc's `SkillScaleValue` curve
    /// (`oag_tables::track_stats::resolve_skill_scale`) is not discarded on
    /// the way in - see `docs/gameplay/ai.md`'s own campaign section for
    /// where this scale comes from and what it does not carry.
    ///
    /// **`Ace` is unreachable through this path**, on purpose: the
    /// campaign's own three-rung vocabulary never names a fourth, harder
    /// setting either, and `Ace` stays what it always was - "the measured
    /// ceiling, with nothing given back" - reachable only by naming it
    /// directly (`--autopilot-skill ace`, the RACE page's own Ace choice).
    ///
    /// Clamped to `1.0..=3.0` - the curve a track authors can run slightly
    /// outside that band (a cell's `skillEasy` a little under `1.0`, or a
    /// track's own `SkillScaleValue` a little over `3.0`), and this project
    /// has no fifth level to extrapolate into.
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
        // The roll axes degrade in the two directions their own meanings run:
        // less often, off more shield, after a longer jump. A floor is a
        // fraction of the pool, so it clamps at one - which is a pilot that
        // never rolls, and the honest end of the scale rather than a nonsense.
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

/// So `--autopilot-skill` can be `Option<Difficulty>` directly, the same way
/// `oag_gameplay::ControlScheme` lets `--scheme` be - clap rejects an
/// unrecognised token at parse time and names the valid ones, rather than a
/// caller having to do that by hand the way `oag_game`'s settings path (which
/// wants to warn and fall back instead of refusing to boot) does through
/// [`Difficulty::from_name`].
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
