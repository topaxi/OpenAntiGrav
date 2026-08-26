//! The lock-on reticle, against the disc that authors it.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(lock_sight_ground_truth)'
//! ```
//!
//! # What only real data can say here
//!
//! The unit tests in `oag_game::race::sight` assert the recovered *law* and the
//! ones in `oag_game::hud::reticle_tests` assert the five sprites it becomes,
//! both against fixtures written by hand. Neither can say the thing that
//! actually broke every previous attempt at this widget: **that the art is
//! reachable at all.** The sights are the one HUD element whose picture is not
//! in a texture file - it is embedded in a `<Mode3D>` `.vex` model, four widgets
//! share one model, and the layout's `Src` has to resolve through an archive
//! that also holds the meshes. A fixture sheet built from `vec![255; 8*8*4]`
//! proves none of that.

use std::path::{Path, PathBuf};

use oag_game::race;

fn image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/pulse-psp-usa.chd");

    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

/// A single race: the one mode whose layout authors the sights at all.
fn single_race() -> Option<race::Loaded> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: oag_physics::SpeedClass::Venom,
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in &loaded.report {
        println!("{line}");
    }
    Some(loaded)
}

/// `Arcade_HUD.xml` authors nine sight widgets over three models.
///
/// The count is the disc's, taken by expanding the layout with
/// `oag-wad cat --expand` and reading its first `<Mode3D>` block - not by asking
/// this parser and writing down what it said.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_arcade_layout_authors_nine_sight_widgets_over_three_models() {
    use oag_game::race::sight;

    let Some(loaded) = single_race() else { return };
    let layout = loaded
        .hud
        .layout
        .as_ref()
        .expect("the arcade layout parsed");

    let sights: Vec<&oag_game::hud::Model> = layout
        .models
        .iter()
        .filter(|model| sight::is_sight_widget(&model.name))
        .collect();
    assert_eq!(
        sights.len(),
        9,
        "the disc authors {} sight widgets, not nine: {:?}",
        sights.len(),
        sights.iter().map(|m| &m.name).collect::<Vec<_>>()
    );

    let mut models: Vec<&str> = sights.iter().map(|m| m.src.as_str()).collect();
    models.sort_unstable();
    models.dedup();
    assert_eq!(
        models.len(),
        3,
        "nine widgets resolve to {} models, not three: {models:?}",
        models.len()
    );

    // And the four brackets really do share one, which is what makes the
    // rotation recovered in `HudSight_Update` load-bearing rather than
    // decorative: without it the same picture would be drawn four times the
    // same way up.
    let brackets: Vec<&str> = sight::MISSILE_BRACKETS
        .iter()
        .map(|name| {
            layout
                .models
                .iter()
                .find(|m| m.name == *name)
                .unwrap_or_else(|| panic!("{name} is not in the layout"))
                .src
                .as_str()
        })
        .collect();
    assert!(
        brackets.windows(2).all(|pair| pair[0] == pair[1]),
        "the four missile brackets name different models: {brackets:?}"
    );
}

/// And the art behind them decodes out of those models into the HUD sheet.
///
/// **The load-bearing one.** A sight whose model did not resolve draws nothing
/// at all - deliberately, that being this project's rule for an asset it cannot
/// play - so a broken asset path is invisible on screen and in every test that
/// uses a fixture sheet. This is what notices.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_sight_art_decodes_out_of_the_models_into_the_hud_sheet() {
    use oag_game::race::sight;

    let Some(loaded) = single_race() else { return };
    let layout = loaded
        .hud
        .layout
        .as_ref()
        .expect("the arcade layout parsed");

    for name in sight::MISSILE_BRACKETS
        .iter()
        .chain(std::iter::once(&sight::MISSILE_INNER))
        .chain(sight::LEACHBEAM_BRACKETS.iter())
    {
        let model = layout
            .models
            .iter()
            .find(|m| m.name == *name)
            .unwrap_or_else(|| panic!("{name} is not in the layout"));
        let placed = loaded.hud.sheet.get(&model.src).unwrap_or_else(|| {
            panic!(
                "{name}'s model {} is not in the HUD sheet - its art did not \
                 reach the screen, and a sight with no art draws nothing",
                model.src
            )
        });
        assert!(
            placed.width > 0 && placed.height > 0,
            "{name} decoded to {}x{}",
            placed.width,
            placed.height
        );
    }
}

/// Racing on a real circuit with a Missile in hand puts the reticle up and locks.
///
/// The whole chain against real geometry: a real starting grid to pick a target
/// out of, the real chase camera to project through, and the recovered hold to
/// wait out. A synthetic straight has no craft to lock and no camera worth
/// projecting through.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_missile_in_hand_locks_a_craft_on_a_real_circuit() {
    use oag_game::race::sight;

    let Some(loaded) = single_race() else { return };
    let mut race = race::Race::start(loaded.setup);
    // A few ticks to settle the grid, then hand slot 0 a Missile the way a pad
    // would. Slot 0 starts at the back, so the field is ahead of it.
    for _ in 0..30 {
        race.tick(&oag_gameplay::InputSnapshot::default());
    }
    race.world.ships[0].pickup.weapon = Some(oag_formats::weapons::Weapon::Missile);

    let mut seeking = false;
    let mut locked_at = None;
    for tick in 0..240u32 {
        race.tick(&oag_gameplay::InputSnapshot::default());
        match race.sight_state() {
            sight::State::Seeking => seeking = true,
            sight::State::Locked if locked_at.is_none() => locked_at = Some(tick),
            _ => {}
        }
    }

    assert!(
        seeking,
        "nothing on the shipped starting grid put a reticle on screen - either \
         the lock found nobody or the projection refused everybody"
    );
    let locked = locked_at.expect("the reticle never locked on a real grid");
    let seconds = f64::from(locked) / 60.0;
    assert!(
        seconds >= 0.8,
        "it locked after {seconds}s, inside the recovered 0.8s hold"
    );
}
