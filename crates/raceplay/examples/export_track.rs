//! Writes a disc circuit as a track in this project's own format.
//!
//! `cargo run -p oag-raceplay --example export_track -- <image> <track name> <out.track.toml>`
//!
//! **Local use only**: the output is content from the disc, so write it under
//! `data/` and never commit it. `just audit-leakage` does not catch a `.glb`
//! named innocently; the rule is the maintainer's, not the script's.

use oag_raceplay as race;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [image, track, out] = args.as_slice() else {
        anyhow::bail!("usage: export_track <image> <track name> <out.track.toml>");
    };
    if !out.ends_with(".track.toml") {
        anyhow::bail!("the output must be named *.track.toml, the glb is written beside it");
    }
    let options = race::Options {
        source: image.clone(),
        track: Some(track.clone()),
        ..race::Options::default()
    };
    let exported = race::export_track(&options)?;
    println!(
        "{}: {} path(s), {} collision node(s), {} + {} pad(s)",
        exported.id,
        exported.ai.paths.len(),
        exported.nodes.len(),
        exported.speedup_pads.len(),
        exported.weapon_pads.len()
    );
    oag_trackfile::write(&exported, std::path::Path::new(out))?;
    Ok(())
}
