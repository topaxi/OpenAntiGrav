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

/// The TRACK TITLE value that means "the player's own tracks".
pub const CUSTOM: &str = "CUSTOM";

/// The directories' files, and what reading them said.
#[derive(Debug, Default)]
pub struct Scan {
    files: Vec<PathBuf>,
    listings: Vec<Listing>,
    task: Option<Task<Vec<Listing>>>,
}

impl Scan {
    /// Lists the files under `bases` now (directory reads only) and starts
    /// reading them.
    pub fn start(bases: &[PathBuf]) -> Self {
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
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Takes the result once the thread is done. `true` on the call that did.
    pub fn poll(&mut self) -> bool {
        if !self.task.as_ref().is_some_and(Task::is_finished) {
            return false;
        }
        self.finish();
        true
    }

    /// Waits for the thread, for a capture that has no frames to wait across.
    pub fn wait(&mut self) {
        self.finish();
    }

    fn finish(&mut self) {
        let Some(mut task) = self.task.take() else {
            return;
        };
        if let Some(Ok(listings)) = task.join() {
            self.listings = listings;
        } else {
            log::error!("the custom track scan did not finish; its tracks stay unchecked");
            self.files.clear();
        }
    }

    /// What TRACK TITLE calls the CUSTOM entry.
    #[must_use]
    pub fn title_label(&self, strings: &StringTable) -> String {
        strings
            .get("OAG_REMIX_CUSTOM")
            .unwrap_or(CUSTOM)
            .to_string()
    }

    /// The TRACK row's options. The stored value is the file's path, which is
    /// unique and known before the file is read.
    pub fn choices(&self, strings: &StringTable) -> Vec<Choice> {
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
    pub fn listing(&self, value_: &str) -> Option<&Listing> {
        self.listings.iter().find(|l| value(&l.path) == value_)
    }

    /// The file to race for a TRACK value, or why none can be.
    pub fn raceable(&self, value_: &str) -> Result<&Path, String> {
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

/// The most characters a row's value holds before it runs into the label beside
/// it on Pulse and Pure (1280 wide, about 29). Wipeout HD draws values in boxes
/// about 20 characters wide, and a longer one runs past the box's right edge over
/// nothing; the text stays whole and readable (measured in `--menu-page remix`).
const ROW_WIDTH: usize = 28;

/// The row: name, author when the file names one, and which format it is. The
/// name is cut with `..` to fit; the loader report has it whole.
fn label(listing: &Listing) -> String {
    let mut tail = format!(" ({})", listing.kind.name().to_uppercase());
    if let Some(author) = &listing.author {
        tail = format!(" BY {}{tail}", author.to_uppercase());
    }
    let room = ROW_WIDTH.saturating_sub(tail.chars().count()).max(8);
    let title = listing.title.to_uppercase();
    let name = if title.chars().count() > room {
        let cut: String = title.chars().take(room - 2).collect();
        format!("{}..", cut.trim_end_matches([' ', '(']))
    } else {
        title
    };
    format!("{name}{tail}")
}

/// What racing the file at `path` sets on the options a remix is built from:
/// exactly what `--unity-track` and `--track-file` set (`Pending::race_options`),
/// and clears what an earlier launch left (a disc circuit's name, a second
/// source for the craft), so that both routes reach the same race.
pub fn apply(options: &mut oag_raceplay::Options, path: &Path) {
    options.foreign_track = Some(path.to_path_buf());
    options.track = None;
    options.craft_source = None;
}

#[cfg(test)]
mod tests;
