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

use super::timeline::{Frame, Grain};
use super::{Bank, COMMAND_LEN, Cue, KEY_ON_OPCODES};

const NOP_MARKER: u8 = 0x15;
const LOOP_BACK: u8 = 0x16;
const GUARD: u8 = 0x22;
const RANDOM_DELAY: u8 = 0x1a;
const END: u8 = 0x2b;
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

/// A cue's command list, part-way through a play.
#[derive(Debug, Clone)]
pub struct Runner {
    commands: Vec<Command>,
    /// The grain each key-on command starts, at tick zero; indexed like
    /// `commands`.
    keyed: Vec<Option<Grain>>,
    parameters: [i8; PARAMETERS],
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
        };
        let mut commands = Vec::with_capacity(cue.commands);
        let mut keyed = Vec::with_capacity(cue.commands);
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
            parameters: [0; PARAMETERS],
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
                if self.guard_skips(command.operand) {
                    self.pc += 1;
                }
            }
            END => self.pc = self.commands.len() as i32 - 1,
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

#[cfg(test)]
mod tests;
