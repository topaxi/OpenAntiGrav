//! Omega's hull is lit from the circuit's spherical-harmonic probes, through the real binary:
//! the housing's dark panels lose the blue cast the constant ambient gave them.
//!
//! The original's spine panel reads `(0.071, 0.072, 0.051)` in linear light (`hull-tone.md`,
//! "The decomposition"): blue 0.7 of green. Ours drew that panel blue-neutral (blue 1.02 of
//! green) while the constant ambient `(0.31, 0.26, 0.71)` lit it, and 0.72 of green once the
//! probes do. Dropping the loader, the table, the per-craft write or the shader branch returns
//! the old ratios and turns both boxes red.
//!
//! **`#[ignore]`d**: it needs the decrypted PS4 package pair in `data/extracted/ps4/` and a GPU
//! adapter. One 1920x1080 frame of `omega/modesto-heights` at tick 100, the compare page's pair.

use std::path::PathBuf;

fn omega() -> Option<PathBuf> {
    oag_testdata::exact("data/extracted/ps4")
}

/// Mean of a box in linear light, per channel (sRGB decoded per pixel, then averaged).
fn box_linear(
    rgba: &[u8],
    width: usize,
    (x0, y0, x1, y1): (usize, usize, usize, usize),
) -> [f32; 3] {
    let decode = |byte: u8| {
        let v = f32::from(byte) / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let mut sum = [0.0f32; 3];
    for y in y0..y1 {
        for x in x0..x1 {
            for (c, total) in sum.iter_mut().enumerate() {
                *total += decode(rgba[(y * width + x) * 4 + c]);
            }
        }
    }
    let n = ((x1 - x0) * (y1 - y0)) as f32;
    sum.map(|total| total / n)
}

#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/ and a GPU adapter"]
fn the_dark_hull_panels_lose_the_constant_ambient_blue_cast() {
    let Some(source) = omega() else {
        return;
    };
    let scratch = oag_testdata::scratch::dir("omega-hull-sh-ambient");
    let out = scratch.join("shot.png");
    let profile = oag_testdata::scratch::dir("omega-sh-profile");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(&source)
        .args([
            "--race",
            "--track",
            r"Data\Environments\modesto_heights\track.vex",
        ])
        .args([
            "--ticks",
            "100",
            "--no-audio",
            "--no-soundtrack",
            "--no-effects",
            "--size",
            "1920x1080",
        ])
        .args([
            "--render-scale",
            "100",
            "--reconstruction",
            "off",
            "--msaa",
            "off",
        ])
        .args([
            "--motion-blur",
            "off",
            "--lod",
            "original",
            "--texture-detail",
            "original",
        ])
        .args([
            "--trail-order",
            "original",
            "--screen-filter",
            "off",
            "--anisotropy",
            "16x",
        ])
        .args([
            "--no-hull-shake",
            "--mode",
            "single_race",
            "--class",
            "venom",
        ])
        .args(["--team", "FEISAR_C1", "--screenshot"])
        .arg(&out)
        .env("XDG_CONFIG_HOME", profile.join("config"))
        .env("XDG_DATA_HOME", profile.join("data"))
        .env("XDG_STATE_HOME", profile.join("state"))
        .status()
        .expect("running oag-game");
    assert!(status.success(), "oag-game --race on Modesto Heights");
    let (width, height, rgba) = super::omega_menu_backdrop_ground_truth::decode_stored_png(
        &std::fs::read(&out).expect("the screenshot"),
    );
    assert_eq!((width, height), (1920, 1080));

    // The spine: engine-light independent, and the panel the original's reading is of.
    let spine = box_linear(&rgba, width, (930, 600, 990, 680));
    assert!(
        spine[2] < 0.85 * spine[1],
        "the spine's blue is 0.72 of its green with the probes, 1.02 without: {spine:?}"
    );
    // The right-hand panel under the engine light's reach, where the constant ambient was bluest
    // (blue 2.3 times green, 1.2 with the probes).
    let panel = box_linear(&rgba, width, (1000, 640, 1060, 700));
    assert!(
        panel[2] < 1.6 * panel[1],
        "blue over green on the right panel: {panel:?}"
    );
}
