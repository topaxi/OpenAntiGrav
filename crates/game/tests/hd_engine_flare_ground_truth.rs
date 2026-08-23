//! Wipeout HD's engine flare, off a real disc image: a per-team model, split
//! into an always-on flame and a boost plume by its own node tree.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all hd_engine_flare
//! ```
//!
//! # What this pins that a screenshot cannot
//!
//! The flame draws into the same few hundred pixels the exhaust ribbon already
//! saturates, so "it appeared" is not readable off a capture - which is exactly
//! the failure mode the load report exists to remove. What is assertable is
//! the chain the picture rests on: that every craft on the grid ships the pair,
//! that it builds, that its one material is the shared additive `flame_test`,
//! and that the node tree really does divide it into the two groups the title
//! package names.
//!
//! It also pins the finding that reopened the plume question. A sweep for a
//! `shipboost.vex` on this disc comes back empty and used to read as "HD has no
//! boost plume"; the plume is the `EF_Boost` subtree asserted here.

use std::path::{Path, PathBuf};

use oag_game::race;
use oag_physics::SpeedClass;

fn image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/hdfury-ps3-eu-dec.iso");
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

fn load() -> Option<race::Loaded> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: SpeedClass::Venom,
        mode: oag_race::Mode::SingleRace,
        opponent_teams: Vec::new(),
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in &loaded.report {
        println!("{line}");
    }
    Some(loaded)
}

/// Every craft on the grid draws its own flame and its own plume.
///
/// **Both halves, per craft.** One team's flare on another's hull would be the
/// kind of wrong that renders plausibly, and the models genuinely differ - 350
/// to 961 triangles for `EF_Main` across the eight, and 616 to 1,492 for
/// `EF_Boost`.
#[test]
#[ignore = "needs a disc image"]
fn every_craft_on_the_grid_carries_both_halves_of_its_flare() {
    let Some(loaded) = load() else {
        return;
    };
    for livery in &loaded.liveries {
        let flame = livery
            .flare
            .as_ref()
            .unwrap_or_else(|| panic!("{}: no engine flare model", livery.team));
        let plume = livery
            .boost
            .as_ref()
            .unwrap_or_else(|| panic!("{}: no boost plume - EF_Boost is missing", livery.team));
        assert!(
            !flame.indices.is_empty() && !plume.indices.is_empty(),
            "{}: {} flame indices, {} plume indices",
            livery.team,
            flame.indices.len(),
            plume.indices.len()
        );
        // **Both halves index the one shared buffer pair**, which is what says
        // the split is a filter over draw calls rather than two builds.
        assert_eq!(
            flame.vertices.len(),
            plume.vertices.len(),
            "{}: the two groups are two views of one model",
            livery.team
        );
        // Additive, off the material rather than off a constant here: every
        // batch carries the factor pair `flame_test.rcsmaterial` authors.
        let blend = flame
            .transparent_draws
            .first()
            .and_then(|draw| draw.blend_state)
            .unwrap_or_else(|| panic!("{}: the flame authors no blend", livery.team));
        assert_eq!(
            blend.color.src_factor,
            wgpu::BlendFactor::SrcAlpha,
            "{}: src factor",
            livery.team
        );
        assert_eq!(
            blend.color.dst_factor,
            wgpu::BlendFactor::One,
            "{}: dst factor",
            livery.team
        );
    }
}

/// The flame is placed at the craft's own `Engine Flare` locator, not at the
/// hull's origin.
///
/// The model is authored about its own origin - every transform in the file is
/// the identity - so a flare that was *not* moved would sit in the middle of
/// the fuselage, which is a picture that reads as "no flare" rather than as a
/// misplaced one. Asserted against the locator the same report names.
#[test]
#[ignore = "needs a disc image"]
fn the_flame_sits_at_the_nozzle_rather_than_at_the_hulls_origin() {
    let Some(loaded) = load() else {
        return;
    };
    for livery in &loaded.liveries {
        let (Some(flame), Some(nozzle)) = (livery.flare.as_ref(), livery.nozzle) else {
            panic!("{}: no flare or no nozzle", livery.team);
        };
        // Well behind the hull's origin on every HD craft: the locators run
        // from z = -4.9 to -8.0. The flame's own centre is within its radius of
        // the nozzle, which is the statement "it was moved there".
        assert!(
            nozzle.z < -1.0,
            "{}: the nozzle locator is at {nozzle:?}",
            livery.team
        );
        let centre = oag_core::math::Vec3::from_array(flame.centre);
        assert!(
            (centre - nozzle).length() < flame.radius,
            "{}: flame centred at {centre:?}, nozzle at {nozzle:?}, radius {:.2}",
            livery.team,
            flame.radius
        );
    }
}

/// The colour set's fourth byte reaches the vertex **alpha**, which is where
/// `flame_test`'s own fragment program reads it.
///
/// Its last two instructions are `MOV H0.w, f[TC2]` and `MUL H0.w, H1, H0`,
/// and the paired vertex program writes `o[TC2] = (world position, v[3].w)`
/// with `v[3]` named `VertexColour1` by its hash. So the byte is the flame's
/// opacity ramp and not the sun-occlusion mask `mesh/rcs.rs` files it as for
/// the circuit materials that reading was measured on. See
/// `crate::livery::flare::alpha_ramp` and
/// `docs/ghidra/functions/ps3-hdfury-eu/engine-flare.md`.
#[test]
#[ignore = "needs a disc image"]
fn the_flame_carries_an_opacity_ramp_in_its_vertex_alpha() {
    let Some(loaded) = load() else {
        return;
    };
    for livery in &loaded.liveries {
        // **Both halves.** The two groups share one buffer pair, so a ramp
        // asserted on the flame alone would be asserted on the plume's
        // vertices too without saying so - but the ramp is per node, and the
        // plume's own nodes carry their own: `ef_BoostLeftShape` spans 0.000 to
        // 1.000 and `ef_DiamondsShape` 0.000 to 0.596 on Feisar.
        for (what, model) in [("flame", &livery.flare), ("plume", &livery.boost)] {
            let Some(model) = model.as_ref() else {
                panic!("{}: no {what}", livery.team);
            };
            let min = model
                .vertices
                .iter()
                .map(|v| v.colour[3])
                .fold(f32::INFINITY, f32::min);
            let max = model
                .vertices
                .iter()
                .map(|v| v.colour[3])
                .fold(f32::NEG_INFINITY, f32::max);
            assert!(
                min <= 0.01 && max >= 0.99,
                "{}: the {what}'s ramp spans {min} to {max}, so it is not a ramp",
                livery.team
            );
            // And the slot it came from is left unmasked, or the generic sun
            // term would be gated by an opacity ramp.
            assert!(
                model.vertices.iter().all(|v| v.sun_mask == 1.0),
                "{}: a {what} vertex still carries a sun mask",
                livery.team
            );
        }
    }
}

/// Both halves of every craft's flare carry the flame's own shading
/// parameters, read off that craft's own material record.
///
/// **Per craft, not shared**, even though all fourteen author the same six
/// numbers: a value that is read is a fact about the data and a value that is
/// assumed is a constant in disguise. The numbers themselves are pinned on the
/// format side by
/// `crates/formats/tests/rcsmodel_material_ground_truth.rs`; what this asserts
/// is that they reach the model the renderer draws.
#[test]
#[ignore = "needs a disc image"]
fn every_flare_carries_the_flames_own_shader_parameters() {
    let Some(loaded) = load() else {
        return;
    };
    for livery in &loaded.liveries {
        for (what, model) in [("flame", &livery.flare), ("plume", &livery.boost)] {
            let Some(model) = model.as_ref() else {
                panic!("{}: no {what}", livery.team);
            };
            let flame = model
                .flame
                .unwrap_or_else(|| panic!("{}: the {what} has no shading parameters", livery.team));
            assert_eq!(flame.rim_power, 10.0, "{}", livery.team);
            assert_eq!(flame.rim_scale, 0.3, "{}", livery.team);
            assert_eq!(flame.rim_min, 0.45, "{}", livery.team);
            assert_eq!(flame.alpha_scale, 2.0, "{}", livery.team);
            assert_eq!(flame.colour_scale, 1.0, "{}", livery.team);
        }
    }
}
