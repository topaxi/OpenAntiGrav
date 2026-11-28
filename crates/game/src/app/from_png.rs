//! `--png-info` and `--from-png`: a screenshot of ours read back (`docs/formats/shot-png.md`).
//!
//! `--png-info` prints the `oag` text chunk and a line on the stream. `--from-png` turns the run
//! into the `--race --screenshot` route the shot could have been made by: the race is the
//! recorded one (`Options::apply_replay`), the capture drives it with the recorded inputs to the
//! shot's tick instead of a held button (`CaptureOptions::restage`), checks the state hash there
//! and draws the frame from the camera the text names.

use std::path::Path;
use std::sync::Arc;

use crate::shot_meta;
use anyhow::{Context, Result, bail};
use oag_replay::ShotRecord;

use crate::app::cli::Cli;

fn read(path: &Path) -> Result<shot_meta::Found> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    shot_meta::read(&bytes).with_context(|| format!("reading {}", path.display()))
}

/// What photo mode does to a frame whatever the settings say: it draws every section the
/// circuit's visibility sets would cull, at the output's size with no upscaler, the glass and
/// sea target at the original's size, and none of the game's own motion blur (a flag the person
/// gave stands).
fn draw_as_photo_mode(cli: &mut Cli) {
    cli.render.pvs = Some(false);
    cli.render.reflection_resolution = Some(oag_display::display::ReflectionResolution::Original);
    cli.capture_view.photo_profile = true;
    cli.render
        .blur
        .motion_blur
        .get_or_insert(oag_display::display::MotionBlur::Off);
}

/// The render scale (supersampling above 100 %) and anti-aliasing the photo was drawn with, from
/// its `[render]` text: a re-render is the photo, not the profile of the machine it runs on.
/// A photo without the section is drawn at the output's size with none.
fn draw_as_the_photo_was(meta: &shot_meta::Meta, cli: &mut Cli) {
    let Some(render) = &meta.render else {
        return;
    };
    if let Some(percent) = render
        .get("render_scale")
        .and_then(toml::Value::as_integer)
        .and_then(|percent| u32::try_from(percent).ok())
    {
        cli.render.render_scale = Some(percent.max(100));
    }
    if let Some(reconstruction) = render
        .get("reconstruction")
        .and_then(toml::Value::as_str)
        .and_then(|text| text.parse().ok())
    {
        cli.render.reconstruction = Some(reconstruction);
    }
}

/// What the re-render does about the picture controls the shot was made with, and points `cli`
/// at them. One line for each thing the reader could take for granted that is not so.
fn picture_lines(meta: &shot_meta::Meta, cli: &mut Cli) -> Vec<String> {
    let Some(picture) = &meta.picture else {
        return if meta.shot.source == "photo mode" {
            vec![
                "no picture controls recorded: the shot predates them, so it is drawn without any"
                    .to_string(),
            ]
        } else {
            Vec::new()
        };
    };
    cli.capture_view.picture = Some(picture.to_flag());
    vec![format!("picture controls applied: {}", picture.to_flag())]
}

/// The motion blur the shot was averaged from, when it was one, and the line that says what the
/// re-render does about it. A track blur needs the camera it rode the craft with
/// (`[camera.mount]`): a shot that predates the key is drawn as the one frame, and says so.
fn blurred_of(meta: &shot_meta::Meta) -> (Option<shot_meta::Blurred>, Option<String>) {
    let Some(picture) = &meta.picture else {
        return (None, None);
    };
    let Some(effects) = picture.blurred_effects() else {
        return (None, None);
    };
    let said = format!(
        "the shot is a {} motion blur (shutter {}, {} frames)",
        picture.motion_blur,
        picture.shutter.unwrap_or_default(),
        picture.frames
    );
    let mount = meta
        .camera
        .as_ref()
        .and_then(|camera| camera.mount.as_ref())
        .map(shot_meta::Mount::mount);
    let mount = match mount {
        Some(Ok(mount)) => Some(mount),
        Some(Err(why)) if picture.motion_blur == "track" => {
            return (
                None,
                Some(format!(
                    "{said}; its [camera.mount] cannot be used ({why}), so this re-render is the one frame"
                )),
            );
        }
        Some(Err(_)) | None => None,
    };
    if picture.motion_blur == "track" && mount.is_none() {
        return (
            None,
            Some(format!(
                "{said}; it predates the camera mount a track blur is drawn with, so this re-render is the one frame"
            )),
        );
    }
    let blurred = shot_meta::Blurred {
        effects,
        frames: picture.frames,
        mount,
    };
    (Some(blurred), Some(format!("{said}; drawing its frames")))
}

/// Prints what `path` carries to stdout.
pub(crate) fn info(path: &Path) -> Result<()> {
    let found = read(path)?;
    match &found.text {
        Some(text) => print!("{text}"),
        None => println!(
            "{}: no `oag` text chunk; not a screenshot of ours",
            path.display()
        ),
    }
    match &found.record {
        Some(record) => println!(
            "\n# oaGs: {} ticks of input for slot(s) {:?}, state hash {:#018x} at tick {}",
            record.replay.ticks(),
            record.replay.header.slots,
            record.hash,
            record.tick,
        ),
        None => println!("\n# oaGs: none (this shot carries no input stream)"),
    }
    if let Some(record) = &found.record {
        for on in &record.replay.header.active_mods {
            println!("# mod on: {} {}", on.id, on.version);
        }
        for used in &record.replay.header.mods {
            println!(
                "# simulation module: {} {} ({}), sha256 {}, picks {:?}",
                used.id, used.version, used.module, used.sha256, used.picks
            );
        }
    }
    Ok(())
}

/// Reads `path` and points `cli` at the `--race --screenshot` run that makes its frame again.
pub(crate) fn prepare(
    cli: &mut Cli,
    path: &Path,
) -> Result<(Arc<ShotRecord>, Option<shot_meta::Blurred>)> {
    let found = read(path)?;
    let Some(record) = found.record else {
        let why = found
            .meta
            .as_ref()
            .map(|meta| meta.shot.no_stream_because.clone())
            .filter(|why| !why.is_empty())
            .unwrap_or_else(|| "it has no oaGs chunk".to_string());
        bail!("{} cannot be made again: {why}", path.display());
    };
    let meta = found.meta.unwrap_or_default();
    cli.race = true;
    cli.capture.ticks = u32::try_from(record.tick).context("the shot's tick does not fit")?;
    if cli.screenshot.is_none() {
        let stem = path
            .file_stem()
            .map_or_else(|| "shot".into(), |s| s.to_string_lossy().into_owned());
        cli.screenshot = Some(format!("{stem}-rerender.png").into());
    }
    if meta.shot.source == "photo mode" {
        draw_as_photo_mode(cli);
        draw_as_the_photo_was(&meta, cli);
    }
    if meta.shot.width > 0 && meta.shot.height > 0 {
        cli.capture.size = format!("{}x{}", meta.shot.width, meta.shot.height);
    }
    if let Some(camera) = &meta.camera {
        let [ex, ey, ez] = camera.eye;
        let [fx, fy, fz] = camera.forward;
        let [ux, uy, uz] = camera.up;
        cli.capture_view.camera_pose =
            Some(format!("{ex},{ey},{ez},{fx},{fy},{fz},{ux},{uy},{uz}"));
        cli.capture_view.camera_fov = camera.fov_deg;
    }
    let (blurred, blur_line) = blurred_of(&meta);
    for line in picture_lines(&meta, cli).into_iter().chain(blur_line) {
        println!("from-png: {line}");
    }
    println!(
        "from-png: {}, {} on {}, tick {}, recorded hash {:#018x}",
        path.display(),
        meta.title.name,
        meta.race.as_ref().map_or("", |race| race.track.as_str()),
        record.tick,
        record.hash
    );
    Ok((Arc::new(record), blurred))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser as _;

    fn meta(source: &str, picture: Option<shot_meta::Picture>) -> shot_meta::Meta {
        shot_meta::Meta {
            shot: shot_meta::Shot {
                source: source.to_string(),
                ..shot_meta::Shot::default()
            },
            picture,
            ..shot_meta::Meta::default()
        }
    }

    #[test]
    fn a_photo_is_drawn_again_without_the_games_motion_blur_unless_asked_for() {
        let mut cli = Cli::try_parse_from(["oag-game"]).expect("parses");
        draw_as_photo_mode(&mut cli);
        assert_eq!(cli.render.pvs, Some(false));
        assert!(cli.capture_view.photo_profile);
        assert_eq!(
            cli.render.blur.motion_blur,
            Some(oag_display::display::MotionBlur::Off)
        );
        let mut asked = Cli::try_parse_from(["oag-game", "--motion-blur", "high"]).expect("parses");
        draw_as_photo_mode(&mut asked);
        assert_eq!(
            asked.render.blur.motion_blur,
            Some(oag_display::display::MotionBlur::High)
        );
    }

    #[test]
    fn a_photo_is_drawn_again_at_the_output_size_whatever_the_flags_of_the_run() {
        use oag_display::display::{Reconstruction, Scale};
        let mut cli = Cli::try_parse_from([
            "oag-game",
            "--render-scale",
            "50",
            "--reconstruction",
            "fsr3",
        ])
        .expect("parses");
        let mut profile = crate::settings::RenderProfile::default();
        cli.apply_render_overrides(&mut profile, Some(Scale::HALF));
        assert_eq!(profile.render_scale, Scale::HALF, "the flags apply");
        draw_as_photo_mode(&mut cli);
        cli.apply_render_overrides(&mut profile, Some(Scale::HALF));
        assert_eq!(profile.scale_for((1920.0, 1080.0), false), Scale::FULL);
        assert_eq!(profile.reconstruction, Reconstruction::Off);
    }

    #[test]
    fn a_photo_is_drawn_again_with_the_glass_target_at_the_originals_size() {
        use oag_display::display::ReflectionResolution;
        let mut cli = Cli::try_parse_from(["oag-game", "--reflection-resolution", "quarter"])
            .expect("parses");
        draw_as_photo_mode(&mut cli);
        assert_eq!(
            cli.render.reflection_resolution,
            Some(ReflectionResolution::Original)
        );
        let mut profile = crate::settings::RenderProfile {
            reflection_resolution: ReflectionResolution::Quarter,
            ..Default::default()
        };
        cli.apply_render_overrides(&mut profile, None);
        assert_eq!(
            profile.reflection_resolution,
            ReflectionResolution::Original
        );
    }

    #[test]
    fn a_photo_is_drawn_again_with_the_render_scale_and_anti_aliasing_it_was_drawn_with() {
        use oag_display::display::Reconstruction;
        let text = |scale: i64, reconstruction: &str| shot_meta::Meta {
            render: Some(toml::toml! {
                render_scale = scale
                reconstruction = reconstruction
            }),
            ..shot_meta::Meta::default()
        };
        let mut cli = Cli::try_parse_from([
            "oag-game",
            "--render-scale",
            "50",
            "--reconstruction",
            "fsr3",
        ])
        .expect("parses");
        draw_as_the_photo_was(&text(150, "smaa"), &mut cli);
        assert_eq!(cli.render.render_scale, Some(150));
        assert_eq!(cli.render.reconstruction, Some(Reconstruction::Smaa));
        draw_as_the_photo_was(&text(50, "off"), &mut cli);
        assert_eq!(
            cli.render.render_scale,
            Some(100),
            "a photo is never drawn below 100"
        );
        draw_as_the_photo_was(&shot_meta::Meta::default(), &mut cli);
        assert_eq!(cli.render.render_scale, Some(100), "no text, no change");
    }

    #[test]
    fn the_recorded_controls_are_drawn_again() {
        let mut cli = Cli::try_parse_from(["oag-game"]).expect("parses");
        let picture = shot_meta::Picture {
            exposure: 60,
            saturation: 20,
            ..shot_meta::Picture::default()
        };
        let lines = picture_lines(&meta("photo mode", Some(picture)), &mut cli);
        assert_eq!(
            cli.capture_view.picture.as_deref(),
            Some("exposure=60,saturation=20,speed_blur=0")
        );
        assert!(
            lines[0].starts_with("picture controls applied"),
            "{lines:?}"
        );
    }

    fn blurred(kind: &str, mount: Option<shot_meta::Mount>) -> shot_meta::Meta {
        let picture = shot_meta::Picture {
            motion_blur: kind.to_string(),
            shutter: Some(40),
            frames: 120,
            ..shot_meta::Picture::default()
        };
        let mut meta = meta("photo mode", Some(picture));
        meta.camera = Some(shot_meta::Camera {
            mount,
            ..shot_meta::Camera::default()
        });
        meta
    }

    #[test]
    fn a_motion_blur_is_drawn_from_its_frames_and_a_track_blur_needs_its_mount() {
        let mount = shot_meta::Mount {
            follow: "craft".to_string(),
            rotation: [0.0, 0.0, 0.0, 1.0],
            fov_deg: 65.0,
            ..shot_meta::Mount::default()
        };
        let (ship, line) = blurred_of(&blurred("ship", None));
        assert!(ship.is_some_and(|ship| ship.mount.is_none()));
        assert!(line.is_some_and(|line| line.contains("drawing its frames")));
        let (track, line) = blurred_of(&blurred("track", Some(mount)));
        assert!(track.is_some_and(|track| track.mount.is_some() && track.frames == 120));
        assert!(line.is_some_and(|line| line.contains("track motion blur")));
        let (old, line) = blurred_of(&blurred("track", None));
        assert!(old.is_none());
        assert!(line.is_some_and(|line| line.contains("predates the camera mount")));
        // A section that is partial, or names a rule this build lacks, is refused by what is wrong
        // with it: its defaults would put a NaN view on every frame.
        let partial = shot_meta::Mount {
            follow: "craft".to_string(),
            ..shot_meta::Mount::default()
        };
        let (refused, line) = blurred_of(&blurred("track", Some(partial)));
        assert!(refused.is_none());
        assert!(line.is_some_and(|line| line.contains("not a rotation")));
        let unknown = shot_meta::Mount {
            follow: "orbit".to_string(),
            rotation: [0.0, 0.0, 0.0, 1.0],
            fov_deg: 65.0,
            ..shot_meta::Mount::default()
        };
        let (refused, line) = blurred_of(&blurred("track", Some(unknown)));
        assert!(refused.is_none());
        assert!(line.is_some_and(|line| line.contains("`orbit` is not a camera rule")));
        for (rotation, fov_deg, said) in [
            ([f32::NAN, 0.0, 0.0, 1.0], 65.0, "not a rotation"),
            ([0.0, 0.0, 0.0, 1.0], 0.0, "not in (0, 180)"),
            ([0.0, 0.0, 0.0, 1.0], f32::NAN, "not in (0, 180)"),
        ] {
            let bad = shot_meta::Mount {
                follow: "craft".to_string(),
                rotation,
                fov_deg,
                ..shot_meta::Mount::default()
            };
            let (refused, line) = blurred_of(&blurred("track", Some(bad)));
            assert!(refused.is_none());
            assert!(line.is_some_and(|line| line.contains(said)), "{said}");
        }
        assert_eq!(blurred_of(&meta("photo mode", None)), (None, None));
    }

    #[test]
    fn a_photo_without_the_keys_says_so_and_another_source_says_nothing() {
        let mut cli = Cli::try_parse_from(["oag-game"]).expect("parses");
        let lines = picture_lines(&meta("photo mode", None), &mut cli);
        assert!(
            lines[0].contains("no picture controls recorded"),
            "{lines:?}"
        );
        assert_eq!(cli.capture_view.picture, None);
        assert!(picture_lines(&meta("screenshot", None), &mut cli).is_empty());
    }
}
