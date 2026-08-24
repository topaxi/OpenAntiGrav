//! Loads the race's sound cues off a real disc: both Pulse releases, the PS2
//! pressing, both Pure pressings, and Wipeout HD.
//!
//! **`#[ignore]`d and never run in CI.** They need game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test sfx_ground_truth --run-ignored all
//! ```
//!
//! # What this is for
//!
//! `crates/formats/tests/sblk_cue_ground_truth.rs` proves the cue-to-waveform
//! rule against the banks. This is the layer above: that the entry names
//! `audio::sfx` asks for are entries these discs *have*, that every wired cue
//! resolves through the whole chain - archive, container, name table, command
//! run, descriptor, PS-ADPCM - and that what comes out the far end is audio
//! rather than a buffer of zeros.
//!
//! The legs that would break silently are the ones where nothing in `audio::sfx`
//! branches: it asks `Archives::read_name` for whatever
//! [`oag_title::SoundBanks`] names and lets the source decide where that lives.
//! A wrong path is a silent race and a line in a report nobody reads - which is
//! why the HD test below asserts the *report text* as well as the cue set.

use std::path::{Path, PathBuf};

use oag_game::audio::sfx::{Banks, Cue};
use oag_game::race;

/// The discs this runs against, and what each is called in a failure message.
const DISCS: [(&str, &str); 5] = [
    ("pulse-psp-usa.chd", "Pulse PSP USA"),
    ("pulse-psp-eu.chd", "Pulse PSP EU"),
    ("pulse-ps2-eu.chd", "Pulse PS2 EU"),
    ("pure-psp-usa.chd", "Pure PSP USA"),
    ("pure-psp-eu.chd", "Pure PSP EU"),
];

fn image(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images")
        .join(name);
    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

/// Opens a source the way a race does, and loads its banks.
///
/// **Takes the bank table off whichever title the source turned out to be**,
/// exactly as `race::load` does. Hard-coding Pulse's here would make the HD leg
/// below test the wrong paths and pass for the wrong reason.
fn banks(image: &Path, zone: bool) -> Banks {
    let opened = oag_game::title::open_source(&image.display().to_string(), Vec::new())
        .expect("opening the source");
    let sounds = opened.title.race.sounds;
    let mut archives = opened.archives;
    Banks::load(&mut archives, sounds, zone)
}

/// Plays one sound through a real mixer and reports what came out.
///
/// Measured through [`oag_audio::Mixer`] rather than off the decoded buffer,
/// because that is the path the game takes and it is the path that can be
/// wrong: a cue that decodes perfectly and is played at the wrong pitch, on a
/// muted bus or into no voice at all is silent in exactly the way this catches.
fn render(sound: std::sync::Arc<oag_audio::Sound>) -> (f32, f32) {
    let mut mixer = oag_audio::Mixer::new(44_100);
    mixer.play(oag_audio::Play::once(sound, oag_audio::Bus::Sfx));
    let mut out = vec![0.0; 44_100 * 2];
    mixer.render(&mut out);
    let peak = out.iter().fold(0.0f32, |a, &s| a.max(s.abs()));
    #[expect(clippy::cast_precision_loss, reason = "a fixed-size buffer")]
    let rms = (out.iter().map(|s| s * s).sum::<f32>() / out.len() as f32).sqrt();
    (peak, rms)
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_wired_cue_resolves_on_every_psp_and_ps2_disc() {
    let mut ran = 0;
    for (file, label) in DISCS {
        let Some(path) = image(file) else {
            continue;
        };
        ran += 1;
        let banks = banks(&path, false);
        println!("{label}");
        for line in &banks.report {
            println!("  {line}");
        }

        let mut rng = oag_core::Rng::new(1);
        for cue in Cue::ALL {
            let Some((sound, looping)) = banks.pick(cue, &mut rng) else {
                panic!("{label}: {} did not load", cue.name());
            };
            // The chain ends in PS-ADPCM, so a cue that resolved to the wrong
            // span decodes to *something* - what it cannot do is be empty, and
            // a span of digital silence would mean the offsets landed in the
            // run-out between waveforms rather than on one.
            assert!(sound.frames() > 0, "{label}: {} decoded empty", cue.name());
            println!(
                "  {:<12} {:>7} frames, {:.2}s, loop {looping}",
                cue.name(),
                sound.frames(),
                sound.seconds(),
            );
        }

        // The `~` cue is the one that has to loop, and it is the descriptor's
        // own flag saying so rather than the name - two statements agreeing.
        let (_, engine_loops) = banks.pick(Cue::Engine, &mut rng).expect("engine");
        assert!(engine_loops, "{label}: ~ENGINE is not marked looping");
        // **Asserted as "at least one alternate loops", not "every draw does".**
        // Pulse's `~SHIELD` is two waveforms and both loop; Pure's is four and
        // only two do, so a per-draw assertion here would fail on about half
        // of the seeds. The mixed case is the reason `Loaded` keeps the flag
        // per waveform - see `audio::sfx`.
        let mut shield_loops = false;
        for _ in 0..64 {
            shield_loops |= banks.pick(Cue::Shield, &mut rng).expect("shield").1;
        }
        assert!(shield_loops, "{label}: no ~SHIELD alternate loops");
        for cue in [
            Cue::SpeedupPad,
            Cue::Collision,
            Cue::Absorb,
            Cue::ShieldActive,
        ] {
            for _ in 0..64 {
                let (_, loops) = banks.pick(cue, &mut rng).expect("cue");
                assert!(!loops, "{label}: {} loops and should not", cue.name());
            }
        }
    }
    assert!(ran > 0, "no disc image was present");
}

/// Wipeout HD loads five of the six cues, and says why it misses the sixth.
///
/// Pinned as a *list* rather than a count, because the interesting part is
/// which one and for which reason. `~ENGINE` does not exist on HD at all: its
/// ship audio is a per-event `c_*` set, a different design rather than a
/// renamed cue, so nothing is substituted and the held voice never opens.
///
/// `.COLLISIONS` used to be the second miss, recorded here as binding no
/// waveform "because all four of its commands are among the 43 unread
/// opcodes". Two of those four are `0x08`, which plays another cue by name -
/// see `oag_formats::sblk::child` - so the cue now resolves through a tree of
/// `c_CShipShip` and `c_CShipWall` to 112 waveforms.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn wipeout_hd_loads_the_five_cues_it_has_and_reports_the_one_it_does_not() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/hdfury-ps3-eu-dec.iso");
    if !path.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            path.display()
        );
        println!("skipping: {} not present", path.display());
        return;
    }
    let banks = banks(&path, false);
    for line in &banks.report {
        println!("{line}");
    }

    let mut rng = oag_core::Rng::new(3);
    let loaded: Vec<&str> = Cue::ALL
        .into_iter()
        .filter(|&c| banks.pick(c, &mut rng).is_some())
        .map(Cue::name)
        .collect();
    assert_eq!(
        loaded,
        vec![
            "SPEEDUPPAD",
            ".COLLISIONS",
            "ABSORB",
            "~SHIELD",
            "shieldactive"
        ],
        "HD's loadable cue set changed"
    );

    // The paths are the finding: HD has no `hud.bnk`, so `SPEEDUPPAD` comes out
    // of `weapons.bnk`, and a run that quietly fell back to Pulse's table would
    // load nothing at all rather than the wrong thing.
    let says = |needle: &str| banks.report.iter().any(|l| l.contains(needle));
    assert!(says(
        r"SPEEDUPPAD -> 7 waveform(s) from Data\Sound\weapons.bnk"
    ));
    assert!(says(r#"~ENGINE" names no cue in shipHD"#));
    // The child-grain walk, end to end: 112 leaves under a cue that binds
    // nothing itself. A regression to the cue's own run reads 0 here.
    assert!(says(
        r".COLLISIONS -> 112 waveform(s) from Data\Sound\shiphd.bnk"
    ));
    // The not-PS-ADPCM path, working end to end on the only disc that needs it.
    assert!(
        says("skipped as not PS-ADPCM"),
        "no HD cue exercised the non-ADPCM skip"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_collision_cue_decodes_to_fifteen_different_impacts() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let banks = banks(&path, false);

    // Drawn enough times to see the whole set: `ship.bnk`'s `.COLLISIONS` binds
    // fifteen alternates, and a picker that only ever reached one would leave
    // fourteen of the disc's own samples unplayed for ever.
    let mut rng = oag_core::Rng::new(4);
    let mut distinct = std::collections::BTreeSet::new();
    let mut lengths = Vec::new();
    for _ in 0..400 {
        let (sound, _) = banks.pick(Cue::Collision, &mut rng).expect("collision");
        if distinct.insert(std::sync::Arc::as_ptr(&sound) as usize) {
            lengths.push(sound.seconds());
        }
    }
    lengths.sort_by(f32::total_cmp);
    println!("{} distinct impacts, {lengths:.3?}", distinct.len());
    assert_eq!(distinct.len(), 15, "the alternates are not all reachable");

    // They are alternates rather than layers, and this is the measurement that
    // says so: fifteen samples of one event, all within a tenth of a second of
    // each other. A cue whose commands were meant to play *together* would be
    // a stack of different lengths.
    let (shortest, longest) = (lengths[0], lengths[lengths.len() - 1]);
    assert!(
        longest - shortest < 0.2,
        "the fifteen span {shortest:.3}s to {longest:.3}s, which is not one event"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_decoded_cues_are_audio_and_not_silence() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let banks = banks(&path, false);
    let mut rng = oag_core::Rng::new(2);
    for cue in Cue::ALL {
        let (sound, _) = banks.pick(cue, &mut rng).expect("cue");
        let seconds = sound.seconds();
        let (peak, rms) = render(std::sync::Arc::clone(&sound));
        println!(
            "{:<12} {seconds:.3}s  peak {peak:.3}  rms {rms:.4}",
            cue.name()
        );

        assert!(
            seconds > 0.01,
            "{} decoded to {seconds:.4}s, which is not a sound",
            cue.name()
        );
        // Every effect on either disc is under three seconds; a cue that came
        // back longer would have resolved to a span running past its own
        // waveform into its neighbours.
        assert!(
            seconds < 3.0,
            "{} decoded to {seconds:.2}s, longer than any effect on the disc",
            cue.name()
        );
        // The check the whole chain reduces to: samples reached the output.
        // A wrong offset, a muted bus or a voice that was never allocated all
        // land here as a flat zero.
        assert!(peak > 0.01, "{} rendered silence", cue.name());
        assert!(
            rms > 0.0,
            "{} rendered a single non-zero sample",
            cue.name()
        );
        // Nothing clips: the mixer is fed 16-bit PCM at unit gain, so a peak
        // above full scale would mean the decode overflowed rather than
        // clamped.
        assert!(
            peak <= 1.0,
            "{} rendered at {peak:.3}, above full scale",
            cue.name()
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn zone_mode_reads_its_own_ship_bank() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let ordinary = banks(&path, false);
    let zone = banks(&path, true);
    let mut rng = oag_core::Rng::new(6);

    let (ordinary_engine, _) = ordinary.pick(Cue::Engine, &mut rng).expect("engine");
    let mut zone_rng = oag_core::Rng::new(6);
    let (zone_engine, _) = zone.pick(Cue::Engine, &mut zone_rng).expect("zone engine");

    println!(
        "ship.bnk ~ENGINE {:.3}s, ship_zone.bnk ~ENGINE {:.3}s",
        ordinary_engine.seconds(),
        zone_engine.seconds()
    );
    // `SHIP_ZM`'s `~ENGINE` binds nine layers to `SHIP`'s one, so no draw from
    // one can equal the other's single waveform unless the wrong bank was read.
    // Asserted on the length rather than on the count, because `pick` returns
    // one waveform by design.
    assert!(
        (ordinary_engine.seconds() - zone_engine.seconds()).abs() > 1e-6,
        "a Zone race loaded the ordinary ship bank"
    );
}

/// The whole path, on real data: a real `Race`, a real `Audio` on the null
/// backend, and the composition root's own call.
///
/// This is the one that catches a *wiring* bug rather than a decoding one. The
/// engine is a held loop, and the frame loop's finished-race arm steps nothing,
/// so a version that skipped `race_tick` there left `~ENGINE` sounding at
/// racing pitch under the results table until the player backed out. That
/// version existed.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_engine_sounds_while_a_race_runs_and_stops_when_it_finishes() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: path.display().to_string(),
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);

    // `Some(dump)` forces the null backend, which is what makes this runnable
    // on a machine with no sound card and on CI - see `Audio::open`.
    let mut audio = oag_game::audio::Audio::open(
        &oag_game::settings::Audio::default(),
        Some(std::path::PathBuf::from("/dev/null")),
    );
    let voices =
        |audio: &oag_game::audio::Audio| audio.output().with_mixer(|mixer| mixer.active_voices());

    assert_eq!(voices(&audio), 0, "something was playing before the race");
    for _ in 0..120 {
        race.tick(&oag_gameplay::InputSnapshot::default());
        audio.race_tick(&mut race);
        // The other half of the composition root's per-tick pair: with the
        // null backend this is the only thing that renders, and a voice that is
        // never rendered never *ends* - so without it a fired one-shot would
        // sit in the pool for the whole test and every count below would be
        // measuring the wrong thing.
        audio.tick();
    }
    assert!(
        voices(&audio) >= 1,
        "two seconds of racing and the engine never opened a voice"
    );

    // Forced rather than driven: reaching a real finish is a lap of real
    // circuit, and what is under test is the frame loop's finished-race arm,
    // not the lap counter. This is the same flag `Race::finished` reads.
    race.world.race.finished = true;
    for _ in 0..600 {
        // Deliberately **not** calling `race.tick` - that is exactly what the
        // finished arm does not do.
        audio.race_tick(&mut race);
        audio.tick();
        // The other half of the composition root's per-tick pair: with the
        // null backend this is the only thing that renders, and a voice that is
        // never rendered never *ends* - so without it a fired one-shot would
        // sit in the pool for the whole test and every count below would be
        // measuring the wrong thing.
        audio.tick();
    }
    assert_eq!(
        voices(&audio),
        0,
        "the engine is still sounding ten seconds after the race ended"
    );
}

/// The whole grid sounds, and it does not all sound from the same place.
///
/// The end-to-end half of positional audio: unit tests check the law
/// (`oag_audio::spatial`) and the wiring (`oag_game::audio::sfx`), and neither
/// can see whether eight real `~ENGINE` waveforms out of a real bank actually
/// reach eight voices with eight positions. Before this landed the answer was
/// one voice, and every test still passed.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_whole_grid_is_audible_and_not_all_from_one_place() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: path.display().to_string(),
        // The only mode that fields opponents, which is the whole point here.
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    assert!(
        race.ship_count() > 1,
        "the fixture raced alone, so nothing below is testing anything"
    );

    let mut audio = oag_game::audio::Audio::open(
        &oag_game::settings::Audio::default(),
        Some(std::path::PathBuf::from("/dev/null")),
    );
    for _ in 0..120 {
        race.tick(&oag_gameplay::InputSnapshot::default());
        audio.race_tick(&mut race);
        audio.tick();
    }

    let voices = audio.output().with_mixer(|mixer| mixer.active_voices());
    assert_eq!(
        voices,
        race.ship_count() as usize,
        "one engine per craft is what `ExhaustFlare_Init` builds; got {voices} \
         for {} craft",
        race.ship_count()
    );

    // **Rendered, not inspected.** A pan that is stored and never applied is
    // exactly the bug this is here to catch, and the only way to see it is in
    // the samples. Nothing but the effects bus is open - this test never starts
    // the music - so any difference between the channels is the panner.
    //
    // **The craft is moved deliberately rather than raced into place.** Two
    // seconds after the flag the AI has driven the field well past the engine
    // emitter's 50-unit radius, so a grid left to itself renders the player's
    // engine alone and centred - which is the recovered behaviour and tests
    // nothing. Putting one rival ten units off the camera's own right axis is
    // the same kind of fixture `race::tests::cues` uses for its wall.
    let camera = race.view().inverse();
    let right = camera.x_axis.truncate().normalize();
    race.world.ships[1].physics.body.position = camera.w_axis.truncate() + right * 10.0;
    audio.race_tick(&mut race);

    let (left, right) = audio.output().with_mixer(|mixer| {
        let mut out = vec![0.0f32; 2 * 4096];
        mixer.render(&mut out);
        out.as_chunks::<2>()
            .0
            .iter()
            .fold((0.0f32, 0.0f32), |(l, r), f| {
                (l + f[0] * f[0], r + f[1] * f[1])
            })
    });
    assert!(
        left > 0.0 && right > 0.0,
        "the grid rendered silence: {left} / {right}"
    );
    assert!(
        right > left * 1.5,
        "a craft ten units off the camera's right axis came out at {left} \
         left against {right} right - the pan is being computed and dropped"
    );
}

/// The shield's two cues through the whole path, which nothing else covers.
///
/// `~SHIELD` is driven by a *level* rather than by an edge - the audio layer
/// reads `Race::shield_is_up` - so it is the one held voice with no queue entry
/// to inspect, and the only way to see it is to watch the pool.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_shield_opens_a_held_voice_and_closes_it_when_the_pickup_expires() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: path.display().to_string(),
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    let mut audio = oag_game::audio::Audio::open(
        &oag_game::settings::Audio::default(),
        Some(std::path::PathBuf::from("/dev/null")),
    );
    let voices =
        |audio: &oag_game::audio::Audio| audio.output().with_mixer(|mixer| mixer.active_voices());

    // Settle first, so the engine's own voice is already open and the counts
    // below are differences rather than absolutes.
    for _ in 0..60 {
        race.tick(&oag_gameplay::InputSnapshot::default());
        audio.race_tick(&mut race);
        // The other half of the composition root's per-tick pair: with the
        // null backend this is the only thing that renders, and a voice that is
        // never rendered never *ends* - so without it a fired one-shot would
        // sit in the pool for the whole test and every count below would be
        // measuring the wrong thing.
        audio.tick();
    }
    let idle = voices(&audio);
    assert!(idle >= 1, "the engine never opened");

    // Set directly: this is the field `oag_physics::damage` drives and the one
    // `Race::shield_is_up` reads, and granting a real pickup would be testing
    // the pad table instead.
    race.world.ships[0].physics.shield_pickup_timer = 1.0;
    race.tick(&oag_gameplay::InputSnapshot::default());
    audio.race_tick(&mut race);
    assert!(
        voices(&audio) > idle,
        "the shield came up and opened no voice"
    );

    // Held across ticks rather than re-triggered - it is a `~` cue.
    let held = voices(&audio);
    for _ in 0..30 {
        race.tick(&oag_gameplay::InputSnapshot::default());
        audio.race_tick(&mut race);
        // The other half of the composition root's per-tick pair: with the
        // null backend this is the only thing that renders, and a voice that is
        // never rendered never *ends* - so without it a fired one-shot would
        // sit in the pool for the whole test and every count below would be
        // measuring the wrong thing.
        audio.tick();
    }
    assert!(
        voices(&audio) <= held,
        "the shield loop is being re-triggered every tick"
    );

    race.world.ships[0].physics.shield_pickup_timer = 0.0;
    race.tick(&oag_gameplay::InputSnapshot::default());
    audio.race_tick(&mut race);
    assert_eq!(
        voices(&audio),
        idle,
        "the shield dropped and its voice kept sounding"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_loaded_race_reports_what_its_banks_did() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: path.display().to_string(),
        ..race::Options::default()
    })
    .expect("loading the race");

    let sfx: Vec<&String> = loaded
        .report
        .iter()
        .filter(|line| line.starts_with("sfx:"))
        .collect();
    for line in &sfx {
        println!("{line}");
    }
    // One line per cue, whether it loaded or not: a race that came up silent
    // has to say so in the same place a race that did not says what it read.
    assert_eq!(sfx.len(), Cue::ALL.len(), "the load report is incomplete");
    assert!(
        !loaded.setup.sounds.is_empty(),
        "a real disc loaded no cues"
    );
}
