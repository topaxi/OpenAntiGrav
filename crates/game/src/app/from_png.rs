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
    cli.picture = Some(picture.to_flag());
    let mut lines = vec![format!("picture controls applied: {}", picture.to_flag())];
    if !picture.motion_blur.is_empty() {
        lines.push(format!(
            "the shot is a {} motion blur (shutter {}, {} frames); this re-render is the one frame",
            picture.motion_blur,
            picture.shutter.unwrap_or_default(),
            picture.frames
        ));
    }
    lines
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
pub(crate) fn prepare(cli: &mut Cli, path: &Path) -> Result<Arc<ShotRecord>> {
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
    cli.ticks = u32::try_from(record.tick).context("the shot's tick does not fit")?;
    if cli.screenshot.is_none() {
        let stem = path
            .file_stem()
            .map_or_else(|| "shot".into(), |s| s.to_string_lossy().into_owned());
        cli.screenshot = Some(format!("{stem}-rerender.png").into());
    }
    // Photo mode draws every section whatever the circuit's visibility sets say.
    if meta.shot.source == "photo mode" {
        cli.pvs = Some(false);
    }
    if meta.shot.width > 0 && meta.shot.height > 0 {
        cli.size = format!("{}x{}", meta.shot.width, meta.shot.height);
    }
    if let Some(camera) = &meta.camera {
        let [ex, ey, ez] = camera.eye;
        let [fx, fy, fz] = camera.forward;
        let [ux, uy, uz] = camera.up;
        cli.camera_pose = Some(format!("{ex},{ey},{ez},{fx},{fy},{fz},{ux},{uy},{uz}"));
        cli.camera_fov = camera.fov_deg;
    }
    for line in picture_lines(&meta, cli) {
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
    Ok(Arc::new(record))
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
    fn the_recorded_controls_are_drawn_again_and_a_blur_is_said_not_to_be() {
        let mut cli = Cli::try_parse_from(["oag-game"]).expect("parses");
        let picture = shot_meta::Picture {
            exposure: 60,
            saturation: 20,
            motion_blur: "track".to_string(),
            shutter: Some(40),
            frames: 120,
            ..shot_meta::Picture::default()
        };
        let lines = picture_lines(&meta("photo mode", Some(picture)), &mut cli);
        assert_eq!(
            cli.picture.as_deref(),
            Some("exposure=60,saturation=20,speed_blur=0")
        );
        assert!(
            lines[0].starts_with("picture controls applied"),
            "{lines:?}"
        );
        assert!(
            lines.iter().any(|line| line.contains("track motion blur")),
            "{lines:?}"
        );
    }

    #[test]
    fn a_photo_without_the_keys_says_so_and_another_source_says_nothing() {
        let mut cli = Cli::try_parse_from(["oag-game"]).expect("parses");
        let lines = picture_lines(&meta("photo mode", None), &mut cli);
        assert!(
            lines[0].contains("no picture controls recorded"),
            "{lines:?}"
        );
        assert_eq!(cli.picture, None);
        assert!(picture_lines(&meta("screenshot", None), &mut cli).is_empty());
    }
}
