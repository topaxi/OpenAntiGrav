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
/// the disc, through `Race::note_song` when the song starts at the release, the scroll and the
/// fade, into the frame `oag_hud::now_playing::append` adds to the HUD's own.
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
    let song = oag_sound::NowPlaying {
        serial: 1,
        name: Some(name),
    };

    let mut shown = Vec::new();
    for tick in 0..1500u32 {
        race.tick(&PlayerInputs::none());
        // The sound tells the race on the step that voices `go`, which is the last gated one.
        if u64::from(tick) == oag_race::COUNTDOWN_TICKS - 1 {
            race.note_song(Some(&song));
        }
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
    let Some(beats) = timeline(
        "data/images/pulse-psp-eu.chd",
        oag_race::Mode::TimeTrial,
        "VENOM",
    ) else {
        return;
    };
    let text = beats
        .iter()
        .find_map(|beat| beat.shown.clone())
        .expect("the display came up");
    assert_eq!(
        text, "Aphex Twin - Fenix Funk 5 (Wipeout Edit)",
        "the first track of Pulse's soundtrack"
    );
}

/// One step of a race as a seat sees it: whether `go` was raised in it, whether a race voice is
/// sounding after it, and what the marquee shows.
struct Beat {
    go: bool,
    sounding: bool,
    shown: Option<String>,
}

/// Races `image` on a null audio backend with the title's own start rule, from the first step to
/// a little past the release, and records each step.
fn timeline(image: &str, mode: oag_race::Mode, class: &str) -> Option<Vec<Beat>> {
    use oag_gameplay::PlayerInputs;
    use oag_gameplay::cue::Cue;

    let source = oag_testdata::image(image)?.display().to_string();
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: source.clone(),
        mode,
        class: class.to_string(),
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading the race");
    let cache = Path::new(env!("CARGO_TARGET_TMPDIR")).join("now-playing-audio");
    let discs = oag_sound::MusicDiscs::survey(&source, &GameLibrary);
    let mut audio = oag_sound::Audio::open(
        &oag_sound::settings::Settings::default(),
        None,
        None,
        oag_audio::MIN_BUFFER,
        true,
    );
    // What the race hand-off does: the title's rule, then the music, then the wait for the fetch
    // (a `--screenshot` capture's seam) so the song is a function of the tick count.
    oag_game::sound::hold_race_song(&mut audio, loaded.title);
    audio.start_race_music(&discs, oag_sound::MusicSource::Auto, &cache);
    audio.settle_music();
    let mut race = oag_raceplay::Race::start(loaded.setup);

    let mut beats = Vec::new();
    for _ in 0..oag_race::COUNTDOWN_TICKS + 30 {
        race.tick(&PlayerInputs::none());
        let go = race.pending_cues().iter().any(|event| event.cue == Cue::Go);
        oag_game::sound::race_tick(&mut audio, &mut race);
        beats.push(Beat {
            go,
            sounding: audio.playhead_race().is_some(),
            shown: race.readout().now_playing.map(|marquee| marquee.text),
        });
    }
    Some(beats)
}

/// The claim, for one title and mode: no race song and no display before the step that voices
/// `go`; both on it.
fn assert_the_song_starts_on_go(beats: &[Beat], name: &str) {
    let go = beats
        .iter()
        .position(|beat| beat.go)
        .unwrap_or_else(|| panic!("{name}: the start voiced no go"));
    assert_eq!(
        go as u64,
        oag_race::COUNTDOWN_TICKS - 1,
        "{name}: go's step"
    );
    for (tick, beat) in beats[..go].iter().enumerate() {
        assert!(!beat.sounding, "{name}: a race song sounds in step {tick}");
        assert!(
            beat.shown.is_none(),
            "{name}: the display is up in step {tick}"
        );
    }
    assert!(beats[go].sounding, "{name}: the song starts on go");
    let text = beats[go]
        .shown
        .as_deref()
        .unwrap_or_else(|| panic!("{name}: the display is not up on go's step"));
    assert!(text.contains(" - "), "{name}: {text:?}");
    assert!(beats[go + 1..].iter().all(|beat| beat.sounding), "{name}");
}

/// Pulse starts the song on `go`, in Time Trial, a single race and Zone alike; the PS2 pressing
/// carries the PSP's rule, which `oag_pulse::MUSIC` states once for both.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn pulse_starts_the_song_and_its_display_together_on_go() {
    for (mode, name) in [
        (oag_race::Mode::TimeTrial, "time trial"),
        (oag_race::Mode::SingleRace, "single race"),
        (oag_race::Mode::Zone, "zone"),
    ] {
        let Some(beats) = timeline("data/images/pulse-psp-eu.chd", mode, "VENOM") else {
            return;
        };
        assert_the_song_starts_on_go(&beats, name);
    }
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hd_starts_the_song_and_its_display_together_on_go() {
    let Some(beats) = timeline(
        "data/images/hdfury-ps3-eu-dec.iso",
        oag_race::Mode::TimeTrial,
        "VENOM",
    ) else {
        return;
    };
    assert_the_song_starts_on_go(&beats, "hd");
}

#[test]
#[ignore = "needs data/images/pure-psp-eu.chd"]
fn pure_inherits_pulses_start_and_holds_the_song_for_go() {
    let Some(beats) = timeline(
        "data/images/pure-psp-eu.chd",
        oag_race::Mode::TimeTrial,
        "VENOM",
    ) else {
        return;
    };
    assert_the_song_starts_on_go(&beats, "pure");
}

/// Omega and 2048 inherit HD's start; their songs are Wwise and plugin-declared, but a song is a
/// decoded track on the race voice the same way.
#[test]
#[ignore = "needs data/extracted/ps4"]
fn omega_inherits_hds_start_and_holds_the_song_for_go() {
    let Some(beats) = timeline("data/extracted/ps4", oag_race::Mode::TimeTrial, "VENOM") else {
        return;
    };
    assert_the_song_starts_on_go(&beats, "omega");
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn the_2048_inherits_hds_start_and_holds_the_song_for_go() {
    let Some(beats) = timeline(
        "data/extracted/vita/PCSF00007",
        oag_race::Mode::TimeTrial,
        "VENOM",
    ) else {
        return;
    };
    assert_the_song_starts_on_go(&beats, "2048");
}

/// The field is read: a title whose rule were `AtLoad` would have its song at step 0. Flips the
/// hold off the way such a title's hand-off would and expects exactly that.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn a_title_that_starts_its_song_at_load_has_it_from_the_first_step() {
    use oag_gameplay::PlayerInputs;

    let Some(source) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let source = source.display().to_string();
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: source.clone(),
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading a Pulse race");
    assert!(
        loaded
            .title
            .music
            .is_some_and(oag_title::Music::song_waits_for_go)
    );
    let cache = Path::new(env!("CARGO_TARGET_TMPDIR")).join("now-playing-audio");
    let discs = oag_sound::MusicDiscs::survey(&source, &GameLibrary);
    let mut audio = oag_sound::Audio::open(
        &oag_sound::settings::Settings::default(),
        None,
        None,
        oag_audio::MIN_BUFFER,
        true,
    );
    audio.hold_race_song(false);
    audio.start_race_music(&discs, oag_sound::MusicSource::Auto, &cache);
    audio.settle_music();
    let mut race = oag_raceplay::Race::start(loaded.setup);
    race.tick(&PlayerInputs::none());
    oag_game::sound::race_tick(&mut audio, &mut race);
    assert!(audio.playhead_race().is_some(), "sounding at step 0");
    assert!(race.readout().now_playing.is_some(), "and shown with it");
}

/// Pure authors no display, so it borrows Pulse's through the fallback layer; its own
/// `PI_Music` nodes name the songs. Shown with the song, as Pulse's is.
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
