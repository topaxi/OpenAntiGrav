//! Pad rumble: the pattern player and the motor backend it drives.
//!
//! [`Player`] is the playback law of `docs/gameplay/rumble.md` ("Playback
//! law"): a ring of 20 voices per pad, five mixed, a class gate, a pause gate,
//! looping by the release flag. It knows no device; [`MotorOutput`] takes its
//! [`Motors`] to one. Rumble is a per-tick output and never simulation state
//! (ADR-0018), so nothing here feeds a hash and no gameplay crate reaches it.
//!
//! A pattern is played by id: its place in the [`Library`] the player owns
//! ([`Library::index_of`]).

use oag_tables::rumble::{Class, Library, Pattern, Priority};

/// How the small motor reads the mixed voices' small tracks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmallMotor {
    /// Any non-zero is full on (HD/Fury, PS3).
    OnOff,
    /// The byte sum, clamped to 255 (Omega, PS4).
    Sum,
}

/// The RUMBLE setting: what the pad's motors play.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Nothing.
    None,
    /// The booted title's own law, where its original plays patterns.
    Original,
    /// HD's small motor: any non-zero is full on.
    Ps3,
    /// Omega's small motor: the byte sum. The default.
    #[default]
    Ps4,
}

impl Mode {
    /// Every value, in menu order.
    pub const ALL: [Self; 4] = [Self::None, Self::Original, Self::Ps3, Self::Ps4];

    /// The settings-file and menu token.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Original => "original",
            Self::Ps3 => "ps3",
            Self::Ps4 => "ps4",
        }
    }

    /// The mode a token names.
    #[must_use]
    pub fn from_name(token: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.name() == token)
    }

    /// The small-motor law this mode plays on a title whose original plays
    /// `original` (`None`: no patterns); `None` plays nothing. `Original` on a
    /// title that plays no patterns plays nothing too.
    #[must_use]
    pub fn law(self, original: Option<SmallMotor>) -> Option<SmallMotor> {
        match self {
            Self::None => None,
            Self::Original => original,
            Self::Ps3 => Some(SmallMotor::OnOff),
            Self::Ps4 => Some(SmallMotor::Sum),
        }
    }
}

/// Voices in one pad's ring.
pub const VOICES: usize = 20;
/// Voices mixed into the motors at once.
pub const MIX: usize = 5;

/// What a pad's two motors are told, one byte each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Motors {
    /// The large (low-frequency) motor.
    pub large: u8,
    /// The small (high-frequency) motor, under the player's [`SmallMotor`] law.
    pub small: u8,
}

impl Motors {
    /// Both motors off.
    pub const OFF: Self = Self { large: 0, small: 0 };
}

/// Somewhere motor levels go.
///
/// A seat is a player slot (`Assignment::slot_of`'s numbering); the backend
/// finds the pad behind it. A backend with no motors accepts and ignores.
pub trait MotorOutput {
    /// Sets the levels `seat`'s pad holds until the next call.
    fn set(&mut self, seat: usize, motors: Motors);
    /// Stops every motor on every pad now.
    fn silence(&mut self);
}

/// A voice a caller may later finish or stop. Stale once the voice has been
/// reused: the generation no longer matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Handle {
    slot: u8,
    generation: u32,
}

/// What a step needs to know about the game around it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StepContext {
    /// In a race: `Game` patterns play. Outside one only `Ui` patterns do.
    pub race: bool,
    /// Paused: everything but `Ui` patterns is silent.
    pub paused: bool,
    /// Where the positional gain measures from (the active ship).
    pub listener: [f32; 3],
}

#[derive(Debug, Clone, Copy, Default)]
struct Voice {
    active: bool,
    /// Set: runs to the end of the track. Clear: a looping pattern wraps.
    release: bool,
    frame: i32,
    pattern: usize,
    position: [f32; 3],
    generation: u32,
}

/// One pad's ring of voices over a pattern library.
#[derive(Debug, Clone)]
pub struct Player {
    library: Library,
    law: SmallMotor,
    voices: [Voice; VOICES],
    cursor: usize,
}

impl Player {
    /// A player of `library`'s patterns whose small motor reads as `law`.
    #[must_use]
    pub fn new(library: Library, law: SmallMotor) -> Self {
        Self {
            library,
            law,
            voices: [Voice::default(); VOICES],
            cursor: 0,
        }
    }

    /// The library, for resolving names to ids.
    #[must_use]
    pub fn library(&self) -> &Library {
        &self.library
    }

    /// Voices currently active.
    #[must_use]
    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }

    fn queue(&mut self, id: usize, release: bool, at: Option<[f32; 3]>) -> Option<Handle> {
        self.library.at(id)?;
        for _ in 0..VOICES {
            let slot = self.cursor;
            self.cursor = (self.cursor + 1) % VOICES;
            if !self.voices[slot].active {
                let generation = self.voices[slot].generation.wrapping_add(1);
                self.voices[slot] = Voice {
                    active: true,
                    release,
                    frame: -1,
                    pattern: id,
                    position: at.unwrap_or_default(),
                    generation,
                };
                return Some(Handle {
                    slot: u8::try_from(slot).expect("a ring of 20"),
                    generation,
                });
            }
        }
        None
    }

    /// Plays pattern `id` once through (release flag set), at `at` for a
    /// positional pattern; `None` keeps the origin, as the original does with
    /// one local player. Dropped, with `false`, when all 20 voices are busy or
    /// `id` is not in the library.
    pub fn one_shot(&mut self, id: usize, at: Option<[f32; 3]>) -> bool {
        self.queue(id, true, at).is_some()
    }

    /// Plays pattern `id` held (release flag clear): a looping pattern wraps
    /// until [`Self::finish`] or [`Self::stop`].
    pub fn hold(&mut self, id: usize) -> Option<Handle> {
        self.queue(id, false, None)
    }

    fn voice_mut(&mut self, handle: Handle) -> Option<&mut Voice> {
        let voice = self.voices.get_mut(usize::from(handle.slot))?;
        (voice.active && voice.generation == handle.generation).then_some(voice)
    }

    /// Sets the release flag: the voice plays on to the end of its track.
    pub fn finish(&mut self, handle: Handle) {
        if let Some(voice) = self.voice_mut(handle) {
            voice.release = true;
        }
    }

    /// Silences the voice at once.
    pub fn stop(&mut self, handle: Handle) {
        if let Some(voice) = self.voice_mut(handle) {
            voice.active = false;
        }
    }

    /// Ends every voice. What a race teardown or a disconnect does.
    pub fn stop_all(&mut self) {
        for voice in &mut self.voices {
            voice.active = false;
        }
    }

    /// One pad update, sent to `seat`'s pad through `out`.
    pub fn drive(&mut self, ctx: StepContext, seat: usize, out: &mut dyn MotorOutput) -> Motors {
        let motors = self.step(ctx);
        out.set(seat, motors);
        motors
    }

    /// One pad update. Returns the levels for the device.
    pub fn step(&mut self, ctx: StepContext) -> Motors {
        let mask: u8 = if ctx.race { 0b11 } else { 0b01 };
        let mut slots: [Option<(u8, u8, Priority)>; MIX] = [None; MIX];
        for voice in &mut self.voices {
            if !voice.active {
                continue;
            }
            let Some(pattern) = self.library.at(voice.pattern) else {
                voice.active = false;
                continue;
            };
            voice.frame += 1;
            let frame = voice.frame as usize;
            if frame >= pattern.large.len() && frame >= pattern.small.len() {
                voice.active = false;
                continue;
            }
            if !voice.release && wraps(pattern) && frame >= pattern.loop_end as usize {
                voice.frame = pattern.loop_start as i32;
            }
            let frame = voice.frame as usize;
            let mut large = pattern.large.get(frame).copied().unwrap_or(0);
            let mut small = pattern.small.get(frame).copied().unwrap_or(0);
            if let Some(fall) = pattern.positional {
                let gain = gain(
                    fall.inner,
                    fall.outer,
                    distance(voice.position, ctx.listener),
                );
                large = (f32::from(large) * gain) as u8;
            }
            let class = pattern.effective_class();
            let bit = match class {
                Class::Ui => 0b01,
                Class::Game => 0b10,
            };
            if mask & bit == 0 || (ctx.paused && class != Class::Ui) {
                large = 0;
                small = 0;
            }
            mix_in(&mut slots, (large, small, pattern.effective_priority()));
        }
        let mut large = 0u32;
        let mut small = 0u32;
        for (l, s, _) in slots.iter().flatten() {
            large += u32::from(*l);
            small += u32::from(*s);
        }
        Motors {
            large: large.min(255) as u8,
            small: match self.law {
                SmallMotor::OnOff => {
                    if small > 0 {
                        255
                    } else {
                        0
                    }
                }
                SmallMotor::Sum => small.min(255) as u8,
            },
        }
    }
}

/// A held voice wraps over a real loop range, whatever the pattern's `Type`.
/// An empty range (a one-shot's 0 to 0) would otherwise sit on frame 0 for
/// ever, a stuck motor.
fn wraps(pattern: &Pattern) -> bool {
    pattern.loop_start < pattern.loop_end
}

/// Five slots: a voice takes an empty one, else overwrites the lowest
/// priority, the first of equals, even when its own is lower.
fn mix_in(slots: &mut [Option<(u8, u8, Priority)>; MIX], voice: (u8, u8, Priority)) {
    if let Some(empty) = slots.iter_mut().find(|s| s.is_none()) {
        *empty = Some(voice);
        return;
    }
    let mut lowest = 0;
    for (i, slot) in slots.iter().enumerate() {
        if slot.map(|s| s.2) < slots[lowest].map(|s| s.2) {
            lowest = i;
        }
    }
    slots[lowest] = Some(voice);
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

/// `docs/gameplay/rumble.md`, "Positional gain".
fn gain(inner: f32, outer: f32, d: f32) -> f32 {
    if d <= inner {
        1.0
    } else if d < outer {
        (inner - d) / outer + 1.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests;
