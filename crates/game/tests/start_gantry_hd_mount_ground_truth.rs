//! Wipeout HD's start gantry: the mount `oag_render::gantry::mount` measures
//! on Talon's Junction, and that `race::load` actually stands a gantry on it.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(start_gantry_hd_mount_ground_truth)'
//! ```
//!
//! # What this is for, and how it differs from `crates/hd`'s own gantry test
//!
//! `crates/hd/tests/start_gantry_hd_ground_truth.rs` reads `321go_startfinish`'s
//! own bytes directly - the asset's own claims, independent of this project's
//! model builder. This file is the other half: that `oag_render::gantry::mount`
//! actually finds HD's `billboard8.gtf`-bound chunk on a real track model built
//! through this project's own `oag_mesh::mesh::rcs` pipeline, and that
//! `race::load` places a gantry there rather than reporting an absence -
//! `crates/game/tests/start_gantry_report_ground_truth.rs`'s own shape, for
//! Pulse, ported to HD's differently-packaged disc.
//!
//! **HD's mount is measured here as flatly false to call "the flat stub Pulse's
//! is".** `docs/rendering/start-gantry.md`'s Pulse section measures
//! `Mount::thickness` at 0.000-0.100 on all twelve circuits that author one;
//! Talon's Junction's own `billboard8.gtf`-bound chunk measures 3.0 - a real
//! 3D structure (a boost-gate frame), not a thin placeholder quad. The width
//! and height still fit a plane well enough to mount a board on (46.3 x 9.8,
//! within the same order Pulse's own 45.13-45.50 x 8.61-12.96 panels take),
//! so [`oag_render::gantry::mount`]'s principal-axis fit is not asserted wrong -
//! only asserted honestly, at the number this circuit actually gives it.
//!
//! Confirmed on Talon's Junction alone - `just check-test-budget`'s ratchet is
//! the reason the sweep stays to one circuit rather than the four this pass
//! measured by hand (see the lane's own report for the other three's numbers).

use std::path::PathBuf;

use oag_core::math::Vec3;
use oag_raceplay as race;

/// The decrypted PS3 image.
const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

const TRACK: &str = r"Data\Environments\Talons_Junction\track.vex";

fn image() -> Option<PathBuf> {
    oag_testdata::image(PS3_IMAGE)
}

/// The circuit's own track model, built exactly as `race::load` builds it -
/// through `oag_mesh::mesh::rcs::build_scene` off the `.vex` and its sibling
/// `.rcsmodel` - so the mount is measured on the same geometry a race draws,
/// not a hand-rolled stand-in.
fn track_model(image: &std::path::Path) -> oag_mesh::mesh::Model {
    let mut archives = oag_hd::open(&image.display().to_string()).expect("opening the disc");
    let track_blob = archives.read_name(TRACK).expect("the track .vex reads");
    let sibling =
        oag_mesh::mesh::rcs::sibling_name(TRACK).expect("a .vex name has an .rcsmodel sibling");
    let geometry = archives
        .read_name(&sibling)
        .expect("the sibling .rcsmodel reads");
    let (model, _report) =
        oag_mesh::mesh::rcs::build_scene(TRACK, &track_blob, &geometry, &mut |path| {
            archives.read_name(path).ok()
        })
        .expect("the track model builds");
    model
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn talons_junction_authors_a_billboard8_mount_and_it_is_not_flat() {
    let Some(image) = image() else { return };
    let model = track_model(&image);

    let mount = oag_render::gantry::mount(&model).expect("Talon's Junction authors a mount");
    println!(
        "mount: centre {:?} width {:.2} height {:.2} thickness {:.3} vertices {}",
        mount.centre, mount.width, mount.height, mount.thickness, mount.vertices
    );

    // Measured once by hand at this pass (see the lane's own report) and
    // pinned here with a tolerance wide enough to survive an unrelated model
    // decode change, tight enough that a wrong surface still fails it.
    assert!(
        (40.0..=50.0).contains(&mount.width),
        "unexpected mount width: {}",
        mount.width
    );
    assert!(
        (8.0..=12.0).contains(&mount.height),
        "unexpected mount height: {}",
        mount.height
    );
    // **The load-bearing assertion.** HD's own billboard8.gtf chunk here is
    // not Pulse's flat stub (thickness 0.000-0.100) - it is several units
    // deep. A future change that makes this assert near-zero has found a
    // *different* chunk, not a more accurate one, and should be treated as
    // a regression until checked against the disc by hand.
    assert!(
        mount.thickness > 1.0,
        "expected Talon's Junction's mount to measure as a real 3D structure \
         (thickness > 1.0), got {} - if this is now near zero, confirm which \
         chunk billboard8.gtf resolved to before trusting it",
        mount.thickness
    );
    assert!(
        mount.thickness < mount.height,
        "the mount should still read as plane-like against its own height, \
         not as a blob: thickness {} vs height {}",
        mount.thickness,
        mount.height
    );

    // Exactly one draw binds this texture on Talon's Junction - not a union
    // of several unrelated surfaces, which would produce a similar-looking
    // "thick" measurement for a completely different reason.
    let slots: Vec<usize> = model
        .textures
        .iter()
        .enumerate()
        .filter(|(_, t)| {
            t.as_ref().is_some_and(|t| {
                t.label
                    .rsplit(['/', '\\'])
                    .next()
                    .unwrap_or(&t.label)
                    .eq_ignore_ascii_case("billboard8.gtf")
            })
        })
        .map(|(slot, _)| slot)
        .collect();
    let draws = [
        &model.draws,
        &model.alpha_tested_draws,
        &model.transparent_draws,
    ]
    .into_iter()
    .flatten()
    .filter(|d| d.texture.is_some_and(|s| slots.contains(&s)))
    .count();
    assert_eq!(
        draws, 1,
        "expected exactly one draw bound to billboard8.gtf on Talon's Junction; \
         a union of several surfaces would explain a spurious thickness reading"
    );
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn race_load_places_a_gantry_on_talons_junction() {
    let Some(image) = image() else { return };

    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        team: Some("feisar_c1".to_string()),
        hull_variant: Some("concept1".to_string()),
        track: Some(TRACK.to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    let report = loaded.report.join("\n");
    println!("{report}");

    assert!(
        report.contains("start gantry"),
        "the gantry is not named: {report}"
    );
    assert!(
        report.contains("321Go_StartFinish.vex"),
        "the gantry line does not name HD's own model: {report}"
    );
    assert!(
        report.contains("measured off this circuit's own geometry"),
        "the gantry line does not say the mount is measured: {report}"
    );
    assert!(
        !report.contains("no start gantry"),
        "the report both places and fails to place the gantry: {report}"
    );

    assert!(
        loaded.billboards.gantry.is_none()
            && loaded
                .billboards
                .adverts
                .iter()
                .any(|card| card.slot == 8 && card.timeline.is_some()),
        "race::load did not carry slot 8 through to Loaded as a clocked card"
    );

    // Slot 7's own art, borrowed into slot 8's shared model - the ring/logo
    // cluster (`polySurface155/156/157`, local z ~33) that reads as a stray
    // "Wipeout symbol" floating over the open road once placed, matching a
    // player's own report. `oag_render::gantry::strip_fx350_art` drops every
    // draw bound to `fx350_nomip.gtf`, not just that trio, since the texture
    // is the disc's own discriminator - see that function's doc comment.
    assert!(
        report.contains("fx350_nomip.gtf"),
        "the report does not say the borrowed slot-7 art is stripped: {report}"
    );
}

/// The digit board's own `Anim Transform` teleports it +10 in world Y at the
/// 6.000 s loop close, sampled through this project's own model builder
/// (`oag_mesh::mesh::rcs::build` + `Model::sample_anim_nodes`) rather than
/// the raw `.vex` track `crates/hd/tests/start_gantry_hd_ground_truth.rs`
/// already pins - this is the integration half, that the pipeline `race::gantry`
/// actually calls reproduces the same number.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_digit_board_teleports_through_this_projects_own_pipeline() {
    let Some(image) = image() else { return };
    let mut archives = oag_hd::open(&image.display().to_string()).expect("opening the disc");

    let gantry_name = "/Data/Billboards/HD_Adverts/321Go/321Go_StartFinish.vex";
    let blob = archives
        .read_name(gantry_name)
        .expect("the gantry .vex reads");
    let sibling = oag_mesh::mesh::rcs::sibling_name(gantry_name).expect("an .rcsmodel sibling");
    let geometry = archives
        .read_name(&sibling)
        .expect("the sibling .rcsmodel reads");
    let (model, _report) = oag_mesh::mesh::rcs::build(
        gantry_name,
        &blob,
        &geometry,
        &mut |p| archives.read_name(p).ok(),
        |c| c.mesh,
    )
    .expect("the gantry model builds");

    // `pasted__Go_HD_start_light_321go` (`crates/hd/tests/start_gantry_hd_ground_truth.rs`'s
    // own `GLYPH_NODE`) is found here by which texture it binds and by being
    // the *largest* such draw, rather than by node index -
    // `oag_mesh::mesh::rcs::build`'s own node numbering is an implementation
    // detail this test should not have to track. Several small letter-shaped
    // meshes elsewhere in this file bind the same `321_go_64.gtf` texture at a
    // different material slot; the glyph board's own draw is roughly four
    // times any single one of theirs (measured: 570 indices against ~48).
    let digit_draw = [&model.draws, &model.transparent_draws]
        .into_iter()
        .flatten()
        .filter(|d| {
            d.texture
                .and_then(|t| model.textures.get(t))
                .and_then(|t| t.as_ref())
                .is_some_and(|t| t.label.to_ascii_lowercase().contains("321_go_64"))
        })
        .max_by_key(|d| d.range.len())
        .expect("a draw bound to the digit palette texture");
    let v0 = model.vertices[model.indices[digit_draw.range.start as usize] as usize];
    assert_ne!(
        v0.xform, 0,
        "the digit board should sit under an Anim Transform"
    );

    let sample = |seconds: f32| -> Vec3 {
        let matrices = model.sample_anim_nodes(seconds);
        let m = oag_core::math::Mat4::from_cols_array(&matrices[(v0.xform - 1) as usize]);
        m.transform_point3(Vec3::from(v0.position))
    };
    let before = sample(5.9);
    let after = sample(6.0);
    let delta_y = after.y - before.y;
    assert!(
        delta_y > 8.0,
        "expected the digit board to teleport roughly ten world units in Y \
         at the 6.000 s loop close, got {delta_y}"
    );
    assert!(
        (before.x - after.x).abs() < 1e-3 && (before.z - after.z).abs() < 1e-3,
        "the teleport should be a pure Y move: before {before:?} after {after:?}"
    );
}
