//! The HUD outputs' vocabulary: the elements a mod addresses, the nine anchors, a
//! placement packed in one word, and a string read out of the reply's pool.

use crate::{POOL_AT, POOL_LEN};

/// The most `hud-text` and `hud-icon` items one slot may draw in a tick. Past it the
/// module is faulted, naming the cap. Chosen.
pub const MAX_HUD_ITEMS: usize = 32;
/// The `fire-effect` and `fire-mod-effect` commands one module may send in one tick;
/// a further one faults it. Each is a particle emission the view plays, and a stage
/// that is full evicts the title's own one-shot effects, so a mod cannot flood it.
/// Chosen.
pub const MAX_FIRE_EFFECTS_PER_TICK: usize = 8;
/// The longest `hud-text` string or `hud-icon` sprite name, in bytes. Chosen.
pub const MAX_HUD_TEXT: usize = 48;
/// The largest `hud-shake` amplitude, in HUD pixels. Chosen.
pub const MAX_HUD_SHAKE: f32 = 16.0;
/// The largest `hud-bar-extra` share: a bar may show up to five times its value.
/// Chosen.
pub const MAX_HUD_EXTRA: f32 = 4.0;
/// A placement's offset range, in HUD pixels either way: 12 bits, signed.
pub const MAX_HUD_OFFSET: i16 = 2047;

coded! {
    /// A HUD element a mod addresses. Title-agnostic: a title maps each to its own
    /// widgets, and one it has none for draws nothing.
    HudElement: u16, first 1 {
        /// Code 1: the speed bar and its readout.
        Speed,
        /// Code 2: the thrust bar (HD, Omega, 2048 and the retro skins).
        Thrust,
        /// Code 3: the energy bar (Pulse's `ShieldBar`).
        Energy,
        /// Code 4: the held pickup's icon and backdrop.
        Pickup,
        /// Code 5: the race position.
        Position,
        /// Code 6: the lap counter.
        Lap,
        /// Code 7: the race and lap times.
        Time,
    }
}

coded! {
    /// Where a placement's offset is measured from, in the title's HUD layout space
    /// inside the safe area.
    Anchor: u8, first 0 {
        /// Code 0.
        TopLeft,
        /// Code 1.
        Top,
        /// Code 2.
        TopRight,
        /// Code 3.
        Left,
        /// Code 4.
        Center,
        /// Code 5.
        Right,
        /// Code 6.
        BottomLeft,
        /// Code 7.
        Bottom,
        /// Code 8.
        BottomRight,
    }
}

/// Where a `hud-text` or `hud-icon` goes, packed in one word: anchor in the low 4
/// bits, size in quarters of the title's own size in the next 4 (1 to 15), then x and
/// y as signed 12-bit offsets in HUD pixels from the anchor, y down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudPlace {
    /// The anchor.
    pub anchor: Anchor,
    /// Size in quarters: 4 is the title's own text height or the sprite's own size.
    pub quarters: u8,
    /// Offset right of the anchor, `-MAX_HUD_OFFSET..=MAX_HUD_OFFSET`.
    pub x: i16,
    /// Offset down from the anchor, same range.
    pub y: i16,
}

impl HudPlace {
    /// The packed word.
    #[must_use]
    pub const fn to_bits(self) -> u32 {
        (self.anchor as u32)
            | (self.quarters as u32 & 15) << 4
            | (self.x as u32 & 0xfff) << 8
            | (self.y as u32 & 0xfff) << 20
    }

    /// Unpacked; `None` for a size of 0 or an anchor past the nine.
    #[must_use]
    pub fn from_bits(word: u32) -> Option<Self> {
        let sign = |raw: u32| ((raw << 20) as i32 >> 20) as i16;
        let quarters = ((word >> 4) & 15) as u8;
        let anchor = Anchor::from_code((word & 15) as u8)?;
        (quarters > 0).then_some(Self {
            anchor,
            quarters,
            x: sign((word >> 8) & 0xfff),
            y: sign((word >> 20) & 0xfff),
        })
    }

    /// The size as a multiple of the title's own.
    #[must_use]
    pub fn scale(self) -> f32 {
        f32::from(self.quarters) * 0.25
    }
}

/// A short string carried in a command: at most [`MAX_HUD_TEXT`] bytes of UTF-8.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct HudStr {
    len: u8,
    bytes: [u8; MAX_HUD_TEXT],
}

impl core::fmt::Debug for HudStr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self.as_str(), f)
    }
}

impl HudStr {
    /// The string, if it fits.
    #[must_use]
    pub fn new(text: &str) -> Option<Self> {
        let source = text.as_bytes();
        if source.len() > MAX_HUD_TEXT {
            return None;
        }
        let mut bytes = [0; MAX_HUD_TEXT];
        bytes[..source.len()].copy_from_slice(source);
        Some(Self {
            len: source.len() as u8,
            bytes,
        })
    }

    /// The text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap_or("")
    }
}

/// The pool entry at `at`: a length byte, then that many bytes of UTF-8.
pub(crate) fn pool_string(pool: &[u8], at: u16) -> Option<HudStr> {
    let at = usize::from(at);
    let len = usize::from(*pool.get(at)?);
    HudStr::new(core::str::from_utf8(pool.get(at + 1..at + 1 + len)?).ok()?)
}

/// The reply's pool inside a whole buffer.
#[must_use]
pub fn pool(io: &[u8; crate::IO_LEN]) -> &[u8] {
    &io[POOL_AT..POOL_AT + POOL_LEN]
}
