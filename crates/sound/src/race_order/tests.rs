use super::*;
use crate::race_music::{RacePick, random_first_index};

fn discs() -> MusicDiscs {
    MusicDiscs {
        booted: Some(Platform::Psp),
        psp: Some("psp.chd".into()),
        ..MusicDiscs::default()
    }
}

/// An audio mid-race on song `index` of a sixteen-song playlist.
fn on_song(order: MusicOrder, index: usize) -> Audio {
    let (mut audio, ..) = super::super::tests::psp_boot_fixture();
    audio.race_tracks = 16;
    audio.set_music_order(order);
    audio.seed_music_shuffle(42);
    audio.set_race_index(index);
    audio
}

fn exact_index(pick: RacePick) -> usize {
    (0..16)
        .find(|&i| pick == RacePick::exact(i))
        .expect("an exact pick of the playlist")
}

#[test]
fn a_sequential_order_asks_for_the_index_after_and_the_one_before() {
    let discs = discs();
    let mut audio = on_song(MusicOrder::Sequential, 5);
    assert_eq!(audio.peek_next_race_pick(&discs, 5), RacePick::after(5));
    assert_eq!(audio.take_next_race_pick(&discs, 5), RacePick::after(5));
    assert_eq!(
        audio.take_previous_race_pick(&discs, 5),
        Some(RacePick::before(5))
    );
}

#[test]
fn a_random_order_plays_every_song_once_and_the_prefetch_is_the_pick_taken() {
    let discs = discs();
    let mut audio = on_song(MusicOrder::Random, 5);
    let mut played = vec![5];
    for _ in 0..15 {
        let ahead = audio.peek_next_race_pick(&discs, *played.last().unwrap());
        let taken = audio.take_next_race_pick(&discs, *played.last().unwrap());
        assert_eq!(
            ahead, taken,
            "the fetch started ahead is the one the boundary wants"
        );
        let index = exact_index(taken);
        // What a landing does with the song it chose.
        audio.set_race_index(index);
        played.push(index);
    }
    played.sort_unstable();
    assert_eq!(played, (0..16).collect::<Vec<_>>());
}

#[test]
fn previous_under_a_random_order_is_the_song_that_played_before() {
    let discs = discs();
    let mut audio = on_song(MusicOrder::Random, 5);
    assert_eq!(
        audio.take_previous_race_pick(&discs, 5),
        None,
        "nothing earlier: the caller starts the song over"
    );
    let second = exact_index(audio.take_next_race_pick(&discs, 5));
    audio.set_race_index(second);
    assert_eq!(
        audio.take_previous_race_pick(&discs, second),
        Some(RacePick::exact(5))
    );
}

#[test]
fn switching_the_order_drops_a_song_fetched_ahead_for_the_old_one() {
    let discs = discs();
    let mut audio = on_song(MusicOrder::Sequential, 5);
    audio.race_prefetch = Some(MusicFetchWorker::spawn(
        discs,
        MusicSource::Auto,
        oag_testdata::scratch::cache("race-order-music"),
        RacePick::after(5),
        "race-music",
    ));
    audio.set_music_order(MusicOrder::Sequential);
    assert!(audio.race_prefetch.is_some(), "the same order keeps it");
    audio.set_music_order(MusicOrder::Random);
    assert!(audio.race_prefetch.is_none());
}

#[test]
fn a_random_run_opens_on_a_drawn_song_and_a_ps2_boot_never_on_the_menus() {
    let discs = discs();
    let mut audio = on_song(MusicOrder::Random, 0);
    audio.race_index = None;
    let pick = audio.race_music_pick();
    assert_eq!(pick.known(&discs), None, "drawn once the length is known");
    let mut seen = std::collections::BTreeSet::new();
    for seed in 1..400 {
        let index = random_first_index(seed, 16, true);
        assert!(index < 16);
        assert_ne!(index, MUSIC_TRACK, "seed {seed}: the menu is playing it");
        seen.insert(random_first_index(seed, 16, false));
    }
    assert!(seen.len() > 8, "a draw, not a constant: {seen:?}");
    assert_eq!(random_first_index(7, 0, true), MUSIC_TRACK);
    assert_eq!(
        pick.resolve_for_test(&discs, 16),
        random_first_index(
            // The seed the pick drew is private to it; the same draw comes out of a twin.
            {
                let mut twin = on_song(MusicOrder::Random, 0);
                twin.race_index = None;
                twin.race_order.shuffle.draw_seed()
            },
            16,
            false,
        )
    );
}

#[test]
fn a_sequential_run_still_opens_on_the_disc_track() {
    let discs = discs();
    let mut audio = on_song(MusicOrder::Sequential, 0);
    audio.race_index = None;
    assert_eq!(audio.race_music_pick().known(&discs), Some(MUSIC_TRACK));
}
