//! What movie playback in [`super`] is asserted to do: the frame clock and its
//! audio-following form, the cache lookup and transcode planning, and the ring
//! of decoded frames.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `movie.rs`: the tests are past the 200 lines an inline test module may
//! hold. See `scripts/check-file-size.py`, which is the rule as a gate. The
//! audio-track tests moved with `movie_audio` to `movie/track/tests.rs` when
//! that code split out into its own file - what is left here is everything
//! else `movie.rs` still owns.

use super::*;
use oag_ui::frontend::Player;

/// One frame at 30000/1001 Hz, to the nanosecond.
const FRAME: f64 = 1001.0 / 30_000.0;

#[test]
fn advances_one_frame_per_period() {
    let mut player = Player::new(100, false, FRAME_RATE);
    assert_eq!(player.frame(), 0);
    player.update(FRAME);
    assert_eq!(player.frame(), 1);
    player.update(FRAME * 2.0);
    assert_eq!(player.frame(), 3);
}

#[test]
fn a_short_step_does_not_advance_but_is_not_lost() {
    let mut player = Player::new(100, false, FRAME_RATE);
    player.update(FRAME / 2.0);
    assert_eq!(player.frame(), 0);
    player.update(FRAME / 2.0);
    assert_eq!(player.frame(), 1, "the remainder carries");
}

#[test]
fn pausing_holds_the_frame() {
    let mut player = Player::new(100, false, FRAME_RATE);
    player.update(FRAME * 10.0);
    player.pause();
    player.update(FRAME * 10.0);
    assert_eq!(player.frame(), 10);
    player.resume();
    player.update(FRAME);
    assert_eq!(player.frame(), 11);
}

#[test]
fn finishing_clamps_to_the_last_frame() {
    let mut player = Player::new(5, false, FRAME_RATE);
    player.update(FRAME * 100.0);
    assert!(player.is_finished());
    assert_eq!(player.frame(), 4);
}

#[test]
fn repeating_wraps_instead_of_finishing() {
    let mut player = Player::new(5, true, FRAME_RATE);
    player.update(FRAME * 5.0);
    assert!(!player.is_finished());
    assert_eq!(player.frame(), 0);
}

#[test]
fn an_empty_movie_is_finished_immediately() {
    let player = Player::new(0, false, FRAME_RATE);
    assert!(player.is_finished());
}

#[test]
fn frames_produced_counts_from_one() {
    let mut player = Player::new(300, false, FRAME_RATE);
    // Stepped one frame at a time, the way the game does, rather than in one
    // big jump: 259 steps from frame 0 lands on frame 259, the 260th frame.
    // This is what the intro's `260` counter compares against.
    for _ in 0..259 {
        player.update(FRAME);
    }
    assert_eq!(player.frame(), 259);
    assert_eq!(player.frames_produced(), 260);
}

/// The audio clock names the frame the picture should be on, which is the
/// whole of what [`Player::follow`] promises.
#[test]
fn following_the_audio_names_the_frame_that_second_belongs_to() {
    let mut player = Player::new(1200, false, FRAME_RATE);
    player.follow(0.0);
    assert_eq!(player.frame(), 0);
    // One second of sound is 29.97 frames of picture, so frame 29.
    player.follow(1.0);
    assert_eq!(player.frame(), 29);
    player.follow(10.0);
    assert_eq!(player.frame(), 299);
}

/// A slow tick moves the playhead several frames at once and the picture
/// has to jump rather than crawl - the opposite of an accumulator, and the
/// reason audio clocking cannot drift.
#[test]
fn a_jump_in_the_audio_takes_the_picture_with_it() {
    let mut player = Player::new(1200, false, FRAME_RATE);
    player.follow(0.5);
    player.follow(20.0);
    assert_eq!(player.frame(), 599);
    assert_eq!(player.position(), 599);
}

/// A [`Feed`] hands each position over once, so a playhead that went
/// backwards would ask for a frame the ring had already dropped and freeze
/// the picture.
#[test]
fn the_audio_clock_never_runs_the_picture_backwards() {
    let mut player = Player::new(1200, false, FRAME_RATE);
    player.follow(5.0);
    let position = player.position();
    player.follow(4.0);
    assert_eq!(player.position(), position, "a rewind is ignored");
    assert_eq!(player.frame(), position as usize);
}

/// A movie that does not repeat ends on the audio clock too, and clamps to
/// its last frame exactly as `update` does - the intro's `AutoRedirect`
/// depends on `is_finished` becoming true whichever clock got it there.
#[test]
fn following_past_the_end_finishes_and_clamps() {
    let mut player = Player::new(5, false, FRAME_RATE);
    player.follow(100.0);
    assert!(player.is_finished());
    assert_eq!(player.frame(), 4);
    assert_eq!(player.position(), 4);
}

/// The invariant a [`Feed`] shares with the player, on the other clock:
/// `position % frames == frame`, however the position was arrived at.
#[test]
fn following_wraps_a_repeating_movie_where_the_feed_wraps() {
    let mut player = Player::new(30, true, FRAME_RATE);
    for seconds in [0.0, 0.5, 1.0, 2.0, 3.5] {
        player.follow(seconds);
        assert_eq!(
            frame_at(player.position(), 30, true),
            Some(player.frame()),
            "the feed and the player disagree at {seconds}s"
        );
    }
    assert!(!player.is_finished());
}

/// The `--reel` leg holds the picture for two seconds at a time and the
/// sound cannot be held with it, so those seconds are discounted. Without
/// this the picture jumps sixty frames the instant the hold ends - which is
/// the whole of what audio clocking is supposed to prevent, arriving by a
/// different door.
#[test]
fn seconds_spent_holding_the_picture_are_discounted_from_the_audio_clock() {
    let mut player = Player::new(1200, false, FRAME_RATE);
    player.follow(1.0);
    assert_eq!(player.frame(), 29);

    // The hold: the caller keeps calling while the sound runs on.
    player.pause();
    for tick in 0..120 {
        player.follow(1.0 + f64::from(tick) / 60.0);
    }
    assert_eq!(player.frame(), 29, "the picture is held");

    player.resume();
    // Two seconds of sound went by, so the next frame is the next frame -
    // not the one sixty frames further on.
    player.follow(3.0 + 1.0 / 60.0);
    assert_eq!(
        player.frame(),
        29,
        "the hold is discounted rather than played through"
    );
    player.follow(4.0);
    assert_eq!(player.frame(), 59, "and a second on from there is a second");
}

#[test]
fn a_paused_player_ignores_the_audio_clock_too() {
    let mut player = Player::new(1200, false, FRAME_RATE);
    player.follow(1.0);
    player.pause();
    player.follow(20.0);
    assert_eq!(player.frame(), 29, "paused is paused on either clock");
    player.resume();
    player.follow(20.0);
    assert_eq!(player.frame(), 599);
}

/// **An uncapped conversion still knows how many frames it will make.**
///
/// The regression this pins: `Extent::Whole` used to become a bare `None`
/// that meant both "do not pass `-frames:v`" and "there is no total", so the
/// PS2's `.PSS` transcode reported frame numbers with no denominator and the
/// loading screen's bar stood still through the longest wait in the boot.
/// The container was measured before any of it started.
#[test]
fn an_uncapped_conversion_keeps_the_count_it_was_measured_at() {
    let whole = Frames::plan(Extent::Whole, 950);
    assert_eq!(
        whole.cap, None,
        "`-frames:v` must stay off, or the cache file is renamed and the \
         encode is truncated"
    );
    assert_eq!(
        whole.total,
        Some(950),
        "the denominator the progress report divides by"
    );

    let capped = Frames::plan(Extent::Frames(30), 950);
    assert_eq!(capped.cap, Some(30));
    assert_eq!(
        capped.total,
        Some(30),
        "a capped encode makes exactly its cap, so the two agree"
    );
}

/// **A cache file that is there is used; one that is not is not invented.**
///
/// [`cached`] is what makes an existing cache file beat the platform decoder
/// with no flag, so the two ways it can be wrong both matter: a false miss
/// sends every boot back through GStreamer for 4.80 s, and a false hit hands
/// back a file that is not this movie. It answers from the name
/// [`cache_name`] builds and from opening the container, and never converts.
#[test]
fn a_cache_lookup_finds_only_a_file_that_is_really_there() {
    let dir = std::env::temp_dir().join("oag-cached-unit");
    std::fs::create_dir_all(&dir).expect("the temp directory");
    let to = Conversion {
        key: "0badcafe-1234",
        cache_dir: &dir,
        width: 480,
        height: 272,
        refresh: false,
        watch: None,
    };

    assert!(
        cached(to, Frames::capped(1200)).is_none(),
        "nothing is cached under a key nothing has ever written"
    );
    assert!(
        cached(to, Frames::whole(950)).is_none(),
        "and an uncapped lookup does not find one either"
    );

    // A file at the right name that is not a decodable container is a miss,
    // not a hit and not an error: `FrameStore::open` is the check.
    let name = cache_name(to.key, to.width, to.height, Some(1200));
    std::fs::write(dir.join(&name), b"not an IVF file").expect("writing the decoy");
    assert!(
        cached(to, Frames::capped(1200)).is_none(),
        "{name} opened as a movie, which it is not"
    );

    assert!(
        cached(
            Conversion {
                refresh: true,
                ..to
            },
            Frames::capped(1200)
        )
        .is_none(),
        "refresh answers None to everything - that is the whole of the flag"
    );

    std::fs::remove_file(dir.join(&name)).expect("cleaning up");
}

#[test]
fn a_frame_limit_caps_at_what_exists() {
    assert_eq!(Extent::Frames(261).limit(1200), 261);
    assert_eq!(Extent::Frames(261).limit(100), 100);
    assert_eq!(Extent::Whole.limit(1200), 1200);
}

/// The invariant [`Feed`] depends on: the player's position and the feed's
/// agree about which frame is which, across as many wraps as you like.
#[test]
fn position_counts_through_a_wrap_and_the_frame_follows_it() {
    let mut player = Player::new(5, true, FRAME_RATE);
    for expected in 1..=12u64 {
        player.update(FRAME);
        assert_eq!(player.position(), expected);
        assert_eq!(
            player.frame(),
            (expected % 5) as usize,
            "position {expected} names the wrong frame"
        );
        assert_eq!(
            frame_at(player.position(), 5, true),
            Some(player.frame()),
            "the feed and the player disagree at position {expected}"
        );
    }
}

#[test]
fn a_movie_that_does_not_repeat_has_position_equal_to_frame() {
    let mut player = Player::new(5, false, FRAME_RATE);
    for _ in 0..20 {
        player.update(FRAME);
        assert_eq!(player.position(), player.frame() as u64);
    }
    // Clamped with the frame, rather than counting on past the end.
    assert!(player.is_finished());
    assert_eq!(player.position(), 4);
}

#[test]
fn pausing_holds_the_position_too() {
    let mut player = Player::new(100, true, FRAME_RATE);
    player.update(FRAME * 3.0);
    player.pause();
    player.update(FRAME * 50.0);
    assert_eq!(player.position(), 3);
}

#[test]
fn a_loop_maps_positions_round_and_round() {
    assert_eq!(frame_at(0, 270, true), Some(0));
    assert_eq!(frame_at(269, 270, true), Some(269));
    // The wrap: one past the last frame is the first one again.
    assert_eq!(frame_at(270, 270, true), Some(0));
    assert_eq!(frame_at(271, 270, true), Some(1));
    assert_eq!(frame_at(270 * 4 + 7, 270, true), Some(7));
}

#[test]
fn a_movie_that_does_not_repeat_runs_out() {
    assert_eq!(frame_at(0, 3, false), Some(0));
    assert_eq!(frame_at(2, 3, false), Some(2));
    assert_eq!(frame_at(3, 3, false), None);
    assert_eq!(frame_at(9_999, 3, false), None);
}

/// An empty cache has no frame at any position, whichever way it is played.
/// Without this the modulo below would divide by zero.
#[test]
fn an_empty_movie_has_no_frame_anywhere() {
    assert_eq!(frame_at(0, 0, true), None);
    assert_eq!(frame_at(0, 0, false), None);
}

fn frame(position: u64) -> Frame {
    Frame {
        position,
        index: position as usize,
        // One byte, standing in for a picture: the ring does not read them.
        picture: VideoFrame {
            bytes: vec![position as u8],
            ..VideoFrame::default()
        },
    }
}

fn ring(positions: impl IntoIterator<Item = u64>) -> Ring {
    let mut ring = Ring::new(LOOKAHEAD);
    for position in positions {
        ring.push(frame(position));
    }
    ring
}

#[test]
fn an_empty_ring_has_nothing_to_show() {
    assert!(Ring::new(LOOKAHEAD).take_upto(0).is_none());
    assert!(Ring::new(LOOKAHEAD).take_upto(9_999).is_none());
}

#[test]
fn the_ring_hands_frames_over_in_order() {
    let mut ring = ring(0..3);
    for expected in 0..3 {
        assert_eq!(ring.take_upto(expected).map(|f| f.position), Some(expected));
    }
    assert!(ring.take_upto(2).is_none(), "and only once each");
}

/// Taking is a **pop**, and this is the invariant both front-end transition
/// defects rested on.
///
/// A consumer that asks for a position, uses the frame or throws it away,
/// and then asks for the same position again gets nothing the second time:
/// the ring has no memory of what it handed over. That is correct here -
/// popping is what frees a slot for the worker to decode into - so the
/// caller is the one that has to keep the picture. `FrontendStage` does,
/// in `held_backdrop`; before it did, the frame popped on a frame that drew
/// the intro was gone by the time `Show Logo` wanted to draw it.
#[test]
fn a_frame_taken_is_a_frame_gone_even_at_the_same_position() {
    let mut ring = ring(0..3);
    assert_eq!(ring.take_upto(1).map(|f| f.position), Some(1));
    assert!(
        ring.take_upto(1).is_none(),
        "the same position asked twice hands nothing over the second time"
    );
    assert_eq!(
        ring.take_upto(2).map(|f| f.position),
        Some(2),
        "and the ring has moved on rather than been emptied"
    );
}

/// The playhead being ahead of the decoder is the ordinary underrun, and the
/// answer is *nothing* rather than a stale frame or a wrong one: the caller
/// keeps the picture it has.
#[test]
fn a_playhead_past_everything_decoded_gets_the_newest_there_is() {
    let mut ring = ring(0..3);
    assert_eq!(ring.take_upto(100).map(|f| f.position), Some(2));
    assert!(ring.take_upto(100).is_none());
}

/// A frame slow enough to move the playhead several frames on must not
/// replay the frames it skipped. This is the case a plain channel `recv`
/// gets wrong, and it is why the ring is a ring.
#[test]
fn a_jump_forwards_drops_what_it_skipped_and_keeps_the_newest() {
    let mut ring = ring(0..4);
    let taken = ring.take_upto(2).expect("frame 2 is there");
    assert_eq!(taken.position, 2, "the newest at or before, not the oldest");
    // 0 and 1 are gone rather than queued up behind it.
    assert_eq!(ring.take_upto(3).map(|f| f.position), Some(3));
    assert!(ring.take_upto(3).is_none());
}

/// The playhead sitting on a frame older than anything the ring holds - the
/// shape a missed [`Feed::restart`] would produce. Nothing comes out, so the
/// picture freezes rather than jumping to the wrong frame.
#[test]
fn a_playhead_behind_the_ring_gets_nothing() {
    let mut ring = ring(10..13);
    assert!(ring.take_upto(0).is_none());
    assert!(ring.take_upto(9).is_none());
    assert_eq!(ring.take_upto(10).map(|f| f.position), Some(10));
}

#[test]
fn the_ring_is_full_at_the_lookahead_and_empties_as_it_is_read() {
    let mut ring = ring(0..LOOKAHEAD as u64);
    assert!(ring.is_full(), "the worker should park here");
    ring.take_upto(0);
    assert!(!ring.is_full(), "and wake once a frame is taken");
}

#[test]
fn clearing_the_ring_leaves_nothing_behind() {
    let mut ring = ring(0..3);
    ring.clear();
    assert!(ring.take_upto(u64::MAX).is_none());
    assert!(!ring.is_full());
}

/// A decoder that, like the browser's, has each frame only after being asked
/// `waits` times: [`Pending`] until then.
#[derive(Debug)]
struct Asynchronous {
    waits: usize,
    asked: usize,
    len: usize,
}

impl VideoDecoder for Asynchronous {
    fn label(&self) -> &'static str {
        "asynchronous"
    }

    fn geometry(&self) -> av1::Geometry {
        av1::Geometry::new(2, 2)
    }

    fn len(&self) -> usize {
        self.len
    }

    fn frame(&mut self, index: usize, out: &mut VideoFrame) -> Result<()> {
        if self.asked < self.waits {
            self.asked += 1;
            return Err(Pending.into());
        }
        self.asked = 0;
        out.bytes = vec![index as u8; 6];
        Ok(())
    }

    fn rewind(&mut self) {}
}

fn polled(waits: usize, len: usize) -> (Shared, FrameStore) {
    let shared = Shared {
        state: Mutex::new(State {
            ring: Ring::new(LOOKAHEAD),
            next: 0,
            epoch: 0,
            error: None,
            failed: false,
            done: false,
            stop: false,
        }),
        wake: Condvar::new(),
    };
    let store = FrameStore {
        path: PathBuf::from("asynchronous"),
        source: Box::new(Asynchronous {
            waits,
            asked: 0,
            len,
        }),
        len,
        luma_len: 4,
        chroma_len: 1,
        chroma_width: 1,
        chroma_height: 1,
    };
    (shared, store)
}

fn positions(shared: &Shared) -> Vec<u64> {
    let state = shared.state.lock().unwrap();
    state
        .ring
        .slots
        .iter()
        .map(|frame| frame.position)
        .collect()
}

#[test]
fn a_pending_frame_is_retried_on_the_next_poll_and_never_fails_the_feed() {
    let (shared, mut store) = polled(2, 10);
    pump(&shared, &mut store, 10, true);
    assert!(positions(&shared).is_empty(), "nothing out after one poll");
    pump(&shared, &mut store, 10, true);
    pump(&shared, &mut store, 10, true);
    assert_eq!(positions(&shared), [0]);
    for _ in 0..20 {
        pump(&shared, &mut store, 10, true);
    }
    assert_eq!(positions(&shared), [0, 1, 2, 3], "in order, up to the ring");
    let state = shared.state.lock().unwrap();
    assert!(!state.failed && state.error.is_none());
}

#[test]
fn a_polled_feed_restarts_at_position_zero_and_wraps_a_repeating_movie() {
    let (shared, mut store) = polled(0, 3);
    pump(&shared, &mut store, 3, true);
    let taken = shared.state.lock().unwrap().ring.take_upto(3).unwrap();
    assert_eq!(
        (taken.position, taken.index),
        (3, 0),
        "position 3 is frame 0 again"
    );
    {
        let mut state = shared.state.lock().unwrap();
        state.ring.clear();
        state.next = 0;
        state.epoch += 1;
    }
    pump(&shared, &mut store, 3, true);
    assert_eq!(positions(&shared), [0, 1, 2, 3]);
}
