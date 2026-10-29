//! The I/O buffer: ABI 1's fixed little-endian layout, the one place a guest's bytes
//! are read.
//!
//! A guest exports `memory`, `oag_abi() -> u32` (returns [`crate::ABI`]),
//! `oag_io() -> u32` (the address of an [`IO_LEN`]-byte buffer in its memory) and
//! `tick() -> u32` (0 on success). Before `tick` the host writes the observation;
//! after it, the host reads the state blob back from where it wrote it and the reply
//! from [`REPLY_AT`].
//!
//! | Offset | Size | Field |
//! | --- | --- | --- |
//! | 0 | 4 | ABI major |
//! | 8 | 8 | world tick |
//! | 16 | 4 | slot count ([`MAX_SLOTS`]) |
//! | 20 | 4 | active mask, one bit per slot holding a craft |
//! | 24 | 4 | event count |
//! | 28 | 4 | state length |
//! | [`EVENTS_AT`] | 8 x [`MAX_EVENTS`] | events: kind `u8`, slot `u8`, 6 bytes zero |
//! | [`STATE_AT`] | [`MAX_STATE_BYTES`] | the module's state blob, in and out |
//! | [`REPLY_AT`] | 4 | command count, written by the guest |
//! | [`REPLY_AT`] + 4 | 12 x [`MAX_COMMANDS`] | commands: kind `u8`, slot `u8`, `u16` argument, `f32`, `u32` |
//! | [`CRAFTS_AT`] | 16 x [`MAX_CRAFTS`] | one record per grid slot ([`Craft`]): pickup `u8`, flags `u8`, 2 bytes zero, shield `f32`, shield maximum `f32`, speed km/h `f32` |
//!
//! The README's WIT (section 2.1) is the documentation of the records; this is the
//! subset Kiss Curse needs: one event, one command.

use crate::{Fault, Observation};

/// The buffer's length. A guest's buffer must be at least this long.
pub const IO_LEN: usize = 4096;
/// Grid slots the active mask can name. `oag_gameplay::MAX_SHIPS` is 8.
pub const MAX_SLOTS: u32 = 32;
/// Events per call. Past it the newest are dropped by the caller, which counts them.
pub const MAX_EVENTS: usize = 32;
/// The longest state blob.
pub const MAX_STATE_BYTES: usize = 1024;
/// Commands per reply; more is a fault (README 2.1: 64, proposed).
pub const MAX_COMMANDS: usize = 64;

/// Where the events start.
pub const EVENTS_AT: usize = 32;
const EVENT_LEN: usize = 8;
/// Where the state blob starts.
pub const STATE_AT: usize = EVENTS_AT + EVENT_LEN * MAX_EVENTS;
/// Where the guest writes its reply.
pub const REPLY_AT: usize = STATE_AT + MAX_STATE_BYTES;
const COMMAND_LEN: usize = 12;
/// Where the craft records start.
pub const CRAFTS_AT: usize = REPLY_AT + 4 + COMMAND_LEN * MAX_COMMANDS;
const CRAFT_LEN: usize = 16;
/// Craft records per observation: a grid's slots.
pub const MAX_CRAFTS: usize = 16;
const _: () = assert!(CRAFTS_AT + CRAFT_LEN * MAX_CRAFTS <= IO_LEN);

/// [`Craft::flags`]: a person flies the slot.
pub const CRAFT_PERSON: u8 = 1;
/// [`Craft::flags`]: a person flying the slot holds the fire button (`SQUARE`) this tick.
pub const CRAFT_FIRE_HELD: u8 = 2;
/// [`Craft::flags`]: the craft is racing (not wrecked, not respawning).
pub const CRAFT_RACING: u8 = 4;
/// [`Craft::flags`]: the craft's pickup is claimed by a module (`claim-pickup`).
pub const CRAFT_CLAIMED: u8 = 8;

/// The largest `scale-thrust` factor: a mod may double an engine's thrust, no more.
/// Chosen (README 2.1 proposed `[0.25, 2.0]`; the lower end is `0` here).
pub const MAX_THRUST_FACTOR: f32 = 2.0;

/// Event kind codes.
pub const EVENT_WALL_CONTACT: u8 = 1;
/// `pickup-fire{slot}`.
pub const EVENT_PICKUP_FIRE: u8 = 2;
/// `pickup-absorbed{slot}`.
pub const EVENT_PICKUP_ABSORBED: u8 = 3;
/// `pad-energy{slot}`.
pub const EVENT_PAD_ENERGY: u8 = 4;
/// Command kind codes.
pub const COMMAND_SCALE_THRUST: u8 = 1;
/// `shield-delta{slot, amount}`.
pub const COMMAND_SHIELD_DELTA: u8 = 2;
/// `claim-pickup{slot}`.
pub const COMMAND_CLAIM_PICKUP: u8 = 3;
/// `pad-mode{slot}`.
pub const COMMAND_PAD_MODE: u8 = 4;
/// `play-cue{slot, cue}`: an output, never state.
pub const COMMAND_PLAY_CUE: u8 = 5;
/// `fire-effect{slot, effect}`: an output, never state.
pub const COMMAND_FIRE_EFFECT: u8 = 6;
/// `pickup-tint{slot, rgba}`: an output, never state.
pub const COMMAND_PICKUP_TINT: u8 = 7;

/// `play-cue`'s codes: a title's own sounds, by the engine's cue names. 0 `SpeedupPad`,
/// 1 `Turbo`, 2 `Collision`, 3 `Absorb`, 4 `Shield`, 5 `ShieldActive`.
pub const CUES: u16 = 6;
/// `fire-effect`'s codes: a title's own particle effects, by trigger. 0 the wall
/// contact's sparks (`CollisionSpark`), 1 a weapon hit's (`HitSpark`).
pub const EFFECTS: u16 = 2;

/// One grid slot as a guest reads it: copies of what the HUD already shows.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Craft {
    /// The held pickup: 0 for none, else its place in the title's weapon list plus one
    /// (`oag_tables::weapons::Weapon::ALL`: Rocket 1, Missile 2, Quake 3, Cannon 4,
    /// Turbo 5, ...).
    pub pickup: u8,
    /// [`CRAFT_PERSON`], [`CRAFT_FIRE_HELD`], [`CRAFT_RACING`], [`CRAFT_CLAIMED`].
    pub flags: u8,
    /// The energy pool.
    pub shield: f32,
    /// Its maximum: the bound of a `shield-delta` for this slot.
    pub shield_max: f32,
    /// The body's speed, km/h.
    pub speed_kmh: f32,
}

/// Something that happened in the race since the last call (README 2.1, `event`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// A craft touched a wall (`hooks.md`, `wall-contact`).
    WallContact {
        /// The grid slot.
        slot: u8,
    },
    /// A craft whose pickup a module claimed asked to fire it: a person's fire press, or
    /// an AI craft's decision to fire. The pickup is kept.
    PickupFire {
        /// The grid slot.
        slot: u8,
    },
    /// A craft absorbed its claimed pickup: the pickup is gone and the engine paid no
    /// energy for it.
    PickupAbsorbed {
        /// The grid slot.
        slot: u8,
    },
    /// A craft in `pad-mode` entered a speed pad and was given no boost for it.
    PadEnergy {
        /// The grid slot.
        slot: u8,
    },
}

/// What a guest asked for, checked (README 2.1, `command`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    /// Scale the slot's engine output this tick, `factor` in `[0, MAX_THRUST_FACTOR]`:
    /// below `1.0` through `Ship::pending_thrust_scale`, above it through
    /// `Environment::extension_thrust`. No timer: a guest re-sends it each tick.
    ScaleThrust {
        /// The grid slot.
        slot: u8,
        /// The scale, finite and in `[0, MAX_THRUST_FACTOR]`.
        factor: f32,
    },
    /// Add `amount` to the slot's energy pool (negative takes it away) through the damage
    /// path: a loss can wreck the craft as a weapon hit does, a gain is clamped at the
    /// maximum. `|amount|` at most the slot's shield maximum.
    ShieldDelta {
        /// The grid slot.
        slot: u8,
        /// Energy, finite.
        amount: f32,
    },
    /// Take over the slot's held pickup: from now until it leaves the slot, the engine
    /// neither fires nor absorbs it, and raises `pickup-fire` / `pickup-absorbed` for the
    /// module instead. The slot must hold a pickup; one module's claim at a time.
    ClaimPickup {
        /// The grid slot.
        slot: u8,
    },
    /// This tick, a speed pad gives the slot no boost: on entering one the engine raises
    /// `pad-energy` instead. Sent each tick it should hold.
    PadMode {
        /// The grid slot.
        slot: u8,
    },
    /// Play one of the title's own sounds from the craft ([`CUES`]). An output: queued
    /// beside the engine's cues, heard, never hashed.
    PlayCue {
        /// The grid slot.
        slot: u8,
        /// A [`CUES`] code.
        cue: u16,
    },
    /// Fire one of the title's own particle effects at the craft's nozzle ([`EFFECTS`]).
    /// An output: drawn, never hashed. A title that ships no such effect draws nothing.
    FireEffect {
        /// The grid slot.
        slot: u8,
        /// An [`EFFECTS`] code.
        effect: u16,
    },
    /// This tick, draw the slot's held-pickup backdrop in `rgba` instead of the
    /// weapon's own colour. An output: drawn, never hashed.
    PickupTint {
        /// The grid slot.
        slot: u8,
        /// Red, green, blue, alpha, a byte each, red in the low byte.
        rgba: u32,
    },
}

fn put_u32(buffer: &mut [u8], at: usize, value: u32) {
    buffer[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn get_u32(buffer: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([buffer[at], buffer[at + 1], buffer[at + 2], buffer[at + 3]])
}

pub(crate) fn write_observation(buffer: &mut [u8], observation: &Observation<'_>, state: &[u8]) {
    put_u32(buffer, 0, crate::ABI);
    buffer[8..16].copy_from_slice(&observation.tick.to_le_bytes());
    put_u32(buffer, 16, MAX_SLOTS);
    put_u32(buffer, 20, observation.active);
    let events = &observation.events[..observation.events.len().min(MAX_EVENTS)];
    put_u32(buffer, 24, events.len() as u32);
    put_u32(buffer, 28, state.len() as u32);
    for (i, event) in events.iter().enumerate() {
        let at = EVENTS_AT + i * EVENT_LEN;
        match *event {
            Event::WallContact { slot } => {
                buffer[at] = EVENT_WALL_CONTACT;
                buffer[at + 1] = slot;
            }
            Event::PickupFire { slot } => {
                buffer[at] = EVENT_PICKUP_FIRE;
                buffer[at + 1] = slot;
            }
            Event::PickupAbsorbed { slot } => {
                buffer[at] = EVENT_PICKUP_ABSORBED;
                buffer[at + 1] = slot;
            }
            Event::PadEnergy { slot } => {
                buffer[at] = EVENT_PAD_ENERGY;
                buffer[at + 1] = slot;
            }
        }
    }
    buffer[STATE_AT..STATE_AT + state.len()].copy_from_slice(state);
    for (i, craft) in observation.crafts.iter().take(MAX_CRAFTS).enumerate() {
        let at = CRAFTS_AT + i * CRAFT_LEN;
        buffer[at] = craft.pickup;
        buffer[at + 1] = craft.flags;
        put_u32(buffer, at + 4, craft.shield.to_bits());
        put_u32(buffer, at + 8, craft.shield_max.to_bits());
        put_u32(buffer, at + 12, craft.speed_kmh.to_bits());
    }
}

pub(crate) fn state(buffer: &[u8], len: usize) -> &[u8] {
    &buffer[STATE_AT..STATE_AT + len]
}

/// Reads and checks the reply: every field in bounds, at most one of a kind per slot,
/// a slot that holds a craft. A broken rule is a [`Fault::Reply`], never a clamp.
pub(crate) fn read_reply(
    buffer: &[u8],
    observation: &Observation<'_>,
) -> Result<Vec<Command>, Fault> {
    let active = observation.active;
    let count = get_u32(buffer, REPLY_AT) as usize;
    if count > MAX_COMMANDS {
        return Err(Fault::Reply(format!(
            "{count} commands, the most is {MAX_COMMANDS}"
        )));
    }
    let mut thrust_seen = 0_u32;
    let mut shield_seen = 0_u32;
    let mut commands = Vec::with_capacity(count);
    for i in 0..count {
        let at = REPLY_AT + 4 + i * COMMAND_LEN;
        let (kind, slot) = (buffer[at], buffer[at + 1]);
        if u32::from(slot) >= MAX_SLOTS || active & (1 << slot) == 0 {
            return Err(Fault::Reply(format!(
                "command {i} names slot {slot}, which holds no craft"
            )));
        }
        match kind {
            COMMAND_SCALE_THRUST => {
                let factor = f32::from_bits(get_u32(buffer, at + 4));
                if !(0.0..=MAX_THRUST_FACTOR).contains(&factor) {
                    return Err(Fault::Reply(format!(
                        "scale-thrust factor {factor} for slot {slot} is not in [0, {MAX_THRUST_FACTOR}]"
                    )));
                }
                if thrust_seen & (1 << slot) != 0 {
                    return Err(Fault::Reply(format!(
                        "two scale-thrust commands for slot {slot}"
                    )));
                }
                thrust_seen |= 1 << slot;
                commands.push(Command::ScaleThrust { slot, factor });
            }
            COMMAND_SHIELD_DELTA => {
                let amount = f32::from_bits(get_u32(buffer, at + 4));
                let max = observation
                    .crafts
                    .get(usize::from(slot))
                    .map_or(0.0, |craft| craft.shield_max);
                if !amount.is_finite() || amount.abs() > max {
                    return Err(Fault::Reply(format!(
                        "shield-delta {amount} for slot {slot} is not within its maximum {max}"
                    )));
                }
                if shield_seen & (1 << slot) != 0 {
                    return Err(Fault::Reply(format!(
                        "two shield-delta commands for slot {slot}"
                    )));
                }
                shield_seen |= 1 << slot;
                commands.push(Command::ShieldDelta { slot, amount });
            }
            COMMAND_CLAIM_PICKUP => {
                let holds = observation
                    .crafts
                    .get(usize::from(slot))
                    .is_some_and(|craft| craft.pickup != 0);
                if !holds {
                    return Err(Fault::Reply(format!(
                        "claim-pickup for slot {slot}, which holds no pickup"
                    )));
                }
                commands.push(Command::ClaimPickup { slot });
            }
            COMMAND_PAD_MODE => commands.push(Command::PadMode { slot }),
            COMMAND_PLAY_CUE | COMMAND_FIRE_EFFECT => {
                let code = u16::from_le_bytes([buffer[at + 2], buffer[at + 3]]);
                let (count, name) = if kind == COMMAND_PLAY_CUE {
                    (CUES, "play-cue")
                } else {
                    (EFFECTS, "fire-effect")
                };
                if code >= count {
                    return Err(Fault::Reply(format!(
                        "{name} code {code} is not one of {count}"
                    )));
                }
                commands.push(if kind == COMMAND_PLAY_CUE {
                    Command::PlayCue { slot, cue: code }
                } else {
                    Command::FireEffect { slot, effect: code }
                });
            }
            COMMAND_PICKUP_TINT => commands.push(Command::PickupTint {
                slot,
                rgba: get_u32(buffer, at + 8),
            }),
            other => {
                return Err(Fault::Reply(format!(
                    "command {i} has unknown kind {other}"
                )));
            }
        }
    }
    Ok(commands)
}
