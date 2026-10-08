//! Wipeout HD's Missile explosion draws, on a real disc: `HD_missile_explosion`
//! (core, rays and shockwave rings) written and drawn when a Missile reaches a
//! craft, on the clock `oag_raceplay::missile_blast` plays.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test hd_missile_blast_ground_truth --run-ignored all
//! ```
//!
//! # What only a frame can say here
//!
//! The ageing and the pool are pinned in `missile_blast`'s own tests; what a
//! frame adds is that the model loads, its three programs run and reach the
//! screen. The scenario is `--force-missile-hit`'s: the player fires at tick
//! 286, rival 1 is put on the missile at the end of tick 289 and the
//! simulation's own hit test takes the explosion on tick 290. The control is
//! tick 289, one tick before; a dropped write or draw leaves what the psys
//! stage alone draws.

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

/// The scene at `ticks`: Talon's Junction, the player firing one Missile,
/// rival 1 onto it after tick 289, from the default chase camera.
fn frame_of(ticks: u32) -> Option<Vec<[u8; 3]>> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    let scratch = std::env::temp_dir().join(format!(
        "oag-hd-missile-blast-{}-{ticks}",
        std::process::id()
    ));
    std::fs::create_dir_all(&scratch).expect("creating the scratch directory");
    let out = scratch.join("frame.png");
    let script = scratch.join("fire.inputs");
    std::fs::write(&script, "285 none\n1 square\n400 none\n").expect("the input script");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(image)
        .args(["--race", "--no-audio", "--size", "960x544", "--opponents"])
        .args(["--render-scale", "100", "--msaa", "off"])
        .args(["--screen-filter", "off", "--anisotropy", "off"])
        .args(["--motion-blur", "off", "--ticks", &ticks.to_string()])
        .args(["--give", "missile", "--force-missile-hit", "289:1"])
        .arg("--input-script")
        .arg(&script)
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"))
        .status()
        .expect("running oag-game");
    assert!(status.success());
    let frame = rgb_of(&std::fs::read(&out).expect("the frame"));
    std::fs::remove_dir_all(&scratch).ok();
    Some(frame)
}

/// Pixels of `frame` for which `keep` holds.
fn count(frame: &[[u8; 3]], keep: impl Fn([u8; 3]) -> bool) -> usize {
    frame.iter().filter(|&&p| keep(p)).count()
}

/// The core's cream-white and the rings' yellow: red and green high.
fn bright([r, g, b]: [u8; 3]) -> bool {
    r > 235 && g > 200 && b > 120
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn a_missile_that_reaches_a_craft_draws_its_explosion() {
    // Bright pixels of 522,240. The control: 36,393. At 0.1 s: 200,364 drawn,
    // 38,411 with the model draw dropped (the psys stage's own burst remains).
    // At 0.2 s: 299,761 drawn, 32,539 dropped.
    let at = |ticks| count(&frame_of(ticks).expect("image"), bright);
    let (before, early, late) = (at(289), at(296), at(302));
    println!("bright pixels: before {before}, 0.1 s {early}, 0.2 s {late}");
    assert!(before < 60_000, "the control already glows: {before}");
    assert!(
        early > 120_000,
        "the explosion is missing at 0.1 s: {early}"
    );
    assert!(
        late > 200_000 && late > early,
        "the blast does not grow: {early} then {late}"
    );
}
