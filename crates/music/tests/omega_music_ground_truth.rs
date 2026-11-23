//! Omega's soundtrack as the bank resolves it: the playlist, the mix of every
//! song, and the eight-channel songs folded to stereo.
//!
//! **`#[ignore]`d and never run in CI**; needs `data/extracted/ps4`.

use oag_music::omega;

fn source() -> Option<String> {
    let path = oag_testdata::exact("data/extracted/ps4")?;
    Some(path.display().to_string())
}

/// The playlist is the 29 `PI_Music` entries: 28 play (7 to 11 stereo stems, or
/// one eight-channel stream folded to stereo) and location 0 has no
/// `Set_Music_Track_0__frontend` event.
#[test]
#[ignore = "needs data/extracted/ps4"]
fn the_playlist_resolves_twenty_eight_songs_and_names_why_one_does_not() {
    let Some(source) = source() else { return };
    let mut archives = oag_omega::open(&source).expect("opens");
    let st = oag_omega::MUSIC
        .state_tracks
        .expect("Omega declares state tracks");
    let plan = omega::plan(&mut archives, &st, true).expect("plan");

    let played: Vec<u32> = plan.songs.iter().map(|s| s.location).collect();
    assert_eq!(played, (1..=28).collect::<Vec<u32>>());
    let (stems, single): (Vec<_>, Vec<_>) = plan.songs.iter().partition(|s| s.stems.len() > 1);
    assert!(stems.iter().all(|s| (7..=11).contains(&s.stems.len())));
    assert_eq!(stems.iter().map(|s| s.stems.len()).sum::<usize>(), 154);
    assert_eq!(
        single.iter().map(|s| s.location).collect::<Vec<_>>(),
        FOLDED
    );

    assert_eq!(
        plan.skipped
            .iter()
            .map(|(_, location, why)| (*location, why.clone()))
            .collect::<Vec<_>>(),
        vec![(0, omega::Skip::NoEvent)]
    );
}

/// The `PI_Music` locations stored as one eight-channel stream.
const FOLDED: [u32; 11] = [2, 4, 5, 6, 9, 10, 11, 12, 13, 24, 25];

/// A mix's peak, RMS (both of full scale) and the share of samples at full scale.
fn level(samples: &[i16]) -> (f64, f64, f64) {
    let peak = samples
        .iter()
        .map(|s| i32::from(*s).abs())
        .max()
        .unwrap_or(0);
    let sum: f64 = samples.iter().map(|s| f64::from(*s).powi(2)).sum();
    let rms = (sum / samples.len() as f64).sqrt();
    let clipped = samples
        .iter()
        .filter(|s| s.unsigned_abs() >= 32_767)
        .count();
    (
        f64::from(peak) / 32_768.0,
        rms / 32_768.0,
        clipped as f64 / samples.len() as f64,
    )
}

/// Each of the eleven folded songs mixes to stereo at its segment's length, is
/// audible, and does not clip, against a stereo song of the same disc. With
/// `OAG_FOLD_WAV_DIR` set it also writes the first 20 s of each as a WAV.
fn the_folded_songs_in_a_slice_sit_beside_the_stereo_ones(slice: usize) {
    let Some(source) = source() else { return };
    let mut archives = oag_omega::open(&source).expect("opens");
    let st = oag_omega::MUSIC.state_tracks.expect("state tracks");
    let plan = omega::plan(&mut archives, &st, false).expect("plan");

    let reference = plan.songs.iter().find(|s| s.location == 1).expect("song 1");
    let stereo = omega::mix(&mut archives, &st, reference).expect("mix");
    let (ref_peak, ref_rms, _) = level(&stereo.samples);
    println!("stereo song 1: peak {ref_peak:.3} rms {ref_rms:.4}");

    for location in FOLDED.into_iter().skip(slice).step_by(SLICES) {
        let song = plan
            .songs
            .iter()
            .find(|s| s.location == location)
            .unwrap_or_else(|| panic!("location {location} is not in the playlist"));
        let pcm = omega::mix(&mut archives, &st, song).expect("mix");
        assert_eq!((pcm.channels, pcm.sample_rate), (2, 48_000), "{location}");
        let want = (song.seconds * 48_000.0).round() as usize;
        assert_eq!(pcm.samples.len() / 2, want, "{location}");
        let (peak, rms, clipped) = level(&pcm.samples);
        println!(
            "location {location}: peak {peak:.3} rms {rms:.4} (x{:.2} of the stereo song) clipped {clipped:.6}",
            rms / ref_rms
        );
        assert!(clipped < 0.0005, "{location} clips: {clipped}");
        assert!(rms > ref_rms * 0.1, "{location} is near-silent: {rms}");
        assert!(rms < ref_rms * 3.0, "{location} is far louder: {rms}");
        if let Ok(dir) = std::env::var("OAG_FOLD_WAV_DIR") {
            let head = &pcm.samples[..(20 * 48_000 * 2).min(pcm.samples.len())];
            let floats: Vec<f32> = head.iter().map(|s| f32::from(*s) / 32_768.0).collect();
            let wav = oag_audio::wav::from_samples(&floats, 48_000);
            std::fs::write(format!("{dir}/location-{location}-head.wav"), wav).expect("writes");
        }
    }
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

#[test]
#[ignore = "needs data/extracted/ps4"]
fn folded_songs_slice_0_sit_beside_the_stereo_ones() {
    the_folded_songs_in_a_slice_sit_beside_the_stereo_ones(0);
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn folded_songs_slice_1_sit_beside_the_stereo_ones() {
    the_folded_songs_in_a_slice_sit_beside_the_stereo_ones(1);
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn folded_songs_slice_2_sit_beside_the_stereo_ones() {
    the_folded_songs_in_a_slice_sit_beside_the_stereo_ones(2);
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn folded_songs_slice_3_sit_beside_the_stereo_ones() {
    the_folded_songs_in_a_slice_sit_beside_the_stereo_ones(3);
}
