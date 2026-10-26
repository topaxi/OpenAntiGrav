//! The song names the now-playing display shows, against Pulse's real discs.
//!
//! **`#[ignore]`d and never run in CI**; needs `data/images/`. Run with `just test-data`.
//!
//! Pulse is found by what its entries are, not by its `PI_Music` declaration, so the display
//! matches each entry to its node by the name hash of the node's audio file. The claim here
//! is that every one of the sixteen entries gets a name, and that the playlist order is the
//! one it had before names existed.

use std::path::Path;

use oag_game::sound::GameLibrary;
use oag_sound::library;

fn named(
    image: &str,
) -> Option<(
    Vec<oag_music::Entry>,
    Vec<Option<oag_music::song_names::SongName>>,
)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(image);
    if !path.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            path.display()
        );
        return None;
    }
    Some(
        library::named_listing(&GameLibrary, &path.display().to_string())
            .expect("reading the listing")
            .expect("a PSP disc this build knows"),
    )
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_pulse_soundtrack_entry_has_an_artist_and_a_name() {
    for image in [
        "data/images/pulse-psp-eu.chd",
        "data/images/pulse-psp-usa.chd",
    ] {
        let Some((entries, names)) = named(image) else {
            continue;
        };
        assert_eq!(entries.len(), 16, "{image}");
        assert_eq!(names.len(), entries.len(), "{image}");
        for (entry, name) in entries.iter().zip(&names) {
            let name = name
                .as_ref()
                .unwrap_or_else(|| panic!("{image}: entry {:08x} has no name", entry.at));
            assert!(name.artist.is_some(), "{image}: {name:?}");
        }
        assert!(
            names
                .iter()
                .flatten()
                .any(|name| name.text(" - ") == "Aphex Twin - Fenix Funk 5 (Wipeout Edit)"),
            "{image}: {names:?}"
        );
    }
}

/// What the HUD's draw list gets for Pulse's first soundtrack entry, end to end: the name off
/// the disc, through `Race::note_song`, the lights, the scroll and the fade, into the frame
/// `oag_hud::now_playing::append` adds to the HUD's own.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn pulse_shows_the_song_from_the_green_light_and_takes_it_down_again() {
    use oag_gameplay::PlayerInputs;

    let Some(source) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let source = source.display().to_string();
    let (_, names) = library::named_listing(&GameLibrary, &source)
        .expect("listing")
        .expect("a PSP disc");
    let name = names.into_iter().next().flatten().expect("a first name");
    let text = name.text(" - ");

    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source,
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading a Pulse race");
    let mut race = oag_raceplay::Race::start(loaded.setup);
    race.note_song(Some(&oag_sound::NowPlaying {
        serial: 1,
        name: Some(name),
    }));

    let mut shown = Vec::new();
    for tick in 0..1500u32 {
        race.tick(&PlayerInputs::none());
        shown.push((tick, race.readout().now_playing));
    }
    let first = shown
        .iter()
        .position(|(_, m)| m.is_some())
        .expect("never shown");
    let last = shown
        .iter()
        .rposition(|(_, m)| m.is_some())
        .expect("never shown");
    let green = oag_race::COUNTDOWN_TICKS as usize;
    assert!(
        first >= green - 1 && first <= green + 1,
        "the display comes up with the green light ({green}), not at {first}"
    );
    assert!(
        last > first + 300,
        "it scrolls along the bar: {first}..{last}"
    );
    assert!(
        last < 1500 - 1,
        "it takes itself down again: ended at {last}"
    );

    let marquee_at = |tick: usize| shown[tick].1.clone().expect("up");
    let early = marquee_at(first + 10);
    let later = marquee_at(first + 100);
    assert_eq!(early.text, text);
    assert!(early.color[3] < later.color[3], "fading in");
    assert!(later.x < early.x, "scrolling left");
    assert!(
        marquee_at(last).color[3] < marquee_at(last - 30).color[3],
        "fading out"
    );

    let mut frame = oag_hud::Frame::default();
    oag_hud::now_playing::append(&mut frame, &later, [0.0, 0.0, 0.0, 1.0]);
    assert_eq!(frame.small_text.len(), 1);
    match &frame.small_text[0] {
        oag_ui::frontend::Draw::Text { text: drawn, .. } => assert_eq!(drawn, &text),
        other => panic!("{other:?}"),
    }
    assert_eq!(frame.spans.last().expect("clipped").clip, later.clip);
}
