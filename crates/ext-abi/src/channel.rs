//! Channels: the named wires mods and a track's triggers send each other events over
//! (`docs/architecture/wasm-extensions/hooks.md`, "Events between mods").
//!
//! A channel is named `<mod id>/<name>` and travels as the FNV-1a 32 hash of that name
//! ([`channel_key`], the same hash as a property's [`prop_key`](crate::track::prop_key),
//! computed at compile time by the SDK's `channel!`). A mod declares in `oag-mod.toml`
//! the channels it publishes and listens to; a module may send only on a channel its mod
//! publishes. The caps are chosen, not measured.

use crate::track::prop_key;

/// What `from` holds in a `mod` event the track itself raised (a trigger's `oag_emit`).
/// A mod's id hashes to a nonzero key (the host refuses an id that does not).
pub const TRACK_SENDER: u32 = 0;
/// The `emit` commands one module may send in one tick; a further one faults it.
pub const MAX_EMITS_PER_TICK: usize = 4;
/// The `mod` events one tick delivers that modules sent: every module's share of
/// [`MAX_EMITS_PER_TICK`] at the most modules a race runs. In sorted order (channel,
/// slot, sender, value) the later are dropped, counted and logged by the host. Chosen.
pub const MAX_MOD_EVENTS: usize = 16;
/// The `mod` events one tick delivers that the track's triggers sent (`from` is
/// [`TRACK_SENDER`]), held apart from [`MAX_MOD_EVENTS`] so a crowd of module emits cannot
/// starve a trigger. In sorted order the later are dropped, counted and logged. Together
/// with [`MAX_MOD_EVENTS`] they are delivered ahead of the engine's own events, which
/// share the rest of [`MAX_EVENTS`](crate::MAX_EVENTS). Chosen.
pub const MAX_TRACK_EVENTS: usize = 8;
/// The longest channel name, bytes.
pub const MAX_CHANNEL_LEN: usize = 64;

/// The key a channel's name travels as.
#[must_use]
pub const fn channel_key(name: &str) -> u32 {
    prop_key(name)
}

/// Why a name is not a channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadChannel {
    /// Longer than [`MAX_CHANNEL_LEN`].
    TooLong,
    /// Not `<mod id>/<name>`: exactly one `/`, with something on each side.
    Shape,
    /// A character outside the allowed set.
    Character(char),
}

impl core::fmt::Display for BadChannel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::TooLong => write!(f, "longer than {MAX_CHANNEL_LEN} bytes"),
            Self::Shape => {
                f.write_str("not `<mod id>/<name>` (exactly one `/`, text on both sides)")
            }
            Self::Character(c) => write!(
                f,
                "{c:?} is not allowed (lowercase letters, digits, `-`, `_` and, in the mod id, `.`)"
            ),
        }
    }
}

/// Checks a namespaced channel name: `<mod id>/<name>`, the id of lowercase letters,
/// digits, `.`, `-` and `_`, the name of the same without the `.`.
///
/// # Errors
///
/// A [`BadChannel`] naming the first rule it breaks.
pub fn check_channel(name: &str) -> Result<(), BadChannel> {
    if name.len() > MAX_CHANNEL_LEN {
        return Err(BadChannel::TooLong);
    }
    let mut parts = name.split('/');
    let (Some(id), Some(local), None) = (parts.next(), parts.next(), parts.next()) else {
        return Err(BadChannel::Shape);
    };
    if id.is_empty() || local.is_empty() {
        return Err(BadChannel::Shape);
    }
    let plain = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_';
    if let Some(c) = id.chars().find(|&c| !plain(c) && c != '.') {
        return Err(BadChannel::Character(c));
    }
    if let Some(c) = local.chars().find(|&c| !plain(c)) {
        return Err(BadChannel::Character(c));
    }
    Ok(())
}

/// The mod id of a channel name that passed [`check_channel`].
#[must_use]
pub fn namespace(name: &str) -> &str {
    name.split('/').next().unwrap_or(name)
}
