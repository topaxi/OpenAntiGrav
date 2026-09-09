//! Pure's weapon-icon `<Model>`s, against the disc that authors them.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(pickup_icon_ground_truth)'
//! ```
//!
//! # What only real data can say here
//!
//! `crates/pure/src/hud.rs`'s `PICKUP_ICON_MODELS` and
//! `oag_game::hud::pickup_model_tests` both assert against a hand-written
//! fixture - three widgets and an invented `.vex` for the backdrop grid. What
//! they cannot say is the thing `docs/gameplay/pickups.md`'s retracted claim
//! got wrong in the first place: **that the ten real icon models actually
//! resolve, decode, and land in the sheet with a usable quad extent.** This
//! is what checks the real disc.

use std::path::PathBuf;

use oag_game::race;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pure-psp-usa.chd")
}

fn single_race() -> Option<race::Loaded> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in &loaded.report {
        println!("{line}");
    }
    Some(loaded)
}

/// The ten weapon-icon widgets `oag_pure::hud::PICKUP_ICON_MODELS` names, plus
/// the backdrop grid, all resolve into the HUD sheet with a real quad extent -
/// not the invented one the unit fixture uses.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_named_icon_model_decodes_with_a_real_quad_extent() {
    let Some(loaded) = single_race() else { return };
    let layout = loaded
        .hud
        .layout
        .as_ref()
        .expect("Pure's arcade layout parsed");

    let mut checked = 0;
    for name in oag_pure::hud::PICKUP_ICON_MODELS.into_iter().flatten() {
        let model = layout
            .models
            .iter()
            .find(|m| m.name == name)
            .unwrap_or_else(|| panic!("Pure's arcade layout has no {name}"));
        let placed = loaded.hud.sheet.get(&model.src).unwrap_or_else(|| {
            panic!(
                "{name}'s model {} did not reach the HUD sheet - a weapon \
                 held on a real race would draw nothing for it",
                model.src
            )
        });
        let extent = placed
            .quad_extent
            .unwrap_or_else(|| panic!("{name}'s model {} decoded with no quad extent", model.src));
        assert!(
            extent[0] > 0.0 && extent[1] > 0.0,
            "{name} decoded to a degenerate quad {extent:?}"
        );
        checked += 1;
    }
    assert_eq!(
        checked, 9,
        "PICKUP_ICON_MODELS should name nine icons on the real table \
         (Cannon/LeachBeam/Repulser/Shuriken excluded)"
    );

    let grid = oag_pure::hud::ART
        .pickup_icon_backdrop_model
        .expect("Pure names a backdrop grid model");
    let model = layout
        .models
        .iter()
        .find(|m| m.name == grid)
        .unwrap_or_else(|| panic!("Pure's arcade layout has no {grid}"));
    assert!(
        loaded.hud.sheet.get(&model.src).is_some(),
        "{grid}'s model {} did not reach the HUD sheet",
        model.src
    );
}

/// A held Turbo draws the grid and its own coloured icon on a real race, in
/// `TURBO_icon`'s disc-authored `0xff40ff40`.
///
/// The end-to-end path: real archive, real layout, real sheet, real
/// `oag_game::hud::draw::pickup_model_draws` - nothing here is a fixture.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_held_turbo_draws_its_own_authored_green_on_a_real_race() {
    let Some(loaded) = single_race() else { return };
    let layout = loaded
        .hud
        .layout
        .as_ref()
        .expect("Pure's arcade layout parsed");
    let turbo_model = layout
        .models
        .iter()
        .find(|m| m.name == "TURBO_icon")
        .expect("Pure's arcade layout has TURBO_icon");
    let expected_color = turbo_model
        .colour
        .expect("TURBO_icon authors its own colour");

    let context = loaded
        .hud
        .context()
        .expect("Pure's arcade layout parses; a context is buildable");

    let mut race = race::Race::start(loaded.setup);
    race.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Turbo);
    race.tick(&oag_gameplay::InputSnapshot::default());

    let readout = race.readout();
    assert_eq!(
        readout.pickup,
        Some(oag_tables::weapons::Weapon::Turbo),
        "the readout must carry the held weapon for the draw list to find it"
    );
    let frame = oag_game::hud::draw_list(&context, &readout);

    // A `<Mode3D><Model>` widget, so a `BlendedSprite`: the quad carries the
    // blend its own model declares rather than taking the pipeline's. See
    // `oag_game::sprite::Placed::blend`.
    let icon_draw = frame
        .sprites
        .iter()
        .find(|draw| matches!(draw, oag_game::frontend::Draw::BlendedSprite { color, .. } if *color == expected_color))
        .unwrap_or_else(|| {
            panic!(
                "no sprite drew in TURBO_icon's own colour {expected_color:?}; \
                 the frame carried: {:?}",
                frame.sprites
            )
        });
    // **Measured on Pure's own disc, and it differs from Pulse's sights.**
    // `TURBO_icon`'s model declares `AlphaOver` (`pass_mask & 0x100`), where
    // all three of Pulse's lock-on sight models declare `Additive` (`& 0x200`)
    // - which is exactly why the class is read off each model rather than
    // tabulated per widget. Pinned so a change to that reading has to say so
    // here.
    let oag_game::frontend::Draw::BlendedSprite { blend, .. } = icon_draw else {
        unreachable!("found by that pattern");
    };
    assert_eq!(
        *blend,
        Some(oag_vex::vex::BlendClass::AlphaOver),
        "TURBO_icon's own model declares this blend"
    );
    println!("{icon_draw:?}");
}
