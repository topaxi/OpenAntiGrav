//! Wipeout HD's weapon drawables are shaded from the circuit's fog, light and
//! eye, on a real disc: the Mine's halo, the Rocket's body, the Bomb's body,
//! the Cannon's muzzle flash and the Plasma explosion's shells.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test hd_weapon_scene_ground_truth --run-ignored all
//! ```
//!
//! # What only a frame can say here
//!
//! A drawable nothing writes a scene block holds `Scene::off`: fog off, a
//! stand-in light and a camera at the world's origin. Every HD weapon program
//! reads those the way a hull's does, so the model, the program and the
//! placement can all be right and the picture still wrong
//! (`weapon_models::write_fog`). Each test below pins the player's own
//! weapon at a tick the run reaches with **no rival on the grid**, so nothing
//! here follows the AI, and counts the pixels that tell the written drawable
//! from the unwritten one. The thresholds sit between the two measured values,
//! which are in each test's comment (written, then the write dropped).

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

/// Talon's Junction, the player alone, a `weapon` fired at tick 300 and the
/// frame at `ticks`, from the chase camera or from `pose`.
fn frame_of(weapon: &str, ticks: u32, pose: Option<&str>) -> Option<Vec<[u8; 3]>> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    let scratch = std::env::temp_dir().join(format!(
        "oag-hd-weapon-scene-{}-{weapon}-{ticks}",
        std::process::id()
    ));
    std::fs::create_dir_all(&scratch).expect("creating the scratch directory");
    let out = scratch.join("frame.png");
    let script = scratch.join("fire.inputs");
    std::fs::write(&script, "300 cross\n2 cross square\n2000 cross\n").expect("the input script");
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"));
    command
        .arg(image)
        .args(["--race", "--no-audio", "--size", "960x544"])
        .args(["--render-scale", "100", "--msaa", "off"])
        .args(["--screen-filter", "off", "--anisotropy", "off"])
        .args(["--motion-blur", "off", "--ticks", &ticks.to_string()])
        .args(["--give", weapon, "--input-script"])
        .arg(&script)
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"));
    if let Some(pose) = pose {
        command.arg(format!("--camera-pose={pose}"));
    }
    assert!(command.status().expect("running oag-game").success());
    let frame = rgb_of(&std::fs::read(&out).expect("the frame"));
    std::fs::remove_dir_all(&scratch).ok();
    Some(frame)
}

/// Pixels of `frame` inside `[x0, x1) x [y0, y1)` for which `keep` holds.
fn count(
    frame: &[[u8; 3]],
    (x0, y0, x1, y1): (usize, usize, usize, usize),
    keep: impl Fn([u8; 3]) -> bool,
) -> usize {
    (y0..y1)
        .flat_map(|y| (x0..x1).map(move |x| frame[y * SIZE.0 + x]))
        .filter(|&p| keep(p))
        .count()
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn a_laid_mine_shows_its_green_halo_spikes() {
    // The mine lies at (12.53, -50.04, -195.84) at tick 312; the camera is
    // 7 units above and behind it. Green pixels: 5,751 written, 115 dropped.
    let Some(frame) = frame_of("mine", 312, Some("12.5,-46.5,-190.0,0,-0.51,-0.85,0,1,0")) else {
        return;
    };
    let green = count(&frame, (0, 0, SIZE.0, SIZE.1), |[r, g, b]| {
        // Windows scaled by 0.73 (the encoded value of one half, 2026-10-09): HD's
        // lit programs end on a /2 output scale.
        g > 146 && r < 88 && b < 95
    });
    println!("green pixels: {green}");
    assert!(
        green > 2000,
        "the halo's spikes are faint: the eye is the world's origin again"
    );
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn a_rockets_body_is_lit_not_flat_black() {
    // The first rocket is at (24.6, -50.98, -196.54) at tick 304. Pixels with
    // every channel under 40 in the rocket's box: 57 written, 1,675 dropped.
    let Some(frame) = frame_of(
        "rocket",
        304,
        Some("24.60,-47.48,-190.74,0,-0.51,-0.85,0,1,0"),
    ) else {
        return;
    };
    let black = count(&frame, (470, 230, 780, 390), |p| {
        p.into_iter().max() < Some(40)
    });
    println!("black pixels in the rocket's box: {black}");
    assert!(
        black < 600,
        "the rocket is flat black: no light rig reached it"
    );
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn a_laid_bombs_body_is_lit_by_the_circuits_rig() {
    // The bomb lies at (10.04, -50.05, -195.83) at tick 310. Pixels with every
    // channel under 40 in its box: 1,463 written, 2,297 dropped.
    let Some(frame) = frame_of(
        "bomb",
        310,
        Some("10.04,-46.55,-190.03,0,-0.51,-0.85,0,1,0"),
    ) else {
        return;
    };
    let black = count(&frame, (360, 170, 560, 300), |p| {
        p.into_iter().max() < Some(40)
    });
    println!("black pixels in the bomb's box: {black}");
    assert!(
        black < 1900,
        "the bomb is as dark as the stand-in light left it"
    );
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn the_cannons_muzzle_flash_takes_the_circuits_scene() {
    // The chase camera at tick 304, the flash in a 66 x 72 box ahead of the
    // nose. Near-white pixels: 1,742 written, 1,576 dropped (2,916 and 2,849
    // before the hull's `VertexColour1` became a factor on its light, which
    // darkened the hull pixels this box also holds). The frame is
    // deterministic, so 1,660 sits between the two.
    let Some(frame) = frame_of("cannon", 304, None) else {
        return;
    };
    let white = count(&frame, (383, 246, 449, 318), |p| {
        p.into_iter().max() > Some(168)
    });
    println!("near-white pixels in the flash's box: {white}");
    assert!(
        white > 1660,
        "the muzzle flash is shaded from the stand-in scene"
    );
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn a_plasma_explosions_shells_glow_violet_from_the_real_eye() {
    // The bolt bursts at (278.9, -40.6, -196.6) and its halo, ring and sphere
    // are 46 across at tick 490; the camera is 90 units off. Violet pixels:
    // 10,521 written, 38 dropped.
    let Some(frame) = frame_of(
        "plasma",
        490,
        Some("190.00,-20.00,-170.00,0.94,-0.22,-0.28,0.00,1.00,0.00"),
    ) else {
        return;
    };
    let violet = count(&frame, (0, 0, SIZE.0, SIZE.1), |[r, g, b]| {
        let (r, g, b) = (i32::from(r), i32::from(g), i32::from(b));
        b > 124 && r > 80 && r < 172 && g + 15 < r && b > g + 37
    });
    println!("violet pixels: {violet}");
    assert!(
        violet > 3000,
        "the shells are ghosts: their rim is read from the world's origin"
    );
}
