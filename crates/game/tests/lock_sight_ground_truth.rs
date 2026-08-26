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

fn image_named(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images")
        .join(name);

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

fn image() -> Option<PathBuf> {
    image_named("pulse-psp-usa.chd")
}

fn race_on(image: &Path) -> race::Loaded {
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
    loaded
}

/// A single race: the one mode whose layout authors the sights at all.
fn single_race() -> Option<race::Loaded> {
    Some(race_on(&image()?))
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

/// Wipeout Pure authors the Missile's five sights, and its art reaches the sheet.
///
/// **The reticle is one engine, three titles, and this is the half of that which
/// is data.** Pure's `Arcade_HUD.xml` carries `missile_sight_inner` and
/// `missile_sight_1` … `_4` off the same two models Pulse names - and *not* the
/// LeachBeam's four, which is the disc agreeing that the LeachBeam is a Pulse
/// weapon.
///
/// **It also covers a path Pulse cannot reach.** Every one of Pure's four HUD
/// layouts names no `.mip` at all - the whole HUD is `<Model>` geometry - so
/// Pure always takes the "no texture" branch of the atlas loader, which used to
/// return an empty sheet before it ever looked for sight art. Pulse never takes
/// that branch, so nothing else in the tree would notice.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pure_authors_the_missiles_sights_and_its_art_reaches_the_sheet() {
    use oag_game::race::sight;

    let Some(image) = image_named("pure-psp-usa.chd") else {
        return;
    };
    let loaded = race_on(&image);
    let layout = loaded
        .hud
        .layout
        .as_ref()
        .expect("Pure's arcade layout parsed");

    for name in sight::MISSILE_BRACKETS
        .iter()
        .chain(std::iter::once(&sight::MISSILE_INNER))
    {
        let model = layout
            .models
            .iter()
            .find(|m| m.name == *name)
            .unwrap_or_else(|| panic!("Pure's arcade layout has no {name}"));
        assert!(
            loaded.hud.sheet.get(&model.src).is_some(),
            "{name}'s model {} did not reach Pure's HUD sheet - a layout that \
             names no atlas must still load its sight art",
            model.src
        );
    }

    assert!(
        !layout
            .models
            .iter()
            .any(|m| sight::LEACHBEAM_BRACKETS.contains(&m.name.as_str())),
        "Pure authors LeachBeam sights, which would mean it has the weapon"
    );
}

/// But Pure ships no weapon table, so nothing there has a lock to draw.
///
/// **The blocker is upstream of everything this change touched.** `Race::load`
/// looks for `Data\XML\WeaponStats_Race.xml` and Pure's archives hold no entry
/// by that name, so there are no Missile stats, no pickup odds and no lock
/// window - the sights are authored and nothing can drive them. Whether Pure
/// names that file something else, or authors its weapons somewhere else
/// entirely, is unread.
///
/// Asserted rather than left as a note so the day Pure's table is found, this
/// fails and says where to look.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pure_ships_no_weapon_table_so_its_sights_have_nothing_to_drive_them() {
    let Some(image) = image_named("pure-psp-usa.chd") else {
        return;
    };
    let loaded = race_on(&image);
    let race = race::Race::start(loaded.setup);
    assert!(
        race.missile_stats().is_none(),
        "Pure now parses Missile stats - wire its sights up and delete this test"
    );
}

/// Wipeout HD locks, and draws its reticle a different way entirely.
///
/// **The lock is shared and the reticle is not.** HD's `WeaponStats_Race.xml`
/// parses, Missile block included, so `Ship_AcquireLock`'s window, the 0.8 s
/// hold and the unguided shot all run on HD exactly as they do on Pulse - that
/// half is engine code with no title in it.
///
/// Its **sights** are another dialect: HD's arcade HUD composes to **zero**
/// `<Mode3D>` models, and its reticle is `<Image>` sprites named `MissileSight*`
/// and `LeachBeamSight*` off `Data\HUD\Textures\missile_reticule.gtf`. So
/// `crate::race::sight`'s placement law applies and its *widget names and
/// geometry* do not. Wiring it is a second naming table and a sprite path, not
/// new recovery.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_parses_its_missile_but_authors_its_reticle_as_sprites() {
    use oag_game::race::sight;

    let Some(image) = image_named("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let loaded = race_on(&image);
    let layout = loaded
        .hud
        .layout
        .as_ref()
        .expect("HD's arcade layout parsed");

    assert!(
        !layout
            .models
            .iter()
            .any(|m| sight::is_sight_widget(&m.name)),
        "HD authors PSP-named sight models; the reticle would draw twice"
    );

    let race = race::Race::start(loaded.setup);
    let stats = race
        .missile_stats()
        .expect("HD authors a Missile with lock distances");
    assert!(
        stats.lock_max_dist > stats.lock_min_dist && stats.lock_min_dist >= 0.0,
        "HD's lock window is {}..{}, which cannot select anything",
        stats.lock_min_dist,
        stats.lock_max_dist
    );
}
