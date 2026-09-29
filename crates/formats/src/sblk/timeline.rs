//! A cue as a timeline: which waveform starts when, and at what pan angle.
//!
//! [`Bank::cue_sounds`](super::Bank::cue_sounds) and
//! [`Bank::cue_tree_sounds`](super::Bank::cue_tree_sounds) answer "which
//! waveforms does this cue reach", as a flat set. That is the right shape for
//! `.COLLISIONS`' fifteen takes of one event and the wrong one for a cue whose
//! grains are a **sequence**: Pulse's `zone_5` reaches three different words
//! (`ZONE`, the number, `CLEAR`), and playing one of the three at random is
//! what a player hears as "clear" or "zone" alone.
//!
//! # What the command list does
//!
//! Read from `Scream_StartSound` (`0x0898f864`), `Scream_StepCommandList`
//! (`0x0898efd8`) and the per-tick stepper `0x0898db80` in Pulse's `BOOT.BIN`,
//! see `docs/formats/psp-audio.md`:
//!
//! - A command is `{ u8 opcode, u24 operand, u32 delay }`. The **second word is
//!   the delay before the command's own execution**, counted from the previous
//!   command's execution (from the cue's start for the first). After running a
//!   command the stepper loads `handler + 0x48` with the *next* command's
//!   second word; the per-tick stepper decrements it once per master tick and
//!   runs commands while it is below one. A delay of zero runs in the same
//!   tick.
//! - `0x01`/`0x09` key a waveform on. The descriptor's `+0x04` is the voice's
//!   pan angle in degrees ([`Grain::angle`]).
//! - `0x05` starts a **child cue in parallel**: the child's own list runs from
//!   its own tick zero, and the parent's next delay counts from this command,
//!   not from the child's end. Both handlers return zero, so no extra wait is
//!   added to the next delay.
//! - Every other opcode is not walked and is reported in
//!   [`Timeline::unread`] rather than skipped silently: `0x1a` adds a random
//!   wait, `0x19` chooses between alternates, `0x08` replaces the voice's own
//!   playback state, and a timeline that met any of them is not the whole
//!   truth about the cue.
//!
//! # The tick
//!
//! One master tick is [`TICKS_PER_SECOND`] on the PSP build: `Audio_OutputThread`
//! runs the master tick three times per two 256-frame mixer grains at 44,100
//! Hz, and a live count against the PSP's own cycle counter agreed (257.7,
//! 259.7 and 258.3 ticks per emulated second over three 6 s windows).
//! Wipeout HD's engine is a different build and its tick is **not measured**,
//! so [`tick_seconds`] answers `None` for its byte order rather than lending it
//! the PSP's rate.

use super::child::{CHILD_INDEX_AT, CHILD_RECORD_LEN, MAX_CHILD_DEPTH};
use super::{Bank, COMMAND_LEN, Cue, KEY_ON_OPCODES, Sound};
use crate::byte_order::ByteOrder;

/// Master ticks per second of the PSP's SCREAM, `44100 * 3 / 512`.
pub const TICKS_PER_SECOND: f64 = 44_100.0 * 3.0 / 512.0;

/// Seconds per master tick for a bank of this byte order, when it is known.
///
/// Little-endian banks are the PSP/PS2 family and Pure; the rate was measured
/// on Pulse's PSP build only. Big-endian (HD) is not measured.
#[must_use]
pub fn tick_seconds(order: ByteOrder) -> Option<f64> {
    match order {
        ByteOrder::Little => Some(1.0 / TICKS_PER_SECOND),
        ByteOrder::Big => None,
    }
}

/// The opcode that starts a child cue in parallel.
const PLAY_CHILD: u8 = 0x05;

/// Offset of a child record's angle word, added to the child's voice angle.
const CHILD_ANGLE_AT: usize = 0x04;

/// One waveform started at one moment.
#[derive(Debug, Clone, PartialEq)]
pub struct Grain {
    /// Master ticks after the cue was started.
    pub tick: u32,
    /// The waveform, as [`Bank::sounds`](super::Bank::sounds) yields it.
    pub sound: Sound,
    /// The volume the playing handler carries: the cue's own `+0x00` for the
    /// root, the child record's volume for a child.
    pub cue_volume: i8,
    /// The handler's volume scale, `1.0` for the root and, for a child, the
    /// parent's scale times the parent's `cue_volume / 127`
    /// (`Scream_OpPlayChild`'s `(+0x3a * +0xc) / 0x7f`).
    pub scale: f32,
    /// Pan angle in degrees: the descriptor's `+0x04` plus the child records'
    /// angle words on the way down.
    pub angle: i32,
}

/// Everything a cue starts, in the order it starts it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Timeline {
    /// The key-ons, sorted by tick, ties in walk order.
    pub grains: Vec<Grain>,
    /// Opcodes met that this walk does not model, in walk order.
    pub unread: Vec<u8>,
    /// Child grains whose record did not resolve to a cue in this bank.
    pub unresolved: usize,
}

impl Timeline {
    /// Whether every command this walk met was one it models and every child
    /// resolved - the only case where the timeline is the whole cue.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.unread.is_empty() && self.unresolved == 0
    }
}

impl Bank<'_> {
    /// The timeline a cue plays, following the children it starts.
    ///
    /// See the module docs for what is modelled. Depth is bounded by
    /// [`MAX_CHILD_DEPTH`] and a cue already on the walk's own path is not
    /// entered again, so a hostile bank terminates.
    #[must_use]
    pub fn cue_timeline(&self, cue: &Cue) -> Timeline {
        let sounds = self.sounds();
        let mut out = Timeline::default();
        let mut path = vec![cue.index];
        self.walk(cue, 0, 0, cue.volume, 1.0, &sounds, &mut path, &mut out);
        // Stable, so two grains on one tick keep the order they were walked in.
        out.grains.sort_by_key(|grain| grain.tick);
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn walk(
        &self,
        cue: &Cue,
        start: u32,
        angle: i32,
        cue_volume: i8,
        scale: f32,
        sounds: &[Sound],
        path: &mut Vec<u16>,
        out: &mut Timeline,
    ) {
        if !cue.plays() {
            return;
        }
        let mut tick = start;
        for command in cue.range() {
            let at = command * COMMAND_LEN;
            let Some(grain) = self.commands.get(at..at + COMMAND_LEN) else {
                continue;
            };
            let word = self.order.u32(grain, 0);
            let opcode = (word >> 24) as u8;
            // `handler + 0x48` is a signed halfword, and a delay that is not
            // positive runs in the same tick.
            let delay = self.order.u32(grain, 4) as u16 as i16;
            tick = tick.saturating_add(u32::try_from(delay).unwrap_or(0));

            if KEY_ON_OPCODES.contains(&opcode) {
                let Some(sound) = sounds.iter().find(|s| s.command == command) else {
                    out.unread.push(opcode);
                    continue;
                };
                let descriptor_angle = self
                    .block
                    .get(sound.descriptor as usize + 4..)
                    .and_then(|tail| tail.get(..2))
                    .map_or(0, |b| i32::from(self.order.u16(b, 0) as i16));
                out.grains.push(Grain {
                    tick,
                    sound: sound.clone(),
                    cue_volume,
                    scale,
                    angle: angle + descriptor_angle,
                });
            } else if opcode == PLAY_CHILD {
                self.walk_child(
                    word & 0x00ff_ffff,
                    tick,
                    angle,
                    cue_volume,
                    scale,
                    sounds,
                    path,
                    out,
                );
            } else {
                out.unread.push(opcode);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn walk_child(
        &self,
        operand: u32,
        tick: u32,
        angle: i32,
        cue_volume: i8,
        scale: f32,
        sounds: &[Sound],
        path: &mut Vec<u16>,
        out: &mut Timeline,
    ) {
        let Some(record_at) = self.parameter_offset.checked_add(operand) else {
            out.unresolved += 1;
            return;
        };
        let Some(record) = self
            .block
            .get(record_at as usize..)
            .and_then(|tail| tail.get(..CHILD_RECORD_LEN))
        else {
            out.unresolved += 1;
            return;
        };
        let index = self.order.u32(record, CHILD_INDEX_AT);
        let child = u16::try_from(index)
            .ok()
            .filter(|&i| i < self.cue_count)
            .and_then(|i| self.cue(i))
            .or_else(|| {
                self.cue_children_named(record)
                    .and_then(|name| self.cue_named(&name))
            });
        let Some(child) = child else {
            out.unresolved += 1;
            return;
        };
        if path.len() > MAX_CHILD_DEPTH || path.contains(&child.index) {
            out.unresolved += 1;
            return;
        }
        let child_angle = self.order.u32(record, CHILD_ANGLE_AT) as i32;
        if child_angle < 0 {
            // The negative range is `Scream_OpPlayChild`'s parameter-register
            // sentinels, which this walk does not model.
            out.unread.push(PLAY_CHILD);
            return;
        }
        let record_volume = self.order.u32(record, 0).min(127) as i8;
        path.push(child.index);
        self.walk(
            &child,
            tick,
            angle + child_angle,
            record_volume,
            scale * f32::from(cue_volume.unsigned_abs()) / 127.0,
            sounds,
            path,
            out,
        );
        path.pop();
    }

    /// The name a child record carries in place of an index, when it does.
    fn cue_children_named(&self, record: &[u8]) -> Option<String> {
        let field = &record[super::child::CHILD_NAME_AT..super::child::CHILD_NAME_AT + 16];
        let end = field.iter().position(|&b| b == 0).unwrap_or(field.len());
        (end > 0 && field[..end].iter().all(u8::is_ascii_graphic))
            .then(|| String::from_utf8_lossy(&field[..end]).into_owned())
    }
}

#[cfg(test)]
mod tests;
