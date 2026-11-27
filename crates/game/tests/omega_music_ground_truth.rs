//! Omega's soundtrack, read off the bank and played.
//!
//! **`#[ignore]`d and never run in CI**; needs `data/extracted/ps4`.

use oag_game::sound::GameLibrary;
use oag_sound::library;

fn source() -> Option<String> {
    let path = oag_testdata::exact("data/extracted/ps4")?;
    Some(path.display().to_string())
}

/// The engine's own listing offers the 28, and a song mixes to its segment's
/// length without clipping.
#[test]
#[ignore = "needs data/extracted/ps4"]
fn the_listing_and_a_song_load_through_the_engines_path() {
    let Some(source) = source() else { return };
    let listing = library::listing(&GameLibrary, &source)
        .expect("lists")
        .expect("Omega is an archived title");
    assert_eq!(listing.len(), 28);
    for e in &listing {
        assert!(e.seconds > 150.0 && e.seconds < 600.0, "{e:?}");
    }

    let cache = oag_testdata::scratch::cache("omega-music");
    // A song stored as one eight-channel stream plays through the same path.
    let folded = library::load_entry(&GameLibrary, &source, 2, &cache).expect("location 2 loads");
    let want = listing
        .iter()
        .find(|e| e.at == 2)
        .expect("location 2 is listed");
    assert_eq!(folded.channels(), 2);
    let seconds = folded.frames() as f64 / f64::from(folded.sample_rate());
    assert!((seconds - want.seconds).abs() < 0.001, "{seconds}");
    let sound = library::load_entry(&GameLibrary, &source, listing[0].at, &cache).expect("loads");
    let seconds = sound.frames() as f64 / f64::from(sound.sample_rate());
    assert!((seconds - listing[0].seconds).abs() < 0.001, "{seconds}");
}

/// The menus' `Game_FLOW` state selects one stereo loop; it is not one of the
/// soundtrack's songs.
#[test]
#[ignore = "needs data/extracted/ps4"]
fn the_front_ends_loop_is_walked_from_the_menus_state() {
    let Some(source) = source() else { return };
    let (name, sound) = library::load_front_end(
        &GameLibrary,
        &source,
        &oag_testdata::scratch::cache("omega-music"),
    )
    .expect("loads")
    .expect("Omega has a front-end loop");
    let seconds = sound.frames() as f64 / f64::from(sound.sample_rate());
    println!("{name}: {seconds:.2} s, {} Hz", sound.sample_rate());
    assert!(seconds > 10.0);
}
