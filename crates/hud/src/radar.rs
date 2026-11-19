//! The 2048-dialect radar's dots: `RadarDot0`-`7` placed and faded off each other craft's
//! position relative to the player.
//!
//! The law is Omega's 2048-mode HUD update, read on the 1.07 patch executable (the dot loop
//! after `hud+0x8f0..0x8f8` are bound, `docs/ghidra/functions/ps4-omega-eu/hud-radar.md`,
//! "The dots"). Dot `N` follows grid slot `N`; the player's own slot and slots past the field
//! hide theirs. For a craft at offset `d` from the player:
//!
//! - `f` is `d` normalised dotted with the player's forward axis, `r` with its right axis, and
//!   `t = min(|d| / RANGE, 1)`.
//! - `x = -(t * r * half_width)`, from the radar's origin.
//! - `y = f * t * half_height * k`, where `k` is [`Kind::behind_scale`] when `f < 0` and `-0.5`
//!   otherwise.
//! - Alpha: half radar `255 - 127.5 t` (truncated), `0` beyond [`RANGE`]; full radar
//!   `max(1 - t, 0.2) * 255`. The colour is white.
//! - The half radar places a craft only when `f < 0` (behind); the full radar places all.
//!
//! Which radar a layout carries is read off the layout: the half radar's dots start on the top
//! edge of the `Radar` rectangle (its origin is `(480, 439)`, where the bowl hangs from), the
//! full radar's at the ellipse's centre.

use oag_title::HudArt;

use super::{Layout, Readout, Sprite};

/// How many dots the HUD binds.
pub const DOTS: usize = 8;

/// The world distance at which a dot reaches the rim: `200.0`, stored at `hud+0x8f8` by the
/// bind.
pub const RANGE: f32 = 200.0;

/// One other craft as the radar sees it, from the player's frame.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Blip {
    /// The offset from the player to the craft, normalised, along the player's right axis.
    pub right: f32,
    /// The same, along the player's forward axis.
    pub forward: f32,
    /// The offset's length in world units.
    pub distance: f32,
}

impl Blip {
    /// The blip of a craft at `offset` (its position less the player's) for a player facing
    /// `forward` with `right` at its side. `None` when the craft sits on the player: the
    /// original's normalise has no direction there.
    #[must_use]
    pub fn from_offset(offset: [f32; 3], forward: [f32; 3], right: [f32; 3]) -> Option<Self> {
        let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let distance = dot(offset, offset).sqrt();
        if distance <= 0.0 {
            return None;
        }
        let direction = offset.map(|c| c / distance);
        Some(Self {
            right: dot(direction, right),
            forward: dot(direction, forward),
            distance,
        })
    }
}

/// Which of the two radars the layout draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The bowl behind the craft (`HUD_half_radar.xml`).
    Half,
    /// The full ellipse (`HUD_full_radar.xml`, the Eliminator and Zombie roots).
    Full,
}

impl Kind {
    /// The per-radar constant a craft behind the player scales `y` by: the two floats at the
    /// table the update indexes with the full-radar flag.
    #[must_use]
    pub const fn behind_scale(self) -> f32 {
        match self {
            Self::Half => -2.0,
            Self::Full => -1.0,
        }
    }
}

/// `y`'s scale for a craft ahead of the player (full radar only).
const AHEAD_SCALE: f32 = -0.5;
/// The full radar's alpha floor.
const FULL_FLOOR: f32 = 0.2;

/// Where a dot goes and how opaque it is: an offset from the radar's origin in layout pixels,
/// and alpha `0..=1`. `None` hides the dot.
#[must_use]
pub fn place(kind: Kind, blip: Blip, half_size: [f32; 2]) -> Option<([f32; 2], f32)> {
    let t = (blip.distance / RANGE).min(1.0);
    if kind == Kind::Half && blip.forward >= 0.0 {
        return None;
    }
    let x = -(t * blip.right * half_size[0]);
    let scale = if blip.forward >= 0.0 {
        AHEAD_SCALE
    } else {
        kind.behind_scale()
    };
    let y = blip.forward * t * half_size[1] * scale;
    let alpha = match kind {
        Kind::Half if blip.distance > RANGE => 0.0,
        Kind::Half => (255.0 - 127.5 * t).trunc() / 255.0,
        Kind::Full => ((1.0 - t).max(FULL_FLOOR) * 255.0).trunc() / 255.0,
    };
    Some(([x, y], alpha))
}

/// The radar's dots for `readout`: each placed blip's `RadarDotN` sprite moved to its spot and
/// faded. Empty for a title or layout that draws no radar.
pub fn dot_sprites(layout: &Layout, art: &HudArt, readout: &Readout) -> Vec<Sprite> {
    if !art.always_on.contains(&"Radar") {
        return Vec::new();
    }
    let Some(radar) = layout.sprite("Radar") else {
        return Vec::new();
    };
    let half_size = [radar.rect[2] * 0.5, radar.rect[3] * 0.5];
    let mut out = Vec::new();
    for (index, blip) in readout.radar_blips.iter().enumerate() {
        let (Some(blip), Some(dot)) = (blip, layout.sprite(&format!("RadarDot{index}"))) else {
            continue;
        };
        let centre = [
            dot.rect[0] + dot.rect[2] * 0.5,
            dot.rect[1] + dot.rect[3] * 0.5,
        ];
        let kind = if (centre[1] - radar.rect[1]).abs() < 1.0 {
            Kind::Half
        } else {
            Kind::Full
        };
        let Some((offset, alpha)) = place(kind, *blip, half_size) else {
            continue;
        };
        let mut sprite = dot.clone();
        sprite.rect[0] += offset[0];
        sprite.rect[1] += offset[1];
        sprite.color = [1.0, 1.0, 1.0, alpha];
        out.push(sprite);
    }
    out
}
