//! The Disruptor's `<Stats>` and `<Effect>` blocks: Pure's weapon, and the one
//! block in the table that is a tree rather than a row.
//!
//! ```xml
//! <Weapon type="Disruptor">
//!   <Stats absorb speed/>
//!   <Effect type="Stall">          <EffStats time/>               </Effect>
//!   <Effect type="Autopilot Slow"> <EffStats speed_percent time/> </Effect>
//!   <Effect type="Drunk">          <EffStats amount time/>        </Effect>
//!   ...
//! </Weapon>
//! ```
//!
//! Only Pure authors it. `docs/formats/weapon-stats.md`'s Pure dialect table has
//! the roster; `docs/ghidra/functions/psp-pure-usa/weapons.md` has the parser
//! (`WeaponStats_ParseDisruptor`, `0x08809d9c`) this module mirrors, the struct
//! offsets, and the two functions that spend them. Every address below is on
//! that page.

use super::{Error, Node, Result, number, optional};

/// One `<Effect type="..."><EffStats .../></Effect>` the parser reads a duration
/// for.
///
/// Every effect authors a `time`; three also an `amount`, two a `speed_percent`.
/// Kept as `Option`s on one type because the original stores one time slot and
/// one amount slot per effect in `WeaponStats_Table`, and the consumer
/// (`Disruptor_ApplyEffect`, `0x08850ec8`) switches on the *kind*.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Effect {
    /// How long the victim stays disrupted, in seconds.
    /// How long the victim stays disrupted, in seconds. Written to the victim's
    /// `craft+0x13c`, counted down by `Ship_UpdateWeapons` (`0x0892935c`), which
    /// tears the effect down at zero.
    pub time: f32,
    /// The effect's strength, where authored. `Drunk` reads it as the scale of a
    /// random steering offset (`Ship_ApplySteeringTorque`, `0x0892edfc`);
    /// `Rubber Ship` and `Drunk Camera` carry it to `craft+0x140` and spend it in
    /// unread code. `None` if the effect authors none.
    pub amount: Option<f32>,
    /// A throttle scale in per cent, where authored. **Carried by the two
    /// Autopilot effects and read by nothing found**: `Disruptor_ApplyEffect`
    /// copies it to `craft+0x144`, but the thrust scale `Ship_UpdateEngine`
    /// (`0x0892f1f0`) applies is a literal `0.7` or `1.3` from
    /// `Ship_UpdateWeapons`'s switch. Decoded because the original's parser
    /// decodes it; spending it *would* use a figure the original ignores.
    pub speed_percent: Option<f32>,
}

/// The ten `<Effect type=...>` names `WeaponStats_ParseDisruptor` matches, in its
/// `if` chain's order.
///
/// **Fourteen are authored, ten parsed, eight rolled.** `Fire Weapon`, `Turbo
/// Now`, `Steal Weapon` and `Adjust Gravity` match no parser branch and are
/// strings in neither Pure executable, so they are not decoded. `HudFlicker` and
/// `Trippy` are parsed but never rolled (`Disruptor_RollEffect`, `0x0884fa10`,
/// is `rand() % 8` over the other eight); decoded so they read as the recovered
/// dead layer, not a parse gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EffectKind {
    /// Engine output zeroed for `time`.
    Stall,
    /// Yaw steering negated, blending back over the last second.
    MirrorLeftRight,
    /// Both airbrake inputs zeroed.
    NoAirbrakes,
    /// Autopilot takes the craft at reduced thrust.
    AutopilotSlow,
    /// Autopilot takes the craft at raised thrust.
    AutopilotFast,
    /// Parsed, never rolled.
    HudFlicker,
    /// A random yaw offset redrawn one tick in sixteen.
    Drunk,
    /// A hover damping term scaled down.
    RubberShip,
    /// A camera wobble, unread.
    DrunkCamera,
    /// Parsed, never rolled.
    Trippy,
}

impl EffectKind {
    /// Every parsed effect, in parser order.
    pub const ALL: [Self; 10] = [
        Self::Stall,
        Self::MirrorLeftRight,
        Self::NoAirbrakes,
        Self::AutopilotSlow,
        Self::AutopilotFast,
        Self::HudFlicker,
        Self::Drunk,
        Self::RubberShip,
        Self::DrunkCamera,
        Self::Trippy,
    ];

    /// The eight rolled at grant time, in `Disruptor_RollEffect`'s switch order,
    /// so `ROLLED[rand % 8]` is the original's roll-to-effect mapping (kinds 0..7:
    /// Stall, Mirror, No Airbrakes, Autopilot Slow, Autopilot Fast, Drunk, Drunk
    /// Camera, Rubber Ship). The two unrolled kinds can never be drawn.
    pub const ROLLED: [Self; 8] = [
        Self::Stall,
        Self::MirrorLeftRight,
        Self::NoAirbrakes,
        Self::AutopilotSlow,
        Self::AutopilotFast,
        Self::Drunk,
        Self::DrunkCamera,
        Self::RubberShip,
    ];

    /// The `type` attribute's spelling, as the file and executable string table
    /// have it, spaces included.
    #[must_use]
    pub fn as_type(self) -> &'static str {
        match self {
            Self::Stall => "Stall",
            Self::MirrorLeftRight => "Mirror Left Right",
            Self::NoAirbrakes => "No Airbrakes",
            Self::AutopilotSlow => "Autopilot Slow",
            Self::AutopilotFast => "Autopilot Fast",
            Self::HudFlicker => "HUD Flicker",
            Self::Drunk => "Drunk",
            Self::RubberShip => "Rubber Ship",
            Self::DrunkCamera => "Drunk Camera",
            Self::Trippy => "Trippy",
        }
    }

    /// Which `<EffStats>` attributes this kind's parser branch reads besides
    /// `time`: `(amount, speed_percent)`. It is why a missing `amount` on `Drunk`
    /// is an error and on `Stall` nothing.
    const fn reads(self) -> (bool, bool) {
        match self {
            Self::AutopilotSlow | Self::AutopilotFast => (false, true),
            Self::Drunk | Self::RubberShip | Self::DrunkCamera => (true, false),
            _ => (false, false),
        }
    }
}

/// The Disruptor's whole block: two `<Stats>` attributes and the ten parsed
/// effects.
///
/// **Nothing here is a blast**: no `damage`, `blastforce`, `blastradius` or
/// `slowdown_time`. The one projectile weapon on either disc that hurts nobody; a
/// hit applies an effect for its `time` (`Disruptor_ApplyEffect`,
/// `0x08850ec8`; `Ship_UpdateWeapons`'s disruption switch).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DisruptorStats {
    /// Energy paid back for absorbing it rather than firing it.
    /// `Ship_UpdateWeapons`'s absorb switch reads it at `WeaponStats_Table +0x50`
    /// for weapon id 4, which pins the id.
    pub absorb: f32,
    /// The bolt's flight speed in km/h, before the per-class term. **The one
    /// authored speed the code makes per-class**: `Disruptor_SpeedForClass`
    /// (`0x088592d8`) returns `speed + 80.0 * class_index` (0..4, Vector..Phantom)
    /// and `Disruptor_Update` divides by 3.6. See [`Self::speed_for_class`].
    pub speed: f32,
    /// The parsed effects, in [`EffectKind::ALL`]'s order; `None` for one the
    /// file does not author. The original's zeroed table makes that a
    /// `0.0`-second effect; an `Option` because a quietly-zero `time` reads as a
    /// gameplay bug where "no such block" is a fact about the disc.
    pub effects: [Option<Effect>; 10],
}

impl DisruptorStats {
    /// One parsed effect, or `None` when the file authors no such block.
    #[must_use]
    pub fn effect(&self, kind: EffectKind) -> Option<Effect> {
        self.effects[kind as usize]
    }

    /// The speed a bolt flies at on one speed class, in km/h:
    /// `Disruptor_SpeedForClass` (`0x088592d8`), `speed + 80.0 * class`, with
    /// `class` the 0-based index in Pure's five-rung ladder, Vector first. The
    /// `80.0` is the executable's literal, not authored.
    #[must_use]
    pub fn speed_for_class(&self, class_index: u8) -> f32 {
        self.speed + PER_CLASS_KMH * f32::from(class_index)
    }

    /// [`Self::speed_for_class`] by class name, or `None` for a name on no
    /// ladder.
    ///
    /// Pure's ladder is Pulse's four with `Vector` under them: `Vector` is rung 0,
    /// [`crate::handling::SpeedClass`]'s four are 1..=4, so Phantom is `speed +
    /// 320`. **The order is inferred from the class-name run at `0x08a445d0`, not
    /// read**: the evidence page records only that `class_index` *is*
    /// `DAT_08b173e0` in `0..4`, with the five values behind it unread.
    /// Case-insensitive, as the two XML files spell classes differently.
    #[must_use]
    pub fn speed_for_named(&self, class: &str) -> Option<f32> {
        let rung = if class.eq_ignore_ascii_case(VECTOR) {
            0
        } else {
            crate::handling::SpeedClass::from_name(class)? as u8 + 1
        };
        Some(self.speed_for_class(rung))
    }
}

/// Pure's fifth speed class, under Venom, as `<Pickupodds class>` spells it.
pub const VECTOR: &str = "Vector";

/// How much faster a bolt flies per rung of the class ladder, in km/h.
/// **Recovered, confidence 82**: the `80.0` in `Disruptor_SpeedForClass`
/// (`0x088592d8` on `psp-pure-usa`).
pub const PER_CLASS_KMH: f32 = 80.0;

/// Decodes one `<Weapon type="Disruptor">` element.
///
/// Mirrors `WeaponStats_ParseDisruptor` branch for branch: `<Stats absorb
/// speed>`, then each `<Effect>` of the ten [`EffectKind`]s, reading `time` and
/// whichever of `amount`/`speed_percent` its branch reads. Other types are
/// skipped, as the original skips `Fire Weapon` and its siblings.
///
/// # Errors
///
/// A missing `absorb`/`speed`, or an effect missing an attribute its branch
/// reads, is [`Error::MissingAttribute`], which `optional_block` turns into a
/// skipped weapon. A present non-number is an error either way.
pub(super) fn parse(weapon: &Node, block: &Node) -> Result<DisruptorStats> {
    let mut effects = [None; 10];
    for effect in weapon.children_named("Effect") {
        let Some(kind) = effect
            .value("type")
            .and_then(|name| EffectKind::ALL.into_iter().find(|k| k.as_type() == name))
        else {
            continue;
        };
        let Some(stats) = effect.children_named("EffStats").next() else {
            // The original opens `<EffStats>` and reads nothing when absent,
            // leaving a zero slot; here a missing element, not a zero (see
            // `DisruptorStats::effects`).
            return Err(Error::MissingElement {
                parent: "Effect",
                element: "EffStats",
            });
        };
        let (wants_amount, wants_percent) = kind.reads();
        effects[kind as usize] = Some(Effect {
            time: number(stats, "EffStats", "time")?,
            amount: if wants_amount {
                Some(number(stats, "EffStats", "amount")?)
            } else {
                optional(stats, "EffStats", "amount")?
            },
            speed_percent: if wants_percent {
                Some(number(stats, "EffStats", "speed_percent")?)
            } else {
                optional(stats, "EffStats", "speed_percent")?
            },
        });
    }
    Ok(DisruptorStats {
        absorb: number(block, "Stats", "absorb")?,
        speed: number(block, "Stats", "speed")?,
        effects,
    })
}

#[cfg(test)]
mod tests;
