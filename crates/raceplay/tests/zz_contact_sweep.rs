//! Sweeps over HD's contact cues, printed and never asserted: how many `.COLLISIONS` and
//! `.GRIND` starts a Fury race raises, and how long each grind voice lasts
//! (`docs/ghidra/functions/ps3-hdfury-eu/collision-cues.md`, "Grind held back").
//!
//! Both tests return at once unless `OAG_SWEEP` is set. `OAG_SWEEP_GRIND=1` plays the held-back
//! grind, `OAG_SWEEP_REARMED=1` runs the re-armed law (what the title did before the impulse
//! law), `OAG_SWEEP_WEAVE=<0..1>` swings the stick with a 2.5 s period, `OAG_SWEEP_TICKS=1`
//! prints each of the player's events.

use oag_gameplay::PlayerInputs;
use oag_gameplay::cue::Cue;
use oag_gameplay::input::{Button, Input};
use oag_raceplay as race;
use std::collections::BTreeMap;

#[test]
#[ignore = "a printing sweep: set OAG_SWEEP to run it"]
fn zz_contact_sweep() {
    if std::env::var("OAG_SWEEP").is_err() {
        return;
    }
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mut loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        team: Some("Feisar".to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    if std::env::var("OAG_SWEEP_REARMED").is_ok() {
        loaded.setup.contact_cues = oag_title::contact_cue::ContactCues::Rearmed;
    } else if std::env::var("OAG_SWEEP_GRIND").is_ok()
        && let oag_title::contact_cue::ContactCues::Impulse(law) = &mut loaded.setup.contact_cues
    {
        law.value.grind_played = true;
    }
    let weave: f32 = std::env::var("OAG_SWEEP_WEAVE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);
    let mut race = race::Race::start(loaded.setup);
    let mut tally: BTreeMap<String, u32> = BTreeMap::new();
    let mut ticks_with: BTreeMap<String, u32> = BTreeMap::new();
    for tick in 0..(oag_race::COUNTDOWN_TICKS + 60 * 60) {
        let mut buttons = Input::new();
        buttons.begin_frame(Button::Cross.bit());
        buttons.begin_frame(Button::Cross.bit());
        let phase = (tick as f32 / 150.0 * std::f32::consts::TAU).sin();
        let input = oag_gameplay::InputSnapshot {
            buttons,
            stick_x: weave * phase,
            ..oag_gameplay::InputSnapshot::new()
        };
        race.tick(&PlayerInputs::single(input));
        let mut seen = std::collections::BTreeSet::new();
        for event in race.sim.drain_cues() {
            if !matches!(
                event.cue,
                Cue::Collision | Cue::Grind | Cue::CollisionShield | Cue::GrindShield
            ) {
                continue;
            }
            let who = if event.slot == 0 { "player" } else { "other" };
            let key = format!("{:?} {who}", event.cue);
            if std::env::var("OAG_SWEEP_TICKS").is_ok() && event.slot == 0 {
                println!("EVT {tick} {:?} {:?}", event.cue, event.contact);
            }
            *tally.entry(key.clone()).or_default() += 1;
            seen.insert(key);
        }
        for key in seen {
            *ticks_with.entry(key).or_default() += 1;
        }
    }
    println!(
        "SWEEP rearmed={} weave={weave}: {tally:?}",
        std::env::var("OAG_SWEEP_REARMED").is_ok()
    );
}

#[test]
#[ignore = "a printing sweep: set OAG_SWEEP to run it"]
fn zz_grind_voice_lengths() {
    if std::env::var("OAG_SWEEP").is_err() {
        return;
    }
    use oag_sound::sfx::Banks;
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let opened =
        oag_source::title::open_source(&image.display().to_string(), Vec::new(), Vec::new())
            .expect("opening the source");
    let sounds = opened.title.race.sounds;
    let tick = opened
        .title
        .race
        .zone_announcer
        .map_or(oag_title::SequenceTick::Unknown, |z| z.tick);
    let mut archives = opened.archives;
    let banks = Banks::load(&mut archives, sounds, false, tick);
    let mut rng = oag_core::Rng::new(5);
    for cue in [Cue::Grind, Cue::Collision] {
        for severity in 0..=2i8 {
            for surface in 0..=1i8 {
                let persp = if cue == Cue::Grind { 0 } else { 1 };
                let mut lens = Vec::new();
                for _ in 0..20 {
                    if let Some(v) =
                        banks.contact_voices(cue, [severity, surface, persp, 0], &mut rng)
                    {
                        for x in v {
                            lens.push(format!(
                                "{:.2}s{}+{:.2}",
                                x.sound.frames() as f64 / x.sound.sample_rate() as f64,
                                if x.looping { "L" } else { "" },
                                x.delay
                            ));
                        }
                    }
                }
                lens.sort();
                lens.dedup();
                println!("VOICES {cue:?} sev={severity} surf={surface}: {lens:?}");
            }
        }
    }
}
