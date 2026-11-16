//! One grid slot as a guest reads it: the craft record.

use crate::{CRAFT_LEN, Pickup, get_u32, put_u32};

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
/// Craft flag: the craft's Shield is running, so a weapon hit this tick is absorbed.
pub const CRAFT_SHIELDED: u8 = 8;
/// Craft flag: the craft is in a `transit`, out of the world.
pub const CRAFT_IN_TRANSIT: u8 = 16;

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
    /// The craft's Shield is running: a weapon hit is absorbed (it still raises `hit`).
    pub shielded: bool,
    /// The craft is in a `transit`, out of the world.
    pub in_transit: bool,
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

/// Km/h in a track unit a second: what the host multiplies a body's speed by for
/// [`Craft::speed_kmh`], and a guest divides by to have units back.
pub const SPEED_TO_KMH: f32 = 3.6;

/// The simulation's ticks a second: what a count of ticks (a transit's, an effect's life) is
/// a time in, and a speed in units a second is a distance a tick by.
pub const TICKS_PER_SECOND: f32 = 60.0;

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
            (self.shielded, CRAFT_SHIELDED),
            (self.in_transit, CRAFT_IN_TRANSIT),
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
            shielded: flags & CRAFT_SHIELDED != 0,
            in_transit: flags & CRAFT_IN_TRANSIT != 0,
            lap: record[CRAFT_LAP],
            laps_target: record[CRAFT_LAPS_TARGET],
            shield: f32::from_bits(get_u32(record, CRAFT_SHIELD)),
            shield_max: f32::from_bits(get_u32(record, CRAFT_SHIELD_MAX)),
            speed_kmh: f32::from_bits(get_u32(record, CRAFT_SPEED)),
        }
    }
}
