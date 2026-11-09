//! The commands that draw into a view: `screen-fill`, `fov-offset`,
//! `fire-mod-effect-ahead`, `draw-beam`, `set-shader-param` and `portal-view`. Outputs only:
//! re-sent each tick they should hold, never hashed, never read by the simulation.

use crate::{Command, Distance, Malformed, Rgba, Slot};

/// The largest `fov-offset`, degrees either way. Chosen.
pub const MAX_FOV_OFFSET_DEG: f32 = 30.0;

/// The largest beam, half its width in world units. Chosen.
pub const MAX_BEAM_HALF_WIDTH: f32 = 32.0;

/// `draw-beam` commands one module may send a tick; its next is a fault. Chosen.
pub const MAX_BEAMS_PER_TICK: usize = 16;

/// The scalars a mod's shader has.
pub const MAX_SHADER_PARAMS: u8 = 4;

/// The last place in a mod's `[shaders]` a draw can name.
pub const MAX_SHADER_INDEX: u16 = 254;

/// The generations of a `portal-view` snapshot: six bits of `arg`.
pub const MAX_VIEW_GENERATION: u8 = 64;

/// When a `portal-view` is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewUpdate {
    /// One snapshot, drawn the first frame the portal is in the view, and again when
    /// `generation` changes.
    Once,
    /// Every frame the portal is in the view.
    #[default]
    Continuous,
}

impl ViewUpdate {
    const fn code(self) -> u16 {
        match self {
            Self::Once => 1,
            Self::Continuous => 2,
        }
    }

    const fn from_code(code: u16) -> Option<Self> {
        match code {
            1 => Some(Self::Once),
            2 => Some(Self::Continuous),
            _ => None,
        }
    }
}

/// The sign bit of a `portal-view` exit word: set, the word is a trigger index in its low 16
/// bits. A progress is never negative, so a sign bit is free to say which it is.
const EXIT_IS_TRIGGER: u32 = 1 << 31;

/// A `portal-view` exit's word: a progress as its `f32` bits, a trigger as
/// [`EXIT_IS_TRIGGER`] and the index. `None` for any other kind of distance.
fn exit_word(exit: Distance) -> Option<u32> {
    match exit {
        Distance::Progress(progress) => Some(progress.to_bits()),
        Distance::Trigger(index) => Some(EXIT_IS_TRIGGER | u32::from(index)),
        _ => None,
    }
}

/// The exit a word names; `None` for a set sign bit with anything above the index (a negative
/// progress, negative zero apart: that reads as trigger 0).
fn exit_of(word: u32) -> Option<Distance> {
    if word & EXIT_IS_TRIGGER == 0 {
        return Some(Distance::Progress(f32::from_bits(word)));
    }
    (word & !EXIT_IS_TRIGGER <= u32::from(u16::MAX))
        .then_some(Distance::Trigger((word & 0xffff) as u16))
}

/// A shader in the byte `screen-fill` and `draw-beam` carry it in: 0 for none, else its
/// place plus one.
fn shader_byte(shader: Option<u16>) -> u16 {
    shader.map_or(0, |shader| shader + 1)
}

fn shader_of(byte: u16) -> Option<u16> {
    byte.checked_sub(1)
}

/// `(arg, value, word)` of one of this module's commands.
pub(crate) fn encode(command: &Command) -> Option<(u16, u32, u32)> {
    Some(match *command {
        Command::ScreenFill { rgba, shader, .. } => (shader_byte(shader), 0, rgba.to_u32()),
        Command::FovOffset { degrees, .. } => (0, degrees.to_bits(), 0),
        Command::FireModEffectAhead {
            index, distance, ..
        } => {
            let (kind, amount) = distance.split();
            (index, amount.to_bits(), u32::from(kind))
        }
        Command::DrawBeam {
            to,
            rgba,
            half_width,
            shader,
            ..
        } => (
            to.index() as u16 | shader_byte(shader) << 8,
            half_width.to_bits(),
            rgba.to_u32(),
        ),
        Command::SetShaderParam {
            shader,
            index,
            value,
            ..
        } => (shader, value.to_bits(), u32::from(index)),
        Command::PortalView {
            entrance,
            exit,
            radius,
            update,
            generation,
            ..
        } => (
            update.code() | u16::from(generation) << 2 | u16::from(radius) << 8,
            entrance.to_bits(),
            exit_word(exit)?,
        ),
        _ => return None,
    })
}

/// The command of one of this module's kinds: an offset in `[-MAX_FOV_OFFSET_DEG,
/// MAX_FOV_OFFSET_DEG]` (a number), a distance of a known kind with an amount above zero,
/// a shader byte, a beam to another slot of a width in range, a scalar that exists, a portal
/// with two progresses that are numbers and not below zero, a known update and a radius.
pub(crate) fn decode(
    kind: crate::CommandKind,
    slot: Slot,
    (arg, value, word): (u16, f32, u32),
) -> Result<Command, Malformed> {
    let bad = Malformed::Scene {
        kind: kind.code(),
        arg,
        value,
        word,
    };
    match kind {
        crate::CommandKind::ScreenFill => {
            if arg > MAX_SHADER_INDEX + 1 {
                return Err(bad);
            }
            Ok(Command::ScreenFill {
                slot,
                rgba: Rgba::from_u32(word),
                shader: shader_of(arg),
            })
        }
        crate::CommandKind::DrawBeam => {
            let to = Slot::new(usize::from(arg & 0xff)).filter(|to| *to != slot);
            let to = to.ok_or(bad)?;
            if !(value > 0.0 && value <= MAX_BEAM_HALF_WIDTH) {
                return Err(bad);
            }
            Ok(Command::DrawBeam {
                slot,
                to,
                rgba: Rgba::from_u32(word),
                half_width: value,
                shader: shader_of(arg >> 8),
            })
        }
        crate::CommandKind::SetShaderParam => {
            let index = u8::try_from(word)
                .ok()
                .filter(|index| *index < MAX_SHADER_PARAMS)
                .ok_or(bad)?;
            if !value.is_finite() {
                return Err(bad);
            }
            Ok(Command::SetShaderParam {
                slot,
                shader: arg,
                index,
                value,
            })
        }
        crate::CommandKind::PortalView => {
            let on_a_lap = |progress: f32| progress.is_finite() && progress >= 0.0;
            let radius = (arg >> 8) as u8;
            let update = ViewUpdate::from_code(arg & 3);
            let exit = exit_of(word).filter(|exit| match *exit {
                Distance::Progress(progress) => on_a_lap(progress),
                _ => true,
            });
            let (Some(update), Some(exit), true, true) =
                (update, exit, on_a_lap(value), radius > 0)
            else {
                return Err(bad);
            };
            Ok(Command::PortalView {
                slot,
                entrance: value,
                exit,
                radius,
                update,
                generation: (arg >> 2 & 0x3f) as u8,
            })
        }
        crate::CommandKind::FovOffset => {
            if !(-MAX_FOV_OFFSET_DEG..=MAX_FOV_OFFSET_DEG).contains(&value) {
                return Err(bad);
            }
            Ok(Command::FovOffset {
                slot,
                degrees: value,
            })
        }
        _ => {
            let distance = u8::try_from(word)
                .ok()
                .and_then(|kind| Distance::join(kind, value))
                .ok_or(bad)?;
            Ok(Command::FireModEffectAhead {
                slot,
                index: arg,
                distance,
            })
        }
    }
}
