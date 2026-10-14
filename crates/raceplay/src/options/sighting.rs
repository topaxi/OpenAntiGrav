//! [`Sighting`]: the two facts a race needs to run a lock-on and draw its reticle,
//! carried in [`super::Setup`] so [`crate::Race::start`] applies them and a
//! server or a replay player is correct with no extra call.

use oag_race::sight::Sight;

/// Where the lock-on reticle lives and which law it runs.
///
/// Both halves are title facts (the HUD's authored space, the title's sight
/// dialect), read once at load. Before 2026-10-14 they were two setters on `Race`
/// that the windowed game and the capture path called and nothing else did, so a
/// race started without them ran Pulse's LeachBeam under the Missile's law (a lock
/// at 48 ticks instead of 20).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sighting {
    /// The grid the **simulation's** locks are in: the title's whole HUD space,
    /// whatever the local screen is cut into. The locks are hashed state, so a
    /// split-screen client and a server with no screen must agree on them; the
    /// view grid below is a fact about this machine's display and may not enter
    /// it.
    pub lock_screen: [f32; 2],
    /// The grid the drawn reticle projects into: the title's HUD space, or one
    /// view of it when the race loaded an authored two-view split
    /// (`oag_hud::Assets::sight_screen`). View-only.
    pub view_screen: [f32; 2],
    /// Whether a held LeachBeam runs its own law (Pulse) rather than the
    /// Missile's. `oag_title::hud::Sights::runs_leach_law`.
    pub leach_law: bool,
}

impl Default for Sighting {
    /// The PSP's screen and the Missile's law: what a hand-made `Setup` ran
    /// before this existed.
    fn default() -> Self {
        Self {
            lock_screen: oag_race::sight::SCREEN,
            view_screen: oag_race::sight::SCREEN,
            leach_law: false,
        }
    }
}

impl Sighting {
    /// A title's reticle, off its loaded HUD.
    #[must_use]
    pub fn of(hud: &oag_hud::Assets) -> Self {
        let view = hud.sight_screen();
        Self {
            lock_screen: [hud.space.size.0, hud.space.size.1],
            view_screen: [view.0, view.1],
            leach_law: hud.art.sights.runs_leach_law(),
        }
    }

    /// The drawn reticle, in the view grid.
    #[must_use]
    pub fn view_sight(&self) -> Sight {
        let mut sight = Sight::new(self.view_screen);
        sight.set_leach_law(self.leach_law);
        sight
    }

    /// One slot's simulation lock, in the lock grid.
    #[must_use]
    pub fn lock_sight(&self) -> Sight {
        let mut sight = Sight::new(self.lock_screen);
        sight.set_leach_law(self.leach_law);
        sight
    }
}
