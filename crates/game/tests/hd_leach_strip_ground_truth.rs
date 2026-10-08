//! Wipeout HD's LeachBeam strip is drawn from the craft's anchor trail, on a real disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test hd_leach_strip_ground_truth --run-ignored all
//! ```
//!
//! Talon's Junction with rivals, a locked beam on slot 1 from tick 276, seen from
//! the side at tick 330 (`docs/ghidra/functions/ps3-hdfury-eu/leach-beam-strips.md`,
//! "hd-leach-draw"). The strip is a bright glow ribbon from the player's hull toward
//! the rival: 9,332 near-white pixels in the box its near end crosses against 46 with
//! no beam. Dropping the strip, its textures or its anchor trail empties the box.

use std::path::Path;

const SIZE: (usize, usize) = (960, 544);

/// The smallest channel of every pixel of a PNG this project's own writer made.
///
/// That writer stores its deflate blocks uncompressed and filters nothing
/// (`oag_texture::png`), so a reader needs only the chunk and block framing.
fn floor_of(png: &[u8]) -> Vec<u8> {
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
                .map(|p| p[0].min(p[1]).min(p[2]))
        })
        .collect()
}

fn frame_of(image: &Path, scratch: &Path, locked: bool) -> Vec<u8> {
    let out = scratch.join("frame.png");
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"));
    command
        .arg(image)
        .args(["--race", "--opponents", "--no-audio", "--size", "960x544"])
        .args(["--render-scale", "100", "--msaa", "off"])
        .args(["--screen-filter", "off", "--anisotropy", "off"])
        .args(["--motion-blur", "off", "--ticks", "330"])
        .arg("--camera-pose=-112,-47,-162,0.89,-0.07,-0.45,0,1,0")
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"));
    if locked {
        command.args(["--force-leach-lock", "276:1"]);
    }
    assert!(command.status().expect("running oag-game").success());
    floor_of(&std::fs::read(&out).expect("the frame"))
}

/// Near-white pixels in the box the strip's near end crosses.
fn white(frame: &[u8]) -> usize {
    (250..420)
        .flat_map(|y| (0..450).map(move |x| (x, y)))
        .filter(|&(x, y)| frame[y * SIZE.0 + x] > 200)
        .count()
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn the_leach_strip_is_drawn_from_the_players_hull_toward_the_rival() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let scratch = std::env::temp_dir().join(format!("oag-hd-leach-strip-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("creating the scratch directory");
    let locked = frame_of(&image, &scratch, true);
    let control = frame_of(&image, &scratch, false);
    std::fs::remove_dir_all(&scratch).ok();

    let (with, without) = (white(&locked), white(&control));
    println!("near-white pixels in the strip's box: {with} with a beam, {without} without");
    assert!(without < 400, "the control's box is not road any more");
    assert!(with > without + 3000, "the strip is not drawn");
}
