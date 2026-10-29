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
//! | [`REPLY_AT`] + 4 | 12 x [`MAX_COMMANDS`] | commands: kind `u8`, slot `u8`, 2 bytes zero, `f32`, `u32` |
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
const _: () = assert!(REPLY_AT + 4 + COMMAND_LEN * MAX_COMMANDS <= IO_LEN);

/// Event kind codes.
pub const EVENT_WALL_CONTACT: u8 = 1;
/// Command kind codes.
pub const COMMAND_SCALE_THRUST: u8 = 1;

/// Something that happened in the race since the last call (README 2.1, `event`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// A craft touched a wall (`hooks.md`, `wall-contact`).
    WallContact {
        /// The grid slot.
        slot: u8,
    },
}

/// What a guest asked for, checked (README 2.1, `command`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    /// Scale the slot's engine output this tick, `factor` in `[0, 1]`
    /// (`Ship::pending_thrust_scale`). No timer: a guest re-sends it each tick.
    ScaleThrust {
        /// The grid slot.
        slot: u8,
        /// The scale, finite and in `[0, 1]`.
        factor: f32,
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
        }
    }
    buffer[STATE_AT..STATE_AT + state.len()].copy_from_slice(state);
}

pub(crate) fn state(buffer: &[u8], len: usize) -> &[u8] {
    &buffer[STATE_AT..STATE_AT + len]
}

/// Reads and checks the reply: every field in bounds, at most one of a kind per slot,
/// a slot that holds a craft. A broken rule is a [`Fault::Reply`], never a clamp.
pub(crate) fn read_reply(buffer: &[u8], active: u32) -> Result<Vec<Command>, Fault> {
    let count = get_u32(buffer, REPLY_AT) as usize;
    if count > MAX_COMMANDS {
        return Err(Fault::Reply(format!(
            "{count} commands, the most is {MAX_COMMANDS}"
        )));
    }
    let mut thrust_seen = 0_u32;
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
                if !(0.0..=1.0).contains(&factor) {
                    return Err(Fault::Reply(format!(
                        "scale-thrust factor {factor} for slot {slot} is not in [0, 1]"
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
            other => {
                return Err(Fault::Reply(format!(
                    "command {i} has unknown kind {other}"
                )));
            }
        }
    }
    Ok(commands)
}
