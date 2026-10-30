//! The reply record rules a guest can break: what the host faults a module for.

use core::fmt;

use crate::{
    Cue, Effect, HudElement, MAX_COMMANDS, MAX_HUD_EXTRA, MAX_HUD_SHAKE, MAX_HUD_TEXT,
    MAX_PAD_FACTOR, MAX_ROLL_EVERY, MAX_THRUST_FACTOR, Slot, Weapon, WeatherKind,
};

/// A reply record that breaks a rule of its own. The host faults the module; it never
/// clamps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Malformed {
    /// More commands than [`MAX_COMMANDS`].
    TooMany(u32),
    /// A kind code this revision does not know.
    Kind(u8),
    /// A slot with no record.
    Slot(u8),
    /// A `scale-thrust` factor outside `[0, MAX_THRUST_FACTOR]`, or not a number.
    Factor {
        /// The slot.
        slot: Slot,
        /// The factor sent.
        factor: f32,
    },
    /// A `scale-pad` factor outside `[0, MAX_PAD_FACTOR]` (NaN included), or a
    /// race-wide one with a slot other than 0.
    PadFactor {
        /// The slot byte.
        slot: u8,
        /// The factor sent.
        factor: f32,
    },
    /// An `emit` whose value is not finite.
    EmitValue {
        /// The slot.
        slot: Slot,
        /// The value sent.
        value: f32,
    },
    /// A `shield-delta` that is not finite.
    Amount {
        /// The slot.
        slot: Slot,
        /// The amount sent.
        amount: f32,
    },
    /// A `play-cue` code outside [`Cue::ALL`].
    Cue(u16),
    /// A `fire-effect` code outside [`Effect::ALL`].
    Effect(u16),
    /// A `pickup-roll` with a slot other than 0, an unknown weapon, or a period
    /// outside `1..=MAX_ROLL_EVERY`.
    Roll {
        /// The slot byte.
        slot: u8,
        /// The weapon code.
        weapon: u16,
        /// The period.
        every: u32,
    },
    /// A race-wide `set-weather`, `set-sun` or `grade-sky` with a slot other than 0, or a
    /// value outside `[0, 1]` (NaN included).
    Unit {
        /// The command kind code.
        kind: u8,
        /// The slot byte.
        slot: u8,
        /// The value sent.
        value: f32,
    },
    /// A `set-weather` kind code outside [`WeatherKind::ALL`].
    Weather(u16),
    /// A `hud-*` element code outside [`HudElement::ALL`].
    HudElement(u16),
    /// A `hud-shake` amplitude or `hud-bar-extra` share out of range, or not a number.
    HudValue {
        /// The command kind code.
        kind: u8,
        /// The value sent.
        value: f32,
    },
    /// A `hud-text` or `hud-icon` anchor or scale out of range.
    HudPlace,
    /// A `hud-tint` of the pickup element: a mod's pickup is coloured by `pickup-look`.
    PickupTint,
    /// A `pickup-look` naming no weapon, as its icon (`arg`) or its backdrop colour
    /// (`value`).
    PickupLook {
        /// The record's argument.
        arg: u16,
        /// The record's value.
        value: u32,
    },
    /// A `hud-text` or `hud-icon` whose pool offset holds no whole string of at most
    /// [`MAX_HUD_TEXT`] bytes of UTF-8.
    Pool(u16),
}

impl fmt::Display for Malformed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::TooMany(count) => write!(f, "{count} commands, the most is {MAX_COMMANDS}"),
            Self::Kind(kind) => write!(f, "unknown command kind {kind}"),
            Self::Slot(slot) => write!(f, "slot {slot} is not a grid slot"),
            Self::Factor { slot, factor } => write!(
                f,
                "scale-thrust factor {factor} for slot {} is not in [0, {MAX_THRUST_FACTOR}]",
                slot.0
            ),
            Self::PadFactor { slot, factor } => write!(
                f,
                "scale-pad factor {factor} (slot byte {slot}) is not in [0, {MAX_PAD_FACTOR}], or a race-wide one names a slot"
            ),
            Self::EmitValue { slot, value } => {
                write!(f, "emit value {value} for slot {} is not finite", slot.0)
            }
            Self::Amount { slot, amount } => {
                write!(f, "shield-delta {amount} for slot {} is not finite", slot.0)
            }
            Self::Cue(code) => write!(f, "play-cue code {code} is not one of {}", Cue::ALL.len()),
            Self::Effect(code) => write!(
                f,
                "fire-effect code {code} is not one of {}",
                Effect::ALL.len()
            ),
            Self::Unit { kind, slot, value } => write!(
                f,
                "race-wide command {kind} slot {slot} value {value}: slot 0 and a value in [0, 1]"
            ),
            Self::Weather(code) => write!(
                f,
                "set-weather kind {code} is not one of {}",
                WeatherKind::ALL.len()
            ),
            Self::HudElement(code) => write!(
                f,
                "hud element {code} is not one of {}",
                HudElement::ALL.len()
            ),
            Self::HudValue { kind, value } => write!(
                f,
                "hud command {kind} value {value}: hud-shake in [0, {MAX_HUD_SHAKE}], hud-bar-extra in (0, {MAX_HUD_EXTRA}]"
            ),
            Self::PickupTint => write!(
                f,
                "hud-tint no longer reaches the pickup: use pickup-look to give a mod pickup its colour"
            ),
            Self::PickupLook { arg, value } => write!(
                f,
                "pickup-look icon argument {arg} and colour value {value}: each a weapon in 1..={} (or an own icon, or an rgba)",
                Weapon::ALL.len()
            ),
            Self::HudPlace => write!(
                f,
                "hud placement: an anchor of the nine and a scale of 1..=15 quarters"
            ),
            Self::Pool(at) => write!(
                f,
                "hud string at pool offset {at} is not whole UTF-8 of at most {MAX_HUD_TEXT} bytes"
            ),
            Self::Roll {
                slot,
                weapon,
                every,
            } => write!(
                f,
                "pickup-roll slot {slot} weapon {weapon} every {every}: slot 0, a weapon in 1..={}, every in 1..={MAX_ROLL_EVERY}",
                Weapon::ALL.len()
            ),
        }
    }
}
