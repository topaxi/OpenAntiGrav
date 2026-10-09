//! Vineta K's tunnel glass shows the behind-the-glass target, on a real disc:
//! the panes over the track are the original's dark teal.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test hd_behind_glass_frame_ground_truth --run-ignored all
//! ```
//!
//! The camera is the original's own at pose A of the RPCS3 capture the target
//! was measured on (`docs/formats/rcsmaterial.md`, "The behind-the-glass
//! target is drawn"): the main view's matrix read out of the frame's command
//! stream. There the right ceiling panes are teal, median `(4, 87, 87)`; drawn
//! over our own frame they were lime and cyan, `(135, 176, 57)`, and drawn
//! with no target at all they would be the glass's own dark lit colour. The
//! threshold sits between the measured counts, which are in the test.

const SIZE: (usize, usize) = (960, 544);

/// The capture's main-view camera at pose A: eye, forward, up.
const POSE_A: &str =
    "-840.8534,-143.7014,202.7764,-0.015665,-0.105703,0.994274,-0.128137,0.986409,0.102848";

/// Every pixel of a PNG this project's own writer made, as `[r, g, b]` - the
/// writer stores its deflate blocks uncompressed and filters nothing
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

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn the_ceiling_panes_show_the_teal_behind_the_glass() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let scratch = std::env::temp_dir().join(format!("oag-hd-behind-glass-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("creating the scratch directory");
    let out = scratch.join("frame.png");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(image)
        .args(["--race", "--no-audio", "--size", "960x544"])
        .args(["--track", r"Data\Environments\01_Vineta_K\track.vex"])
        .args(["--team", "feisar_c1", "--variant", "concept1"])
        .args(["--render-scale", "100", "--msaa", "off"])
        .args([
            "--screen-filter",
            "off",
            "--anisotropy",
            "off",
            "--motion-blur",
            "off",
        ])
        .arg(format!("--camera-pose={POSE_A}"))
        .args(["--camera-fov", "60", "--screenshot"])
        .arg(&out)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"))
        .status()
        .expect("running oag-game");
    assert!(status.success());
    let frame = rgb_of(&std::fs::read(&out).expect("the frame"));
    std::fs::remove_dir_all(&scratch).ok();
    // The right ceiling, the capture's `(1250, 120)-(1880, 380)` at this size.
    // Windows scaled by 0.73 (the encoded value of one half, 2026-10-09): HD's lit programs end on a /2 output scale, so every lit surface reads half the light it did.
    let teal = (62..194)
        .flat_map(|y| (640..958).map(move |x| (x, y)))
        .map(|(x, y)| frame[y * SIZE.0 + x])
        .filter(|&[r, g, b]| r < 33 && (33..102).contains(&g) && (33..102).contains(&b))
        .filter(|&[_, g, b]| g.abs_diff(b) < 15)
        .count();
    // Teal pixels of 41,976: 33,430 with the target, 3,387 with the glass
    // drawn over our own frame (two blended passes, before 2026-10-07), 116
    // with the target left unbound (the black placeholder).
    assert!(teal > 20_000, "{teal} teal pane pixels");
}
