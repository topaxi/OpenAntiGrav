use super::*;

fn play(shuffle: &mut Shuffle, len: usize, count: usize) -> Vec<usize> {
    (0..count)
        .map(|_| shuffle.advance(len).expect("a playlist of songs"))
        .collect()
}

#[test]
fn every_song_plays_once_per_cycle() {
    let mut shuffle = Shuffle::new(7);
    shuffle.landed(3);
    let len = 9;
    let rest = play(&mut shuffle, len, len - 1);
    let mut seen: Vec<usize> = rest.clone();
    seen.push(3);
    seen.sort_unstable();
    assert_eq!(
        seen,
        (0..len).collect::<Vec<_>>(),
        "the other eight, then none twice"
    );
    for cycle in 0..5 {
        let mut songs = play(&mut shuffle, len, len);
        songs.sort_unstable();
        assert_eq!(songs, (0..len).collect::<Vec<_>>(), "cycle {cycle}");
    }
}

#[test]
fn a_cycle_never_opens_on_the_song_that_just_played() {
    for len in 2..7 {
        for seed in 1..200 {
            let mut shuffle = Shuffle::new(seed);
            let songs = play(&mut shuffle, len, len * 12);
            for pair in songs.windows(2) {
                assert_ne!(pair[0], pair[1], "len {len} seed {seed}: {songs:?}");
            }
        }
    }
}

#[test]
fn the_same_seed_plays_the_same_order_and_another_seed_a_different_one() {
    let order = |seed| {
        let mut shuffle = Shuffle::new(seed);
        play(&mut shuffle, 16, 48)
    };
    assert_eq!(order(11), order(11));
    assert_ne!(order(11), order(12));
    assert_ne!(
        order(11)[..16],
        (0..16).collect::<Vec<_>>(),
        "a shuffle is not the pool's own order"
    );
}

#[test]
fn a_peek_is_the_song_the_advance_plays_and_consumes_nothing() {
    let mut shuffle = Shuffle::new(5);
    shuffle.landed(0);
    for _ in 0..40 {
        let peeked = shuffle.peek_next(6);
        assert_eq!(
            shuffle.peek_next(6),
            peeked,
            "asking twice does not move it"
        );
        assert_eq!(shuffle.advance(6), peeked);
    }
}

#[test]
fn previous_walks_the_songs_played_and_forward_replays_them() {
    let mut shuffle = Shuffle::new(3);
    shuffle.landed(2);
    let played = play(&mut shuffle, 8, 4);
    assert_eq!(shuffle.previous(8), Some(played[2]));
    assert_eq!(shuffle.previous(8), Some(played[1]));
    assert_eq!(shuffle.current(), Some(played[1]));
    assert_eq!(
        shuffle.peek_next(8),
        Some(played[2]),
        "forward is where it came from"
    );
    assert_eq!(shuffle.advance(8), Some(played[2]));
    assert_eq!(shuffle.advance(8), Some(played[3]));
    let fresh = shuffle.advance(8).expect("past what was played");
    assert!(
        !played.contains(&fresh) && fresh != 2,
        "the deck goes on, no repeat of this cycle"
    );
    assert_eq!(shuffle.previous(8), Some(played[3]));
}

#[test]
fn previous_stops_at_the_start_of_what_is_remembered() {
    let mut shuffle = Shuffle::new(3);
    assert_eq!(shuffle.previous(8), None, "nothing played yet");
    shuffle.landed(4);
    assert_eq!(shuffle.previous(8), None, "one song played");
    shuffle.advance(8);
    assert!(shuffle.previous(8).is_some());
    assert_eq!(shuffle.previous(8), None);
}

#[test]
fn a_landing_this_did_not_choose_restarts_the_cycle_around_it() {
    let mut shuffle = Shuffle::new(9);
    shuffle.landed(1);
    let first = play(&mut shuffle, 5, 2);
    shuffle.landed(4);
    assert_eq!(shuffle.current(), Some(4));
    let mut after = play(&mut shuffle, 5, 4);
    after.push(4);
    after.sort_unstable();
    assert_eq!(
        after,
        vec![0, 1, 2, 3, 4],
        "the resumed song counts as played"
    );
    assert_eq!(first.len(), 2);
}

#[test]
fn landing_on_the_song_it_chose_moves_nothing() {
    let mut shuffle = Shuffle::new(9);
    shuffle.landed(0);
    let next = shuffle.advance(5).expect("a song");
    let peeked = shuffle.peek_next(5);
    shuffle.landed(next);
    assert_eq!(shuffle.peek_next(5), peeked, "the deck was not rebuilt");
}

#[test]
fn a_playlist_that_grows_plays_the_new_songs_before_the_cycle_ends() {
    let mut shuffle = Shuffle::new(21);
    shuffle.landed(0);
    play(&mut shuffle, 4, 1);
    let rest = play(&mut shuffle, 6, 5);
    let mut all = rest;
    all.push(0);
    all.sort_unstable();
    all.dedup();
    assert!(all.contains(&4) && all.contains(&5), "{all:?}");
}

#[test]
fn a_playlist_that_shrinks_never_plays_a_song_it_lost() {
    let mut shuffle = Shuffle::new(21);
    shuffle.landed(0);
    play(&mut shuffle, 10, 2);
    for song in play(&mut shuffle, 3, 12) {
        assert!(song < 3);
    }
}

#[test]
fn a_one_song_playlist_and_an_empty_one_do_not_hang() {
    let mut shuffle = Shuffle::new(1);
    assert_eq!(shuffle.advance(0), None);
    assert_eq!(play(&mut shuffle, 1, 3), vec![0, 0, 0]);
}

#[test]
fn the_order_parses_and_prints_its_settings_spelling() {
    assert_eq!(MusicOrder::default(), MusicOrder::Sequential);
    for order in MusicOrder::ALL {
        assert_eq!(order.name().parse::<MusicOrder>(), Ok(order));
    }
    assert_eq!("RANDOM".parse::<MusicOrder>(), Ok(MusicOrder::Random));
    assert!("shuffle".parse::<MusicOrder>().is_err());
}
