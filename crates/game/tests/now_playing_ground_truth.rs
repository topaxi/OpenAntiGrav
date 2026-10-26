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
    oag_hud::now_playing::append(
        &mut frame,
        &later,
        [0.0, 0.0, 0.0, 1.0],
        &oag_hud::sprite::Sheet::default(),
    );
    assert_eq!(frame.small_text.len(), 1);
    match &frame.small_text[0] {
        oag_ui::frontend::Draw::Text { text: drawn, .. } => assert_eq!(drawn, &text),
        other => panic!("{other:?}"),
    }
    assert_eq!(frame.spans.last().expect("clipped").clip, later.clip);
}

/// The seam: the song the display shows is chosen with no device open. A `--no-audio` run
/// lands the same race song, and `oag_game::sound::race_tick` hands it to the race - deleting
/// that call, or tying the song to a sounding voice, leaves this red.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn a_no_audio_run_shows_the_song_it_would_play() {
    use oag_gameplay::PlayerInputs;

    let Some(source) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let source = source.display().to_string();
    let cache = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("now-playing-audio");
    let discs = oag_sound::MusicDiscs::survey(&source, &GameLibrary);
    let mut audio = oag_sound::Audio::open(
        &oag_sound::settings::Settings::default(),
        None,
        None,
        oag_audio::MIN_BUFFER,
        true,
    );
    audio.start_race_music(&discs, oag_sound::MusicSource::Auto, &cache);
    audio.settle_music();
    let song = audio.now_playing().expect("the race song landed");
    assert_eq!(
        song.name.as_ref().map(|name| name.text(" - ")).as_deref(),
        Some("Aphex Twin - Fenix Funk 5 (Wipeout Edit)"),
        "the first track of Pulse's soundtrack"
    );

    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source,
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading a Pulse race");
    let mut race = oag_raceplay::Race::start(loaded.setup);
    let mut text = None;
    for _ in 0..400 {
        race.tick(&PlayerInputs::none());
        oag_game::sound::race_tick(&mut audio, &mut race);
        text = text.or_else(|| race.readout().now_playing.map(|marquee| marquee.text));
    }
    assert_eq!(
        text.as_deref(),
        Some("Aphex Twin - Fenix Funk 5 (Wipeout Edit)")
    );
}

/// Pure authors no display, so it borrows Pulse's through the fallback layer; its own
/// `PI_Music` nodes name the songs. Shown from the green light like Pulse's.
#[test]
#[ignore = "needs data/images/pure-psp-eu.chd"]
fn pure_borrows_pulses_display_and_names_its_own_songs() {
    use oag_gameplay::PlayerInputs;

    let Some(source) = oag_testdata::image("data/images/pure-psp-eu.chd") else {
        return;
    };
    let source = source.display().to_string();
    let (entries, names) = library::named_listing(&GameLibrary, &source)
        .expect("listing")
        .expect("a PSP disc");
    assert_eq!(entries.len(), 19);
    assert!(
        names
            .iter()
            .all(|name| name.as_ref().is_some_and(|n| n.artist.is_some()))
    );
    let name = names.into_iter().next().flatten().expect("a first name");
    let text = name.text(" - ");

    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source,
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading a Pure race");
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.starts_with("now playing:")
                && line.contains("borrowed from Wipeout Pulse")),
        "the load says it is borrowed: {:?}",
        loaded
            .report
            .iter()
            .filter(|l| l.contains("now playing"))
            .collect::<Vec<_>>()
    );
    let mut race = oag_raceplay::Race::start(loaded.setup);
    race.note_song(Some(&oag_sound::NowPlaying {
        serial: 1,
        name: Some(name),
    }));
    let mut shown = None;
    for _ in 0..400 {
        race.tick(&PlayerInputs::none());
        shown = shown.or_else(|| race.readout().now_playing.map(|marquee| marquee.text));
    }
    assert_eq!(shown, Some(text));
}

/// HD authors the same item with a textured bar from `HUD_Components.gtf`, in its 1920x1080
/// grid; the bar's art has to be in the HUD's sheet for the bar to draw.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hd_shows_its_own_item_with_the_bar_art_from_its_sheet() {
    use oag_gameplay::PlayerInputs;

    let Some(source) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let source = source.display().to_string();
    let (entries, names) = library::named_listing(&GameLibrary, &source)
        .expect("listing")
        .expect("a PS3 disc");
    assert_eq!(entries.len(), 15);
    assert!(
        names
            .iter()
            .all(|name| name.as_ref().is_some_and(|n| n.artist.is_some())),
        "{names:?}"
    );
    let name = names.into_iter().next().flatten().expect("a first name");
    let text = name.text(" - ");

    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source,
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading an HD race");
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.starts_with("now playing:") && line.contains("measured")),
        "{:?}",
        loaded
            .report
            .iter()
            .filter(|l| l.contains("now playing"))
            .collect::<Vec<_>>()
    );
    let sheet = loaded.hud.sheet.clone();
    let mut race = oag_raceplay::Race::start(loaded.setup);
    race.note_song(Some(&oag_sound::NowPlaying {
        serial: 1,
        name: Some(name),
    }));
    let mut shown = None;
    for _ in 0..400 {
        race.tick(&PlayerInputs::none());
        shown = shown.or_else(|| race.readout().now_playing);
    }
    let marquee = shown.expect("shown");
    assert_eq!(marquee.text, text);
    let mut frame = oag_hud::Frame::default();
    oag_hud::now_playing::append(&mut frame, &marquee, [0.0, 0.0, 0.0, 1.0], &sheet);
    assert_eq!(
        frame.sprites.len(),
        1,
        "the bar's texture is in HD's HUD sheet: {:?}",
        marquee.bar_art
    );
}

/// Omega ships HD's item in its own `data00.psarc`; its 17 playable songs are named by the music
/// plugin's `PI_Music` nodes, addressed by `location`.
#[test]
#[ignore = "needs data/extracted/ps4"]
fn omega_shows_hds_item_and_names_its_seventeen_songs() {
    use oag_gameplay::PlayerInputs;

    let Some(source) = oag_testdata::image("data/extracted/ps4") else {
        return;
    };
    let source = source.display().to_string();
    let (entries, names) = library::named_listing(&GameLibrary, &source)
        .expect("listing")
        .expect("Omega is an archived title");
    assert_eq!(entries.len(), 17);
    assert!(names.iter().all(Option::is_some), "{names:?}");
    let name = names.into_iter().next().flatten().expect("a first name");
    let text = name.text(" - ");

    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source,
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading an Omega race");
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.starts_with("now playing:")
                && line.contains("borrowed from Wipeout HD")),
        "{:?}",
        loaded
            .report
            .iter()
            .filter(|l| l.contains("now playing"))
            .collect::<Vec<_>>()
    );
    let sheet = loaded.hud.sheet.clone();
    let mut race = oag_raceplay::Race::start(loaded.setup);
    race.note_song(Some(&oag_sound::NowPlaying {
        serial: 1,
        name: Some(name),
    }));
    let mut shown = None;
    for _ in 0..400 {
        race.tick(&PlayerInputs::none());
        shown = shown.or_else(|| race.readout().now_playing);
    }
    let marquee = shown.expect("shown");
    assert_eq!(marquee.text, text);
    let mut frame = oag_hud::Frame::default();
    oag_hud::now_playing::append(&mut frame, &marquee, [0.0, 0.0, 0.0, 1.0], &sheet);
    assert_eq!(frame.sprites.len(), 1, "the bar's texture is in the sheet");
}

/// 2048 authors no display (checked in the base package and the 1.04 patch), so it borrows HD's
/// through the fallback layer, and names its songs from its own `PI_Music` nodes.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn the_2048_borrows_hds_display_and_names_its_own_songs() {
    use oag_gameplay::PlayerInputs;

    let Some(source) = oag_testdata::image("data/extracted/vita/PCSF00007") else {
        return;
    };
    let source = source.display().to_string();
    let (entries, names) = library::named_listing(&GameLibrary, &source)
        .expect("listing")
        .expect("2048 is an archived title");
    assert_eq!(entries.len(), 11);
    assert!(names.iter().all(Option::is_some), "{names:?}");
    let name = names.into_iter().next().flatten().expect("a first name");

    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source,
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading a 2048 race");
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.starts_with("now playing:")
                && line.contains("borrowed from Wipeout HD")),
        "{:?}",
        loaded
            .report
            .iter()
            .filter(|l| l.contains("now playing"))
            .collect::<Vec<_>>()
    );
    let mut race = oag_raceplay::Race::start(loaded.setup);
    race.note_song(Some(&oag_sound::NowPlaying {
        serial: 1,
        name: Some(name),
    }));
    let mut shown = None;
    for _ in 0..400 {
        race.tick(&PlayerInputs::none());
        shown = shown.or_else(|| race.readout().now_playing.map(|marquee| marquee.text));
    }
    // The node is named `01`; the title is the second line of the language table's `MT_01`.
    let shown = shown.expect("shown");
    assert_eq!(shown, "DirtyLoud - School of Funk");
}
