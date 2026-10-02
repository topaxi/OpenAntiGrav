//! Pulse PS2's glow mask, on a real disc: graded, and zero everywhere nothing glows.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test ps2_glow_mask_ground_truth --run-ignored all
//! ```
//!
//! # What only real data can say here
//!
//! Replayed GS dumps of the PS2 original on Moa Therma's grid
//! (`docs/rendering/ps2-bloom.md`, "The mask, measured") put the frame's alpha
//! at **zero on 96.7 % of the picture** and **graded** over the rest - 222
//! distinct levels, up to `0xfd` - because a glow batch writes its fragments'
//! own alpha, texel times vertex colour. The PSP's rule is the opposite on both
//! counts: its opaque batches stamp a floor of `4` over the whole frame and a
//! batch stamps one constant. So a frame whose mask is mostly zero and holds
//! over a hundred levels is the PS2's rule, and a renderer that drops the
//! PS2 branch - the loader's `glow_by_texel`, the `GLOW_BATCH` slot bit or the
//! shader's `glow_texel` - reads as the PSP's floor of `4` or as nothing at all.

use std::path::Path;

/// The greyscale values of a PNG this project's own writer made - see
/// `glow_stamp_ground_truth.rs` for why this needs no decoder.
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
        .args(["--race", "--no-audio", "--size", "480x272", "--opponents"])
        .args(["--render-scale", "100", "--msaa", "off"])
        .args(["--screen-filter", "off", "--anisotropy", "off"])
        .args(["--motion-blur", "off", "--ticks", "1"])
        .args(["--track", r"Data\Environments\03_Track\track.vex"])
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
#[ignore = "needs data/images/pulse-ps2-eu.chd and a GPU adapter"]
fn moa_therma_s_grid_mask_is_graded_and_zero_where_nothing_glows() {
    let Some(image) = oag_testdata::image("data/images/pulse-ps2-eu.chd") else {
        return;
    };
    let scratch = std::env::temp_dir().join(format!("oag-ps2-mask-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("creating the scratch directory");
    let mask = mask_of(&image, &scratch);
    std::fs::remove_dir_all(&scratch).ok();

    let zero = mask.iter().filter(|&&v| v == 0).count() as f32 / mask.len() as f32;
    let mut seen = [false; 256];
    for &v in &mask {
        seen[usize::from(v)] = true;
    }
    let levels = seen.iter().filter(|&&s| s).count();
    println!("mask: {:.1} % zero, {levels} distinct levels", zero * 100.0);
    // The original's grid frame is 96.7 % zero. Ours carries the HUD's own
    // alpha on top (the mask dump is taken after it), some percent, so this
    // asks for 85 % - against the PSP rule's floor of 4, which leaves none.
    assert!(
        zero >= 0.85,
        "{:.1} % of the mask is zero: the PS2's rule writes nothing outside a glow batch",
        zero * 100.0
    );
    // A glow batch's own alpha is a ramp (the original's frame holds 222
    // levels); one constant per batch would hold a handful.
    assert!(
        levels >= 100,
        "{levels} distinct mask levels: a batch stamped one constant, not its own alpha"
    );
}
