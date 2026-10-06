//! Omega's soundtrack, read off the bank and played.
//!
//! **`#[ignore]`d and never run in CI**; needs `data/extracted/ps4`.

use oag_game::sound::GameLibrary;
use oag_music::omega;
use oag_sound::library;

fn source() -> Option<String> {
    let path = oag_testdata::exact("data/extracted/ps4")?;
    Some(path.display().to_string())
}

/// The playlist is the 29 `PI_Music` entries: 17 play (7 to 11 stereo stems),
/// 11 are one eight-channel stream (refused by name) and location 0 has no
/// `Set_Music_Track_0__frontend` event.
#[test]
#[ignore = "needs data/extracted/ps4"]
fn the_playlist_resolves_seventeen_songs_and_names_why_twelve_do_not() {
    let Some(source) = source() else { return };
    let mut archives = oag_omega::open(&source).expect("opens");
    let st = oag_omega::MUSIC
        .state_tracks
        .expect("Omega declares state tracks");
    let plan = omega::plan(&mut archives, &st, true).expect("plan");

    let played: Vec<u32> = plan.songs.iter().map(|s| s.location).collect();
    assert_eq!(
        played,
        vec![
            1, 3, 7, 8, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 26, 27, 28
        ]
    );
    assert!(plan.songs.iter().all(|s| (7..=11).contains(&s.stems.len())));
    assert_eq!(plan.songs.iter().map(|s| s.stems.len()).sum::<usize>(), 154);

    let skipped: Vec<(u32, omega::Skip)> = plan
        .skipped
        .iter()
        .map(|(_, location, why)| (*location, why.clone()))
        .collect();
    assert_eq!(skipped.len(), 12);
    assert_eq!(skipped[0], (0, omega::Skip::NoEvent));
    assert!(
        skipped[1..]
            .iter()
            .all(|(_, why)| *why == omega::Skip::Channels(8))
    );
    assert_eq!(
        skipped.iter().map(|(l, _)| *l).collect::<Vec<_>>(),
        vec![0, 2, 4, 5, 6, 9, 10, 11, 12, 13, 24, 25]
    );
}

/// The engine's own listing offers the 17, and a song mixes to its segment's
/// length without clipping.
#[test]
#[ignore = "needs data/extracted/ps4"]
fn the_listing_and_a_song_load_through_the_engines_path() {
    let Some(source) = source() else { return };
    let listing = library::listing(&GameLibrary, &source)
        .expect("lists")
        .expect("Omega is an archived title");
    assert_eq!(listing.len(), 17);
    assert!(
        listing
            .iter()
            .all(|e| e.seconds > 150.0 && e.seconds < 400.0)
    );

    let cache = std::env::temp_dir();
    let sound = library::load_entry(&GameLibrary, &source, listing[0].at, &cache).expect("loads");
    let seconds = sound.frames() as f64 / f64::from(sound.sample_rate());
    assert!((seconds - listing[0].seconds).abs() < 0.001, "{seconds}");
}

/// Every multi-stem song mixes: its stems agree on rate, the mix is the
/// segment's length to the frame, and it is audible.
///
/// # Four slices, one test each
///
/// The claim is per song, so any partition of the multi-stem songs asserts the
/// same things as the one loop did. Song `i` (in plan order, multi-stem songs
/// only) belongs to slice `i % SLICES`, which covers every song with no
/// leftover whatever the count. The reason to split is wall clock: one test is
/// one process on one core, and the whole loop was 166 s under load.
const SLICES: usize = 4;

fn every_song_in_a_slice_mixes_to_its_segment_length(slice: usize) {
    let Some(source) = source() else { return };
    let mut archives = oag_omega::open(&source).expect("opens");
    let st = oag_omega::MUSIC.state_tracks.expect("state tracks");
    let plan = omega::plan(&mut archives, &st, false).expect("plan");
    let songs: Vec<_> = plan.songs.iter().filter(|s| s.stems.len() > 1).collect();
    assert!(
        songs.len() >= SLICES,
        "{} multi-stem songs leave a slice empty",
        songs.len()
    );
    for song in songs.into_iter().skip(slice).step_by(SLICES) {
        let pcm = omega::mix(&mut archives, &st, song).expect("mix");
        assert_eq!(
            (pcm.channels, pcm.sample_rate),
            (2, 48_000),
            "{}",
            song.title
        );
        let frames = pcm.samples.len() / 2;
        let want = (song.seconds * 48_000.0).round() as usize;
        assert_eq!(frames, want, "{}", song.title);
        let peak = pcm
            .samples
            .iter()
            .map(|s| i32::from(*s).abs())
            .max()
            .unwrap();
        assert!(peak > 1000, "{} is silent", song.title);
    }
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn songs_slice_0_mix_to_their_segment_length() {
    every_song_in_a_slice_mixes_to_its_segment_length(0);
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn songs_slice_1_mix_to_their_segment_length() {
    every_song_in_a_slice_mixes_to_its_segment_length(1);
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn songs_slice_2_mix_to_their_segment_length() {
    every_song_in_a_slice_mixes_to_its_segment_length(2);
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn songs_slice_3_mix_to_their_segment_length() {
    every_song_in_a_slice_mixes_to_its_segment_length(3);
}

/// The menus' `Game_FLOW` state selects one stereo loop; it is not one of the
/// soundtrack's songs.
#[test]
#[ignore = "needs data/extracted/ps4"]
fn the_front_ends_loop_is_walked_from_the_menus_state() {
    let Some(source) = source() else { return };
    let (name, sound) = library::load_front_end(&GameLibrary, &source, &std::env::temp_dir())
        .expect("loads")
        .expect("Omega has a front-end loop");
    let seconds = sound.frames() as f64 / f64::from(sound.sample_rate());
    println!("{name}: {seconds:.2} s, {} Hz", sound.sample_rate());
    assert!(seconds > 10.0);
}
