//! Wipeout HD's LeachBall glows face-on from where the camera really is, on a real disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test hd_leach_ball_ground_truth --run-ignored all
//! ```
//!
//! # What only a frame can say here
//!
//! The ball's `RIM_GLOW` program paints `a = (0.9 (1 - rim^5))^5` where `rim` is
//! one minus `N.V`, brightest face-on (`docs/rendering/hd-unlit-programs.md`). The eye
//! in `N.V` is read out of the drawable's own scene block, and a drawable
//! nothing wrote holds `Scene::off`, whose camera is the world's origin: the
//! ball was then shaded from the direction to the origin, a ragged half-lit
//! disc, and it carried about 40 % of the bright pixels the right eye gives.
//! Nothing in the model or the program was wrong, so only a close frame tells
//! the two apart. On Talon's Junction at that camera: 1,186 near-white pixels in
//! the ball's box with the write, 369 with it dropped, 0 with no ball.

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

/// Talon's Junction with rivals, tick 280: a locked LeachBeam put on slot 1 at
/// tick 276, photographed from a camera 7 units off the ball (which sits at
/// about `(-0.6, -51.9, -194.8)` then), or the same frame with no beam. The camera sits 1.846
/// lower than it did (`y = -47.04`): HD's grid craft hover that much lower since the grid hover
/// landed, rivals included.
///
/// **Four ticks after the green light and no input**, so slot 1 has barely
/// left its grid mark and the ball, which starts at the target, is where the
/// camera was pointed whatever the rival AI does in the seconds after. A
/// later tick would make this test a function of the AI's driving.
fn frame_of(image: &Path, scratch: &Path, locked: bool) -> Vec<u8> {
    let out = scratch.join("frame.png");
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"));
    command
        .arg(image)
        .args(["--race", "--opponents", "--no-audio", "--size", "960x544"])
        .args(["--render-scale", "100", "--msaa", "off"])
        .args(["--screen-filter", "off", "--anisotropy", "off"])
        .args(["--motion-blur", "off", "--ticks", "280"])
        .arg("--camera-pose=-0.58,-48.886,-188.2,0,-0.45,-0.89,0,1,0")
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

/// Near-white pixels in the box the ball sits in at that camera.
fn white(frame: &[u8]) -> usize {
    (235..300)
        .flat_map(|y| (440..520).map(move |x| (x, y)))
        .filter(|&(x, y)| frame[y * SIZE.0 + x] > 240)
        .count()
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn the_leach_ball_glows_face_on_from_the_real_eye() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let scratch = std::env::temp_dir().join(format!("oag-hd-leach-ball-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("creating the scratch directory");
    let locked = frame_of(&image, &scratch, true);
    let control = frame_of(&image, &scratch, false);
    std::fs::remove_dir_all(&scratch).ok();

    let (with, without) = (white(&locked), white(&control));
    println!("near-white pixels in the ball's box: {with} with a ball, {without} without");
    assert!(without < 400, "the control's box is not road any more");
    assert!(
        with > without + 800,
        "the ball is not glowing face-on: its eye is the world's origin again"
    );
}
