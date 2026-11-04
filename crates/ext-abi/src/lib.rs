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
//! | [`COURSE_LENGTH_AT`] | 4 | course length `f32`, units (0: no course) |
//! | [`TICK_AT`] | 8 | world tick |
//! | [`SLOTS_AT`] | 4 | slot count ([`MAX_SLOTS`]) |
//! | [`ACTIVE_AT`] | 4 | active mask, one bit per slot holding a craft |
//! | [`EVENT_COUNT_AT`] | 4 | event count |
//! | [`STATE_LEN_AT`] | 4 | state length |
//! | [`EVENTS_AT`] | [`EVENT_LEN`] x [`MAX_EVENTS`] | events ([`Event`]): kind `u8`, slot `u8`, index `u16`, value `f32`, then for a `mod` event the channel key `u32` and the sender key `u32` (zero in the others) |
//! | [`STATE_AT`] | [`MAX_STATE_BYTES`] | the module's state blob, in and out |
//! | [`REPLY_AT`] | 4 | command count, written by the guest |
//! | [`REPLY_AT`] + 4 | [`COMMAND_LEN`] x [`MAX_COMMANDS`] | commands ([`Command`]): kind `u8`, slot `u8`, `u16` argument, `f32`, `u32` |
//! | [`POOL_AT`] | [`POOL_LEN`] | the reply's string pool: entries of a length byte then UTF-8, named by offset from a `hud-text` or `hud-icon` command |
//! | [`CRAFTS_AT`] | [`CRAFT_LEN`] x [`MAX_CRAFTS`] | one record per grid slot ([`Craft`]): pickup `u8`, flags `u8`, lap `u8`, laps target `u8`, shield `f32`, shield maximum `f32`, speed km/h `f32` |
//! | [`TRACKING_AT`] | [`TRACKING_LEN`] x [`MAX_CRAFTS`] | one record per grid slot ([`Tracking`]): progress `f32`, place `u8`, flags `u8` (bit 0: progress is a fix), 2 bytes unused |
//! | [`TITLE_AT`] | 4 | the title's key: `channel_key` of its lowercased hyphenated name (0: none) |
//! | [`OPTIONS_AT`] | [`OPTIONS_LEN`] | the module's options, one byte each: the index of the player's pick on the mod's own choice rows |
//! | [`AIM_AT`] | [`AIM_TOTAL`] | [`AIM_SLOTS`] observers x [`AIM_SLOTS`] targets ([`Aim`]): where the target sits ahead of, right of and above the observer, three `i8` in [`AIM_STEP`] units |
//!
//! The track region is a second buffer, written by the host before each call when the
//! guest exports `oag_track() -> u32`: [`track`] lays it out.
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

pub use aiming::{AIM_AT, AIM_LEN, AIM_SLOTS, AIM_STEP, AIM_TOTAL, Aim, Aiming, read as read_aim};
pub use channel::{
    BadChannel, MAX_CHANNEL_LEN, MAX_EMITS_PER_TICK, MAX_MOD_EVENTS, MAX_TRACK_EVENTS,
    TRACK_SENDER, channel_key, check_channel, namespace,
};
pub use craft::{
    CRAFT_FIRE_HELD, CRAFT_FLAGS, CRAFT_IN_TRANSIT, CRAFT_LAP, CRAFT_LAPS_TARGET, CRAFT_PERSON,
    CRAFT_PICKUP, CRAFT_RACING, CRAFT_SHIELD, CRAFT_SHIELD_MAX, CRAFT_SHIELDED, CRAFT_SPEED, Craft,
};
pub use event::Event;
use hud::pool_string;
pub use hud::{
    Anchor, HudElement, HudPlace, HudStr, MAX_FIRE_EFFECTS_PER_TICK, MAX_HUD_EXTRA, MAX_HUD_ITEMS,
    MAX_HUD_SHAKE, MAX_HUD_TEXT,
};
pub use kind::CommandKind;
pub use malformed::Malformed;
pub use motion::{Distance, MAX_PROBE_RADIUS, MAX_SPEED_FACTOR, MAX_TRANSIT_TICKS};
pub use pickup::{Colour, Icon, ModIcon, Pickup};
pub use scene::{
    MAX_BEAM_HALF_WIDTH, MAX_BEAMS_PER_TICK, MAX_FOV_OFFSET_DEG, MAX_SHADER_INDEX,
    MAX_SHADER_PARAMS, MAX_VIEW_GENERATION, ViewUpdate,
};
pub use sound::SoundMode;
pub use tracking::{
    COURSE_LENGTH_AT, Course, OPTIONS_AT, OPTIONS_LEN, TITLE_AT, TRACKING_AT, TRACKING_FIXED,
    TRACKING_FLAGS, TRACKING_LEN, TRACKING_PLACE, TRACKING_PROGRESS, Tracking,
};

/// The ABI major. A manifest's `[extension] api` names it.
pub const ABI: u32 = 1;
/// The layout revision within [`ABI`]: bumped by hand on any edit to the layout. A
/// module built before revisions existed returns the bare major and is revision 0.
pub const REVISION: u32 = 3;
const _: () = assert!(REVISION > 0 && REVISION < 256);
const _: () = assert!(CRAFTS_AT + CRAFT_LEN * MAX_CRAFTS <= IO_LEN);
const _: () = assert!(MAX_CRAFTS <= MAX_SLOTS as usize);
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
/// Events per call. Past it the later are dropped by the caller, which counts and logs
/// them (the mod events come first, so only the engine's own can be).
pub const MAX_EVENTS: usize = 32;
/// The most linear memory a module may ask for, in 64 KiB pages (16 MiB). Proposed in the
/// README (section 4.1), chosen.
pub const MAX_MEMORY_PAGES: u32 = 256;
/// The longest state blob.
pub const MAX_STATE_BYTES: usize = 1024;
/// Commands per reply; more is a fault (README 2.1 proposed 64; 96 leaves a slot's
/// [`MAX_HUD_ITEMS`] draw items room beside the rest). Chosen.
pub const MAX_COMMANDS: usize = 96;
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
pub const EVENT_LEN: usize = 16;
/// Where the state blob starts.
pub const STATE_AT: usize = EVENTS_AT + EVENT_LEN * MAX_EVENTS;
/// Where the guest writes its reply: the count, then the records.
pub const REPLY_AT: usize = STATE_AT + MAX_STATE_BYTES;
/// One command record.
pub const COMMAND_LEN: usize = 12;
/// Where the reply's string pool starts: after the commands, inside the reply section.
/// A `hud-text` or `hud-icon` command names its string by offset into it.
pub const POOL_AT: usize = REPLY_AT + 4 + COMMAND_LEN * MAX_COMMANDS;
/// The pool's length. Chosen.
pub const POOL_LEN: usize = 768;
/// Where the craft records start.
pub const CRAFTS_AT: usize = POOL_AT + POOL_LEN;
/// One craft record.
pub const CRAFT_LEN: usize = 16;

/// Within an event record: the kind code.
pub const EVENT_KIND: usize = 0;
/// Within an event record: the slot.
pub const EVENT_SLOT: usize = 1;
/// Within an event record: the pad or trigger index, `u16`.
pub const EVENT_INDEX: usize = 2;
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
/// Within an event record: the `f32` of an event that carries an amount (`hit`).
pub const EVENT_VALUE: usize = 4;
/// Offset of a `mod` event's channel key, a `u32` ([`channel_key`]).
pub const EVENT_CHANNEL: usize = 8;
/// Offset of a `mod` event's sender, a `u32`: the key of the sending mod's id, or
/// [`TRACK_SENDER`].
pub const EVENT_FROM: usize = 12;

/// The largest `scale-pad` factor, race-wide or per slot. Chosen.
pub const MAX_PAD_FACTOR: f32 = 2.0;
/// The largest `scale-thrust` factor: a mod may double an engine's thrust, no more.
/// Chosen (README 2.1 proposed `[0.25, 2.0]`; the lower end is `0` here).
pub const MAX_THRUST_FACTOR: f32 = 2.0;
/// The longest `pickup-roll` period: every 1st up to every 255th grant.
pub const MAX_ROLL_EVERY: u32 = 255;

/// Event kind codes, in [`Event`]'s order from 1.
const EVENT_WALL_CONTACT: u8 = 1;
const EVENT_PICKUP_FIRE: u8 = 2;
const EVENT_PICKUP_ABSORBED: u8 = 3;
const EVENT_SPEED_PAD: u8 = 5;
const EVENT_WEAPON_PAD: u8 = 6;
const EVENT_TRIGGER_ENTER: u8 = 7;
const EVENT_TRIGGER_EXIT: u8 = 8;
const EVENT_HIT: u8 = 9;
const EVENT_MOD: u8 = 10;
const EVENT_TRANSIT_END: u8 = 11;
const EVENT_CLEARANCE: u8 = 12;
/// Bit 15 of a `hud-text`, `hud-icon` or `fire-effect` argument: the string or effect is
/// the sending mod's own, not the title's. The pool offset or effect index is the rest.
const OWN_FLAG: u16 = 1 << 15;
/// The craft record's pickup code for a simulation mod's own pickup.
const PICKUP_OURS: u8 = u8::MAX;
/// The craft record's first pickup code for another mod's pickup: [`MAX_PICKUP_KINDS`] codes
/// from the code after the last title weapon.
const PICKUP_OTHER_FIRST: u8 = Weapon::Disruptor.code() + 1;
/// The mods that can each hand out a pickup of their own in one race.
pub const MAX_PICKUP_KINDS: u8 = 32;
const PICKUP_OTHER_LAST: u8 = PICKUP_OTHER_FIRST + MAX_PICKUP_KINDS - 1;
const _: () = assert!(PICKUP_OTHER_LAST < PICKUP_OURS);

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
    /// Race-wide (written with slot 0): from now on, speed pad `pad` pushes `factor`
    /// times its own push, held until replaced. The engine's, hashed.
    ScalePad {
        /// The pad, among the track's speed pads.
        pad: u16,
        /// Finite, in `[0, MAX_PAD_FACTOR]`.
        factor: f32,
    },
    /// This tick, every speed pad pushes the slot `factor` times as hard. Re-sent each
    /// tick it should hold; hashed. The push is the pad's factor times the slot's; at
    /// 0 the pad gives no push, no plume and no sound, and `speed-pad` still fires.
    ScaleSlotPads {
        /// The grid slot.
        slot: Slot,
        /// Finite, in `[0, MAX_PAD_FACTOR]`.
        factor: f32,
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
    /// Fire one of the sending mod's own particle effects (`[effects]` of its manifest)
    /// at the craft's nozzle. An output: drawn, never hashed. Shares `fire-effect`'s
    /// code, told apart by bit 15 of the argument; an `index` the mod has no effect at
    /// draws nothing, and the load report names the effects that did not load.
    FireModEffect {
        /// The grid slot.
        slot: Slot,
        /// The effect's place in the mod's `[effects]`, in name order (15 bits).
        index: u16,
    },
    /// This tick, recolour the slot's HUD `element`. An output, re-sent each tick it
    /// should hold: drawn, never hashed. A title without the element draws nothing.
    HudTint {
        /// The grid slot.
        slot: Slot,
        /// The element.
        element: HudElement,
        /// The colour.
        rgba: Rgba,
    },
    /// This tick, jitter the slot's HUD `element` by up to `amplitude` HUD pixels
    /// (finite, in `[0, MAX_HUD_SHAKE]`). An output, never hashed.
    HudShake {
        /// The grid slot.
        slot: Slot,
        /// The element.
        element: HudElement,
        /// HUD pixels.
        amplitude: f32,
    },
    /// This tick, the slot's bar `element` also shows `extra` times its value on top,
    /// the bar rescaled so value x (1 + extra) fills it: the base keeps its colour and
    /// the added share blends into `rgba`. `extra` finite, in `(0, MAX_HUD_EXTRA]`.
    /// An output, never hashed.
    HudBarExtra {
        /// The grid slot.
        slot: Slot,
        /// The element, a bar.
        element: HudElement,
        /// The added share of the value, 0.9 for a turbo of x1.9.
        extra: f32,
        /// The colour the added share blends to.
        rgba: Rgba,
    },
    /// This tick, draw `text` on the slot's HUD at `place`, in the title's HUD font. An
    /// output, never hashed.
    HudText {
        /// The grid slot, and the view of it in split screen.
        slot: Slot,
        /// Where.
        place: HudPlace,
        /// The colour.
        rgba: Rgba,
        /// The text, or with `own` the id of a string in the sending mod's `strings/<language>.toml`.
        text: HudStr,
        /// `text` is a key into the mod's own string table, resolved by the host in the
        /// game's language (English when the language lacks the key).
        own: bool,
    },
    /// This tick, draw the title's HUD sprite named `sprite` on the slot's HUD at
    /// `place`, `rgba` its modulating colour. An output, never hashed.
    HudIcon {
        /// The grid slot, and the view of it in split screen.
        slot: Slot,
        /// Where.
        place: HudPlace,
        /// The colour.
        rgba: Rgba,
        /// The sprite's name in the title's HUD layout, or with `own` the name of an
        /// `[icons]` entry of the sending mod.
        sprite: HudStr,
        /// `sprite` names one of the mod's own PNGs.
        own: bool,
    },
    /// This tick, the look of the slot's held mod pickup: its icon and its backdrop's
    /// colour, each naming its source. An output, re-sent each tick it should hold
    /// (else the manifest's `[pickup]` holds): drawn, never hashed. The title's own
    /// fade, overshoot and size pulse play on whatever look it is. `hud-tint` no longer
    /// reaches the pickup.
    PickupLook {
        /// The grid slot.
        slot: Slot,
        /// The icon.
        icon: Icon,
        /// The backdrop's colour.
        colour: Colour,
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
    /// Start the title's own Shield on the slot, for the title's authored time, as
    /// firing a held Shield pickup does. A Shield already running is not refreshed (the
    /// original's rule); a race without weapons refuses it.
    GiveShield {
        /// The grid slot.
        slot: Slot,
    },
    /// Keep the slot's running Shield from running out this tick. Re-sent each tick it
    /// should hold; hashed. It never starts one, and a Shield not held ends within two
    /// ticks of the last send.
    HoldShield {
        /// The grid slot.
        slot: Slot,
    },
    /// End the slot's running Shield now, before the tick's step.
    ClearShield {
        /// The grid slot.
        slot: Slot,
    },
    /// At its next fork, the slot's driver takes route `route` (an index into the track
    /// region's routes) instead of the one its coin drew, if the route leaves that fork
    /// and the driver can drive it. Re-sent each tick it should hold; hashed. The coin is
    /// still thrown, so the race's random streams are unchanged. An AI craft's alone: a
    /// person steers.
    PreferRoute {
        /// The grid slot.
        slot: Slot,
        /// The route, below the region's route count.
        route: u16,
    },
    /// Send `value` on `channel` about the slot: every module receives a `mod` event for
    /// it on the next tick, the track's and the mods' together, in an order that does not
    /// depend on which module sent. The channel must be one the sending mod publishes
    /// (`oag-mod.toml` `[channels]`), at most [`MAX_EMITS_PER_TICK`] a module a tick;
    /// either rule broken faults the module. The queue is race state: hashed.
    Emit {
        /// The craft it is about.
        slot: Slot,
        /// The channel ([`channel_key`]).
        channel: u32,
        /// What to attach, finite.
        value: f32,
    },
    /// Take the craft out of the world for `ticks` ticks, then put it `distance` further
    /// round the circuit at `exit_factor` times the speed it had when this landed. Out of
    /// the world it is frozen where it was: not stepped, not drawn, not collided with,
    /// not hit, its controls and pickup presses ignored, its reset triggers held. Its
    /// laps count through the jump; a finish in it is timed at the exit tick. A
    /// `transit-end` event follows the exit. Refused (a fault) on a track with no
    /// course, a craft not racing or already in one, or a distance not inside
    /// `(0, half the lap)`.
    Transit {
        /// The grid slot.
        slot: Slot,
        /// Ticks out of the world, `1..=MAX_TRANSIT_TICKS`.
        ticks: u16,
        /// How far round it comes out.
        distance: Distance,
        /// Its exit speed over its entry speed, `[0, MAX_SPEED_FACTOR]`, in steps of
        /// `1 / EXIT_FACTOR_STEPS`.
        exit_factor: f32,
    },
    /// Multiply the craft's velocity by `factor`, once, at the call.
    ScaleSpeed {
        /// The grid slot.
        slot: Slot,
        /// The scale, in `[0, MAX_SPEED_FACTOR]`.
        factor: f32,
    },
    /// This tick, wash the slot's view with `rgba` over the whole picture (the alpha is
    /// the strength). Re-sent each tick it should hold; never hashed. The slot's own
    /// view: a split-screen seat flown by that slot wears it, no other. With a `shader`
    /// the sending mod's shader draws the fill, handed `rgba` as its colour, in place of
    /// the additive wash; one that did not load draws nothing.
    ScreenFill {
        /// The grid slot whose view is filled.
        slot: Slot,
        /// The colour, straight alpha.
        rgba: Rgba,
        /// The shader, by its place in the mod's `[shaders]`, at most [`MAX_SHADER_INDEX`].
        shader: Option<u16>,
    },
    /// This tick, widen (positive) or narrow (negative) the vertical field of view of the
    /// slot's view by `degrees`, in `[-MAX_FOV_OFFSET_DEG, MAX_FOV_OFFSET_DEG]`, before the
    /// player's setting and the aspect fit. Re-sent each tick it should hold; never hashed.
    FovOffset {
        /// The grid slot whose view changes.
        slot: Slot,
        /// Degrees.
        degrees: f32,
    },
    /// Play the sending mod's own effect number `index` (its place in `[effects]`) at the
    /// course point `distance` ahead of the craft, on the road's centre line, where it
    /// stays for the effect's own lifetime: a sprite of the mod's texture standing in the
    /// world (a portal). Counts toward [`MAX_FIRE_EFFECTS_PER_TICK`]; a distance that
    /// does not resolve draws nothing. Never hashed.
    FireModEffectAhead {
        /// The grid slot the point is ahead of.
        slot: Slot,
        /// The effect, by its place in the mod's `[effects]`.
        index: u16,
        /// How far ahead along the circuit.
        distance: Distance,
    },
    /// This tick, draw a ribbon facing the camera from the craft in `slot` to the craft in
    /// `to`, flat `rgba` or through the sending mod's shader. Re-sent each tick it should
    /// hold; draws nothing while either craft is out of the world, or with a shader that
    /// did not load. Never hashed. At most [`MAX_BEAMS_PER_TICK`].
    DrawBeam {
        /// The craft it starts at.
        slot: Slot,
        /// The craft it ends at, never `slot`.
        to: Slot,
        /// The colour, straight alpha.
        rgba: Rgba,
        /// Half its width in world units, above 0 and at most [`MAX_BEAM_HALF_WIDTH`].
        half_width: f32,
        /// The shader, by its place in the mod's `[shaders]`, at most [`MAX_SHADER_INDEX`].
        shader: Option<u16>,
    },
    /// This tick, scalar `index` (its place in the shader's header, under
    /// [`MAX_SHADER_PARAMS`]) of the sending mod's shader, as the view of `slot` draws it.
    /// Re-sent each tick it should hold; unset, the header's default. Never hashed.
    SetShaderParam {
        /// The grid slot whose view reads it.
        slot: Slot,
        /// The shader, by its place in the mod's `[shaders]`.
        shader: u16,
        /// The scalar.
        index: u8,
        /// Its value, finite.
        value: f32,
    },
    /// This tick, the view of `slot` wants what its camera would see through a portal pair on
    /// the course, in the texture a mod shader reads as `engine_texture`. Re-sent each tick
    /// it should hold; a progress off the lap, or a race with no shader, asks for nothing.
    /// Never hashed.
    PortalView {
        /// The grid slot whose camera looks through it.
        slot: Slot,
        /// The entrance, its progress on the lap in track units.
        entrance: f32,
        /// The exit, its progress on the lap in track units.
        exit: f32,
        /// How far from the entrance's centre the surface reaches, in whole world units,
        /// from 1: what decides whether the view has the portal in it.
        radius: u8,
        /// When the texture is drawn.
        update: ViewUpdate,
        /// Under [`ViewUpdate::Once`], a new value draws the snapshot again. Under
        /// [`MAX_VIEW_GENERATION`].
        generation: u8,
    },
    /// Play the sending mod's own sound number `index` (its place in `[sounds]`, name
    /// order) from the craft, placed like a title cue (positioned, HRTF and rear cue
    /// apply; the player's own craft is heard dry). One the mod lacks or that did not
    /// load plays nothing and the load report says so. Heard, never hashed.
    PlayModSound {
        /// The grid slot the sound comes from.
        slot: Slot,
        /// The sound, by its place in the mod's `[sounds]`.
        index: u16,
        /// Once, a loop to start, or a loop to stop.
        mode: SoundMode,
    },
    /// Ask whether a disc of `radius` standing upright across the track `distance` ahead
    /// of the craft fits without touching track geometry: answered by a `clearance`
    /// event at the next call.
    ProbeClearance {
        /// The grid slot.
        slot: Slot,
        /// How far ahead along the circuit.
        distance: Distance,
        /// The disc's radius, `(0, MAX_PROBE_RADIUS]`.
        radius: f32,
    },
}

impl Command {
    /// Its record's kind.
    #[must_use]
    pub const fn kind(&self) -> CommandKind {
        match *self {
            Self::ScaleThrust { .. } => CommandKind::ScaleThrust,
            Self::ShieldDelta { .. } => CommandKind::ShieldDelta,
            Self::GivePickup { .. } => CommandKind::GivePickup,
            Self::ClearPickup { .. } => CommandKind::ClearPickup,
            Self::PickupRoll { .. } => CommandKind::PickupRoll,
            Self::ScalePad { .. } => CommandKind::ScalePad,
            Self::ScaleSlotPads { .. } => CommandKind::ScaleSlotPads,
            Self::PlayCue { .. } => CommandKind::PlayCue,
            Self::FireEffect { .. } | Self::FireModEffect { .. } => CommandKind::FireEffect,
            Self::HudTint { .. } => CommandKind::HudTint,
            Self::HudShake { .. } => CommandKind::HudShake,
            Self::HudBarExtra { .. } => CommandKind::HudBarExtra,
            Self::HudText { .. } => CommandKind::HudText,
            Self::HudIcon { .. } => CommandKind::HudIcon,
            Self::PickupLook { .. } => CommandKind::PickupLook,
            Self::SetWeather { .. } => CommandKind::SetWeather,
            Self::SetSun { .. } => CommandKind::SetSun,
            Self::GradeSky { .. } => CommandKind::GradeSky,
            Self::GiveShield { .. } => CommandKind::GiveShield,
            Self::HoldShield { .. } => CommandKind::HoldShield,
            Self::ClearShield { .. } => CommandKind::ClearShield,
            Self::PreferRoute { .. } => CommandKind::PreferRoute,
            Self::Emit { .. } => CommandKind::Emit,
            Self::Transit { .. } => CommandKind::Transit,
            Self::ScaleSpeed { .. } => CommandKind::ScaleSpeed,
            Self::ProbeClearance { .. } => CommandKind::ProbeClearance,
            Self::ScreenFill { .. } => CommandKind::ScreenFill,
            Self::FovOffset { .. } => CommandKind::FovOffset,
            Self::FireModEffectAhead { .. } => CommandKind::FireModEffectAhead,
            Self::DrawBeam { .. } => CommandKind::DrawBeam,
            Self::SetShaderParam { .. } => CommandKind::SetShaderParam,
            Self::PortalView { .. } => CommandKind::PortalView,
            Self::PlayModSound { .. } => CommandKind::PlayModSound,
        }
    }

    /// The slot it names; a race-wide command names none.
    #[must_use]
    pub const fn slot(&self) -> Option<Slot> {
        match *self {
            Self::ScaleThrust { slot, .. }
            | Self::ShieldDelta { slot, .. }
            | Self::GivePickup { slot }
            | Self::ClearPickup { slot }
            | Self::ScaleSlotPads { slot, .. }
            | Self::PlayCue { slot, .. }
            | Self::FireEffect { slot, .. }
            | Self::FireModEffect { slot, .. }
            | Self::HudTint { slot, .. }
            | Self::HudShake { slot, .. }
            | Self::HudBarExtra { slot, .. }
            | Self::HudText { slot, .. }
            | Self::HudIcon { slot, .. }
            | Self::PickupLook { slot, .. }
            | Self::GiveShield { slot }
            | Self::HoldShield { slot }
            | Self::ClearShield { slot }
            | Self::PreferRoute { slot, .. }
            | Self::Emit { slot, .. }
            | Self::Transit { slot, .. }
            | Self::ScaleSpeed { slot, .. }
            | Self::ProbeClearance { slot, .. }
            | Self::ScreenFill { slot, .. }
            | Self::FovOffset { slot, .. }
            | Self::FireModEffectAhead { slot, .. }
            | Self::DrawBeam { slot, .. }
            | Self::SetShaderParam { slot, .. }
            | Self::PortalView { slot, .. }
            | Self::PlayModSound { slot, .. } => Some(slot),
            Self::PickupRoll { .. }
            | Self::ScalePad { .. }
            | Self::SetWeather { .. }
            | Self::SetSun { .. }
            | Self::GradeSky { .. } => None,
        }
    }

    /// The record, with a pool offset of 0: a `hud-text` or `hud-icon` record is only
    /// whole through [`Self::encode_pooled`].
    #[must_use]
    pub fn encode(&self) -> [u8; COMMAND_LEN] {
        self.encode_pooled(0)
    }

    /// The record, a `hud-text` or `hud-icon` naming its string at `pool_at` in the
    /// reply's pool.
    #[must_use]
    pub fn encode_pooled(&self, pool_at: u16) -> [u8; COMMAND_LEN] {
        let (arg, value, word) = match *self {
            Self::ScaleThrust { factor, .. } => (0, factor.to_bits(), 0),
            Self::ShieldDelta { amount, .. } => (0, amount.to_bits(), 0),
            Self::GivePickup { .. } => (0, 0, 0),
            Self::ClearPickup { .. } => (0, 0, 0),
            Self::PickupRoll { weapon, every } => (u16::from(weapon.code()), 0, u32::from(every)),
            Self::ScalePad { pad, factor } => (pad, factor.to_bits(), 0),
            Self::ScaleSlotPads { factor, .. } => (0, factor.to_bits(), 0),
            Self::PlayCue { cue, .. } => (cue.code(), 0, 0),
            Self::FireEffect { effect, .. } => (effect.code(), 0, 0),
            Self::FireModEffect { index, .. } => (index | OWN_FLAG, 0, 0),
            Self::HudTint { element, rgba, .. } => (element.code(), 0, rgba.to_u32()),
            Self::HudShake {
                element, amplitude, ..
            } => (element.code(), amplitude.to_bits(), 0),
            Self::HudBarExtra {
                element,
                extra,
                rgba,
                ..
            } => (element.code(), extra.to_bits(), rgba.to_u32()),
            Self::HudText {
                place, rgba, own, ..
            } => (
                pool_at | if own { OWN_FLAG } else { 0 },
                place.to_bits(),
                rgba.to_u32(),
            ),
            Self::HudIcon {
                place, rgba, own, ..
            } => (
                pool_at | if own { OWN_FLAG } else { 0 },
                place.to_bits(),
                rgba.to_u32(),
            ),
            Self::PickupLook { icon, colour, .. } => {
                let (value, word) = match colour {
                    Colour::Title(weapon) => (u32::from(weapon.code()), 0),
                    Colour::Rgba(rgba) => (0, rgba.to_u32()),
                };
                let arg = match icon {
                    Icon::Title(weapon) => u16::from(weapon.code()),
                    Icon::Mod(_) => pool_at | OWN_FLAG,
                };
                (arg, value, word)
            }
            Self::SetWeather { kind, strength } => (kind.code(), strength.to_bits(), 0),
            Self::SetSun { intensity } => (0, intensity.to_bits(), 0),
            Self::GradeSky { grey } => (0, grey.to_bits(), 0),
            Self::GiveShield { .. } => (0, 0, 0),
            Self::HoldShield { .. } => (0, 0, 0),
            Self::ClearShield { .. } => (0, 0, 0),
            Self::PreferRoute { route, .. } => (route, 0, 0),
            Self::Emit { channel, value, .. } => (0, value.to_bits(), channel),
            Self::Transit { .. } | Self::ScaleSpeed { .. } | Self::ProbeClearance { .. } => {
                motion::encode(self).unwrap_or_default()
            }
            Self::ScreenFill { .. }
            | Self::FovOffset { .. }
            | Self::FireModEffectAhead { .. }
            | Self::DrawBeam { .. }
            | Self::SetShaderParam { .. }
            | Self::PortalView { .. } => scene::encode(self).unwrap_or_default(),
            Self::PlayModSound { .. } => sound::encode(self).unwrap_or_default(),
        };
        let mut record = [0; COMMAND_LEN];
        record[COMMAND_KIND] = self.kind().code();
        record[COMMAND_SLOT] = self.slot().map_or(0, |slot| slot.0);
        let [lo, hi] = arg.to_le_bytes();
        record[COMMAND_ARG] = lo;
        record[COMMAND_ARG + 1] = hi;
        put_u32(&mut record, COMMAND_VALUE, value);
        put_u32(&mut record, COMMAND_WORD, word);
        record
    }

    /// The command a record describes, with every check that needs no observation.
    ///
    /// # Errors
    ///
    /// A [`Malformed`] naming the broken rule.
    pub fn decode(record: &[u8; COMMAND_LEN]) -> Result<Self, Malformed> {
        Self::decode_in(record, &[])
    }

    /// [`Self::decode`] with the reply's string pool, which a `hud-text` or `hud-icon`
    /// record reads its string from.
    ///
    /// # Errors
    ///
    /// A [`Malformed`] naming the broken rule.
    pub fn decode_in(record: &[u8; COMMAND_LEN], pool: &[u8]) -> Result<Self, Malformed> {
        let (raw_kind, raw_slot) = (record[COMMAND_KIND], record[COMMAND_SLOT]);
        let known = CommandKind::from_code(raw_kind);
        let arg = u16::from_le_bytes([record[COMMAND_ARG], record[COMMAND_ARG + 1]]);
        let value = f32::from_bits(get_u32(record, COMMAND_VALUE));
        let word = get_u32(record, COMMAND_WORD);
        if known == Some(CommandKind::PickupRoll) {
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
            known,
            Some(CommandKind::ScalePad | CommandKind::ScaleSlotPads)
        ) && !(0.0..=MAX_PAD_FACTOR).contains(&value)
        {
            return Err(Malformed::PadFactor {
                slot: raw_slot,
                factor: value,
            });
        }
        if known == Some(CommandKind::ScalePad) {
            if raw_slot != 0 {
                return Err(Malformed::PadSlot { slot: raw_slot });
            }
            return Ok(Self::ScalePad {
                pad: arg,
                factor: value,
            });
        }
        if let Some(
            kind @ (CommandKind::SetWeather | CommandKind::SetSun | CommandKind::GradeSky),
        ) = known
        {
            if raw_slot != 0 || !(0.0..=1.0).contains(&value) {
                return Err(Malformed::Unit {
                    kind: raw_kind,
                    slot: raw_slot,
                    value,
                });
            }
            return Ok(match kind {
                CommandKind::SetWeather => Self::SetWeather {
                    kind: WeatherKind::from_code(arg).ok_or(Malformed::Weather(arg))?,
                    strength: value,
                },
                CommandKind::SetSun => Self::SetSun { intensity: value },
                _ => Self::GradeSky { grey: value },
            });
        }
        let slot = Slot::new(usize::from(raw_slot)).ok_or(Malformed::Slot(raw_slot))?;
        let hud_element = || HudElement::from_code(arg).ok_or(Malformed::HudElement(arg));
        let kind = known.ok_or(Malformed::Kind(raw_kind))?;
        Ok(match kind {
            CommandKind::ScaleThrust => {
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
            CommandKind::ShieldDelta => {
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
            CommandKind::GivePickup => Self::GivePickup { slot },
            CommandKind::ClearPickup => Self::ClearPickup { slot },
            CommandKind::ScaleSlotPads => Self::ScaleSlotPads {
                slot,
                factor: value,
            },
            CommandKind::PlayCue => Self::PlayCue {
                slot,
                cue: Cue::from_code(arg).ok_or(Malformed::Cue(arg))?,
            },
            CommandKind::FireEffect if arg & OWN_FLAG != 0 => Self::FireModEffect {
                slot,
                index: arg & !OWN_FLAG,
            },
            CommandKind::FireEffect => Self::FireEffect {
                slot,
                effect: Effect::from_code(arg).ok_or(Malformed::Effect(arg))?,
            },
            CommandKind::HudTint if arg == HudElement::Pickup.code() => {
                return Err(Malformed::PickupTint);
            }
            CommandKind::HudTint => Self::HudTint {
                slot,
                element: hud_element()?,
                rgba: Rgba::from_u32(word),
            },
            CommandKind::HudShake => {
                if !(0.0..=MAX_HUD_SHAKE).contains(&value) {
                    return Err(Malformed::HudValue {
                        kind: raw_kind,
                        value,
                    });
                }
                Self::HudShake {
                    slot,
                    element: hud_element()?,
                    amplitude: value,
                }
            }
            CommandKind::HudBarExtra => {
                if !(value > 0.0 && value <= MAX_HUD_EXTRA) {
                    return Err(Malformed::HudValue {
                        kind: raw_kind,
                        value,
                    });
                }
                Self::HudBarExtra {
                    slot,
                    element: hud_element()?,
                    extra: value,
                    rgba: Rgba::from_u32(word),
                }
            }
            CommandKind::HudText | CommandKind::HudIcon => {
                let place = HudPlace::from_bits(get_u32(record, COMMAND_VALUE))
                    .ok_or(Malformed::HudPlace)?;
                let own = arg & OWN_FLAG != 0;
                let at = arg & !OWN_FLAG;
                let string = pool_string(pool, at).ok_or(Malformed::Pool(at))?;
                let rgba = Rgba::from_u32(word);
                if kind == CommandKind::HudText {
                    Self::HudText {
                        slot,
                        place,
                        rgba,
                        text: string,
                        own,
                    }
                } else {
                    Self::HudIcon {
                        slot,
                        place,
                        rgba,
                        sprite: string,
                        own,
                    }
                }
            }
            CommandKind::PickupLook => {
                let raw = get_u32(record, COMMAND_VALUE);
                let bad = Malformed::PickupLook { arg, value: raw };
                let icon = if arg & OWN_FLAG != 0 {
                    let at = arg & !OWN_FLAG;
                    Icon::Mod(ModIcon::from_string(
                        pool_string(pool, at).ok_or(Malformed::Pool(at))?,
                    ))
                } else {
                    Icon::Title(
                        u8::try_from(arg)
                            .ok()
                            .and_then(Weapon::from_code)
                            .ok_or(bad)?,
                    )
                };
                let colour = match raw {
                    0 => Colour::Rgba(Rgba::from_u32(word)),
                    code => Colour::Title(
                        u8::try_from(code)
                            .ok()
                            .and_then(Weapon::from_code)
                            .ok_or(bad)?,
                    ),
                };
                Self::PickupLook { slot, icon, colour }
            }
            CommandKind::GiveShield => Self::GiveShield { slot },
            CommandKind::HoldShield => Self::HoldShield { slot },
            CommandKind::ClearShield => Self::ClearShield { slot },
            CommandKind::PreferRoute => Self::PreferRoute { slot, route: arg },
            CommandKind::Transit | CommandKind::ScaleSpeed | CommandKind::ProbeClearance => {
                return motion::decode(kind, slot, (arg, value, word));
            }
            CommandKind::ScreenFill
            | CommandKind::FovOffset
            | CommandKind::FireModEffectAhead
            | CommandKind::DrawBeam
            | CommandKind::SetShaderParam
            | CommandKind::PortalView => {
                return scene::decode(kind, slot, (arg, value, word));
            }
            CommandKind::PlayModSound => return sound::decode(slot, arg, word),
            CommandKind::Emit => {
                if !value.is_finite() {
                    return Err(Malformed::EmitValue { slot, value });
                }
                Self::Emit {
                    slot,
                    channel: word,
                    value,
                }
            }
            // The race-wide kinds returned above.
            CommandKind::PickupRoll
            | CommandKind::ScalePad
            | CommandKind::SetWeather
            | CommandKind::SetSun
            | CommandKind::GradeSky => return Err(Malformed::Kind(raw_kind)),
        })
    }
}
/// Writes one tick's observation into a whole buffer, the state blob included. Events
/// past [`MAX_EVENTS`] and crafts and tracking records past [`MAX_CRAFTS`] are left out.
pub fn write_observation(
    io: &mut [u8; IO_LEN],
    tick: u64,
    active: u32,
    events: &[Event],
    crafts: &[Craft],
    course: &Course<'_>,
    state: &[u8],
) {
    let events = &events[..events.len().min(MAX_EVENTS)];
    put_u32(io, ABI_AT, ABI);
    put_u32(io, COURSE_LENGTH_AT, course.length.to_bits());
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
    for (i, tracking) in course.tracking.iter().take(MAX_CRAFTS).enumerate() {
        let at = TRACKING_AT + i * TRACKING_LEN;
        io[at..at + TRACKING_LEN].copy_from_slice(&tracking.encode());
    }
    put_u32(io, TITLE_AT, course.title);
    if let Some(aiming) = course.aiming {
        aiming.write(io);
    }
    io[OPTIONS_AT..OPTIONS_AT + OPTIONS_LEN].copy_from_slice(&course.options);
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

    /// The course length, units; 0 when the race has no course.
    #[must_use]
    pub fn course_length(head: &[u8]) -> f32 {
        f32::from_bits(get_u32(head, COURSE_LENGTH_AT))
    }

    /// The slot's tracking record; `crafts` is [`Self::crafts`], which starts at
    /// [`CRAFTS_AT`].
    #[must_use]
    pub fn tracking_record(crafts: &[u8], slot: Slot) -> Option<&[u8; TRACKING_LEN]> {
        let at = (TRACKING_AT - CRAFTS_AT) + slot.index() * TRACKING_LEN;
        crafts.get(at..at + TRACKING_LEN)?.try_into().ok()
    }

    /// The module's `n`-th option ([`OPTIONS_AT`]); `crafts` is [`Self::crafts`], which
    /// starts at [`CRAFTS_AT`]. 0 past [`OPTIONS_LEN`].
    #[must_use]
    pub fn option(crafts: &[u8], n: usize) -> u8 {
        if n >= OPTIONS_LEN {
            return 0;
        }
        crafts.get(OPTIONS_AT - CRAFTS_AT + n).copied().unwrap_or(0)
    }

    /// The title's key; `crafts` is [`Self::crafts`], which starts at [`CRAFTS_AT`].
    #[must_use]
    pub fn title(crafts: &[u8]) -> u32 {
        get_u32(crafts, TITLE_AT - CRAFTS_AT)
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

mod aiming;
mod channel;
mod craft;
mod event;
pub mod hud;
mod kind;
mod malformed;
pub mod motion;
mod pickup;
mod scene;
mod sound;
pub mod track;
mod tracking;

#[cfg(test)]
mod tests;
