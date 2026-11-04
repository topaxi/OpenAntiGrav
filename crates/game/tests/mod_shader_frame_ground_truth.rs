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
//! The second half is a portal: `oag.window` asks for `portal-view`, and its shader shows the
//! engine's view of the road further on inside a round surface. Five runs a title at one tick:
//! the mod, no mod (no secondary view, so the one the texels outside the surface are held to),
//! its shader drawing a procedural surface instead (the Low quality of a portal mod), a flat shader that gives the surface's exact texels, and no surface at all from the
//! camera the portal view was drawn from (`--camera-pose`, off the debug log).
//!
//! What fails that half: a texture other than the view from that camera, an empty or undecoded
//! one (HD's float target reads dark), a view that draws into the main picture or leaves it the
//! wrong matrices (the texels outside the surface move), and a shader that never gets the
//! texture (it matches the procedural frame). The reference camera is the view's own, off its
//! log, so a wrong camera agrees with itself here: that it is the right camera is
//! `oag_raceplay`'s `tests::portal_view` and `oag_render::camera::portal`'s tests.
//!
//! What fails the first half: dropping the beam's or the fill's draw from `Recorder::draw_effects` or from
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
    run_with(image, mods, scratch, ticks, "oag.tether", "", &[])
}

/// [`run`] with the mod `enabled`, a `[graphics]` line and arguments of the caller's.
fn run_with(
    image: &Path,
    mods: &Path,
    scratch: &Path,
    ticks: u32,
    enabled: &str,
    graphics: &str,
    args: &[&str],
) -> (Vec<u8>, String) {
    let out = scratch.join("frame.png");
    let _ = std::fs::remove_file(&out);
    let config = scratch.join("xdg/config/oag");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(
        config.join("settings.toml"),
        format!("[mods]\nenabled = [\"{enabled}\"]\n[graphics]\n{graphics}\n"),
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
        .args(args)
        .env("OAG_MODS", mods)
        .env("RUST_LOG", "info,oag_raceplay::scene::portal_view=debug")
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
    let mut out = Vec::new();
    for row in raw.chunks(1 + WIDTH as usize * 4) {
        for px in row[1..].as_chunks::<4>().0 {
            out.extend_from_slice(&px[..3]);
        }
    }
    out
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

const WINDOW: &str = "assets/mods/examples/window";

/// A window's surface in one flat colour, short of the rim the mod draws.
const FLAT_WINDOW: &str = "//! blend = \"alpha\"
fn mod_fragment(input: ModInput) -> vec4<f32> {
    if length(input.uv * 2.0 - vec2<f32>(1.0)) > 0.86 { return vec4<f32>(0.0); }
    return vec4<f32>(1.0, 0.0, 1.0, 1.0);
}
";

/// The window as the Low quality of a portal mod draws it: no engine texture.
const LOW_WINDOW: &str = "//! blend = \"alpha\"
fn mod_fragment(input: ModInput) -> vec4<f32> {
    let radius = length(input.uv * 2.0 - vec2<f32>(1.0));
    if radius > 0.96 { return vec4<f32>(0.0); }
    let ripple = 0.5 + 0.5 * sin(radius * 24.0 - input.time * 6.0);
    return vec4<f32>(input.colour.rgb * (0.4 + 0.6 * ripple), 1.0);
}
";

/// A copy of `oag.window` with the shader `text`, the mod's own with none.
fn window_mod(source: &Path, to: &Path, text: Option<&str>) {
    copy_dir(source, &to.join("window"));
    if let Some(text) = text {
        std::fs::write(to.join("window/fx/window.wgsl"), text).unwrap();
    }
}

/// The texels the surface covers: what the flat shader turned from what the run without it drew.
fn surface(flat: &[u8], plain: &[u8]) -> Vec<bool> {
    (0..flat.len() / 3)
        .map(|at| {
            let texel = |frame: &[u8], c: usize| i32::from(frame[at * 3 + c]);
            let moved = (0..3)
                .map(|c| (texel(flat, c) - texel(plain, c)).abs())
                .sum::<i32>();
            moved > 60
                && texel(flat, 0) > texel(flat, 1) + 40
                && texel(flat, 2) > texel(flat, 1) + 40
        })
        .collect()
}

/// `mask` grown by `by` texels: the mod's rim and its bloom reach past the flat surface.
fn grown(mask: &[bool], by: i32) -> Vec<bool> {
    let (width, height) = (WIDTH as i32, HEIGHT as i32);
    let mut out = vec![false; mask.len()];
    for y in 0..height {
        for x in 0..width {
            if mask[(y * width + x) as usize] {
                for dy in (-by).max(-y)..=by.min(height - 1 - y) {
                    let row = ((y + dy) * width) as usize;
                    let from = row + (x - by).max(0) as usize;
                    let to = row + (x + by).min(width - 1) as usize;
                    out[from..=to].fill(true);
                }
            }
        }
    }
    out
}

/// Of the texels `within` picks, the share two frames disagree on and their mean difference a
/// channel, out of 255.
fn gap(a: &[u8], b: &[u8], within: impl Fn(usize) -> bool) -> (f32, f32) {
    let (mut hits, mut sum, mut all) = (0u32, 0u64, 0u32);
    for at in (0..a.len() / 3).filter(|at| within(*at)) {
        let moved: i32 = (0..3)
            .map(|c| (i32::from(a[at * 3 + c]) - i32::from(b[at * 3 + c])).abs())
            .sum();
        hits += u32::from(moved > 12);
        sum += moved as u64;
        all += 1;
    }
    (
        hits as f32 / all.max(1) as f32,
        sum as f32 / 3.0 / all.max(1) as f32,
    )
}

/// The camera the portal view was drawn from, as `--camera-pose` takes it.
fn logged_pose(log: &str) -> String {
    let line = log
        .lines()
        .find_map(|line| line.split("portal view from ").nth(1))
        .expect("the view was drawn and the log says from where");
    line.replace(" forward ", ",")
        .replace(" up ", ",")
        .replace(['[', ']', ' '], "")
}

/// `outside` is the most of the texels away from the surface that may differ from a race with no
/// mod, which draws no secondary view.
fn check_portal(image: &str, ticks: u32, graphics: &str, outside: f32) {
    let Some(image) = oag_testdata::image(image) else {
        return;
    };
    let Some(manifest) = oag_testdata::exact(&format!("{WINDOW}/oag-mod.toml")) else {
        return;
    };
    let source = manifest.parent().unwrap();
    let scratch: PathBuf =
        std::env::temp_dir().join(format!("oag-portal-frame-{}-{ticks}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    let dir = |name: &str, text: Option<&str>| {
        let at = scratch.join(name);
        std::fs::create_dir_all(&at).unwrap();
        if name != "off" {
            window_mod(source, &at, text);
        }
        at
    };
    let (on, off, low, flat) = (
        dir("on", None),
        dir("off", None),
        dir("low", Some(LOW_WINDOW)),
        dir("flat", Some(FLAT_WINDOW)),
    );
    let frame = |mods: &Path, args: &[&str]| {
        run_with(&image, mods, &scratch, ticks, "oag.window", graphics, args)
    };

    let (with, log) = frame(&on, &[]);
    let (without, _) = frame(&off, &[]);
    let (low_frame, _) = frame(&low, &[]);
    let (flat_frame, _) = frame(&flat, &[]);
    let pose = format!("--camera-pose={}", logged_pose(&log));
    let (reference, _) = frame(&off, &[pose.as_str()]);

    let inside = surface(&flat_frame, &without);
    let near = grown(&inside, 88);
    let share = inside.iter().filter(|hit| **hit).count() as f32 / inside.len() as f32;
    let (off_share, off_mean) = gap(&with, &without, |at| inside[at]);
    let (low_share, low_mean) = gap(&with, &low_frame, |at| inside[at]);
    let (reference_share, reference_mean) = gap(&with, &reference, |at| inside[at]);
    let (outside_share, outside_mean) = gap(&with, &without, |at| !near[at]);
    eprintln!(
        "surface {share:.4}; inside against no mod {off_share:.3} (mean {off_mean:.1}), against \
         the procedural frame {low_share:.3} (mean {low_mean:.1}), against the reference camera \
         {reference_share:.3} (mean {reference_mean:.1}); outside against no mod \
         {outside_share:.5} (mean {outside_mean:.2})"
    );
    assert!(share > 0.03, "the surface is on the screen: {share}");
    assert!(off_share > 0.4, "the view shows nothing new: {off_share}");
    assert!(
        low_share > 0.4,
        "the view is the procedural surface: {low_share}"
    );
    assert!(
        reference_mean * 3.0 < off_mean,
        "the view is not the exit's: {reference_mean} from the exit camera, {off_mean} from none"
    );
    assert!(
        outside_share < outside,
        "the secondary view reached the main picture: {outside_share}"
    );
    let _ = std::fs::remove_dir_all(&scratch);
}

#[test]
#[ignore = "needs a disc image and the examples submodule"]
fn pulse_shows_the_road_further_on_through_a_portal() {
    check_portal("data/images/pulse-psp-eu.chd", 548, "", 0.0005);
}

/// A frame reading the velocity buffer: a view that writes the main view's camera history moves
/// 0.22 of the texels outside the surface. The bound stands off a patch at the craft's plume, the
/// same texels with a guest that asks for no view at all: the mod's, not the secondary view's.
#[test]
#[ignore = "needs a disc image and the examples submodule"]
fn the_portal_leaves_the_motion_blur_alone() {
    check_portal(
        "data/images/pulse-psp-eu.chd",
        548,
        "motion_blur = \"high\"",
        0.006,
    );
}

/// HD's float target stores encoded values, and a view read as it stands would be dark.
#[test]
#[ignore = "needs a disc image and the examples submodule"]
fn hd_shows_the_road_further_on_in_its_float_target() {
    check_portal("data/images/hdfury-ps3-eu-dec.iso", 1022, "", 0.0005);
}
