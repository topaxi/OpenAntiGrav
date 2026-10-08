//! Wipeout HD's Bomb detonation draws, on a real disc: the fireball, its white
//! core, the bloom disc and the shockwave rings, written and drawn from the
//! blast object `oag_raceplay::bomb_blast::hd` plays back.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test hd_bomb_blast_ground_truth --run-ignored all
//! ```
//!
//! # What only a frame can say here
//!
//! The arithmetic is pinned in `bomb_blast::hd`'s own tests; what a frame adds
//! is that the models load, take the circuit's scene block, run their own
//! programs (`BOMB_FIRE`, `BOMB_SHOCK`) and reach the screen. The scenario is
//! the one `--force-bomb-trip` exists for: the player lays a Bomb at tick 401,
//! rival 1 is put on it at the end of tick 420 and the simulation's own trip
//! test detonates it on tick 421. The camera is posed at the blast from the
//! track's far side, so the count does not follow the AI or the chase camera.
//! The control is tick 420, the same scene one tick before the blast; a dropped
//! write or draw leaves what the psys stage alone draws, which is brown.

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

/// The scene at `ticks`: Talon's Junction, the player laying one Bomb, rival 1
/// onto it after tick 420, seen from a fixed camera.
fn frame_of(ticks: u32) -> Option<Vec<[u8; 3]>> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    let scratch =
        std::env::temp_dir().join(format!("oag-hd-bomb-blast-{}-{ticks}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("creating the scratch directory");
    let out = scratch.join("frame.png");
    let script = scratch.join("lay.inputs");
    std::fs::write(&script, "400 cross\n1 cross square\n400 cross\n").expect("the input script");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(image)
        .args(["--race", "--no-audio", "--size", "960x544", "--opponents"])
        .args(["--render-scale", "100", "--msaa", "off"])
        .args(["--screen-filter", "off", "--anisotropy", "off"])
        // The fireball's own alpha test is what the white count reads; the
        // blast's point lights whiten the walls around it (`weapon_light`).
        .arg("--no-weapon-lights")
        .args(["--motion-blur", "off", "--ticks", &ticks.to_string()])
        .args([
            "--give",
            "bomb",
            "--force-bomb-trip",
            "420:1",
            "--input-script",
        ])
        .arg(&script)
        .arg("--camera-pose=-52,-40,-200,34,-9.7,24,0,1,0")
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

/// The fireball's yellow-orange and the rings' glow: red and green high, blue
/// low. The scene without a blast has a few hundred (a lit rail, a craft's
/// glow); the blast's psys puffs are brown and add none.
fn yellow([r, g, b]: [u8; 3]) -> bool {
    r > 230 && g > 190 && b < 170
}

/// Near-white: the white-hot fireball and the sky behind it.
fn white([r, g, b]: [u8; 3]) -> bool {
    r > 245 && g > 245 && b > 235
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn a_detonating_bomb_draws_its_fireball_and_then_its_ripple_rings() {
    // Yellow pixels of 522,240. Before the blast: 366. The fireball at 0.67 s:
    // 13,025 drawn, 578 with the model draw dropped (the smoke ring and rays
    // the psys stage draws remain). The ripple rings at 1.8 s: 2,957 drawn, 348
    // dropped.
    let at = |ticks| count(&frame_of(ticks).expect("image"), yellow);
    let (before, ball, rings) = (at(420), at(460), at(528));
    println!("yellow pixels: before {before}, fireball {ball}, ripple rings {rings}");
    // Age 1.32 s: the fireball is white-hot and burning away. Its alpha test is
    // `GL_LESS` 0.5, so as `AlphaAnim` falls the texels with the most alpha go;
    // near-white pixels of the frame: 55,217 with the test, 67,264 with the
    // discard dropped (a solid ball), 52,661 and 65,326 either side of it at
    // ticks 440 and 480, where nothing has gone yet.
    let burnt = count(&frame_of(500).expect("image"), white);
    println!("near-white pixels at 1.32 s: {burnt}");
    assert!(burnt < 62_000, "the fireball is not burning away: {burnt}");
    assert!(before < 1_000, "the control already glows: {before}");
    assert!(ball > 5_000, "the fireball is missing: {ball}");
    assert!(rings > 1_500, "the ripple rings are missing: {rings}");
}
