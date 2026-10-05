//! Wipeout 2048's crossfaded engine, played off the real package.
//!
//! **`#[ignore]`d and never run in CI.** They need the decrypted Vita package
//! under `data/extracted/vita/`. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test sfx_xfade_2048_ground_truth --run-ignored all
//! ```
//!
//! What it proves: each of the five `<team>2048` tables, and Zone's five, loads
//! through the title's own [`oag_title::Crossfade`] axis and binds its looping
//! cues in the bank the axis names; a craft at rest sounds through a real
//! mixer; and the kind-2 layer's modulation reaches the idle layer. What it
//! does not prove: that the result matches the original by ear, and **it is
//! half the engine** - the gearbox cues (`~ShipN_*Acc_*`) the executable plays
//! beside these tables are not wired.

use std::sync::Arc;

use oag_audio::{Listener, Mixer};
use oag_sound::sfx::{Banks, XfadeCraft, XfadeInputs, XfadeSource};

const SAMPLE_RATE: u32 = 48_000;
const TICK_HZ: u32 = 60;

/// The slot-team strings a 2048 grid carries: the team and its livery.
const TEAMS: [&str; 5] = [
    r"AG_Systems2048\1",
    r"Auricom2048\2",
    r"Feisar2048\3",
    r"Piranha2048\4",
    r"Qirex2048\5",
];

fn load(zone: bool) -> Option<(Banks, Vec<String>)> {
    let root = oag_testdata::exact("data/extracted/vita/PCSF00007")?;
    let opened =
        oag_source::title::open_source(&root.display().to_string(), Vec::new(), Vec::new())
            .expect("opening the source");
    let sounds = opened.title.race.sounds;
    let crossfade = sounds.crossfade.expect("2048 names its crossfade banks");
    let mut archives = opened.archives;
    let mut banks = Banks::load(
        &mut archives,
        sounds,
        zone,
        oag_title::SequenceTick::Unknown,
    );
    let mut report = Vec::new();
    let teams: Vec<String> = TEAMS.iter().map(|t| (*t).to_string()).collect();
    banks.load_xfade(
        &mut archives,
        XfadeSource {
            bank: if zone {
                crossfade.ship_zone
            } else {
                crossfade.ship
            },
            infix: crossfade.zone_infix.filter(|_| zone).unwrap_or(""),
        },
        &teams,
        &mut report,
    );
    Some((banks, report))
}

fn ears() -> Listener {
    Listener {
        position: [0.0; 3],
        right: [1.0, 0.0, 0.0],
    }
}

fn tick(
    craft: &mut XfadeCraft,
    mixer: &mut Mixer,
    speed_field: f32,
    throttle: f32,
    out: &mut Vec<f32>,
) {
    craft.tick(
        mixer,
        XfadeInputs {
            speed_field,
            throttle: Some(throttle),
        },
        true,
        [0.0; 3],
        &ears(),
        false,
        1.0 / TICK_HZ as f32,
    );
    mixer.render_tick(TICK_HZ, out);
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn every_team_table_loads_with_its_looping_layers_in_both_modes() {
    for (zone, expected) in [(false, [3, 3, 3, 3, 3]), (true, [4, 3, 3, 4, 4])] {
        let Some((banks, report)) = load(zone) else {
            return;
        };
        for line in &report {
            println!("{line}");
        }
        for (team, playable) in TEAMS.iter().zip(expected) {
            let table = banks
                .xfade_team(team)
                .unwrap_or_else(|| panic!("{team} (zone {zone}): no table loaded"));
            assert_eq!(table.playable(), playable, "{team} (zone {zone})");
        }
    }
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn a_craft_at_rest_sounds_and_the_pedal_bends_the_idle_layer_through_the_modulator() {
    let Some((banks, _)) = load(false) else {
        return;
    };
    let team = banks.xfade_team(r"Feisar2048\3").expect("the feisar table");
    let mut mixer = Mixer::new(SAMPLE_RATE);
    let mut craft = XfadeCraft::new(Arc::clone(&team));
    let mut out = Vec::new();
    for _ in 0..120 {
        tick(&mut craft, &mut mixer, 0.0, 0.0, &mut out);
    }
    let rms = (out.iter().map(|s| s * s).sum::<f32>() / out.len() as f32).sqrt();
    println!("at rest: rms {rms:.4}, {} voice(s)", craft.open_voices());
    assert!(rms > 0.001, "the standing craft is silent");
    assert!(craft.open_voices() >= 1);

    let idle = craft.level(0).expect("layer 0");
    // The modulator (layer 3, kind 2) multiplies layer 0 by 646 / 1024 and bends
    // it by its pitch curve at channel 4's value, which is zero at rest.
    assert!((idle.gain - 646.0 / 1024.0).abs() < 1e-4, "{idle:?}");
    for _ in 0..120 {
        tick(&mut craft, &mut mixer, 0.0, 100.0, &mut out);
    }
    let pressed = craft.level(0).expect("layer 0");
    println!("idle layer: rest {idle:?}, pedal down {pressed:?}");
    assert!(
        pressed.bend > idle.bend,
        "the pedal did not bend the idle layer"
    );
}
