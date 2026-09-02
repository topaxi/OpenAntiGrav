//! Scratch probe: does 2048's `SkyCube/skycube.rcsmodel` decode through the
//! same generic `psp2::parse` the track and craft geometry already use, and
//! what does the decoded model look like (bounds, materials, texture names)?
//!
//! ```sh
//! cargo run -q --release -p oag-game --example vita_skycube_probe
//! ```

use oag_formats::rcsmodel::psp2;

const PACKAGE: &str = "data/extracted/vita/PCSF00007/base/PSP2/data.psarc";

const ENVIRONMENTS: [&str; 9] = [
    "altima",
    "arena",
    "bridge",
    "cathedral",
    "mall",
    "park",
    "sol",
    "subway",
    "tower",
];

fn main() -> anyhow::Result<()> {
    let mut archive = oag_assets::psarc::Archive::open(PACKAGE)?;
    for env in ENVIRONMENTS {
        let path = format!("data/art/published/environments/{env}/skycube.rcsmodel");
        let track_path = format!("data/art/published/environments/{env}/track.rcsmodel");
        let Ok(blob) = archive.read_path(&path) else {
            println!("{path}: not in the archive");
            continue;
        };
        let Ok(track_blob) = archive.read_path(&track_path) else {
            println!("{track_path}: not in the archive");
            continue;
        };
        let model = match psp2::parse(&blob) {
            Ok(m) => m,
            Err(e) => {
                println!("{path}: {e}");
                continue;
            }
        };
        let track = psp2::parse(&track_blob)?;

        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        let mut vertex_count = 0usize;
        let mut triangle_count = 0usize;
        for sub in &model.submeshes {
            vertex_count += sub.positions.len();
            triangle_count += sub.triangle_count();
            for p in &sub.positions {
                for a in 0..3 {
                    lo[a] = lo[a].min(p[a]);
                    hi[a] = hi[a].max(p[a]);
                }
            }
        }

        let mut track_lo = [f32::MAX; 3];
        let mut track_hi = [f32::MIN; 3];
        for sub in &track.submeshes {
            for p in &sub.positions {
                for a in 0..3 {
                    track_lo[a] = track_lo[a].min(p[a]);
                    track_hi[a] = track_hi[a].max(p[a]);
                }
            }
        }

        println!("\n=== {env} ===");
        println!(
            "  sky: {} submesh(es), {} material(s), {vertex_count} vertices, {triangle_count} triangles, {} unpaired pointer(s)",
            model.submeshes.len(),
            model.materials.len(),
            model.unpaired_pointers,
        );
        println!("  sky sections: {:?}", model.sections);
        println!("  sky file length: {}", blob.len());
        println!("  sky bounds: {lo:?} .. {hi:?}");
        println!("  track bounds: {track_lo:?} .. {track_hi:?}");
        for (i, material) in model.materials.iter().enumerate() {
            println!(
                "  material[{i}]: name={:?} textures={:?}",
                material.name, material.textures,
            );
        }
        for (i, sub) in model.submeshes.iter().enumerate() {
            println!(
                "  submesh[{i}]: {} vertices, {} indices, material={:?}",
                sub.positions.len(),
                sub.indices.len(),
                sub.material,
            );
        }
    }
    Ok(())
}
