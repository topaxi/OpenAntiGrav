//! Wipeout HD's Plasma bolt head draws a violet core inside a bright rim, on a real disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game hd_plasma_ball_ground_truth:: --run-ignored all
//! ```
//!
//! # What only a frame can say here
//!
//! `HD_plasma_ball`'s program paints `1000 rim^5` times a texture tap, where
//! `rim = 1 - N.V`: dark face-on, blown out at the silhouette
//! (`docs/rendering/hd-unlit-programs.md`). The eye in `N.V` is read out of the
//! drawable's own scene block, and a drawable nothing wrote holds `Scene::off`,
//! whose camera is the origin: the rim then came out of the direction to the
//! world's origin and the whole disc painted white. Nothing in the model, the
//! program or the cull was wrong, so only a frame tells the two apart.
//!
//! The head's two glow sprites (`WO_PLASMA_HEAD`'s templates) draw over the ball, so
//! the dark face of the program reads violet in the frame, as the original's bolt does
//! on its craft's nose (`docs/ghidra/functions/ps3-hdfury-eu/plasma.md`, "The pairs").

use std::path::Path;

const SIZE: (usize, usize) = (960, 544);

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

/// Talon's Junction's grid at tick 112, a plasma shot released at tick 100,
/// or nothing fired, with the particle stage's stream started from `stage_seed`.
fn frame_of(image: &Path, scratch: &Path, fire: bool, stage_seed: Option<u64>) -> Vec<[u8; 3]> {
    let out = scratch.join("frame.png");
    let script = scratch.join("fire.inputs");
    std::fs::write(&script, "100 none\n2 square\n1000 none\n").expect("the input script");
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"));
    command
        .arg(image)
        .args([
            "--race",
            "--no-audio",
            "--no-soundtrack",
            "--no-effects",
            "--size",
            &format!("{}x{}", SIZE.0, SIZE.1),
        ])
        .args(["--render-scale", "100", "--msaa", "off"])
        .args(["--screen-filter", "off", "--anisotropy", "off"])
        .args(["--motion-blur", "off", "--shadows", "off", "--ticks", "112"])
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"));
    if let Some(seed) = stage_seed {
        command.args(["--stage-seed", &seed.to_string()]);
    }
    if fire {
        command
            .args(["--give", "plasma", "--input-script"])
            .arg(&script);
    }
    assert!(command.status().expect("running oag-game").success());
    rgb_of(&std::fs::read(&out).expect("the frame"))
}

/// The bolt on the nose at tick 112: its inner disc, and the box round the whole ball.
/// The ball is the live object's size (`0.5625`) on the hull's `Ship Muzzle` node.
const INNER: (usize, usize, usize, usize) = (470, 266, 490, 286);
const BALL: (usize, usize, usize, usize) = (455, 250, 505, 300);

/// Pixels of `frame` inside `[x0, x1) x [y0, y1)` for which `keep` holds.
fn count(
    frame: &[[u8; 3]],
    (x0, y0, x1, y1): (usize, usize, usize, usize),
    keep: impl Fn(i32, i32, i32) -> bool,
) -> usize {
    (y0..y1)
        .flat_map(|y| (x0..x1).map(move |x| frame[y * SIZE.0 + x]))
        .filter(|&[r, g, b]| keep(i32::from(r), i32::from(g), i32::from(b)))
        .count()
}

fn violet(r: i32, g: i32, b: i32) -> bool {
    r - g > 30 && b - g > 30
}

fn white(r: i32, g: i32, b: i32) -> bool {
    r > 235 && g > 235 && b > 235
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn the_plasma_bolts_head_has_a_violet_core_not_a_white_disc() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let scratch = oag_testdata::scratch::dir("hd-plasma-ball");
    let control = frame_of(&image, &scratch, false, None);
    assert_eq!(
        count(&control, BALL, violet) + count(&control, BALL, white),
        0,
        "the control's box holds neither colour: it is the road"
    );
    // The head's glow sprites (`WO_PLASMA_HEAD`'s templates) draw over the ball from the
    // stage's random stream, so each claim holds over several arrangements of them.
    for stage_seed in STAGE_SEEDS {
        let fired = frame_of(&image, &scratch, true, stage_seed);
        let (core_violet, core_white) = (count(&fired, INNER, violet), count(&fired, INNER, white));
        let ball_white = count(&fired, BALL, white);
        println!(
            "stage seed {stage_seed:?}: inner disc {core_violet} violet, {core_white} white of 400; ball box {ball_white} white"
        );
        // Thirteen arrangements read 270 to 397 violet and 2 to 97 white of the inner
        // disc's 400. A disc shaded with the eye at the world's origin has `rim` near 1
        // all over: with the weapon scene left unwritten, five arrangements read 6 to 23
        // violet and 368 to 392 white.
        assert!(
            core_violet > 200 && core_white < 150,
            "stage seed {stage_seed:?}: the bolt's core is not violet: a white disc is the eye read from the origin"
        );
        // The head's colour tap is decoded on fetch (unit 0 gamma nibble 7,
        // `docs/rendering/hd-unlit-programs.md`, "The weapon units, read live"): the rim
        // whitens 374 to 501 pixels of the ball's box decoded over thirteen arrangements,
        // and 951 with the tap raw.
        assert!(
            ball_white < 650,
            "stage seed {stage_seed:?}: the bolt's colour tap is read raw, as the console does not: {ball_white}"
        );
    }
    std::fs::remove_dir_all(&scratch).ok();
}

/// The fixed stage seed and four others; seed 4 is the worst of thirteen read.
const STAGE_SEEDS: [Option<u64>; 5] = [None, Some(1), Some(2), Some(3), Some(4)];
