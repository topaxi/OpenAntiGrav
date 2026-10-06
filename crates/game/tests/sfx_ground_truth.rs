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

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;
use oag_sound::sfx::{Banks, Cue};

/// The discs this runs against, and what each is called in a failure message.
const DISCS: [(&str, &str); 5] = [
    ("pulse-psp-usa.chd", "Pulse PSP USA"),
    ("pulse-psp-eu.chd", "Pulse PSP EU"),
    ("pulse-ps2-eu.chd", "Pulse PS2 EU"),
    ("pure-psp-usa.chd", "Pure PSP USA"),
    ("pure-psp-eu.chd", "Pure PSP EU"),
];

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

/// Opens a source the way a race does, and loads its banks.
///
/// **Takes the bank table off whichever title the source turned out to be**,
/// exactly as `race::load` does. Hard-coding Pulse's here would make the HD leg
/// below test the wrong paths and pass for the wrong reason.
fn banks(image: &Path, zone: bool) -> Banks {
    let opened =
        oag_source::title::open_source(&image.display().to_string(), Vec::new(), Vec::new())
            .expect("opening the source");
    let sounds = opened.title.race.sounds;
    let mut archives = opened.archives;
    Banks::load(
        &mut archives,
        sounds,
        zone,
        opened
            .title
            .race
            .zone_announcer
            .map_or(oag_title::SequenceTick::Unknown, |z| z.tick),
    )
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

/// **Correction, verified against a real disc rather than `oag-wad sounds`
/// alone.** The handover thread that wired `Cue::CannonHitShip` read
/// `CANNONEXPLSHIP` as an empty, zero-waveform cue off `oag-wad sounds`'
/// own listing - true of `cue_sounds`, the direct command run alone, but
/// `Banks::load` resolves through `Bank::cue_tree_sounds` instead.
/// `CANNONEXPLSHIP` owns exactly one command in `Data.wad`'s weapon bank,
/// opcode `0x05` (one of `oag_formats::sblk::child::CHILD_OPCODES`), whose
/// record indexes cue 37 - `CANNONEXPLWALL` - directly: read straight off
/// `Bank::cue_children`/`Bank::resolve_child` against a real disc, not
/// inferred from the waveform count alone. So a craft hit plays the same
/// nine waveforms a wall hit does, by the disc's own construction, and the
/// cue is not silent after all; nothing here needs an exception for it any
/// more.
///
/// Cues Pure's own `weapons.bnk` does not carry at all, because Pure ships
/// none of the four weapons that name them (the Cannon, the LeachBeam, the
/// Shuriken and the Repulser). `pure-psp-usa.chd` and `pure-psp-eu.chd`'s own weapon bank
/// was read with `oag-wad sounds` and carries no `CANNON*`, `LEACH*` or
/// `SHURIKEN*` entry of any kind, and no `REPULSOR*` either; Pure's own weapon
/// in their place is the Disruptor.
const NOT_ON_PURE: [Cue; 13] = [
    Cue::MissileExpire,
    Cue::Cannon,
    Cue::CannonHitWall,
    Cue::CannonHitShip,
    Cue::Leach,
    Cue::LeachFail,
    Cue::LeachAttach,
    Cue::LeachEnergy,
    Cue::ShurikenLaunch,
    Cue::ShurikenHit,
    Cue::ShurikenTravel,
    Cue::Repulsor,
    Cue::RepulsorHit,
];

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

        let on_pure = label.starts_with("Pure");
        let mut rng = oag_core::Rng::new(1);
        for cue in Cue::ALL {
            // HD-lineage only: the PSP and PS2 banks carry no `~magstrip01`.
            if cue == Cue::Magstrip {
                assert!(
                    banks.pick(cue, &mut rng).is_none(),
                    "{label}: ~magstrip01 loaded, so the cue is not HD-only"
                );
                continue;
            }
            if on_pure && NOT_ON_PURE.contains(&cue) {
                assert!(
                    banks.pick(cue, &mut rng).is_none(),
                    "{label}: {} loaded, but Pure ships no weapon that names it - \
                     NOT_ON_PURE is stale",
                    cue.name()
                );
                continue;
            }
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
            // `oag-wad sounds ...:PSP_GAME/USRDIR/Data.wad --cue MINELAUNCH`
            // reports one waveform, no loop bit - see `mine.md`'s 2026-09-06
            // section.
            Cue::MineLaunch,
        ] {
            for _ in 0..64 {
                let (_, loops) = banks.pick(cue, &mut rng).expect("cue");
                assert!(!loops, "{label}: {} loops and should not", cue.name());
            }
        }
    }
    assert!(ran > 0, "no disc image was present");
}

/// Wipeout HD loads every cue this port fires but one, and says why it
/// misses that one.
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
///
/// `~ROCKLOCK` is the newest addition, and a different kind of gain from
/// `~SHIELD` growing two waveforms: **every** one of its waveforms is in
/// `oag_formats::sblk::NOT_ADPCM_FLAG`'s second codec, so before
/// `oag_formats::sblk::decode_pcm16` existed it failed the loader's
/// `waveforms.is_empty()` check entirely and did not even reach this list.
/// Its presence here is the not-PS-ADPCM path's clearest end-to-end proof:
/// a cue that could not have loaded any other way.
///
/// **`MINELAUNCH` is the newest addition, and it resolves on HD's disc with no
/// per-title work at all - `weapons.bnk` carries it, 88 waveforms.** That is
/// bank presence, not a confirmed HD trigger: `Cue::MineLaunch`'s own doc
/// comment cites only `psp-pulse-usa`'s `Mine_Init`, and `crates/sound/src/sfx.rs`'s
/// module doc already states the confidence-50 bet this rides on - a title in
/// the same series with the same cue names is likely to fire them at the same
/// moments, and nothing here has looked at HD's own weapon-fire dispatch to
/// check.
///
/// **The Plasma's four cues (`PLASMA`, `~PLASMATVL`, `PLASMAHITWALL`,
/// `PLASMAHITSHIP`) are the newest addition, found stale here 2026-09-16 by
/// the multiplayer-prerequisite merge's own verification pass rather than by
/// the commit that wired them** (`687d8743`, weeks earlier - this test is
/// `#[ignore]`d, so `just` stayed green the whole time, the same trap the
/// `MINELAUNCH` paragraph above already names). All four resolve on HD's own
/// `weapons.bnk` with no per-title work: 4, 3, 5 and 15 waveforms
/// respectively. Bank presence only, the same caveat as `MINELAUNCH`'s -
/// `Cue::Plasma`/`PlasmaTravel`/`PlasmaHitWall`/`PlasmaHitShip`'s own doc
/// comments cite `psp-pulse-usa` evidence alone, confidence 90 for the *PSP*
/// trigger; nothing here has looked at HD's own Plasma dispatch to confirm it
/// fires the same four cues at the same moments.
///
/// **The Rocket's, the Missile's, the Cannon's, `QUAKEHIT`'s, the LeachBeam's
/// and the Shuriken's fourteen cues are the newest addition, and every one
/// of them resolves on HD's own `weapons.bnk` with no per-title work at
/// all** - `CANNONEXPLSHIP` included, at 8 waveforms; see
/// `every_wired_cue_resolves_on_every_psp_and_ps2_disc`'s own doc comment
/// for the correction that cue's reading needed once this test ran against a
/// real disc rather than `oag-wad sounds` alone. Bank presence only, the
/// same caveat as every addition above: each cue's own doc comment in
/// `oag_sound::sfx::Cue` cites `psp-pulse-usa` evidence alone, and
/// nothing here has looked at HD's own dispatch for any of the six weapons.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn wipeout_hd_loads_every_cue_but_one_and_reports_the_miss() {
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
    // **Eight since `oag_formats::sblk::decode_pcm16` identified HD's second
    // codec**, up from seven on 2026-08-24: `disengaging` and `~BLOWUP`
    // resolve on HD's own banks with no per-title work at all, the same way
    // the first five did, and `~ROCKLOCK` newly resolves because its
    // waveforms are all in the second codec and none decoded before. `~ENGINE`
    // is still the one miss - HD's ship audio is a per-event `c_*` set - see
    // `oag_title::SoundBanks`. **`LEACHENERGY`, `~AUTOPILOT`, `autopilot_eng`,
    // `ROCKET` and `QUAKELAUNCH` all joined the loaded set the same day this
    // port wired them** (2026-09-25) - all five were already present in HD's
    // own `weapons.bnk`/`speech.bnk`, this test just never asked for any of
    // them until then.
    //
    // **Five more on 2026-09-30**, when their triggers were recovered
    // (`MISSILEEXPSHIP`, `SHURIKENEXPL`, `~QUAKETRAVEL`, `LEACHFAIL`,
    // `SHURIKEN`): each is already in HD's own `weapons.bnk`.
    //
    // **Two more on 2026-10-04**, `REPULSOR` and `REPULSORHIT`, wired with the
    // Repulser; HD's own `weapons.bnk` carries both.
    //
    // **`TURBO` on 2026-10-04**, the perfect start's cue
    // (`ExhaustFlare_OnPerfectStart`, Pulse `0x08904fd4`): HD's `weapons.bnk`
    // names it with 15 waveforms. Nothing raises it on HD, which applies no
    // launch boost.
    //
    // **`MESSAGE` on 2026-10-06**, the HUD message line's cue
    // (`Hud_UpdateMessages`, Pulse `0x0881f148`): HD's `weapons.bnk` names it
    // with 6 waveforms. HD's own message dispatch is unread (the lines run on
    // Pulse's law there), so this is bank presence only.
    //
    // **This test is `#[ignore]`d, so `just` stayed green while it was stale.**
    // `disengaging` was added a commit earlier and this list was not updated
    // with it; the failure surfaced only on the next `--run-ignored all`. The
    // same shape as the `--reel` trap on HANDOVER: a guard that does not run in
    // the gate is a guard that lags.
    assert_eq!(
        loaded,
        vec![
            "SPEEDUPPAD",
            "MESSAGE",
            "TURBO",
            ".COLLISIONS",
            "ABSORB",
            "~SHIELD",
            "shieldactive",
            "~AUTOPILOT",
            "autopilot_eng",
            "disengaging",
            "~BLOWUP",
            "~ROCKLOCK",
            "MINELAUNCH",
            "PLASMA",
            "~PLASMATVL",
            "PLASMAHITWALL",
            "PLASMAHITSHIP",
            "ROCKET",
            "~ROCKETTVL",
            "ROCKEXPLWALL",
            "ROCKEXPLSHIP",
            "MISSILE",
            "~MISSILETVL",
            "MISSILEEXPWALL",
            "MISSILEEXPSHIP",
            "SHURIKENEXPL",
            "CANNON",
            "CANNONEXPLWALL",
            "CANNONEXPLSHIP",
            "QUAKELAUNCH",
            "QUAKEHIT",
            "~QUAKETRAVEL",
            "LEACH",
            "LEACHFAIL",
            "~LEACHATTACH",
            "LEACHENERGY",
            "SHURIKEN",
            "SHURIKENHIT",
            "~SHURIKENTRAVEL",
            "REPULSOR",
            "REPULSORHIT",
            // 2026-10-05, the magstrip hum: `shiphd.bnk`, a 35-waveform tree.
            "~magstrip01",
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
    // The not-PS-ADPCM path, working end to end: `~SHIELD` binds eight
    // waveforms now, not the six it bound while the second codec was
    // undecoded - the other two used to be silently dropped.
    assert!(
        says(r"~SHIELD -> 8 waveform(s) from Data\Sound\weapons.bnk"),
        "no HD cue shows the second codec's waveforms decoding"
    );
    // `~ROCKLOCK` could not have loaded at all before `decode_pcm16`: every
    // one of its waveforms is in the second codec, so `load_named_cue` would
    // have found `waveforms` empty and refused it outright.
    assert!(
        says(r"~ROCKLOCK -> 2 waveform(s) from Data\Sound\weapons.bnk"),
        "the all-PCM cue did not load"
    );
    // `MINELAUNCH` is bank presence, not a confirmed HD trigger - see this
    // test's own doc comment - but the bank read itself is checkable, the
    // same way every cue above is.
    assert!(says(
        r"MINELAUNCH -> 88 waveform(s) from Data\Sound\weapons.bnk"
    ));
    // The Plasma's four cues, bank presence only - see this test's own doc
    // comment for the same caveat `MINELAUNCH` carries.
    assert!(says(r"PLASMA -> 4 waveform(s) from Data\Sound\weapons.bnk"));
    assert!(says(
        r"~PLASMATVL -> 3 waveform(s) from Data\Sound\weapons.bnk"
    ));
    assert!(says(
        r"PLASMAHITWALL -> 5 waveform(s) from Data\Sound\weapons.bnk"
    ));
    assert!(says(
        r"PLASMAHITSHIP -> 15 waveform(s) from Data\Sound\weapons.bnk"
    ));
}

/// Structural checks - the bank names these cues, in this order, at these
/// indices - are `hd_title_ground_truth.rs`'s job. This is the layer above,
/// on the same terms as [`wipeout_hd_loads_every_cue_but_one_and_reports_the_miss`]:
/// that a cue reported as loaded actually decodes to a waveform, not a report
/// line that reads well and a silent voice.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn wipeout_hd_s_speed_class_announcer_decodes_all_fourteen_cues() {
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
    let opened =
        oag_source::title::open_source(&path.display().to_string(), Vec::new(), Vec::new())
            .expect("opening HD");
    let mut archives = opened.archives;
    let announcer =
        oag_sound::sfx::ClassAnnouncer::load(&mut archives, opened.title.race.zone_class_announcer);
    for line in &announcer.report {
        println!("{line}");
    }
    assert!(!announcer.is_empty(), "the class announcer loaded nothing");

    let mut rng = oag_core::Rng::new(3);
    let decoded: Vec<u32> = (1..=14)
        .filter(|&stage| announcer.pick(stage, &mut rng).is_some())
        .collect();
    // **All fourteen, since `oag_formats::sblk::decode_pcm16` identified HD's
    // second codec.** Stage 11 (`MR_SUZ`, "Super Zen") used to be the one
    // miss: both of its waveforms are in that codec, and unlike stages 12-14
    // it has no PS-ADPCM alternate to fall back to, so it was silent until
    // the second codec decoded - not a wiring bug, and not invented around.
    // `docs/formats/psp-audio.md`'s "A third of HD's waveforms are not
    // PS-ADPCM" section has the evidence.
    assert_eq!(
        decoded,
        vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14],
        "HD's speed-class announcer's decodable set changed"
    );
    let says = |needle: &str| announcer.report.iter().any(|l| l.contains(needle));
    assert!(
        says("MR_SUZ (stage 11) -> 2 waveform(s)"),
        "the once-silent class stopped decoding both its waveforms"
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
    // says so: fifteen samples of one event, the longest under twice the
    // shortest (0.58 s to 0.99 s at the 15,569 Hz their descriptors key them
    // on with). A cue whose commands were meant to play *together* would be
    // a stack of different lengths. A ratio rather than a difference in
    // seconds, so the bound means the same thing whatever rate the bank
    // plays at - the earlier `< 0.2 s` was calibrated on a placeholder
    // 44,100 Hz and broke the day the real rate landed.
    let (shortest, longest) = (lengths[0], lengths[lengths.len() - 1]);
    assert!(
        longest < shortest * 2.0,
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
    for cue in Cue::ALL.into_iter().filter(|&c| c != Cue::Magstrip) {
        let (sound, looping) = banks.pick(cue, &mut rng).expect("cue");
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
        // Every one-shot effect on either disc is under three seconds; a cue
        // that came back longer would have resolved to a span running past
        // its own waveform into its neighbours. A held loop is exempt: it is
        // meant to be heard for as long as its holder lives, not once, and
        // `~SHURIKENTRAVEL`'s own 6.4s waveform is the longest of them -
        // still one clean span, just a longer one.
        assert!(
            looping || seconds < 3.0,
            "{} decoded to {seconds:.2}s, longer than any one-shot effect on the disc",
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
    let mut audio = oag_sound::Audio::open(
        &oag_sound::settings::Settings::default(),
        Some(std::path::PathBuf::from("/dev/null")),
        None,
        oag_audio::MIN_BUFFER,
        false,
    );
    let voices =
        |audio: &oag_sound::Audio| audio.output().with_mixer(|mixer| mixer.active_voices());

    assert_eq!(voices(&audio), 0, "something was playing before the race");
    for _ in 0..120 {
        race.tick(&PlayerInputs::none());
        oag_game::sound::race_tick(&mut audio, &mut race);
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
    race.sim.world.primary_race_mut().finished = true;
    for _ in 0..600 {
        // Deliberately **not** calling `race.tick` - that is exactly what the
        // finished arm does not do.
        oag_game::sound::race_tick(&mut audio, &mut race);
        audio.tick();
        // The other half of the composition root's per-tick pair: with the
        // null backend this is the only thing that renders, and a voice that is
        // never rendered never *ends* - so without it a fired one-shot would
        // sit in the pool for the whole test and every count below would be
        // measuring the wrong thing.
        audio.tick();
    }
    // Everything still sounding is the *circuit's*, not the craft's. The track
    // ambience deliberately outlives the flag: the circuit is still loaded
    // under the results table and `VexSound_Init`'s cues were never tied to the
    // race's state in the first place. See `audio::sfx::TrackEmitters`.
    assert_eq!(
        voices(&audio),
        audio.ambient_voices(),
        "the engine is still sounding ten seconds after the race ended"
    );
}

/// A destroyed craft opens `~BLOWUP` and lets it go when the explosion ends.
///
/// The level-driven half of `Ship_SetState`'s case 4, off the real `hud.bnk`
/// waveform rather than a fixture: the cue is looping, so a port that fired it
/// as a one-shot or never released it would drone under whatever came next.
/// `docs/ghidra/functions/psp-pulse-usa/zone-mode.md`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_destroyed_craft_sounds_and_stops_sounding() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: path.display().to_string(),
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    let mut audio = oag_sound::Audio::open(
        &oag_sound::settings::Settings::default(),
        Some(std::path::PathBuf::from("/dev/null")),
        None,
        oag_audio::MIN_BUFFER,
        false,
    );
    let voices =
        |audio: &oag_sound::Audio| audio.output().with_mixer(|mixer| mixer.active_voices());

    for _ in 0..30 {
        race.tick(&PlayerInputs::none());
        oag_game::sound::race_tick(&mut audio, &mut race);
        audio.tick();
    }
    let engine_only = voices(&audio);
    assert!(engine_only >= 1, "the engine never opened");
    assert!(
        !race.craft_is_exploding(),
        "the fixture blew up before the test began"
    );

    // Set directly - reaching zero shield honestly is `oag_physics`'s business
    // and needs a wall this fixture has no reason to build. What is under test
    // is the audio layer reading the state.
    race.sim.world.ships[0].physics.craft_state = oag_physics::CraftState::Destroyed;
    oag_game::sound::race_tick(&mut audio, &mut race);
    audio.tick();
    assert!(
        voices(&audio) > engine_only,
        "the craft blew up and nothing sounded"
    );

    // And it is released when the state ends rather than looping for ever.
    //
    // **Rendered out rather than checked on the next tick**, because
    // `.pick` draws between two alternates and only one of them loops: the
    // non-looping one is played and forgotten, so it is still sounding a tick
    // later either way. A second of audio separates the two answers - a
    // one-shot is 0.37 s and has ended, and a loop that was not released never
    // will.
    race.sim.world.ships[0].physics.craft_state = oag_physics::CraftState::Eliminated;
    for _ in 0..60 {
        oag_game::sound::race_tick(&mut audio, &mut race);
        audio.tick();
    }
    assert_eq!(
        voices(&audio),
        engine_only,
        "`~BLOWUP` is still sounding after the explosion ended - it loops, so \
         nothing else will stop it"
    );
}

/// The whole grid sounds, and it does not all sound from the same place.
///
/// The end-to-end half of positional audio: unit tests check the law
/// (`oag_audio::spatial`) and the wiring (`oag_sound::sfx`), and neither
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
    let mut loaded = loaded;
    // **The circuit's own ambience is dropped from this fixture on purpose.**
    // What is under test below is the *panner*, measured in the rendered
    // samples, and the circuit's authored emitters sit on the same bus with
    // positions of their own - so leaving them in would make a claim about one
    // rival's stereo position into a claim about wherever the start line
    // happens to be. `track_audio_ground_truth` is where they are tested.
    loaded.setup.track_emitters = oag_sound::sfx::TrackEmitters::default();
    // **The start-of-race voice is dropped from this fixture.** `ready` opens
    // voices from tick 91 and this counts them; what it counts is the held
    // loops, and the start voice has its own file
    // (`countdown_voice_ground_truth`).
    loaded.setup.countdown_voice = false;
    let mut race = race::Race::start(loaded.setup);
    // The pan ratio below is calibrated against the far chase view: the player's own
    // engine sits at the listener's feet and its share of the energy grows as the
    // eye nears it. The default became the close view on 2026-10-01 (a fresh Pulse
    // profile starts on `OPT_CLOSE`), so the view this was written for is named.
    race.set_camera_view(oag_display::display::CameraView::Far);
    assert!(
        race.ship_count() > 1,
        "the fixture raced alone, so nothing below is testing anything"
    );

    let mut audio = oag_sound::Audio::open(
        &oag_sound::settings::Settings::default(),
        Some(std::path::PathBuf::from("/dev/null")),
        None,
        oag_audio::MIN_BUFFER,
        false,
    );
    for _ in 0..120 {
        race.tick(&PlayerInputs::none());
        oag_game::sound::race_tick(&mut audio, &mut race);
        audio.tick();
    }

    // **The circuit's own ambience is subtracted, not switched off.** A race
    // on a real circuit opens a held voice for every authored emitter within
    // its radius (`oag_sound::sfx::TrackEmitters`), and those are not
    // engines. Counting them here would make this assertion a function of
    // which circuit the default happens to be.
    let voices = audio
        .output()
        .with_mixer(|mixer| mixer.active_voices())
        .saturating_sub(audio.ambient_voices());
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
    race.sim.world.ships[1].physics.body.position = camera.w_axis.truncate() + right * 10.0;
    oag_game::sound::race_tick(&mut audio, &mut race);

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
/// to inspect, and the only way to see it is to watch the pool. The
/// Autopilot's own pair (`~AUTOPILOT`/`autopilot_eng`) is the same shape,
/// off `Race::autopilot_is_active` - its own test is in
/// `sfx_weapon_ground_truth.rs`, moved there under the 1,000-line rule.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_shield_opens_a_held_voice_and_closes_it_when_the_pickup_expires() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut loaded = race::load(&race::Options {
        source: path.display().to_string(),
        ..race::Options::default()
    })
    .expect("loading the race");
    // **The start-of-race voice is dropped from this fixture.** `ready` opens
    // voices from tick 91 and this counts them; what it counts is the held
    // loops, and the start voice has its own file
    // (`countdown_voice_ground_truth`).
    loaded.setup.countdown_voice = false;
    let mut race = race::Race::start(loaded.setup);
    let mut audio = oag_sound::Audio::open(
        &oag_sound::settings::Settings::default(),
        Some(std::path::PathBuf::from("/dev/null")),
        None,
        oag_audio::MIN_BUFFER,
        false,
    );
    let voices =
        |audio: &oag_sound::Audio| audio.output().with_mixer(|mixer| mixer.active_voices());

    // Settle first, so the engine's own voice is already open and the counts
    // below are differences rather than absolutes.
    for _ in 0..60 {
        race.tick(&PlayerInputs::none());
        oag_game::sound::race_tick(&mut audio, &mut race);
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
    // the pad table instead. Ten seconds, so it outlasts the wait below for
    // `shieldactive` to end; a one-second timer used to do, and expired on
    // its own inside that wait once the line played at its real rate.
    race.sim.world.ships[0].physics.shield_pickup_timer = 10.0;
    race.tick(&PlayerInputs::none());
    oag_game::sound::race_tick(&mut audio, &mut race);
    assert!(
        voices(&audio) > idle,
        "the shield came up and opened no voice"
    );

    // Held across ticks rather than re-triggered - it is a `~` cue. The
    // shield coming up also fires `shieldactive`, a one-shot voice line, so
    // the pool is walked until that has ended and only the two loops are left over
    // idle: 60 ticks used to be enough at the placeholder 44,100 Hz, and at
    // the 18,002 Hz its descriptor keys it on with the line runs past a
    // second. Three seconds is longer than any effect on the disc, so a pool
    // still above `idle + 1` after that is a re-triggered loop, not a slow
    // one-shot.
    let held = voices(&audio);
    let mut settled = None;
    for tick in 0..180 {
        race.tick(&PlayerInputs::none());
        oag_game::sound::race_tick(&mut audio, &mut race);
        // The other half of the composition root's per-tick pair: with the
        // null backend this is the only thing that renders, and a voice that is
        // never rendered never *ends* - so without it a fired one-shot would
        // sit in the pool for the whole test and every count below would be
        // measuring the wrong thing.
        audio.tick();
        assert!(
            voices(&audio) <= held,
            "the shield loop is being re-triggered every tick"
        );
        // Two loops now: Pulse's `~SHIELD` keys both of its waveforms at
        // tick zero and both loop, where the flat pick held one of them.
        if voices(&audio) == idle + 2 {
            settled = Some(tick);
            break;
        }
    }
    let settled =
        settled.expect("the shield's one-shot line never ended, or the loops are not two voices");
    println!("shield held with only its loop open after {settled} ticks");

    race.sim.world.ships[0].physics.shield_pickup_timer = 0.0;
    race.tick(&PlayerInputs::none());
    oag_game::sound::race_tick(&mut audio, &mut race);
    assert_eq!(
        voices(&audio),
        idle,
        "the shield dropped and its voice kept sounding"
    );
}

/// `~AUTOPILOT` and `autopilot_eng` through the whole path, the same shape
/// [`the_shield_opens_a_held_voice_and_closes_it_when_the_pickup_expires`]
/// already proves for the shield: `Cue::Autopilot` is a level
/// (`Race::autopilot_is_active`), not a queued edge, so watching the pool is
/// the only way to see it open and close on the right tick.
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
    // Pulse also loads the start-of-race voice from the mode's own speech bank,
    // one line for `ready` and one for `go`.
    assert_eq!(
        sfx.len(),
        Cue::ALL.len() + Cue::COUNTDOWN.len(),
        "the load report is incomplete"
    );
    assert!(
        !loaded.setup.sounds.is_empty(),
        "a real disc loaded no cues"
    );
}
