//! A title's own pause screens, as its disc authors them: which rows each game mode's
//! screen has, and which of them this build has something behind. Per-title behaviour as
//! data ([ADR-0058]), shaped like [`crate::main_menu::MainMenu`].
//!
//! The rows are **read off the disc** (Pulse: `InGame_Definition.xml`'s `InGame Pause SP`
//! and its Time Trial, Speed Lap and Tournament variants), by the `idstring` the file
//! spells, in the file's order. The ground-truth test that asserts a list reads the same
//! file back and compares the whole list, so a row dropped or reordered fails by name.
//!
//! [`PauseRow::builds`] is the one thing that is ours: what a row does here. `None` means
//! this build has nothing behind it (the game manual, the session-ending rows) and the row
//! is not drawn; it stays in the list so the list is the disc's, and the report can say
//! what is missing. Nothing is drawn as an inert stand-in.
//!
//! [ADR-0058]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0058-per-title-behaviour-is-title-data-with-provenance.md

use crate::effects::Origin;

/// What a pause row does in this build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseAction {
    /// Back to the race where it stopped.
    Resume,
    /// The front end's OPTIONS page, over the parked race.
    Options,
    /// The front end's AUDIO page, over the parked race.
    Audio,
    /// Photo mode ([`oag_ui`'s `Action::PhotoMode`]).
    Photo,
    /// Starts the race again from the grid.
    Restart,
    /// Leaves the race: the front end, or the campaign's cell for a campaign race.
    Quit,
}

/// One row of a pause screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PauseRow {
    /// The `idstring` the disc spells.
    pub disc_id: &'static str,
    /// What the row does here, or `None` where this build has nothing behind it.
    pub builds: Option<PauseAction>,
}

/// One authored pause screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PauseScreen {
    /// The screen's `name` in the definition file.
    pub name: &'static str,
    /// The rows, in the disc's order.
    pub rows: &'static [PauseRow],
}

/// A title's pause screens and which game mode opens which.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PauseMenu {
    /// The `idstring` of the heading the screen draws (`IG_PAUSE_GPAUSED`).
    pub heading: &'static str,
    /// Every screen the title authors for a race it can pause.
    pub screens: &'static [PauseScreen],
    /// The screen a mode that is not in [`Self::by_mode`] opens: the one the disc's pause
    /// redirect names as its default.
    pub default_screen: &'static str,
    /// `(mode name, screen name)`: the modes whose pause is not the default. A mode's name
    /// is `oag_race::Mode::name`.
    pub by_mode: &'static [(&'static str, &'static str)],
    /// The tint the screen draws over the race, as the `ARGB` its full-screen image authors
    /// (`InGame Pause`'s `0xe8000000`).
    pub scrim_argb: u32,
    /// Whether a screen with no photo row of its own gets this project's PHOTO MODE row
    /// after OPTIONS. Photo mode is one implementation on every title.
    pub offers_photo: bool,
    /// Where [`Self::screens`] and [`Self::by_mode`] were read.
    pub origin: Origin,
}

impl PauseMenu {
    /// The screen the mode named `mode` pauses into.
    #[must_use]
    pub fn screen_for(&self, mode: &str) -> &'static PauseScreen {
        let name = self
            .by_mode
            .iter()
            .find_map(|(name, screen)| name.eq_ignore_ascii_case(mode).then_some(*screen))
            .unwrap_or(self.default_screen);
        self.screen(name)
            .or_else(|| self.screen(self.default_screen))
            .unwrap_or(&self.screens[0])
    }

    /// The screen called `name`.
    #[must_use]
    pub fn screen(&self, name: &str) -> Option<&'static PauseScreen> {
        self.screens.iter().find(|screen| screen.name == name)
    }
}

/// A [`PauseRow`], for a title package's table.
#[must_use]
pub const fn row(disc_id: &'static str, builds: Option<PauseAction>) -> PauseRow {
    PauseRow { disc_id, builds }
}
