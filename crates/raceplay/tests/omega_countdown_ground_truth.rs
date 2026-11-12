//! Omega's race start holds the cues `_321_NGP` and `Go_NGP`, and no shipped
//! bank defines an event by either bare name: the executable posts
//! `<cue>[_PC]__<bank>`, and `crates/formats/tests/wwise_ground_truth.rs`
//! (`posted_events_resolve`) checks that those full names resolve. Ours still
//! lends Pulse's `ready` and `go`, which load nothing from `speech.bnk`, a Wwise
//! stub, so the start is silent; its `Go` cue event only times the song.
//!
//! **`#[ignore]`d and never run in CI**: it needs the decrypted Omega packages
//! under `data/extracted/ps4`. Evidence:
//! `docs/ghidra/functions/ps4-omega-eu/wwise-sound.md` ("The countdown, end to end").

use std::path::Path;

use oag_formats::wwise::{Bank, Kind, name_hash};

#[test]
#[ignore = "needs the decrypted Omega packages in data/extracted/ps4/"]
fn no_omega_bank_defines_the_start_events() {
    let mut checked = 0;
    for (dir, file) in [
        ("omega-eu", "data00.psarc"),
        ("omega-eu-patch", "data08.psarc"),
    ] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/extracted/ps4")
            .join(dir)
            .join("uroot")
            .join(file);
        if !path.exists() {
            assert!(
                std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
                "OAG_REQUIRE_GAME_DATA is set but {} is missing",
                path.display()
            );
            println!("skipping: {} not present", path.display());
            return;
        }
        let mut archive = oag_assets::psarc::Archive::open_file(&path).expect("archive opens");
        let paths: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.ends_with(".bnk"))
            .cloned()
            .collect();
        let blobs: Vec<Vec<u8>> = paths
            .iter()
            .map(|p| archive.read_path(p).expect("a listed bank reads"))
            .collect();
        let banks: Vec<Bank> = blobs
            .iter()
            .map(|b| Bank::parse(b).expect("a Wwise bank"))
            .collect();
        assert!(banks.len() >= 50, "{file}: {} banks", banks.len());
        for name in ["_321_NGP", "321_NGP", "Go_NGP", "HBEAT_ZCHANGE", "HBEAT"] {
            let id = name_hash(name);
            assert!(
                !banks
                    .iter()
                    .any(|b| b.objects_with_id(id).any(|o| o.kind == Kind::Event)),
                "{file}: an event named {name} exists"
            );
        }
        checked += 1;
    }
    assert_eq!(checked, 2);
}

/// The start plays the events the executable posts, `<cue>__Weapons_NGP`, as
/// their timelines: the VO and UI "three, two, one" on the action delays 400,
/// 1,300 and 2,150 ms, the legacy beeps on their initial delays 0.521, 1.375 and
/// 2.229 s, and the ship layers cut by the `Stop` at 3,100 ms with a 50 ms fade.
/// Evidence: `docs/ghidra/functions/ps4-omega-eu/wwise-sound.md` ("The countdown,
/// end to end").
#[test]
#[ignore = "needs the decrypted Omega packages in data/extracted/ps4/"]
fn the_start_plays_its_wwise_events_as_timelines() {
    let Some(source) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let mut archives = oag_omega::open(&source.display().to_string()).expect("the Omega source");
    let race = oag_omega::TITLE.race;
    let voice = race.countdown_voice.expect("Omega voices its start");
    assert!(voice.event_every_mode, "every mode plays the event pair");
    let mut banks = oag_sound::sfx::Banks::default();
    banks.load_countdown(&mut archives, race.sounds.speech, voice);
    let line = |name: &str| {
        banks
            .report
            .iter()
            .find(|l| l.starts_with(&format!("sfx: {name} -> ")))
            .unwrap_or_else(|| panic!("{name} not loaded: {:?}", banks.report))
            .clone()
    };
    assert!(
        line("_321_NGP__Weapons_NGP").contains("17 voice(s)"),
        "{:?}",
        banks.report
    );
    assert!(
        line("Go_NGP__Weapons_NGP").contains("6 voice(s)"),
        "{:?}",
        banks.report
    );

    let mut rng = oag_core::Rng::new(1);
    let ready = banks
        .voices(oag_gameplay::cue::Cue::Ready, &mut rng)
        .expect("the start's first event");
    let onset = |t: f64| ready.iter().filter(|v| (v.delay - t).abs() < 1e-3).count();
    for (t, voices) in [
        (0.4, 2),
        (1.3, 2),
        (2.15, 2),
        (0.521, 2),
        (1.375, 2),
        (2.229, 2),
    ] {
        assert_eq!(onset(t), voices, "voices keyed at {t} s");
    }
    let ends: Vec<f64> = ready
        .iter()
        .map(|v| v.delay + v.sound.frames() as f64 / f64::from(v.sound.sample_rate()))
        .collect();
    assert!(
        ends.iter()
            .filter(|&&end| (end - 3.15).abs() < 0.002)
            .count()
            >= 3,
        "the three ship layers end at the Stop plus its fade: {ends:?}"
    );
}

/// The missile launch is the event `NGP_MissileFire__Weapons_NGP`, a relay to
/// `WPN_MISSILE_Fire`, which plays one layer container: four random containers
/// and a sound. A post sounds five media, one per layer, and the seed decides
/// which child each random container gives. The player's `_PC` event is not a
/// twin: it plays its own sound, relays to a second event 100 ms later and sets
/// a state, so it is not loaded here.
#[test]
#[ignore = "needs the decrypted Omega packages in data/extracted/ps4/"]
fn the_missile_launch_plays_one_child_of_each_random_layer() {
    use oag_gameplay::cue::Cue;
    let Some(source) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let mut archives = oag_omega::open(&source.display().to_string()).expect("the Omega source");
    let entry = oag_omega::TITLE
        .race
        .countdown_voice
        .and_then(|v| v.event)
        .expect("Omega's start event")
        .bank;
    let mut media_of = |seed: u64, event: &str| {
        let mut banks = oag_sound::sfx::Banks::default();
        let mut rng = oag_core::Rng::new(seed);
        assert!(
            banks.load_wwise_event(&mut archives, entry, Cue::Rocket, event, &mut rng),
            "{event}: {:?}",
            banks.report
        );
        assert!(
            banks.report[0].contains("5 voice(s)") && banks.report[0].contains("4 container(s)"),
            "{event}: {:?}",
            banks.report
        );
        let mut draw = oag_core::Rng::new(1);
        let voices = banks.voices(Cue::Rocket, &mut draw).expect("loaded");
        assert_eq!(voices.len(), 5, "{event}: one media per layer");
        assert!(
            voices.iter().all(|v| (0.0..1.0).contains(&v.delay)),
            "{event}: the layers' initial delays are under a second"
        );
        let mut frames: Vec<usize> = voices.iter().map(|v| v.sound.frames()).collect();
        frames.sort_unstable();
        frames
    };
    let mut picks = std::collections::BTreeSet::new();
    for seed in 0..32 {
        picks.insert(media_of(seed, "NGP_MissileFire__Weapons_NGP"));
    }
    assert!(picks.len() > 1, "32 seeds all picked the same media");
    let a = media_of(7, "NGP_MissileFire__Weapons_NGP");
    assert_eq!(
        a,
        media_of(7, "NGP_MissileFire__Weapons_NGP"),
        "a seed is repeatable"
    );
}

/// The race cues Omega posts as Wwise events, out of [`oag_gameplay::cue::Cue::ALL`]'s 43: each
/// launcher, impact and pickup's cue string and bank slot, read in the 1.07 executable
/// (`docs/ghidra/functions/ps4-omega-eu/wwise-cues.md`). The other 17 are held loops (loop flags
/// unread), legacy events the executable never posts, or the collision and missile ship-hit
/// choices that are unread.
const POSTED: [oag_gameplay::cue::Cue; 26] = {
    use oag_gameplay::cue::Cue;
    [
        Cue::SpeedupPad,
        Cue::Message,
        Cue::Turbo,
        Cue::Absorb,
        Cue::ShieldActive,
        Cue::Engaging,
        Cue::Disengaging,
        Cue::MineLaunch,
        Cue::Plasma,
        Cue::PlasmaHitWall,
        Cue::PlasmaHitShip,
        Cue::Rocket,
        Cue::RocketHitWall,
        Cue::RocketHitShip,
        Cue::Missile,
        Cue::MissileHitWall,
        Cue::Cannon,
        Cue::CannonHitWall,
        Cue::CannonHitShip,
        Cue::QuakeLaunch,
        Cue::QuakeHit,
        Cue::Leach,
        Cue::LeachFail,
        Cue::LeachEnergy,
        Cue::ShurikenLaunch,
        Cue::Repulsor,
    ]
};

/// The posted cues whose event loads for every emitter: 10 of the 26. The rest reach a switch
/// container (its group value is the game's, refused at play), media the ATRAC9 decoder rejects
/// (`QUAKELAUNCH`), or no sound in the shipped banks (`NGP_LEACH`, whose player event does load).
const WIRED: [oag_gameplay::cue::Cue; 10] = {
    use oag_gameplay::cue::Cue;
    [
        Cue::Message,
        Cue::Absorb,
        Cue::MineLaunch,
        Cue::Plasma,
        Cue::Rocket,
        Cue::Missile,
        Cue::CannonHitShip,
        Cue::LeachFail,
        Cue::LeachEnergy,
        Cue::Repulsor,
    ]
};

/// An Omega race load plays its posted cues from their events, `<cue>__<slot>` for every emitter
/// and `<cue>_PC__<slot>` for the player's, through the race loader. Every posted cue either
/// loads or names the limit that stops it; no held loop resolves to a one-shot.
#[test]
#[ignore = "needs the decrypted Omega packages in data/extracted/ps4/"]
fn a_race_plays_its_posted_cues_from_their_wwise_events() {
    use oag_gameplay::cue::Cue;
    let Some(source) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: source.display().to_string(),
        mode: oag_race::Mode::SingleRace,
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("an Omega race loads");
    let sounds = &loaded.setup.sounds;
    let report: Vec<&String> = sounds
        .report
        .iter()
        .filter(|l| l.contains("__") || l.contains("Wwise"))
        .collect();
    let mut rng = oag_core::Rng::new(1);
    let mut plays = |cue: Cue, player: bool| {
        sounds
            .voices_for(cue, player, &mut rng)
            .is_some_and(|v| !v.is_empty())
    };
    let silent: Vec<Cue> = WIRED
        .iter()
        .copied()
        .filter(|&cue| !plays(cue, false) || !plays(cue, true))
        .collect();
    assert!(
        silent.is_empty(),
        "{} of {} wired cues silent: {silent:?}\n{report:#?}",
        silent.len(),
        WIRED.len()
    );
    assert!(
        !plays(Cue::Leach, false) && plays(Cue::Leach, true),
        "NGP_LEACH plays for the player alone: {report:#?}"
    );
    for held in [Cue::RocketTravel, Cue::Shield, Cue::Magstrip] {
        assert!(
            !plays(held, false),
            "{held:?} is a held loop and stays unplayed"
        );
    }
    assert!(
        report
            .iter()
            .any(|l| l.starts_with("sfx: 10 of 43 race cues play their Wwise event")),
        "{report:#?}"
    );
    assert!(
        !sounds
            .report
            .iter()
            .any(|l| l.contains("no Wwise cue read for it")),
        "every cue of the 43 has a row: {report:#?}"
    );
    // The player's missile is its own event: a `SetState` beside the relay.
    assert!(
        report
            .iter()
            .any(|l| l.starts_with("sfx: NGP_MissileFire_PC__Weapons_NGP: action kind 0x1204")),
        "{report:#?}"
    );
    // Each posted cue that does not play names the limit that stops it.
    let limits = [
        "SwitchContainer",
        "does not decode",
        "starts no sound that plays",
    ];
    for cue in POSTED.iter().filter(|c| !WIRED.contains(c)) {
        let named = report.iter().any(|l| {
            l.contains(" not loaded: ")
                && limits.iter().any(|why| l.contains(why))
                && line_is_for(l, *cue)
        });
        assert!(
            named,
            "{cue:?} neither plays nor names a limit: {report:#?}"
        );
    }
}

/// Whether report line `l` is about `cue`'s posted event.
fn line_is_for(l: &str, cue: oag_gameplay::cue::Cue) -> bool {
    let row = oag_omega::race::WWISE_CUES
        .cues
        .iter()
        .find(|(name, _)| oag_sound::sfx::CueTables::name(cue) == *name);
    match row {
        Some((_, oag_title::WwiseCue::Posts { cue, slot })) => {
            l.starts_with(&format!("sfx: {cue}__{slot} "))
        }
        _ => false,
    }
}
