//! ABI 1 of the WebAssembly extensions, defined once: the I/O buffer's fixed
//! little-endian layout, its records, and the vocabulary the host (`oag-modhost`) and
//! the guest SDK (`oag-ext`) share. Both compile against this crate, so the layout
//! cannot drift between them.
//!
//! A guest exports `memory`, `oag_abi() -> u32` (returns [`ABI_WORD`]),
//! `oag_io() -> u32` (the address of an [`IO_LEN`]-byte buffer in its memory),
//! `tick() -> u32` (0 on success) and, optionally, `oag_state_len() -> u32` (its state
//! blob's length, checked against the manifest). Before `tick` the host writes the
//! observation; after it, the host reads the state blob back from where it wrote it and
//! the reply from [`REPLY_AT`].
//!
//! | Offset | Size | Field |
//! | --- | --- | --- |
//! | [`ABI_AT`] | 4 | ABI major |
//! | [`TICK_AT`] | 8 | world tick |
//! | [`SLOTS_AT`] | 4 | slot count ([`MAX_SLOTS`]) |
//! | [`ACTIVE_AT`] | 4 | active mask, one bit per slot holding a craft |
//! | [`EVENT_COUNT_AT`] | 4 | event count |
//! | [`STATE_LEN_AT`] | 4 | state length |
//! | [`EVENTS_AT`] | [`EVENT_LEN`] x [`MAX_EVENTS`] | events ([`Event`]): kind `u8`, slot `u8`, 6 bytes zero |
//! | [`STATE_AT`] | [`MAX_STATE_BYTES`] | the module's state blob, in and out |
//! | [`REPLY_AT`] | 4 | command count, written by the guest |
//! | [`REPLY_AT`] + 4 | [`COMMAND_LEN`] x [`MAX_COMMANDS`] | commands ([`Command`]): kind `u8`, slot `u8`, `u16` argument, `f32`, `u32` |
//! | [`CRAFTS_AT`] | [`CRAFT_LEN`] x [`MAX_CRAFTS`] | one record per grid slot ([`Craft`]): pickup `u8`, flags `u8`, lap `u8`, laps target `u8`, shield `f32`, shield maximum `f32`, speed km/h `f32` |
//!
//! The README's WIT (`docs/architecture/wasm-extensions/README.md`, section 2.1) is
//! the documentation of the records; this is what is built of it.
//!
//! **The revision.** ABI 1 is edited in place before the first release (README 4.3).
//! Every edit to this file's layout, codes or encodings bumps [`REVISION`]; a unit test
//! folds all of it into a committed constant, so a forgotten bump fails the gate. A
//! guest returns [`ABI_WORD`] and the host refuses any other revision by name.
#![no_std]
#![forbid(unsafe_code)]

use core::fmt;

/// The ABI major. A manifest's `[extension] api` names it.
pub const ABI: u32 = 1;
/// The layout revision within [`ABI`]: bumped by hand on any edit to the layout. A
/// module built before revisions existed returns the bare major and is revision 0.
pub const REVISION: u32 = 3;
const _: () = assert!(REVISION > 0 && REVISION < 256);
/// What a guest's `oag_abi` returns: the major above the revision's 8 bits.
pub const ABI_WORD: u32 = (ABI << 8) | REVISION;

/// Splits an `oag_abi` return into the major and the revision. A value below 256 is a
/// module from before revisions: its major alone, revision `None`.
#[must_use]
pub const fn split_abi_word(word: u32) -> (u32, Option<u32>) {
    if word < 256 {
        (word, None)
    } else {
        (word >> 8, Some(word & 0xff))
    }
}

/// The buffer's length. A guest's buffer must be at least this long.
pub const IO_LEN: usize = 4096;
/// Grid slots the active mask can name.
pub const MAX_SLOTS: u32 = 32;
/// Events per call. Past it the newest are dropped by the caller, which counts them.
pub const MAX_EVENTS: usize = 32;
/// The longest state blob.
pub const MAX_STATE_BYTES: usize = 1024;
/// Commands per reply; more is a fault (README 2.1: 64, proposed).
pub const MAX_COMMANDS: usize = 64;
/// Craft records per observation: a grid's slots. A [`Slot`] is below it.
pub const MAX_CRAFTS: usize = 16;

/// Where the ABI major is written.
pub const ABI_AT: usize = 0;
/// Where the world tick is written, `u64`.
pub const TICK_AT: usize = 8;
/// Where [`MAX_SLOTS`] is written.
pub const SLOTS_AT: usize = 16;
/// Where the active mask is written.
pub const ACTIVE_AT: usize = 20;
/// Where the event count is written.
pub const EVENT_COUNT_AT: usize = 24;
/// Where the state blob's length is written.
pub const STATE_LEN_AT: usize = 28;
/// Where the events start.
pub const EVENTS_AT: usize = 32;
/// One event record.
pub const EVENT_LEN: usize = 8;
/// Where the state blob starts.
pub const STATE_AT: usize = EVENTS_AT + EVENT_LEN * MAX_EVENTS;
/// Where the guest writes its reply: the count, then the records.
pub const REPLY_AT: usize = STATE_AT + MAX_STATE_BYTES;
/// One command record.
pub const COMMAND_LEN: usize = 12;
/// Where the craft records start.
pub const CRAFTS_AT: usize = REPLY_AT + 4 + COMMAND_LEN * MAX_COMMANDS;
/// One craft record.
pub const CRAFT_LEN: usize = 16;
const _: () = assert!(CRAFTS_AT + CRAFT_LEN * MAX_CRAFTS <= IO_LEN);
const _: () = assert!(MAX_CRAFTS <= MAX_SLOTS as usize);

/// Within an event record: the kind code.
pub const EVENT_KIND: usize = 0;
/// Within an event record: the slot.
pub const EVENT_SLOT: usize = 1;
/// Within a command record: the kind code.
pub const COMMAND_KIND: usize = 0;
/// Within a command record: the slot.
pub const COMMAND_SLOT: usize = 1;
/// Within a command record: the `u16` argument.
pub const COMMAND_ARG: usize = 2;
/// Within a command record: the `f32`.
pub const COMMAND_VALUE: usize = 4;
/// Within a command record: the `u32`.
pub const COMMAND_WORD: usize = 8;
/// Within a craft record: the pickup code.
pub const CRAFT_PICKUP: usize = 0;
/// Within a craft record: the flags.
pub const CRAFT_FLAGS: usize = 1;
/// Within a craft record: the lap being driven, 1-based, saturating at 255.
pub const CRAFT_LAP: usize = 2;
/// Within a craft record: the laps the race ends after; 0 for a race with no target.
pub const CRAFT_LAPS_TARGET: usize = 3;
/// Within a craft record: the shield, `f32`.
pub const CRAFT_SHIELD: usize = 4;
/// Within a craft record: the shield maximum, `f32`.
pub const CRAFT_SHIELD_MAX: usize = 8;
/// Within a craft record: the speed in km/h, `f32`.
pub const CRAFT_SPEED: usize = 12;

/// Craft flag: a person flies the slot.
pub const CRAFT_PERSON: u8 = 1;
/// Craft flag: a person flying the slot holds the fire button (`SQUARE`) this tick.
pub const CRAFT_FIRE_HELD: u8 = 2;
/// Craft flag: the craft is racing: past the start, not wrecked, not respawning.
pub const CRAFT_RACING: u8 = 4;

/// The largest `scale-thrust` factor: a mod may double an engine's thrust, no more.
/// Chosen (README 2.1 proposed `[0.25, 2.0]`; the lower end is `0` here).
pub const MAX_THRUST_FACTOR: f32 = 2.0;
/// The longest `pickup-roll` period: every 1st up to every 255th grant.
pub const MAX_ROLL_EVERY: u32 = 255;

/// Event kind codes, in [`Event`]'s order from 1.
const EVENT_WALL_CONTACT: u8 = 1;
const EVENT_PICKUP_FIRE: u8 = 2;
const EVENT_PICKUP_ABSORBED: u8 = 3;
const EVENT_PAD_ENERGY: u8 = 4;
/// Command kind codes.
const COMMAND_SCALE_THRUST: u8 = 1;
const COMMAND_SHIELD_DELTA: u8 = 2;
const COMMAND_GIVE_PICKUP: u8 = 3;
const COMMAND_PAD_MODE: u8 = 4;
const COMMAND_PLAY_CUE: u8 = 5;
const COMMAND_FIRE_EFFECT: u8 = 6;
const COMMAND_PICKUP_TINT: u8 = 7;
const COMMAND_CLEAR_PICKUP: u8 = 8;
const COMMAND_PICKUP_ROLL: u8 = 9;
const COMMAND_SET_WEATHER: u8 = 10;
const COMMAND_SET_SUN: u8 = 11;
const COMMAND_GRADE_SKY: u8 = 12;
/// The craft record's pickup code for a simulation mod's own pickup.
const PICKUP_OURS: u8 = u8::MAX;

/// A grid slot that may hold a craft: below [`MAX_CRAFTS`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Slot(u8);

impl Slot {
    /// The slot at `index`, if there is one.
    #[must_use]
    pub const fn new(index: usize) -> Option<Self> {
        if index < MAX_CRAFTS {
            Some(Self(index as u8))
        } else {
            None
        }
    }

    /// Its place on the grid, for indexing a per-slot array.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    /// Its bit in a per-slot mask.
    #[must_use]
    pub const fn bit(self) -> u32 {
        1 << self.0
    }

    /// Every slot the buffer has a record for, in order.
    pub fn all() -> impl Iterator<Item = Self> {
        (0..MAX_CRAFTS as u8).map(Self)
    }
}

/// Enum plumbing for the code-carrying vocabulary: the list, the code and back.
macro_rules! coded {
    ($(#[$doc:meta])* $name:ident: $repr:ty, first $first:literal { $($(#[$vdoc:meta])* $variant:ident),+ $(,)? }) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name {
            $($(#[$vdoc])* $variant),+
        }

        impl $name {
            /// Every value, in code order.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            /// Its code in the buffer.
            #[must_use]
            pub const fn code(self) -> $repr {
                self as $repr + $first
            }

            /// The value a code names, if any.
            #[must_use]
            pub fn from_code(code: $repr) -> Option<Self> {
                Self::ALL.get(code.checked_sub($first)? as usize).copied()
            }
        }
    };
}

coded! {
    /// A title's weapon, in `oag_tables::weapons::Weapon::ALL`'s order: the codes a
    /// craft record's pickup and a `pickup-roll` use, from 1.
    Weapon: u8, first 1 {
        /// Code 1.
        Rocket,
        /// Code 2.
        Missile,
        /// Code 3.
        Quake,
        /// Code 4.
        Cannon,
        /// Code 5.
        Turbo,
        /// Code 6.
        Shield,
        /// Code 7.
        Autopilot,
        /// Code 8.
        Plasma,
        /// Code 9.
        Bomb,
        /// Code 10.
        Mine,
        /// Code 11.
        LeachBeam,
        /// Code 12.
        Repulser,
        /// Code 13.
        Shuriken,
        /// Code 14.
        Disruptor,
    }
}

coded! {
    /// `play-cue`'s sounds: a title's own, by the engine's cue names.
    Cue: u16, first 0 {
        /// `SpeedupPad`.
        SpeedupPad,
        /// `Turbo`.
        Turbo,
        /// `Collision`.
        Collision,
        /// `Absorb`.
        Absorb,
        /// `Shield`.
        Shield,
        /// `ShieldActive`.
        ShieldActive,
    }
}

coded! {
    /// `fire-effect`'s particle effects: a title's own, by trigger.
    Effect: u16, first 0 {
        /// The wall contact's sparks (`CollisionSpark`).
        CollisionSpark,
        /// A weapon hit's sparks (`HitSpark`).
        HitSpark,
    }
}

coded! {
    /// `set-weather`'s kinds: the host title's own weather, by kind.
    WeatherKind: u16, first 0 {
        /// The title's authored rain.
        Rain,
    }
}

/// What a craft holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Pickup {
    /// Nothing.
    #[default]
    None,
    /// One of the title's weapons.
    Weapon(Weapon),
    /// A simulation mod's own pickup (`give-pickup`).
    Ours,
}

impl Pickup {
    /// Its code in a craft record.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Weapon(weapon) => weapon.code(),
            Self::Ours => PICKUP_OURS,
        }
    }

    /// The pickup a code names, if any.
    #[must_use]
    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::None),
            PICKUP_OURS => Some(Self::Ours),
            _ => Weapon::from_code(code).map(Self::Weapon),
        }
    }
}

/// A colour, a byte a channel; packed red in the low byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rgba {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
    /// Alpha.
    pub a: u8,
}

impl Rgba {
    /// A colour from its channels.
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Packed, red in the low byte.
    #[must_use]
    pub const fn to_u32(self) -> u32 {
        u32::from_le_bytes([self.r, self.g, self.b, self.a])
    }

    /// Unpacked from red in the low byte.
    #[must_use]
    pub const fn from_u32(packed: u32) -> Self {
        let [r, g, b, a] = packed.to_le_bytes();
        Self { r, g, b, a }
    }
}

/// One grid slot as a guest reads it: copies of what the HUD already shows.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Craft {
    /// The held pickup.
    pub pickup: Pickup,
    /// A person flies the slot.
    pub person: bool,
    /// A person flying the slot holds the fire button this tick.
    pub fire_held: bool,
    /// Racing: past the start, not wrecked, not respawning.
    pub racing: bool,
    /// The lap being driven, 1-based, saturating at 255.
    pub lap: u8,
    /// The laps the race ends after; 0 when it has no target.
    pub laps_target: u8,
    /// The energy pool.
    pub shield: f32,
    /// Its maximum: the bound of a `shield-delta` for this slot.
    pub shield_max: f32,
    /// The body's speed, km/h.
    pub speed_kmh: f32,
}

impl Craft {
    /// The record.
    #[must_use]
    pub fn encode(&self) -> [u8; CRAFT_LEN] {
        let mut record = [0; CRAFT_LEN];
        record[CRAFT_PICKUP] = self.pickup.code();
        record[CRAFT_FLAGS] = [
            (self.person, CRAFT_PERSON),
            (self.fire_held, CRAFT_FIRE_HELD),
            (self.racing, CRAFT_RACING),
        ]
        .into_iter()
        .fold(0, |flags, (on, bit)| if on { flags | bit } else { flags });
        record[CRAFT_LAP] = self.lap;
        record[CRAFT_LAPS_TARGET] = self.laps_target;
        put_u32(&mut record, CRAFT_SHIELD, self.shield.to_bits());
        put_u32(&mut record, CRAFT_SHIELD_MAX, self.shield_max.to_bits());
        put_u32(&mut record, CRAFT_SPEED, self.speed_kmh.to_bits());
        record
    }

    /// The craft a record describes. A pickup code this revision does not know reads
    /// as [`Pickup::None`]; the host writes none.
    #[must_use]
    pub fn decode(record: &[u8; CRAFT_LEN]) -> Self {
        let flags = record[CRAFT_FLAGS];
        Self {
            pickup: Pickup::from_code(record[CRAFT_PICKUP]).unwrap_or_default(),
            person: flags & CRAFT_PERSON != 0,
            fire_held: flags & CRAFT_FIRE_HELD != 0,
            racing: flags & CRAFT_RACING != 0,
            lap: record[CRAFT_LAP],
            laps_target: record[CRAFT_LAPS_TARGET],
            shield: f32::from_bits(get_u32(record, CRAFT_SHIELD)),
            shield_max: f32::from_bits(get_u32(record, CRAFT_SHIELD_MAX)),
            speed_kmh: f32::from_bits(get_u32(record, CRAFT_SPEED)),
        }
    }
}

/// Something that happened in the race since the last call (README 2.1, `event`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// A craft touched a wall (`hooks.md`, `wall-contact`).
    WallContact {
        /// The grid slot.
        slot: Slot,
    },
    /// A craft holding the mod's own pickup ([`Pickup::Ours`]) asked to fire it: a
    /// person's fire press, or an AI craft's decision to fire. The pickup is kept.
    PickupFire {
        /// The grid slot.
        slot: Slot,
    },
    /// A craft absorbed the mod's own pickup: the pickup is gone and the engine paid no
    /// energy for it.
    PickupAbsorbed {
        /// The grid slot.
        slot: Slot,
    },
    /// A craft in `pad-mode` entered a speed pad and was given no boost for it.
    PadEnergy {
        /// The grid slot.
        slot: Slot,
    },
}

impl Event {
    /// The slot it happened to.
    #[must_use]
    pub const fn slot(self) -> Slot {
        match self {
            Self::WallContact { slot }
            | Self::PickupFire { slot }
            | Self::PickupAbsorbed { slot }
            | Self::PadEnergy { slot } => slot,
        }
    }

    /// The record.
    #[must_use]
    pub const fn encode(self) -> [u8; EVENT_LEN] {
        let kind = match self {
            Self::WallContact { .. } => EVENT_WALL_CONTACT,
            Self::PickupFire { .. } => EVENT_PICKUP_FIRE,
            Self::PickupAbsorbed { .. } => EVENT_PICKUP_ABSORBED,
            Self::PadEnergy { .. } => EVENT_PAD_ENERGY,
        };
        let mut record = [0; EVENT_LEN];
        record[EVENT_KIND] = kind;
        record[EVENT_SLOT] = self.slot().0;
        record
    }

    /// The event a record describes, or `None` for a kind or slot this revision does
    /// not know (a guest skips it).
    #[must_use]
    pub fn decode(record: &[u8; EVENT_LEN]) -> Option<Self> {
        let slot = Slot::new(usize::from(record[EVENT_SLOT]))?;
        Some(match record[EVENT_KIND] {
            EVENT_WALL_CONTACT => Self::WallContact { slot },
            EVENT_PICKUP_FIRE => Self::PickupFire { slot },
            EVENT_PICKUP_ABSORBED => Self::PickupAbsorbed { slot },
            EVENT_PAD_ENERGY => Self::PadEnergy { slot },
            _ => return None,
        })
    }
}

/// What a guest asks for (README 2.1, `command`). Decoded, a command has passed every
/// check that needs no observation; the host checks the rest (the slot holds a craft,
/// the shield maximum, the pickup held, one of a kind per slot).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    /// Scale the slot's engine output this tick, `factor` in `[0, MAX_THRUST_FACTOR]`:
    /// below `1.0` through `Ship::pending_thrust_scale`, above it through
    /// `Environment::extension_thrust`. No timer: a guest re-sends it each tick.
    ScaleThrust {
        /// The grid slot.
        slot: Slot,
        /// The scale, finite and in `[0, MAX_THRUST_FACTOR]`.
        factor: f32,
    },
    /// Add `amount` to the slot's energy pool (negative takes it away) through the damage
    /// path: a loss can wreck the craft as a weapon hit does, a gain is clamped at the
    /// maximum. `|amount|` at most the slot's shield maximum.
    ShieldDelta {
        /// The grid slot.
        slot: Slot,
        /// Energy, finite.
        amount: f32,
    },
    /// Put the mod's own pickup ([`Pickup::Ours`]) in the slot, which must hold none.
    /// The engine never fires or absorbs it; it raises `pickup-fire` / `pickup-absorbed`
    /// instead. One module owns the kind for the race; a race without weapons refuses it.
    GivePickup {
        /// The grid slot.
        slot: Slot,
    },
    /// Take the mod's own pickup out of the slot, which must hold it.
    ClearPickup {
        /// The grid slot.
        slot: Slot,
    },
    /// From now on, every `every`-th time a weapon pad hands out `weapon`, it hands out
    /// the mod's own pickup instead. Race-wide (written with slot 0), held until
    /// replaced; the count is the engine's, hashed.
    PickupRoll {
        /// The weapon replaced.
        weapon: Weapon,
        /// `1..=MAX_ROLL_EVERY`.
        every: u8,
    },
    /// This tick, a speed pad gives the slot no boost: on entering one the engine raises
    /// `pad-energy` instead. Sent each tick it should hold.
    PadMode {
        /// The grid slot.
        slot: Slot,
    },
    /// Play one of the title's own sounds from the craft. An output: queued beside the
    /// engine's cues, heard, never hashed.
    PlayCue {
        /// The grid slot.
        slot: Slot,
        /// The sound.
        cue: Cue,
    },
    /// Fire one of the title's own particle effects at the craft's nozzle. An output:
    /// drawn, never hashed. A title that ships no such effect draws nothing.
    FireEffect {
        /// The grid slot.
        slot: Slot,
        /// The effect.
        effect: Effect,
    },
    /// This tick, draw the slot's held-pickup backdrop in `rgba` instead of the
    /// weapon's own colour. An output: drawn, never hashed.
    PickupTint {
        /// The grid slot.
        slot: Slot,
        /// The colour.
        rgba: Rgba,
    },
    /// Race-wide (written with slot 0): play the host title's own `kind` of weather at
    /// `strength`, drawn with the enhanced weather whatever the player's setting. An
    /// output, re-sent each tick it should hold: drawn, never hashed.
    SetWeather {
        /// The kind.
        kind: WeatherKind,
        /// Finite, in `[0, 1]`.
        strength: f32,
    },
    /// Race-wide: scale the sun's draw and the lens flare's alpha by `intensity`
    /// (0 = both off). The sun as a light is untouched. An output, never hashed.
    SetSun {
        /// Finite, in `[0, 1]`.
        intensity: f32,
    },
    /// Race-wide: mix the sky toward a grey, rainy colour by `grey` (0 = the authored
    /// sky). An output, never hashed.
    GradeSky {
        /// Finite, in `[0, 1]`.
        grey: f32,
    },
}

impl Command {
    /// The slot it names; a race-wide command names none.
    #[must_use]
    pub const fn slot(&self) -> Option<Slot> {
        match *self {
            Self::ScaleThrust { slot, .. }
            | Self::ShieldDelta { slot, .. }
            | Self::GivePickup { slot }
            | Self::ClearPickup { slot }
            | Self::PadMode { slot }
            | Self::PlayCue { slot, .. }
            | Self::FireEffect { slot, .. }
            | Self::PickupTint { slot, .. } => Some(slot),
            Self::PickupRoll { .. }
            | Self::SetWeather { .. }
            | Self::SetSun { .. }
            | Self::GradeSky { .. } => None,
        }
    }

    /// The record.
    #[must_use]
    pub fn encode(&self) -> [u8; COMMAND_LEN] {
        let (kind, arg, value, word) = match *self {
            Self::ScaleThrust { factor, .. } => (COMMAND_SCALE_THRUST, 0, factor, 0),
            Self::ShieldDelta { amount, .. } => (COMMAND_SHIELD_DELTA, 0, amount, 0),
            Self::GivePickup { .. } => (COMMAND_GIVE_PICKUP, 0, 0.0, 0),
            Self::ClearPickup { .. } => (COMMAND_CLEAR_PICKUP, 0, 0.0, 0),
            Self::PickupRoll { weapon, every } => (
                COMMAND_PICKUP_ROLL,
                u16::from(weapon.code()),
                0.0,
                u32::from(every),
            ),
            Self::PadMode { .. } => (COMMAND_PAD_MODE, 0, 0.0, 0),
            Self::PlayCue { cue, .. } => (COMMAND_PLAY_CUE, cue.code(), 0.0, 0),
            Self::FireEffect { effect, .. } => (COMMAND_FIRE_EFFECT, effect.code(), 0.0, 0),
            Self::PickupTint { rgba, .. } => (COMMAND_PICKUP_TINT, 0, 0.0, rgba.to_u32()),
            Self::SetWeather { kind, strength } => (COMMAND_SET_WEATHER, kind.code(), strength, 0),
            Self::SetSun { intensity } => (COMMAND_SET_SUN, 0, intensity, 0),
            Self::GradeSky { grey } => (COMMAND_GRADE_SKY, 0, grey, 0),
        };
        let mut record = [0; COMMAND_LEN];
        record[COMMAND_KIND] = kind;
        record[COMMAND_SLOT] = self.slot().map_or(0, |slot| slot.0);
        let [lo, hi] = arg.to_le_bytes();
        record[COMMAND_ARG] = lo;
        record[COMMAND_ARG + 1] = hi;
        put_u32(&mut record, COMMAND_VALUE, value.to_bits());
        put_u32(&mut record, COMMAND_WORD, word);
        record
    }

    /// The command a record describes, with every check that needs no observation.
    ///
    /// # Errors
    ///
    /// A [`Malformed`] naming the broken rule.
    pub fn decode(record: &[u8; COMMAND_LEN]) -> Result<Self, Malformed> {
        let (kind, raw_slot) = (record[COMMAND_KIND], record[COMMAND_SLOT]);
        let arg = u16::from_le_bytes([record[COMMAND_ARG], record[COMMAND_ARG + 1]]);
        let value = f32::from_bits(get_u32(record, COMMAND_VALUE));
        let word = get_u32(record, COMMAND_WORD);
        if kind == COMMAND_PICKUP_ROLL {
            let weapon = u8::try_from(arg).ok().and_then(Weapon::from_code);
            let every = u8::try_from(word).ok().filter(|every| *every > 0);
            return match (raw_slot, weapon, every) {
                (0, Some(weapon), Some(every)) => Ok(Self::PickupRoll { weapon, every }),
                _ => Err(Malformed::Roll {
                    slot: raw_slot,
                    weapon: arg,
                    every: word,
                }),
            };
        }
        if matches!(
            kind,
            COMMAND_SET_WEATHER | COMMAND_SET_SUN | COMMAND_GRADE_SKY
        ) {
            if raw_slot != 0 || !(0.0..=1.0).contains(&value) {
                return Err(Malformed::Unit {
                    kind,
                    slot: raw_slot,
                    value,
                });
            }
            return Ok(match kind {
                COMMAND_SET_WEATHER => Self::SetWeather {
                    kind: WeatherKind::from_code(arg).ok_or(Malformed::Weather(arg))?,
                    strength: value,
                },
                COMMAND_SET_SUN => Self::SetSun { intensity: value },
                _ => Self::GradeSky { grey: value },
            });
        }
        let slot = Slot::new(usize::from(raw_slot)).ok_or(Malformed::Slot(raw_slot))?;
        Ok(match kind {
            COMMAND_SCALE_THRUST => {
                if !(0.0..=MAX_THRUST_FACTOR).contains(&value) {
                    return Err(Malformed::Factor {
                        slot,
                        factor: value,
                    });
                }
                Self::ScaleThrust {
                    slot,
                    factor: value,
                }
            }
            COMMAND_SHIELD_DELTA => {
                if !value.is_finite() {
                    return Err(Malformed::Amount {
                        slot,
                        amount: value,
                    });
                }
                Self::ShieldDelta {
                    slot,
                    amount: value,
                }
            }
            COMMAND_GIVE_PICKUP => Self::GivePickup { slot },
            COMMAND_CLEAR_PICKUP => Self::ClearPickup { slot },
            COMMAND_PAD_MODE => Self::PadMode { slot },
            COMMAND_PLAY_CUE => Self::PlayCue {
                slot,
                cue: Cue::from_code(arg).ok_or(Malformed::Cue(arg))?,
            },
            COMMAND_FIRE_EFFECT => Self::FireEffect {
                slot,
                effect: Effect::from_code(arg).ok_or(Malformed::Effect(arg))?,
            },
            COMMAND_PICKUP_TINT => Self::PickupTint {
                slot,
                rgba: Rgba::from_u32(word),
            },
            other => return Err(Malformed::Kind(other)),
        })
    }
}

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

/// Writes one tick's observation into a whole buffer, the state blob included. Events
/// past [`MAX_EVENTS`] and crafts past [`MAX_CRAFTS`] are left out.
pub fn write_observation(
    io: &mut [u8; IO_LEN],
    tick: u64,
    active: u32,
    events: &[Event],
    crafts: &[Craft],
    state: &[u8],
) {
    let events = &events[..events.len().min(MAX_EVENTS)];
    put_u32(io, ABI_AT, ABI);
    io[TICK_AT..TICK_AT + 8].copy_from_slice(&tick.to_le_bytes());
    put_u32(io, SLOTS_AT, MAX_SLOTS);
    put_u32(io, ACTIVE_AT, active);
    put_u32(io, EVENT_COUNT_AT, events.len() as u32);
    put_u32(io, STATE_LEN_AT, state.len() as u32);
    for (i, event) in events.iter().enumerate() {
        let at = EVENTS_AT + i * EVENT_LEN;
        io[at..at + EVENT_LEN].copy_from_slice(&event.encode());
    }
    io[STATE_AT..STATE_AT + state.len()].copy_from_slice(state);
    for (i, craft) in crafts.iter().take(MAX_CRAFTS).enumerate() {
        let at = CRAFTS_AT + i * CRAFT_LEN;
        io[at..at + CRAFT_LEN].copy_from_slice(&craft.encode());
    }
}

/// The reply's command count.
///
/// # Errors
///
/// [`Malformed::TooMany`] past [`MAX_COMMANDS`].
pub fn reply_count(io: &[u8; IO_LEN]) -> Result<usize, Malformed> {
    let count = get_u32(io, REPLY_AT);
    if count as usize > MAX_COMMANDS {
        return Err(Malformed::TooMany(count));
    }
    Ok(count as usize)
}

/// The reply's `index`-th command record, `index` below [`MAX_COMMANDS`].
#[must_use]
pub fn command_record(io: &[u8; IO_LEN], index: usize) -> [u8; COMMAND_LEN] {
    let at = REPLY_AT + 4 + index * COMMAND_LEN;
    let mut record = [0; COMMAND_LEN];
    record.copy_from_slice(&io[at..at + COMMAND_LEN]);
    record
}

/// A tick's buffer split into what a guest reads and what it writes, for the guest
/// side: the header and events, the state blob, the reply, the craft records.
#[derive(Debug)]
pub struct Sections<'a> {
    /// Everything before the state blob: the header and the events.
    pub head: &'a [u8],
    /// The state blob's whole area, [`MAX_STATE_BYTES`] long.
    pub state: &'a mut [u8],
    /// The reply: the count, then the command records.
    pub reply: &'a mut [u8],
    /// The craft records.
    pub crafts: &'a [u8],
}

impl<'a> Sections<'a> {
    /// Splits a whole buffer.
    #[must_use]
    pub fn split(io: &'a mut [u8; IO_LEN]) -> Self {
        let (head, rest) = io.split_at_mut(STATE_AT);
        let (state, rest) = rest.split_at_mut(MAX_STATE_BYTES);
        let (reply, crafts) = rest.split_at_mut(CRAFTS_AT - REPLY_AT);
        Self {
            head,
            state,
            reply,
            crafts,
        }
    }

    /// The world tick.
    #[must_use]
    pub fn tick(head: &[u8]) -> u64 {
        u64::from(get_u32(head, TICK_AT)) | (u64::from(get_u32(head, TICK_AT + 4)) << 32)
    }

    /// The active mask.
    #[must_use]
    pub fn active(head: &[u8]) -> u32 {
        get_u32(head, ACTIVE_AT)
    }

    /// The state blob's length as the host wrote it.
    #[must_use]
    pub fn state_len(head: &[u8]) -> usize {
        get_u32(head, STATE_LEN_AT) as usize
    }

    /// The events written, at most [`MAX_EVENTS`].
    #[must_use]
    pub fn event_count(head: &[u8]) -> usize {
        (get_u32(head, EVENT_COUNT_AT) as usize).min(MAX_EVENTS)
    }

    /// The `index`-th event record.
    #[must_use]
    pub fn event_record(head: &[u8], index: usize) -> Option<&[u8; EVENT_LEN]> {
        let at = EVENTS_AT + index * EVENT_LEN;
        head.get(at..at + EVENT_LEN)?.try_into().ok()
    }

    /// The slot's craft record.
    #[must_use]
    pub fn craft_record(crafts: &[u8], slot: Slot) -> Option<&[u8; CRAFT_LEN]> {
        let at = slot.index() * CRAFT_LEN;
        crafts.get(at..at + CRAFT_LEN)?.try_into().ok()
    }

    /// Writes the `index`-th command record of a reply section.
    pub fn put_command(reply: &mut [u8], index: usize, command: &Command) {
        let at = 4 + index * COMMAND_LEN;
        if let Some(slot) = reply.get_mut(at..at + COMMAND_LEN) {
            slot.copy_from_slice(&command.encode());
        }
    }

    /// Writes a reply section's count.
    pub fn put_count(reply: &mut [u8], count: usize) {
        put_u32(reply, 0, count as u32);
    }
}

fn put_u32(buffer: &mut [u8], at: usize, value: u32) {
    if let Some(bytes) = buffer.get_mut(at..at + 4) {
        bytes.copy_from_slice(&value.to_le_bytes());
    }
}

fn get_u32(buffer: &[u8], at: usize) -> u32 {
    match buffer.get(at..at + 4) {
        Some(&[a, b, c, d]) => u32::from_le_bytes([a, b, c, d]),
        _ => 0,
    }
}

#[cfg(test)]
mod tests;
