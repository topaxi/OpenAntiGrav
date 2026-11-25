//! Omega's Eliminator, from a real race: its own score law, its own Racebox target and the
//! rows its HUD draws.
//!
//! **`#[ignore]`d and never run in CI.** It needs the decrypted PS4 packages.
//!
//! The law is on `docs/ghidra/functions/ps4-omega-eu/eliminator.md`.

use oag_gameplay::PlayerInputs;
use oag_ui::frontend::Draw;

fn loaded() -> Option<oag_raceplay::Loaded> {
    let source = oag_testdata::exact("data/extracted/ps4")?;
    Some(
        oag_raceplay::load(&oag_raceplay::Options {
            source: source.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::Eliminator,
            seed: Some(1),
            no_effects: true,
            ..oag_raceplay::Options::default()
        })
        .expect("loading an Omega Eliminator"),
    )
}

#[test]
#[ignore = "needs the extracted package under data/extracted/ps4/"]
fn an_omega_eliminator_is_won_on_a_score_off_its_own_slider() {
    let Some(loaded) = loaded() else { return };
    assert_eq!(
        loaded.setup.score_law,
        Some(oag_race::ScoreLaw {
            damage: 0.1,
            kill: 20.0,
            lap: 5.0,
            death: 5.0,
            // Omega's `WeaponAIstats.xml` carries no `<EliminatorAIStats>` row.
            opponent_scale: 1.0,
        })
    );
    // `Eliminations` slider's `minSlide` in `DATA09`'s `racebox_definition.xml`.
    assert_eq!(loaded.setup.eliminator_kill_target, Some(50));

    let context = loaded.hud.context().expect("Omega's layout parses");
    let mut race = oag_raceplay::Race::start(loaded.setup, loaded.view, loaded.modules);
    race.tick(&PlayerInputs::none());
    let readout = race.readout();
    assert!(readout.score_rows, "the rows read a score, not kills");
    let list = oag_hud::draw_list(&context, &readout);
    let text: Vec<&str> = list
        .hud_text
        .iter()
        .chain(&list.small_text)
        .chain(&list.default_text)
        .chain(&list.skin_text)
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        text.iter()
            .any(|t| t.contains("TARGET") && t.contains("( 50 )")),
        "the header: {text:?}"
    );
    let rows = text.iter().filter(|t| t.ends_with("( 0 )")).count();
    assert!(rows >= 8, "a row per craft: {text:?}");
}
