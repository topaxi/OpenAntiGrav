//! The commands that draw into a view: `screen-fill`, `fov-offset` and
//! `fire-mod-effect-ahead`. Outputs only: re-sent each tick they should hold, never
//! hashed, never read by the simulation.

use crate::{Command, Distance, Malformed, Rgba, Slot};

/// The largest `fov-offset`, degrees either way. Chosen.
pub const MAX_FOV_OFFSET_DEG: f32 = 30.0;

/// `(arg, value, word)` of one of this module's commands.
pub(crate) fn encode(command: &Command) -> Option<(u16, u32, u32)> {
    Some(match *command {
        Command::ScreenFill { rgba, .. } => (0, 0, rgba.to_u32()),
        Command::FovOffset { degrees, .. } => (0, degrees.to_bits(), 0),
        Command::FireModEffectAhead {
            index, distance, ..
        } => {
            let (kind, amount) = distance.split();
            (index, amount.to_bits(), u32::from(kind))
        }
        _ => return None,
    })
}

/// The command of one of this module's kinds: an offset in `[-MAX_FOV_OFFSET_DEG,
/// MAX_FOV_OFFSET_DEG]` (a number), a distance of a known kind with an amount above zero.
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
        crate::CommandKind::ScreenFill => Ok(Command::ScreenFill {
            slot,
            rgba: Rgba::from_u32(word),
        }),
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
