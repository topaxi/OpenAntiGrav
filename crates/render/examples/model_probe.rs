//! Renders one model headlessly and prints a digest of the pixels, so a change
//! meant to be invisible can be **shown** to be invisible.
//!
//! The A/B harness for a refactor whose acceptance criterion is "zero changed
//! pixels". Run it before a change and after, on a track from each title, and
//! compare - the digest is over the raw RGBA the GPU produced, so any drift at
//! all moves it.
//!
//! ```sh
//! cargo run --release -p oag-render --example model_probe -- \
//!     data/images/pulse-psp-eu.chd:'PSP_GAME/USRDIR/DATA.WAD' 'Data\Track\01_Track.vex'
//! cargo run --release -p oag-render --example model_probe -- \
//!     data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC \
//!     /data/environments/talons_junction/track.vex out.png
//! ```
//!
//! A third argument writes the frame as a PNG as well, for looking at.
//!
//! **This replaces a `threshold_probe.rs` that no longer exists.** `mesh.wgsl`
//! cited it for the alpha-threshold measurement, but it was deleted after that
//! change landed and the citation outlived it - which is the failure this file
//! exists to stop repeating.

use oag_mesh::capture::make_opaque;
use oag_mesh::mesh;
use oag_mesh::mesh_render::{Anisotropy, capture_pixels_from};

/// Wide enough to show a track's structure, small enough to run in a second.
const WIDTH: u32 = 1024;
/// Matched to [`WIDTH`].
const HEIGHT: u32 = 1024;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args
        .next()
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC".into());
    let name = args
        .next()
        .unwrap_or_else(|| "/data/environments/talons_junction/track.vex".into());
    let out = args.next();

    // The HD path first: a `.vex` with a `.rcsmodel` beside it builds through
    // `rcs`, and anything else through the shared `.vex` builder.
    let data = mesh::read_blob(&spec, &name)?;
    let model = match mesh::rcs::scene_from(&spec, &name, &data)? {
        Some((model, report)) => {
            println!("{}", report.describe());
            model
        }
        None => mesh::build(&name, &data)?,
    };

    // Near `PI / 2` looks straight down, which is what a track wants -
    // `capture_from`'s own note. A model-shaped subject is fine at this angle
    // too; the probe's job is sensitivity to change, not a flattering view.
    let pixels = capture_pixels_from(&model, WIDTH, HEIGHT, 0.6, 1.35, Anisotropy::default(), 0.0)?;

    // A digest rather than a mean: a mean can be unchanged while pixels move,
    // and this exists to catch movement.
    let mut digest = 0xcbf2_9ce4_8422_2325u64;
    for b in &pixels {
        digest ^= u64::from(*b);
        digest = digest.wrapping_mul(0x1000_0000_01b3);
    }
    let lit = pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2]) > 100)
        .count();
    let mean = pixels
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| f64::from(p[0]) + f64::from(p[1]) + f64::from(p[2]))
        .sum::<f64>()
        / (pixels.len() / 4) as f64
        / 3.0
        / 255.0;

    println!("{name}");
    println!(
        "  {WIDTH}x{HEIGHT}  digest {digest:#018x}  lit {lit}  mean {mean:.5}  \
         {} draw(s), {} vertices",
        model.draws.len(),
        model.vertices.len()
    );

    if let Some(path) = out {
        // Scene geometry does not write alpha - see `capture::make_opaque` -
        // so a PNG straight off the readback is fully transparent.
        let mut rgba = pixels.clone();
        make_opaque(&mut rgba);
        std::fs::write(&path, oag_texture::png::encode_rgba(WIDTH, HEIGHT, &rgba))?;
        println!("  wrote {path}");
    }
    Ok(())
}
