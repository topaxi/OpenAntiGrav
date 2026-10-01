//! A weapon's body stamps the bloom's mask like every other `.vex` model, on a real disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test weapon_stamp_ground_truth --run-ignored all
//! ```
//!
//! # What only real data can say here
//!
//! A PPSSPP running the original on its **software renderer** (so EDRAM is the real
//! framebuffer), a Bomb dropped at speed 106 on Talon's Junction with the chase camera
//! behind it, the alpha plane read at fire+3 to +7 beside a no-fire control
//! (`scripts/psp-weapon-pair.py --edram`): where the canister is drawn the mask holds
//! `0xba` (186) over 1,006 to 1,326 pixels at fire+5 to +7 - `mine_flash_GLOW`'s own
//! glow byte, a value nothing else in the frame writes - and the road under its body
//! reads `4` where the control's read `255` (756 pixels at fire+5, 2,965 at fire+6): the
//! opaque batch stamps the neutral value. Until 2026-10-01 this engine stamped neither,
//! because the weapon bodies were not on `pulse_psp::finish`'s list, so the canister
//! kept the road's `255` under it and bloomed as a pale wash.
//!
//! Ours reads the original's two numbers off the same frame. The control is the same
//! scenario with no weapon: the scenery's own animation and the HUD differ between any
//! two runs, so the assertion is on the *two values the bomb alone writes*.

use std::path::Path;

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

/// The mask at tick 407 of the weapon-after-GO scenario, holding `give` or nothing.
fn mask_of(image: &Path, scratch: &Path, give: Option<&str>, name: &str) -> Vec<u8> {
    let out = scratch.join(format!("{name}.png"));
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../verification/scenarios/weapon-after-go.inputs");
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"));
    command
        .arg(image)
        .args(["--race", "--mode", "time_trial", "--team", "Assegai"])
        .args(["--class", "venom", "--no-audio", "--size", "480x272"])
        .args(["--track", r"Data\Environments\16_Track\track.vex"])
        .args(["--render-scale", "100", "--msaa", "off"])
        .args(["--screen-filter", "off", "--anisotropy", "off"])
        .args(["--motion-blur", "off", "--ticks", "407"])
        // The bomb's ring is on `g_ingame->0x40`; the original's clock at fire+5.
        .args(["--anim-seconds", "70.338"])
        .arg("--input-script")
        .arg(script)
        .arg("--screenshot")
        .arg(scratch.join(format!("{name}-frame.png")))
        .env("OAG_DUMP_GLOW_MASK", &out)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"));
    if let Some(weapon) = give {
        command.args(["--give", weapon]);
    }
    assert!(command.status().expect("running oag-game").success());
    grey_of(&std::fs::read(&out).expect("the mask"), 480, 272)
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and a GPU adapter"]
fn a_laid_bombs_body_and_lamp_stamp_the_mask() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let scratch = std::env::temp_dir().join(format!("oag-weapon-stamp-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("creating the scratch directory");
    let bomb = mask_of(&image, &scratch, Some("bomb"), "bomb");
    let control = mask_of(&image, &scratch, None, "control");
    std::fs::remove_dir_all(&scratch).ok();

    let lamp = |m: &[u8]| m.iter().filter(|&&v| v == 0xba).count();
    let road_to_neutral = bomb
        .iter()
        .zip(&control)
        .filter(|&(&b, &c)| c == 255 && b == 4)
        .count();
    let lamp_pixels = lamp(&bomb).saturating_sub(lamp(&control));
    println!("bomb: {lamp_pixels} pixels at 0xba, {road_to_neutral} road pixels 255 -> 4");
    // Ours reads about 1,400 and 3,600 here; the original's frames 1,006 to 1,326 and
    // 756 to 2,965. A frame that drops the weapon bodies from the stamp list holds none
    // of the first and almost none of the second.
    assert!(
        lamp_pixels >= 700,
        "{lamp_pixels} lamp pixels at 0xba: the bomb's glow batch did not stamp"
    );
    assert!(
        road_to_neutral >= 700,
        "{road_to_neutral} road pixels reached 4 under the bomb: its opaque batch did not stamp"
    );
}
