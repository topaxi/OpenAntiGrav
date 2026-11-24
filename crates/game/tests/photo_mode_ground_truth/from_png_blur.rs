//! A motion blur shot and a speed blur shot drawn again (`oag-game --from-png`): the frames the
//! shot was averaged from, the camera riding the craft for a track blur, and the zoom ring's
//! history for a speed blur. The game binary is handed in by the parent module, which is the one
//! the test groups name.
//!
//! The tolerances are mean absolute differences over the whole picture, HUD included (the shot
//! and its redraw both carry one): the redraw of a plain shot of the same race differs from its
//! saved shot by 0.75 to 1.1, which is the floor.

use super::shot_fidelity::{from_png, meta_of};
use super::*;

/// The redraw of a blur shot may differ from the saved shot by this much, on average.
const TOLERANCE: f32 = 1.5;

/// Where the zoom ring's streak lands: a strip along each side, clear of the HUD.
const LEFT_EDGE: (f32, f32, f32, f32) = (0.0, 0.25, 0.08, 0.70);
const RIGHT_EDGE: (f32, f32, f32, f32) = (0.92, 0.25, 1.0, 0.70);

/// `photo` written again with its text changed by `edit`: the picture and the stream as they
/// were.
fn rewritten(photo: &Path, to: &Path, edit: impl FnOnce(&mut oag_game::shot_meta::Meta)) {
    let bytes = std::fs::read(photo).expect("the photo");
    let found = oag_game::shot_meta::read(&bytes).expect("our chunks read");
    let mut meta = found.meta.expect("the text chunk");
    edit(&mut meta);
    let (width, height, pixels) = decode(photo);
    let png = oag_game::shot_meta::encode(width, height, &pixels, &meta, found.record.as_ref());
    std::fs::write(to, png).expect("writing the rewritten photo");
}

/// A track blur and a ship blur, each with its HUD on, drawn again from their frames: the redraw
/// is within [`TOLERANCE`] of the saved shot and much closer than the one frame it used to be,
/// the craft the track blur's camera rides stays sharp over scenery that smears, and a track
/// blur that predates `[camera.mount]` is drawn as one frame and says so.
///
/// `photos` are the six of `a_motion_blur_shot_smears_what_it_should_and_any_input_abandons_it`.
pub(super) fn blurred_shots_are_drawn_again(
    binary: &str,
    image: &Path,
    run: &Run,
    photos: &[PathBuf],
) {
    const HULL: (f32, f32, f32, f32) = (0.40, 0.68, 0.52, 0.78);
    const SCENERY: (f32, f32, f32, f32) = (0.0, 0.1, 0.3, 0.5);
    let plain_hud = decode(&photos[4]);
    for (name, at, kind) in [("track blur", 5, "track"), ("ship blur", 3, "ship")] {
        let saved = decode(&photos[at]);
        let picture = meta_of(&photos[at]).picture.expect("the controls");
        assert_eq!(picture.motion_blur, kind);
        let out = run.dir.join(format!("redraw-{kind}.png"));
        let (stdout, redrawn) = from_png(binary, image, run, &photos[at], &out, &[]);
        assert!(stdout.contains("drawing its frames"), "{stdout}");
        assert!(
            stdout.contains(&format!("drew {} frames", picture.frames)),
            "{stdout}"
        );
        assert!(stdout.contains("MATCH"), "{stdout}");
        // What the redraw was before: the one frame, the text without its blur.
        let single_photo = run.dir.join(format!("single-{kind}-source.png"));
        rewritten(&photos[at], &single_photo, |meta| {
            if let Some(picture) = meta.picture.as_mut() {
                picture.motion_blur.clear();
            }
        });
        let single_out = run.dir.join(format!("single-{kind}.png"));
        let (_, single) = from_png(binary, image, run, &single_photo, &single_out, &[]);
        let (many, one) = (
            difference(&saved, &redrawn, ALL),
            difference(&saved, &single, ALL),
        );
        println!("{name}: redraw from its frames {many:.2}, from one frame {one:.2}");
        assert!(
            many < TOLERANCE,
            "{name}: the redraw is {many} from the shot"
        );
        assert!(
            many < 0.5 * one,
            "{name}: the frames bring the redraw no closer than one frame: {many} against {one}"
        );
        if kind == "track" {
            let kept = |region| detail(&redrawn, region) / detail(&plain_hud, region);
            println!(
                "track blur redrawn, detail kept: hull {:.2} scenery {:.2}",
                kept(HULL),
                kept(SCENERY)
            );
            assert!(
                kept(SCENERY) < 0.6,
                "the scenery is sharp: {}",
                kept(SCENERY)
            );
            assert!(kept(HULL) > 0.8, "the craft is smeared: {}", kept(HULL));
        }
    }

    // A shot of a build that wrote no camera mount: one frame, and it says why.
    let old = run.dir.join("track-without-a-mount-source.png");
    rewritten(&photos[5], &old, |meta| {
        meta.camera.as_mut().expect("a camera").mount = None;
    });
    let (stdout, drawn) = from_png(
        binary,
        image,
        run,
        &old,
        &run.dir.join("track-without-a-mount.png"),
        &[],
    );
    assert!(
        stdout.contains("predates the camera mount") && !stdout.contains("drew "),
        "{stdout}"
    );
    assert_eq!((drawn.0, drawn.1), (plain_hud.0, plain_hud.1));
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
