//! Each title's music choice: one `[music_profiles."<Title::name>"]` table holding MUSIC
//! SOURCE and the MUSIC page's switches.
//!
//! Keyed by title alone, not `(title, platform)` like the render profiles: MUSIC SOURCE's
//! own value picks a pressing, and the race playlist's saved place is already per title.
//! The folders themselves (`[audio] music_folders`), MUSIC ORDER and the volumes are one
//! value for every title.

use oag_sound::MusicSource;
use oag_sound::pool::SwitchesByTitle;
use oag_sound::settings::MusicProfile;

use super::Settings;
use super::render_profile::KNOWN_TITLES;

/// The keys `[audio]` held for every title at once.
const MOVED_FROM_AUDIO: [&str; 2] = ["music_source", "music_switches"];

/// Copies an old file's `[audio]` music choice into every known title's table and drops the
/// old keys. A per-title value already there wins, so a hand edit that puts an old key back
/// cannot overwrite one.
pub(super) fn migrate_music_profiles(table: &mut toml::Table) {
    if table
        .get("music_profiles")
        .is_some_and(|value| !value.is_table())
    {
        return;
    }
    let Some(audio) = table.get_mut("audio").and_then(toml::Value::as_table_mut) else {
        return;
    };
    let moved: Vec<(&str, toml::Value)> = MOVED_FROM_AUDIO
        .iter()
        .filter_map(|&key| audio.remove(key).map(|value| (key, value)))
        .collect();
    if moved.is_empty() {
        return;
    }
    let profiles = table
        .entry("music_profiles")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()))
        .as_table_mut()
        .expect("checked on the way in, and only ever inserted as a table");
    for &title in KNOWN_TITLES {
        let profile = profiles
            .entry(title.name)
            .or_insert_with(|| toml::Value::Table(toml::Table::new()));
        // Anything but a table is left for deserialising to report by name.
        let Some(profile) = profile.as_table_mut() else {
            continue;
        };
        for (key, value) in &moved {
            profile.entry(*key).or_insert_with(|| value.clone());
        }
    }
}

/// Gives every known title a table, so the file shows what it can hold.
pub(super) fn ensure_known_music_profiles(settings: &mut Settings) {
    for title in KNOWN_TITLES {
        settings
            .music_profiles
            .entry(title.name.to_string())
            .or_default();
    }
}

impl Settings {
    /// `title`'s music choice; the default with no title open or none saved.
    #[must_use]
    pub fn music_profile(&self, title: Option<&oag_title::Title>) -> MusicProfile {
        title
            .and_then(|title| self.music_profiles.get(title.name))
            .cloned()
            .unwrap_or_default()
    }

    /// Which pressing's soundtrack `title` plays.
    #[must_use]
    pub fn music_source(&self, title: Option<&oag_title::Title>) -> MusicSource {
        self.music_profile(title).music_source
    }

    /// The row for `title`'s music, made if the file had none.
    pub fn music_profile_mut(&mut self, title: &oag_title::Title) -> &mut MusicProfile {
        self.music_profiles
            .entry(title.name.to_string())
            .or_default()
    }

    /// Every title's MUSIC page switches, for the pool, which applies the booted title's.
    #[must_use]
    pub fn music_switches(&self) -> SwitchesByTitle {
        self.music_profiles
            .iter()
            .map(|(title, profile)| (title.clone(), profile.music_switches.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests;
