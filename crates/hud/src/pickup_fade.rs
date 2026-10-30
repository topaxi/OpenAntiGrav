//! The held pickup's backdrop and icon fade: an eased overshoot on a grant, a hold, and the
//! reverse curve after use.
//!
//! Pulse's `Hud_UpdatePickupWidgets` (`0x0881cdc4`), recovered and verified live
//! (`docs/ghidra/functions/psp-pulse-usa/pickup-colour.md`, confidence 90): a timer `T` gains
//! `dt * rate` per update while a weapon is held and stops at `0.5`; the backdrop word is the
//! weapon's table word scaled, alpha included, by [`ease_with_bump`]`(3 T, 1)`. After use `T`
//! runs on from `0.5` to `1.0` and the word is scaled by [`ease_with_bump`]`((2 - 2 T) * 1.5, 1)`,
//! after a hold at full strength while the icon's own timer is above `0.1`. HD's
//! `Hud_UpdatePickupBar` (`0x0008df30`) has the same shape with rate `1.5` (read: the rate, the
//! cap, the `x3`, the `-25.0`); its hold threshold and fade-out branch are **inherited from
//! Pulse, not read**. Omega inherits HD's.
//!
//! Render-side only: the state advances per rendered tick off the held weapon and `dt`, lives in
//! the HUD's per-slot memory and never reaches the simulation or the race hash.

use oag_tables::weapons::Weapon;

/// The cap `T` holds at while a weapon is held.
const GROWN: f32 = 0.5;
/// The icon starts to show once `T` reaches this.
const ICON_GATE: f32 = 0.4;
/// After use the backdrop holds while the icon's timer is above this.
const HOLD_ABOVE: f32 = 0.1;
/// The icon timer's cap.
const ICON_CAP: f32 = 0.5;

/// `Ease_Cosine` (`0x08819374`): `0` below `0`, `1` above `1`, `(1 - cos(pi x)) / 2` between.
#[must_use]
pub fn ease(x: f32) -> f32 {
    if x < 0.0 {
        0.0
    } else if x > 1.0 {
        1.0
    } else {
        (1.0 - (std::f32::consts::PI * x).cos()) * 0.5
    }
}

/// `Ease_GaussianBump` (`0x088193ec`): `exp(-25 (u - 0.5)^2)` for `u` in `[0, 1]`, else `0`.
fn bump(u: f32) -> f32 {
    if (0.0..=1.0).contains(&u) {
        let d = u - 0.5;
        (-25.0 * d * d).exp()
    } else {
        0.0
    }
}

/// `Ease_CosineWithBump(x, k)` (`0x08819458`): [`ease`] plus `k` times a Gaussian bump centred at
/// `x = 0.75`. With `k = 1` it overshoots to about `1.85` and settles on `1.0` by `x = 1.5`.
#[must_use]
pub fn ease_with_bump(x: f32, k: f32) -> f32 {
    ease(x) + k * bump((x - 0.5) * 2.0)
}

/// `word` (`0xAARRGGBB`) with all four channels scaled by `k`, each truncated and clamped to 255
/// (`Colour_UnpackToFloat4`, multiply, `Colour_PackFromFloat4`).
#[must_use]
pub fn scale_word(word: u32, k: f32) -> u32 {
    let channel = |shift: u32| -> u32 {
        let value = ((word >> shift) & 0xFF) as f32 * k;
        (value.max(0.0) as u32).min(255)
    };
    channel(24) << 24 | channel(16) << 16 | channel(8) << 8 | channel(0)
}

/// What the fade draws this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    /// The weapon whose backdrop word and icon are up: the held one, or after use the last.
    pub weapon: Weapon,
    /// The factor the backdrop word is scaled by (alpha included); above `1` on the overshoot.
    pub backdrop: f32,
    /// The size pulse `f`: the backdrop is `width / f` wide and `height * f` tall.
    pub size: f32,
    /// The factor the icon's white is scaled by; `0` until the icon gate.
    pub icon: f32,
}

impl Look {
    /// The fully grown look: nothing animating.
    #[must_use]
    pub fn settled(weapon: Weapon) -> Self {
        Self {
            weapon,
            backdrop: 1.0,
            size: 1.0,
            icon: 1.0,
        }
    }
}

/// One slot's pickup fade state: `status+0x44` (`T`), the icon's row timer and the last row.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Fade {
    weapon: Option<Weapon>,
    t: f32,
    icon_timer: f32,
    fading: bool,
}

impl Fade {
    /// One update. `rate` is the timer's units per second ([`oag_title::hud::PickupFade`]).
    ///
    /// A grant from a clean slate starts at `T = 0`; a bare switch to another weapon keeps `T`
    /// (live: no new fade-in). After use `T` holds at `0.5` while the icon timer is above
    /// `0.1`, then runs to `1.0`; the next update resets. A grant during that run-on is not
    /// measured: the run-on finishes and the new weapon fades in after it (decompile only).
    pub fn advance(&mut self, held: Option<Weapon>, rate: f32, dt: f32) {
        let step = dt * rate;
        if self.fading && self.t >= 1.0 {
            *self = Self::default();
        }
        match held {
            Some(weapon) if !self.fading => {
                self.weapon = Some(weapon);
                self.t = (self.t + step).min(GROWN);
                if self.t >= ICON_GATE {
                    self.icon_timer = (self.icon_timer + step).min(ICON_CAP);
                }
            }
            _ if self.weapon.is_some() => {
                self.icon_timer = (self.icon_timer - step).max(0.0);
                if self.fading || self.t < GROWN || self.icon_timer <= HOLD_ABOVE {
                    self.fading = true;
                    self.t = (self.t + step).min(1.0);
                }
            }
            _ => {}
        }
    }

    /// What to draw, or `None` with nothing to show. `held` is the readout's held weapon: a
    /// readout built without this state (a test) still draws its weapon fully grown.
    #[must_use]
    pub fn look(&self, held: Option<Weapon>) -> Option<Look> {
        let Some(weapon) = self.weapon else {
            return held.map(Look::settled);
        };
        let (backdrop, size) = if self.fading {
            let x = 2.0 - 2.0 * self.t;
            (ease_with_bump(x * 1.5, 1.0), ease(x * 0.99 + 0.01))
        } else {
            (
                ease_with_bump(3.0 * self.t, 1.0),
                ease(2.0 * self.t * 0.99 + 0.01),
            )
        };
        Some(Look {
            weapon,
            backdrop,
            size,
            icon: ease_with_bump(3.0 * self.icon_timer, 1.0).min(1.0),
        })
    }

    /// Whether a weapon (held, or fading out) is being shown.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.weapon.is_some()
    }
}
