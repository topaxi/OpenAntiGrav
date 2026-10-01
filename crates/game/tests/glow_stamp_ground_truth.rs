//! A blended batch with the glow bits stamps the bloom's mask, on a real disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test glow_stamp_ground_truth --run-ignored all
//! ```
//!
//! # What only real data can say here
//!
//! A GE dump and an EDRAM read of Outpost 7 on a PPSSPP running the original
//! (`docs/rendering/glow-mask.md`, "Transparent batches stamp") show the glow
//! mask holding `0xfa` along the tunnel's arch lights and `0xaf` along the
//! start-line laser: the stencil reference of two blended batches, each its
//! texture's own glow byte (`07_Pulse_light_BLEND_GLOW` and
//! `startline_laser_ADD_GLOW`). Nothing else in that frame writes either
//! value, so a frame whose mask holds thousands of them has stamped through
//! the blended draw, and a renderer that drops the stamp - or the loader's
//! per-batch byte - holds none.
//!
//! The circuit is `track_reversed.vex`, the one the original loads for the
//! Black variant; `track.vex` carries different geometry.

use std::path::Path;

/// `07_Pulse_light_BLEND_GLOW`'s glow byte: the arch lights' alpha-over batches.
const ARCH_LIGHTS: u8 = 0xfa;
/// `startline_laser_ADD_GLOW`'s: the additive laser across the grid.
const START_LASER: u8 = 0xaf;

/// The greyscale values of a PNG this project's own writer made.
///
/// That writer stores its deflate blocks uncompressed and filters nothing
/// (`oag_texture::png`), so the pixels are in the file as they were written
/// and a reader needs only the chunk and block framing - not a decoder.
fn grey_of(png: &[u8], width: usize, height: usize) -> Vec<u8> {
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
    let stride = 1 + width * 4;
    assert_eq!(
        raw.len(),
        stride * height,
        "an RGBA PNG of the frame's size"
    );
    raw.chunks_exact(stride)
        .flat_map(|row| row[1..].as_chunks::<4>().0.iter().map(|p| p[0]))
        .collect()
}

fn mask_of(image: &Path, scratch: &Path) -> Vec<u8> {
    let out = scratch.join("mask.png");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(image)
        .args(["--race", "--no-audio", "--size", "480x272"])
        .args(["--render-scale", "100", "--msaa", "off"])
        .args(["--screen-filter", "off", "--anisotropy", "off"])
        .args(["--motion-blur", "off", "--ticks", "1"])
        .args(["--track", r"Data\Environments\07_Track\track_reversed.vex"])
        .arg("--screenshot")
        .arg(scratch.join("frame.png"))
        .env("OAG_DUMP_GLOW_MASK", &out)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"))
        .status()
        .expect("running oag-game");
    assert!(status.success(), "oag-game");
    grey_of(&std::fs::read(&out).expect("the mask"), 480, 272)
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and a GPU adapter"]
fn the_grid_frame_of_outpost_7_stamps_its_blended_glow_batches() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let scratch = std::env::temp_dir().join(format!("oag-glow-stamp-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("creating the scratch directory");
    let mask = mask_of(&image, &scratch);
    std::fs::remove_dir_all(&scratch).ok();

    let count = |value: u8| mask.iter().filter(|&&v| v == value).count();
    let (arches, laser) = (count(ARCH_LIGHTS), count(START_LASER));
    println!("mask: {arches} pixels at {ARCH_LIGHTS:#x}, {laser} at {START_LASER:#x}");
    // Ours at this default slot holds about 2,600 and 1,100 pixels; the original's
    // Black grid, a different slot, 4,076 and 1,469. A slot sees a different share
    // of each, so this asks for a third to a half of the original's - far more than
    // anything but the stamp can write.
    assert!(
        arches >= 900,
        "{arches} arch-light pixels in the mask: a blended glow batch did not stamp"
    );
    assert!(
        laser >= 500,
        "{laser} start-laser pixels in the mask: an additive glow batch did not stamp"
    );
}
