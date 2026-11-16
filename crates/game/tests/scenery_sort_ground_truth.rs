//! SCENERY SORT reaches the frame a player sees: `original` is the default and the original's
//! order, and `depth` puts a nearer tree over a farther one whatever order the list draws in.
//!
//! **`#[ignore]`d and never run in CI.** It needs a Pulse disc image under `data/images/` and
//! a GPU adapter.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What only the whole game can say here
//!
//! The picture is Platinum Rush White's opening fly-by at tick 148 on a 21:9 window (the report's), where two tree strips on the
//! right of the right tower swap which is drawn over the other as the camera pans
//! (`docs/rendering/draw-order.md`, "Trees overlapping trees on Platinum Rush White"). Each
//! tree is painted its own flat colour (`OAG_DEBUG_TINT_BLENDED=tree`), and
//! `OAG_DEBUG_REVERSE_BLENDED=1` draws the circuit's blended list in the opposite order. A
//! tree over a nearer one shows as a pixel whose colour changes with the order, so:
//!
//! - `original` changes thousands of pixels when the order is reversed: the order decides which
//!   tree is on top, and that is the fault;
//! - `depth` changes none: the nearer tree is on top whichever order the list draws in.
//!
//! Dropping the foliage depth pass (`Foliage::depth`, the `foliage_depth` flag through the frame)
//! makes `depth` change as many pixels as `original`. `original` is also the default: a run
//! with no flag is byte-identical to `--scenery-sort original`.

use std::path::{Path, PathBuf};
use std::process::Command;

const SIZE: (usize, usize) = (3360, 1440);

fn scratch() -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("scenery-sort");
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// The fly-by frame at tick 148, RGBA. `sort` is `--scenery-sort`'s value or none, `tint` the
/// `OAG_DEBUG_TINT_BLENDED` unit, `reverse` the list's order.
fn shot(
    image: &Path,
    name: &str,
    sort: Option<&str>,
    tint: Option<&str>,
    reverse: bool,
) -> Vec<u8> {
    let dir = scratch();
    let out = dir.join(format!("{name}.png"));
    // The wide field of view the report was made at: more of the strips is in the picture.
    let config = dir.join("config/oag");
    std::fs::create_dir_all(&config).expect("a config directory");
    std::fs::write(config.join("settings.toml"), "[graphics]\nfov = 100\n").expect("settings");
    let mut command = Command::new(env!("CARGO_BIN_EXE_oag-game"));
    command
        .arg(image)
        .args(["--race", "--no-audio", "--size", "3360x1440"])
        .args([
            "--render-scale",
            "100",
            "--msaa",
            "off",
            "--motion-blur",
            "off",
        ])
        .args(["--screen-filter", "off", "--anisotropy", "off"])
        .args(["--track", "Data\\Environments\\13_Track\\track.vex"])
        .args(["--ticks", "0", "--intro-ticks", "148"])
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_STATE_HOME", dir.join("state"))
        .env("XDG_DATA_HOME", dir.join("data"));
    if let Some(sort) = sort {
        command.args(["--scenery-sort", sort]);
    }
    match tint {
        Some(unit) => command.env("OAG_DEBUG_TINT_BLENDED", unit),
        None => command.env_remove("OAG_DEBUG_TINT_BLENDED"),
    };
    if reverse {
        command.env("OAG_DEBUG_REVERSE_BLENDED", "1");
    } else {
        command.env_remove("OAG_DEBUG_REVERSE_BLENDED");
    }
    let run = command.output().expect("running oag-game");
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let file = std::fs::File::open(&out).expect("the screenshot");
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .expect("a PNG");
    let mut pixels = vec![0; reader.output_buffer_size().expect("a buffer size")];
    let info = reader.next_frame(&mut pixels).expect("a frame");
    assert_eq!((info.width as usize, info.height as usize), SIZE);
    assert_eq!(info.color_type, png::ColorType::Rgba);
    pixels
}

/// The hue in degrees of a pixel that is one of the debug tint's flat colours (full saturation
/// and value), or `None` for the scene's own pixels.
fn tint_hue(pixel: &[u8]) -> Option<f32> {
    let [r, g, b] = [pixel[0], pixel[1], pixel[2]].map(|c| f32::from(c) / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let delta = max - min;
    if max < 0.75 || delta / max < 0.75 {
        return None;
    }
    let hue = if max == r {
        ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };
    Some(hue * 60.0)
}

/// How many pixels are one tree's colour in one frame and another tree's in the other.
fn recoloured(a: &[u8], b: &[u8]) -> usize {
    a.as_chunks::<4>()
        .0
        .iter()
        .zip(b.as_chunks::<4>().0)
        .filter(|(a, b)| match (tint_hue(*a), tint_hue(*b)) {
            (Some(x), Some(y)) => {
                let d = (x - y).abs();
                d.min(360.0 - d) > 40.0
            }
            _ => false,
        })
        .count()
}

/// With no flag the frame is `--scenery-sort original`'s, and reversing the list is off unless
/// asked: the default path is untouched.
#[test]
#[ignore = "needs a Pulse disc image in data/images/ and a GPU adapter"]
fn original_is_the_default() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let plain = shot(&image, "plain", None, None, false);
    let original = shot(&image, "original", Some("original"), None, false);
    assert!(plain == original, "no flag is `original`");
    let depth = shot(&image, "depth", Some("depth"), None, false);
    assert!(plain != depth, "`depth` reaches the frame");
}

/// The order decides which tree is on top under `original` and nothing under `depth`.
#[test]
#[ignore = "needs a Pulse disc image in data/images/ and a GPU adapter"]
fn depth_puts_the_nearer_tree_on_top_whichever_order_the_list_draws_in() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let tint = Some("tree");
    let original = recoloured(
        &shot(&image, "o-fwd", Some("original"), tint, false),
        &shot(&image, "o-rev", Some("original"), tint, true),
    );
    let depth = recoloured(
        &shot(&image, "d-fwd", Some("depth"), tint, false),
        &shot(&image, "d-rev", Some("depth"), tint, true),
    );
    assert!(
        original > 1_000,
        "`original`: reversing the list recoloured only {original} pixels"
    );
    assert!(
        depth < 50,
        "`depth`: reversing the list recoloured {depth} pixels"
    );
}
