//! What the audio policy in [`super`] is asserted to do: the music-source
//! setting, pairing the two discs' soundtracks by length, seeking rather than
//! restarting on a source change, and the headless dump's clock.
//!
//! A file of its own because the tests exceed the 200 lines an inline module
//! may hold (`scripts/check-file-size.py`). Split further under the 1,000-line
//! rule: [`volumes`] (the four AUDIO rows of
//! [ADR-0027](../../../../docs/architecture/adr/0027-three-mix-buses.md)),
//! [`source_switch`] (the `MUSIC SOURCE` row's async fetch) and [`race_switch`]
//! (the race-voice/menu-voice handover).

use super::*;

mod no_audio;
mod race_switch;
mod source_switch;
mod volumes;

#[test]
fn a_music_source_round_trips_through_its_own_text() {
    for value in MusicSource::ALL {
        assert_eq!(value.to_string().parse::<MusicSource>(), Ok(value));
    }
    assert_eq!("PS2".parse::<MusicSource>(), Ok(MusicSource::Ps2));
    assert!("umd".parse::<MusicSource>().is_err());
    assert_eq!(MusicSource::default(), MusicSource::Auto);
    assert_eq!(MusicSource::Auto.platform(), None, "auto names no release");
}

/// The row is offered only when both discs are reachable, and `pick` is what an
/// unhonourable value falls through: `ps2` on a PSP-only machine plays the
/// PSP's music, not nothing.
#[test]
fn a_release_that_is_not_there_falls_back_to_the_booted_one() {
    let psp_only = MusicDiscs {
        library: &NoLibrary,
        psp: Some("psp.chd".into()),
        ps2: None,
        ps3: None,
        vita: None,
        ps4: None,
        booted: Some(Platform::Psp),
    };
    assert!(!psp_only.both(), "one disc is not a choice");
    for choice in MusicSource::ALL {
        assert_eq!(
            psp_only.pick(choice),
            Some(("psp.chd", Platform::Psp)),
            "{choice} on a machine with only the PSP disc"
        );
    }

    let both = MusicDiscs {
        library: &NoLibrary,
        psp: Some("psp.chd".into()),
        ps2: Some("ps2.chd".into()),
        ps3: None,
        vita: None,
        ps4: None,
        booted: Some(Platform::Psp),
    };
    assert!(both.both());
    assert_eq!(
        both.pick(MusicSource::Auto),
        Some(("psp.chd", Platform::Psp)),
        "auto is the booted disc, not the better one"
    );
    assert_eq!(
        both.pick(MusicSource::Ps2),
        Some(("ps2.chd", Platform::Ps2))
    );
    assert_eq!(
        both.pick(MusicSource::Psp),
        Some(("psp.chd", Platform::Psp))
    );

    // Neither release (an extracted directory of something else): no fallback.
    assert_eq!(MusicDiscs::default().pick(MusicSource::Auto), None);
}

/// Track lengths as a listing by position: a fixture, not how a disc addresses one.
fn lengths(seconds: &[f64]) -> Vec<Track> {
    seconds
        .iter()
        .enumerate()
        .map(|(index, seconds)| Track {
            at: index as u32,
            seconds: *seconds,
        })
        .collect()
}

/// The pairing rule on the real numbers: these sixteen lengths are the EU PS2
/// archive's, in its order, and the PSP lengths its sixteen large stereo
/// `Data.wad` entries in theirs, a different order, hence pairing by length.
#[test]
fn the_two_discs_soundtracks_pair_one_for_one_by_length() {
    let ps2 = [
        187.592, 195.789, 188.739, 193.550, 189.731, 187.814, 182.009, 177.226, 183.913, 202.632,
        200.624, 204.347, 194.013, 183.440, 182.143, 197.474,
    ];
    let psp = [
        187.581, 194.001, 188.728, 195.778, 193.538, 189.719, 187.803, 181.998, 177.215, 183.902,
        202.620, 200.612, 204.336, 183.429, 182.132, 197.462,
    ];
    let listing = Soundtrack {
        tracks: psp
            .iter()
            .enumerate()
            .map(|(index, seconds)| Track {
                at: index as u32,
                seconds: *seconds,
            })
            .collect(),
    };

    let mut matched: Vec<u32> = ps2
        .iter()
        .map(|seconds| {
            listing
                .nearest(*seconds)
                .unwrap_or_else(|| panic!("{seconds} s has no partner"))
                .at
        })
        .collect();
    matched.sort_unstable();
    assert_eq!(
        matched,
        (0..16).collect::<Vec<u32>>(),
        "every PSP track must be claimed exactly once"
    );

    // The pairing is not the identity (PS2 track 1 is the PSP's fourth in
    // `Data.wad` order), so indexing one archive by the other's order is wrong.
    assert_eq!(listing.nearest(ps2[1]).expect("a partner").at, 3);
    assert_eq!(listing.nearest(ps2[12]).expect("a partner").at, 1);
}

/// A different game's soundtrack must not be taken for the counterpart:
/// `pure-psp-eu.chd` carries the serial `UCES-00001`, which
/// `oag_assets::Layout::resolve` gives no verdict on by design, and was reported
/// as the PSP counterpart until a soundtrack had to pair.
///
/// Both listings are read values (Pulse's sixteen from the USA UMD, Pure's
/// nineteen from `pure-psp-eu.chd`, `fact` count over 44,100). Pure's shortest
/// is 205.2 s and Pulse's longest 204.3 s, inside [`PAIR_TOLERANCE`]: one track
/// matching is not the test, sixteen distinct partners is.
#[test]
fn another_games_soundtrack_does_not_pair() {
    let pulse = Soundtrack {
        tracks: lengths(&[
            187.581, 194.001, 188.728, 195.778, 193.538, 189.719, 187.803, 181.998, 177.215,
            183.902, 202.620, 200.612, 204.336, 183.429, 182.132, 197.462,
        ]),
    };
    let pure = Soundtrack {
        tracks: lengths(&[
            217.896, 213.693, 210.884, 205.217, 224.955, 220.667, 208.237, 219.103, 219.011,
            228.984, 209.816, 214.047, 213.862, 215.612, 227.857, 326.078, 213.240, 217.780,
            216.526,
        ]),
    };
    assert!(!pure.pairs_with(&pulse), "nineteen tracks is not sixteen");
    assert!(!pulse.pairs_with(&pure));

    // The near miss the count check catches first: Pure's shortest against
    // Pulse's longest, 0.87 s apart.
    assert!(
        (205.217f64 - 204.336).abs() < PAIR_TOLERANCE,
        "the two populations really do overlap within the tolerance"
    );

    // The PS2 side of the real pair must pass: every partner within 11.4 ms.
    let ps2 = Soundtrack {
        tracks: lengths(&[
            187.592, 195.789, 188.739, 193.550, 189.731, 187.814, 182.009, 177.226, 183.913,
            202.632, 200.624, 204.347, 194.013, 183.440, 182.143, 197.474,
        ]),
    };
    assert!(ps2.pairs_with(&pulse), "the two Pulse discs must pair");
    assert!(
        pulse.pairs_with(&ps2),
        "and it must not depend on the order"
    );

    // Sixteen tracks each within a second of the *same* Pulse track: passes a
    // per-track match, must fail a bijection.
    let all_alike = Soundtrack {
        tracks: lengths(&[187.6; 16]),
    };
    assert!(!all_alike.pairs_with(&pulse), "distinctness is the test");

    assert!(
        !Soundtrack { tracks: Vec::new() }.pairs_with(&Soundtrack { tracks: Vec::new() }),
        "two empty listings pair with nothing, not with each other"
    );
}

/// Nothing within a second is no answer, not the least bad one: the wrong three
/// minutes is worse than none.
#[test]
fn a_length_nothing_matches_pairs_with_nothing() {
    let listing = Soundtrack {
        tracks: vec![
            Track {
                at: 0,
                seconds: 187.5,
            },
            Track {
                at: 1,
                seconds: 204.3,
            },
        ],
    };
    assert!(listing.nearest(28.0).is_none(), "the front end's own music");
    assert_eq!(listing.nearest(187.511).expect("within tolerance").at, 0);
    assert!(
        listing.nearest(186.4).is_none(),
        "1.1 s out is a hundred times the observed error"
    );
}

/// Seek, do not restart: the row's headline constraint, measured through the
/// real pieces (a real mixer, two [`Sound`]s at the releases' actual rates,
/// pulled by [`Audio::tick`] at 60 Hz, swapped by [`Audio::set_music_source`]).
///
/// The rates are the point: 48,000 frames is one second on PS2 and 1.088 on PSP,
/// so carrying frames would land 8.8% out, two seconds adrift three minutes in.
/// No disc is read: both sounds go straight into [`Audio::held`], as a second
/// visit finds them.
#[test]
fn changing_the_music_source_seeks_rather_than_restarting() {
    // Booted from PS2, whose front-end music is a soundtrack track the row can
    // move (see `MusicSource` and the PSP-boot test below).
    let discs = MusicDiscs {
        library: &NoLibrary,
        psp: Some("psp.chd".into()),
        ps2: Some("ps2.chd".into()),
        ps3: None,
        vita: None,
        ps4: None,
        booted: Some(Platform::Ps2),
    };
    // Three minutes of silence at each release's rate: only the playhead is
    // measured, and the mixer advances it whatever the samples are.
    let psp = Arc::new(Sound::new(vec![0i16; 180 * 44_100 * 2], 2, 44_100).expect("a sound"));
    let ps2 = Arc::new(Sound::new(vec![0i16; 180 * 48_000 * 2], 2, 48_000).expect("a sound"));

    let mut audio = Audio {
        output: Output::null(DUMP_SAMPLE_RATE),
        dump: Some(Dump {
            path: PathBuf::from("unused"),
            samples: Vec::new(),
        }),
        music: None,
        music_attempted: false,
        music_from: None,
        held: vec![
            (Platform::Psp, Arc::clone(&psp)),
            (Platform::Ps2, Arc::clone(&ps2)),
        ],
        movie: None,
        race_voice: None,
        race_from: None,
        race_index: None,
        race_position: 0.0,
        race_cache: None,
        menu_sound: None,
        race_context: None,
        race_prefetch: None,
        source_switch: None,
        sfx: None,
        hd: Default::default(),
    };
    audio.start_music(&discs, MusicSource::Auto, Path::new("unused"));
    assert_eq!(
        audio.music_from,
        Some(Platform::Ps2),
        "auto is the booted release"
    );

    // Twenty seconds in, so a restart is unmistakable against a seek.
    for _ in 0..(60 * 20) {
        audio.tick();
    }
    let before = audio.playhead().expect("music is playing");
    assert!(
        (before - 20.0).abs() < 0.01,
        "expected 20 s of the PS2 track, got {before}"
    );

    audio.set_music_source(&discs, MusicSource::Psp, Path::new("unused"));
    assert_eq!(audio.music_from, Some(Platform::Psp), "it moved");
    let after = audio.playhead().expect("music is still playing");
    assert!(
        (after - before).abs() < 1.0 / 60.0,
        "the playhead moved from {before} s to {after} s; a swap must seek, not restart"
    );

    // And back: the return trip would expose a frame count carried across, as
    // the rate ratio inverts.
    for _ in 0..(60 * 5) {
        audio.tick();
    }
    let before = audio.playhead().expect("music is playing");
    audio.set_music_source(&discs, MusicSource::Ps2, Path::new("unused"));
    let after = audio.playhead().expect("music is still playing");
    assert!(
        (after - before).abs() < 1.0 / 60.0,
        "coming back: {before} s became {after} s"
    );

    // Choosing what is already playing does nothing, so nudging past a value
    // cannot restart the track.
    let before = audio.playhead().expect("music is playing");
    let voice = audio.music;
    audio.set_music_source(&discs, MusicSource::Auto, Path::new("unused"));
    assert_eq!(audio.music, voice, "the same voice, untouched");
    assert_eq!(audio.playhead(), Some(before));
}

/// Asked every tick, acted on once. The menu music's cue is the intro reel
/// ending and neither tick loop tracks that as an edge: both ask "still in a
/// movie state" each tick and call [`Audio::start_music`] when not, so the
/// second ask and the hundredth must leave the playing track alone.
///
/// The failure pinned is a *restart*, not a duplicate voice: a guard comparing
/// only voice ids would still re-read the disc, and a loop jumping to zero once
/// a second is the audible bug. Twenty seconds go on the playhead first so a
/// restart cannot hide.
#[test]
fn asking_for_the_music_again_never_restarts_it() {
    let discs = MusicDiscs {
        library: &NoLibrary,
        psp: Some("psp.chd".into()),
        ps2: Some("ps2.chd".into()),
        ps3: None,
        vita: None,
        ps4: None,
        booted: Some(Platform::Ps2),
    };
    let ps2 = Arc::new(Sound::new(vec![0i16; 180 * 48_000 * 2], 2, 48_000).expect("a sound"));
    let mut audio = Audio {
        output: Output::null(DUMP_SAMPLE_RATE),
        dump: Some(Dump {
            path: PathBuf::from("unused"),
            samples: Vec::new(),
        }),
        music: None,
        music_attempted: false,
        music_from: None,
        held: vec![(Platform::Ps2, Arc::clone(&ps2))],
        movie: None,
        race_voice: None,
        race_from: None,
        race_index: None,
        race_position: 0.0,
        race_cache: None,
        menu_sound: None,
        race_context: None,
        race_prefetch: None,
        source_switch: None,
        sfx: None,
        hd: Default::default(),
    };

    audio.start_music(&discs, MusicSource::Auto, Path::new("unused"));
    let voice = audio.music.expect("the held track plays");
    for _ in 0..(60 * 20) {
        audio.tick();
    }

    for _ in 0..600 {
        audio.start_music(&discs, MusicSource::Auto, Path::new("unused"));
    }
    assert_eq!(audio.music, Some(voice), "the same voice throughout");
    let at = audio.playhead().expect("music is playing");
    assert!(
        (at - 20.0).abs() < 0.01,
        "ten seconds of asking must not move the playhead: {at} s"
    );
}

/// The same guard where it would cost something: a source whose music will not
/// load leaves no voice, so "is a voice playing" cannot mean "already tried".
/// Without [`Audio::music_attempted`] a tick loop would re-open a disc, re-run
/// `ffmpeg` and re-print the failure sixty times a second.
#[test]
fn a_source_with_no_music_is_not_retried_every_tick() {
    let nothing = MusicDiscs {
        library: &NoLibrary,
        psp: None,
        ps2: None,
        ps3: None,
        vita: None,
        ps4: None,
        booted: None,
    };
    let mut audio = Audio {
        output: Output::null(DUMP_SAMPLE_RATE),
        dump: None,
        music: None,
        music_attempted: false,
        music_from: None,
        held: Vec::new(),
        movie: None,
        race_voice: None,
        race_from: None,
        race_index: None,
        race_position: 0.0,
        race_cache: None,
        menu_sound: None,
        race_context: None,
        race_prefetch: None,
        source_switch: None,
        sfx: None,
        hd: Default::default(),
    };
    audio.start_music(&nothing, MusicSource::Auto, Path::new("unused"));
    assert!(audio.music.is_none(), "there was nothing to play");
    assert!(audio.music_attempted, "and it has now been tried");

    // A second ask with a release whose track is already in hand and certain to
    // load is declined anyway: the first ask is the only one.
    let discs = MusicDiscs {
        library: &NoLibrary,
        psp: None,
        ps2: Some("ps2.chd".into()),
        ps3: None,
        vita: None,
        ps4: None,
        booted: Some(Platform::Ps2),
    };
    audio.held.push((
        Platform::Ps2,
        Arc::new(Sound::new(vec![0i16; 48_000 * 2], 2, 48_000).expect("a sound")),
    ));
    audio.start_music(&discs, MusicSource::Auto, Path::new("unused"));
    assert!(audio.music.is_none(), "the second ask does nothing at all");
}

/// The scope of the row: the PSP front end's music is not one of the sixteen and
/// has no PS2 counterpart, so no value of MUSIC SOURCE may touch it, even to the
/// release it already is (it would be swapped for an unrelated soundtrack track
/// and seeked into). A voice with no [`Audio::music_from`] is that case: all
/// three values leave the same voice at the same playhead on the same 28-second
/// loop.
#[test]
fn music_with_no_counterpart_is_left_alone_whatever_the_row_says() {
    let discs = MusicDiscs {
        library: &NoLibrary,
        psp: Some("psp.chd".into()),
        ps2: Some("ps2.chd".into()),
        ps3: None,
        vita: None,
        ps4: None,
        booted: Some(Platform::Psp),
    };
    // The PSP front end's music: 28 seconds, never stamped with a release.
    let front_end = Arc::new(Sound::new(vec![0i16; 28 * 44_100 * 2], 2, 44_100).expect("a sound"));
    let ps2 = Arc::new(Sound::new(vec![0i16; 180 * 48_000 * 2], 2, 48_000).expect("a sound"));

    let mut audio = Audio {
        output: Output::null(DUMP_SAMPLE_RATE),
        dump: Some(Dump {
            path: PathBuf::from("unused"),
            samples: Vec::new(),
        }),
        music: None,
        music_attempted: false,
        music_from: None,
        // The PS2 track is there to be chosen and must not be, so this cannot
        // pass by the swap finding nothing.
        held: vec![(Platform::Ps2, Arc::clone(&ps2))],
        movie: None,
        race_voice: None,
        race_from: None,
        race_index: None,
        race_position: 0.0,
        race_cache: None,
        menu_sound: None,
        race_context: None,
        race_prefetch: None,
        source_switch: None,
        sfx: None,
        hd: Default::default(),
    };
    audio.music = audio
        .output
        .with_mixer(|mixer| mixer.play(Play::looping(Arc::clone(&front_end), Bus::Music)));
    assert!(audio.music.is_some(), "a free voice");
    assert_eq!(audio.music_from, None, "not one of the sixteen");

    for _ in 0..(60 * 5) {
        audio.tick();
    }
    let voice = audio.music;
    let before = audio.playhead().expect("music is playing");

    for choice in MusicSource::ALL {
        audio.set_music_source(&discs, choice, Path::new("unused"));
        assert_eq!(
            audio.music, voice,
            "{choice} restarted the front end's music"
        );
        assert_eq!(
            audio.playhead(),
            Some(before),
            "{choice} moved the playhead"
        );
        assert_eq!(
            audio.music_from, None,
            "{choice} claimed it as a soundtrack"
        );
    }
}

/// The dump's length must be a function of the tick count alone, the claim
/// `--dump-audio` makes: a wall clock in the path would show as a count that
/// moves between runs.
#[test]
fn a_dump_is_exactly_as_long_as_the_ticks_it_was_given() {
    let mut audio = Audio {
        output: Output::null(DUMP_SAMPLE_RATE),
        dump: Some(Dump {
            path: PathBuf::from("unused"),
            samples: Vec::new(),
        }),
        music: None,
        music_attempted: false,
        music_from: None,
        held: Vec::new(),
        movie: None,
        race_voice: None,
        race_from: None,
        race_index: None,
        race_position: 0.0,
        race_cache: None,
        menu_sound: None,
        race_context: None,
        race_prefetch: None,
        source_switch: None,
        sfx: None,
        hd: Default::default(),
    };
    for _ in 0..120 {
        audio.tick();
    }
    let dump = audio.dump.as_ref().expect("the dump is set");
    let frames = dump.samples.len() / 2;
    assert_eq!(frames, 120 * (DUMP_SAMPLE_RATE as usize / 60));
}

/// The A/V sync measurement, the only one possible without listening: over a
/// full 40-second reel, does the frame the picture is on stay within one frame
/// of where the sound has got to?
///
/// Run through the real pieces (a real [`Sound`] in a real mixer pulled by
/// [`Audio::tick`] at 60 Hz, the playhead read where both tick loops read it and
/// given to [`oag_ui::frontend::Player::follow`]). Drift is cumulative, so 2,402
/// ticks, the whole intro. The bound is one frame (33 ms of picture against
/// 44,100 samples a second); the error is a floor, so the frame is at worst the
/// one before the sound's, never after.
#[test]
fn the_picture_stays_within_a_frame_of_the_sound_for_a_whole_reel() {
    let seconds = 40.17;
    let rate = 44_100;
    let frames = (seconds * f64::from(rate)) as usize;
    // Silence is fine: only the playhead is measured.
    let sound = Sound::new(vec![0i16; frames * 2], 2, rate).expect("a sound");

    let mut audio = Audio {
        output: Output::null(DUMP_SAMPLE_RATE),
        dump: Some(Dump {
            path: PathBuf::from("unused"),
            samples: Vec::new(),
        }),
        music: None,
        music_attempted: false,
        music_from: None,
        held: Vec::new(),
        movie: None,
        race_voice: None,
        race_from: None,
        race_index: None,
        race_position: 0.0,
        race_cache: None,
        menu_sound: None,
        race_context: None,
        race_prefetch: None,
        source_switch: None,
        sfx: None,
        hd: Default::default(),
    };
    assert!(audio.start_movie(sound), "a free voice");

    let (num, den) = oag_ui::frontend::FRAME_RATE;
    let mut player = oag_ui::frontend::Player::new(1200, false, oag_ui::frontend::FRAME_RATE);
    let mut worst = 0.0f64;

    for tick in 0..(60 * 41) {
        let playhead = audio.movie_playhead().expect("a sounding voice");
        player.follow(playhead);
        audio.tick();

        if player.is_finished() {
            break;
        }
        // Where the sound says the picture should be, unrounded.
        let wanted = playhead * num as f64 / den as f64;
        let error = wanted - player.frame() as f64;
        assert!(
            (0.0..1.0).contains(&error),
            "tick {tick}: the picture is {error} frames from the sound"
        );
        worst = worst.max(error);
    }

    assert!(worst > 0.0, "the reel should actually have played");
}

/// The clock rule's failure mode: a run with neither a device nor a dump never
/// advances the mixer, so a movie paced against it would stop on frame one and
/// the boot sequence never end; "is a voice playing" is not enough.
#[test]
fn a_mixer_that_is_never_advanced_offers_no_clock() {
    let mut audio = Audio {
        output: Output::null(DUMP_SAMPLE_RATE),
        dump: None,
        music: None,
        music_attempted: false,
        music_from: None,
        held: Vec::new(),
        movie: None,
        race_voice: None,
        race_from: None,
        race_index: None,
        race_position: 0.0,
        race_cache: None,
        menu_sound: None,
        race_context: None,
        race_prefetch: None,
        source_switch: None,
        sfx: None,
        hd: Default::default(),
    };
    let sound = Sound::new(vec![0i16; 44_100 * 2], 2, 44_100).expect("a sound");
    assert!(audio.start_movie(sound), "a free voice");

    assert_eq!(
        audio.movie_playhead(),
        None,
        "a voice on a mixer nothing pulls from is not a clock"
    );
}

/// A movie with no sound is tick-clocked, as `Backdrop.PMF`, the movie that
/// plays most.
#[test]
fn a_movie_with_no_voice_has_no_playhead() {
    let mut audio = Audio {
        output: Output::null(DUMP_SAMPLE_RATE),
        dump: Some(Dump {
            path: PathBuf::from("unused"),
            samples: Vec::new(),
        }),
        music: None,
        music_attempted: false,
        music_from: None,
        held: Vec::new(),
        movie: None,
        race_voice: None,
        race_from: None,
        race_index: None,
        race_position: 0.0,
        race_cache: None,
        menu_sound: None,
        race_context: None,
        race_prefetch: None,
        source_switch: None,
        sfx: None,
        hd: Default::default(),
    };
    assert_eq!(audio.movie_playhead(), None, "nothing started");

    let sound = Sound::new(vec![0i16; 4], 2, 44_100).expect("a sound");
    assert!(audio.start_movie(sound));
    assert_eq!(audio.movie_playhead(), Some(0.0));

    audio.stop_movie();
    assert_eq!(audio.movie_playhead(), None, "and none once it is stopped");
}

/// With no dump asked for nothing is accumulated: a windowed run must not grow
/// a buffer nobody reads.
#[test]
fn a_run_with_no_dump_accumulates_nothing() {
    let mut audio = Audio {
        output: Output::null(DUMP_SAMPLE_RATE),
        dump: None,
        music: None,
        music_attempted: false,
        music_from: None,
        held: Vec::new(),
        movie: None,
        race_voice: None,
        race_from: None,
        race_index: None,
        race_position: 0.0,
        race_cache: None,
        menu_sound: None,
        race_context: None,
        race_prefetch: None,
        source_switch: None,
        sfx: None,
        hd: Default::default(),
    };
    for _ in 0..120 {
        audio.tick();
    }
    assert!(audio.dump.is_none());
}

/// `next_race_index` in isolation: the playlist's real length needs a disc
/// [`Audio::booted_soundtrack_len`] cannot fabricate in a unit test.
#[test]
fn the_race_index_wraps_at_the_soundtracks_length() {
    assert_eq!(next_race_index(0, 16), 1);
    assert_eq!(next_race_index(15, 16), 0, "wraps back to the first track");
    assert_eq!(
        next_race_index(5, 0),
        5,
        "an unreadable soundtrack leaves the index where it was"
    );
}
/// Silence at `seconds` long, for tests that only care where the playhead gets to.
fn silence(seconds: f64, channels: u16, rate: u32) -> Arc<Sound> {
    let frames = (seconds * f64::from(rate)) as usize;
    Arc::new(Sound::new(vec![0i16; frames * channels as usize], channels, rate).expect("a sound"))
}

/// A PSP-boot-shaped fixture: the menu plays its own 28-second loop
/// (`music_from: None`) and a race track is already decoded and cached, so
/// [`Audio::start_race_music`] never touches a disc (the technique of
/// [`changing_the_music_source_seeks_rather_than_restarting`]).
fn psp_boot_fixture() -> (Audio, Arc<Sound>, Arc<Sound>) {
    let menu_sound = silence(28.0, 2, 44_100);
    let race_sound = silence(180.0, 2, 44_100);
    let mut audio = Audio {
        output: Output::null(DUMP_SAMPLE_RATE),
        dump: Some(Dump {
            path: PathBuf::from("unused"),
            samples: Vec::new(),
        }),
        music: None,
        music_attempted: true,
        music_from: None,
        held: Vec::new(),
        movie: None,
        race_voice: None,
        race_from: None,
        race_index: None,
        race_position: 0.0,
        race_cache: Some((Platform::Psp, MUSIC_TRACK, Arc::clone(&race_sound))),
        menu_sound: Some(Arc::clone(&menu_sound)),
        race_context: None,
        race_prefetch: None,
        source_switch: None,
        sfx: None,
        hd: Default::default(),
    };
    audio.music = audio
        .output
        .with_mixer(|mixer| mixer.play(Play::looping(Arc::clone(&menu_sound), Bus::Music)));
    (audio, menu_sound, race_sound)
}
