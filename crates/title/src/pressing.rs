//! A title's pressings: what its executable resolves differently per release,
//! keyed by the disc's serial. [ADR-0058]'s third axis.
//!
//! Wipeout Pure's two PSP pressings bake different literals into their own
//! executables: the title screen's wordmark texture and the suffix a
//! `localised="true"` boot movie resolves to (`_EU`/`_US`). The names the
//! boot code asked for were `oag_pure::frontend::*` functions behind a
//! `title.name == "Wipeout Pure"` test in `oag-game`; here the table is data
//! on the title, and a title with none (`None` on [`crate::Title::pressings`])
//! resolves nothing per pressing.
//!
//! A serial this table does not list takes [`Pressings::unlisted`], the EU
//! row: this project's own "prefer EU over USA" convention for a source that
//! cannot say which pressing it is (an extracted directory carries no serial).
//!
//! [ADR-0058]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0058-per-title-behaviour-is-title-data-with-provenance.md

use crate::effects::Origin;

/// What one pressing's executable resolves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pressing {
    /// The disc's own normalised `AAAA-NNNNN` serial, or `""` on the row that
    /// answers for every unlisted serial.
    pub serial: &'static str,
    /// The region a `localised="true"` widget resolves with
    /// (`oag_ui::screen::Movie::entry_name`).
    pub movie_region: &'static str,
    /// The `TitleFrame` widget's `(id, src)` this pressing bakes in.
    pub title_frame: (&'static str, &'static str),
    /// The first boot movie's entry name on this pressing.
    pub intro_movie: &'static str,
    /// The second boot movie's entry name on this pressing.
    pub fmv_intro_movie: &'static str,
}

/// A title's per-pressing table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pressings {
    /// The pressings this build has measured, by serial.
    pub listed: &'static [Pressing],
    /// What every other serial, and a source with none, gets.
    pub unlisted: Pressing,
    /// Where the table came from.
    pub origin: Origin,
}

impl Pressings {
    /// The row for `serial`: its own if listed, [`Self::unlisted`] otherwise.
    #[must_use]
    pub fn of(&self, serial: Option<&str>) -> &Pressing {
        serial
            .and_then(|serial| self.listed.iter().find(|row| row.serial == serial))
            .unwrap_or(&self.unlisted)
    }
}
