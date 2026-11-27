//! A cue's command list run as the handler runs it: tick by tick, with a
//! parameter, so a cue that **repeats** can be played for as long as it is held.
//!
//! [`Bank::cue_timeline`](super::Bank::cue_timeline) lays a list down once, which
//! suits a cue that plays and stops. It cannot answer a list with a loop:
//! `~BLOWUP` re-keys a waveform every 43 ticks while the explosion holds the
//! handler, and `~ROCKLOCK` beeps at a tempo the reticle's parameter picks.
//! Both end only when the caller kills the handler, so there is no honest
//! horizon to unroll to.
//!
//! A [`Runner`] is `Scream_StartSound`'s and `Scream_TickCommandList`'s own
//! arithmetic (`0x0898f864`, `0x0898db80`, `0x0898efd8`) in whole master ticks:
//!
//! - **Start**: the counter is the first command's delay, and commands run while
//!   it is zero, so a zero-delay prefix keys on inside the start call.
//! - **Every tick**: the counter falls by one, the per-tick flag byte
//!   (`handler + 0x16`, `& 0xaf`) is cleared, and commands run while the counter
//!   is below one.
//! - **After a command**: the program counter steps on and the counter loads the
//!   *next* command's delay plus whatever the handler returned. Running off the
//!   end stops the list.
//! - **`0x15`** is a no-op marker. **`0x16`** (`Scream_OpLoopBack_q`) scans back
//!   for the nearest `0x15` and resumes there, returning `1` (one extra tick)
//!   only when run a **second time within one tick**: the flag it sets is
//!   cleared by the tick, so a waiting loop pays nothing and a non-waiting loop
//!   cannot spin. No `0x15` behind it kills the handler.
//! - **`0x22`** (`Scream_OpGuard`) compares one of the handler's four parameter
//!   bytes (`handler + 0x4c + index`, written by `Scream_SetCueParameter`'s
//!   `FUN_0898daf0`) against an immediate and, when the test says so, steps the
//!   program counter over the **next** command, skipping its delay too.
//! - **`0x1a`** returns `rand() % (operand + 1)` as extra delay on the next.
//!
//! Every parameter byte starts at zero: `Scream_StartSound` clears them unless
//! its caller supplies some, and `Sound_PlayNamedInSlot` supplies none.
//!
//! Anything else in the list - a child, an alternate group, a bend, a goto, a
//! global-variable guard - is not run: [`Bank::cue_runner`] returns `None`
//! rather than play a list it would run wrong.
//!
//! # HD's layout
//!
//! [`Bank::cue_runner_modelled`] under [`WalkModel::cue_variables`] reads the
//! list as HD's PS3 handlers do. The stepper and the per-tick countdown are the
//! same (`Scream_StepCommandList` `0x00625218`, `Scream_TickCommandList`
//! `0x006253d0`), the byte operands sit one place over (`a` is `operand >> 16`),
//! and four more things run: the alternate group `0x19`, the marker gotos
//! `0x24`/`0x25`, the variable writes `0x1e`/`0x20`, the fixed bend `0x1c` and
//! the stop `0x18`. The guard reads the same four bytes
//! `Scream_SetCueParameter` writes (`docs/ghidra/functions/ps3-hdfury-eu/cue-variables.md`,
//! "Held cues run as lists").

use super::timeline::{Frame, Grain, WalkModel, variable_guard_skips};
use super::{Bank, COMMAND_LEN, Cue, KEY_ON_OPCODES};

const NOP_MARKER: u8 = 0x15;
const LOOP_BACK: u8 = 0x16;
const GUARD: u8 = 0x22;
const RANDOM_DELAY: u8 = 0x1a;
const END: u8 = 0x2b;
const STOP: u8 = 0x18;
const ALTERNATE: u8 = 0x19;
const FIXED_BEND: u8 = 0x1c;
const SET_VARIABLE: u8 = 0x1e;
const INCREMENT_VARIABLE: u8 = 0x20;
const MARKER: u8 = 0x23;
const GOTO: u8 = 0x24;
const RANDOM_GOTO: u8 = 0x25;
/// Gotos one tick may run before HD's `Scream_DoGrainGoto` refuses.
const MAX_GOTOS_PER_TICK: u32 = 8;
/// Opcodes that return zero and change nothing a runner reports.
const PASS_THROUGH: [u8; 7] = [0x14, 0x1e, 0x1f, 0x20, 0x21, 0x23, NOP_MARKER];

/// Cue parameters a handler carries, `handler + 0x4c ..= 0x4f`.
pub const PARAMETERS: usize = 4;

/// Steps one tick may take, so a list that cannot make progress terminates.
const MAX_STEPS_PER_TICK: u32 = 256;

#[derive(Debug, Clone, Copy)]
struct Command {
    opcode: u8,
    operand: u32,
    delay: i16,
}

/// Whether HD's layout runs a command that is not a key-on. A variable outside
/// the handler's four (HD's global table) is not run.
fn hd_runs(opcode: u8, operand: u32) -> bool {
    let a = (operand >> 16) as u8 as i8;
    let local = (0..PARAMETERS as i8).contains(&a);
    match opcode {
        GUARD | SET_VARIABLE | INCREMENT_VARIABLE => local,
        0x14 | NOP_MARKER | LOOP_BACK | STOP | ALTERNATE | RANDOM_DELAY | FIXED_BEND | MARKER
        | GOTO | RANDOM_GOTO | END => true,
        _ => false,
    }
}

/// A cue's command list, part-way through a play.
#[derive(Debug, Clone)]
pub struct Runner {
    commands: Vec<Command>,
    /// The grain each key-on command starts, at tick zero; indexed like
    /// `commands`.
    keyed: Vec<Option<Grain>>,
    /// Whether the list is read with HD's operand bytes and extra opcodes.
    hd: bool,
    parameters: [i8; PARAMETERS],
    /// The handler's bend, `0x1c`'s write (`Scream_SetVoiceBend`).
    bend: i16,
    /// The last alternate each `0x19` picked, by command index; starts as the
    /// bank's own byte, which the original caches in place.
    last_pick: Vec<u8>,
    /// An open alternate block: commands left in it, and the skip past the rest.
    repeat: Option<(i32, i32)>,
    gotos_this_tick: u32,
    pc: i32,
    countdown: i32,
    looped_this_tick: bool,
    ended: bool,
    tick: u32,
}

impl Bank<'_> {
    /// `cue`'s list as a [`Runner`], or `None` when the cue does not play or
    /// its list carries anything a runner does not run.
    #[must_use]
    pub fn cue_runner(&self, cue: &Cue) -> Option<Runner> {
        self.cue_runner_modelled(cue, WalkModel::default())
    }

    /// [`Self::cue_runner`] under `model`: with [`WalkModel::cue_variables`] the
    /// list is read as HD's handlers read it and the runner starts with those
    /// four variables; the model's other switches are ignored.
    #[must_use]
    pub fn cue_runner_modelled(&self, cue: &Cue, model: WalkModel) -> Option<Runner> {
        let hd = model.cue_variables.is_some();
        let variables = model.cue_variables.unwrap_or_default();
        if !cue.plays() {
            return None;
        }
        let sounds = self.sounds();
        let frame = Frame {
            start: 0,
            angle: 0,
            cue_volume: cue.volume,
            scale: 1.0,
            bend: None,
            locals: variables,
        };
        let mut commands = Vec::with_capacity(cue.commands);
        let mut keyed = Vec::with_capacity(cue.commands);
        let mut last_pick = Vec::with_capacity(cue.commands);
        for k in 0..cue.commands {
            let command = cue.first_command + k;
            let at = command * COMMAND_LEN;
            let bytes = self.commands.get(at..at + COMMAND_LEN)?;
            let word = self.order.u32(bytes, 0);
            let opcode = (word >> 24) as u8;
            let operand = word & 0x00ff_ffff;
            let delay = self.order.u32(bytes, 4) as u16 as i16;
            let mut grain = None;
            if KEY_ON_OPCODES.contains(&opcode) {
                let sound = sounds.iter().find(|s| s.command == command)?;
                grain = Some(self.keyed(sound, 0, &frame));
            } else if hd {
                if !hd_runs(opcode, operand) {
                    return None;
                }
            } else if opcode == GUARD {
                // A negative index reads a runtime global, not a parameter.
                let variable = (operand & 0xff) as u8 as i8;
                if !(0..PARAMETERS as i8).contains(&variable) {
                    return None;
                }
            } else if !(PASS_THROUGH.contains(&opcode)
                || matches!(opcode, LOOP_BACK | RANDOM_DELAY | END))
            {
                return None;
            }
            last_pick.push((operand & 0xff) as u8);
            commands.push(Command {
                opcode,
                operand,
                delay,
            });
            keyed.push(grain);
        }
        let first = commands.first()?;
        Some(Runner {
            countdown: i32::from(first.delay),
            commands,
            keyed,
            hd,
            parameters: variables,
            bend: 0,
            last_pick,
            repeat: None,
            gotos_this_tick: 0,
            pc: 0,
            looped_this_tick: false,
            ended: false,
            tick: 0,
        })
    }
}

impl Runner {
    /// Every waveform the list can key on, at tick zero: what a caller decodes
    /// up front so a tick never has to.
    pub fn key_ons(&self) -> impl Iterator<Item = &Grain> {
        self.keyed.iter().flatten()
    }

    /// Whether the list has run off its end (a list with a loop never does).
    #[must_use]
    pub fn ended(&self) -> bool {
        self.ended
    }

    /// Master ticks run since the start, `0` straight after [`Self::start`].
    #[must_use]
    pub fn tick_count(&self) -> u32 {
        self.tick
    }

    /// The handler's bend as `Scream_SetVoiceBend` holds it, `0` until a `0x1c`
    /// runs. It reaches every voice the cue has keyed, sounding or not.
    #[must_use]
    pub fn bend(&self) -> i16 {
        self.bend
    }

    /// Writes cue parameter `index`, as `Scream_SetCueParameter` does. An
    /// index outside the four a handler holds is ignored, as in the original.
    pub fn set_parameter(&mut self, index: usize, value: i8) {
        if let Some(slot) = self.parameters.get_mut(index) {
            *slot = value;
        }
    }

    /// Runs the zero-delay prefix, as `Scream_StartSound` does before it returns.
    pub fn start(&mut self, rand: &mut dyn FnMut() -> u32) -> Vec<Grain> {
        let mut out = Vec::new();
        self.looped_this_tick = false;
        self.gotos_this_tick = 0;
        let mut steps = 0;
        while !self.ended && self.countdown == 0 {
            steps += 1;
            if steps > MAX_STEPS_PER_TICK || !self.step(rand, &mut out) {
                self.ended = true;
            }
        }
        out
    }

    /// Runs one master tick: every command whose delay has run out.
    pub fn tick(&mut self, rand: &mut dyn FnMut() -> u32) -> Vec<Grain> {
        let mut out = Vec::new();
        if self.ended {
            return out;
        }
        self.tick += 1;
        self.countdown -= 1;
        self.looped_this_tick = false;
        self.gotos_this_tick = 0;
        let mut steps = 0;
        while !self.ended && self.countdown < 1 {
            steps += 1;
            if steps > MAX_STEPS_PER_TICK || !self.step(rand, &mut out) {
                self.ended = true;
            }
        }
        out
    }

    /// One `Scream_StepCommandList`. `false` is the handler dying.
    fn step(&mut self, rand: &mut dyn FnMut() -> u32, out: &mut Vec<Grain>) -> bool {
        let Some(&command) = usize::try_from(self.pc)
            .ok()
            .and_then(|pc| self.commands.get(pc))
        else {
            return false;
        };
        let mut extra = 0_i32;
        match command.opcode {
            RANDOM_DELAY => {
                extra = i32::try_from(rand() % (command.operand + 1)).unwrap_or(i32::MAX);
            }
            LOOP_BACK => match self.loop_back() {
                Some(returned) => extra = returned,
                None => return false,
            },
            GUARD => {
                let skips = if self.hd {
                    variable_guard_skips(command.operand, &self.parameters) == Some(true)
                } else {
                    self.guard_skips(command.operand)
                };
                if skips {
                    self.pc += 1;
                }
            }
            END => self.pc = self.commands.len() as i32 - 1,
            STOP if self.hd => return false,
            FIXED_BEND if self.hd => self.bend = fixed_bend((command.operand >> 16) as u8 as i8),
            SET_VARIABLE | INCREMENT_VARIABLE if self.hd => {
                let a = usize::from((command.operand >> 16) as u8) % PARAMETERS;
                self.parameters[a] = if command.opcode == SET_VARIABLE {
                    (command.operand >> 8) as u8 as i8
                } else {
                    self.parameters[a].saturating_add(1)
                };
            }
            GOTO | RANDOM_GOTO if self.hd => {
                self.gotos_this_tick += 1;
                if self.gotos_this_tick > MAX_GOTOS_PER_TICK {
                    // The handler stays on this command and waits one tick more.
                    self.pc -= 1;
                    extra = 1;
                } else {
                    let low = (command.operand >> 16) as u8;
                    let id = if command.opcode == GOTO {
                        low
                    } else {
                        let high = (command.operand >> 8) as u8;
                        let Some(span) = high.checked_sub(low) else {
                            return false;
                        };
                        low + (rand() & 0xffff).rem_euclid(u32::from(span) + 1) as u8
                    };
                    let Some(marker) = self.marker(id) else {
                        return false;
                    };
                    self.pc = marker - 1;
                }
            }
            ALTERNATE if self.hd => {
                if !self.alternate(command.operand, rand) {
                    return false;
                }
            }
            _ => {
                if let Some(Some(grain)) = usize::try_from(self.pc)
                    .ok()
                    .and_then(|pc| self.keyed.get(pc))
                {
                    out.push(Grain {
                        tick: self.tick,
                        ..grain.clone()
                    });
                }
            }
        }
        if let Some((left, skip)) = self.repeat.as_mut() {
            *left -= 1;
            if *left == 0 {
                self.pc += *skip;
                self.repeat = None;
            }
        }
        self.pc += 1;
        match usize::try_from(self.pc)
            .ok()
            .and_then(|pc| self.commands.get(pc))
        {
            Some(next) => self.countdown = i32::from(next.delay) + extra,
            None => self.ended = true,
        }
        true
    }

    /// The first `0x23` marker carrying `id`.
    fn marker(&self, id: u8) -> Option<i32> {
        self.commands
            .iter()
            .position(|c| c.opcode == MARKER && (c.operand >> 16) as u8 == id)
            .map(|k| k as i32)
    }

    /// `Scream_DoGrainAlternate` (`0x00625e88`): draw one of `count` blocks of
    /// `stride` commands, never the last draw, run it and skip the rest. A
    /// second group opened inside a block kills the handler, as a zero count
    /// does here (the original divides by it).
    fn alternate(&mut self, operand: u32, rand: &mut dyn FnMut() -> u32) -> bool {
        let count = i32::from((operand >> 16) as u8 as i8);
        let stride = i32::from((operand >> 8) as u8 as i8);
        let Some(last) = usize::try_from(self.pc)
            .ok()
            .and_then(|pc| self.last_pick.get_mut(pc))
        else {
            return false;
        };
        if self.repeat.is_some() || count <= 0 || stride <= 0 {
            return false;
        }
        let mut pick = (rand() & 0xffff) as i32 % count;
        if i32::from(*last as i8) == pick {
            pick = (pick + 1) % count;
        }
        *last = pick as u8;
        self.pc += pick * stride;
        // This command counts one down, so the block's `stride` follow it.
        self.repeat = Some((stride + 1, (count - pick - 1) * stride));
        true
    }

    /// `Scream_OpLoopBack_q`: the value it returns, or `None` when no `0x15`
    /// lies behind it and the handler dies.
    fn loop_back(&mut self) -> Option<i32> {
        let marker = (0..self.pc as usize)
            .rev()
            .find(|&k| self.commands[k].opcode == NOP_MARKER)?;
        self.pc = marker as i32 - 1;
        let returned = i32::from(self.looped_this_tick);
        self.looped_this_tick = true;
        Some(returned)
    }

    /// `Scream_OpGuard`: whether the command after it is skipped.
    fn guard_skips(&self, operand: u32) -> bool {
        let variable = self.parameters[(operand & 0xff) as usize % PARAMETERS];
        let mode = (operand >> 8) as u8 as i8;
        let immediate = (operand >> 16) as u8 as i8;
        match mode {
            0 => immediate <= variable,
            1 => variable != immediate,
            2 => variable <= immediate,
            _ => false,
        }
    }
}

/// `Scream_DoGrainSetBend`'s (`0x006242b0`) word for operand `a`.
fn fixed_bend(a: i8) -> i16 {
    if a < 0 {
        i16::from(a) << 8
    } else {
        (i32::from(a) * 0x7fff / 0x7f) as i16
    }
}

#[cfg(test)]
mod tests;
