//! A mod's shaders as a player sees them: frames of a race with `oag.tether` on, against the
//! same tick of the same race with no mod and with its beam's shader broken.
//!
//! **`#[ignore]`d and never run in CI**: it needs the disc images, the `assets/mods/examples`
//! submodule and the `extensions` feature:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --features extensions \
//!     --run-ignored only -E 'test(/mod_shader_frame_ground_truth::/)'
//! ```
//!
//! Three real `oag-game` runs a title, autopilot from the grid. The mod draws nothing that
//! reaches the race, so every frame is the same race and any difference is the mod's. The beam
//! runs through the middle of the view and the screen fill darkens its corners.
//!
//! What fails it: dropping the beam's or the fill's draw from `Recorder::draw_effects` or from
//! `Scene::effects_drawn`, not attaching the stage, a stage built for a target the race pass
//! does not write (HD's float target: a wgpu panic, so the run fails), or a broken shader that
//! falls back to a flat colour.
#![cfg(feature = "extensions")]

use std::path::{Path, PathBuf};

const MOD: &str = "assets/mods/examples/tether";
const WIDTH: u32 = 960;
const HEIGHT: u32 = 544;

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

/// Runs the title's disc to `ticks` with the mods under `mods`, and gives the frame's RGB and
/// the run's log.
fn run(image: &Path, mods: &Path, scratch: &Path, ticks: u32) -> (Vec<u8>, String) {
    let out = scratch.join("frame.png");
    let _ = std::fs::remove_file(&out);
    let config = scratch.join("xdg/config/oag");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(
        config.join("settings.toml"),
        "[mods]\nenabled = [\"oag.tether\"]\n",
    )
    .unwrap();
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(image)
        .args([
            "--race",
            "--mode",
            "single_race",
            "--autopilot",
            "--no-audio",
        ])
        .args(["--size", &format!("{WIDTH}x{HEIGHT}")])
        .args(["--ticks", &ticks.to_string()])
        .arg("--screenshot")
        .arg(&out)
        .env("OAG_MODS", mods)
        .env("XDG_CONFIG_HOME", scratch.join("xdg/config"))
        .env("XDG_DATA_HOME", scratch.join("xdg/data"))
        .env("XDG_STATE_HOME", scratch.join("xdg/state"))
        .output()
        .expect("running oag-game");
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(run.status.success(), "oag-game failed:\n{log}");
    (rgb(&std::fs::read(out).expect("the frame")), log)
}

/// RGB bytes of a PNG `oag_texture::png::encode_rgba` wrote: stored deflate blocks, filter 0.
fn rgb(png: &[u8]) -> Vec<u8> {
    let mut at = 8;
    let mut zlib = Vec::new();
    while at + 8 <= png.len() {
        let len = u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
        if &png[at + 4..at + 8] == b"IDAT" {
            zlib.extend_from_slice(&png[at + 8..at + 8 + len]);
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
    raw.chunks_exact(1 + WIDTH as usize * 4)
        .flat_map(|row| {
            row[1..]
                .chunks_exact(4)
                .flat_map(|px| [px[0], px[1], px[2]])
        })
        .collect()
}

/// The share of the texels in a box that two frames disagree on by more than a rounding.
fn differ(a: &[u8], b: &[u8], [x0, y0, x1, y1]: [u32; 4]) -> f32 {
    let (mut hits, mut all) = (0, 0);
    for y in y0..y1 {
        for x in x0..x1 {
            let at = ((y * WIDTH + x) * 3) as usize;
            let gap: i32 = (0..3)
                .map(|c| (i32::from(a[at + c]) - i32::from(b[at + c])).abs())
                .sum();
            hits += i32::from(gap > 12);
            all += 1;
        }
    }
    hits as f32 / all as f32
}

/// The view's middle third, where the road ahead and so the beam run; HUD-free.
const MIDDLE: [u32; 4] = [320, 150, 640, 300];
/// The top-left corner: the fill alone, the beam never reaches it from a chase camera.
const CORNER: [u32; 4] = [0, 90, 60, 150];

fn check(image: &str, ticks: u32) {
    let Some(image) = oag_testdata::image(image) else {
        return;
    };
    let Some(manifest) = oag_testdata::exact(&format!("{MOD}/oag-mod.toml")) else {
        return;
    };
    let scratch: PathBuf = std::env::temp_dir().join(format!(
        "oag-mod-shader-frame-{}-{ticks}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&scratch);
    let (on, off, broken) = (
        scratch.join("on"),
        scratch.join("off"),
        scratch.join("broken"),
    );
    std::fs::create_dir_all(&off).unwrap();
    for mods in [&on, &broken] {
        copy_dir(manifest.parent().unwrap(), &mods.join("tether"));
    }
    std::fs::write(
        broken.join("tether/fx/beam.wgsl"),
        "fn mod_fragment(input: ModInput) -> vec4<f32> { loop { } }\n",
    )
    .unwrap();

    let (with, _) = run(&image, &on, &scratch, ticks);
    let (without, _) = run(&image, &off, &scratch, ticks);
    let (no_beam, log) = run(&image, &broken, &scratch, ticks);

    let beam = differ(&with, &without, MIDDLE);
    let fill = differ(&with, &without, CORNER);
    let left = differ(&no_beam, &without, MIDDLE);
    let kept = differ(&no_beam, &without, CORNER);
    eprintln!("beam {beam:.4} fill {fill:.4} broken beam {left:.4} its fill {kept:.4}");
    assert!(beam > 0.01, "no beam in the middle of the view: {beam}");
    assert!(fill > 0.5, "no fill in the corner: {fill}");
    assert!(
        left * 5.0 < beam,
        "a broken shader still drew: {left} of {beam}"
    );
    assert!(
        kept > 0.5,
        "a broken shader took the mod's others with it: {kept}"
    );
    assert!(
        log.contains("shader beam left out"),
        "the report names the shader: {log}"
    );
    let _ = std::fs::remove_dir_all(&scratch);
}

#[test]
#[ignore = "needs a disc image and the examples submodule"]
fn pulse_draws_the_beam_and_the_fill_through_the_mods_shaders() {
    check("data/images/pulse-psp-eu.chd", 640);
}

/// HD's race pass writes a float target of its own and stores encoded values.
#[test]
#[ignore = "needs a disc image and the examples submodule"]
fn hd_draws_them_into_its_float_target() {
    check("data/images/hdfury-ps3-eu-dec.iso", 800);
}
