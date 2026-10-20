//! The craft's sun-occlusion map holds the road's mask sRGB-encoded, as Wipeout HD's own does.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not ship.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game hd_sun_occlusion_ground_truth:: --run-ignored all
//! ```
//!
//! Read off RPCS3 (2026-10-20, Harimau classic at the Talon's Junction Time Trial start, the hull's
//! unit-3 map, 256x256 `B8`): the road under the craft holds **61** across almost the whole map, and
//! the circuit's lightmap alpha there is the byte **12**. `encode(12 / 255) * 255` is 61.4, and the
//! smallest alpha of that lightmap, 1, lands on the map's smallest texel, 13 (`encode(1 / 255) * 255`
//! is 12.9). So the pass writes the mask through the sRGB encode and the hull reads it raw as its sun
//! gate: a road at alpha 12 lets a fifth of the sun through when it is stored linear and a quarter when
//! stored as the original stores it. Our map read 12 there until the encode went in.

use std::path::Path;

/// The grey values of a PNG `oag_texture::png::encode_rgba` wrote: stored deflate blocks, filter 0.
fn grey_texels(png: &[u8]) -> Vec<u8> {
    let mut at = 8;
    let (mut width, mut zlib) = (0usize, Vec::new());
    while at + 8 <= png.len() {
        let len = u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
        let kind = &png[at + 4..at + 8];
        let body = &png[at + 8..at + 8 + len];
        if kind == b"IHDR" {
            width = u32::from_be_bytes(body[..4].try_into().unwrap()) as usize;
        } else if kind == b"IDAT" {
            zlib.extend_from_slice(body);
        }
        at += 12 + len;
    }
    let (mut raw, mut at) = (Vec::new(), 2);
    loop {
        let last = zlib[at] & 1 == 1;
        let len = u16::from_le_bytes([zlib[at + 1], zlib[at + 2]]) as usize;
        raw.extend_from_slice(&zlib[at + 5..at + 5 + len]);
        at += 5 + len;
        if last {
            break;
        }
    }
    raw.chunks_exact(1 + width * 4)
        .flat_map(|row| row[1..].as_chunks::<4>().0.iter().map(|px| px[0]))
        .collect()
}

fn occlusion_map(scratch: &Path) -> Option<Vec<u8>> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    std::fs::create_dir_all(scratch).ok()?;
    let dump = scratch.join("occlusion.png");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(&image)
        .args(["--race", "--no-audio", "--no-intro", "--team", "harimau"])
        .args(["--size", "480x272", "--ticks", "5"])
        .arg("--screenshot")
        .arg(scratch.join("frame.png"))
        .env("OAG_DUMP_SUN_OCCLUSION", &dump)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"))
        .output()
        .expect("running oag-game");
    assert!(out.status.success(), "oag-game failed");
    Some(grey_texels(&std::fs::read(dump).expect("the dump")))
}

#[test]
#[ignore = "needs the HD disc image"]
fn the_road_mask_is_stored_srgb_encoded() {
    let Some(mut texels) = occlusion_map(&std::env::temp_dir().join("oag-hd-sun-occlusion")) else {
        return;
    };
    texels.sort_unstable();
    let median = texels[texels.len() / 2];
    assert!(
        (59..=63).contains(&median),
        "the road's lightmap alpha is 12, which the original's map holds as 61; the median texel is {median}"
    );
}
