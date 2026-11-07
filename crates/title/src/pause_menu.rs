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
    /// The on/off list for Pilot Assist (`FE_PILOT_ASSIST`): `controls.pilot_assist`, on being
    /// the level HD flies with.
    PilotAssist,
    /// The on/off list for the ghost ship (`FE_GHOST`): `display.ghost_ship_visible`.
    Ghost,
    /// Drawn dimmed and skipped by the cursor, doing nothing (`ONL_VIEW_INV`: PSN's invites,
    /// which the original greys offline).
    Inert,
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

/// Where a title's pause screens are authored and what its draw reads off them: the file, the
/// shared chrome screen both are authored in, and the grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PauseLayout {
    /// The archive path of the definition file the screens are in.
    pub definition: &'static str,
    /// The screen of that file whose rules and footer every pause screen draws under its own
    /// widgets (HD's `InGame Main Menu`: two rules, the START glyph and `IG_PAUSE_RESU`,
    /// CONFIRM). `None` where each screen authors its own band (Pure's).
    pub chrome: Option<&'static str>,
    /// The grid the file is authored in.
    pub grid: [u32; 2],
    /// How a screen that authors one `<Menu>` of entries, rather than a widget per row, lays
    /// its rows out. `None` for a title whose screens are `Block` and `List` widgets.
    pub menu_rows: Option<PauseMenuRows>,
}

/// The two numbers a `<Menu>` widget does not author: how far apart its rows stand and how
/// bright a row without the cursor is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PauseMenuRows {
    /// The distance between two rows at the widget's `scale` of `1.0`, in the file's grid.
    pub pitch: f32,
    /// The alpha a row without the cursor draws its label with, the focused one drawing `1.0`.
    pub unfocused_alpha: f32,
}

/// A title's pause screens and which game mode opens which.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PauseMenu {
    /// The `idstring` of the heading the screen draws (`IG_PAUSE_GPAUSED`).
    pub heading: &'static str,
    /// Every screen the title authors for a race it can pause.
    pub screens: &'static [PauseScreen],
    /// The screen a mode that is not in [`Self::by_mode`] opens: the one the disc's pause
    /// redirect names as its default.
    pub default_screen: &'static str,
    /// `(mode name, screen name)`: the modes whose pause is not the default. A mode's name
    /// is `oag_race::Mode::name`, or `free_play` for a Free Play event.
    pub by_mode: &'static [(&'static str, &'static str)],
    /// The tint the screen draws over the race, as the `ARGB` its full-screen image authors
    /// (`InGame Pause`'s `0xe8000000`).
    pub scrim_argb: u32,
    /// Where the screens' geometry is read from, for a title that draws them as the disc authors
    /// them. `None` draws the page as the front end's own list.
    pub layout: Option<PauseLayout>,
    /// Whether the race's HUD stays on screen under the tint: the original's does, dimmed with
    /// the rest of the picture.
    pub hud_under_scrim: bool,
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
