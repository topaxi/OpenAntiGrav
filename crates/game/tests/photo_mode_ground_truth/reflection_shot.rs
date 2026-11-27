//! A photo mode shot draws the glass and sea target at the original's size, whatever
//! `graphics.reflection_resolution` the race plays with, and the race's own size comes back
//! with the next frame. Each test makes a shot on one race and tick under `quarter` and under
//! `original` and compares the files, and reads the target's size off the run's state log.

use super::*;

/// HD's Time Trial on the circuit `pick` lands on to a photo mode shot (a single frame, the HUD off), then
/// out of the mode and a screenshot, which is a frame drawn with the race's own profile (a
/// driven run draws only the frames something asks for).
fn script(pick: &str) -> String {
    format!(
        "boot; select racebox; press cross; until page=race; \
         set race.mode=time_trial; select launch_race; press cross; until screen=picker; until opening_reads=landed; \
         {pick}; press cross; until screen.id=ship-select; press cross; \
         until stage=race within 4000; ticks 300; escape; until stage=menu; \
         select photo_mode; press cross; until photo=free within 200; ticks 20; press cross; \
         ticks 10; escape; expect stage=race photo=none; ticks 20; \
         screenshot SHOTS/after.png; ticks 5"
    )
}

fn shot(image: &Path, pick: &str, tag: &str, resolution: &str) -> (Run, Vec<PathBuf>) {
    let shots = oag_testdata::scratch::dir(&format!("photo-glass-{tag}-shots"));
    let _ = std::fs::remove_dir_all(&shots);
    let run = drive_unmodded(
        image,
        &format!("glass-{tag}"),
        &script(pick),
        &shots,
        &["--reflection-resolution", resolution],
    );
    assert!(run.status.success(), "{tag}: {}", run.stderr);
    for bad in ["wgpu error", "Validation Error", "panicked"] {
        assert!(!run.stderr.contains(bad), "{tag}: {bad} in\n{}", run.stderr);
    }
    let photos = the_photos(&run);
    assert_eq!(photos.len(), 1, "{tag}: {photos:?}");
    (run, photos)
}

/// The glass target's size on each state line of `run` that names one, repeats dropped.
fn sizes(run: &Run) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for state in &run.states {
        if let Some(size) = state.get("glass_target").and_then(Value::as_str)
            && seen.last().map(String::as_str) != Some(size)
        {
            seen.push(size.to_string());
        }
    }
    seen
}

/// The picture of a shot is the same under `quarter` and `original` (to the level of noise
/// every HD shot has), and the target is the original's size while the shot is drawn and the
/// race's own before and after.
fn a_photo_draws_the_target_at_the_originals_size(pick: &str, tag: &str, full: (u32, u32)) {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let (quarter, quarter_shots) = shot(&image, pick, &format!("{tag}-quarter"), "quarter");
    let (original, original_shots) = shot(&image, pick, &format!("{tag}-original"), "original");
    let text = |(w, h): (u32, u32)| format!("{w}x{h}");
    let own = text((full.0 / 4, full.1 / 4));
    assert_eq!(
        sizes(&quarter),
        [own.clone(), text(full), own],
        "{tag}: the race's size, the original's while the shot is drawn, the race's again"
    );
    assert_eq!(
        sizes(&original),
        [text(full)],
        "{tag}: an original target never changes size"
    );
    let (a, b) = (decode(&quarter_shots[0]), decode(&original_shots[0]));
    assert_eq!((a.0, a.1), (b.0, b.1));
    let widest = a.2.iter().zip(&b.2).map(|(x, y)| x.abs_diff(*y)).max();
    let share = a.2.iter().zip(&b.2).filter(|(x, y)| x != y).count() as f32 / a.2.len() as f32;
    println!("{tag}: widest {widest:?}, {share:.5} of the bytes differ");
    // One level of one channel is the noise every HD shot has (`native_shot::HD_DEFAULT`); a
    // target a quarter the size shows in the glass and the sea by tens of levels.
    assert!(
        widest.unwrap_or(0) <= 1,
        "{tag}: a shot under quarter is not the shot under original (widest {widest:?})"
    );
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn a_photo_on_vineta_k_draws_the_glass_target_at_the_originals_size() {
    a_photo_draws_the_target_at_the_originals_size("press left; ticks 10", "vineta-k", (640, 360));
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn a_photo_on_moa_therma_draws_the_sea_target_at_the_originals_size() {
    a_photo_draws_the_target_at_the_originals_size("select 11_Track", "moa-therma", (960, 540));
}
