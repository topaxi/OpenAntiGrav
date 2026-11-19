//! The profile's lifetime statistics, the half of [`Store`] the footer ticker quotes (the
//! original's table at `DAT_08b31774 + 0x11c`, `docs/ghidra/functions/psp-pulse-usa/race-box-screens.md`,
//! "The footer ticker's message choice").
//!
//! **Only what this build can count is counted**: laps and time per mode off each recorded race
//! ([`Store::record`]), the time spent in the menus, and the race songs heard, per track. Every
//! other slot reads `0`, which the ticker's own law treats as "no message". A row is per title,
//! as the other tables are, and persists in the same file by the same writer.

use std::collections::BTreeMap;

use oag_core::TickRate;
use oag_ui_screens::news::Statistics;
use serde::{Deserialize, Serialize};

use super::Store;

/// The ticker's `TKR_SONGS` slot.
const SONGS: u32 = 69;
/// `TKR_MENTIM`: time spent in the menus.
const MENU_TIME: u32 = 85;
/// `TKR_TOTTIM`: total play time.
const TOTAL_TIME: u32 = 5;
/// `TKR_TLC`: laps completed in races.
const LAPS: u32 = 2;
/// `TKR_FAVMUS`, a `Profile` member.
const FAVOURITE_MUSIC: u32 = 3;
/// Each mode's time slot, by [`oag_race::Mode::name`].
const MODE_TIME: [(&str, u32); 7] = [
    ("single_race", 6),
    ("time_trial", 7),
    ("zone", 8),
    ("tournament", 10),
    ("eliminator", 11),
    ("head_to_head", 12),
    ("speed_lap", 13),
];
/// The original copies a favourite's name into a 30-character field.
const NAME_FIELD: usize = 30;

/// One title's lifetime counters.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LifetimeRecord {
    /// [`super::Key::title`]'s spelling.
    pub title: String,
    /// Laps completed in recorded races.
    #[serde(default)]
    pub laps: u64,
    /// Race ticks per mode, by [`oag_race::Mode::name`].
    #[serde(default)]
    pub race_ticks: BTreeMap<String, u64>,
    /// Seconds in the front end.
    #[serde(default)]
    pub menu_seconds: f64,
    /// Plays per song, by the title the disc names it.
    #[serde(default)]
    pub songs: BTreeMap<String, u32>,
}

impl LifetimeRecord {
    fn total_seconds(&self) -> f64 {
        let ticks: u64 = self.race_ticks.values().sum();
        self.menu_seconds + ticks as f64 / f64::from(TickRate::DEFAULT.hz())
    }
}

impl Store {
    fn lifetime_row(&mut self, title: &str) -> &mut LifetimeRecord {
        let title = title.trim().to_ascii_lowercase();
        if !self.lifetime.iter().any(|row| row.title == title) {
            self.lifetime.push(LifetimeRecord {
                title: title.clone(),
                ..LifetimeRecord::default()
            });
            // Stable, for the same reason `Self::record` sorts its own table.
            self.lifetime.sort_by(|a, b| a.title.cmp(&b.title));
        }
        self.lifetime
            .iter_mut()
            .find(|row| row.title == title)
            .expect("present or just pushed")
    }

    /// One recorded race: `ticks` of `mode` (a [`oag_race::Mode::name`]) and `laps` completed.
    pub(super) fn count_race(&mut self, title: &str, mode: &str, ticks: u64, laps: u32) {
        let row = self.lifetime_row(title);
        row.laps += u64::from(laps);
        *row.race_ticks.entry(mode.to_string()).or_default() += ticks;
    }

    /// `seconds` spent in the front end.
    pub fn count_menu_seconds(&mut self, title: &str, seconds: f64) {
        self.lifetime_row(title).menu_seconds += seconds;
        self.lifetime_unsaved = true;
    }

    /// One race song started, as the disc names it.
    pub fn count_song(&mut self, title: &str, song: &str) {
        *self
            .lifetime_row(title)
            .songs
            .entry(song.to_string())
            .or_default() += 1;
        self.lifetime_unsaved = true;
    }

    /// Whether counters changed that no save has taken yet; clears the flag.
    pub fn take_lifetime_unsaved(&mut self) -> bool {
        std::mem::take(&mut self.lifetime_unsaved)
    }

    /// What `title`'s ticker quotes. A title never played reads as all zero.
    #[must_use]
    pub fn lifetime(&self, title: &str) -> Lifetime<'_> {
        let title = title.trim().to_ascii_lowercase();
        Lifetime(self.lifetime.iter().find(|row| row.title == title))
    }
}

/// [`Store::lifetime`]: the ticker's view of one title's counters.
#[derive(Debug)]
pub struct Lifetime<'a>(Option<&'a LifetimeRecord>);

impl Lifetime<'_> {
    /// A number that changes when a counted event does, for a caller that rebuilds the ticker's
    /// messages then and not every frame: songs and laps, not the seconds that tick on.
    #[must_use]
    pub fn events(&self) -> u64 {
        self.0.map_or(0, |row| {
            row.laps
                + row
                    .songs
                    .values()
                    .map(|plays| u64::from(*plays))
                    .sum::<u64>()
        })
    }
}

impl Statistics for Lifetime<'_> {
    fn general(&self, member: u32) -> f32 {
        let Some(row) = self.0 else {
            return 0.0;
        };
        let hz = f64::from(TickRate::DEFAULT.hz());
        let value = match member {
            LAPS => row.laps as f64,
            SONGS => row.songs.values().map(|plays| f64::from(*plays)).sum(),
            MENU_TIME => row.menu_seconds,
            TOTAL_TIME => row.total_seconds(),
            _ => MODE_TIME
                .iter()
                .find(|(_, slot)| *slot == member)
                .and_then(|(mode, _)| row.race_ticks.get(*mode))
                .map_or(0.0, |ticks| *ticks as f64 / hz),
        };
        value as f32
    }

    fn favourite(&self, member: u32) -> Option<String> {
        if member != FAVOURITE_MUSIC {
            return None;
        }
        let row = self.0?;
        // The first song with the most plays, which is how the original's strict `<` keeps one.
        let (song, _) = row.songs.iter().fold(
            None,
            |best: Option<(&String, u32)>, (song, plays)| match best {
                Some((_, most)) if most >= *plays => best,
                _ => Some((song, *plays)),
            },
        )?;
        Some(song.chars().take(NAME_FIELD).collect())
    }
}

#[cfg(test)]
mod tests;
