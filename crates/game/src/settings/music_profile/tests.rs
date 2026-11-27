//! Each title's music choice: kept apart across a save and load, filled from an old file's
//! one global value, and what the menus and the pool read.

use std::path::PathBuf;

use oag_sound::MusicSource;
use oag_ui::menu::Value;

use super::super::render_profile::KNOWN_TITLES;
use super::super::{Settings, load_from, menu_seeds, render};

const HD: &str = "Wipeout HD";
const PULSE: &str = "Wipeout Pulse";
const DISC: &str = "disc:wipeout-pure:psp";

/// A settings file of this test's own under the temp directory.
struct File(PathBuf);

impl File {
    fn new(name: &str, text: &str) -> Self {
        let dir = oag_testdata::scratch::dir(&format!("music-{name}"));
        std::fs::create_dir_all(&dir).expect("a temp directory");
        let path = dir.join("settings.toml");
        std::fs::write(&path, text).expect("write");
        Self(path)
    }

    fn load(&self) -> Settings {
        load_from(&self.0).expect("loads")
    }

    fn table(&self) -> toml::Table {
        std::fs::read_to_string(&self.0)
            .expect("read")
            .parse()
            .expect("parses")
    }
}

impl Drop for File {
    fn drop(&mut self) {
        if let Some(dir) = self.0.parent() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

#[test]
fn two_titles_keep_their_own_choice_across_a_save_and_load() {
    let file = File::new("two-titles", "");
    let mut settings = file.load();
    let hd = settings.music_profile_mut(oag_hd::TITLE);
    hd.music_source = MusicSource::Ps2;
    hd.music_switches.insert(DISC.into(), true);
    std::fs::write(&file.0, render(&settings).expect("serialises")).expect("write");

    let again = file.load();
    let hd = again.music_profile(Some(oag_hd::TITLE));
    assert_eq!(hd.music_source, MusicSource::Ps2);
    assert_eq!(hd.music_switches.get(DISC), Some(&true));
    let pulse = again.music_profile(Some(oag_pulse::TITLE));
    assert_eq!(pulse.music_source, MusicSource::Auto);
    assert!(pulse.music_switches.is_empty(), "{pulse:?}");
    assert_eq!(again.music_switches()[HD].get(DISC), Some(&true));
    assert!(again.music_switches()[PULSE].is_empty());
}

#[test]
fn an_old_global_choice_is_copied_to_every_title_and_the_old_keys_go() {
    let file = File::new(
        "old-global",
        "[audio]\nmusic_volume = 60\nmusic_source = \"ps2\"\n\n\
         [audio.music_switches]\n\"disc:wipeout-pure:psp\" = true\n\"folder:/songs\" = false\n",
    );
    let settings = file.load();
    for title in KNOWN_TITLES {
        let profile = settings.music_profile(Some(title));
        assert_eq!(profile.music_source, MusicSource::Ps2, "{}", title.name);
        assert_eq!(
            profile.music_switches.get(DISC),
            Some(&true),
            "{}",
            title.name
        );
        assert_eq!(
            profile.music_switches.get("folder:/songs"),
            Some(&false),
            "{}",
            title.name
        );
    }
    assert_eq!(settings.audio.music_volume.percent(), 60, "the rest stays");
    let written = file.table();
    let audio = written["audio"].as_table().expect("[audio]");
    assert!(!audio.contains_key("music_source"), "{audio}");
    assert!(!audio.contains_key("music_switches"), "{audio}");
    assert_eq!(
        written["music_profiles"][HD]["music_source"].as_str(),
        Some("ps2")
    );
}

#[test]
fn a_titles_own_value_wins_over_a_global_one_put_back() {
    let file = File::new(
        "own-wins",
        "[audio]\nmusic_source = \"ps2\"\n\n\
         [music_profiles.\"Wipeout HD\"]\nmusic_source = \"psp\"\n",
    );
    let settings = file.load();
    assert_eq!(settings.music_source(Some(oag_hd::TITLE)), MusicSource::Psp);
    assert_eq!(
        settings.music_source(Some(oag_pulse::TITLE)),
        MusicSource::Ps2
    );
}

#[test]
fn the_menus_seed_music_source_from_the_open_title() {
    let mut settings = Settings::default();
    settings.music_profile_mut(oag_pulse::TITLE).music_source = MusicSource::Ps2;
    let seeded = |title| {
        menu_seeds(
            &settings,
            Default::default(),
            title,
            oag_disc::Platform::Psp,
        )
        .into_iter()
        .find(|(setting, _)| *setting == "audio.music_source")
        .expect("seeded")
        .1
    };
    assert_eq!(seeded(oag_pulse::TITLE), Value::Text("ps2".to_string()));
    assert_eq!(seeded(oag_hd::TITLE), Value::Text("auto".to_string()));
}

#[test]
fn a_fresh_file_has_a_row_for_every_title() {
    let settings = super::super::fresh();
    for title in KNOWN_TITLES {
        assert!(
            settings.music_profiles.contains_key(title.name),
            "{}",
            title.name
        );
    }
}
