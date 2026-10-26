//! A weapon hit throws `CockpitHitFx`'s noise over the cockpit view, on a real
//! Pulse disc: an opponent's Quake reaching the player in the internal camera.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game cockpit_hit_ground_truth:: --run-ignored all
//! ```
//!
//! # What only a frame can say here
//!
//! The node's timeline and the strips' geometry are pinned in `oag_fx`, and the
//! arm in `oag_raceplay`; what a frame adds is that `noisematt.mip` loads off the
//! disc, the pipeline reaches the screen through the race's own pass, and the
//! picture has the original's shape. The scenario is `--force-quake`'s: Talon's
//! Junction White, Assegai, Venom, eight craft, the rival's wave reaching the
//! player on tick 301. The control is tick 301, the frame before the hit.
//!
//! The measure is the one `cannon-quake-leachbeam.md` used on the original's
//! cockpit frames: the mean absolute vertical difference of the luma over rows
//! 30 to 240 of the PSP's 480 by 272 frame, per band of columns. The original
//! reads 41 to 43 in the outer 30 px, 14 to 15 at x 120 to 150 and 9 to 10 in the
//! middle at rows +2 and +4, against 4, 1.5 and 7.5 before. Ours is rendered at
//! that size, so a texel is the same fraction of the frame.

const SIZE: (usize, usize) = (480, 272);

/// The bands the original's table is read in.
const BANDS: [(usize, usize); 7] = [
    (0, 30),
    (60, 90),
    (120, 150),
    (225, 255),
    (330, 360),
    (390, 420),
    (450, 480),
];

/// The luma of every pixel of a PNG this project's own writer made.
///
/// That writer stores its deflate blocks uncompressed and filters nothing
/// (`oag_texture::png`), so a reader needs only the chunk and block framing.
fn luma_of(png: &[u8]) -> Vec<f32> {
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
                .map(|p| {
                    0.299 * f32::from(p[0]) + 0.587 * f32::from(p[1]) + 0.114 * f32::from(p[2])
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The cockpit frame at `ticks`, with the rival's Quake launched at the end of
/// tick 300, as the mean vertical luma difference in each of [`BANDS`].
fn noise_bands(ticks: u32) -> Option<[f32; 7]> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let scratch =
        std::env::temp_dir().join(format!("oag-cockpit-hit-{}-{ticks}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("creating the scratch directory");
    let out = scratch.join("frame.png");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(image)
        .args(["--race", "--no-audio", "--size", "480x272", "--opponents"])
        .args(["--camera-view", "internal", "--render-scale", "100"])
        .args([
            "--msaa",
            "off",
            "--screen-filter",
            "off",
            "--motion-blur",
            "off",
        ])
        .args(["--force-quake", "300:1", "--ticks", &ticks.to_string()])
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"))
        .status()
        .expect("running oag-game");
    assert!(status.success());
    let luma = luma_of(&std::fs::read(&out).expect("the frame"));
    std::fs::remove_dir_all(&scratch).ok();
    Some(BANDS.map(|(from, to)| {
        let (mut sum, mut count) = (0.0, 0);
        for y in 30..240 {
            for x in from..to {
                sum += (luma[(y + 1) * SIZE.0 + x] - luma[y * SIZE.0 + x]).abs();
                count += 1;
            }
        }
        sum / count as f32
    }))
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and a GPU adapter"]
fn a_quake_hit_in_the_cockpit_view_draws_noise_at_the_screens_edges() {
    let Some(before) = noise_bands(301) else {
        return;
    };
    let after = noise_bands(304).expect("image");
    println!("before {before:?}");
    println!("after  {after:?}");
    println!("original row -15 [3.8, 2.0, 1.5, 7.5, 5.4, 4.2, 4.4]");
    println!("original row +2  [40.7, 26.5, 15.0, 9.9, 17.3, 25.3, 42.4]");
    println!("original row +4  [43.0, 28.0, 13.9, 9.2, 18.5, 24.0, 42.1]");
    let lift: Vec<f32> = after.iter().zip(&before).map(|(a, b)| a - b).collect();
    // Noise at the outer 30 px of both edges, the original's 37 to 39 over its
    // baseline; ours measures 32 and 39.
    assert!(lift[0] > 20.0, "the left edge is not noisy: {lift:?}");
    assert!(lift[6] > 20.0, "the right edge is not noisy: {lift:?}");
    // Fading inwards, to the middle that the strips barely reach by then.
    assert!(lift[0] > 2.0 * lift[2], "no gradient on the left: {lift:?}");
    assert!(
        lift[6] > 2.0 * lift[4],
        "no gradient on the right: {lift:?}"
    );
    assert!(
        lift[3] < 6.0,
        "the middle of the frame is wiped, not left clear: {lift:?}"
    );
}
