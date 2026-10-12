//! The Quake wave's effect frame takes its `+Y` from the track.
//!
//! `Quake_Update` (`0x0891d268`) builds row 1 of the `WO_QUAKE` frame as
//! `-(+0x20)` of the record `AiTrack_LocatePosition` fills at the span's
//! midpoint - the track's interpolated `down` axis negated - and not as world
//! up. On a banked stretch the two differ; dropping the `orient` call in
//! `Race::advance_quake_visual` takes this to world up everywhere.
//!
//! HD's wave takes the same path, and only because its `Title` lifts the extent
//! law for `WO_QUAKE`: with the effect not listed `quake_frame_up` is `None`
//! everywhere, the frame never being built.
//!
//! **`#[ignore]`d and never run in CI**: it needs a disc image.

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_quake_frame_leans_with_the_banked_track() {
    lean_with_the_track("pulse-psp-usa.chd", true);
}

/// HD's Talon's Junction is flat where the wave runs, so only "the frame is
/// built, and is the track's up" is asked of it.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_hd_quake_frame_is_built_from_the_track() {
    lean_with_the_track("hdfury-ps3-eu-dec.iso", false);
}

fn lean_with_the_track(image: &str, must_lean: bool) {
    let Some(path) = oag_testdata::image(image) else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: path.display().to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    for _ in 0..5 {
        race.tick(&PlayerInputs::none());
    }
    let stats = race.quake_stats().expect("Pulse authors a Quake");
    let length = race.course().expect("a course").length();
    let (mut leaned, mut built) = (0, 0);
    for step in 0..40 {
        let progress = length * (step as f32 / 40.0);
        race.sim.world.quake = Some(oag_weapons::projectile::quake::Wave::launch(
            0, progress, 1.0, &stats,
        ));
        race.tick(&PlayerInputs::none());
        let Some(up) = race.quake_frame_up() else {
            continue;
        };
        built += 1;
        let point = race.quake_point().expect("a road point");
        let (_, sample, _) = race.spline().nearest(point).expect("a sample");
        // The midpoint is not the course point the wave samples at, so the
        // match is to a few hundredths, not exact.
        let expected = (-oag_core::math::Vec3::from_array(sample.down)).normalize();
        assert!(
            (up - expected).length() < 0.05,
            "step {step}: frame up {up:?}, track up {expected:?}"
        );
        if up.dot(oag_core::math::Vec3::Y) < 0.98 {
            leaned += 1;
        }
    }
    assert!(built > 0, "{image}: the wave never built its frame");
    assert!(
        !must_lean || leaned > 0,
        "no banked stretch on the sampled circuit"
    );
}

/// How many of a model's draws start on a vertex carrying `GpuVertex::quake`,
/// and how many draws it has.
fn quake_draws(model: &oag_mesh::mesh::Model) -> (usize, usize) {
    let lists = [
        &model.draws,
        &model.alpha_tested_draws,
        &model.transparent_draws,
    ];
    let all: usize = lists.iter().map(|l| l.len()).sum();
    let flagged = lists
        .iter()
        .flat_map(|list| list.iter())
        .filter(|draw| {
            let first = model.indices[draw.range.start as usize] as usize;
            model.vertices[first].quake > 0.5
        })
        .count();
    (flagged, all)
}

/// The surfaces the Quake's displacement reaches are the materials that ship a
/// `StaticQuake` vertex program: on Talon's Junction 281 of 2,211 draws, read off
/// the materials themselves (`quake.wesl`, `docs/ghidra/functions/
/// ps3-hdfury-eu/quake.md`). Dropping the flag from `skin::variants` marks none,
/// and no other title marks any: Pulse's own track carries no `StaticQuake`
/// program and moves through `oag_render::ripple` instead.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_hd_road_marks_the_surfaces_a_staticquake_program_reaches() {
    let (Some(hd), Some(pulse)) = (
        oag_testdata::image("hdfury-ps3-eu-dec.iso"),
        oag_testdata::image("pulse-psp-usa.chd"),
    ) else {
        return;
    };
    let load = |path: std::path::PathBuf| {
        race::load(&race::Options {
            source: path.display().to_string(),
            mode: oag_race::Mode::SingleRace,
            ..race::Options::default()
        })
        .expect("loading the race")
    };
    let (flagged, all) = quake_draws(&load(hd).track_model);
    assert!(all > 2000, "Talon's Junction has {all} draws");
    assert!(
        (200..400).contains(&flagged),
        "{flagged} of {all} HD draws carry `quake`; the disc's materials say about 281"
    );
    let (flagged, all) = quake_draws(&load(pulse).track_model);
    assert!(all > 0);
    assert_eq!(flagged, 0, "Pulse marks no surface for the shader's wave");
}
