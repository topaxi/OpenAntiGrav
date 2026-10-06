//! Hardware-free tests for the cue layer: [`Banks`] is built by hand so
//! [`Engine`]'s law and [`Banks::pick`] are checked without game content (the
//! disc-backed half is `crates/game/tests/sfx_ground_truth.rs`).

use std::sync::Arc;

use oag_audio::Sound;

use super::*;

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
    // Drawing for a choice of one would make every other cue's stream depend on
    // how many alternates this one has.
    assert_eq!(rng.snapshot(), before);
}

#[test]
fn a_cue_with_alternates_reaches_all_of_them() {
    let banks = banks(&[(Cue::Collision, 15, false)]);
    // Every alternate is one frame long, so told apart by identity.
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
    // `0x19`'s decoded handler re-rolls once, advancing to the next alternate and
    // wrapping, when the draw matches the cue's last play (`sound.md`'s `0x19`
    // section); `Scream_DoGrain` never plays the same alternate twice running.
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
    // The narrowest case for the `(draw + 1) % len` wrap: with two alternates,
    // "never repeat" leaves no choice past the first play, so a wrap bug repeats
    // almost at once.
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
            // One spelling reaches a WAD and a PSARC alike (`oag_assets::psarc`
            // folds case and separators).
            assert!(cue.bank().entry(banks, false).starts_with(r"Data\Sound\"));
            assert!(cue.bank().entry(banks, true).starts_with(r"Data\Sound\"));
        }
    }
    // Zone moves the ship bank only: `SHIP_ZM` is the only bank with a Zone
    // counterpart on either Pulse disc.
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

    // HD moves two, and repeats for Zone (no separate Zone ship bank): the whole
    // per-title difference.
    let hd = oag_hd::race::SOUND_BANKS;
    assert_eq!(BankName::Hud.entry(hd, false), r"Data\Sound\weapons.bnk");
    assert_eq!(BankName::Ship.entry(hd, false), r"Data\Sound\shiphd.bnk");
    assert_eq!(
        BankName::Ship.entry(hd, true),
        BankName::Ship.entry(hd, false)
    );
    // Pure spells them exactly as Pulse does: the finding, not the assumption.
    assert_eq!(oag_pure::race::SOUND_BANKS, pulse);
    // The cues this port holds a handle to, and only those: a held cue must not
    // also fire as a one-shot from the drain loop. The original keeps `~ENGINE`
    // at `flare+0x7c`, `~SHIELD` at `entity+0x54`, `~BLOWUP` at `craft+0xcac`.
    // `PlasmaTravel`, `RocketTravel`, `MissileTravel` and `ShurikenTravel` are
    // per projectile slot (`TravelVoices`); `LeachAttach` once per race on
    // `SfxVoices::leach_attach`; `Autopilot` level-driven at
    // `SfxVoices::autopilot`; `Magstrip` per grid slot, started and ended by the
    // `Magstrip` and `MagstripStop` events (`sfx::magstrip`).
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
    // `hud.bnk`'s `~BLOWUP` is one looping waveform of two and `~AIRBRAKE_MONO`
    // one of three, so the flag cannot collapse to the cue. Nothing wired is
    // mixed, hence a test rather than something a reader would notice.
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
    // No cue may be *unstated*: `Unplaced` means "call site not read" and must
    // be reached deliberately, not by an unrevisited match arm. Six are
    // `Unplaced`: `ShieldActive` (recorded as a full-volume pan-zero play) and
    // `Disengaging` (read as the no-emitter path), both announcer lines;
    // `Blowup` (the player's craft, same path); `LockOn` (a HUD sound at
    // `0x400`, no emitter argument); `Autopilot` and `Engaging` (the same path's
    // callers in `Ship_FireHeldWeapon`'s held-id-6 case).
    const DRY: [Cue; 7] = [
        Cue::Message,
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
    // `ShipCollisionFx_Trigger` passes `craft+0x50` unbranched, so the player's
    // hull is positional too (always close).
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
    // The `racer+0x368` split: the player dry at full volume, a rival through its
    // emitter (`Placement::CraftUnlessPlayer`).
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
    // Not played dry: a rival's collision at full volume the moment its slot
    // empties is louder and more wrong than silence.
    assert!(place(CueEvent::new(Cue::Collision, 5), &ears(), &grid(10.0)).is_none());
    assert!(place(CueEvent::new(Cue::Collision, 200), &ears(), &grid(10.0)).is_none());
}

#[test]
fn an_unplaced_cue_survives_a_slot_that_has_no_craft() {
    // The announcer has no emitter at all, so it must not be gated by one.
    let empty = [None; oag_gameplay::MAX_SHIPS];
    assert!(place(CueEvent::new(Cue::ShieldActive, 0), &ears(), &empty).is_some());
}
