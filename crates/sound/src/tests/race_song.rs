//! The race song held for the green light: silent through the countdown, started by the first
//! step the race is no longer gated, and re-armed by the next race. Built on
//! [`psp_boot_fixture`], whose race track is already decoded, so no disc is read.

use super::*;
use crate::sfx::RaceFrame;

fn discs() -> MusicDiscs {
    MusicDiscs {
        library: &NoLibrary,
        listings: Listings::default(),
        later: Default::default(),
        psp: Some("psp.chd".into()),
        ps2: Some("ps2.chd".into()),
        ps3: None,
        vita: None,
        ps4: None,
        booted: Some(Platform::Psp),
    }
}

/// One step of a race with no effects banks, gated or not.
fn step(audio: &mut Audio, gated: bool) {
    let banks = Default::default();
    let emitters = Default::default();
    let announcer = Default::default();
    let class_announcer = Default::default();
    audio.race_tick(RaceFrame {
        banks: &banks,
        emitters: &emitters,
        announcer: &announcer,
        class_announcer: &class_announcer,
        cues: Vec::new(),
        announcements: Vec::new(),
        class_announcements: Vec::new(),
        listener: oag_audio::Listener::at_origin(),
        craft: [None; oag_gameplay::MAX_SHIPS],
        slot_teams: [None; oag_gameplay::MAX_SHIPS],
        throttle: 0.0,
        projectiles: oag_weapons::projectile::Projectiles::new().slots,
        running: true,
        shielded: false,
        exploding: false,
        autopilot_active: false,
        thrust_gated: gated,
        sight: oag_race::sight::State::Absent,
        quake_point: None,
        leach_beam: None,
    });
}

#[test]
fn a_held_race_song_starts_on_the_first_step_the_race_is_not_gated() {
    let (mut audio, ..) = psp_boot_fixture();
    audio.hold_race_song(true);
    audio.start_race_music(&discs(), MusicSource::Auto, Path::new("unused"));

    assert!(audio.music.is_none(), "the menu voice still gives way");
    assert!(
        audio.race_voice.is_none(),
        "no race voice before the release"
    );
    for _ in 0..272 {
        step(&mut audio, true);
        audio.tick();
        assert!(audio.race_voice.is_none(), "silent through the countdown");
        assert!(audio.now_playing().is_none(), "and nothing to display");
    }

    step(&mut audio, false);
    assert!(audio.race_voice.is_some(), "the release starts the song");
    assert_eq!(audio.playhead_race(), Some(0.0), "from its start");
    let song = audio.now_playing().expect("the display has its song");
    assert_eq!(song.serial, 1);
}

#[test]
fn without_the_hold_the_song_starts_at_load_as_it_did() {
    let (mut audio, ..) = psp_boot_fixture();
    audio.hold_race_song(false);
    audio.start_race_music(&discs(), MusicSource::Auto, Path::new("unused"));
    assert!(audio.race_voice.is_some(), "an AtLoad title plays at once");
    assert!(audio.now_playing().is_some());
}

#[test]
fn a_song_that_lands_after_the_release_starts_when_it_lands() {
    let (mut audio, ..) = psp_boot_fixture();
    audio.hold_race_song(true);
    step(&mut audio, false);
    audio.start_race_music(&discs(), MusicSource::Auto, Path::new("unused"));
    assert!(
        audio.race_voice.is_some(),
        "released already, so a late track does not wait for a second release"
    );
}

#[test]
fn a_restart_arms_the_hold_again_and_starts_one_voice() {
    let (mut audio, ..) = psp_boot_fixture();
    audio.hold_race_song(true);
    audio.start_race_music(&discs(), MusicSource::Auto, Path::new("unused"));
    step(&mut audio, false);
    let first = audio.race_voice.expect("the first race's song");
    assert_eq!(audio.song_serial, 1);

    // The restart: the same hand-off the first race went through.
    audio.pause_race_music();
    audio.hold_race_song(true);
    audio.start_race_music(&discs(), MusicSource::Auto, Path::new("unused"));
    assert!(
        audio.race_voice.is_none(),
        "the second race is silent again"
    );
    assert!(
        audio.now_playing().is_none(),
        "and does not show the song the race before it started"
    );
    assert!(!audio.output.with_mixer(|mixer| mixer.is_playing(first)));

    step(&mut audio, true);
    assert!(audio.race_voice.is_none());
    step(&mut audio, false);
    let second = audio.race_voice.expect("the second race's song");
    assert_ne!(first, second);
    step(&mut audio, false);
    assert_eq!(
        audio.race_voice,
        Some(second),
        "later steps do not start the song again"
    );
    assert_eq!(audio.song_serial, 2, "one release, one new song");
    assert!(
        audio.now_playing().is_some(),
        "the display has it at the release"
    );
}

#[test]
fn leaving_the_race_during_the_countdown_keeps_the_song_for_the_release() {
    let (mut audio, ..) = psp_boot_fixture();
    audio.hold_race_song(true);
    audio.start_race_music(&discs(), MusicSource::Auto, Path::new("unused"));
    step(&mut audio, true);
    audio.pause_race_music();
    assert!(audio.race_voice.is_none());

    // Back from the menus, still before the light: the same hold applies.
    audio.start_race_music(&discs(), MusicSource::Auto, Path::new("unused"));
    assert!(audio.race_voice.is_none());
    step(&mut audio, false);
    assert!(audio.race_voice.is_some());
}
