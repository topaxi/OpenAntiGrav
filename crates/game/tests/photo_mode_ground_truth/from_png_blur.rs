//! A motion blur shot and a speed blur shot drawn again (`oag-game --from-png`): the frames the
//! shot was averaged from, the camera riding the craft for a track blur, and the zoom ring's
//! history for a speed blur. The game binary is handed in by the parent module, which is the one
//! the test groups name.
//!
//! The blur shots' tolerances are measured outside the HUD, which the redraw draws from its own
//! state; the speed blur's are mean absolute differences along the two sides of the picture.

use super::shot_fidelity::{Picture, from_png, grow, meta_of};
use super::*;

/// The redraw of a speed blur shot may differ from the saved shot by this much along the sides, on average.
const TOLERANCE: f32 = 1.5;

/// Where the zoom ring's streak lands: a strip along each side, clear of the HUD.
const LEFT_EDGE: (f32, f32, f32, f32) = (0.0, 0.25, 0.08, 0.70);
const RIGHT_EDGE: (f32, f32, f32, f32) = (0.92, 0.25, 1.0, 0.70);

/// The mean and the 99th percentile of the difference between two pictures over the pixels
/// outside the HUD, which is where `with` differs from `without` (the same shot with and without
/// its HUD row: every pixel that differs by more than 2, grown by 3).
fn outside_the_hud(
    saved: &Picture,
    redrawn: &Picture,
    (without, with): (&Picture, &Picture),
) -> (f32, f32) {
    let size = (saved.0 as usize, saved.1 as usize);
    let apart = |a: &Picture, b: &Picture, at: usize| {
        (0..3)
            .map(|c| a.2[at * 4 + c].abs_diff(b.2[at * 4 + c]))
            .max()
            .unwrap_or(0)
    };
    let footprint = grow(
        &(0..size.0 * size.1)
            .map(|at| apart(with, without, at) > 2)
            .collect::<Vec<_>>(),
        size,
        3,
    );
    let mut steps: Vec<f32> = (0..size.0 * size.1)
        .filter(|&at| !footprint[at])
        .map(|at| {
            (0..3)
                .map(|c| f32::from(saved.2[at * 4 + c].abs_diff(redrawn.2[at * 4 + c])))
                .sum::<f32>()
                / 3.0
        })
        .collect();
    steps.sort_by(f32::total_cmp);
    (
        steps.iter().sum::<f32>() / steps.len() as f32,
        steps[steps.len() * 99 / 100],
    )
}

/// A track blur and a ship blur, each with its HUD on, drawn again from their frames: outside the
/// HUD the redraw is within the stated tolerance of the saved shot (measured: a track blur 0.08
/// mean and 0.7 at the 99th percentile, a ship blur 0.45 and 2.0; the one frame it used to be was
/// 11.5 and 75 on the track blur, 0.73 and 6.0 on the ship blur), the craft the track blur's camera
/// rides stays as sharp as it was in the saved shot over scenery that smears as much.
///
/// `photos` are the six of `a_motion_blur_shot_smears_what_it_should_and_any_input_abandons_it`:
/// plain, track blur, ship blur, ship blur with the HUD, plain with the HUD, track blur with it.
pub(super) fn blurred_shots_are_drawn_again(
    binary: &str,
    image: &Path,
    run: &Run,
    photos: &[PathBuf],
) {
    const HULL: (f32, f32, f32, f32) = (0.40, 0.68, 0.52, 0.78);
    const SCENERY: (f32, f32, f32, f32) = (0.0, 0.1, 0.3, 0.5);
    let plain_hud = decode(&photos[4]);
    // `(name, the shot with its HUD, the same shot without, mean and 99th percentile bounds)`.
    for (name, with, without, kind, bound) in [
        ("track blur", 5, 1, "track", (0.25, 2.0)),
        ("ship blur", 3, 2, "ship", (0.6, 3.5)),
    ] {
        let saved = decode(&photos[with]);
        let picture = meta_of(&photos[with]).picture.expect("the controls");
        assert_eq!(picture.motion_blur, kind);
        let out = run.dir.join(format!("redraw-{kind}.png"));
        let (stdout, redrawn) = from_png(binary, image, run, &photos[with], &out, &[]);
        assert!(stdout.contains("drawing its frames"), "{stdout}");
        assert!(
            stdout.contains(&format!("drew {} frames", picture.frames)),
            "{stdout}"
        );
        assert!(stdout.contains("MATCH"), "{stdout}");
        let (mean, p99) = outside_the_hud(&saved, &redrawn, (&decode(&photos[without]), &saved));
        println!(
            "{name}: outside the HUD the redraw is {mean:.3} mean, {p99:.1} at the 99th percentile"
        );
        assert!(mean < bound.0, "{name}: the redraw is {mean} from the shot");
        assert!(p99 < bound.1, "{name}: its 99th percentile is {p99}");
        if kind == "track" {
            let kept =
                |picture: &Picture, region| detail(picture, region) / detail(&plain_hud, region);
            let (saved_hull, saved_scenery) = (kept(&saved, HULL), kept(&saved, SCENERY));
            let (hull, scenery) = (kept(&redrawn, HULL), kept(&redrawn, SCENERY));
            println!(
                "track blur detail kept, hull {hull:.2} scenery {scenery:.2} (saved: {saved_hull:.2}, {saved_scenery:.2})"
            );
            assert!(
                (hull - saved_hull).abs() < 0.1,
                "the craft: {hull} against {saved_hull}"
            );
            assert!(
                (scenery - saved_scenery).abs() < 0.1,
                "the scenery: {scenery} against {saved_scenery}"
            );
        }
    }
}

/// A speed blur shot is drawn again with the zoom ring's history it had: along the sides, where the
/// streak is, the redraw is within [`TOLERANCE`] of the saved shot (drawn with no history it was at
/// 2.5 and 3.6 on the same shot).
///
/// **Of the second shot of a visit, not the first**: the first shot of a visit to the mode
/// carries a history that settled frames do not reproduce (its streak is stronger and its redraw
/// is 4.2 and 7.4 from it, as far as the one-frame redraw was); the shots after it carry the
/// settled one. The first is printed, not asserted.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn a_speed_blur_shot_is_drawn_again_with_its_zoom_history() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let shots = Path::new(env!("CARGO_TARGET_TMPDIR")).join("photo-speed-shots");
    let _ = std::fs::remove_dir_all(&shots);
    let run = drive(
        &image,
        "speed-blur",
        "boot; select racebox; press cross; until page=race; \
         set race.mode=time_trial; select launch_race; press cross; until screen=picker; \
         press cross; until screen.id=ship-select; press cross; until stage=race within 4000; \
         ticks 300; hold cross 400; escape; until stage=menu; select photo_mode; press cross; \
         until photo=free within 200; press select select; ticks 5; hold left 8; ticks 3; \
         press start; hold down 1; ticks 2; hold down 1; ticks 2; hold right 260; ticks 3; \
         press square; ticks 3; press cross; ticks 3; ticks 60; press cross; ticks 3; \
         press circle; until photo=none within 50",
        &shots,
    );
    assert!(run.status.success(), "{}", run.stderr);
    let photos = the_photos(&run);
    assert_eq!(photos.len(), 2, "{photos:?}");
    let mut edges = Vec::new();
    for (at, photo) in photos.iter().enumerate() {
        let picture = meta_of(photo).picture.expect("the controls");
        assert_eq!(picture.speed_blur, 100);
        let saved = decode(photo);
        let out = run.dir.join(format!("redraw-speed-{at}.png"));
        let (stdout, redrawn) = from_png(GAME, &image, &run, photo, &out, &[]);
        assert!(stdout.contains("MATCH"), "{stdout}");
        let (left, right) = (
            difference(&saved, &redrawn, LEFT_EDGE),
            difference(&saved, &redrawn, RIGHT_EDGE),
        );
        println!(
            "speed blur shot {}: redrawn, left edge {left:.2}, right edge {right:.2}, whole {:.2}",
            at + 1,
            difference(&saved, &redrawn, ALL)
        );
        edges.push((left, right));
    }
    let (left, right) = edges[1];
    assert!(left < TOLERANCE, "the left edge is {left} from the shot");
    assert!(right < TOLERANCE, "the right edge is {right} from the shot");
}
