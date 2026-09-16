//! Scratch probe: does `oag_render::mesh::rcs::psp2::build` actually paint a
//! real GXT onto a single-material 2048 model, end to end - not just
//! compile?
//!
//! Finds the first single-material `.rcsmodel` in the base package, builds
//! it with a real archive-backed texture closure, and reports what came out.
//!
//! ```sh
//! cargo run -q -p oag-game --example vita_rcsmodel_render_e2e_check
//! ```

use oag_rcs::rcsmodel::psp2;
use oag_render::mesh;

const BASE: &str = "data/extracted/vita/PCSF00007/base/PSP2/data.psarc";

fn main() -> anyhow::Result<()> {
    let mut archive = oag_assets::psarc::Archive::open(BASE)?;
    let mut entries: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".rcsmodel"))
        .cloned()
        .collect();
    entries.sort();

    for entry in entries {
        let Ok(blob) = archive.read_path(&entry) else {
            continue;
        };
        let Ok(decoded) = psp2::parse(&blob) else {
            continue;
        };
        if !decoded.has_one_material() || decoded.submeshes.is_empty() {
            continue;
        }
        let Some(diffuse) = decoded.materials[0].diffuse_texture() else {
            continue;
        };
        println!("candidate: {entry}");
        println!("  material: {}", decoded.materials[0].name);
        println!("  diffuse:  {diffuse}");
        println!("  archive.contains(diffuse): {}", archive.contains(diffuse));
        match archive.read_path(diffuse) {
            Ok(tex_blob) => {
                println!("  read_path ok, {} byte(s)", tex_blob.len());
                match oag_texture::gxt::Gxt::parse(&tex_blob) {
                    Ok(parsed) => {
                        println!("  gxt parses, {} texture(s)", parsed.textures.len());
                        match parsed.only() {
                            Some(t) => println!(
                                "  only(): {}x{}, to_rgba: {:?}",
                                t.width,
                                t.height,
                                t.to_rgba(&tex_blob).map(|v| v.len())
                            ),
                            None => println!("  only() is None (not exactly one texture)"),
                        }
                    }
                    Err(e) => println!("  gxt parse failed: {e}"),
                }
            }
            Err(e) => println!("  read_path failed: {e}"),
        }

        let (model, report) = mesh::rcs::psp2::build(&entry, &blob, None, &mut |path| {
            // 2048 stores paths lowercase-with-forward-slashes internally;
            // the material's own string uses the same spelling already.
            archive.read_path(path).ok()
        })?;
        println!("  report: {}", report.describe());
        println!("  model.textures: {}", model.textures.len());
        if let Some(Some(texture)) = model.textures.first() {
            println!(
                "  texture[0]: {}x{}, {} byte(s) of rgba",
                texture.width,
                texture.height,
                texture.rgba().map_or(0, <[u8]>::len)
            );
        }
        println!(
            "  draws with a texture bound: {}/{}",
            model.draws.iter().filter(|d| d.texture.is_some()).count(),
            model.draws.len()
        );
        if report.diffuse_texture.is_some() {
            return Ok(());
        }
    }
    println!("no single-material model with a diffuse texture found");
    Ok(())
}
