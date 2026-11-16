//! `--png-info` and `--from-png`: a screenshot of ours read back (`docs/formats/shot-png.md`).
//!
//! `--png-info` prints the `oag` text chunk and a line on the stream. `--from-png` turns the run
//! into the `--race --screenshot` route the shot could have been made by: the race is the
//! recorded one (`Options::apply_replay`), the capture drives it with the recorded inputs to the
//! shot's tick instead of a held button (`CaptureOptions::restage`), checks the state hash there
//! and draws the frame from the camera the text names.

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use oag_game::shot_meta;
use oag_replay::ShotRecord;

use crate::cli::Cli;

fn read(path: &Path) -> Result<shot_meta::Found> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    shot_meta::read(&bytes).with_context(|| format!("reading {}", path.display()))
}

/// Prints what `path` carries to stdout.
pub(crate) fn info(path: &Path) -> Result<()> {
    let found = read(path)?;
    match &found.text {
        Some(text) => print!("{text}"),
        None => println!("{}: no `oag` text chunk; not a screenshot of ours", path.display()),
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
        let stem = path.file_stem().map_or_else(|| "shot".into(), |s| s.to_string_lossy().into_owned());
        cli.screenshot = Some(format!("{stem}-rerender.png").into());
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
