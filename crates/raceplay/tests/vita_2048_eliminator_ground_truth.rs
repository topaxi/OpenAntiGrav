//! 2048's Eliminator on a real race: its score law and targets, the pads' class colours and its
//! own craft's flip time.
//!
//! **`#[ignore]`d and never run in CI.** It needs the extracted Vita package.
//!
//! `docs/ghidra/functions/ps3-hdfury-eu/eliminator-score.md`, "Other titles", and
//! `eliminator-flip.md`, "Per title".

use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input, InputSnapshot};

fn press_flip() -> PlayerInputs {
    let mut buttons = Input::EMPTY;
    buttons.begin_frame(Button::Flip.bit());
    PlayerInputs::single(InputSnapshot {
        buttons,
        ..InputSnapshot::EMPTY
    })
}

/// The pads cycle by class: yellow, green and red (`docs/rendering/pads.md`, "2048 and Omega").
fn check_pads(setup: &oag_raceplay::SimSetup) {
    assert_eq!(
        setup.weapon_pad_glow,
        &oag_title::weapon_pad::WeaponPadGlow::ByClass(oag_title::weapon_pad::ClassCycles::LINEAGE)
    );
}

fn check_score(setup: &oag_raceplay::SimSetup, target: u32) {
    let law = setup.score_law.expect("won on a score");
    assert_eq!(
        (law.damage, law.kill, law.lap, law.death),
        (0.1, 20.0, 5.0, 5.0)
    );
    assert_eq!(setup.eliminator_kill_target, Some(target));
}

/// `SP.xml`'s "2048 - Event 4-2" authors score bars of 50 and 100; the race ends on the elite
/// one.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn a_2048_elimination_event_races_to_its_elite_score() {
    let Some(source) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    let options = oag_raceplay::Options {
        source: source.display().to_string(),
        class: "VENOM".to_string(),
        seed: Some(1),
        no_effects: true,
        ..oag_raceplay::Options::default()
    };
    let (sim, progress) = oag_raceplay::load_event_sim(&options, "2048 - Event 4-2")
        .expect("loading a 2048 Elimination event");
    check_score(&sim.setup, 100);
    let objectives = progress.objectives.expect("the event authors both bars");
    assert_eq!(
        (objectives.pass.target, objectives.elite.target),
        (Some(50), Some(100))
    );
}

/// `Auricom2048`'s prototype authors `eliminator_flip_time="0.5"`, so the half turn is thirty
/// ticks, not the default twelve.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn a_2048_craft_turns_over_its_own_flip_time() {
    let Some(source) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: source.display().to_string(),
        class: "VENOM".to_string(),
        team: Some(r"Auricom2048\4".to_string()),
        mode: oag_race::Mode::Eliminator,
        seed: Some(1),
        no_effects: true,
        ..oag_raceplay::Options::default()
    })
    .expect("loading a 2048 Eliminator");
    check_pads(&loaded.setup);
    // A grid's variant id names its team on the rows.
    assert_eq!(loaded.setup.slot_teams[0], r"Auricom2048\4");
    assert_eq!(loaded.setup.row_teams[0], "Auricom2048");
    // No event and no Racebox slider: the manager's own default.
    check_score(&loaded.setup, 200);
    let mut race = oag_raceplay::Race::start(loaded.setup, loaded.view, loaded.modules);
    assert!(race.core.sim.l1_is_flip(), "2048's L1 is the flip");
    for _ in 0..60 * 6 {
        race.tick(&PlayerInputs::none());
    }
    let own = race.readout().kill_tags.into_iter().find(|t| t.player);
    assert_eq!(own.map(|t| t.team).as_deref(), Some("Auricom2048"));
    race.tick(&press_flip());
    assert!(race.core.sim.world.ships[0].physics.flip.active);
    let mut ticks = 1;
    while race.core.sim.world.ships[0].physics.flip.active && ticks < 120 {
        race.tick(&PlayerInputs::none());
        ticks += 1;
    }
    assert_eq!(ticks, 30);
}
