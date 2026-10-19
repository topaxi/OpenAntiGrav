//! Which track a run's first race plays, decided on the worker from the booted
//! platform rather than from whether the menu's loop has landed yet.

use super::*;
use crate::race_music::RacePick;

fn booted(platform: Platform) -> MusicDiscs {
    MusicDiscs {
        library: &NoLibrary,
        listings: Listings::default(),
        later: Default::default(),
        psp: (platform == Platform::Psp).then(|| "psp.chd".into()),
        ps2: (platform == Platform::Ps2).then(|| "ps2.chd".into()),
        ps3: None,
        vita: None,
        ps4: None,
        booted: Some(platform),
    }
}

fn first_race(audio_music_from: Option<Platform>) -> RacePick {
    let (mut audio, ..) = super::psp_boot_fixture();
    audio.race_cache = None;
    audio.music_from = audio_music_from;
    audio.race_music_pick()
}

#[test]
fn a_ps2_boot_starts_one_past_the_menu_track_even_before_the_menu_lands() {
    let discs = booted(Platform::Ps2);
    // The menu's loop has not landed: `music_from` is still `None`.
    let pick = first_race(None);
    assert_eq!(pick.known(&discs), None, "needs the listing's length");
    assert_eq!(pick.resolve_for_test(&discs, 16), MUSIC_TRACK + 1);
    assert_eq!(
        first_race(Some(Platform::Ps2)).resolve_for_test(&discs, 16),
        1
    );
}

#[test]
fn a_psp_boot_starts_on_the_menu_track_without_a_listing() {
    let discs = booted(Platform::Psp);
    let pick = first_race(None);
    assert_eq!(pick.known(&discs), Some(MUSIC_TRACK));
    assert_eq!(pick.resolve_for_test(&discs, 16), MUSIC_TRACK);
}
