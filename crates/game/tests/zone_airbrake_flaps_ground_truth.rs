//! A raised airbrake flap sits on its hinge, in a Zone race as in any other.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(zone_airbrake_flaps_ground_truth)'
//! ```
//!
//! HD's and 2048's hulls hang each flap under an `Anim Transform` hinge, so its
//! vertices are baked in the hinge's own space and only the model's node
//! animation table puts it on the hull. The race never wrote that table for a
//! craft, so the flap drew at the model origin - the middle of the hull -
//! whatever the mode. Zone is where a player noticed: its hull is white and the
//! flaps are small, so a brake that raised a blob mid-hull read as "the flaps
//! do not move".
//!
//! Two frames of the same race at the same tick, one with the airbrakes held:
//! everything the brake changes is the flaps, so where the changed pixels sit is
//! where the flaps are drawn. A hull is symmetric about the screen's centre
//! column from the chase camera, and its flaps hang on either side of it.

use std::path::Path;

const SIZE: (usize, usize) = (720, 408);

/// Decodes a PNG written by `oag_texture::png::encode_rgba`: filter 0, stored
/// deflate blocks, RGBA8.
fn decode(png: &[u8]) -> Vec<u8> {
    let mut at = 8;
    let mut idat = Vec::new();
    while at + 8 <= png.len() {
        let len = u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
        if &png[at + 4..at + 8] == b"IDAT" {
            idat.extend_from_slice(&png[at + 8..at + 8 + len]);
        }
        at += 12 + len;
    }
    let mut raw = Vec::new();
    let mut at = 2;
    loop {
        let last = idat[at] & 1 == 1;
        assert_eq!(idat[at] >> 1, 0, "not a stored deflate block");
        let len = u16::from_le_bytes([idat[at + 1], idat[at + 2]]) as usize;
        raw.extend_from_slice(&idat[at + 5..at + 5 + len]);
        at += 5 + len;
        if last {
            break;
        }
    }
    let stride = SIZE.0 * 4 + 1;
    assert_eq!(raw.len(), stride * SIZE.1, "unexpected image size");
    raw.chunks_exact(stride)
        .flat_map(|row| {
            assert_eq!(row[0], 0, "filtered row");
            row[1..].to_vec()
        })
        .collect()
}

fn frame(source: &Path, scratch: &Path, name: &str, mode: &str, hold: &str) -> Vec<u8> {
    let out = scratch.join(format!("{name}.png"));
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(source)
        .args(["--race", "--no-audio", "--size", "720x408"])
        .args(["--render-scale", "100", "--msaa", "off"])
        .args(["--screen-filter", "off", "--anisotropy", "off"])
        .args(["--motion-blur", "off", "--scheme", "novice"])
        .args(["--mode", mode, "--hold", hold, "--ticks", "240"])
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"))
        .output()
        .expect("running oag-game");
    assert!(run.status.success(), "oag-game {name}");
    decode(&std::fs::read(&out).expect("reading the screenshot"))
}

/// The changed pixels' columns: how many on each side of the centre column, and
/// how many in a narrow band around it.
fn columns(brake: &[u8], rest: &[u8]) -> (usize, usize, usize) {
    let (mut left, mut right, mut centre) = (0, 0, 0);
    let half = SIZE.0 / 2;
    let band = SIZE.0 * 3 / 100;
    for (i, (a, b)) in brake
        .as_chunks::<4>()
        .0
        .iter()
        .zip(rest.as_chunks::<4>().0)
        .enumerate()
    {
        let moved = a.iter().zip(b).any(|(x, y)| x.abs_diff(*y) > 12);
        if !moved {
            continue;
        }
        let x = i % SIZE.0;
        if x.abs_diff(half) < band {
            centre += 1;
        } else if x < half {
            left += 1;
        } else {
            right += 1;
        }
    }
    (left, right, centre)
}

fn check(source: &Path, tag: &str) {
    let scratch = std::env::temp_dir().join(format!("oag-zone-flaps-{tag}"));
    std::fs::create_dir_all(&scratch).unwrap();
    for mode in ["zone", "single_race"] {
        let rest = frame(source, &scratch, &format!("{mode}-rest"), mode, "cross");
        let brake = frame(
            source,
            &scratch,
            &format!("{mode}-brake"),
            mode,
            "cross,l,r",
        );
        let (left, right, centre) = columns(&brake, &rest);
        println!("{tag} {mode}: left {left}, right {right}, centre band {centre}");
        assert!(
            left > 200 && right > 200,
            "{tag} {mode}: the brake changed {left} pixels left and {right} right of centre"
        );
        // On HD's single race this is what tells a flap on its hinge (about 2
        // percent) from one at the model origin (over 20); Zone's small white
        // flaps put few pixels in the band either way, so there it only
        // guards the gross case.
        assert!(
            centre * 10 < left + right,
            "{tag} {mode}: {centre} of {} changed pixels sit mid-hull - a flap drawn at the model origin",
            left + right + centre
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_flaps_are_on_their_hinges() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    check(&image, "hd");
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn wipeout_2048_flaps_are_on_their_hinges() {
    let Some(path) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    check(&path, "2048");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulse_flaps_are_on_their_hinges() {
    let Some(image) = oag_testdata::image("pulse-psp-eu.chd") else {
        return;
    };
    check(&image, "pulse");
}
