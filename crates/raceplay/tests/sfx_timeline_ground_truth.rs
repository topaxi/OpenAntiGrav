//! Pulse's sound cues played as the timelines they author, against the discs.
//!
//! **`#[ignore]`d and never run in CI**: they need game content. Run with
//! `just test-data`.
//!
//! What is asserted is structure read off the mixer's own output, not a golden
//! hash: each voice of a layered cue is silent until its authored delay and
//! audible right after it, and the whole cue ends where its last voice ends.
//! The expected delays are the bank's own (`PLASMA`'s tail keyed 215 master
//! ticks in, `ROCKEXPLSHIP`'s third grain at 50), converted at the measured
//! 258.4 Hz.

use std::path::PathBuf;

use oag_audio::{Bus, Mixer};
use oag_core::Rng;
use oag_gameplay::cue::Cue;
use oag_sound::sfx::{Banks, CueTables, CueVoice, VoicePlace, start_voices};

const RATE: u32 = 44_100;
const PULSE: [&str; 3] = ["pulse-psp-usa.chd", "pulse-psp-eu.chd", "pulse-ps2-eu.chd"];
const NOT_PULSE: [&str; 2] = ["pure-psp-usa.chd", "pure-psp-eu.chd"];

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

fn banks(path: &std::path::Path) -> Banks {
    let opened =
        oag_source::title::open_source(&path.display().to_string(), Vec::new(), Vec::new())
            .expect("opening the source");
    let sounds = opened.title.race.sounds;
    let tick = opened
        .title
        .race
        .zone_announcer
        .map_or(oag_title::SequenceTick::Unknown, |z| z.tick);
    let mut archives = opened.archives;
    Banks::load(&mut archives, sounds, false, tick)
}

/// One voice alone through a mixer: (first audible frame, last audible frame)
/// in seconds.
fn extent(voice: &CueVoice) -> (f64, f64) {
    let mut mixer = Mixer::new(RATE);
    let _ = start_voices(
        &mut mixer,
        std::slice::from_ref(voice),
        Bus::Sfx,
        VoicePlace::DRY,
    );
    frames(&mut mixer)
}

fn frames(mixer: &mut Mixer) -> (f64, f64) {
    let mut out = vec![0.0f32; 2 * 4096];
    let (mut first, mut last, mut at) = (None, 0usize, 0usize);
    for _ in 0..(6 * RATE as usize / 4096) {
        mixer.render(&mut out);
        for (i, f) in out.chunks(2).enumerate() {
            if f[0].abs() > 1e-4 || f[1].abs() > 1e-4 {
                first.get_or_insert(at + i);
                last = at + i;
            }
        }
        at += 4096;
        if mixer.active_voices() == 0 {
            break;
        }
    }
    (
        first.map_or(f64::NAN, |i| i as f64 / f64::from(RATE)),
        (last + 1) as f64 / f64::from(RATE),
    )
}

fn voices(banks: &Banks, cue: Cue) -> Vec<CueVoice> {
    banks.voices(cue, &mut Rng::new(3)).expect("cue loads")
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_layered_cue_keys_each_voice_on_at_its_authored_delay() {
    // (cue, the delays of its voices in master ticks)
    let cases: [(Cue, &[u32]); 4] = [
        (Cue::Plasma, &[0, 15, 215]),
        (Cue::RocketHitShip, &[0, 0, 50]),
        (Cue::PlasmaHitShip, &[0, 0, 0, 200]),
        (Cue::Missile, &[0, 15, 15]),
    ];
    let mut ran = 0;
    for file in PULSE {
        let Some(path) = image(file) else { continue };
        ran += 1;
        let banks = banks(&path);
        for (cue, ticks) in cases {
            let started = voices(&banks, cue);
            assert_eq!(started.len(), ticks.len(), "{file}: {}", cue.name());
            let mut end = 0.0f64;
            for (voice, &t) in started.iter().zip(ticks) {
                let want = f64::from(t) / (44_100.0 * 3.0 / 512.0);
                assert!(
                    (voice.delay - want).abs() < 1e-9,
                    "{file}: {} voice at {} not {want}",
                    cue.name(),
                    voice.delay
                );
                let (first, last) = extent(voice);
                assert!(
                    first >= want - 0.001 && first < want + 0.02,
                    "{file}: {} first audible at {first}, authored {want}",
                    cue.name()
                );
                let authored = want + f64::from(voice.sound.seconds()) / f64::from(voice.pitch);
                assert!(
                    (last - authored).abs() < 0.03,
                    "{file}: {} ends at {last}, authored {authored}",
                    cue.name()
                );
                end = end.max(authored);
            }
            // Together: the cue ends where its last voice ends.
            let mut mixer = Mixer::new(RATE);
            let _ = start_voices(&mut mixer, &started, cue.bus(), VoicePlace::DRY);
            let (first, last) = frames(&mut mixer);
            assert!(
                first < 0.02,
                "{file}: {} starts late at {first}",
                cue.name()
            );
            assert!(
                (last - end).abs() < 0.03,
                "{file}: {} ends at {last}, authored {end}",
                cue.name()
            );
        }
    }
    assert!(ran > 0, "no Pulse disc image was present");
}

/// Wipeout HD's SCREAM runs a 240 Hz master tick (`SequenceTick::Ps3`), not the
/// PSP's 258.4: the same authored tick counts land later in seconds.
fn hd_s_layered_cues_key_on_at_the_ps3_tick(banks: &Banks) {
    // (cue, the delays of its voices in master ticks), read off the bank.
    let cases: [(Cue, &[u32]); 2] = [
        (Cue::Plasma, &[0, 15, 215, 230]),
        (Cue::PlasmaHitWall, &[0, 0, 15, 15, 40]),
    ];
    for (cue, ticks) in cases {
        let started = voices(banks, cue);
        assert_eq!(started.len(), ticks.len(), "{}", cue.name());
        let mut end = 0.0f64;
        for (voice, &t) in started.iter().zip(ticks) {
            let want = f64::from(t) / 240.0;
            assert!(
                (voice.delay - want).abs() < 1e-9,
                "{}: voice at {} not {want}",
                cue.name(),
                voice.delay
            );
            let (first, last) = extent(voice);
            assert!(
                first >= want - 0.001 && first < want + 0.02,
                "{}: first audible at {first}, authored {want}",
                cue.name()
            );
            let authored = want + f64::from(voice.sound.seconds()) / f64::from(voice.pitch);
            assert!(
                (last - authored).abs() < 0.03,
                "{}: ends at {last}, authored {authored}",
                cue.name()
            );
            end = end.max(authored);
        }
        let mut mixer = Mixer::new(RATE);
        let _ = start_voices(&mut mixer, &started, cue.bus(), VoicePlace::DRY);
        let (first, last) = frames(&mut mixer);
        assert!(first < 0.02, "{}: starts late at {first}", cue.name());
        assert!(
            (last - end).abs() < 0.03,
            "{}: ends at {last}, authored {end}",
            cue.name()
        );
    }
    assert!(
        banks
            .report
            .iter()
            .any(|l| l.contains("plays its timeline")),
        "{:?}",
        banks.report
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_shield_holds_both_of_its_loops_at_once() {
    let mut ran = 0;
    for file in PULSE {
        let Some(path) = image(file) else { continue };
        ran += 1;
        let started = voices(&banks(&path), Cue::Shield);
        assert_eq!(started.len(), 2, "{file}");
        assert!(
            started.iter().all(|v| v.looping && v.delay == 0.0),
            "{file}"
        );
        assert_ne!(
            started[0].sound.seconds(),
            started[1].sound.seconds(),
            "{file}: two different loops"
        );
    }
    assert!(ran > 0, "no Pulse disc image was present");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn alternates_stay_a_no_repeat_pick_of_one_voice() {
    let mut ran = 0;
    for file in PULSE {
        let Some(path) = image(file) else { continue };
        ran += 1;
        let banks = banks(&path);
        let mut rng = Rng::new(11);
        let mut previous = None;
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..200 {
            let started = banks.voices(Cue::Collision, &mut rng).expect("loads");
            assert_eq!(started.len(), 1, "{file}");
            let key = started[0].sound.frames();
            assert_ne!(previous, Some(key), "{file}: the same take twice running");
            previous = Some(key);
            seen.insert(key);
        }
        assert!(seen.len() > 5, "{file}: only {} takes heard", seen.len());
    }
    assert!(ran > 0, "no Pulse disc image was present");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn titles_without_a_measured_tick_keep_the_flat_pick() {
    for file in NOT_PULSE {
        let Some(path) = image(file) else { continue };
        let banks = banks(&path);
        assert!(
            !banks
                .report
                .iter()
                .any(|l| l.contains("plays its timeline")),
            "{file}: {:?}",
            banks.report
        );
        let started = voices(&banks, Cue::Shield);
        assert_eq!(started.len(), 1, "{file}");
    }
}

const HD_TICK: f64 = 240.0;

/// The summed absolute level on each output channel of one voice played alone.
fn channel_levels(voice: &CueVoice) -> (f32, f32) {
    let mut mixer = Mixer::new(RATE);
    let _ = start_voices(
        &mut mixer,
        std::slice::from_ref(&CueVoice {
            delay: 0.0,
            ..voice.clone()
        }),
        Bus::Sfx,
        VoicePlace::DRY,
    );
    let mut out = vec![0.0f32; 2 * 4096];
    let (mut left, mut right) = (0.0, 0.0);
    for _ in 0..8 {
        mixer.render(&mut out);
        for frame in out.chunks(2) {
            left += frame[0].abs();
            right += frame[1].abs();
        }
    }
    (left, right)
}

/// `SPEEDUPPAD` on Fury, walked with HD's operand layout and cue variables
/// (`docs/ghidra/functions/ps3-hdfury-eu/cue-variables.md`, "`SPEEDUPPAD`"):
/// the player's own pad (perspective 0) keys a body, then a whoosh behind-left
/// at tick 0 and one behind-right ten ticks on, under one bend draw of up to two
/// semitones; another craft's pad (perspective 2) is the body and one whoosh.
fn hd_s_speedup_pad_is_a_body_and_two_rear_whooshes_for_the_player_and_one_for_others(
    banks: &Banks,
) {
    let mut rng = Rng::new(3);
    let mut pitches = Vec::new();
    for _ in 0..200 {
        let mine = banks
            .voices_for(Cue::SpeedupPad, true, &mut rng)
            .expect("the pad loads");
        assert_eq!(mine.len(), 3, "body and two whooshes");
        let ticks: Vec<f64> = mine.iter().map(|v| v.delay * HD_TICK).collect();
        for (got, want) in ticks.iter().zip([0.0, 0.0, 10.0]) {
            assert!((got - want).abs() < 1e-6, "{ticks:?}");
        }
        let angles: Vec<i32> = mine.iter().map(|v| v.angle).collect();
        assert_eq!(angles, vec![0, 220, 140]);
        assert_eq!(mine[1].pitch, mine[2].pitch, "one bend draw for the pair");
        assert_eq!(mine[0].pitch, 1.0, "the body has no bend range");
        pitches.push(mine[1].pitch);
    }
    let low = pitches.iter().copied().fold(f32::MAX, f32::min);
    let high = pitches.iter().copied().fold(f32::MIN, f32::max);
    let two_semitones = 2f32.powf(2.0 / 12.0);
    assert!(
        low >= 1.0 / two_semitones - 1e-4 && high <= two_semitones + 1e-4,
        "{low} {high}"
    );
    assert!(
        low < 0.95 && high > 1.05,
        "the bend is drawn, not fixed: {low} {high}"
    );

    // Behind-left and behind-right, through the mixer: the 220 degree whoosh
    // is louder on the left, the 140 degree one on the right.
    let mine = banks
        .voices_for(Cue::SpeedupPad, true, &mut Rng::new(5))
        .unwrap();
    let (l, r) = channel_levels(&mine[1]);
    assert!(l > 1.3 * r, "220 degrees: left {l} right {r}");
    let (l, r) = channel_levels(&mine[2]);
    assert!(r > 1.3 * l, "140 degrees: left {l} right {r}");

    // Another craft's pad: the body and one of the two whooshes, centred.
    let mut late = std::collections::BTreeSet::new();
    for _ in 0..200 {
        let theirs = banks
            .voices_for(Cue::SpeedupPad, false, &mut rng)
            .expect("the pad loads");
        assert_eq!(theirs.len(), 2, "body and one whoosh");
        assert_eq!(theirs[0].delay, 0.0);
        assert!(theirs.iter().all(|v| v.angle == 0));
        late.insert((theirs[1].delay * HD_TICK).round() as u32);
    }
    assert_eq!(
        late.into_iter().collect::<Vec<_>>(),
        vec![0, 5],
        "the whoosh is keyed at tick 0 or tick 5, by the draw"
    );
}

/// Every voice of every draw of `cue`, by the seat that hears it.
fn hd_draws(banks: &Banks, cue: Cue, player: bool) -> Vec<Vec<CueVoice>> {
    let mut rng = Rng::new(13);
    (0..60)
        .map(|_| banks.voices_for(cue, player, &mut rng).expect("loads"))
        .collect()
}

/// HD's explosion, hit and launch cues, walked with their lists
/// (`docs/ghidra/functions/ps3-hdfury-eu/cue-variables.md`, audit rows 3 and 4):
/// the voices a play keys and the tick the last one starts on, as the bank's
/// guarded lists author them, against the one waveform the flat pick played.
fn hd_s_explosion_and_launch_cues_play_their_layered_voices(banks: &Banks) {
    // (cue, heard by the player's craft, voices per play, last voice's tick)
    let cases: [(Cue, bool, usize, u32); 14] = [
        (Cue::RocketHitShip, false, 10, 50),
        (Cue::RocketHitWall, false, 6, 5),
        (Cue::MissileHitShip, false, 4, 5),
        (Cue::MissileExpire, false, 17, 105),
        (Cue::PlasmaHitShip, false, 8, 215),
        (Cue::RepulsorHit, false, 12, 30),
        (Cue::Rocket, false, 2, 0),
        (Cue::Missile, false, 4, 25),
        (Cue::MineLaunch, true, 8, 95),
        (Cue::Turbo, true, 6, 90),
        (Cue::Turbo, false, 3, 82),
        (Cue::Cannon, true, 3, 10),
        (Cue::Cannon, false, 2, 10),
        (Cue::Repulsor, true, 8, 210),
    ];
    for (cue, player, voices, last) in cases {
        let draws = hd_draws(banks, cue, player);
        for draw in &draws {
            assert_eq!(draw.len(), voices, "{} player={player}", cue.name());
        }
        let latest = draws
            .iter()
            .flatten()
            .map(|v| (v.delay * HD_TICK).round() as u32)
            .max();
        assert_eq!(latest, Some(last), "{} player={player}", cue.name());
    }

    // The rocket is a left/right pair: 90 and 270 degrees, hard right and left.
    let pair = &hd_draws(banks, Cue::Rocket, false)[0];
    let mut angles: Vec<i32> = pair.iter().map(|v| v.angle).collect();
    angles.sort_unstable();
    assert_eq!(angles, vec![90, 270]);
    let (right_l, right_r) = channel_levels(pair.iter().find(|v| v.angle == 90).unwrap());
    assert!(right_r > 4.0 * right_l, "90 degrees: {right_l} {right_r}");
    let (left_l, left_r) = channel_levels(pair.iter().find(|v| v.angle == 270).unwrap());
    assert!(left_l > 4.0 * left_r, "270 degrees: {left_l} {left_r}");

    // Another craft's absorb is authored silent; the player's is a pair at +-30.
    assert!(
        hd_draws(banks, Cue::Absorb, false)
            .iter()
            .all(Vec::is_empty)
    );
    let mut absorb: Vec<i32> = hd_draws(banks, Cue::Absorb, true)[0]
        .iter()
        .map(|v| v.angle.rem_euclid(360))
        .collect();
    absorb.sort_unstable();
    assert_eq!(absorb, vec![0, 30, 330]);

    // The census: every one-shot hud, ship and weapons cue plays a timeline,
    // `.COLLISIONS` too now that the contact's severity and surface are handed to it, and so do
    // the three contact cues only HD has (which are not in `Cue::ALL`).
    let plays_timeline = |c: &Cue| {
        banks
            .report
            .iter()
            .find(|l| l.starts_with(&format!("sfx: {} ->", c.name())))
            .is_some_and(|l| l.contains("plays its timeline"))
    };
    let flat: Vec<&str> = Cue::ALL
        .into_iter()
        .chain(Cue::HD_CONTACT)
        .filter(|c| c.walks_variables())
        .filter(|c| !plays_timeline(c))
        .map(Cue::name)
        .collect();
    assert_eq!(flat, Vec::<&str>::new());
    assert!(
        !Cue::ALL.iter().any(|c| Cue::HD_CONTACT.contains(c)),
        "the three HD-only contact cues are loaded beside Cue::ALL, not in it"
    );
}

/// The identity of a waveform as the mixer's sound shows it: its frames and its rate.
type Waveform = (usize, u32);

/// A waveform of `bank` as the mixer's sound shows it.
fn waveform_of(bank: &oag_formats::sblk::Bank, sound: &oag_formats::sblk::Sound) -> Waveform {
    use oag_formats::sblk;
    let data = bank.waveform(sound).expect("the waveform is in the bank");
    let pcm = if sound.is_adpcm() {
        sblk::decode_adpcm(sblk::adpcm_played(data))
    } else {
        sblk::decode_pcm16(data)
    };
    (pcm.len(), sound.sample_rate())
}

/// The key-on commands the cue `name` and its children bind in `bank`. A command is one cue's
/// own: two cues may key the same waveform data, never the same command.
fn commands_of(bank: &oag_formats::sblk::Bank, name: &str) -> std::collections::BTreeSet<usize> {
    let record = bank.cue_named(name).expect("the cue is in the bank");
    bank.cue_tree_sounds(&record)
        .iter()
        .map(|sound| sound.command)
        .collect()
}

/// The key-on commands and waveforms the bank authors for the cue `name` handed `variables`, as
/// the walk of its list reaches them, over every alternate combination.
fn authored(
    bank: &oag_formats::sblk::Bank,
    name: &str,
    variables: [i8; 4],
) -> (
    std::collections::BTreeSet<usize>,
    std::collections::BTreeSet<Waveform>,
) {
    use oag_formats::sblk::timeline::{GLOBAL_SLOTS, WalkModel};
    let record = bank.cue_named(name).expect("the cue is in the bank");
    let model = WalkModel {
        goto_markers: true,
        globals: Some([0; GLOBAL_SLOTS]),
        tail_waits: true,
        cue_variables: Some(variables),
        ..WalkModel::default()
    };
    let walks = bank
        .cue_timelines_sampled(&record, model, u64::from(record.index))
        .expect("the combinations are enumerable");
    let grains: Vec<_> = walks.iter().flat_map(|walk| &walk.grains).collect();
    (
        grains.iter().map(|grain| grain.sound.command).collect(),
        grains
            .iter()
            .map(|grain| waveform_of(bank, &grain.sound))
            .collect(),
    )
}

/// The waveforms 40 plays of `cue` handed `variables` start.
fn contact_plays(
    banks: &Banks,
    cue: Cue,
    variables: [i8; 4],
) -> std::collections::BTreeSet<Waveform> {
    let mut rng = Rng::new(21);
    let mut heard = std::collections::BTreeSet::new();
    for _ in 0..40 {
        let started = banks
            .contact_voices(cue, variables, &mut rng)
            .unwrap_or_else(|| panic!("{} at {variables:?} built no timeline", cue.name()));
        assert!(
            !started.is_empty(),
            "{} at {variables:?} plays no voice",
            cue.name()
        );
        heard.extend(
            started
                .iter()
                .map(|v| (v.sound.frames(), v.sound.sample_rate())),
        );
    }
    heard
}

/// HD's contact cues play the child the contact's severity and surface select
/// (`docs/ghidra/functions/ps3-hdfury-eu/collision-cues.md`, "`Ship_UpdateCollisionCues`";
/// `cue-variables.md`, "The collision family"): every block of cue variables the game can hand
/// builds, and plays the waveforms of the child the bank authors for it and no other's.
fn hd_s_contact_cues_play_the_child_their_severity_and_surface_select(
    banks: &Banks,
    ship: &oag_formats::sblk::Bank,
) {
    let of = |name: &str| commands_of(ship, name);
    let only = |cue: Cue, variables, children: &[&str]| {
        let wanted: std::collections::BTreeSet<usize> =
            children.iter().flat_map(|&name| of(name)).collect();
        let (commands, waveforms) = authored(ship, cue.name(), variables);
        assert!(
            !commands.is_empty() && commands.is_subset(&wanted),
            "{} at {variables:?} walks commands outside {children:?}: {commands:?}",
            cue.name()
        );
        // What the engine plays is what that walk reaches.
        let heard = contact_plays(banks, cue, variables);
        assert!(
            heard.is_subset(&waveforms),
            "{} at {variables:?} plays waveforms its walk does not reach: {heard:?}",
            cue.name()
        );
    };

    only(Cue::Collision, [2, 0, 2, 0], &["c_CShipShipL"]);
    only(Cue::Collision, [1, 0, 2, 0], &["c_CShipShipL"]);
    only(Cue::Collision, [0, 0, 1, 0], &["c_CShipShipM"]);
    only(Cue::Collision, [0, 1, 1, 0], &["c_CShipWallS"]);
    only(Cue::Collision, [1, 1, 1, 0], &["c_CShipWallM"]);
    only(Cue::Collision, [2, 1, 2, 0], &["c_CShipWallL"]);
    for (severity, size) in [(0, "S"), (1, "M"), (2, "L")] {
        only(
            Cue::Grind,
            [severity, 0, 0, 0],
            &[&format!("c_GShipShip{size}")],
        );
        only(
            Cue::Grind,
            [severity, 1, 0, 0],
            &[&format!("c_GShipWall{size}")],
        );
    }

    // The shield's crackle: the small pair, whose children switch on the perspective.
    let small: std::collections::BTreeSet<usize> = ["c_GS_Small", "c_CS_Small"]
        .iter()
        .flat_map(|&name| of(name))
        .collect();
    let (commands, waveforms) = authored(ship, Cue::CollisionShield.name(), [0, 1, 1, 0]);
    assert!(commands.is_subset(&small), "{commands:?}");
    for name in ["c_GS_Small", "c_CS_Small"] {
        assert!(
            commands.iter().any(|command| of(name).contains(command)),
            "nothing of {name} is walked"
        );
    }
    for name in ["c_GS_Medium", "c_CS_Medium", "c_GS_Large", "c_CS_Large"] {
        let others: std::collections::BTreeSet<usize> =
            of(name).difference(&small).copied().collect();
        assert!(
            commands.is_disjoint(&others),
            "{name} is walked at severity 0"
        );
    }
    let heard = contact_plays(banks, Cue::CollisionShield, [0, 1, 1, 0]);
    assert!(heard.is_subset(&waveforms), "{heard:?}");

    // Every block the game can hand a contact cue builds and plays at least one voice.
    let blocks = |perspectives: &'static [i8]| {
        (0..=2).flat_map(move |severity| {
            (0..=1).flat_map(move |surface| {
                perspectives
                    .iter()
                    .map(move |&perspective| [severity, surface, perspective, 0])
            })
        })
    };
    for (cue, perspectives) in [
        (Cue::Collision, &[1, 2][..]),
        (Cue::CollisionShield, &[1]),
        (Cue::Grind, &[0]),
        (Cue::GrindShield, &[0]),
    ] {
        for variables in blocks(perspectives) {
            assert!(!contact_plays(banks, cue, variables).is_empty());
        }
    }
}

/// The bend is read at 100 % on HD's walked cues: `PLASMAHITWALL`'s voices detune
/// from play to play (the operand byte was read as 0 %, so every play was identical).
fn hd_s_plasma_hit_wall_detunes_from_play_to_play(banks: &Banks) {
    let pitches: std::collections::BTreeSet<u32> = hd_draws(banks, Cue::PlasmaHitWall, false)
        .iter()
        .flatten()
        .map(|v| v.pitch.to_bits())
        .collect();
    assert!(pitches.len() > 20, "{} distinct pitches", pitches.len());
}

/// One pass over Fury's banks, loaded once, for every claim about how HD's race
/// cues play.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_s_race_cues_play_the_lists_the_bank_authors() {
    let Some(path) = image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let banks = banks(&path);
    hd_s_layered_cues_key_on_at_the_ps3_tick(&banks);
    hd_s_speedup_pad_is_a_body_and_two_rear_whooshes_for_the_player_and_one_for_others(&banks);
    hd_s_explosion_and_launch_cues_play_their_layered_voices(&banks);
    hd_s_plasma_hit_wall_detunes_from_play_to_play(&banks);
    let ship = hd_ship_bank(&path);
    hd_s_contact_cues_play_the_child_their_severity_and_surface_select(
        &banks,
        &oag_formats::sblk::Bank::parse(&ship).expect("the ship bank parses"),
    );
}

/// The bytes of HD's ship sound bank, `shiphd.bnk`.
fn hd_ship_bank(path: &std::path::Path) -> Vec<u8> {
    let opened =
        oag_source::title::open_source(&path.display().to_string(), Vec::new(), Vec::new())
            .expect("opening the source");
    let entry = opened.title.race.sounds.ship;
    let mut archives = opened.archives;
    archives.read_name(entry).expect("the ship bank reads")
}
