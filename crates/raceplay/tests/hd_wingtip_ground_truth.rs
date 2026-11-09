//! Wipeout HD's wingtip contrails on a real disc: every hull authors the `wingtip` pair, the
//! `smoke_trails_frame1_alpha` ribbon loads, and a craft under a speed pad lays two ribbons that
//! fade out after the pad.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^hd_wingtip_ground_truth::/)'
//! ```
//!
//! The law is `docs/ghidra/functions/ps3-hdfury-eu/wingtip-vortices.md`.

use oag_core::math::Vec3;
use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

fn loaded() -> Option<race::Loaded> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in loaded.report.iter().filter(|l| l.contains("wingtip")) {
        println!("{line}");
    }
    Some(loaded)
}

/// Every claim that needs the race loaded and not flown, over one load.
#[test]
#[ignore = "needs a disc image"]
fn the_loaded_race_holds_every_wingtip_claim() {
    let Some(loaded) = loaded() else {
        return;
    };
    every_grid_craft_authors_the_mirrored_pair(&loaded);
    the_ribbon_texture_loads_with_the_hd_config(&loaded);
}

/// Every craft on the grid carries `wingtip_left` and `wingtip_right`, side by side at
/// `|x|` 2.1 to 3.0 (the evidence page's census of all 39 `Locators.vex`).
fn every_grid_craft_authors_the_mirrored_pair(loaded: &race::Loaded) {
    for livery in &loaded.liveries {
        let [left, right] = livery.wingtip;
        let left = left.unwrap_or_else(|| panic!("{}: no wingtip_left", livery.team));
        let right = right.unwrap_or_else(|| panic!("{}: no wingtip_right", livery.team));
        let (left, right) = (left.w_axis.truncate(), right.w_axis.truncate());
        assert!(
            (2.0..=3.1).contains(&left.x.abs()),
            "{}: left at {left}",
            livery.team
        );
        assert!(
            (left.x + right.x).abs() < 0.2 && (left.y - right.y).abs() < 0.2,
            "{}: the right is the left's mirror, to the small offsets some hulls author: {left} \
             against {right}",
            livery.team
        );
        assert!(left.x > 0.0, "{}: HD's left is +x", livery.team);
    }
}

fn the_ribbon_texture_loads_with_the_hd_config(loaded: &race::Loaded) {
    let assets = loaded
        .ribbon_textures
        .wingtip
        .as_ref()
        .expect("smoke_trails_frame1_alpha.gtf and a locator pair must both load");
    assert!(assets.texture.width > 0 && assets.texture.height > 0);
    assert_eq!(assets.config.line.chains, 3);
    assert_eq!(assets.config.drift, 2.5);
    assert!(
        loaded
            .report
            .iter()
            .any(|l| l.contains("wingtip contrails: /data/tex/smoke_trails_frame1_alpha.gtf")),
        "{:?}",
        loaded.report
    );
}

/// One craft under a pad on a standing grid takes exactly its two elements, and takes them only
/// while the pad force time is up: the wire from `pad_timer` to the pool, with nothing else
/// (no natural pad, no speed) to confuse it.
#[test]
#[ignore = "needs a disc image"]
fn the_pad_force_time_takes_two_elements_and_releasing_it_lets_them_go() {
    let Some(loaded) = loaded() else {
        return;
    };
    let mut race = race::Race::start(loaded.setup);
    for _ in 0..10 {
        race.tick(&PlayerInputs::none());
    }
    assert_eq!(race.wingtip_live(), 0, "nothing is laid with no pad");
    for _ in 0..10 {
        race.sim.world.ships[0].physics.pad_timer = 0.5;
        race.tick(&PlayerInputs::none());
    }
    assert_eq!(race.wingtip_live(), 2, "one element per wing tip");
    for _ in 0..140 {
        race.sim.world.ships[0].physics.pad_timer = 0.0;
        race.tick(&PlayerInputs::none());
    }
    assert_eq!(race.wingtip_live(), 0, "the timer and the nodes ran out");
}

/// A craft held under a speed pad lays two ribbons, never brighter than the speed ramp's `0.3`,
/// and every element is gone within the timer's 1.3 s and a node's 0.7 s after the pad.
#[test]
#[ignore = "needs a disc image"]
fn a_craft_under_a_speed_pad_lays_two_ribbons_that_fade_out() {
    let Some(loaded) = loaded() else {
        return;
    };
    let mut race = race::Race::start(loaded.setup);
    race.set_autopilot(true);
    // The autopilot takes the grid up to speed; the pad is held on slot 0 by hand.
    for _ in 0..900 {
        race.tick(&PlayerInputs::none());
    }
    let speed = {
        let body = race.sim.world.ships[0].physics.body;
        body.linear_velocity.dot(body.forward()).abs()
    };
    println!("slot 0 at {speed:.1} units/s");
    assert!(speed > 100.0, "the ramp needs the craft above 100: {speed}");
    for _ in 0..30 {
        race.sim.world.ships[0].physics.pad_timer = 0.5;
        race.tick(&PlayerInputs::none());
    }
    let live = race.wingtip_live();
    assert!(live >= 2, "two ribbons for the held craft, {live} live");
    let eye = race.camera_position();
    let vertices = race.wingtip_vertices(eye);
    assert!(!vertices.is_empty(), "the held craft draws a strip");
    let alpha = vertices.iter().map(|v| v.colour[3]).fold(0.0_f32, f32::max);
    println!(
        "{live} elements, {} vertices, peak alpha {alpha:.3}",
        vertices.len()
    );
    assert!(alpha > 0.05 && alpha <= 0.3 + 1e-4, "alpha {alpha}");
    assert!(
        vertices.iter().all(|v| v.colour[..3] == [1.0, 1.0, 1.0]),
        "white times the texture"
    );
    // Strips follow the craft: every vertex is within the ribbon's reach of slot 0.
    let craft = race.sim.world.ships[0].physics.body.position;
    assert!(
        vertices
            .iter()
            .any(|v| (Vec3::from(v.position) - craft).length() < 20.0),
        "the strip sits at the craft"
    );
    // Release the pad: the 1.3 s timer, then a node's 0.7 s.
    for _ in 0..140 {
        race.sim.world.ships[0].physics.pad_timer = 0.0;
        race.tick(&PlayerInputs::none());
    }
    assert_eq!(
        race.wingtip_vertices(eye)
            .iter()
            .map(|v| v.colour[3])
            .fold(0.0_f32, f32::max)
            < 0.01,
        true,
        "nothing visible two seconds after the pad"
    );
}

/// 2048 and Omega carry the same two locators per hull and the same ribbon texture in their own
/// container (`.gxt`, `.gnf`), so a pad takes the same two elements: HD's config by lineage.
#[test]
#[ignore = "needs 2048's and Omega's packages"]
fn the_lineage_titles_lay_the_contrail_under_a_pad_too() {
    let mut checked = Vec::new();
    for image in [
        "data/test-vpk/2048-PCSF00007.vpk",
        "data/images/omega-ps4-eu.pkg",
    ] {
        let Some(path) = oag_testdata::exact(image) else {
            continue;
        };
        let loaded = race::load(&race::Options {
            source: path.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::SingleRace,
            ..race::Options::default()
        })
        .unwrap_or_else(|why| panic!("{image}: {why}"));
        let assets = loaded
            .ribbon_textures
            .wingtip
            .as_ref()
            .unwrap_or_else(|| panic!("{image}: no contrail assets: {:?}", loaded.report));
        assert!(
            matches!(
                assets.config.origin,
                oag_title::Origin::InheritedFrom("Wipeout HD")
            ),
            "{image}: labelled inherited from HD"
        );
        let mut race = race::Race::start(loaded.setup);
        for _ in 0..10 {
            race.tick(&PlayerInputs::none());
        }
        assert_eq!(race.wingtip_live(), 0, "{image}: nothing laid with no pad");
        for _ in 0..10 {
            race.sim.world.ships[0].physics.pad_timer = 0.5;
            race.tick(&PlayerInputs::none());
        }
        assert_eq!(race.wingtip_live(), 2, "{image}: one element per wing tip");
        checked.push(image);
    }
    println!("lineage titles checked: {checked:?}");
}
