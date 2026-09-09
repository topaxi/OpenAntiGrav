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
//! The unit tests in `oag_race::sight` assert the recovered *law* and the
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
    oag_testdata::image(name)
}

fn image() -> Option<PathBuf> {
    image_named("pulse-psp-usa.chd")
}

fn race_on(image: &Path) -> race::Loaded {
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
    use oag_race::sight;

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
    use oag_race::sight;

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
    use oag_race::sight;

    let Some(loaded) = single_race() else { return };
    let mut race = race::Race::start(loaded.setup);
    // A few ticks to settle the grid, then hand slot 0 a Missile the way a pad
    // would. Slot 0 starts at the back, so the field is ahead of it.
    for _ in 0..30 {
        race.tick(&oag_gameplay::InputSnapshot::default());
    }
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Missile);

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

/// The LeachBeam's four are four instances of **one** model, and there is no
/// fifth.
///
/// The Missile's four-to-one instancing is asserted above and it does not
/// follow that the LeachBeam's is the same shape - four widgets naming four
/// models, or three plus an inner, would be a different reticle wearing the
/// same slot run. Measured off the disc's own layout rather than assumed from
/// the Missile: `HudSight_Bind` (`0x0881b604`) binds `+0x108` … `+0x114` and
/// **no** inner slot for this weapon.
///
/// It also pins the model apart from the Missile's. Four widgets sharing one
/// model would still pass the first half of this if that model happened to be
/// `missile_sight_outer.vex`, which is exactly what a layout read through the
/// wrong name would give.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_leachbeams_four_widgets_share_one_model_and_have_no_inner() {
    use oag_race::sight;

    let Some(loaded) = single_race() else { return };
    let layout = loaded
        .hud
        .layout
        .as_ref()
        .expect("the arcade layout parsed");

    let model_of = |name: &str| -> String {
        layout
            .models
            .iter()
            .find(|m| m.name == name)
            .unwrap_or_else(|| panic!("{name} is not in the layout"))
            .src
            .clone()
    };

    let leach: Vec<String> = sight::LEACHBEAM_BRACKETS
        .iter()
        .map(|name| model_of(name))
        .collect();
    assert!(
        leach.windows(2).all(|pair| pair[0] == pair[1]),
        "the four leachbeam sights name different models: {leach:?}"
    );
    assert_ne!(
        leach[0],
        model_of(sight::MISSILE_BRACKETS[0]),
        "the LeachBeam's arrowhead and the Missile's bracket are one model, \
         which would mean one of the two names was read wrong"
    );
    assert_ne!(
        leach[0],
        model_of(sight::MISSILE_INNER),
        "the LeachBeam's arrowhead is the Missile's inner box"
    );
    assert!(
        !layout
            .models
            .iter()
            .any(|m| m.name.starts_with("leachbeam_sight_inner") || m.name == "leachbeam_sight_5"),
        "the disc authors a fifth LeachBeam sight widget, which nothing here \
         drives: {:?}",
        layout
            .models
            .iter()
            .map(|m| &m.name)
            .filter(|n| n.starts_with("leachbeam"))
            .collect::<Vec<_>>()
    );

    // And the title table names exactly those four, which is what the draw path
    // reaches for once `Sight::held` says LeachBeam.
    let oag_title::hud::Sights::Brackets { leach: named, .. } = oag_pulse::hud::ART.sights else {
        panic!("Pulse's sights are not the bracket dialect");
    };
    assert_eq!(
        named.expect("Pulse authors a LeachBeam sight set"),
        sight::LEACHBEAM_BRACKETS
    );
}

/// Racing with a **LeachBeam** in hand puts its own reticle up and locks.
///
/// The Missile's version of this is above; this is the second lockable weapon
/// going through the same recovered law off its own authored window
/// (`stats+0x114`/`+0x118` against the Missile's `+0x50`/`+0x54`) and its own
/// four widgets. The window is authored **shorter** at the far end than the
/// Missile's on both shipped tables - see
/// `crates/formats/tests/weapons_ground_truth.rs` - so this is not the Missile's
/// test with a different enum in it: a grid that locks for one need not lock for
/// the other, and asserting it does is the point.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_leachbeam_in_hand_locks_a_craft_on_a_real_circuit() {
    use oag_race::sight;

    let Some(loaded) = single_race() else { return };
    let mut race = race::Race::start(loaded.setup);
    for _ in 0..30 {
        race.tick(&oag_gameplay::InputSnapshot::default());
    }
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::LeachBeam);

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

    assert_eq!(
        race.sight().held(),
        sight::Held::LeachBeam,
        "the reticle is still wearing the Missile's art with a LeachBeam in hand"
    );
    assert!(
        seeking,
        "a LeachBeam on the shipped starting grid put no reticle on screen"
    );
    let locked = locked_at.expect("the LeachBeam's reticle never locked on a real grid");
    let seconds = f64::from(locked) / 60.0;
    assert!(
        seconds >= 0.8,
        "it locked after {seconds}s, inside the recovered 0.8s hold"
    );
}

/// Wipeout Pure locks and draws its reticle, in the PSP dialect.
///
/// **Pure was authored for this all along and could not reach it**, for two
/// reasons that had nothing to do with the reticle. Its layout carries
/// `missile_sight_inner` and `missile_sight_1` … `_4` off the same two models
/// Pulse names - and *not* the LeachBeam's four, the disc agreeing that is a
/// Pulse weapon - and its weapon table is one lower-cased
/// `Data\XML\weaponstats.xml` rather than Pulse's two.
///
/// It also covers a path Pulse cannot reach: every Pure HUD layout names no
/// `.mip`, so Pure always takes the atlas loader's "no texture" branch, which
/// used to return an empty sheet before it looked for sight art.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pure_locks_a_craft_and_draws_the_psp_reticle() {
    use oag_race::sight;

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

    let mut race = race::Race::start(loaded.setup);
    race.set_sight_screen(loaded.hud.space.size);
    assert!(
        race.missile_stats().is_some(),
        "Pure's weapon table did not parse - its Missile authors one `speed` \
         where Pulse authors four, and no `launchSpeed` at all"
    );
    assert!(
        locks_within(&mut race, 240),
        "Pure never locked a craft on its own starting grid"
    );
    assert_eq!(
        race.leach_beam_stats(),
        None,
        "Pure authors a LeachBeam block after all, which would mean its \
         `leach: None` sight set is now reachable and draws nothing"
    );
}

/// Which titles author a `<Weapon type="LeachBeam">` at all, printed and pinned.
///
/// **The reason this is a test and not a note, and it caught something.**
/// `Race::sight_held` returns `Some(Held::LeachBeam)` for any title whose table
/// authors the block, and the widget set that answers it is per title. Pulse
/// names four. Pure names none *and authors no block*, so its `leach: None` is
/// belt and braces. **HD authors the block** - measured here, and it was assumed
/// otherwise - while its `Sights::Concentric` dialect carries **one** set of
/// widget names and no second one to reach for. Without a guard a held LeachBeam
/// on HD would draw `MissileSight*`: another weapon's reticle, which is exactly
/// the plausible-looking stand-in `CLAUDE.md` forbids. `hud::sight_draw` draws
/// nothing there instead, and this is what stops that pairing drifting.
///
/// Shape only, never values - ADR-0006. `--no-capture` prints which is which.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn only_pulse_and_hd_author_a_leachbeam_block() {
    let mut checked = 0;
    for (name, image, expected) in [
        ("Pulse", "pulse-psp-usa.chd", true),
        ("Pure", "pure-psp-usa.chd", false),
        ("HD/Fury", "hdfury-ps3-eu-dec.iso", true),
    ] {
        let Some(path) = image_named(image) else {
            continue;
        };
        let race = race::Race::start(race_on(&path).setup);
        let authored = race.leach_beam_stats().is_some();
        println!("{name}: authors a LeachBeam block = {authored}");
        assert_eq!(
            authored, expected,
            "{name} disagrees with what `oag_title::hud::Sights` was built \
             against - a title that authors the block but no widget set for it \
             draws another weapon's reticle for a held LeachBeam unless \
             `hud::sight_draw` refuses"
        );
        checked += 1;
    }
    assert!(checked > 0, "no image present to check");
}

/// And HD's four `LeachBeamSight*` sprites really are there, waiting.
///
/// The other half of the finding above: HD authors the weapon *and* art for it,
/// and what is missing is only a reading of which of the four is up when. Pinned
/// so "unwired" stays a statement about this engine rather than about the disc -
/// a future pass looking for them should find them named here, not go hunting.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_authors_four_leachbeam_sight_sprites_that_nothing_draws() {
    let Some(image) = image_named("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let loaded = race_on(&image);
    let layout = loaded
        .hud
        .layout
        .as_ref()
        .expect("HD's arcade layout parsed");

    let found: Vec<&str> = layout
        .sprites
        .iter()
        .map(|s| s.name.as_str())
        .filter(|name| name.starts_with("LeachBeamSight"))
        .collect();
    println!("HD LeachBeam sight sprites: {found:?}");
    assert_eq!(
        found.len(),
        4,
        "HD authors {} LeachBeamSight* sprite(s), not four - which changes what \
         wiring them would mean: {found:?}",
        found.len()
    );
}

/// Wipeout HD locks too, and draws its reticle the other way.
///
/// **One law, two dialects.** HD's arcade HUD composes to zero `<Mode3D>`
/// models: its reticle is concentric `<Image>` sprites named `MissileSight*`
/// off `Data\HUD\Textures\missile_reticule.gtf`, at authored sizes of 128,
/// 108, 80 and 64 with a red outer and a green inner. What it shares with the
/// PSP titles is the placement law and the placeholder idiom - every one of its
/// sight widgets is authored centred on `(-960, 540)`, the negated centre of its
/// own 1920x1080 screen, exactly as the PSP titles use `(-240, 136)` of theirs.
///
/// **That screen is why this test exists.** A reticle projecting into the PSP's
/// 480x272 while the layout draws in 1920x1080 lands in the top-left ninth of
/// the picture and never leaves it - which is what HD did until
/// `Race::set_sight_screen`, and which reads on screen as a stray widget rather
/// than as a missing one.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_locks_a_craft_and_draws_its_own_concentric_reticle() {
    use oag_race::sight;

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

    let oag_title::hud::Sights::Concentric { seeking, locked } = oag_hd::TITLE.hud_art.sights
    else {
        panic!("HD's sight dialect is not the concentric one");
    };
    for name in seeking.iter().chain(locked.iter()) {
        let sprite = layout
            .sprites
            .iter()
            .find(|s| s.name == *name)
            .unwrap_or_else(|| panic!("HD's arcade layout has no {name}"));
        assert!(
            loaded.hud.sheet.get(&sprite.src).is_some(),
            "{name}'s texture {} did not reach HD's HUD sheet",
            sprite.src
        );
        // The authored rectangle is a placeholder whose *centre* is the negated
        // middle of HD's own screen. Its size is real and is what gets drawn.
        let centre = [
            sprite.rect[0] + sprite.rect[2] * 0.5,
            sprite.rect[1] + sprite.rect[3] * 0.5,
        ];
        assert!(
            (centre[0] + 960.0).abs() < 0.01 && (centre[1] - 540.0).abs() < 0.01,
            "{name} is authored centred on {centre:?}, not the (-960, 540) \
             placeholder every other sight widget on every title uses"
        );
    }

    let mut race = race::Race::start(loaded.setup);
    race.set_sight_screen(loaded.hud.space.size);
    assert_eq!(
        race.sight().screen(),
        [1920.0, 1080.0],
        "HD's reticle is projecting into somebody else's screen"
    );
    let stats = race
        .missile_stats()
        .expect("HD authors a Missile with lock distances");
    assert!(stats.lock_max_dist > stats.lock_min_dist && stats.lock_min_dist >= 0.0);
    assert!(
        locks_within(&mut race, 240),
        "HD never locked a craft on its own starting grid"
    );
}

/// Hands slot 0 a Missile and races until the reticle locks something.
fn locks_within(race: &mut race::Race, ticks: u32) -> bool {
    use oag_race::sight;

    for _ in 0..30 {
        race.tick(&oag_gameplay::InputSnapshot::default());
    }
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Missile);
    for _ in 0..ticks {
        race.tick(&oag_gameplay::InputSnapshot::default());
        if race.sight_state() == sight::State::Locked {
            return true;
        }
    }
    false
}
