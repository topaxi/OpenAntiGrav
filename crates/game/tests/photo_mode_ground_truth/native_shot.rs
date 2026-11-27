//! A saved photo is drawn at no less than the output's size and with no upscaler: the file does
//! not depend on the render scale below 100 or the upscaler the player plays with, and a
//! supersampling scale is the player's own. Each test makes one set of shots on the same race
//! and tick in several profiles and compares the files.

use super::shot_fidelity::{self, meta_of};
use super::*;

type Profile = (&'static str, &'static [&'static str]);

/// The profiles that trade quality for speed: every one is drawn at 100 % with no upscaler.
const HALF_FSR3: Profile = (
    "half+fsr3",
    &["--render-scale", "50", "--reconstruction", "fsr3"],
);
const THREE_QUARTERS_FSR1: Profile = (
    "75%+fsr1",
    &["--render-scale", "75", "--reconstruction", "fsr1"],
);
const FULL_FSR3: Profile = ("100%+fsr3", &["--reconstruction", "fsr3"]);
const FULL_FSR1: Profile = ("100%+fsr1", &["--reconstruction", "fsr1"]);
const DEFAULT: Profile = ("default", &[]);
const S150: Profile = ("150%", &["--render-scale", "150"]);
const S150_FSR3: Profile = (
    "150%+fsr3",
    &["--render-scale", "150", "--reconstruction", "fsr3"],
);
const S200: Profile = ("200%", &["--render-scale", "200"]);
const S200_FSR3: Profile = (
    "200%+fsr3",
    &["--render-scale", "200", "--reconstruction", "fsr3"],
);
const REDRAWN: [Profile; 5] = [
    HALF_FSR3,
    THREE_QUARTERS_FSR1,
    FULL_FSR3,
    FULL_FSR1,
    DEFAULT,
];

/// How far a shot redrawn at the output's size and the default's, which is not redrawn, may
/// differ (the widest channel, and the mean over the picture), and two redrawn shots (none).
const SAME: (u8, f32) = (0, 0.0);

/// HD's shot against the default's: one level of one channel on a few pixels (a mean of 0.01).
const HD_DEFAULT: (u8, f32) = (1, 0.02);

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

/// The shots `script` saves in one profile: the run, and its photos in the order taken.
struct Made {
    name: &'static str,
    run: Run,
    photos: Vec<PathBuf>,
}

fn made(
    image: &Path,
    tag: &str,
    (name, flags): Profile,
    script: &str,
    count: usize,
    needle: &str,
) -> Made {
    let shots = oag_testdata::scratch::dir(&format!("photo-native-{tag}-shots"));
    let slug: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let run = drive_unmodded(
        image,
        &format!("native-{tag}-{slug}"),
        script,
        &shots,
        flags,
    );
    assert!(run.status.success(), "{name}: {}", run.stderr);
    for bad in ["wgpu error", "Validation Error", "panicked"] {
        assert!(
            !run.stderr.contains(bad),
            "{name}: {bad} in\n{}",
            run.stderr
        );
    }
    assert!(
        fx_says(&run, needle),
        "{tag} {name}: the run never read {needle}"
    );
    let photos = the_photos(&run);
    assert_eq!(photos.len(), count, "{name}: {photos:?}");
    Made { name, run, photos }
}

fn all_made(
    image: &Path,
    tag: &str,
    profiles: &[Profile],
    script: &str,
    count: usize,
    needle: &str,
) -> Vec<Made> {
    profiles
        .iter()
        .map(|profile| made(image, tag, *profile, script, count, needle))
        .collect()
}

/// The difference between shot `at` of `a` and of `b`: the widest channel and the mean.
fn apart(a: &Made, b: &Made, at: usize) -> (u8, f32) {
    let (x, y) = (decode(&a.photos[at]), decode(&b.photos[at]));
    let result = (widest(&x, &y), difference(&x, &y, ALL));
    println!(
        "shot {at}: {} against {}, widest {}, mean {:.4}",
        a.name, b.name, result.0, result.1
    );
    result
}

/// Every shot of `shots` is the first one's, to the bit, except the default profile's, which may
/// differ by `default` (it is not redrawn: its picture is the frame the preview left).
fn all_the_same(tag: &str, shots: &[Made], count: usize, default: (u8, f32)) {
    for other in &shots[1..] {
        let tolerance = if other.name == DEFAULT.0 {
            default
        } else {
            SAME
        };
        for at in 0..count {
            let (worst, mean) = apart(&shots[0], other, at);
            assert!(
                worst <= tolerance.0 && mean <= tolerance.1,
                "{tag} shot {at}: {} is not {} (widest {worst}, mean {mean})",
                other.name,
                shots[0].name
            );
        }
    }
}

/// `a` and `b` are different pictures: the mean differs by more than `least`.
fn different(a: &Made, b: &Made, at: usize, least: f32) {
    let (_, mean) = apart(a, b, at);
    assert!(
        mean > least,
        "{} and {} gave one picture (mean {mean})",
        a.name,
        b.name
    );
}

/// What `--from-png` draws of shot `at` of `shots` is that shot again outside `skip` (the HUD,
/// which the re-render draws from the race as it stands), whatever render scale and upscaler the
/// run it is drawn in asks for: the run here is a hostile one, half size through FSR 3, and
/// `--presented`, which is what lets a run's render flags reach a capture at all.
fn drawn_again(tag: &str, image: &Path, shots: &[Made], at: usize, skip: &[bool]) {
    let flags = [HALF_FSR3.1, &["--presented"]].concat();
    for one in shots {
        let shot = decode(&one.photos[at]);
        let out = one.run.dir.join(format!("rerender-{tag}.png"));
        let (stdout, again) =
            shot_fidelity::from_png(GAME, image, &one.run, &one.photos[at], &out, &flags);
        assert!(stdout.contains("MATCH"), "{tag} {}: {stdout}", one.name);
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
            "{tag} {}: re-render against the shot outside the HUD, {differing} pixels differ \
             ({:.3} %), mean {mean:.4}",
            one.name,
            share * 100.0
        );
        assert!(
            share < REPRODUCED_SHARE && mean < REPRODUCED_MEAN,
            "{tag} {}: --from-png does not reproduce the shot ({share}, {mean})",
            one.name
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
         press cross; ticks 10; press square; ticks 3; press cross; ticks 10; \
         press circle; until photo=none within 50"
    );
    let shots = all_made(&image, "single", &REDRAWN, &script, 2, "hud=1");
    all_the_same("single", &shots, 2, SAME);
    drawn_again("single", &image, &shots, 1, &hud_footprint(&shots[0]));
}

/// Where the HUD of the second shot of `made` is, grown a little, and the now-playing banner
/// along the bottom that the re-render shows and a shot does not: the pixels a re-render is not
/// held to.
fn hud_footprint(made: &Made) -> Vec<bool> {
    let [plain, hud] = [0, 1].map(|at| decode(&made.photos[at]));
    let size = (plain.0 as usize, plain.1 as usize);
    let hud_pixels: Vec<bool> = (0..size.0 * size.1)
        .map(|pixel| (0..3).any(|c| plain.2[pixel * 4 + c] != hud.2[pixel * 4 + c]))
        .collect();
    let mut footprint = shot_fidelity::grow(&hud_pixels, size, 4);
    assert!(
        footprint.iter().filter(|&&on| on).count() > 2000,
        "the HUD shot is the plain one"
    );
    for y in (size.1 * 90 / 100)..size.1 {
        for x in (size.0 * 26 / 100)..(size.0 * 60 / 100) {
            footprint[y * size.0 + x] = true;
        }
    }
    footprint
}

/// The anti-aliasing a profile asks for stays in the shot: FXAA and SMAA at half size are the
/// shots they are at 100 %, and neither is the shot with none.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_anti_aliasing_the_profile_asks_for_stays_in_the_shot() {
    let Some(image) = pulse() else {
        return;
    };
    let script = format!(
        "{TO_THE_RACE} escape; until stage=menu; select photo_mode; press cross; \
         until photo=free within 200; press select select; ticks 20; \
         press cross; ticks 10; press square; ticks 3; press cross; ticks 10; \
         press circle; until photo=none within 50"
    );
    let fxaa: [Profile; 2] = [
        (
            "half+fxaa",
            &["--render-scale", "50", "--reconstruction", "fxaa"],
        ),
        ("100%+fxaa", &["--reconstruction", "fxaa"]),
    ];
    let smaa: [Profile; 2] = [
        (
            "half+smaa",
            &["--render-scale", "50", "--reconstruction", "smaa"],
        ),
        ("100%+smaa", &["--reconstruction", "smaa"]),
    ];
    let (fxaa, smaa) = (
        all_made(&image, "fxaa", &fxaa, &script, 2, "hud=1"),
        all_made(&image, "smaa", &smaa, &script, 2, "hud=1"),
    );
    all_the_same("fxaa", &fxaa, 2, SAME);
    all_the_same("smaa", &smaa, 2, SAME);
    let none = made(&image, "none", DEFAULT, &script, 2, "hud=1");
    different(&fxaa[0], &none, 0, 0.005);
    different(&smaa[0], &none, 0, 0.005);
    // A re-render keeps the anti-aliasing too.
    drawn_again("fxaa", &image, &fxaa, 1, &hud_footprint(&fxaa[0]));
    drawn_again("smaa", &image, &smaa, 1, &hud_footprint(&smaa[0]));
}

/// A supersampling scale is the player's own: the shot is drawn at 150 % and 200 % and
/// downsampled to the output's size, whether or not an upscaler is on (it is off for the shot),
/// and is not the shot drawn at 100 %.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_supersampled_shot_is_drawn_at_the_players_scale() {
    let Some(image) = pulse() else {
        return;
    };
    let script = format!(
        "{TO_THE_RACE} escape; until stage=menu; select photo_mode; press cross; \
         until photo=free within 200; press select select; ticks 20; \
         press cross; ticks 10; press square; ticks 3; press cross; ticks 10; \
         press circle; until photo=none within 50"
    );
    let one_fifty = all_made(&image, "ss150", &[S150, S150_FSR3], &script, 2, "hud=1");
    let two_hundred = all_made(&image, "ss200", &[S200, S200_FSR3], &script, 2, "hud=1");
    all_the_same("150", &one_fifty, 2, SAME);
    all_the_same("200", &two_hundred, 2, SAME);
    let full = made(&image, "ss100", DEFAULT, &script, 2, "hud=1");
    let redrawn = made(&image, "ss50", HALF_FSR3, &script, 2, "hud=1");
    for shot in [&one_fifty[0], &two_hundred[0]] {
        different(shot, &full, 0, 0.005);
        different(shot, &redrawn, 0, 0.005);
    }
    different(&one_fifty[0], &two_hundred[0], 0, 0.0005);
    // A re-render is drawn at the scale the shot was, not the run's.
    let footprint = hud_footprint(&one_fifty[0]);
    drawn_again("ss150", &image, &one_fifty, 1, &footprint);
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
    let shots = all_made(&image, "blur", &REDRAWN, HD_BLUR, 1, "hud=1");
    all_the_same("blur", &shots, 1, HD_DEFAULT);
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
    let shots = all_made(&image, "dof", &REDRAWN, script, 1, "dof=on");
    all_the_same("dof", &shots, 1, HD_DEFAULT);
    let picture = meta_of(&shots[0].photos[0]).picture.expect("the controls");
    assert!(picture.depth_of_field, "the shot is not a depth of field");
}

/// A redrawn shot's text says what it was drawn with, not what the player plays with.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_redrawn_shots_text_names_the_profile_it_was_drawn_with() {
    let Some(image) = pulse() else {
        return;
    };
    let script = format!(
        "{TO_THE_RACE} escape; until stage=menu; select photo_mode; press cross; \
         until photo=free within 200; press select select; ticks 20; \
         press cross; ticks 10; press circle; until photo=none within 50"
    );
    let shots = all_made(&image, "text", &[HALF_FSR3, DEFAULT], &script, 1, "hud=");
    let render = |at: usize| meta_of(&shots[at].photos[0]).render.expect("the render");
    let (redrawn, default) = (render(0), render(1));
    for key in ["render_scale", "reconstruction"] {
        assert_eq!(redrawn.get(key), default.get(key), "{key}");
    }
    assert_eq!(
        redrawn
            .get("render_scale")
            .and_then(toml::Value::as_integer),
        Some(100)
    );
}

/// Input in the frame that shows the bar of a shot that is to be redrawn at the output's size
/// cancels it: nothing is saved. The same keys with the default profile (nothing to redraw)
/// save a shot with no screenshot asked for to draw it.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn input_before_a_redrawn_shot_begins_cancels_it() {
    let Some(image) = pulse() else {
        return;
    };
    let script = format!(
        "{TO_THE_RACE} escape; until stage=menu; select photo_mode; press cross; \
         until photo=free within 200; press select select; ticks 20; \
         press cross; press cross; ticks 10; press circle; until photo=none within 50"
    );
    let shots = oag_testdata::scratch::dir("photo-native-cancel-shots");
    let scaled = drive_unmodded(&image, "native-cancel-scaled", &script, &shots, HALF_FSR3.1);
    assert!(scaled.status.success(), "{}", scaled.stderr);
    assert!(the_photos(&scaled).is_empty(), "a cancelled shot was saved");
    let default = drive_unmodded(&image, "native-cancel-default", &script, &shots, &[]);
    assert!(default.status.success(), "{}", default.stderr);
    // Each press asks for a shot (the first is drawn by the frame that follows it, with no
    // screenshot asked for, the second once that one is saved).
    assert!(
        !the_photos(&default).is_empty(),
        "the default's shot is not saved"
    );
}
