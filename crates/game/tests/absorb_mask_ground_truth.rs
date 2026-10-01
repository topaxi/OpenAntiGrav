//! A real absorb's hull overlay leaves the bloom's mask almost alone, on a real disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test absorb_mask_ground_truth --run-ignored all
//! ```
//!
//! # What only real data can say here
//!
//! The overlay's own stencil is `REPLACE 0xff`, and a renderer that kept it
//! would mark every hull pixel a full glow and bloom into a white blob. The
//! original does not: read at a frame boundary on a PPSSPP running the
//! software renderer (`docs/rendering/glow-mask.md`, "The hull overlay's mask
//! is wiped"), the hull holds the neutral `4` all through the one-second
//! window, bar the one glow batch (`glowingShape`'s `colours_flashing_GLOW`,
//! drawn after the shadow pass's stencil reset) and the HUD. So the number of
//! `0xff` pixels over the hull grows by about 100 in the original between a
//! frame before the absorb and one at its peak (95 to 123 over two boots, in
//! this test's own box) - and by about 4,000 in a renderer that stamps the
//! whole overlay.

use std::path::Path;

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

/// The mask of the Talon's Junction grid at `ticks`, absorbing a pickup at tick 700
/// or doing nothing.
fn mask_of(image: &Path, scratch: &Path, absorb: bool, ticks: u32) -> Vec<u8> {
    let out = scratch.join("mask.png");
    let script = scratch.join("absorb.inputs");
    std::fs::write(&script, "700 none\n1 circle\n200 none\n").expect("the input script");
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"));
    command
        .arg(image)
        .args(["--race", "--mode", "single_race", "--team", "Assegai"])
        .args(["--class", "venom", "--no-audio", "--size", "480x272"])
        .args(["--track", r"Data\Environments\16_Track\track.vex"])
        .args(["--render-scale", "100", "--msaa", "off"])
        .args(["--screen-filter", "off", "--anisotropy", "off"])
        .args(["--motion-blur", "off", "--ticks", &ticks.to_string()])
        .arg("--screenshot")
        .arg(scratch.join("frame.png"))
        .env("OAG_DUMP_GLOW_MASK", &out)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"));
    if absorb {
        command
            .args(["--give", "mine", "--input-script"])
            .arg(&script);
    }
    assert!(command.status().expect("running oag-game").success());
    grey_of(&std::fs::read(&out).expect("the mask"), 480, 272)
}

/// The pixels of `mask` at `value` inside the hull's neighbourhood: the
/// middle of the lower frame, clear of the speed read-out and the shield bar.
fn hull_count(mask: &[u8], value: u8) -> usize {
    (110..255)
        .flat_map(|y| (150..290).map(move |x| (x, y)))
        .filter(|&(x, y)| mask[y * 480 + x] == value)
        .count()
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and a GPU adapter"]
fn a_real_absorbs_overlay_does_not_flood_the_hull_with_glow() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let scratch = std::env::temp_dir().join(format!("oag-absorb-mask-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("creating the scratch directory");
    // 35 ticks after the press is just past a half second: the peak.
    let peak = mask_of(&image, &scratch, true, 736);
    let control = mask_of(&image, &scratch, false, 736);
    std::fs::remove_dir_all(&scratch).ok();

    let (with, without) = (hull_count(&peak, 0xff), hull_count(&control, 0xff));
    println!("hull neighbourhood at 0xff: {with} at the absorb's peak, {without} without");
    // The original adds about 100 and ours 113; a renderer that keeps the overlay's stamp
    // adds about 4,000.
    assert!(
        with <= without + 400,
        "{with} full-glow pixels over the hull against {without} without an absorb: \
         the overlay's stencil survived the shadow pass's reset"
    );
}
