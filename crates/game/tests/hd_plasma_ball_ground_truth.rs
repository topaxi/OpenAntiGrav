//! Wipeout HD's Plasma bolt head draws a dark core inside a bright rim, on a real disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test hd_plasma_ball_ground_truth --run-ignored all
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

use std::path::Path;

const SIZE: (usize, usize) = (960, 544);

/// The red channel of every pixel of a PNG this project's own writer made.
///
/// That writer stores its deflate blocks uncompressed and filters nothing
/// (`oag_texture::png`), so a reader needs only the chunk and block framing.
fn red_of(png: &[u8]) -> Vec<u8> {
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
        .flat_map(|row| row[1..].as_chunks::<4>().0.iter().map(|p| p[0]))
        .collect()
}

/// Talon's Junction's grid at tick 112, a plasma shot released at tick 100,
/// or nothing fired.
fn frame_of(image: &Path, scratch: &Path, fire: bool) -> Vec<u8> {
    let out = scratch.join("frame.png");
    let script = scratch.join("fire.inputs");
    std::fs::write(&script, "100 none\n2 square\n1000 none\n").expect("the input script");
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"));
    command
        .arg(image)
        .args([
            "--race",
            "--no-audio",
            "--size",
            &format!("{}x{}", SIZE.0, SIZE.1),
        ])
        .args(["--render-scale", "100", "--msaa", "off"])
        .args(["--screen-filter", "off", "--anisotropy", "off"])
        .args(["--motion-blur", "off", "--ticks", "112"])
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"));
    if fire {
        command
            .args(["--give", "plasma", "--input-script"])
            .arg(&script);
    }
    assert!(command.status().expect("running oag-game").success());
    red_of(&std::fs::read(&out).expect("the frame"))
}

/// Pixels dark enough to be the bolt's core, in the box the bolt sits in
/// ahead of the nose: the middle of the frame, above the cockpit.
fn dark_core(frame: &[u8]) -> usize {
    (230..330)
        .flat_map(|y| (420..540).map(move |x| (x, y)))
        // Was `< 40`; HD's lit programs end on a /2 output scale (2026-10-09), so the
        // road and the bolt's core both read darker, and 24 keeps the control
        // under 50 (18) with the bolt over 200 (275).
        .filter(|&(x, y)| frame[y * SIZE.0 + x] < 24)
        .count()
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn the_plasma_bolts_head_has_a_dark_core_not_a_white_disc() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let scratch = std::env::temp_dir().join(format!("oag-hd-plasma-ball-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("creating the scratch directory");
    let fired = frame_of(&image, &scratch, true);
    let control = frame_of(&image, &scratch, false);
    std::fs::remove_dir_all(&scratch).ok();

    let (with, without) = (dark_core(&fired), dark_core(&control));
    println!("dark pixels in the bolt's box: {with} with a bolt, {without} without");
    // With both the halved lit output and the encoded-mean adaptation
    // (2026-10-09) the bolt counts 190 and the control 16: the core is a
    // gradient, so the `< 24` cut takes less of it, and a white disc would
    // count about what the control does. The floor keeps the two far apart.
    assert!(without < 100, "the control's box is not road any more");
    assert!(
        with > 120,
        "the bolt's core is not dark: a white disc is the eye read from the origin"
    );
}
