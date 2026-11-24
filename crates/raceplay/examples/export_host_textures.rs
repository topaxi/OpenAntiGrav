//! Writes the textures a track borrows from a disc as PNG files, for the Blender add-on.
//!
//! `cargo run -p oag-raceplay --example export_host_textures -- <image> <what> <out-dir>`
//!
//! `<what>` is a `*.track.toml` (the host textures its art names, and its sky circuit's)
//! or a circuit's `.vex` entry such as `Data\Environments\03_Track\track.vex` (every
//! texture of that circuit). The files land under
//! `<out-dir>/<title slug>/<circuit entry>/<texture name>.png`, the layout the add-on's
//! "Host texture folder" reads (`docs/tools/blender.md`, "Host textures").
//!
//! **Local use only**: the output is content from the disc, so write it under `data/`
//! and never commit it.

use oag_raceplay as race;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [image, what, out] = args.as_slice() else {
        anyhow::bail!(
            "usage: export_host_textures <image> <track file or circuit entry> <out-dir>"
        );
    };
    let options = race::Options {
        source: image.clone(),
        ..race::Options::default()
    };
    let done = race::export_host_textures(&options, what, std::path::Path::new(out))?;
    for line in &done.skipped {
        eprintln!("skipped: {line}");
    }
    println!("{} texture(s) written under {out}", done.written.len());
    Ok(())
}
