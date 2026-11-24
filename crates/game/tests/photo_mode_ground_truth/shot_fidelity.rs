//! What a saved photo mode shot records of its picture controls and how its HUD is drawn: checks
//! over the runs `photo_mode_ground_truth` already makes, so no race is played for them. The game
//! binary is handed in by that module, which is the one the test groups name.

use super::*;

pub(super) fn meta_of(photo: &Path) -> oag_game::shot_meta::Meta {
    let bytes = std::fs::read(photo).expect("the photo");
    oag_game::shot_meta::read(&bytes)
        .expect("our chunks read")
        .meta
        .expect("the text chunk")
}

/// `oag-game --from-png` on `photo` in the profile `run` made it in: its stdout and the picture
/// it drew.
pub(super) fn from_png(
    binary: &str,
    image: &Path,
    run: &Run,
    photo: &Path,
    out: &Path,
    flags: &[&str],
) -> (String, (u32, u32, Vec<u8>)) {
    let output = std::process::Command::new(binary)
        .arg("--from-png")
        .arg(photo)
        .arg("--screenshot")
        .arg(out)
        .arg(image)
        .arg("--no-audio")
        .args(flags)
        .env("XDG_CONFIG_HOME", run.dir.join("config"))
        .env("XDG_DATA_HOME", run.dir.join("data"))
        .env("XDG_STATE_HOME", run.dir.join("state"))
        .env("OAG_TRACKS", run.dir.join("no-tracks"))
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("DISPLAY")
        .output()
        .expect("running oag-game --from-png");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        output.status.success(),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    (stdout, decode(out))
}

/// Each control a shot was made with is in its text, a shot at neutral writes none, and
/// `--from-png` draws the controls again: a brighter exposure stays brighter in the re-render.
pub(super) fn controls_are_recorded_and_drawn_again(
    binary: &str,
    image: &Path,
    run: &Run,
    photos: &[PathBuf],
) {
    let picture = |at: usize| {
        meta_of(&photos[at])
            .picture
            .expect("a photo mode shot records its controls")
    };
    for (name, at) in [
        ("neutral", 0),
        ("exposure put back", 2),
        ("saturation put back", 4),
    ] {
        assert!(
            picture(at).effects().is_neutral(),
            "{name}: the opening values are recorded as they are"
        );
    }
    let bright = picture(1);
    assert!(bright.exposure > 33 && bright.saturation == 20 && !bright.depth_of_field);
    assert_eq!(picture(3).saturation, 0);
    assert!(picture(5).speed_blur > 0);
    let dof = picture(7);
    assert!(dof.depth_of_field && dof.aperture.is_some() && dof.focus.is_some());

    let out = run.dir.join("rerender-bright.png");
    let (stdout, again) = from_png(binary, image, run, &photos[1], &out, &[]);
    assert!(stdout.contains("MATCH"), "{stdout}");
    assert!(stdout.contains("picture controls applied"), "{stdout}");
    let (neutral, bright) = (decode(&photos[0]), decode(&photos[1]));
    let (plain, graded, redrawn) = (
        over(&neutral, ALL, luma),
        over(&bright, ALL, luma),
        over(&again, ALL, luma),
    );
    println!("mean luma: neutral {plain:.1}, shot {graded:.1}, re-render {redrawn:.1}");
    assert!(
        (redrawn - graded).abs() < 0.5 * (graded - plain),
        "the re-render did not draw the exposure: {plain} {graded} {redrawn}"
    );
}

/// `mask` grown by `radius` pixels in each direction.
pub(super) fn grow(mask: &[bool], (width, height): (usize, usize), radius: usize) -> Vec<bool> {
    let mut across = vec![false; mask.len()];
    for y in 0..height {
        for x in 0..width {
            across[y * width + x] = (x.saturating_sub(radius)..(x + radius + 1).min(width))
                .any(|nx| mask[y * width + nx]);
        }
    }
    let mut grown = vec![false; mask.len()];
    for y in 0..height {
        for x in 0..width {
            grown[y * width + x] = (y.saturating_sub(radius)..(y + radius + 1).min(height))
                .any(|ny| across[ny * width + x]);
        }
    }
    grown
}

pub(super) type Picture = (u32, u32, Vec<u8>);

/// The HUD on a shot of several frames is the HUD of the shot's own tick, drawn once. `plain`
/// and `plain_hud` are the one frame without and with the HUD, `blurred` and `blurred_hud` the
/// shot of several frames. The HUD's glyphs (where the HUD changes the plain shot most) are the
/// same in the blurred shot, and no high-contrast pixel outside the HUD differs from the blurred
/// shot without one: a message that was not on screen, a ghosted digit. The craft's smear and
/// its animation differ between shots, so the region where the blurred shot moves the scene is
/// left out of both.
pub(super) fn the_hud_is_drawn_once(
    plain: &Picture,
    plain_hud: &Picture,
    blurred: &Picture,
    blurred_hud: &Picture,
) {
    let size = (plain.0 as usize, plain.1 as usize);
    let apart = |a: &Picture, b: &Picture, at: usize| {
        (0..3)
            .map(|c| a.2[at * 4 + c].abs_diff(b.2[at * 4 + c]))
            .max()
            .unwrap_or(0)
    };
    let pixels = size.0 * size.1;
    let moved = grow(
        &(0..pixels)
            .map(|at| apart(blurred, plain, at) > 16)
            .collect::<Vec<_>>(),
        size,
        4,
    );
    let footprint = grow(
        &(0..pixels)
            .map(|at| apart(plain_hud, plain, at) > 2)
            .collect::<Vec<_>>(),
        size,
        3,
    );
    let (mut cores, mut kept, mut stray) = (0u32, 0u32, 0u32);
    for at in 0..pixels {
        if moved[at] {
            continue;
        }
        if apart(plain_hud, plain, at) > 120 {
            cores += 1;
            kept += u32::from(apart(blurred_hud, plain_hud, at) <= 8);
        }
        if !footprint[at] {
            stray += u32::from(apart(blurred_hud, blurred, at) > 40);
        }
    }
    println!("HUD glyph pixels {cores}, kept in the blurred shot {kept}; stray {stray}");
    assert!(
        cores > 5000,
        "the HUD row put no glyphs on the plain shot: {cores}"
    );
    assert_eq!(
        kept, cores,
        "the blurred shot's HUD is not the plain shot's"
    );
    assert_eq!(
        stray, 0,
        "pixels outside the HUD differ from the blurred shot without one"
    );
}
