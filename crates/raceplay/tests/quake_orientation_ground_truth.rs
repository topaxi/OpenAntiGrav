//! The Quake wave's effect frame, per title.
//!
//! **Pulse.** `Quake_Update` (`0x0891d268`) builds row 1 of the `WO_QUAKE`
//! frame as `-(+0x20)` of the record `AiTrack_LocatePosition` fills at the
//! span's midpoint - the track's interpolated `down` axis negated - and not as
//! world up. On a banked stretch the two differ; dropping the `orient` call in
//! `Race::advance_quake_visual` takes this to world up everywhere. The anchor
//! is the midpoint itself.
//!
//! **HD's** wave takes the same path, and only because its `Title` lifts the
//! extent law for `WO_QUAKE`: with the effect not listed `quake_frame_up` is
//! `None` everywhere, the frame never being built.
//!
//! **Pure.** `Quake_Update` (`0x08879d54`) loads `(0, 1, 0)` for the frame's
//! `Y` and calls no track lookup, and raises the anchor `10 * ((across x Y) x
//! across)` above the midpoint (`psp-pure-usa/quake.md`): world up on a banked
//! circuit where the track's up leans, and an anchor `5..=10` units above the
//! road. Dropping `oag_pure::effects`'s `extents`, its `QuakeUp::World` or its
//! `lift` fails the matching assertion here.
//!
//! **`#[ignore]`d and never run in CI**: it needs a disc image.

use oag_core::math::Vec3;
use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

/// What a title's wave builds, as the test asserts it.
#[derive(Clone, Copy)]
enum Frame {
    /// The track's up, the anchor on the road; `must_lean` demands a banked
    /// stretch on the sampled circuit, and `washes` that the wave tints the
    /// screen (Pulse's PSP does, HD draws none).
    Track { must_lean: bool, washes: bool },
    /// World up, the anchor lifted, and the wave's own wash: Pure's.
    WorldLifted,
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_quake_frame_leans_with_the_banked_track() {
    lean_with_the_track(
        "pulse-psp-usa.chd",
        Frame::Track {
            must_lean: true,
            washes: true,
        },
    );
}

/// HD's Talon's Junction is flat where the wave runs, so only "the frame is
/// built, and is the track's up" is asked of it.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_hd_quake_frame_is_built_from_the_track() {
    lean_with_the_track(
        "hdfury-ps3-eu-dec.iso",
        Frame::Track {
            must_lean: false,
            washes: false,
        },
    );
}

/// Pure's frame is world up on both pressings' shared circuit, and the circuit
/// has a banked stretch where the track's up would differ.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_pure_quake_frame_is_world_up_and_the_anchor_is_lifted() {
    lean_with_the_track("pure-psp-usa.chd", Frame::WorldLifted);
}

fn lean_with_the_track(image: &str, expect: Frame) {
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
    let stats = race.quake_stats().expect("the title authors a Quake");
    let length = race.course().expect("a course").length();
    let (mut leaned, mut built, mut washed) = (0, 0, 0);
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
        let anchor = race.quake_anchor().expect("an anchor");
        let (_, sample, _) = race.spline().nearest(point).expect("a sample");
        let track_up = (-Vec3::from_array(sample.down)).normalize();
        let lift = anchor - point;
        if race.screen_wash().is_some() {
            washed += 1;
        }
        if track_up.dot(Vec3::Y) < 0.98 {
            leaned += 1;
        }
        match expect {
            Frame::Track { .. } => {
                // The midpoint is not the course point the wave samples at, so
                // the match is to a few hundredths, not exact.
                assert!(
                    (up - track_up).length() < 0.05,
                    "step {step}: frame up {up:?}, track up {track_up:?}"
                );
                assert!(lift.length() < 1.0e-4, "step {step}: lifted by {lift:?}");
            }
            Frame::WorldLifted => {
                assert_eq!(up, Vec3::Y, "step {step}: Pure's frame is world up");
                assert!(
                    lift.length() <= 10.001 && (5.0..=10.001).contains(&lift.y),
                    "step {step}: Pure raises the anchor 10 units along up, got {lift:?}"
                );
            }
        }
    }
    assert!(built > 0, "{image}: the wave never built its frame");
    let (banked, washes) = match expect {
        Frame::Track { must_lean, washes } => (must_lean, washes),
        Frame::WorldLifted => (true, true),
    };
    assert!(
        !banked || leaned > 0,
        "no banked stretch on the sampled circuit"
    );
    assert_eq!(
        washed > 0,
        washes,
        "{image}: the wave washed the screen on {washed} of {built} steps"
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
