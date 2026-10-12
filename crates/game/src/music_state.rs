//! Where the race playlist stood, kept between runs: `music.toml` beside
//! `settings.toml` and `records.toml`.
//!
//! **Chosen, not measured.** The original's race music is not recovered, so this
//! persists this engine's own playlist (a plain cycle of the booted soundtrack,
//! see `oag_sound::RaceMusicState`): which track and how many whole seconds into
//! it, per title. Progress persists on its own, with no save slot, like
//! `records.toml`.
//!
//! A file of its own and not a section of `records.toml`: a periodic save
//! while racing would otherwise rewrite every best lap each time. The write is
//! atomic (temp file, then rename) so a crash mid-save leaves the previous file.
//!
//! A saved place that no longer fits (a track count or length that changed, a
//! disc without a soundtrack) is dropped with a logged reason and the playlist
//! starts where it normally does. Headless captures call none of this, the way
//! they never call `records::save`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use log::{info, warn};
use oag_sound::{Audio, MusicDiscs, RaceMusicState};
use serde::{Deserialize, Serialize};

/// The schema kind this file is written as (`schemas/v1/music-state.tosd`).
pub const KIND: &str = "music-state";

/// How often a race saves its place while it plays, in seconds. **Chosen, not
/// measured**: short enough that a crash costs under a quarter of a minute of
/// music, long enough that a write (a few hundred bytes) is never frequent.
pub const SAVE_EVERY_SECONDS: u32 = 10;

#[derive(Debug, Default, Serialize, Deserialize)]
struct File {
    #[serde(rename = "toml-schema", skip_serializing_if = "Option::is_none")]
    schema: Option<oag_schema::Header>,
    #[serde(default)]
    playlist: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Entry {
    title: String,
    index: usize,
    second: u32,
    tracks: usize,
    track_seconds: f64,
}

/// `music.toml` in the config directory, or [`None`] with no config directory.
#[must_use]
pub fn path() -> Option<PathBuf> {
    crate::profile::config_dir().map(|dir| dir.join("oag").join("music.toml"))
}

fn read(path: &Path) -> File {
    let text = match crate::profile::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return File::default(),
        Err(e) => {
            warn!(
                "could not read {}: {e:#}; no saved music place",
                path.display()
            );
            return File::default();
        }
    };
    let parsed = toml::from_str::<File>(&text)
        .map_err(anyhow::Error::from)
        .and_then(|file| {
            oag_schema::check(file.schema.as_ref(), KIND, 1)?;
            Ok(file)
        });
    parsed.unwrap_or_else(|e| {
        warn!(
            "{} is unusable ({e:#}); no saved music place",
            path.display()
        );
        File::default()
    })
}

/// The place saved for `title` in the file at `path`.
#[must_use]
pub fn load_from(path: &Path, title: &str) -> Option<RaceMusicState> {
    read(path)
        .playlist
        .into_iter()
        .find(|entry| entry.title == title)
        .map(|entry| RaceMusicState {
            index: entry.index,
            second: entry.second,
            tracks: entry.tracks,
            track_seconds: entry.track_seconds,
        })
}

/// Writes `state` as `title`'s place in the file at `path`, keeping every other
/// title's, through a temp file and a rename.
///
/// # Errors
/// A directory or file that cannot be written.
pub fn store_to(path: &Path, title: &str, state: &RaceMusicState) -> Result<()> {
    let mut file = read(path);
    file.playlist.retain(|entry| entry.title != title);
    file.playlist.push(Entry {
        title: title.to_owned(),
        index: state.index,
        second: state.second,
        tracks: state.tracks,
        track_seconds: state.track_seconds,
    });
    file.playlist.sort_by(|a, b| a.title.cmp(&b.title));
    file.schema = None;
    let text = format!(
        "# OpenAntiGrav race music place: which soundtrack track a race was on and how far\n\
         # into it, per title. This project's own file, rewritten as you race.\n\
         {}\n\n{}",
        oag_schema::inline(KIND, 1),
        toml::to_string_pretty(&file).context("serialising the music place")?
    );
    if let Some(parent) = path.parent() {
        crate::profile::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let temp = path.with_extension("toml.tmp");
    crate::profile::write(&temp, text).with_context(|| format!("writing {}", temp.display()))?;
    crate::profile::rename(&temp, path).with_context(|| format!("replacing {}", path.display()))
}

/// Before a run's first race: puts the saved place for `title` back into `audio`
/// so the race resumes there. Logs what it did, or why it fell back.
pub fn resume(audio: &mut Audio, discs: &MusicDiscs, title: &str) {
    let Some(path) = path() else { return };
    resume_from(&path, audio, discs, title);
}

/// [`resume`] against an explicit file.
pub fn resume_from(path: &Path, audio: &mut Audio, discs: &MusicDiscs, title: &str) {
    let Some(saved) = load_from(path, title) else {
        return;
    };
    match audio.restore_race_music(discs, &saved) {
        Ok(()) => info!(
            "audio: race music resumes at track {} and {} s, as saved",
            saved.index, saved.second
        ),
        Err(why) => warn!(
            "audio: not resuming the saved race music ({why}); the playlist starts at its normal place"
        ),
    }
}

/// What a session remembers of its last write, so an unchanged place is not
/// written again.
#[derive(Debug, Default)]
pub struct Saver {
    last: Option<RaceMusicState>,
}

impl Saver {
    /// Writes `audio`'s place for `title` when it moved since the last write.
    /// A failed write is logged and carried on from, like `records::save`.
    pub fn save(&mut self, audio: &Audio, title: &str) {
        let Some(path) = path() else { return };
        self.save_to(&path, audio, title);
    }

    /// [`Self::save`] against an explicit file.
    pub fn save_to(&mut self, path: &Path, audio: &Audio, title: &str) {
        let Some(state) = audio.race_music_state() else {
            return;
        };
        if self.last == Some(state) {
            return;
        }
        match store_to(path, title, &state) {
            Ok(()) => self.last = Some(state),
            Err(e) => warn!("could not save the race music place: {e:#}"),
        }
    }
}

#[cfg(test)]
mod tests;
