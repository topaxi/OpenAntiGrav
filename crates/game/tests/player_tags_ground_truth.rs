//! Pulse PSP's Multiplayer Tags on a real circuit: a two-person race puts each person's name
//! over the other's craft, and the option off puts nothing.
//!
//! The law is `oag_hud::player_tags` (`docs/ghidra/functions/psp-pulse-usa/hud-game-options.md`).
//! This is the other half: that the option reaches the readout of a race that is loaded for real.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game player_tags_ground_truth:: --run-ignored all
//! ```

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

const IMAGE: &str = "data/images/pulse-psp-usa.chd";

fn two_people(tags: bool) -> Option<race::Race> {
    let image = oag_testdata::image(IMAGE)?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        views: 2,
        opponents: true,
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    oag_game::split_screen::apply(&mut race, 2, oag_render::viewport::Cut::Horizontal);
    race.set_player_tags(tags);
    Some(race)
}

/// Every tag either view shows over ten seconds of racing: `(viewer, name, alpha)`.
fn tags_seen(race: &mut race::Race) -> Vec<(usize, String, f32)> {
    let mut seen = Vec::new();
    for _ in 0..600 {
        race.tick(&PlayerInputs::none());
        for viewer in 0..2 {
            for tag in race.readout_for(viewer).player_tags {
                seen.push((viewer, tag.name, tag.alpha));
            }
        }
    }
    seen
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_option_on_names_each_person_over_the_other_and_off_names_nobody() {
    let Some(mut on) = two_people(true) else {
        return;
    };
    let seen = tags_seen(&mut on);
    assert!(
        !seen.is_empty(),
        "no tag was ever visible with the option on"
    );
    assert!(
        seen.iter()
            .any(|(viewer, name, _)| *viewer == 0 && name == "Player 2"),
        "view 0 never names the second person: {seen:?}"
    );
    assert!(
        seen.iter().all(|(viewer, name, alpha)| {
            *alpha > 0.0
                && *alpha <= 1.0
                && (*viewer, name.as_str()) != (0, "Player 1")
                && (*viewer, name.as_str()) != (1, "Player 2")
        }),
        "a view tags its own craft or draws an invisible tag: {seen:?}"
    );

    let mut off = two_people(false).unwrap();
    assert_eq!(
        tags_seen(&mut off),
        Vec::new(),
        "tags drew with the option off"
    );
}

/// The lobby's names win over the stop-gap `Player N`.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_lobby_name_replaces_the_stop_gap() {
    let Some(mut race) = two_people(true) else {
        return;
    };
    race.set_person_names(vec!["ada".into(), "grace".into()]);
    let seen = tags_seen(&mut race);
    assert!(
        seen.iter()
            .any(|(viewer, name, _)| *viewer == 0 && name == "grace")
    );
    assert!(seen.iter().all(|(_, name, _)| name != "Player 2"));
}

/// The face the tags are drawn in comes off the disc's `MPTag_HUD.xml`, and the eight labels
/// share it.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_eight_plrtag_labels_share_one_face() {
    let Some(image) = oag_testdata::image(IMAGE) else {
        return;
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("open");
    let composed = oag_hud::compose(oag_pulse::hud::layouts::MP_TAG, |path| {
        archives.read_name(path).ok()
    })
    .expect("MPTag_HUD.xml composes");
    let first = composed.layout.label("PlrTag0").expect("PlrTag0").clone();
    for n in 1..8 {
        let label = composed.layout.label(&format!("PlrTag{n}")).expect("label");
        assert_eq!(
            (label.font, label.scale, label.align, label.color),
            (first.font, first.scale, first.align, first.color),
            "PlrTag{n}"
        );
    }
}
