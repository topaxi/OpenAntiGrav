//! What the audio policy in [`super`] is asserted to do: the volume and
//! music-source settings, pairing the two discs' soundtracks by length,
//! seeking rather than restarting when the source changes, the menu-to-race
//! handover, and the headless dump's clock.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `audio.rs`: the tests are 946 lines, past the 200 an inline test
//! module may hold. See `scripts/check-file-size.py`, which is the rule as a
//! gate.

use super::*;

#[test]
fn a_volume_round_trips_through_its_own_text() {
    for value in Volume::OFFERED {
        assert_eq!(value.to_string().parse::<Volume>(), Ok(value));
    }
}

#[test]
fn a_volume_outside_the_range_is_refused() {
    assert!("101".parse::<Volume>().is_err());
    assert!("-1".parse::<Volume>().is_err());
    assert_eq!("0".parse::<Volume>(), Ok(Volume(0)));
}

#[test]
fn full_volume_is_unattenuated() {
    assert_eq!(Volume::FULL.gain(), 1.0);
    assert_eq!(Volume::default(), Volume::FULL);
    assert_eq!(Volume(0).gain(), 0.0);
}

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

/// The row is offered only when both discs are reachable, and `pick` is
/// what a value the machine cannot honour falls through: a settings file
/// saying `ps2`, carried onto a machine that has only the PSP disc, has to
/// play the PSP's music rather than nothing.
#[test]
fn a_release_that_is_not_there_falls_back_to_the_booted_one() {
    let psp_only = MusicDiscs {
        psp: Some("psp.chd".into()),
        ps2: None,
        ps3: None,
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
        psp: Some("psp.chd".into()),
        ps2: Some("ps2.chd".into()),
        ps3: None,
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

    // A source that is neither release - an extracted directory of
    // something else - has nothing to fall back to and says so.
    assert_eq!(MusicDiscs::default().pick(MusicSource::Auto), None);
}

/// Track lengths as a listing, addressed by position - a test fixture, not
/// how a disc addresses one.
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

/// The pairing rule, on the real numbers. These sixteen lengths are the
/// EU PS2 archive's, in its own order, and the PSP lengths are its sixteen
/// large stereo `Data.wad` entries in *theirs* - which is a different order,
/// and the whole reason a length is what pairs them.
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

    // And the pairing is not the identity, which is the thing that would
    // make indexing one archive by the other's order silently wrong: PS2
    // track 1 is the PSP's fourth in `Data.wad` order.
    assert_eq!(listing.nearest(ps2[1]).expect("a partner").at, 3);
    assert_eq!(listing.nearest(ps2[12]).expect("a partner").at, 1);
}

/// A *different game's* soundtrack must not be taken for the counterpart,
/// and this is the case that made the check necessary rather than
/// hypothetical: `pure-psp-eu.chd` carries the serial `UCES-00001`, which
/// `oag_assets::Layout::resolve` gives no verdict on by design, and
/// was reported as the PSP counterpart until a soundtrack had to pair.
///
/// Both listings are read values - Pulse's sixteen from the USA UMD, Pure's
/// nineteen from `pure-psp-eu.chd`, each the `fact` count over 44,100. Note
/// how close the two populations come: Pure's shortest is 205.2 s and
/// Pulse's longest 204.3 s, which is *inside* [`PAIR_TOLERANCE`]. One track
/// matching is not the test; sixteen distinct partners is.
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

    // The near miss the count check catches first, isolated: Pure's
    // shortest against Pulse's longest, 0.87 s apart.
    assert!(
        (205.217f64 - 204.336).abs() < PAIR_TOLERANCE,
        "the two populations really do overlap within the tolerance"
    );

    // The PS2 side of the real pair, which does have to pass. Same
    // recordings, so every partner is within 11.4 ms.
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

    // Sixteen tracks that are each within a second of a Pulse track but
    // all of the *same* one: a listing that would pass a per-track match
    // and must fail a bijection.
    let all_alike = Soundtrack {
        tracks: lengths(&[187.6; 16]),
    };
    assert!(!all_alike.pairs_with(&pulse), "distinctness is the test");

    assert!(
        !Soundtrack { tracks: Vec::new() }.pairs_with(&Soundtrack { tracks: Vec::new() }),
        "two empty listings pair with nothing, not with each other"
    );
}

/// Nothing within a second is no answer at all, rather than the least bad
/// one. Playing the wrong three minutes of music is worse than playing
/// none, and it is what a misidentified population would produce.
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

/// **Seek, do not restart** - the row's headline constraint, measured
/// rather than asserted, and through the real pieces: a real mixer, a real
/// pair of [`Sound`]s at the two releases' actual rates, pulled by
/// [`Audio::tick`] at the fixed 60 Hz, swapped by the same
/// [`Audio::set_music_source`] a keypress calls.
///
/// The two rates are the point. 48,000 frames into the PS2's track is one
/// second and into the PSP's is 1.088, so a swap that carried *frames*
/// across would land 8.8% out - two seconds adrift three minutes in, which
/// is most of a bar. Carrying seconds lands where it started.
///
/// No disc is read: both sounds are put straight into [`Audio::held`],
/// which is exactly what a second visit to a release finds there.
#[test]
fn changing_the_music_source_seeks_rather_than_restarting() {
    // Booted from the PS2 release, because that is the one whose front-end
    // music is a soundtrack track and so the one the row can move. See
    // `MusicSource`, and the test below for the PSP boot.
    let discs = MusicDiscs {
        psp: Some("psp.chd".into()),
        ps2: Some("ps2.chd".into()),
        ps3: None,
        booted: Some(Platform::Ps2),
    };
    // Three minutes of silence at each release's own rate. What is measured
    // is where the playhead is, and the mixer advances it whatever the
    // samples are.
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

    // And back, from wherever it has got to by then - the return trip is
    // the one that would expose a frame count carried across, because the
    // rate ratio inverts.
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

    // Choosing what is already playing does nothing at all, so nudging the
    // row past a value it is already on cannot restart the track.
    let before = audio.playhead().expect("music is playing");
    let voice = audio.music;
    audio.set_music_source(&discs, MusicSource::Auto, Path::new("unused"));
    assert_eq!(audio.music, voice, "the same voice, untouched");
    assert_eq!(audio.playhead(), Some(before));
}

/// **Asked every tick, acted on once.** The menu music's cue is the intro
/// reel ending, and neither tick loop tracks that as an edge - both simply
/// ask "is the sequence still in a movie state" every tick and call
/// [`Audio::start_music`] when it is not. So the second ask, and the
/// hundredth, have to be free and have to leave the playing track alone.
///
/// The failure this pins down is not a duplicate voice but a *restart*: a
/// guard that only compared voice ids would still re-read the disc, and a
/// music loop that jumps back to zero once a second is the audible form of
/// the bug. Twenty seconds are put on the playhead first so a restart could
/// not hide.
#[test]
fn asking_for_the_music_again_never_restarts_it() {
    let discs = MusicDiscs {
        psp: Some("psp.chd".into()),
        ps2: Some("ps2.chd".into()),
        ps3: None,
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

/// The same guard, for the case that would actually cost something: a
/// source whose music will not load leaves no voice behind, so "is a voice
/// playing" is not a usable test for "has this already been tried". Without
/// [`Audio::music_attempted`] a tick loop would re-open a disc, re-run
/// `ffmpeg` and re-print the failure sixty times a second.
#[test]
fn a_source_with_no_music_is_not_retried_every_tick() {
    let nothing = MusicDiscs {
        psp: None,
        ps2: None,
        ps3: None,
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
    };
    audio.start_music(&nothing, MusicSource::Auto, Path::new("unused"));
    assert!(audio.music.is_none(), "there was nothing to play");
    assert!(audio.music_attempted, "and it has now been tried");

    // A second ask, this time with a release whose track is already in hand
    // and so certain to load. It is declined anyway: the first ask is the
    // only one, whatever it decided.
    let discs = MusicDiscs {
        psp: None,
        ps2: Some("ps2.chd".into()),
        ps3: None,
        booted: Some(Platform::Ps2),
    };
    audio.held.push((
        Platform::Ps2,
        Arc::new(Sound::new(vec![0i16; 48_000 * 2], 2, 48_000).expect("a sound")),
    ));
    audio.start_music(&discs, MusicSource::Auto, Path::new("unused"));
    assert!(audio.music.is_none(), "the second ask does nothing at all");
}

/// **The scope of the row, and the reason it has one.** The PSP front
/// end's own music is not one of the sixteen and has no counterpart on the
/// PS2 disc, so no value of MUSIC SOURCE may touch it - not even to the
/// release it already is. Left ungoverned it would be swapped for an
/// unrelated three-minute soundtrack track *and seeked into*, landing
/// twenty seconds inside a different piece of music.
///
/// A voice with no [`Audio::music_from`] is exactly that case, and the
/// assertion here is that all three values leave it alone: the same voice,
/// at the same playhead, on the same 28-second loop.
#[test]
fn music_with_no_counterpart_is_left_alone_whatever_the_row_says() {
    let discs = MusicDiscs {
        psp: Some("psp.chd".into()),
        ps2: Some("ps2.chd".into()),
        ps3: None,
        booted: Some(Platform::Psp),
    };
    // The PSP front end's own music: 28 seconds, not three minutes, and
    // never stamped with a release.
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
        // The PS2 track is *there to be chosen* and still must not be, so
        // this cannot pass by the swap merely failing to find anything.
        held: vec![(Platform::Ps2, Arc::clone(&ps2))],
        movie: None,
        race_voice: None,
        race_from: None,
        race_index: None,
        race_position: 0.0,
        race_cache: None,
        menu_sound: None,
        race_context: None,
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

/// The dump's length has to be a function of the tick count and nothing
/// else, because that is the whole claim `--dump-audio` makes: the same
/// control sequence renders the same file. A wall clock anywhere in the
/// path would show up here as a count that moves between runs.
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
    };
    for _ in 0..120 {
        audio.tick();
    }
    let dump = audio.dump.as_ref().expect("the dump is set");
    let frames = dump.samples.len() / 2;
    assert_eq!(frames, 120 * (DUMP_SAMPLE_RATE as usize / 60));
}

/// **The A/V sync measurement**, and the only one that can be made without
/// something to listen with: over a full 40-second reel, does the frame the
/// picture is on stay within one frame of where the sound has got to?
///
/// Run through the real pieces rather than a model of them - a real
/// [`Sound`] in a real mixer, pulled by [`Audio::tick`] at the fixed 60 Hz,
/// with the playhead read exactly where both tick loops read it and handed
/// to [`crate::movie::Player::follow`]. Drift is what audio clocking exists
/// to prevent and it is cumulative, so measuring it over one tick would
/// measure nothing; 2,402 ticks is the whole intro.
///
/// The bound is **one frame**, which is 33 ms of picture against 44,100
/// samples a second of sound. The error is a floor, so the frame is at
/// worst the one before the sound's own, never the one after.
#[test]
fn the_picture_stays_within_a_frame_of_the_sound_for_a_whole_reel() {
    let seconds = 40.17;
    let rate = 44_100;
    let frames = (seconds * f64::from(rate)) as usize;
    // Silence is fine: what is measured is where the playhead is, and the
    // mixer advances it whatever the samples are.
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
    };
    assert!(audio.start_movie(sound), "a free voice");

    let (num, den) = crate::movie::FRAME_RATE;
    let mut player = crate::movie::Player::new(1200, false, crate::movie::FRAME_RATE);
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

/// The clock rule's own failure mode, and the reason the predicate is not
/// just "is a voice playing": a run with **neither** a device nor a dump
/// never advances the mixer, so a movie paced against it would stop on
/// frame one and the boot sequence would never reach its end.
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
    };
    let sound = Sound::new(vec![0i16; 44_100 * 2], 2, 44_100).expect("a sound");
    assert!(audio.start_movie(sound), "a free voice");

    assert_eq!(
        audio.movie_playhead(),
        None,
        "a voice on a mixer nothing pulls from is not a clock"
    );
}

/// A movie with no sound is tick-clocked, which is `Backdrop.PMF` - the
/// movie that plays most, and the one this must not get wrong.
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
    };
    assert_eq!(audio.movie_playhead(), None, "nothing started");

    let sound = Sound::new(vec![0i16; 4], 2, 44_100).expect("a sound");
    assert!(audio.start_movie(sound));
    assert_eq!(audio.movie_playhead(), Some(0.0));

    audio.stop_movie();
    assert_eq!(audio.movie_playhead(), None, "and none once it is stopped");
}

/// With no dump asked for, nothing is accumulated at all - a windowed run
/// must not grow a buffer nobody ever reads.
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
    };
    for _ in 0..120 {
        audio.tick();
    }
    assert!(audio.dump.is_none());
}

/// `next_race_index` in isolation, since the playlist's real length can
/// only come from a disc [`Audio::booted_soundtrack_len`] cannot fabricate
/// in a unit test - this is the arithmetic side of "advances and wraps",
/// pinned without one.
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

/// Silence at `seconds` long, for tests that only care where the playhead
/// gets to, not what it sounds like.
fn silence(seconds: f64, channels: u16, rate: u32) -> Arc<Sound> {
    let frames = (seconds * f64::from(rate)) as usize;
    Arc::new(Sound::new(vec![0i16; frames * channels as usize], channels, rate).expect("a sound"))
}

/// A PSP-boot-shaped fixture: the menu plays its own 28-second loop, which
/// is not one of the sixteen soundtrack tracks (`music_from: None`), and a
/// race track is already decoded and cached - so [`Audio::start_race_music`]
/// never has to touch a disc, the same technique
/// [`changing_the_music_source_seeks_rather_than_restarting`] uses for the
/// menu path.
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
    };
    audio.music = audio
        .output
        .with_mixer(|mixer| mixer.play(Play::looping(Arc::clone(&menu_sound), Bus::Music)));
    (audio, menu_sound, race_sound)
}

/// Starting a race stops the menu voice - not just the field, the actual
/// sounding one - and starts a race voice on the playlist instead.
#[test]
fn starting_a_race_stops_the_menu_voice_and_starts_a_race_voice() {
    let (mut audio, _menu_sound, _race_sound) = psp_boot_fixture();
    let discs = MusicDiscs {
        psp: Some("psp.chd".into()),
        ps2: Some("ps2.chd".into()),
        ps3: None,
        booted: Some(Platform::Psp),
    };
    let menu_voice = audio.music.expect("the fixture starts the menu playing");

    audio.start_race_music(&discs, MusicSource::Auto, Path::new("unused"));

    assert!(
        !audio
            .output
            .with_mixer(|mixer| mixer.is_playing(menu_voice)),
        "the menu voice must actually stop, not just the field clear"
    );
    assert!(audio.music.is_none());
    assert!(audio.race_voice.is_some(), "a race track should be playing");
    assert_eq!(audio.race_from, Some(Platform::Psp));
    assert_eq!(
        audio.playhead_race(),
        Some(0.0),
        "a fresh start, not a seek"
    );
}

/// Ending a race saves the exact position it was cut off at, and the menu
/// voice sounds again - a *new* voice, silent until this moment, not one
/// that kept advancing somewhere unheard.
#[test]
fn ending_a_race_saves_the_position_and_the_menu_voice_sounds_again() {
    let (mut audio, _menu_sound, _race_sound) = psp_boot_fixture();
    let discs = MusicDiscs {
        psp: Some("psp.chd".into()),
        ps2: Some("ps2.chd".into()),
        ps3: None,
        booted: Some(Platform::Psp),
    };
    audio.start_race_music(&discs, MusicSource::Auto, Path::new("unused"));

    for _ in 0..(60 * 45) {
        audio.tick();
    }
    let at = audio.playhead_race().expect("the race track is playing");
    assert!((at - 45.0).abs() < 0.01, "expected 45 s in, got {at}");

    audio.pause_race_music();

    assert!(audio.race_voice.is_none());
    assert!(
        (audio.race_position - 45.0).abs() < 0.01,
        "expected the cut-off saved at 45 s, got {}",
        audio.race_position
    );
    assert_eq!(
        audio.playhead(),
        Some(0.0),
        "the menu voice is a fresh start, not the paused race's position"
    );
}

/// The direct analogue of
/// [`changing_the_music_source_seeks_rather_than_restarting`], for the
/// race path: leaving and re-entering a race resumes the same track close
/// to where it was cut off, not from the start.
#[test]
fn a_second_race_resumes_within_a_sixtieth_of_a_second_of_the_saved_position() {
    let (mut audio, _menu_sound, _race_sound) = psp_boot_fixture();
    let discs = MusicDiscs {
        psp: Some("psp.chd".into()),
        ps2: Some("ps2.chd".into()),
        ps3: None,
        booted: Some(Platform::Psp),
    };
    audio.start_race_music(&discs, MusicSource::Auto, Path::new("unused"));
    for _ in 0..(60 * 90) {
        audio.tick();
    }
    audio.pause_race_music();
    let saved = audio.race_position;

    audio.start_race_music(&discs, MusicSource::Auto, Path::new("unused"));
    let resumed = audio.playhead_race().expect("the race track is playing");

    assert!(
        (resumed - saved).abs() < 1.0 / 60.0,
        "expected to resume at {saved} s, landed at {resumed} s"
    );
}

/// `MUSIC SOURCE` moved while a race is live has to move the *race*
/// voice, seek-preserving - not no-op (both its guard fields would still
/// name the menu) and not silently desync `race_from` from what is
/// actually sounding.
#[test]
fn music_source_changed_while_a_race_is_live_moves_the_race_voice() {
    let discs = MusicDiscs {
        psp: Some("psp.chd".into()),
        ps2: Some("ps2.chd".into()),
        ps3: None,
        booted: Some(Platform::Ps2),
    };
    let psp_track = silence(180.0, 2, 44_100);
    let ps2_track = silence(180.0, 2, 48_000);

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
        race_from: Some(Platform::Ps2),
        race_index: Some(0),
        race_position: 0.0,
        // The *target* platform is already decoded - the same technique
        // `held` uses for the menu row, avoiding a disc read this test
        // has no fixture for. See `psp_boot_fixture`.
        race_cache: Some((Platform::Psp, 0, Arc::clone(&psp_track))),
        menu_sound: None,
        race_context: None,
    };
    audio.race_voice = audio
        .output
        .with_mixer(|mixer| mixer.play(Play::once(Arc::clone(&ps2_track), Bus::Music)));

    for _ in 0..(60 * 20) {
        audio.tick();
    }
    let before = audio.playhead_race().expect("the race track is playing");
    assert!((before - 20.0).abs() < 0.01);

    audio.set_music_source(&discs, MusicSource::Psp, Path::new("unused"));

    assert_eq!(audio.race_from, Some(Platform::Psp), "the row moved it");
    let after = audio.playhead_race().expect("still playing");
    assert!(
        (after - before).abs() < 1.0 / 60.0,
        "the row must seek the race voice, not restart it: {before} s became {after} s"
    );
}

/// A source with no decodable race music still resumes the menu cleanly
/// on pause - degrading the same way [`Audio::start_music`] does rather
/// than leaving a dangling voice or panicking on a `None` where the race
/// never actually started.
#[test]
fn a_source_with_no_decodable_race_music_still_resumes_menu_music_cleanly() {
    let discs = MusicDiscs {
        psp: Some("psp.chd".into()),
        ps2: None,
        ps3: None,
        booted: Some(Platform::Psp),
    };
    let menu_sound = silence(28.0, 2, 44_100);
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
        // Nothing cached and the fake PSP path carries no soundtrack, so
        // `start_race_music` cannot decode anything - the source has no
        // race music, only the menu's own loop.
        race_cache: None,
        menu_sound: Some(Arc::clone(&menu_sound)),
        race_context: None,
    };

    audio.start_race_music(&discs, MusicSource::Auto, Path::new("unused"));
    assert!(audio.race_voice.is_none(), "nothing to decode");

    // Never started, so nothing to pause - but it must still be safe to
    // call, and the menu must still come back.
    audio.pause_race_music();
    assert!(audio.music.is_some(), "the menu voice resumed");
    assert_eq!(audio.playhead(), Some(0.0));
}
