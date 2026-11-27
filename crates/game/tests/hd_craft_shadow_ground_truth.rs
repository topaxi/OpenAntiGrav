//! A craft's shadow on Wipeout HD's road: it is there under `mapped`, and under `original`
//! its edge is as sharp as the shadow resolution says, on a real disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game hd_craft_shadow_ground_truth:: --run-ignored all
//! ```
//!
//! # What only a frame can say here
//!
//! A shadow map is read by a sampler inside another shader, so a caster that draws and a
//! receiver that projects can both be right and the road still be lit. The frame is Talon's
//! Junction's grid from the camera of an RPCS3 capture of the original, where the player's
//! craft throws its shadow on the road beside it (`docs/rendering/shadows.md`, "Shadow
//! resolution", which carries the original's edge width from that capture).

use std::path::Path;

const SIZE: (usize, usize) = (1280, 720);

/// Every pixel of a PNG this project's own writer made, as `[r, g, b]`.
///
/// That writer stores its deflate blocks uncompressed and filters nothing
/// (`oag_texture::png`), so a reader needs only the chunk and block framing.
fn rgb_of(png: &[u8]) -> Vec<[u8; 3]> {
    let mut at = 8;
    let mut zlib = Vec::new();
    while at + 8 <= png.len() {
        let len = u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
        if &png[at + 4..at + 8] == b"IDAT" {
            zlib.extend_from_slice(&png[at + 8..at + 8 + len]);
        }
        at += 12 + len;
    }
    let mut raw = Vec::new();
    let mut at = 2;
    loop {
        let last = zlib[at] & 1 == 1;
        let len = usize::from(u16::from_le_bytes([zlib[at + 1], zlib[at + 2]]));
        raw.extend_from_slice(&zlib[at + 5..at + 5 + len]);
        at += 5 + len;
        if last {
            break;
        }
    }
    let stride = 1 + SIZE.0 * 4;
    assert_eq!(
        raw.len(),
        stride * SIZE.1,
        "an RGBA PNG of the frame's size"
    );
    raw.chunks_exact(stride)
        .flat_map(|row| {
            row[1..]
                .as_chunks::<4>()
                .0
                .iter()
                .map(|p| [p[0], p[1], p[2]])
        })
        .collect()
}

/// Talon's Junction's grid at tick 0 from the original capture's camera, under `shadows` at
/// `resolution`.
fn frame_of(image: &Path, scratch: &Path, shadows: &str, resolution: &str) -> Vec<[u8; 3]> {
    let out = scratch.join("frame.png");
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"));
    command
        .arg(image)
        .args([
            "--race",
            "--opponents",
            "--no-audio",
            "--no-soundtrack",
            "--no-effects",
            "--team",
            "FEISAR",
        ])
        .args(["--track", "Data\\Environments\\Talons_Junction\\track.vex"])
        .arg(
            "--camera-pose=-143.567421,-48.459354,-175.132351,0.994815,-0.101594,-0.004697,\
             0.101551,0.994792,-0.008692",
        )
        .args(["--camera-fov", "60.0001"])
        .args(["--size", &format!("{}x{}", SIZE.0, SIZE.1)])
        .args([
            "--render-scale",
            "100",
            "--msaa",
            "off",
            "--reconstruction",
            "off",
        ])
        .args(["--screen-filter", "off", "--anisotropy", "off"])
        .args(["--motion-blur", "off", "--ticks", "0"])
        .args(["--shadows", shadows, "--shadow-resolution", resolution])
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"));
    assert!(command.status().expect("running oag-game").success());
    rgb_of(&std::fs::read(&out).expect("the frame"))
}

fn luminance(pixel: [u8; 3]) -> f32 {
    0.299 * f32::from(pixel[0]) + 0.587 * f32::from(pixel[1]) + 0.114 * f32::from(pixel[2])
}

/// How much of the lit road's light `frame` takes away at a pixel, `0.0` to `1.0`.
fn coverage(frame: &[[u8; 3]], off: &[[u8; 3]], x: usize, y: usize) -> f32 {
    let at = y * SIZE.0 + x;
    1.0 - luminance(frame[at]) / luminance(off[at]).max(1.0)
}

/// The road inside the player's shadow, beside the hull.
const INSIDE: (usize, usize, usize, usize) = (600, 598, 650, 610);

/// The scanlines the shadow's left edge crosses, and the columns it crosses them in.
const EDGE: (usize, usize, usize, usize) = (578, 598, 596, 626);

fn mean_coverage(frame: &[[u8; 3]], off: &[[u8; 3]]) -> f32 {
    let (x0, y0, x1, y1) = INSIDE;
    let mut sum = 0.0;
    for y in y0..y1 {
        for x in x0..x1 {
            sum += coverage(frame, off, x, y);
        }
    }
    sum / ((x1 - x0) * (y1 - y0)) as f32
}

/// The shadow edge's width in pixels: per scanline, `0.8 * contrast / steepest step`, which
/// is a linear ramp's 10 to 90 % width; the median over the scanlines.
fn edge_width(frame: &[[u8; 3]], off: &[[u8; 3]]) -> f32 {
    let (x0, y0, x1, y1) = EDGE;
    let mut widths = Vec::new();
    for y in y0..y1 {
        let row: Vec<f32> = (x0 - 10..x1 + 10)
            .map(|x| coverage(frame, off, x, y))
            .collect();
        let (mut steepest, mut at) = (0.0f32, 10);
        for k in 10..row.len() - 11 {
            let step = (row[k + 1] - row[k]).abs();
            if step > steepest {
                (steepest, at) = (step, k);
            }
        }
        if steepest <= 0.0 {
            continue;
        }
        let median = |values: &[f32]| {
            let mut sorted = values.to_vec();
            sorted.sort_by(f32::total_cmp);
            sorted[sorted.len() / 2]
        };
        let contrast = (median(&row[at + 5..at + 10]) - median(&row[at - 8..at - 3])).abs();
        widths.push(0.8 * contrast / steepest);
    }
    widths.sort_by(f32::total_cmp);
    widths[widths.len() / 2]
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn a_crafts_shadow_reaches_the_road_and_sharpens_with_the_resolution() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let scratch = oag_testdata::scratch::dir("hd-craft-shadow");
    let off = frame_of(&image, &scratch, "off", "original");
    let mapped = frame_of(&image, &scratch, "mapped", "original");
    let original = frame_of(&image, &scratch, "original", "original");
    let ultra = frame_of(&image, &scratch, "original", "ultra");
    std::fs::remove_dir_all(&scratch).ok();

    let (mapped_cover, original_cover) =
        (mean_coverage(&mapped, &off), mean_coverage(&original, &off));
    let (width, ultra_width) = (edge_width(&original, &off), edge_width(&ultra, &off));
    println!(
        "road under the craft: mapped {mapped_cover:.3}, original {original_cover:.3}; \
         edge width original {width:.2} px, ultra {ultra_width:.2} px"
    );
    // 0.22 under `mapped` and 0.30 under `original`. A caster that draws into a map the road
    // never reads leaves both at zero.
    assert!(
        mapped_cover > 0.1,
        "under `mapped` the craft's own shadow is not on the road: {mapped_cover}"
    );
    assert!(
        original_cover > 0.15,
        "under `original` the craft's shadow is not on the road: {original_cover}"
    );
    // 1.89 px at the default against the original's 1.72 in its own capture of this view;
    // the original's 512-texel side on this renderer's 24-unit box reads 2.29.
    assert!(
        width < 2.1,
        "the default's edge is wider than the original's: {width} px"
    );
    // 1.22 px.
    assert!(
        ultra_width < width - 0.3,
        "`ultra` is no sharper than the default: {ultra_width} against {width} px"
    );
}
