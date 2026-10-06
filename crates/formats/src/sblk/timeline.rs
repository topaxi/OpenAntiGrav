//! A cue as a timeline: which waveform starts when, and at what pan angle.
//!
//! [`Bank::cue_sounds`](super::Bank::cue_sounds) and
//! [`Bank::cue_tree_sounds`](super::Bank::cue_tree_sounds) give a flat set of
//! waveforms. That suits `.COLLISIONS`' fifteen takes of one event, not a cue
//! whose grains are a **sequence**: Pulse's `zone_5` reaches three words
//! (`ZONE`, the number, `CLEAR`), and playing one at random is what a player
//! hears as "clear" or "zone" alone.
//!
//! # What the command list does
//!
//! Read from `Scream_StartSound` (`0x0898f864`), `Scream_StepCommandList`
//! (`0x0898efd8`) and the per-tick stepper `0x0898db80` in Pulse's `BOOT.BIN`,
//! see `docs/formats/psp-audio.md`:
//!
//! - A command is `{ u8 opcode, u24 operand, u32 delay }`. The **second word is
//!   the delay before the command's own execution**, counted from the previous
//!   command's (from the cue's start for the first). After a command the
//!   stepper loads `handler + 0x48` with the *next* command's second word; the
//!   per-tick stepper decrements it per master tick and runs commands while it
//!   is below one, so a delay of zero runs in the same tick.
//! - `0x01`/`0x09` key a waveform on. The descriptor's `+0x04` is the voice's
//!   pan angle in degrees ([`Grain::angle`]).
//! - `0x05` starts a **child cue in parallel**: the child runs from its own
//!   tick zero and the parent's next delay counts from this command. Both
//!   handlers return zero, so no extra wait is added.
//! - `0x19` is an **alternate group**: operand byte 0 is the alternate count
//!   `N`, byte 1 the stride `S` (commands per alternate). The handler adds
//!   `pick * S` to the program counter, arms the repeat mechanism for `S`
//!   commands, then skips `(N - pick - 1) * S`. One block runs and **whatever
//!   follows the group runs after it**: a choice in the middle of a timeline.
//!   The pick is the caller's draw ([`Bank::cue_timeline_with`]);
//!   [`Timeline::groups`] lists the groups met.
//! - `0x2b` **ends the list**: the handler sets the program counter to the last
//!   command, so the stepper's increment runs off the end.
//! - `0x1b` is a **random pitch bend** and returns zero. It applies to every
//!   later key-on (and later children) through each descriptor's bend range
//!   ([`Sound::bend_down`](super::Sound::bend_down)); the draw is the caller's.
//!   Recorded in [`Timeline::bends`] and [`Grain::bend`].
//! - `0x14` is a no-op and `0x1e`/`0x1f`/`0x20`/`0x21` write a register byte;
//!   all return zero. With no guard (`0x22`) or parameter sentinel to read a
//!   register they change nothing reported here. Recorded in
//!   [`Timeline::passed`].
//! - `0x24` is a **goto** and `0x23` its **marker**, walked only when asked
//!   ([`WalkModel::goto_markers`]). The goto scans its cue's commands from the
//!   first for a marker whose operand byte 1 equals its own and runs on from
//!   there; the marker is a no-op returning zero. No marker is an error that
//!   ends the cue, and a ninth goto within one tick is refused. Read on both
//!   binaries (`Scream_DoGrainGoto` on HD, `Scream_OpGoto` on Pulse's).
//! - Every other opcode is reported in [`Timeline::unread`], not skipped
//!   silently: `0x1a` adds a random wait, `0x08` replaces the voice's playback
//!   state, `0x04` starts an LFO. A timeline that met any is not the whole cue.
//!
//! # The tick
//!
//! One master tick is [`TICKS_PER_SECOND`] on the PSP build: `Audio_OutputThread`
//! runs it three times per two 256-frame mixer grains at 44,100 Hz, and a live
//! count agreed (257.7, 259.7 and 258.3 ticks per emulated second over three
//! 6 s windows). **The tick belongs to a build, not a byte order**: HD's PS3
//! build runs 240 Hz, PS2 and Vita are not measured. The caller decides the
//! build from the title's data (`oag_title::SequenceTick`).

use super::child::{CHILD_INDEX_AT, CHILD_RECORD_LEN, MAX_CHILD_DEPTH};
use super::{Bank, COMMAND_LEN, Cue, KEY_ON_OPCODES, Sound};

/// Master ticks per second of the PSP's SCREAM, `44100 * 3 / 512`.
pub const TICKS_PER_SECOND: f64 = 44_100.0 * 3.0 / 512.0;

/// The opcode that starts a child cue in parallel.
const PLAY_CHILD: u8 = 0x05;
/// The opcode that chooses one block of commands out of several.
const ALTERNATE: u8 = 0x19;
/// The opcode that sets a random pitch bend.
const RANDOM_BEND: u8 = 0x1b;
/// The opcode that ends the command list.
const END: u8 = 0x2b;
/// The opcode that jumps to a marker of the same cue.
const GOTO: u8 = 0x24;
/// The opcode a goto lands on; a no-op.
const MARKER: u8 = 0x23;
/// Gotos one tick may run before `Scream_DoGrainGoto` refuses (its depth guard).
const MAX_GOTOS_PER_TICK: u32 = 8;
/// Opcodes that return zero and change nothing this walk reports.
const PASS_THROUGH: [u8; 5] = [0x14, 0x1e, 0x1f, 0x20, 0x21];

/// Offset of a child record's angle word, added to the child's voice angle.
const CHILD_ANGLE_AT: usize = 0x04;

/// The most alternate combinations [`Bank::cue_timelines`] will enumerate.
pub const MAX_COMBINATIONS: usize = 64;

/// Steps one walk may take, so a hostile command table terminates.
const MAX_STEPS: usize = 4096;

/// Which opcodes beyond the always-modelled set a walk follows.
///
/// The default is the walk every caller had before HD's tick was measured, so a
/// title opts in rather than having its cue census move.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WalkModel {
    /// Follow `0x24` to its `0x23` marker within the cue, instead of reporting
    /// both as unread.
    pub goto_markers: bool,
}

/// One waveform started at one moment.
#[derive(Debug, Clone, PartialEq)]
pub struct Grain {
    pub tick: u32,
    /// The waveform, as [`Bank::sounds`](super::Bank::sounds) yields it.
    pub sound: Sound,
    /// The volume the handler carries: the cue's `+0x00` for the root, the child
    /// record's for a child.
    pub cue_volume: i8,
    /// The handler's volume scale: `1.0` for the root, else the parent's scale
    /// times `cue_volume / 127` (`Scream_OpPlayChild`'s `(+0x3a * +0xc) / 0x7f`).
    pub scale: f32,
    /// Pan angle in degrees: the descriptor's `+0x04` plus the child records'
    /// angle words on the way down.
    pub angle: i32,
    /// Which `0x1b` execution (an index into [`Timeline::bends`]) detunes this
    /// grain, when one was in force.
    pub bend: Option<usize>,
}

/// An alternate group (`0x19`) the walk met.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlternateGroup {
    /// Alternates in the group, operand byte 0.
    pub count: u8,
    /// Commands per alternate, operand byte 1.
    pub stride: u8,
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
    /// The alternate groups met, in walk order. The walk took the alternate
    /// the caller's `picks` named for each (the first when it named none).
    pub groups: Vec<AlternateGroup>,
    /// Each `0x1b` executed, as its percentage operand. One random draw per
    /// entry is shared by every [`Grain`] that names it.
    pub bends: Vec<i8>,
    /// Pass-through opcodes met, in walk order.
    pub passed: Vec<u8>,
}

impl Timeline {
    /// Whether every command met was modelled and every child resolved: the
    /// only case where the timeline is the whole cue. A cue with
    /// [`Self::groups`] is complete *for the alternates picked*.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.unread.is_empty() && self.unresolved == 0
    }
}

/// Where a walk is: shared state that threads through children.
struct Walk<'a> {
    model: WalkModel,
    sounds: &'a [Sound],
    picks: &'a [u8],
    path: Vec<u16>,
    steps: usize,
    out: Timeline,
}

/// One handler's own frame.
#[derive(Clone, Copy)]
pub(super) struct Frame {
    pub(super) start: u32,
    pub(super) angle: i32,
    pub(super) cue_volume: i8,
    pub(super) scale: f32,
    pub(super) bend: Option<usize>,
}

impl Bank<'_> {
    /// The timeline a cue plays, following its children, taking the first
    /// alternate in every group. Depth is bounded by [`MAX_CHILD_DEPTH`] and a
    /// cue already on the walk's path is not re-entered, so a hostile bank
    /// terminates.
    #[must_use]
    pub fn cue_timeline(&self, cue: &Cue) -> Timeline {
        self.cue_timeline_with(cue, &[])
    }

    /// [`Self::cue_timeline`] with `picks[k]` the alternate taken in the `k`th
    /// group met (reduced modulo the group's count; the first when absent).
    #[must_use]
    pub fn cue_timeline_with(&self, cue: &Cue, picks: &[u8]) -> Timeline {
        self.cue_timeline_modelled(cue, picks, WalkModel::default())
    }

    /// [`Self::cue_timeline_with`] under an explicit [`WalkModel`].
    #[must_use]
    pub fn cue_timeline_modelled(&self, cue: &Cue, picks: &[u8], model: WalkModel) -> Timeline {
        let sounds = self.sounds();
        let mut walk = Walk {
            model,
            sounds: &sounds,
            picks,
            path: vec![cue.index],
            steps: 0,
            out: Timeline::default(),
        };
        let frame = Frame {
            start: 0,
            angle: 0,
            cue_volume: cue.volume,
            scale: 1.0,
            bend: None,
        };
        self.walk(cue, frame, &mut walk);
        let mut out = walk.out;
        // Stable, so two grains on one tick keep the order they were walked in.
        out.grains.sort_by_key(|grain| grain.tick);
        out
    }

    /// Every timeline the cue can play: one per combination of alternates.
    /// `None` when the combinations pass [`MAX_COMBINATIONS`], or when the
    /// groups met depend on the picks (a group inside an alternate).
    #[must_use]
    pub fn cue_timelines(&self, cue: &Cue) -> Option<Vec<Timeline>> {
        self.cue_timelines_modelled(cue, WalkModel::default())
    }

    /// [`Self::cue_timelines`] under an explicit [`WalkModel`].
    #[must_use]
    pub fn cue_timelines_modelled(&self, cue: &Cue, model: WalkModel) -> Option<Vec<Timeline>> {
        let first = self.cue_timeline_modelled(cue, &[], model);
        let counts: Vec<u8> = first.groups.iter().map(|g| g.count.max(1)).collect();
        let total = counts
            .iter()
            .try_fold(1usize, |n, &c| n.checked_mul(usize::from(c)))?;
        if total > MAX_COMBINATIONS {
            return None;
        }
        let mut out = Vec::with_capacity(total);
        for combination in 0..total {
            let mut rest = combination;
            let picks: Vec<u8> = counts
                .iter()
                .map(|&c| {
                    let pick = (rest % usize::from(c)) as u8;
                    rest /= usize::from(c);
                    pick
                })
                .collect();
            let timeline = self.cue_timeline_modelled(cue, &picks, model);
            if timeline.groups != first.groups {
                return None;
            }
            out.push(timeline);
        }
        Some(out)
    }

    fn walk(&self, cue: &Cue, frame: Frame, walk: &mut Walk) {
        if !cue.plays() {
            return;
        }
        let mut frame = frame;
        let mut tick = frame.start;
        // The stepper's repeat mechanism: commands left in the chosen block
        // (`+0x52`) and the skip when it runs out (`+0x54`).
        let mut repeat: Option<(i32, i32)> = None;
        let mut started = false;
        let mut gotos_this_tick = 0_u32;
        let count = cue.commands as i32;
        let mut pc: i32 = 0;
        while (0..count).contains(&pc) {
            walk.steps += 1;
            if walk.steps > MAX_STEPS {
                walk.out.unread.push(0xff);
                return;
            }
            let command = cue.first_command + pc as usize;
            let at = command * COMMAND_LEN;
            let Some(grain) = self.commands.get(at..at + COMMAND_LEN) else {
                return;
            };
            let word = self.order.u32(grain, 0);
            let opcode = (word >> 24) as u8;
            // `handler + 0x48` is a signed halfword, and a delay that is not
            // positive runs in the same tick.
            let delay = self.order.u32(grain, 4) as u16 as i16;
            tick = tick.saturating_add(u32::try_from(delay).unwrap_or(0));
            if delay > 0 {
                gotos_this_tick = 0;
            }

            if KEY_ON_OPCODES.contains(&opcode) {
                if let Some(sound) = walk.sounds.iter().find(|s| s.command == command) {
                    walk.out.grains.push(self.keyed(sound, tick, &frame));
                    started = true;
                } else {
                    walk.out.unread.push(opcode);
                }
            } else if opcode == PLAY_CHILD {
                started = true;
                self.walk_child(word & 0x00ff_ffff, tick, frame, walk);
            } else if opcode == RANDOM_BEND {
                if started {
                    // Voices already keyed would be re-pitched by the handler;
                    // not modelled.
                    walk.out.unread.push(opcode);
                } else {
                    walk.out.bends.push((word & 0xff) as u8 as i8);
                    frame.bend = Some(walk.out.bends.len() - 1);
                }
            } else if PASS_THROUGH.contains(&opcode)
                || (walk.model.goto_markers && opcode == MARKER)
            {
                walk.out.passed.push(opcode);
            } else if walk.model.goto_markers && opcode == GOTO {
                gotos_this_tick += 1;
                let id = ((word >> 16) & 0xff) as u8;
                let target = self.marker_of(cue, id);
                match target {
                    // The handler refuses a ninth goto in one tick.
                    Some(_) if gotos_this_tick > MAX_GOTOS_PER_TICK => {
                        walk.out.unread.push(opcode);
                        return;
                    }
                    Some(marker) => pc = marker - 1,
                    // No marker is an error return: the cue ends.
                    None => return,
                }
            } else if opcode == END {
                return;
            } else if opcode == ALTERNATE {
                let alternates = (word & 0xff) as i32;
                let stride = ((word >> 8) & 0xff) as i32;
                if repeat.is_some() || alternates == 0 || stride == 0 {
                    walk.out.unread.push(opcode);
                    return;
                }
                let group = walk.out.groups.len();
                let pick = i32::from(walk.picks.get(group).copied().unwrap_or(0)) % alternates;
                walk.out.groups.push(AlternateGroup {
                    count: alternates as u8,
                    stride: stride as u8,
                });
                pc += pick * stride;
                repeat = Some((stride + 1, (alternates - pick - 1) * stride));
            } else {
                walk.out.unread.push(opcode);
            }

            if let Some((left, skip)) = repeat {
                if left - 1 == 0 {
                    repeat = None;
                    pc += skip;
                } else {
                    repeat = Some((left - 1, skip));
                }
            }
            pc += 1;
        }
    }

    /// The grain a key-on of `sound` starts at `tick` under `frame`: the
    /// descriptor's `+0x04` pan angle joins the frame's own.
    pub(super) fn keyed(&self, sound: &Sound, tick: u32, frame: &Frame) -> Grain {
        let descriptor_angle = self
            .block
            .get(sound.descriptor as usize + 4..)
            .and_then(|tail| tail.get(..2))
            .map_or(0, |b| i32::from(self.order.u16(b, 0) as i16));
        Grain {
            tick,
            sound: sound.clone(),
            cue_volume: frame.cue_volume,
            scale: frame.scale,
            angle: frame.angle + descriptor_angle,
            bend: frame.bend,
        }
    }

    /// The first marker (`0x23`) of `cue` whose operand byte 1 is `id`, as an
    /// index into the cue's own commands.
    fn marker_of(&self, cue: &Cue, id: u8) -> Option<i32> {
        (0..cue.commands).find_map(|k| {
            let at = (cue.first_command + k) * COMMAND_LEN;
            let word = self.order.u32(self.commands.get(at..at + COMMAND_LEN)?, 0);
            ((word >> 24) as u8 == MARKER && ((word >> 16) & 0xff) as u8 == id).then_some(k as i32)
        })
    }

    fn walk_child(&self, operand: u32, tick: u32, frame: Frame, walk: &mut Walk) {
        let Some(record_at) = self.parameter_offset.checked_add(operand) else {
            walk.out.unresolved += 1;
            return;
        };
        let Some(record) = self
            .block
            .get(record_at as usize..)
            .and_then(|tail| tail.get(..CHILD_RECORD_LEN))
        else {
            walk.out.unresolved += 1;
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
            walk.out.unresolved += 1;
            return;
        };
        if walk.path.len() > MAX_CHILD_DEPTH || walk.path.contains(&child.index) {
            walk.out.unresolved += 1;
            return;
        }
        let child_angle = self.order.u32(record, CHILD_ANGLE_AT) as i32;
        if child_angle < 0 {
            // The negative range is `Scream_OpPlayChild`'s parameter-register
            // sentinels, not modelled.
            walk.out.unread.push(PLAY_CHILD);
            return;
        }
        let record_volume = self.order.u32(record, 0).min(127) as i8;
        walk.path.push(child.index);
        self.walk(
            &child,
            Frame {
                start: tick,
                angle: frame.angle + child_angle,
                cue_volume: record_volume,
                scale: frame.scale * f32::from(frame.cue_volume.unsigned_abs()) / 127.0,
                bend: frame.bend,
            },
            walk,
        );
        walk.path.pop();
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
pub(crate) mod tests;
