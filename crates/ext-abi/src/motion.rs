//! The commands that move a craft along the circuit or change its speed: `transit`,
//! `scale-speed` and `probe-clearance`, with the distance vocabulary they share.

use crate::{Command, Malformed, Slot};

/// The most ticks a `transit` may last: ten seconds.
pub const MAX_TRANSIT_TICKS: u16 = 600;
/// The most a `transit` may scale the speed it exits at, and a `scale-speed` the speed.
pub const MAX_SPEED_FACTOR: f32 = 2.0;
/// The largest radius a `probe-clearance` may ask about, track units.
pub const MAX_PROBE_RADIUS: f32 = 64.0;
/// A `transit`'s exit speed factor travels in steps of this many to one.
pub const EXIT_FACTOR_STEPS: f32 = 10_000.0;

/// How far along the circuit a `transit` or `probe-clearance` reaches. The host resolves
/// it to track units when the command lands and refuses one that is not inside
/// `(0, half the lap)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Distance {
    /// Track units along the circuit.
    Units(f32),
    /// The ground the craft covers in this many seconds at the speed it has when the
    /// command lands.
    Seconds(f32),
    /// This share of the lap's length.
    Lap(f32),
}

impl Distance {
    /// Its kind code and amount.
    #[must_use]
    pub const fn split(self) -> (u8, f32) {
        match self {
            Self::Units(amount) => (0, amount),
            Self::Seconds(amount) => (1, amount),
            Self::Lap(amount) => (2, amount),
        }
    }

    /// The distance a kind code and an amount name; `None` for an unknown kind or an
    /// amount that is not finite and above zero.
    #[must_use]
    pub fn join(kind: u8, amount: f32) -> Option<Self> {
        if !(amount.is_finite() && amount > 0.0) {
            return None;
        }
        Some(match kind {
            0 => Self::Units(amount),
            1 => Self::Seconds(amount),
            2 => Self::Lap(amount),
            _ => return None,
        })
    }
}

/// `(arg, value, word)` of one of this module's commands.
pub(crate) fn encode(command: &Command) -> Option<(u16, u32, u32)> {
    Some(match *command {
        Command::Transit {
            ticks,
            distance,
            exit_factor,
            ..
        } => {
            let (kind, amount) = distance.split();
            // An integer count of steps, rounded: the same factor on every machine.
            let steps = (exit_factor * EXIT_FACTOR_STEPS + 0.5) as u32;
            (ticks, amount.to_bits(), u32::from(kind) | steps << 16)
        }
        Command::ScaleSpeed { factor, .. } => (0, factor.to_bits(), 0),
        Command::ProbeClearance {
            distance, radius, ..
        } => {
            let (kind, amount) = distance.split();
            (u16::from(kind), amount.to_bits(), radius.to_bits())
        }
        _ => return None,
    })
}

/// The command of one of this module's kinds, every check that needs no observation
/// done: a transit of `1..=MAX_TRANSIT_TICKS` ticks, a known distance kind with an
/// amount above zero, factors in `[0, MAX_SPEED_FACTOR]`, a radius in
/// `(0, MAX_PROBE_RADIUS]`.
pub(crate) fn decode(
    kind: crate::CommandKind,
    slot: Slot,
    (arg, value, word): (u16, f32, u32),
) -> Result<Command, Malformed> {
    let bad = Malformed::Motion {
        kind: kind.code(),
        arg,
        value,
        word,
    };
    match kind {
        crate::CommandKind::Transit => {
            let distance = Distance::join((word & 0xff) as u8, value).ok_or(bad)?;
            let exit_factor = (word >> 16) as f32 / EXIT_FACTOR_STEPS;
            if arg == 0 || arg > MAX_TRANSIT_TICKS || exit_factor > MAX_SPEED_FACTOR {
                return Err(bad);
            }
            Ok(Command::Transit {
                slot,
                ticks: arg,
                distance,
                exit_factor,
            })
        }
        crate::CommandKind::ScaleSpeed => {
            if !(0.0..=MAX_SPEED_FACTOR).contains(&value) {
                return Err(bad);
            }
            Ok(Command::ScaleSpeed {
                slot,
                factor: value,
            })
        }
        _ => {
            let distance = u8::try_from(arg)
                .ok()
                .and_then(|kind| Distance::join(kind, value))
                .ok_or(bad)?;
            let radius = f32::from_bits(word);
            if !(radius > 0.0 && radius <= MAX_PROBE_RADIUS) {
                return Err(bad);
            }
            Ok(Command::ProbeClearance {
                slot,
                distance,
                radius,
            })
        }
    }
}
