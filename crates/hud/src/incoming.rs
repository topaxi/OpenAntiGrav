//! Incoming-weapon warnings: `ForwardWarningIcons` and `RearWarningIcons`.
//!
//! A weapon fired by any craft but the viewer's own raises a warning when the
//! along-track gap `g = progress(viewer) - progress(firer)` lies strictly inside
//! the weapon's window, on the rear slot (a threat behind) or the forward slot (a
//! mine or bomb laid ahead). The slot then runs a 2.0 s timer and blinks its
//! background and the weapon's icon at 4 Hz. The target, the lock and the shot's
//! later flight are never read. Law, windows and the blink schedule:
//! `docs/ghidra/functions/psp-pulse-usa/hud-incoming-warnings.md`
//! (`Hud_RaiseIncomingWarning` `0x0881a364`, `Hud_UpdateIncomingWarnings`
//! `0x0881aa8c`, confidence 95).
//!
//! The warning is a view of the race, never simulation state: [`Warnings`] is
//! advanced by the host once per drawn tick and read through [`Shown`].

use oag_tables::weapons::Weapon;

/// How long a raised warning runs: `Hud_RaiseIncomingWarning`'s `2.0f`.
pub const SECONDS: f32 = 2.0;

/// Beyond this many units either way the gap is not looked at
/// (`Weapon_AnnounceIncoming`'s `|g| > 800` return).
const REACH: f32 = 800.0;

/// Which warning group a raise lands in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// `ForwardWarningIcons`: a mine or bomb ahead of the viewer.
    Forward,
    /// `RearWarningIcons`: a shot from behind.
    Rear,
}

/// The weapons that have a warning icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// `RearWarningRocketIcon`.
    Rocket,
    /// `RearWarningMissileIcon`.
    Missile,
    /// `RearWarningQuakeIcon`.
    Quake,
    /// `RearWarningCannonIcon`.
    Cannon,
    /// `RearWarningPlasmaIcon`.
    Plasma,
    /// `RearWarningLeachBeamIcon`.
    LeachBeam,
    /// `RearWarningRepulserIcon`.
    Repulser,
    /// `RearWarningShurikenIcon`.
    Shuriken,
    /// `ForwardWarningMineIcon`.
    Mine,
    /// `ForwardWarningBombIcon`.
    Bomb,
}

impl Icon {
    /// The layout widget this icon is drawn as.
    #[must_use]
    pub const fn widget(self) -> &'static str {
        match self {
            Self::Rocket => "RearWarningRocketIcon",
            Self::Missile => "RearWarningMissileIcon",
            Self::Quake => "RearWarningQuakeIcon",
            Self::Cannon => "RearWarningCannonIcon",
            Self::Plasma => "RearWarningPlasmaIcon",
            Self::LeachBeam => "RearWarningLeachBeamIcon",
            Self::Repulser => "RearWarningRepulserIcon",
            Self::Shuriken => "RearWarningShurikenIcon",
            Self::Mine => "ForwardWarningMineIcon",
            Self::Bomb => "ForwardWarningBombIcon",
        }
    }

    /// The group this icon belongs to.
    #[must_use]
    pub const fn side(self) -> Side {
        match self {
            Self::Mine | Self::Bomb => Side::Forward,
            _ => Side::Rear,
        }
    }
}

impl Side {
    /// The slot's background widget.
    #[must_use]
    pub const fn background(self) -> &'static str {
        match self {
            Self::Forward => "ForwardWarningBackground",
            Self::Rear => "RearWarningBackground",
        }
    }
}

/// The window a fired weapon warns inside: `(low, high)` of `low < g < high`, and
/// its icon. `None` for a weapon with no warning (Turbo, Shield, Autopilot, the
/// Disruptor and a mod's own).
#[must_use]
pub fn window(weapon: Weapon) -> Option<(Icon, f32, f32)> {
    Some(match weapon {
        Weapon::Rocket => (Icon::Rocket, 0.0, 400.0),
        Weapon::Missile => (Icon::Missile, 0.0, 400.0),
        Weapon::Quake => (Icon::Quake, 0.0, 800.0),
        Weapon::Cannon => (Icon::Cannon, 0.0, 500.0),
        Weapon::Plasma => (Icon::Plasma, 0.0, 500.0),
        Weapon::LeachBeam => (Icon::LeachBeam, 0.0, 500.0),
        Weapon::Repulser => (Icon::Repulser, 0.0, 500.0),
        Weapon::Shuriken => (Icon::Shuriken, 0.0, 500.0),
        Weapon::Mine => (Icon::Mine, -500.0, 0.0),
        Weapon::Bomb => (Icon::Bomb, -500.0, 0.0),
        _ => return None,
    })
}

/// The icon a weapon fired at along-track gap `gap` (viewer minus firer) raises,
/// if any. Both bounds are strict.
#[must_use]
pub fn classify(weapon: Weapon, gap: f32) -> Option<Icon> {
    let (icon, low, high) = window(weapon)?;
    (gap.abs() <= REACH && low < gap && gap < high).then_some(icon)
}

/// One group's timer and the icon it carries.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Slot {
    timer: f32,
    icon: Option<Icon>,
}

impl Slot {
    /// `trunc(timer * 8) & 1` from the timer before this call's decrement, then
    /// the decrement; the icon is dropped on the first call with the timer spent.
    fn advance(&mut self, dt: f32) -> bool {
        let on = (self.timer * 8.0) as i64 & 1 == 1;
        if self.timer > 0.0 {
            self.timer = (self.timer - dt).max(0.0);
        } else {
            self.icon = None;
        }
        on
    }
}

/// What the two groups show this frame: the icon of each group that is in a lit
/// phase. A lit group draws its background and that icon.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Shown {
    /// The forward group, when lit.
    pub forward: Option<Icon>,
    /// The rear group, when lit.
    pub rear: Option<Icon>,
}

/// The two groups' state, the original's `hud+0x180..0x18c`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Warnings {
    forward: Slot,
    rear: Slot,
    shown: Shown,
}

impl Warnings {
    /// Starts `icon`'s group on its 2.0 s timer, overwriting a running one. The one
    /// entry point a raise goes through.
    pub fn raise(&mut self, icon: Icon) {
        let slot = match icon.side() {
            Side::Forward => &mut self.forward,
            Side::Rear => &mut self.rear,
        };
        *slot = Slot {
            timer: SECONDS,
            icon: Some(icon),
        };
    }

    /// Raises the warning a `weapon` fired at `gap` (viewer minus firer) earns, if
    /// it earns one. Returns whether it did.
    pub fn raise_for_fire(&mut self, weapon: Weapon, gap: f32) -> bool {
        classify(weapon, gap).map(|icon| self.raise(icon)).is_some()
    }

    /// One frame of `Hud_UpdateIncomingWarnings`, after any raise of the same
    /// frame (so the raise frame itself is unlit: `8 * 2.0` is even).
    pub fn advance(&mut self, dt: f32) {
        let forward_on = self.forward.advance(dt);
        let rear_on = self.rear.advance(dt);
        // The icon is read after the advance: a group whose timer ran out this call
        // was in an unlit phase, so it shows nothing either way.
        self.shown = Shown {
            forward: self.forward.icon.filter(|_| forward_on),
            rear: self.rear.icon.filter(|_| rear_on),
        };
    }

    /// What to draw this frame.
    #[must_use]
    pub const fn shown(&self) -> Shown {
        self.shown
    }

    /// Whether either group has a timer running.
    #[must_use]
    pub fn active(&self) -> bool {
        self.forward.timer > 0.0 || self.rear.timer > 0.0
    }
}

#[cfg(test)]
mod tests;
