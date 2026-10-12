//! RACE REMIX's CUSTOM track title: the tracks a player has put in the
//! directories `--unity-track` searches, offered on the TRACK row.
//!
//! The list is the one [`oag_source::unity::tracks_in`] gives the command line,
//! so a name that works for `--unity-track` is a row here and the reverse. Each
//! file is read the way a race reads it ([`oag_raceplay::foreign::survey`]), on a
//! thread of its own: 32 Workshop tracks take about 20 s to read in a debug build,
//! more than a menu should hold the window for. Until that finishes the rows are
//! there, by file name, and cannot be started.
//!
//! A file that will not load is listed anyway, dimmed, with the reason in the
//! note slot under the rows ([`oag_ui::menu::Choice::unusable`]). It is never
//! hidden and never raced instead of another.

use std::path::{Path, PathBuf};

use oag_raceplay::foreign::survey::{self, Listing};
use oag_thread::Task;
use oag_ui::language::StringTable;
use oag_ui::menu::Choice;

use super::Session;
use super::remix_menu;

/// The TRACK TITLE value that means "the player's own tracks".
pub(crate) const CUSTOM: &str = "CUSTOM";

/// The directories' files, and what reading them said.
#[derive(Default)]
pub(crate) struct Scan {
    files: Vec<PathBuf>,
    listings: Vec<Listing>,
    task: Option<Task<Vec<Listing>>>,
}

impl Scan {
    /// Lists the files under `bases` now (directory reads only) and starts
    /// reading them.
    pub(crate) fn start(bases: &[PathBuf]) -> Self {
        let files = oag_source::unity::tracks_in(bases);
        let task = (!files.is_empty()).then(|| {
            let files = files.clone();
            Task::spawn("oag-custom-tracks", move || survey::survey(&files))
        });
        Self {
            files,
            listings: Vec::new(),
            task,
        }
    }

    /// Whether the scan found no file at all, so CUSTOM is not offered.
    pub(crate) fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Takes the result once the thread is done. `true` on the call that did.
    pub(crate) fn poll(&mut self) -> bool {
        if !self.task.as_ref().is_some_and(Task::is_finished) {
            return false;
        }
        if let Some(Ok(listings)) = self.task.take().and_then(|mut task| task.join()) {
            self.listings = listings;
        } else {
            log::error!("the custom track scan did not finish; its tracks stay unchecked");
            self.files.clear();
        }
        true
    }

    /// The TRACK row's options. The stored value is the file's path, which is
    /// unique and known before the file is read.
    pub(crate) fn choices(&self, strings: &StringTable) -> Vec<Choice> {
        if self.task.is_some() {
            let reason = strings
                .get("OAG_REMIX_CUSTOM_CHECKING")
                .unwrap_or("still checking this file");
            return self
                .files
                .iter()
                .map(|path| Choice::labelled(value(path), stem(path)).unusable(reason))
                .collect();
        }
        self.listings
            .iter()
            .map(|listing| {
                let choice = Choice::labelled(value(&listing.path), label(listing));
                match &listing.problem {
                    Some(problem) => choice.unusable(problem.clone()),
                    None => choice,
                }
            })
            .collect()
    }

    /// The listing a TRACK value stands for, once the scan has finished.
    pub(crate) fn listing(&self, value_: &str) -> Option<&Listing> {
        self.listings.iter().find(|l| value(&l.path) == value_)
    }

    /// The file to race for a TRACK value, or why none can be.
    pub(crate) fn raceable(&self, value_: &str) -> Result<&Path, String> {
        match self.listing(value_) {
            None if self.task.is_some() => Err("still checking the files".to_string()),
            None => Err(format!("{value_} is not a track this scan found")),
            Some(listing) => match &listing.problem {
                Some(problem) => Err(format!("{}: {problem}", listing.title)),
                None => Ok(&listing.path),
            },
        }
    }
}

fn value(path: &Path) -> String {
    path.display().to_string()
}

fn stem(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_uppercase())
        .unwrap_or_default()
}

/// The row: name, author when the file names one, and which format it is.
fn label(listing: &Listing) -> String {
    let mut text = listing.title.to_uppercase();
    if let Some(author) = &listing.author {
        text.push_str(&format!(" BY {}", author.to_uppercase()));
    }
    text.push_str(&format!(" ({})", listing.kind.name().to_uppercase()));
    text
}

/// What racing the file at `path` sets on the options a remix is built from:
/// exactly what `--unity-track` and `--track-file` set (`Pending::race_options`),
/// and clears what an earlier launch left (a disc circuit's name, a second
/// source for the craft), so that both routes reach the same race.
pub(crate) fn apply(options: &mut oag_raceplay::Options, path: &Path) {
    options.foreign_track = Some(path.to_path_buf());
    options.track = None;
    options.craft_source = None;
}

impl Session {
    /// CUSTOM, as TRACK TITLE offers it - none when the scan found no file.
    pub(super) fn custom_title_choice(&self) -> Option<Choice> {
        let strings = oag_ui::strings::project_table(self.settings.language.as_deref());
        (!self.custom_tracks.is_empty()).then(|| {
            Choice::labelled(
                CUSTOM,
                strings.get("OAG_REMIX_CUSTOM").unwrap_or(CUSTOM).to_string(),
            )
        })
    }

    /// The TRACK row's options for whatever `remix.track_title` names: the
    /// scan's for CUSTOM, a title's own circuits otherwise. `None` when the
    /// title's source will not open, which supplies and settles nothing.
    pub(super) fn remix_track_options(&self) -> Option<Vec<Choice>> {
        let title = &self.settings.remix.track_title;
        if title == CUSTOM {
            let strings = oag_ui::strings::project_table(self.settings.language.as_deref());
            return Some(self.custom_tracks.choices(&strings));
        }
        let catalogue = self.remix_catalogue_for(title)?;
        Some(remix_menu::remix_track_choices(&catalogue))
    }

    /// Called once a frame: when the scan has just finished and the TRACK row
    /// is showing it, hands the row its checked list.
    pub(crate) fn poll_custom_tracks(&mut self) {
        if self.custom_tracks.poll() && self.settings.remix.track_title == CUSTOM {
            self.resupply_remix_tracks();
        }
    }
}

#[cfg(test)]
#[path = "custom_tracks/tests.rs"]
mod tests;
