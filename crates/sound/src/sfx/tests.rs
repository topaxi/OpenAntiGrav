//! Hardware-free tests for the cue layer.
//!
//! Nothing here touches a disc: [`Banks`] is built by hand so the law in
//! [`Engine`] and the choice in [`Banks::pick`] can be checked without game
//! content. The disc-backed half is
//! `crates/game/tests/sfx_ground_truth.rs`.

use std::sync::Arc;

use oag_audio::Sound;

use super::*;

/// A `Loaded` holding `count` distinguishable one-frame sounds.
fn loaded(count: usize, looping: bool) -> Loaded {
    Loaded {
        waveforms: (0..count)
            .map(|i| {
                (
                    Arc::new(Sound::new(vec![i as i16 + 1], 1, 44_100).expect("sound")),
                    looping,
                )
            })
            .collect(),
    }
}

fn banks(entries: &[(Cue, usize, bool)]) -> Banks {
    Banks {
        sounds: entries
            .iter()
            .map(|&(cue, count, looping)| (cue, loaded(count, looping)))
            .collect(),
        report: Vec::new(),
        ..Default::default()
    }
}

#[test]
fn a_cue_with_one_waveform_never_consults_the_generator() {
    let banks = banks(&[(Cue::SpeedupPad, 1, false)]);
    let mut rng = Rng::new(1);
    let before = rng.snapshot();
    let (sound, looping) = banks.pick(Cue::SpeedupPad, &mut rng).expect("loaded");
    assert_eq!(sound.frames(), 1);
    assert!(!looping);
    // Drawing a number for a choice of one would make every other cue's stream
    // depend on how many alternates this one happens to have.
    assert_eq!(rng.snapshot(), before);
}

#[test]
fn a_cue_with_alternates_reaches_all_of_them() {
    let banks = banks(&[(Cue::Collision, 15, false)]);
    // Every alternate is one frame long, so they are told apart by identity
    // rather than by content.
    let mut rng = Rng::new(7);
    let mut pointers = std::collections::BTreeSet::new();
    for _ in 0..600 {
        let (sound, _) = banks.pick(Cue::Collision, &mut rng).expect("loaded");
        pointers.insert(Arc::as_ptr(&sound) as usize);
    }
    assert_eq!(pointers.len(), 15, "some alternate is never chosen");
}

#[test]
fn a_cue_with_alternates_never_repeats_the_previous_pick() {
    // `0x19`'s decoded handler re-rolls once, by advancing to the next
    // alternate and wrapping, whenever the draw matches the cache from the
    // cue's last play - see `sound.md`'s own `0x19` section. `Scream_DoGrain`
    // never plays the same alternate on two consecutive calls for one cue.
    let banks = banks(&[(Cue::Collision, 15, false)]);
    let mut rng = Rng::new(7);
    let mut previous = None;
    for _ in 0..600 {
        let (sound, _) = banks.pick(Cue::Collision, &mut rng).expect("loaded");
        let identity = Arc::as_ptr(&sound) as usize;
        if let Some(previous) = previous {
            assert_ne!(
                identity, previous,
                "the same alternate played twice in a row"
            );
        }
        previous = Some(identity);
    }
}

#[test]
fn a_two_alternate_cue_strictly_alternates() {
    // The narrowest case the `(draw + 1) % len` wrap has to get right: with
    // only two alternates, "never repeat" leaves no choice at all past the
    // first play, so a wrapping bug would show up as a repeat here almost
    // immediately rather than after hundreds of draws.
    let banks = banks(&[(Cue::Collision, 2, false)]);
    let mut rng = Rng::new(11);
    let mut previous = None;
    for _ in 0..50 {
        let (sound, _) = banks.pick(Cue::Collision, &mut rng).expect("loaded");
        let identity = Arc::as_ptr(&sound) as usize;
        if let Some(previous) = previous {
            assert_ne!(identity, previous, "a two-alternate cue repeated itself");
        }
        previous = Some(identity);
    }
}

#[test]
fn an_unloaded_cue_is_silent_rather_than_substituted() {
    let banks = banks(&[(Cue::SpeedupPad, 1, false)]);
    let mut rng = Rng::new(1);
    assert!(banks.pick(Cue::Collision, &mut rng).is_none());
    assert!(banks.pick(Cue::Engine, &mut rng).is_none());
}

#[test]
fn every_cue_names_a_bank_and_a_string() {
    for cue in Cue::ALL {
        assert!(!cue.name().is_empty());
        for banks in [oag_pulse::race::SOUND_BANKS, oag_hd::race::SOUND_BANKS] {
            // One spelling reaches a WAD and a PSARC alike, because
            // `oag_assets::psarc` folds case and separators.
            assert!(cue.bank().entry(banks, false).starts_with(r"Data\Sound\"));
            assert!(cue.bank().entry(banks, true).starts_with(r"Data\Sound\"));
        }
    }
    // Zone moves the ship bank and nothing else, because `SHIP_ZM` is the only
    // bank with a Zone counterpart on either Pulse disc.
    let pulse = oag_pulse::race::SOUND_BANKS;
    assert_eq!(BankName::Ship.entry(pulse, false), r"Data\Sound\ship.bnk");
    assert_eq!(
        BankName::Ship.entry(pulse, true),
        r"Data\Sound\ship_zone.bnk"
    );
    assert_eq!(
        BankName::Hud.entry(pulse, true),
        BankName::Hud.entry(pulse, false)
    );
    assert_eq!(
        BankName::Weapons.entry(pulse, true),
        BankName::Weapons.entry(pulse, false)
    );

    // HD moves two of them, and repeats itself for Zone because it ships no
    // separate Zone ship bank. This is the whole of the per-title difference.
    let hd = oag_hd::race::SOUND_BANKS;
    assert_eq!(BankName::Hud.entry(hd, false), r"Data\Sound\weapons.bnk");
    assert_eq!(BankName::Ship.entry(hd, false), r"Data\Sound\shiphd.bnk");
    assert_eq!(
        BankName::Ship.entry(hd, true),
        BankName::Ship.entry(hd, false)
    );
    // Pure spells them exactly as Pulse does - the finding, not the assumption.
    assert_eq!(oag_pure::race::SOUND_BANKS, pulse);
    // The cues this port holds a handle to, and only those: a held cue must
    // not also be fired as a one-shot from the drain loop. The first three
    // are ones the original opens with an out-parameter and keeps -
    // `~ENGINE` at `flare+0x7c`, `~SHIELD` at `entity+0x54`, `~BLOWUP` at
    // `craft+0xcac`. `PlasmaTravel`, `RocketTravel`, `MissileTravel` and
    // `ShurikenTravel` are held too, but per *projectile* slot rather than
    // as a single race-wide handle - see `TravelVoices`'s own doc comment.
    // `LeachAttach` is held once for the whole race, on
    // `SfxVoices::leach_attach`, since the LeachBeam is a world-wide single
    // instance rather than a projectile slot. `Autopilot` joins `Blowup`'s
    // shape: a level-driven handle at `SfxVoices::autopilot`, not a
    // projectile slot. `Magstrip` is held per grid slot, started and ended by
    // the `Magstrip` and `MagstripStop` events - see `sfx::magstrip`.
    let held: Vec<Cue> = Cue::ALL.into_iter().filter(|c| c.held()).collect();
    assert_eq!(
        held,
        vec![
            Cue::Engine,
            Cue::Shield,
            Cue::Autopilot,
            Cue::Blowup,
            Cue::LockOn,
            Cue::PlasmaTravel,
            Cue::RocketTravel,
            Cue::MissileTravel,
            Cue::QuakeTravel,
            Cue::LeachAttach,
            Cue::ShurikenTravel,
            Cue::Magstrip,
        ]
    );
}

#[test]
fn a_bank_that_mixes_looping_and_one_shot_waveforms_keeps_them_apart() {
    // `hud.bnk`'s `~BLOWUP` is one looping waveform of two and
    // `~AIRBRAKE_MONO` one of three, so the flag cannot be collapsed to the
    // cue. Nothing wired today is mixed, which is exactly why this is a test
    // rather than something the next reader would notice.
    let mixed = Loaded {
        waveforms: vec![
            (
                Arc::new(Sound::new(vec![1], 1, 44_100).expect("sound")),
                false,
            ),
            (
                Arc::new(Sound::new(vec![2], 1, 44_100).expect("sound")),
                true,
            ),
        ],
    };
    let banks = Banks {
        sounds: [(Cue::Collision, mixed)].into_iter().collect(),
        report: Vec::new(),
        ..Default::default()
    };

    let mut rng = Rng::new(23);
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..200 {
        let (_, looping) = banks.pick(Cue::Collision, &mut rng).expect("loaded");
        seen.insert(looping);
    }
    assert_eq!(seen.len(), 2, "the per-waveform flag was collapsed");
}

/// The listener used by the placement tests: at the origin, `+X` to its right.
fn ears() -> oag_audio::Listener {
    oag_audio::Listener::at_origin()
}

/// A grid where slot 0 is on the listener and slot 1 is `x` units to the right.
fn grid(x: f32) -> [Option<(Vec3, f32)>; oag_gameplay::MAX_SHIPS] {
    let mut craft = [None; oag_gameplay::MAX_SHIPS];
    craft[0] = Some((Vec3::ZERO, 0.0));
    craft[1] = Some((Vec3::new(x, 0.0, 0.0), 0.0));
    craft
}

#[test]
fn every_cue_states_where_it_is_heard_from() {
    // The point is that no cue is *unstated*. `Unplaced` is a legitimate answer
    // and it means "the call site has not been read", so it must be reached
    // deliberately rather than by a match arm nobody revisited.
    // Six are `Unplaced` and each for its own reason: `ShieldActive`'s call
    // site is recorded as a full-volume pan-zero play, and `Disengaging`'s is
    // *read* as the no-emitter path - both announcer lines. `Blowup` is the
    // player's own craft through the same no-emitter path, and `LockOn` is a
    // HUD sound about the player's own reticle, opened at `0x400` with no
    // emitter argument at all. `Autopilot` and `Engaging` are the same
    // no-emitter path's other two callers, both inside `Ship_FireHeldWeapon`'s
    // case for held id 6.
    const DRY: [Cue; 6] = [
        Cue::ShieldActive,
        Cue::Autopilot,
        Cue::Engaging,
        Cue::Disengaging,
        Cue::Blowup,
        Cue::LockOn,
    ];
    for cue in Cue::ALL {
        let placement = cue.placement();
        if DRY.contains(&cue) {
            assert_eq!(placement, Placement::Unplaced);
        } else {
            assert_ne!(
                placement,
                Placement::Unplaced,
                "{} has a read call site and should be placed",
                cue.name()
            );
        }
    }
}

#[test]
fn an_opponents_collision_arrives_from_the_side_it_happened_on() {
    let placed = place(CueEvent::new(Cue::Collision, 1), &ears(), &grid(60.0)).unwrap();
    assert_eq!(
        placed.pan,
        Some(1.0),
        "a rival on the right sounded centred"
    );
    assert!(placed.gain < 1.0, "60 units away and still at full volume");
}

#[test]
fn the_players_own_collision_is_placed_too() {
    // `ShipCollisionFx_Trigger` passes `craft+0x50` with no branch, so the
    // player's hull is positional as well - it is simply always close.
    let placed = place(CueEvent::new(Cue::Collision, 0), &ears(), &grid(60.0)).unwrap();
    assert_eq!(placed.pan, Some(0.0));
    assert_eq!(placed.gain, 1.0);
}

#[test]
fn a_rival_beyond_the_craft_radius_is_refused_rather_than_played_quietly() {
    let far = grid(oag_audio::Emitter::CRAFT_RADIUS + 1.0);
    assert!(place(CueEvent::new(Cue::Collision, 1), &ears(), &far).is_none());
}

#[test]
fn the_pad_cue_splits_the_player_off_from_the_field() {
    // The two-branch split on `racer+0x368`: the player dry at full volume, a
    // rival through its own emitter. See `Placement::CraftUnlessPlayer`.
    let craft = grid(60.0);
    let player = place(CueEvent::new(Cue::SpeedupPad, 0), &ears(), &craft).unwrap();
    assert_eq!(player.pan, None);
    assert_eq!(player.gain, 1.0);

    let rival = place(CueEvent::new(Cue::SpeedupPad, 1), &ears(), &craft).unwrap();
    assert_eq!(rival.pan, Some(1.0));
    assert!(rival.gain < 1.0);
}

#[test]
fn the_announcer_is_left_dry() {
    let placed = place(CueEvent::new(Cue::ShieldActive, 0), &ears(), &grid(0.0)).unwrap();
    assert_eq!(placed.pan, None);
    assert_eq!(placed.gain, 1.0);
}

#[test]
fn a_cue_from_a_slot_with_no_craft_is_dropped() {
    // Not played dry: a rival's collision arriving at full volume the moment
    // its slot empties is louder and more wrong than silence.
    assert!(place(CueEvent::new(Cue::Collision, 5), &ears(), &grid(10.0)).is_none());
    assert!(place(CueEvent::new(Cue::Collision, 200), &ears(), &grid(10.0)).is_none());
}

#[test]
fn an_unplaced_cue_survives_a_slot_that_has_no_craft() {
    // The announcer has no emitter at all, so it must not be gated by one.
    let empty = [None; oag_gameplay::MAX_SHIPS];
    assert!(place(CueEvent::new(Cue::ShieldActive, 0), &ears(), &empty).is_some());
}
