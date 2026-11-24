//! A saved photo is drawn at the output's size with no upscaler: the file does not depend on the
//! render scale or the upscaler the player plays with. Each test makes one set of shots on the
//! same race and tick in three profiles - half size through FSR 3, three quarters through FSR 1
//! and the default (100 %, no upscaler) - and compares the files.
//!
//! The two scaled profiles redraw the shot at the output's size, so they hold the same picture
//! to the bit. The default does not redraw: its picture is the frame the preview left, and on
//! HD the bloom's exposure adaptation is where it differs (`HD_TOLERANCE`).

use super::shot_fidelity::{self, meta_of};
use super::*;

/// The profiles a player trades quality for speed with.
const HALF_FSR3: [&str; 4] = ["--render-scale", "50", "--reconstruction", "fsr3"];
const THREE_QUARTERS_FSR1: [&str; 4] = ["--render-scale", "75", "--reconstruction", "fsr1"];

/// The most one channel of a pixel and the mean over the picture may differ between a shot that
/// was redrawn at the output's size and the default's, which is not redrawn. Pulse has no
/// difference. HD's bloom restarts its exposure adaptation on the reallocation and seeds it from
/// the frame, where the default's has drifted over the frames of the preview: a mean of 0.6 of
/// 255 and at most 8, measured with the race stood at one tick.
const HD_TOLERANCE: (u8, f32) = (8, 1.0);

/// How much of the picture a `--from-png` re-render may differ in, and by how much on average:
/// the craft's own animation is drawn from the clock and not from the race, so the hull and
/// its plume are not the same pixels twice (measured 0.5 % of the picture, a mean of 0.006).
const REPRODUCED_SHARE: f32 = 0.01;
const REPRODUCED_MEAN: f32 = 0.1;

/// The most one channel of any pixel differs between two pictures of one size.
fn widest(a: &(u32, u32, Vec<u8>), b: &(u32, u32, Vec<u8>)) -> u8 {
    assert_eq!((a.0, a.1), (b.0, b.1), "the pictures are not one size");
    a.2.iter()
        .zip(&b.2)
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap_or(0)
}

/// Whether some state of `run` reads `needle` in the mode's picture controls.
fn fx_says(run: &Run, needle: &str) -> bool {
    run.states.iter().any(|state| {
        state
            .get("photo_fx")
            .and_then(Value::as_str)
            .is_some_and(|fx| fx.contains(needle))
    })
}

/// The shots `script` saves in each profile: the run, and its photos in the order taken.
struct Made {
    run: Run,
    photos: Vec<PathBuf>,
}

fn made(image: &Path, tag: &str, name: &str, script: &str, args: &[&str], count: usize) -> Made {
    let shots = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("photo-native-{tag}-shots"));
    let _ = std::fs::remove_dir_all(&shots);
    let run = drive_unmodded(image, &format!("native-{tag}-{name}"), script, &shots, args);
    assert!(run.status.success(), "{name}: {}", run.stderr);
    for bad in ["wgpu error", "Validation Error", "panicked"] {
        assert!(
            !run.stderr.contains(bad),
            "{name}: {bad} in\n{}",
            run.stderr
        );
    }
    let photos = the_photos(&run);
    assert_eq!(photos.len(), count, "{name}: {photos:?}");
    Made { run, photos }
}

/// Makes the shots in the three profiles and checks that the two scaled ones are the same
/// pictures and the default's are within `tolerance` of them.
fn same_shots(
    tag: &str,
    image: &Path,
    script: &str,
    count: usize,
    needle: &str,
    tolerance: (u8, f32),
) -> [Made; 3] {
    let all = [
        made(image, tag, "half-fsr3", script, &HALF_FSR3, count),
        made(
            image,
            tag,
            "three-quarters-fsr1",
            script,
            &THREE_QUARTERS_FSR1,
            count,
        ),
        made(image, tag, "default", script, &[], count),
    ];
    for one in &all {
        assert!(
            fx_says(&one.run, needle),
            "{tag}: the run never read {needle}"
        );
    }
    for at in 0..count {
        let [a, b, default] = [&all[0], &all[1], &all[2]].map(|one| decode(&one.photos[at]));
        let (worst, mean) = (widest(&a, &b), difference(&a, &b, ALL));
        println!("{tag} shot {at}: half+fsr3 against 75%+fsr1, widest {worst}, mean {mean:.4}");
        assert_eq!(
            worst, 0,
            "{tag} shot {at}: the shot depends on the render scale or the upscaler (mean {mean})"
        );
        let (worst, mean) = (widest(&a, &default), difference(&a, &default, ALL));
        println!("{tag} shot {at}: half+fsr3 against the default, widest {worst}, mean {mean:.4}");
        assert!(
            worst <= tolerance.0 && mean <= tolerance.1,
            "{tag} shot {at}: the default's shot is not the redrawn one (widest {worst}, mean {mean})"
        );
    }
    all
}

/// What `--from-png` draws of shot `at` of `shots` is that shot again, whatever the profile it
/// runs in, outside `skip`: the HUD, which the re-render draws from the race as it stands.
fn drawn_again(tag: &str, image: &Path, shots: &[Made; 3], at: usize, skip: &[bool]) {
    let shot = decode(&shots[0].photos[at]);
    for (name, flags, one) in [
        ("half+fsr3", &HALF_FSR3[..], &shots[0]),
        ("75%+fsr1", &THREE_QUARTERS_FSR1[..], &shots[1]),
        ("default", &[][..], &shots[2]),
    ] {
        let out = one.run.dir.join(format!("rerender-{tag}.png"));
        let (stdout, again) =
            shot_fidelity::from_png(GAME, image, &one.run, &one.photos[at], &out, flags);
        assert!(stdout.contains("MATCH"), "{tag} {name}: {stdout}");
        let outside: Vec<usize> = (0..skip.len()).filter(|&pixel| !skip[pixel]).collect();
        let apart = |pixel: usize| {
            (0..3)
                .map(|c| shot.2[pixel * 4 + c].abs_diff(again.2[pixel * 4 + c]))
                .max()
                .unwrap_or(0)
        };
        let differing = outside.iter().filter(|&&pixel| apart(pixel) > 0).count();
        let share = differing as f32 / outside.len() as f32;
        let mean = outside
            .iter()
            .map(|&pixel| f32::from(apart(pixel)))
            .sum::<f32>()
            / outside.len() as f32;
        println!(
            "{tag} {name}: re-render against the shot outside the HUD, {differing} pixels differ \
             ({:.3} %), mean {mean:.4}",
            share * 100.0
        );
        assert!(
            share < REPRODUCED_SHARE && mean < REPRODUCED_MEAN,
            "{tag} {name}: --from-png does not reproduce the shot ({share}, {mean})"
        );
    }
}

/// A plain frame, then the same frame with the HUD drawn over it.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_single_frame_shot_does_not_depend_on_the_render_scale_or_the_upscaler() {
    let Some(image) = pulse() else {
        return;
    };
    let script = format!(
        "{TO_THE_RACE} escape; until stage=menu; select photo_mode; press cross; \
         until photo=free within 200; press select select; ticks 20; \
         press cross; ticks 2; screenshot SHOTS/plain.png; ticks 8; \
         press square; ticks 3; press cross; ticks 2; screenshot SHOTS/hud.png; ticks 8; \
         press circle; until photo=none within 50"
    );
    let shots = same_shots("single", &image, &script, 2, "hud=1", (0, 0.0));
    // The HUD is in the second shot, at the output's size, and not in the first. `--from-png`
    // draws the race's HUD as the race stands, so the re-render is held to the shot outside it.
    let [plain, hud] = [0, 1].map(|at| decode(&shots[0].photos[at]));
    let size = (plain.0 as usize, plain.1 as usize);
    let hud_pixels: Vec<bool> = (0..size.0 * size.1)
        .map(|pixel| (0..3).any(|c| plain.2[pixel * 4 + c] != hud.2[pixel * 4 + c]))
        .collect();
    let mut footprint = shot_fidelity::grow(&hud_pixels, size, 4);
    // The now-playing banner along the bottom: the re-render shows it and the shot does not.
    for y in (size.1 * 90 / 100)..size.1 {
        for x in (size.0 * 26 / 100)..(size.0 * 60 / 100) {
            footprint[y * size.0 + x] = true;
        }
    }
    assert!(
        footprint.iter().filter(|&&on| on).count() > 2000,
        "the HUD shot is the plain one"
    );
    drawn_again("single", &image, &shots, 1, &footprint);
}

/// HD's motion blur at its longest shutter, a track blur, with the HUD on.
const HD_BLUR: &str = "boot; select racebox; press cross; until page=race; \
     set race.mode=time_trial; select launch_race; press cross; until screen=picker; \
     press cross; until screen.id=ship-select; press cross; until stage=race within 4000; \
     ticks 300; hold cross 400; escape; until stage=menu; select photo_mode; press cross; \
     until photo=free within 200; press select select; ticks 5; hold left 8; ticks 3; \
     press start; hold down 1; ticks 2; hold down 1; ticks 2; hold down 1; ticks 2; \
     hold down 1; ticks 2; hold right 1; ticks 2; hold down 1; ticks 2; hold right 260; ticks 3; \
     press square; ticks 3; press cross; ticks 3; until photo_job=none within 800; ticks 3; \
     press circle; until photo=none within 50";

/// A motion blur shot of several frames, and the HUD drawn once over it at the output's size.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn a_motion_blur_shot_does_not_depend_on_the_render_scale_or_the_upscaler() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let shots = same_shots("blur", &image, HD_BLUR, 1, "hud=1", HD_TOLERANCE);
    let picture = meta_of(&shots[0].photos[0]).picture.expect("the controls");
    assert!(!picture.motion_blur.is_empty(), "the shot is not a blur");
}

/// HD's depth of field: the lens samples of one tick, averaged.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn a_depth_of_field_shot_does_not_depend_on_the_render_scale_or_the_upscaler() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let script = "boot; select racebox; press cross; until page=race; \
         set race.mode=time_trial; select launch_race; press cross; until screen=picker; \
         press cross; until screen.id=ship-select; press cross; until stage=race within 4000; \
         ticks 300; hold cross 400; escape; until stage=menu; select photo_mode; press cross; \
         until photo=free within 200; press select select; ticks 5; hold left 8; ticks 3; \
         press start; hold down 1; ticks 2; hold down 1; ticks 2; hold down 1; ticks 2; \
         hold right 1; ticks 3; press cross; ticks 3; until photo_job=none within 800; ticks 3; \
         press circle; until photo=none within 50";
    let shots = same_shots("dof", &image, script, 1, "dof=on", HD_TOLERANCE);
    let picture = meta_of(&shots[0].photos[0]).picture.expect("the controls");
    assert!(picture.depth_of_field, "the shot is not a depth of field");
}

/// Input in the frame that shows the bar of a shot that is to be redrawn at the output's size
/// cancels it: nothing is saved. The same keys with a default profile (nothing to redraw) save
/// the shot on the first press.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn input_before_a_redrawn_shot_begins_cancels_it() {
    let Some(image) = pulse() else {
        return;
    };
    let script = format!(
        "{TO_THE_RACE} escape; until stage=menu; select photo_mode; press cross; \
         until photo=free within 200; press select select; ticks 20; \
         press cross; press cross; ticks 6; screenshot SHOTS/after.png; ticks 6; \
         press circle; until photo=none within 50"
    );
    let shots = Path::new(env!("CARGO_TARGET_TMPDIR")).join("photo-native-cancel-shots");
    let _ = std::fs::remove_dir_all(&shots);
    let scaled = drive_unmodded(&image, "native-cancel-scaled", &script, &shots, &HALF_FSR3);
    assert!(scaled.status.success(), "{}", scaled.stderr);
    assert!(the_photos(&scaled).is_empty(), "a cancelled shot was saved");
    let default = drive_unmodded(&image, "native-cancel-default", &script, &shots, &[]);
    assert!(default.status.success(), "{}", default.stderr);
    assert_eq!(the_photos(&default).len(), 1, "the default's shot is saved");
}
