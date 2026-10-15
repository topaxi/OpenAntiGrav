//! Writes each screen-filter preset's finished WGSL module to a directory.
//!
//!     cargo run -p oag-post --example screen_filters_wgsl -- <preset dir> <out dir>
//!
//! A preset is the prelude, a generated `param_*` block and the file's own body,
//! joined at run time by [`oag_post::screen::Preset::wgsl`], so no build script
//! ever sees the text the device is handed. `scripts/check-web-shaders.py` reads
//! this output to hand the same modules to a browser's WGSL compiler.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use oag_post::screen::Preset;

fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let (Some(presets), Some(out), None) = (args.next(), args.next(), args.next()) else {
        bail!("usage: screen_filters_wgsl <preset dir> <out dir>");
    };
    let (presets, out) = (PathBuf::from(presets), PathBuf::from(out));
    std::fs::create_dir_all(&out)?;
    let mut written = 0;
    for entry in std::fs::read_dir(&presets).with_context(|| presets.display().to_string())? {
        let path = entry?.path();
        if path.extension().is_none_or(|ext| ext != "wgsl") {
            continue;
        }
        let id = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .context("a preset file name that is not UTF-8")?;
        let preset = Preset::parse(id, &std::fs::read_to_string(&path)?)?;
        std::fs::write(out.join(format!("{id}.wgsl")), preset.wgsl())?;
        written += 1;
    }
    println!(
        "screen_filters_wgsl: wrote {written} presets to {}",
        out.display()
    );
    Ok(())
}
