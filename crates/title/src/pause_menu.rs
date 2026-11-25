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
    /// The on/off list for the ghost ship (`FE_GHOST`): `display.ghost_ship_visible`; on a row
    /// that [`PauseRow::opens`] a screen, the tile that opens it.
    Ghost,
    /// A touch list of which ghost to race (2048's `GhostOption2048`): NONE and MY BEST are
    /// `display.ghost_ship_visible` off and on, and the entries past them (a friend's and the
    /// world's, which need the online service) are drawn and cannot be chosen.
    GhostList,
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
    /// The `idstring` the disc spells; for an icon-only widget that has none (2048's `quitButton`),
    /// the widget's `name`.
    pub disc_id: &'static str,
    /// The disc string the row is worded by: [`Self::disc_id`], or for a widget with no
    /// `idstring` the id of a string the page labels it with.
    pub word: &'static str,
    /// What the row does here, or `None` where this build has nothing behind it.
    pub builds: Option<PauseAction>,
    /// The screen of [`PauseMenu::screens`] the row opens as a page of its own: for a QUIT or
    /// RESTART row the "are you sure" it asks on before it acts (2048's `PauseQuitConfirm`), for
    /// a [`PauseAction::Ghost`] tile the list of ghosts. `None` for a row that acts at once.
    pub opens: Option<&'static str>,
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
    /// How a screen of `<TouchButton>` tiles takes its colours. `None` for a title whose screens
    /// are `Block`, `List` or `Menu` widgets.
    pub touch: Option<PauseTouch>,
}

/// The skin globals a touch pause screen's tiles are filled with: Wipeout 2048's tile is its
/// `Blue2048` box and the pad's cursor fills the tile the pointer would be pressing in
/// `Orange2048` (`oag_ui::frontend::Frontend::draw_tile`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PauseTouch {
    /// The global the resting tile and the heading's colour come from.
    pub tile: &'static str,
    /// The global the tile with the pad cursor is filled with.
    pub cursor: &'static str,
    /// The global a list entry that cannot be chosen is filled with (`Grey2048`, the palette's
    /// inactive colour).
    pub inactive: &'static str,
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
    /// Whether the race's song-name line stays on screen under the tint. Pure's pause screens are
    /// siblings of `InGame`, whose widgets carry the line, and its original frames show none.
    pub song_line_under_scrim: bool,
    /// Whether a screen with no photo row of its own gets this project's PHOTO MODE row
    /// after OPTIONS. Photo mode is one implementation on every title.
    pub offers_photo: bool,
    /// The music list the screens draw under their rows, where the title authors one (Pulse's
    /// `MusicList`). `None` draws none.
    pub music_list: Option<PauseMusicList>,
    /// [`Self::music_list`] on the PS2 pressing, whose screens author it in their own grid and
    /// face. `None` takes [`Self::music_list`].
    pub music_list_ps2: Option<PauseMusicList>,
    /// Where [`Self::screens`] and [`Self::by_mode`] were read.
    pub origin: Origin,
}

/// A pause screen's music list: the race playlist's song names, one shown, stepped by LEFT and
/// RIGHT whichever row the cursor is on; the step plays nothing until the race resumes, which
/// starts the song shown from its beginning (`docs/ghidra/functions/psp-pulse-usa/music-list.md`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PauseMusicList {
    /// The text's top in the screen's grid (the widget's authored `y`); its left edge is the
    /// menu rows' own.
    pub y: f32,
    /// The face the text is drawn in, as the widget names it: `small`, or `menu` (the rows' own).
    pub font: &'static str,
    /// Whether a step past either end goes round to the other (the widget's `Wrap`).
    pub wraps: bool,
    /// The sheet the two arrows are cut from.
    pub sheet: &'static str,
    /// The left and right arrow in the sheet, `[u, v, w, h]`.
    pub arrows: [[f32; 4]; 2],
    /// Each arrow's top-left from the text's left edge and top, left then right.
    pub arrow_offset: [(f32, f32); 2],
    /// The share of its alpha an arrow half keeps while the selection is at the end it points
    /// to, whether or not the list wraps.
    pub arrow_dim: f32,
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

    /// The music list the pressing for `platform` draws.
    #[must_use]
    pub fn music_list_for(&self, platform: crate::Platform) -> Option<PauseMusicList> {
        match platform {
            crate::Platform::Ps2 => self.music_list_ps2.or(self.music_list),
            _ => self.music_list,
        }
    }
}

/// A [`PauseRow`], for a title package's table.
#[must_use]
pub const fn row(disc_id: &'static str, builds: Option<PauseAction>) -> PauseRow {
    PauseRow {
        disc_id,
        word: disc_id,
        builds,
        opens: None,
    }
}

/// A [`PauseRow`] for an icon-only widget the disc names by `widget` and words by `word`.
#[must_use]
pub const fn named(
    widget: &'static str,
    word: &'static str,
    builds: Option<PauseAction>,
) -> PauseRow {
    PauseRow {
        disc_id: widget,
        word,
        builds,
        opens: None,
    }
}

impl PauseRow {
    /// This row opening `screen` as a page of its own.
    #[must_use]
    pub const fn opening(self, screen: &'static str) -> Self {
        Self {
            opens: Some(screen),
            ..self
        }
    }
}
