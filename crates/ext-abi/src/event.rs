//! The events a module is handed: what happened to a craft, as a 16-byte record of the
//! observation.

use super::*;

/// Something that happened in the race since the last call (README 2.1, `event`).
#[derive(Debug, Clone, Copy, PartialEq)]
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
    /// A craft entered a speed pad: its index among the track's speed pads, the order
    /// of the track region's speed-pad entities. Raised on every entry, whatever the
    /// pad's `scale-pad` factor.
    SpeedPad {
        /// The grid slot.
        slot: Slot,
        /// The pad.
        pad: u16,
    },
    /// A craft entered a weapon pad, cooling down or not, granting or not.
    WeaponPad {
        /// The grid slot.
        slot: Slot,
        /// The pad, among the track's weapon pads.
        pad: u16,
    },
    /// A craft entered a trigger volume of the track file.
    TriggerEnter {
        /// The grid slot.
        slot: Slot,
        /// The trigger, among the track's triggers.
        trigger: u16,
    },
    /// A craft left a trigger volume.
    TriggerExit {
        /// The grid slot.
        slot: Slot,
        /// The trigger.
        trigger: u16,
    },
    /// Weapons hit a craft since the last call, a hit the craft's Shield absorbed
    /// included. At most one per craft per tick: `amount` is the sum of what the tick's
    /// hits would have dealt, `weapon` the first of them.
    Hit {
        /// The grid slot struck.
        slot: Slot,
        /// The energy the hits would have taken, before any Shield.
        amount: f32,
        /// The weapon that hit first.
        weapon: Weapon,
    },
    /// A mod's `emit`, or a trigger's `oag_emit`, sent on the tick before: delivered to
    /// every module, whoever sent it, whether or not it listens to the channel.
    Mod {
        /// The craft the sender named; for a trigger, the craft that entered it.
        slot: Slot,
        /// The key of the sending mod's id (`channel_key` of it), or [`TRACK_SENDER`].
        from: u32,
        /// The channel ([`channel_key`]).
        channel: u32,
        /// What the sender attached.
        value: f32,
    },
    /// A `transit` ended: the craft is back in the world, further round, this tick.
    TransitEnd {
        /// The grid slot.
        slot: Slot,
    },
    /// The answer to the slot's `probe-clearance` of the tick before.
    Clearance {
        /// The grid slot.
        slot: Slot,
        /// The disc fits on the track there without touching its geometry.
        fits: bool,
        /// How far ahead was probed, in track units (the distance resolved); 0 when the
        /// track has no course to probe along.
        ahead: f32,
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
            | Self::SpeedPad { slot, .. }
            | Self::WeaponPad { slot, .. }
            | Self::TriggerEnter { slot, .. }
            | Self::TriggerExit { slot, .. }
            | Self::Hit { slot, .. }
            | Self::Mod { slot, .. }
            | Self::TransitEnd { slot }
            | Self::Clearance { slot, .. } => slot,
        }
    }

    /// The record.
    #[must_use]
    pub fn encode(self) -> [u8; EVENT_LEN] {
        let (kind, index) = match self {
            Self::WallContact { .. } => (EVENT_WALL_CONTACT, 0),
            Self::PickupFire { .. } => (EVENT_PICKUP_FIRE, 0),
            Self::PickupAbsorbed { .. } => (EVENT_PICKUP_ABSORBED, 0),
            Self::SpeedPad { pad, .. } => (EVENT_SPEED_PAD, pad),
            Self::WeaponPad { pad, .. } => (EVENT_WEAPON_PAD, pad),
            Self::TriggerEnter { trigger, .. } => (EVENT_TRIGGER_ENTER, trigger),
            Self::TriggerExit { trigger, .. } => (EVENT_TRIGGER_EXIT, trigger),
            Self::Hit { weapon, .. } => (EVENT_HIT, weapon.code() as u16),
            Self::Mod { .. } => (EVENT_MOD, 0),
            Self::TransitEnd { .. } => (EVENT_TRANSIT_END, 0),
            Self::Clearance { fits, .. } => (EVENT_CLEARANCE, u16::from(fits)),
        };
        let mut record = [0; EVENT_LEN];
        record[EVENT_KIND] = kind;
        record[EVENT_SLOT] = self.slot().0;
        let [lo, hi] = index.to_le_bytes();
        record[EVENT_INDEX] = lo;
        record[EVENT_INDEX + 1] = hi;
        match self {
            Self::Hit { amount, .. } | Self::Clearance { ahead: amount, .. } => {
                put_u32(&mut record, EVENT_VALUE, amount.to_bits());
            }
            Self::Mod {
                from,
                channel,
                value,
                ..
            } => {
                put_u32(&mut record, EVENT_VALUE, value.to_bits());
                put_u32(&mut record, EVENT_CHANNEL, channel);
                put_u32(&mut record, EVENT_FROM, from);
            }
            _ => {}
        }
        record
    }

    /// The event a record describes, or `None` for a kind or slot this revision does
    /// not know (a guest skips it).
    #[must_use]
    pub fn decode(record: &[u8; EVENT_LEN]) -> Option<Self> {
        let slot = Slot::new(usize::from(record[EVENT_SLOT]))?;
        let index = u16::from_le_bytes([record[EVENT_INDEX], record[EVENT_INDEX + 1]]);
        Some(match record[EVENT_KIND] {
            EVENT_WALL_CONTACT => Self::WallContact { slot },
            EVENT_PICKUP_FIRE => Self::PickupFire { slot },
            EVENT_PICKUP_ABSORBED => Self::PickupAbsorbed { slot },
            EVENT_SPEED_PAD => Self::SpeedPad { slot, pad: index },
            EVENT_WEAPON_PAD => Self::WeaponPad { slot, pad: index },
            EVENT_TRIGGER_ENTER => Self::TriggerEnter {
                slot,
                trigger: index,
            },
            EVENT_TRIGGER_EXIT => Self::TriggerExit {
                slot,
                trigger: index,
            },
            EVENT_HIT => Self::Hit {
                slot,
                amount: f32::from_bits(get_u32(record, EVENT_VALUE)),
                weapon: u8::try_from(index).ok().and_then(Weapon::from_code)?,
            },
            EVENT_TRANSIT_END => Self::TransitEnd { slot },
            EVENT_CLEARANCE => Self::Clearance {
                slot,
                fits: index != 0,
                ahead: f32::from_bits(get_u32(record, EVENT_VALUE)),
            },
            EVENT_MOD => Self::Mod {
                slot,
                from: get_u32(record, EVENT_FROM),
                channel: get_u32(record, EVENT_CHANNEL),
                value: f32::from_bits(get_u32(record, EVENT_VALUE)),
            },
            _ => return None,
        })
    }
}
