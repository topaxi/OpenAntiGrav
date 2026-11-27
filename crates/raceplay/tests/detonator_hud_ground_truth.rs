//! A Detonator race draws its own HUD, and each widget reads the cannon.
//!
//! **The disc-backed tests are `#[ignore]`d and never run in CI.** They need game content, which
//! this project does not ship.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^detonator_hud_ground_truth::/)'
//! ```
//!
//! The bindings are `docs/ghidra/functions/ps3-hdfury-eu/detonator-hud.md`, "The answer in one
//! table". The test stands a race on a known `Cannon`, reads the HUD's `Readout` and draws it
//! through the title's own layout, so it fails when the layout is not the Detonator one, when a
//! widget stops reading its value, when a widget the original never draws is drawn, or when the
//! atlas does not resolve (an invisible HUD with a green readout).

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;
use oag_ui::frontend::Draw;

const HD_IMAGE: &str = "data/images/hdfury-ps3-eu-dec.iso";
const OMEGA_IMAGE: &str = "data/images/omega-ps4-eu.pkg";

/// What the cannon holds in these tests: ten volleys left, five volleys down the clip that each
/// hit, a score before the streak pays, and three stages laid.
const SCORE: i32 = 12_345;

/// The score after the five-volley streak pays on the next tick.
const PAID: i32 = SCORE + oag_weapons::detonator::cannon::STREAK_FIVE;

#[test]
fn a_detonator_race_names_the_detonator_layout_on_every_title_that_races_it() {
    use oag_race::Mode::Detonator;
    assert_eq!(
        race::hud_layout(oag_hd::hud::LAYOUTS, Detonator),
        "/data/xml/detonator_hud.xml"
    );
    assert_eq!(
        race::hud_layout(oag_omega::hud::LAYOUTS, Detonator),
        r"Data\XML\Detonator_HUD.xml"
    );
    assert_eq!(
        race::hud_layout(oag_2048::hud::LAYOUTS, Detonator),
        r"Data\XML\2048_hud\Detonator_HUD_HD.xml"
    );
    assert_eq!(
        race::hud_layout(oag_2048::hud::HD_CAMPAIGN, Detonator),
        r"Data\XML\2048_hud\Detonator_HUD_HD.xml"
    );
    for skin in oag_hd::hud::SKINS {
        assert_eq!(
            race::hud_layout(skin.layouts, Detonator),
            "/data/xml/detonator_hud.xml",
            "{}: the disc has no skinned Detonator layout",
            skin.name
        );
    }
}

fn load(image: &str) -> Option<race::Loaded> {
    let path = oag_testdata::image(image)?;
    Some(
        race::load(&race::Options {
            source: path.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::Detonator,
            ..race::Options::default()
        })
        .expect("loading a Detonator race"),
    )
}

/// The race one tick past the start gate, the craft at full shield.
fn racing(loaded: &race::Loaded) -> race::Race {
    let mut race = race::Race::start(
        loaded.setup.clone(),
        loaded.view.clone(),
        loaded.modules.clone(),
    );
    while oag_race::RaceState::thrust_gated(race.core.sim.world.tick) {
        race.tick(&PlayerInputs::none());
    }
    race
}

/// The race on the known cannon, one tick on (the streak pays), and its HUD readout.
fn known_cannon(loaded: &race::Loaded) -> oag_hud::Readout {
    let mut race = racing(loaded);
    {
        let world = &mut race.core.sim.world;
        world.detonator.stages_laid = 3;
        let cannon = &mut world.detonator.cannon;
        cannon.clip = 10;
        cannon.score = SCORE;
        world.detonator.emp.charge = 5;
        for volley in 0..5 {
            cannon.hits[2 * volley] = 2;
            cannon.hits[2 * volley + 1] = 1;
        }
    }
    race.tick(&PlayerInputs::none());
    race.readout()
}

/// How many sprites `readout` draws with its clip and hit mask replaced.
fn sprites_with(
    context: &oag_hud::Context<'_>,
    readout: &oag_hud::Readout,
    clip: u8,
    hit: u16,
) -> usize {
    let mut changed = readout.clone();
    let shown = changed.detonator.as_mut().expect("a readout");
    (shown.clip, shown.hit) = (clip, hit);
    oag_hud::draw_list(context, &changed).sprites.len()
}

fn texts(frame: &oag_hud::Frame) -> Vec<String> {
    let mut out: Vec<String> = [
        &frame.hud_text,
        &frame.small_text,
        &frame.default_text,
        &frame.skin_text,
    ]
    .into_iter()
    .flatten()
    .filter_map(|draw| match draw.unstretched() {
        Draw::Text { text, .. } => Some(text.clone()),
        _ => None,
    })
    .collect();
    out.sort();
    // The title's text shadow draws each string twice.
    out.dedup();
    out
}

/// Everything the Detonator HUD says about a loaded title, on a cannon with ten volleys left,
/// five that hit, the score above and three stages laid.
fn the_detonator_hud_reads_the_cannon(image: &str, localised_streak: bool, emp_ready: bool) {
    let Some(loaded) = load(image) else {
        return;
    };
    let report = loaded.report.join("\n");
    let context = loaded
        .hud
        .context()
        .unwrap_or_else(|| panic!("the Detonator layout did not load:\n{report}"));
    for name in [
        "DetonatorScore",
        "PulseBullets",
        "DetonatorStage",
        "PulseBonus",
        "ShieldBarText",
    ] {
        assert!(context.layout.label(name).is_some(), "{name} is authored");
    }
    for name in ["Bullet00", "Bullet14", "Multi00", "Multi14", "BulletBack"] {
        let sprite = context
            .layout
            .sprite(name)
            .unwrap_or_else(|| panic!("{name} is authored"));
        assert!(
            oag_hud::sprite_draw(sprite, context.sheet).is_some(),
            "{name}'s atlas `{}` is on the sheet:\n{report}",
            sprite.src
        );
    }
    assert!(
        !report.contains("is not in this source"),
        "every include resolves:\n{report}"
    );
    assert!(
        !report.contains("HUD: skipped"),
        "every widget is read, Omega's `<aImageModel>` typo on `EmpBarEffect` included:\n{report}"
    );
    assert!(
        report.contains("HUD: Detonator draws no EmpBarEffect"),
        "the undrawn widgets are named in the loader report:\n{report}"
    );

    let readout = known_cannon(&loaded);
    let shown = readout.detonator.expect("a Detonator race has a readout");
    assert_eq!(
        (
            shown.score,
            shown.clip,
            shown.stage,
            shown.hit,
            shown.tinted
        ),
        (PAID, 10, 3, 0b1_1111, 0b1_1111),
        "the score, clip, stage, volleys that hit and the pips the streak recoloured"
    );
    let bonus = shown.bonus.expect("the five-volley streak raised its line");
    assert_eq!((bonus.streak, bonus.amount, bonus.alpha), (5, 1000, 1.0));

    // Exactly the texts the original draws: the score, the clip, the stage, the energy and the
    // streak line, with every other label of the layout silent (the four it binds nothing to,
    // the EMP and score-feedback widgets, the info lines).
    let frame = oag_hud::draw_list(&context, &readout);
    let drawn = texts(&frame);
    let streak = drawn.iter().find(|text| text.ends_with(" - 1000"));
    let mut expected = vec![
        format!("{PAID:07}"),
        "10".to_string(),
        "3/14".to_string(),
        "100".to_string(),
    ];
    if localised_streak {
        let streak = streak.unwrap_or_else(|| panic!("the streak line is drawn: {drawn:?}"));
        assert!(
            !streak.contains("HUD_CHAIN"),
            "the line is the localised text, not its id: {streak}"
        );
        expected.push(streak.clone());
    } else {
        assert!(
            report.contains("HUD_CHAIN_5, HUD_CHAIN_10, HUD_CHAIN_15 are not in this title's"),
            "a table without the ids says so:\n{report}"
        );
    }
    expected.sort();
    assert_eq!(drawn, expected, "only the bound widgets draw text");

    // Each icon family is wired: dropping the readout's clip or its hit mask changes nothing
    // else about the frame, so compare the sprite counts.
    let sprites = |clip, hit| sprites_with(&context, &readout, clip, hit);
    assert_eq!(
        sprites(15, 0),
        sprites(10, 0) + 5,
        "a spent bullet is hidden"
    );
    assert_eq!(
        sprites(10, 0b111),
        sprites(10, 0) + 3,
        "a pip lights per hit volley"
    );
    assert_eq!(sprites(0, 0), sprites(15, 0) - 15);

    // The EMP's pips light while the charge is over their index, and `EmpReady` shows at the
    // threshold (HD and Omega author them; 2048's skin binds `EmpCharge` and `EmpFull`).
    assert_eq!(
        (shown.emp_charge, shown.emp_ready),
        (5, true),
        "the readout carries the charge"
    );
    for name in ["Emp_0", "Emp_7"] {
        let sprite = context
            .layout
            .sprite(name)
            .unwrap_or_else(|| panic!("{name} is authored"));
        assert!(
            oag_hud::sprite_draw(sprite, context.sheet).is_some(),
            "{name}'s atlas `{}` is on the sheet",
            sprite.src
        );
    }
    let emp = |charge, ready| {
        let mut changed = readout.clone();
        let shown = changed.detonator.as_mut().expect("a readout");
        (shown.emp_charge, shown.emp_ready) = (charge, ready);
        oag_hud::draw_list(&context, &changed).sprites.len()
    };
    assert_eq!(emp(3, false), emp(0, false) + 3, "a pip per charge");
    assert_eq!(emp(12, false), emp(0, false) + 8, "eight pips at most");
    // `EmpReady` where the executable binds it (HD); Omega's layout authors it and its
    // executable names no such string, so it is dropped (`oag_title::Detonator::binds_emp_ready`).
    if emp_ready {
        let sprite = context
            .layout
            .sprite("EmpReady")
            .expect("EmpReady is authored");
        assert!(oag_hud::sprite_draw(sprite, context.sheet).is_some());
        assert_eq!(emp(5, true), emp(5, false) + 1, "the ready lamp");
    } else {
        assert!(
            context.layout.sprite("EmpReady").is_none(),
            "EmpReady is dropped"
        );
        assert!(report.contains("HUD: EmpReady is authored but this title's executable"));
        assert_eq!(emp(5, true), emp(5, false), "no ready lamp");
    }

    // Off Detonator's own state nothing of it draws.
    let plain = oag_hud::Readout {
        detonator: None,
        ..readout
    };
    assert!(
        !texts(&oag_hud::draw_list(&context, &plain)).contains(&format!("{PAID:07}")),
        "no Detonator readout, no score"
    );
}

/// A campaign cell's targets, and the score at which each medal is first reached.
const TARGETS: oag_tables::race_campaign::MedalTargets = oag_tables::race_campaign::MedalTargets {
    gold: 9_000,
    silver: 6_000,
    bronze: 3_000,
};

/// The Detonator HUD draws `DetonatorGoldMedal`, `Silver` or `Bronze` as the score passes the
/// cell's targets: the model of the best medal reached, loaded from the disc, and none without
/// a cell. Fails when the layout reader drops the three `<ImageModel>`s, when the targets stop
/// reaching the race, when a medal's `.vex` does not decode, or when a title that does not
/// show them (2048) loads them.
fn the_medal_follows_the_score(image: &str) {
    use oag_tables::race_campaign::Medal;
    let Some(loaded) = load(image) else {
        return;
    };
    let report = loaded.report.join("\n");
    let context = loaded
        .hud
        .context()
        .unwrap_or_else(|| panic!("the Detonator layout did not load:\n{report}"));
    let medal_models = &loaded.medal_models;
    assert_eq!(
        medal_models.len(),
        3,
        "the three medal models decode:\n{report}"
    );
    for (widget, model) in medal_models {
        assert!(
            !model.vertices.is_empty() && !model.indices.is_empty(),
            "{} is a mesh:\n{}",
            widget.src,
            report
                .lines()
                .filter(|line| line.contains("Trophies"))
                .collect::<Vec<_>>()
                .join("\n")
        );
        assert!(!widget.start_enabled, "{} starts off", widget.name);
        assert_eq!(widget.rect, [1440.0, 80.0, 208.0, 150.0], "{}", widget.name);
    }
    let mut race = racing(&loaded);
    race.set_medal_targets(Some(TARGETS));
    let shown = |race: &mut race::Race, score: i32| {
        race.core.sim.world.detonator.cannon.score = score;
        race.tick(&PlayerInputs::none());
        let readout = race.readout();
        oag_hud::detonator_medal_model(context.layout, &readout).map(|model| model.name.clone())
    };
    assert_eq!(shown(&mut race, 0), None, "no score, no medal");
    assert_eq!(shown(&mut race, 2_999), None, "under bronze");
    for (score, medal) in [
        (3_000, Medal::Bronze),
        (5_999, Medal::Bronze),
        (6_000, Medal::Silver),
        (8_999, Medal::Silver),
        (9_000, Medal::Gold),
        (40_000, Medal::Gold),
    ] {
        let name = oag_hud::DETONATOR_MEDAL_WIDGETS
            .iter()
            .find(|(m, _)| *m == medal)
            .map(|(_, name)| *name);
        assert_eq!(shown(&mut race, score).as_deref(), name, "score {score}");
    }
    race.set_medal_targets(None);
    assert_eq!(
        shown(&mut race, 40_000),
        None,
        "a race with no cell shows none"
    );
}

/// HD's language table carries no `HUD_CHAIN_n`, so the streak line is empty there.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_hd_detonator_hud_reads_the_cannon() {
    the_detonator_hud_reads_the_cannon(HD_IMAGE, false, true);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_hd_detonator_race_shows_the_medal_the_score_has_reached() {
    the_medal_follows_the_score(HD_IMAGE);
}

/// Omega's `Detonator_HUD.xml` authors the same three medals at the same values.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_omega_detonator_race_shows_the_medal_the_score_has_reached() {
    the_medal_follows_the_score(OMEGA_IMAGE);
}

/// Omega's weapons file spells `EmpBarEffect`'s element `<aImageModel>`; the layout still loads.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_omega_detonator_hud_reads_the_cannon() {
    the_detonator_hud_reads_the_cannon(OMEGA_IMAGE, true, false);
}

/// 2048 loads its own layout, a text-only score and stage over `2048_hud\DetonatorWeapons.xml`;
/// the copy of that file the archive order serves is the one with `EmpCharge`/`EmpFull`, the
/// widgets the executable binds. The same widgets read the same cannon.
#[test]
#[ignore = "needs the unpacked 2048 packages in data/extracted/"]
fn a_2048_detonator_hud_reads_the_cannon() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/extracted/vita/PCSF00007");
    if !path.join("patch-v104").exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            path.display()
        );
        return;
    }
    let loaded = race::load(&race::Options {
        source: path.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Detonator,
        track: Some(r"data\art\published\DLC1\environments\zone_1\track.vex".to_string()),
        ..race::Options::default()
    })
    .expect("loading a 2048 Detonator race");
    let report = loaded.report.join("\n");
    assert!(
        report.contains(r"HUD Data\XML\2048_hud\Detonator_HUD_HD.xml:"),
        "{report}"
    );
    let context = loaded.hud.context().expect("the layout loads");
    // Its skin authors the medals at its own place on its own grid, which is left undrawn
    // (`oag_title::Detonator::shows_medals`).
    assert!(
        loaded.medal_models.is_empty() && context.layout.image_models.is_empty(),
        "2048 shows no medal: {:?}",
        context.layout.image_models
    );
    for name in ["Bullet14", "Multi14", "DetonatorWeaponDisplay", "EmpCharge"] {
        let sprite = context
            .layout
            .sprite(name)
            .unwrap_or_else(|| panic!("{name}"));
        assert!(
            oag_hud::sprite_draw(sprite, context.sheet).is_some(),
            "{name}'s atlas is on the sheet"
        );
    }
    let readout = known_cannon(&loaded);
    let shown = readout.detonator.expect("a readout");
    assert_eq!((shown.score, shown.clip, shown.stage), (PAID, 10, 3));
    let drawn = texts(&oag_hud::draw_list(&context, &readout));
    for expected in [format!("{PAID:07}"), "10".to_string(), "3/14".to_string()] {
        assert!(drawn.contains(&expected), "{expected} in {drawn:?}");
    }
    let sprites = |clip, hit| sprites_with(&context, &readout, clip, hit);
    assert_eq!(sprites(15, 0), sprites(10, 0) + 5);
    assert_eq!(sprites(10, 0b111), sprites(10, 0) + 3);
}
