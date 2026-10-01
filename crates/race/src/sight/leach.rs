//! The LeachBeam's own reticle law, which is not the Missile's.
//!
//! # Recovered from `FUN_0881e8c8` (`0x0881e8c8`)
//!
//! `Hud_Update` runs `HudSight_Update` (the Missile's) and, when that returns no
//! lock, this: a second, separate function over the other four widgets the bind
//! makes (`leachbeam_sight_1` .. `_4`, `hud+0x108` .. `+0x114`). This module used
//! to hold that these four "take the Missile's law" as an inference and said so;
//! the function was not found until 2026-10-01, and it is a different law:
//!
//! - **The gate is the held weapon.** `view+0x48 == 0xb` - the player's status
//!   record carries the held weapon id plus one, and the LeachBeam is `10` -
//!   with a target at `view+0xec` and no `0x1000` on its flags. `Weapon_FireLeachBeam`
//!   sets the craft's held-weapon slot to `-1` on the tick it fires, so the gate
//!   closes **on the fire tick** and the arrowheads open out and go. Read live on
//!   PPSSPP: `view+0x48` was `0` with nothing held and `11` the tick after a
//!   LeachBeam was written into the slot.
//! - **No hold timer, no chase, no extrapolation.** The target's raw projection
//!   is the centre the instant it is on screen, and it is on screen when it is
//!   inside the screen rectangle (the Missile's tests `2 * new - old`).
//! - **The extent closes to `6.0` and the lock is "it got there"**: `locked =
//!   extent == 6.0`. Taking a target for the first time resets the extent to
//!   `30.0` (`DAT_08ab0840`) so the arrowheads always close in from open.
//! - **Losing the target opens them at `1.5x` the rate** (`DAT_08ab0a78`) toward
//!   `30.0`, and they are hidden once there - about `0.24` seconds from locked.
//! - **The whole figure spins.** One angle (`hud+0x104`) turns the four corner
//!   positions about the centre (`FUN_0881b478`) and is added to each piece's own
//!   quarter-turn: `2.0` rad/s while seeking, `4.0` once locked
//!   (`DAT_08ab0a80`, `DAT_08ab0a7c`), and back at `-2.0` rad/s while open.
//!   Wrapped by `fmodf` at `2 * pi`.
//! - **The colour carries the fade in its alpha byte**: `6 / extent` while a
//!   target is held, `1 - extent / 30` while it is let go, scaled to `255`, with
//!   RGB yellow (`0xffff`) seeking and red (`0xff`) locked. No blink.
//!
//! Constants read live off PPSSPP (`30.0`, `1.5`, `4.0`, `2.0`); the rest are
//! instruction literals. Evidence and the disassembly walk:
//! `docs/ghidra/functions/psp-pulse-usa/lock-sight.md`, "The LeachBeam's reticle
//! is its own function".

use super::{EXTENT_OPEN, EXTENT_RATE, EXTENT_SNAP, Held, Piece, Projected, Sight, State};

/// The half-extent a held target closes to.
pub const LEACH_EXTENT_CLOSED: f32 = 6.0;

/// What [`LEACH_EXTENT_CLOSED`] is scaled by to give the figure `locked` is tested
/// against while nothing is held (`0x3fcccccd`, `1.6`): `9.6`, which an opening
/// extent only ever equals by accident.
pub const LEACH_UNHELD_REFERENCE: f32 = 6.0 * 1.6;

/// The extra rate at which the arrowheads open once the target is let go
/// (`DAT_08ab0a78`).
pub const LEACH_OPEN_RATE: f32 = 1.5;

/// The figure's turn rate while seeking, radians a second (`DAT_08ab0a80`).
pub const LEACH_SPIN_SEEKING: f32 = 2.0;

/// The figure's turn rate once locked (`DAT_08ab0a7c`).
pub const LEACH_SPIN_LOCKED: f32 = 4.0;

impl Sight {
    /// Chooses the law a held LeachBeam's reticle runs on.
    ///
    /// **Off by default, and on for the PSP dialect only.** Wipeout HD's own
    /// LeachBeam reticle has its own update (`Hud_UpdateLeachBeamSight`), unread
    /// as a law here, and keeps the Missile's - see [`Sight::hold_progress`].
    pub fn set_leach_law(&mut self, on: bool) {
        self.leach_law = on;
    }

    /// Whether the title's LeachBeam reticle runs its own law
    /// ([`Self::set_leach_law`]), the Pulse dialect.
    ///
    /// The fire gate keys on this and not on [`Self::runs_leach_law`]: the shot
    /// is taken while the sight still holds the LeachBeam, but a title whose
    /// reticle keeps the Missile's law (Wipeout HD) fires off the window alone.
    #[must_use]
    pub fn leach_law(&self) -> bool {
        self.leach_law
    }

    /// Whether this frame's law is the LeachBeam's own.
    pub(super) fn runs_leach_law(&self) -> bool {
        self.leach_law && self.held == Held::LeachBeam
    }

    /// One frame of `FUN_0881e8c8`, given where the held target projected.
    pub(super) fn update_leach(&mut self, dt: f32, target: Option<Projected>) -> State {
        let on_screen = target.map(|p| p.screen).filter(|s| {
            (0.0..self.screen[0]).contains(&s[0]) && (0.0..self.screen[1]).contains(&s[1])
        });
        self.previous = target.map(|p| p.screen);

        if let Some(at) = on_screen {
            if !self.leach_seen {
                // First frame on a target: always close in from open.
                self.extent = EXTENT_OPEN;
            }
            self.centre = at;
        }
        self.target = on_screen;
        let visible = on_screen.is_some();

        let (wanted, reference) = if visible {
            (LEACH_EXTENT_CLOSED, LEACH_EXTENT_CLOSED)
        } else {
            (EXTENT_OPEN, LEACH_UNHELD_REFERENCE)
        };
        let mut step = dt * EXTENT_RATE;
        if self.extent > reference {
            step *= EXTENT_SNAP;
        }
        if !visible {
            step *= LEACH_OPEN_RATE;
        }
        self.extent = super::ease_toward(self.extent, wanted, step);
        self.locked = self.extent == reference;

        self.spin = if visible {
            let rate = if self.locked {
                LEACH_SPIN_LOCKED
            } else {
                LEACH_SPIN_SEEKING
            };
            self.spin + dt * rate
        } else {
            self.spin - LEACH_SPIN_SEEKING * dt
        } % std::f32::consts::TAU;

        let fade = if visible {
            LEACH_EXTENT_CLOSED / self.extent
        } else {
            1.0 - self.extent / EXTENT_OPEN
        };
        // `trunc.w.s` of `min(fade * 255, 255)`.
        self.alpha = (fade * 255.0).min(255.0).trunc();
        self.leach_seen = visible;
        self.hold = 0.0;

        match (visible, self.locked) {
            (false, _) => State::Absent,
            (true, false) => State::Seeking,
            (true, true) => State::Locked,
        }
    }

    /// The four arrowheads: the corners turned about the centre by the spin, and
    /// each piece's own quarter turn plus the spin.
    pub(super) fn leach_pieces(&self) -> [Piece; 4] {
        let e = self.extent;
        let [cx, cy] = self.centre;
        let (sin, cos) = oag_core::math::sin_cos(self.spin);
        let corners = [[-e, -e], [e, -e], [-e, e], [e, e]];
        std::array::from_fn(|i| {
            let [dx, dy] = corners[i];
            Piece {
                centre: [cx + dx * cos - dy * sin, cy + dx * sin + dy * cos],
                rotation: super::BRACKET_ROTATIONS[i] + self.spin,
            }
        })
    }
}
