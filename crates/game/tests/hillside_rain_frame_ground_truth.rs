//! Hillside's rain as a player sees it: the rendered frame of the final lap, against the same
//! tick of the same race without the track module.
//!
//! **`#[ignore]`d and never run in CI**: it needs `data/images/pulse-psp-eu.chd`, the
//! `assets/mods/examples` submodule and the `extensions` feature:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --features extensions \
//!     --run-ignored only -E 'test(/hillside_rain_frame_ground_truth::/)'
//! ```
//!
//! Two copies of the mod, one with its `[extension]` removed, each raced through the real
//! `oag-game` binary (`--track-file`, Single Race with opponents, the player idle so the
//! camera never moves after the first bend), a frame captured at two ticks. Measured
//! 2026-10-30 at 960x544 (sky box x 300..420, y 0..55; flare patch x 420..520, y 150..230):
//!
//! | Tick | Sky saturation | Sky luminance | Flare patch luminance |
//! | --- | --- | --- | --- |
//! | 6,000, module on | 0.201 | 0.711 | 0.441 |
//! | 6,000, module off | 0.201 | 0.711 | pixel-identical frame |
//! | 9,000, module on | 0.088 | 0.537 | 0.255 |
//! | 9,000, module off | 0.202 | 0.753 | 0.433 |
//!
//! What fails it: dropping `grade-sky`'s uniform write (the sky stays blue), `set-sun`'s alpha
//! scale (the flare patch stays bright), or a track module that runs before its final lap (the
//! tick 6,000 frames differ).
#![cfg(feature = "extensions")]

use std::path::{Path, PathBuf};

const MOD: &str = "assets/mods/examples/hillside";

/// RGB texels of a PNG `oag_texture::png::encode_rgba` wrote: stored deflate blocks, filter 0.
fn decode(png: &[u8]) -> (usize, Vec<[u8; 3]>) {
    let mut at = 8;
    let (mut width, mut zlib) = (0usize, Vec::new());
    while at + 8 <= png.len() {
        let len = u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
        let body = &png[at + 8..at + 8 + len];
        match &png[at + 4..at + 8] {
            b"IHDR" => width = u32::from_be_bytes(body[..4].try_into().unwrap()) as usize,
            b"IDAT" => zlib.extend_from_slice(body),
            _ => {}
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
    let texels = raw
        .chunks_exact(1 + width * 4)
        .flat_map(|row| {
            row[1..]
                .as_chunks::<4>()
                .0
                .iter()
                .map(|px| [px[0], px[1], px[2]])
                .collect::<Vec<_>>()
        })
        .collect();
    (width, texels)
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            if entry.file_name() != "target" {
                copy_dir(&entry.path(), &target);
            }
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// `with` and `without` the module: the mod copied under `scratch`, the second with its
/// `[extension]` tables cut off.
fn copies(scratch: &Path) -> Option<[PathBuf; 2]> {
    let manifest = oag_testdata::exact(&format!("{MOD}/oag-mod.toml"))?;
    let source = manifest.parent().unwrap();
    let dirs = [scratch.join("on"), scratch.join("off")];
    for dir in &dirs {
        let _ = std::fs::remove_dir_all(dir);
        copy_dir(source, &dir.join("hillside"));
    }
    let path = dirs[1].join("hillside/oag-mod.toml");
    let text = std::fs::read_to_string(&path).unwrap();
    let cut = text
        .find("\n[extension]")
        .expect("the mod declares [extension]");
    // The comment block above `[extension]` goes with it.
    let cut = text[..cut].rfind("\n# Hillside's rain").unwrap_or(cut);
    std::fs::write(&path, &text[..cut]).unwrap();
    Some(dirs)
}

fn frame(mods: &Path, scratch: &Path, ticks: u32) -> (usize, Vec<[u8; 3]>) {
    let image = oag_testdata::image("data/images/pulse-psp-eu.chd").expect("the image");
    let out = scratch.join(format!(
        "{}-{ticks}.png",
        mods.file_name().unwrap().to_string_lossy()
    ));
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(&image)
        .args([
            "--race",
            "--mode",
            "single_race",
            "--opponents",
            "--no-audio",
        ])
        .arg("--track-file")
        .arg(mods.join("hillside/tracks/hillside.track.toml"))
        .args(["--size", "960x544", "--ticks", &ticks.to_string()])
        .arg("--screenshot")
        .arg(&out)
        .env("OAG_MODS", mods)
        .env("XDG_CONFIG_HOME", scratch.join("xdg/config"))
        .env("XDG_DATA_HOME", scratch.join("xdg/data"))
        .env("XDG_STATE_HOME", scratch.join("xdg/state"))
        .output()
        .expect("running oag-game");
    assert!(run.status.success(), "oag-game failed");
    decode(&std::fs::read(out).expect("the frame"))
}

/// Mean saturation and luminance of a box.
fn measure((width, texels): &(usize, Vec<[u8; 3]>), [x0, y0, x1, y1]: [usize; 4]) -> (f32, f32) {
    let (mut saturation, mut luminance, mut n) = (0.0, 0.0, 0.0);
    for y in y0..y1 {
        for x in x0..x1 {
            let [r, g, b] = texels[y * width + x].map(|c| f32::from(c) / 255.0);
            let (max, min) = (r.max(g).max(b), r.min(g).min(b));
            saturation += if max > 0.0 { (max - min) / max } else { 0.0 };
            luminance += 0.2126 * r + 0.7152 * g + 0.0722 * b;
            n += 1.0;
        }
    }
    (saturation / n, luminance / n)
}

#[test]
#[ignore = "needs a Pulse disc image, the mod examples and --features extensions"]
fn the_final_lap_frame_is_greyer_darker_and_flareless_and_an_earlier_one_is_untouched() {
    let scratch = std::env::temp_dir().join("oag-hillside-rain-frame");
    let Some([on, off]) = copies(&scratch) else {
        return;
    };
    // Before the final lap the two runs draw the very same frame.
    let (early_on, early_off) = (frame(&on, &scratch, 6_000), frame(&off, &scratch, 6_000));
    assert!(
        early_on == early_off,
        "a lap before the final one differs from the vanilla frame"
    );
    let (late_on, late_off) = (frame(&on, &scratch, 9_000), frame(&off, &scratch, 9_000));
    let sky = [300, 0, 420, 55];
    let flare = [420, 150, 520, 230];
    let ((sat_on, lum_on), (sat_off, lum_off)) = (measure(&late_on, sky), measure(&late_off, sky));
    let ((_, flare_on), (_, flare_off)) = (measure(&late_on, flare), measure(&late_off, flare));
    println!(
        "sky saturation {sat_off:.3} to {sat_on:.3}, luminance {lum_off:.3} to {lum_on:.3}, flare patch {flare_off:.3} to {flare_on:.3}"
    );
    assert!(
        sat_on < sat_off * 0.6,
        "the sky greys: {sat_off} to {sat_on}"
    );
    assert!(
        lum_on < lum_off * 0.85,
        "the sky darkens: {lum_off} to {lum_on}"
    );
    assert!(
        flare_on < flare_off * 0.8,
        "the flare fades: {flare_off} to {flare_on}"
    );
    assert!(late_on != late_off, "the final lap draws something else");
}
