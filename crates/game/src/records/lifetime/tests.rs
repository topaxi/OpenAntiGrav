//! The lifetime counters and the ticker they feed, through the pure functions only (no config
//! directory is touched).

use std::collections::HashMap;

use oag_ui::language::StringTable;
use oag_ui_screens::campaign::footer::TickerLayout;
use oag_ui_screens::news::{self, Feed, Statistics};

use super::*;
use crate::records::{Key, Observation, parse, ticker_tips};

fn race(store: &mut Store, mode: &str, ticks: u64, laps: u32) {
    store.record(
        Key::new("pulse", Some("16_track"), mode, "venom"),
        Observation {
            finished: true,
            place: Some(1),
            laps_completed: laps,
            tick: ticks,
            best_lap_ticks: None,
            campaign_medal: None,
            campaign_difficulty: None,
        },
    );
}

#[test]
fn a_recorded_race_adds_its_laps_and_its_time_to_its_mode() {
    let mut store = Store::default();
    race(&mut store, "speed_lap", 3600, 2);
    race(&mut store, "speed_lap", 1800, 1);
    race(&mut store, "zone", 600, 0);
    let view = store.lifetime("Pulse");
    assert_eq!(view.general(2), 3.0, "laps");
    assert_eq!(view.general(13), 90.0, "speed lap seconds");
    assert_eq!(view.general(8), 10.0, "zone seconds");
    assert_eq!(view.general(6), 0.0, "a mode never raced");
    assert_eq!(
        view.general(5),
        100.0,
        "total time is every race and the menus"
    );
    store.count_menu_seconds("pulse", 25.0);
    assert_eq!(store.lifetime("pulse").general(5), 125.0);
    assert_eq!(store.lifetime("pulse").general(85), 25.0);
    assert_eq!(
        store.lifetime("pure").general(2),
        0.0,
        "another title is untouched"
    );
}

#[test]
fn the_favourite_song_is_the_most_played_and_the_first_on_a_tie() {
    let mut store = Store::default();
    assert_eq!(store.lifetime("pulse").favourite(3), None);
    for song in ["Zeta", "Alpha", "Alpha", "Zeta"] {
        store.count_song("pulse", song);
    }
    assert_eq!(store.lifetime("pulse").general(69), 4.0);
    assert_eq!(
        store.lifetime("pulse").favourite(3).as_deref(),
        Some("Alpha")
    );
    store.count_song("pulse", "Zeta");
    assert_eq!(
        store.lifetime("pulse").favourite(3).as_deref(),
        Some("Zeta")
    );
    assert_eq!(
        store.lifetime("pulse").favourite(0),
        None,
        "only member 3 is the music"
    );
    store.count_song(
        "pulse",
        "A track whose name is longer than thirty characters",
    );
    store.count_song(
        "pulse",
        "A track whose name is longer than thirty characters",
    );
    store.count_song(
        "pulse",
        "A track whose name is longer than thirty characters",
    );
    assert_eq!(
        store
            .lifetime("pulse")
            .favourite(3)
            .unwrap()
            .chars()
            .count(),
        30,
        "the original copies the name into a 30 character field"
    );
}

#[test]
fn the_counters_survive_the_file_and_an_old_file_has_none() {
    let mut store = Store::default();
    race(&mut store, "single_race", 7200, 3);
    store.count_song("pulse", "Fenix Funk 5");
    store.count_menu_seconds("pulse", 61.5);
    let text = toml::to_string_pretty(&store).expect("serialises");
    let (back, notes) = parse(&text).expect("parses");
    assert!(notes.is_empty(), "{notes:?}");
    assert_eq!(back.lifetime("pulse").general(6), 120.0);
    assert_eq!(back.lifetime("pulse").general(69), 1.0);
    assert_eq!(back.lifetime("pulse").general(85), 61.5);
    let (old, notes) = parse("xp = 4\n").expect("parses");
    assert!(notes.is_empty());
    assert_eq!(old.lifetime("pulse").events(), 0);
}

#[test]
fn a_song_or_menu_time_marks_the_counters_unsaved_until_taken() {
    let mut store = Store::default();
    assert!(!store.take_lifetime_unsaved());
    store.count_menu_seconds("pulse", 1.0);
    assert!(store.take_lifetime_unsaved());
    assert!(!store.take_lifetime_unsaved());
}

fn layout(xml_items: &str) -> TickerLayout {
    let skin = r#"<Screen><Viewport name="TextInfoIsAlwaysLast" OffsetX="85" OffsetY="235">
        <Values width="370" height="32"></Values>
        <TextInfo><Values type="bar" x="0" y="0" scale="1.0"></Values></TextInfo></Viewport></Screen>"#;
    let mut layout = TickerLayout::read(&oag_ui::screen::parse(skin), &HashMap::new())
        .expect("the viewport is authored");
    layout.news = Feed {
        title: "pulse".to_string(),
        items: news::parse(xml_items),
    };
    layout
}

/// What the ticker plays is the profile's own counters through the disc's own items: drop the
/// store from the call and the song name is not in the rotation.
#[test]
fn the_ticker_plays_the_profiles_favourite_song() {
    let items = r#"<Screen>
        <PI_News><Values NewsID="TKR_FAVMUS" NewsClass="Profile" NewsMember="3" NewsFrequency="0"></Values></PI_News>
        <PI_News><Values NewsID="TKR_SONGS" NewsFormat="int" NewsClass="GenStats" NewsMember="69" NewsFrequency="0"></Values></PI_News>
        </Screen>"#;
    let strings = StringTable::from_xml(
        r#"<Entry ID="TKR_FAVMUS" String="Your most listened to music track is %s"></Entry>
           <Entry ID="TKR_SONGS" String="You've listened to a total of %d songs"></Entry>"#,
    );
    let layout = layout(items);
    let mut store = Store::default();
    assert!(ticker_tips(Some(&layout), &strings, &store, 1).is_empty());
    for _ in 0..6 {
        store.count_song("pulse", "Fenix Funk 5 (Wipeout Edit)");
    }
    let tips = ticker_tips(Some(&layout), &strings, &store, 1);
    assert!(
        tips.iter()
            .any(|tip| tip == "Your most listened to music track is Fenix Funk 5 (Wipeout Edit)"),
        "{tips:?}"
    );
    assert!(
        tips.iter()
            .any(|tip| tip == "You've listened to a total of 6 songs")
    );
    assert!(
        ticker_tips(None, &strings, &store, 1).is_empty(),
        "no layout, no ticker"
    );
}
