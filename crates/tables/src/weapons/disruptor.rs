//! The Disruptor's `<Stats>` and `<Effect>` blocks - Pure's weapon, and the
//! one block in the table that is a tree rather than a row.
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
//! Only Pure authors it: `docs/formats/weapon-stats.md`'s Pure dialect table
//! has the roster, and `docs/ghidra/functions/psp-pure-usa/weapons.md` has
//! the parser (`WeaponStats_ParseDisruptor`, `0x08809d9c`) whose branches
//! this module mirrors, the struct offsets each one fills, and the two
//! functions that spend them. Everything below that names an address is on
//! that page.

use super::{Error, Node, Result, number, optional};

/// One `<Effect type="..."><EffStats .../></Effect>` the parser reads a
/// duration for.
///
/// Every effect the original's parser knows authors a `time`; three also
/// author an `amount` and two a `speed_percent`. The three are kept as three
/// `Option`s on one type rather than as a type per effect because the
/// original stores them that way too - one time slot and one amount slot per
/// effect, side by side in `WeaponStats_Table` - and because the consumer
/// (`Disruptor_ApplyEffect`, `0x08850ec8`) switches on the *kind* and reads
/// whichever of the two the kind has.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Effect {
    /// How long the victim stays disrupted, in seconds.
    ///
    /// Written into the victim's `craft+0x13c` on the hit and counted down by
    /// `Ship_UpdateWeapons` (`0x0892935c`), which tears the effect down at
    /// zero.
    pub time: f32,
    /// The effect's own strength, where it authors one.
    ///
    /// `Drunk` reads it as the scale of a random steering offset (`Ship_
    /// ApplySteeringTorque`, `0x0892edfc`, straight from the table);
    /// `Rubber Ship` and `Drunk Camera` carry it to `craft+0x140` and spend it
    /// in code this project has not read. `None` for an effect that authors
    /// none.
    pub amount: Option<f32>,
    /// A throttle scale in per cent, where it authors one.
    ///
    /// **Carried by the two Autopilot effects and read by nothing found.**
    /// `Disruptor_ApplyEffect` copies it to `craft+0x144`, and the thrust
    /// scale `Ship_UpdateEngine` (`0x0892f1f0`) actually applies is a literal
    /// `0.7` or `1.3` written by `Ship_UpdateWeapons`'s switch, not this. It
    /// is decoded because the original's parser decodes it, and the doc
    /// comment is the warning: spending it *would* be using a figure the
    /// original ignores. `None` for an effect that authors none.
    pub speed_percent: Option<f32>,
}

/// The ten `<Effect type=...>` names `WeaponStats_ParseDisruptor` matches, in
/// the order its `if` chain tests them.
///
/// **Fourteen are authored, ten are parsed, eight are rolled.** The four
/// authored names not here - `Fire Weapon`, `Turbo Now`, `Steal Weapon`,
/// `Adjust Gravity` - match no parser branch and exist as no string in either
/// Pure executable, so they are not decoded: a field with no reader in the
/// *original* is not a field this project invents a reader for. `HudFlicker`
/// and `Trippy` are parsed by the original and never rolled
/// (`Disruptor_RollEffect`, `0x0884fa10`, is `rand() % 8` over the other
/// eight); they are decoded here because leaving them out would read as a
/// parse gap rather than as the recovered dead layer it is.
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

    /// The eight the original rolls between at grant time, in the order of
    /// `Disruptor_RollEffect`'s switch - so `ROLLED[rand % 8]` is the
    /// original's own mapping from a roll to an effect.
    ///
    /// Kinds 0..7 there are Stall, Mirror, No Airbrakes, Autopilot Slow,
    /// Autopilot Fast, Drunk, Drunk Camera, Rubber Ship; the two parsed but
    /// unrolled kinds are not in this array and never can be drawn.
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

    /// The `type` attribute's spelling, exactly as the file and the
    /// executable's string table have it - spaces included.
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
    /// `time`: `(amount, speed_percent)`.
    ///
    /// This is the original parser's shape, branch by branch, and it is what
    /// makes a missing `amount` on a `Drunk` block an error while a missing
    /// one on a `Stall` block is nothing at all.
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
/// # Nothing here is a blast
///
/// No `damage`, no `blastforce`, no `blastradius`, no `slowdown_time`: this
/// is the one projectile weapon on either disc that hurts nobody. A hit
/// applies one of the effects for its `time` and that is all - see
/// `Disruptor_ApplyEffect` (`0x08850ec8`) and `Ship_UpdateWeapons`'s
/// disruption switch, both on the evidence page.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DisruptorStats {
    /// Energy paid back for absorbing it rather than firing it.
    ///
    /// `Ship_UpdateWeapons`'s absorb switch reads it at `WeaponStats_Table
    /// +0x50` for weapon id 4, which is what pins the id.
    pub absorb: f32,
    /// The bolt's flight speed, in km/h, before the per-class term.
    ///
    /// **The one authored speed the code does make per-class.**
    /// `Disruptor_SpeedForClass` (`0x088592d8`) returns
    /// `speed + 80.0 * class_index`, 0..4 for Vector..Phantom, and
    /// `Disruptor_Update` divides by 3.6 on the way to a velocity like every
    /// other Pure weapon. [`Self::speed_for_class`] is that function.
    pub speed: f32,
    /// The parsed effects, indexed by [`EffectKind::ALL`]'s order; `None` for
    /// one the file does not author.
    ///
    /// The original zero-initialises its table and a missing block leaves
    /// the slot zero, so a `None` here is a `0.0`-second effect there. It is
    /// an `Option` rather than a zero for the reason the module docs give:
    /// a quietly-zero `time` reads as an effect that ends at once, which is a
    /// gameplay bug, where "the file authors no such block" is a fact about
    /// the disc.
    pub effects: [Option<Effect>; 10],
}

impl DisruptorStats {
    /// One parsed effect, or `None` when the file authors no such block.
    #[must_use]
    pub fn effect(&self, kind: EffectKind) -> Option<Effect> {
        self.effects[kind as usize]
    }

    /// The speed a bolt flies at on one speed class, in km/h.
    ///
    /// `Disruptor_SpeedForClass` (`0x088592d8`): `speed + 80.0 * class`, where
    /// `class` is the 0-based index in Pure's five-rung ladder, Vector first.
    /// The `80.0` is the executable's own literal, not an authored figure.
    #[must_use]
    pub fn speed_for_class(&self, class_index: u8) -> f32 {
        self.speed + PER_CLASS_KMH * f32::from(class_index)
    }

    /// [`Self::speed_for_class`] by the class's name, or `None` for a name on
    /// no ladder.
    ///
    /// Pure's ladder is Pulse's four with `Vector` under them, so `Vector` is
    /// rung 0 and [`crate::handling::SpeedClass`]'s four are rungs 1..=4 -
    /// which puts Phantom at `speed + 320`. **The rung order is inferred from
    /// the class-name run at `0x08a445d0`, not read**: the evidence page
    /// records only that `class_index` *is* `DAT_08b173e0` in `0..4`, and the
    /// five values behind it were not read, so Vector-first is the order the
    /// names are listed in and nothing stronger. Case-insensitive, as
    /// `SpeedClass::from_name` is, because the two XML files spell a class
    /// differently.
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

/// Pure's fifth speed class, the one under Venom, as `<Pickupodds class>`
/// spells it.
pub const VECTOR: &str = "Vector";

/// How much faster a bolt flies per rung of the speed-class ladder, in km/h.
///
/// **Recovered, confidence 82**: the `80.0` in `Disruptor_SpeedForClass`
/// (`0x088592d8` on `psp-pure-usa`).
pub const PER_CLASS_KMH: f32 = 80.0;

/// Decodes one `<Weapon type="Disruptor">` element.
///
/// Mirrors `WeaponStats_ParseDisruptor` branch for branch: `<Stats absorb
/// speed>`, then every `<Effect>` child whose `type` is one of the ten
/// [`EffectKind`]s, each reading `time` and whichever of `amount` and
/// `speed_percent` its branch reads. An `<Effect>` whose type is not one of
/// the ten is walked past, exactly as the original walks past `Fire Weapon`
/// and its three siblings.
///
/// # Errors
///
/// A missing `absorb` or `speed`, or a parsed effect missing an attribute
/// its branch reads, is [`Error::MissingAttribute`] - which the caller's
/// `optional_block` turns into a skipped weapon rather than a failed file.
/// A present attribute that is not a number is an error either way.
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
            // The original's branch opens the `<EffStats>` child and reads
            // nothing when there is none, leaving the slot zero. Here that
            // is a missing attribute rather than a zero, for the reason
            // `DisruptorStats::effects` gives.
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
