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
    /// The grid a view of a **left-and-right** window projects into, where the title authors
    /// a `vert_` split family whose view is not [`Self::view_screen`]'s (HD: 960x1080 against
    /// 1920x540). `None` draws every view in `view_screen`. A `2h,2v` race has both kinds of
    /// window, so the grid is per view, fitted by `Race::fit_sights`.
    pub columns_screen: Option<[f32; 2]>,
    /// Whether a held LeachBeam runs its own law (Pulse) rather than the
    /// Missile's. `oag_title::hud::Sights::runs_leach_law`.
    pub leach_law: bool,
    /// The square the law runs in when it is not the grid (the PS2's 512x512
    /// buffer), or `None`. See [`oag_race::sight::Sight::in_span`].
    pub span: Option<[f32; 2]>,
}

impl Default for Sighting {
    /// The PSP's screen and the Missile's law: what a hand-made `Setup` ran
    /// before this existed.
    fn default() -> Self {
        Self {
            lock_screen: oag_race::sight::SCREEN,
            view_screen: oag_race::sight::SCREEN,
            columns_screen: None,
            leach_law: false,
            span: None,
        }
    }
}

impl Sighting {
    /// A title's reticle, off its loaded HUD.
    #[cfg(feature = "view")]
    #[must_use]
    pub fn of(hud: &oag_hud::Assets) -> Self {
        let view = hud.sight_screen();
        Self {
            lock_screen: [hud.space.size.0, hud.space.size.1],
            view_screen: [view.0, view.1],
            columns_screen: hud
                .split_columns
                .as_ref()
                .map(|columns| [columns.view.0, columns.view.1]),
            leach_law: hud.art.sights.runs_leach_law(),
            span: span_of(hud.space.size),
        }
    }

    /// A title's reticle with no HUD loaded: the lock grid is the whole HUD
    /// space of `platform` (what [`Self::of`] reads off `hud.space`, which
    /// `load_hud` sets from the same platform) and the law is the title's
    /// art, as [`Self::of`] reads it; with no view, the view grid is the lock
    /// grid. What a build without the `view` feature loads.
    #[must_use]
    pub fn without_view(platform: oag_disc::Platform, art: &oag_title::hud::HudArt) -> Self {
        let size = oag_display::space::Space::of(platform).size;
        Self {
            lock_screen: [size.0, size.1],
            view_screen: [size.0, size.1],
            columns_screen: None,
            leach_law: art.sights.runs_leach_law(),
            span: span_of(size),
        }
    }

    /// The drawn reticle, in the view grid.
    #[must_use]
    pub fn view_sight(&self) -> Sight {
        let mut sight = Sight::new(self.view_screen);
        sight.set_leach_law(self.leach_law);
        self.spanned(sight)
    }

    /// One slot's simulation lock, in the lock grid.
    #[must_use]
    pub fn lock_sight(&self) -> Sight {
        let mut sight = Sight::new(self.lock_screen);
        sight.set_leach_law(self.leach_law);
        self.spanned(sight)
    }

    fn spanned(&self, sight: Sight) -> Sight {
        match self.span {
            Some(span) => sight.in_span(span),
            None => sight,
        }
    }
}

/// The PS2's span: its HUD grid is 640x448 and the original runs the reticle in
/// a 512x512 buffer (measured on PCSX2, `docs/ui/hud.md`). Keyed on the grid
/// because only the PS2 pressing of Pulse authors that one.
fn span_of(size: (f32, f32)) -> Option<[f32; 2]> {
    let ps2 = oag_display::space::Space::PS2.size;
    (size == ps2).then_some([512.0, 512.0])
}
