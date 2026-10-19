//! Census of re-cooked content: one TSV row per `.rcsmodel` and per texture
//! under every archive path that contains `<needle>` (case-insensitive), so a
//! circuit or hull can be set beside its copy on another title.
//!
//! Rows, tab-separated:
//! - `model <archive> <path> <meshes> <submeshes> <vertices> <triangles> <materials>`
//!   where `<meshes>` is the `.rcsmodel` section count on Vita and PS4 and the
//!   mesh count (extra surfaces included) on HD.
//! - `texture <archive> <path> <count> <width> <height> <mips> <format>`, from
//!   the first texture of a `.gxt`, `.gtf` or `.gnf` container.
//! - `refused <archive> <path> <error>`
//!
//! The model reader is picked by the file's own magic: `0xca5caded` is the
//! Vita/PS4 `RCSMODEL`, anything else goes to HD's reader. `<platform>` is a
//! label kept for the caller's own bookkeeping and selects nothing.
//!
//! `cargo run -p oag-tools --example recook_census -- <platform> <needle> <archive>...`

use oag_rcs::rcsmodel;
use oag_rcs::rcsmodel::psp2;
use oag_texture::{gnf, gtf, gxt};

fn main() {
    let mut args = std::env::args().skip(1);
    let platform = args
        .next()
        .expect("usage: recook_census <platform> <needle> <archive>...");
    let needle = args
        .next()
        .expect("usage: recook_census <platform> <needle> <archive>...")
        .to_lowercase();
    for archive_path in args {
        let mut archive = match oag_assets::psarc::Archive::open(&archive_path) {
            Ok(archive) => archive,
            Err(e) => {
                println!("refused\t{platform}:{archive_path}\t-\t{e}");
                continue;
            }
        };
        let label = std::path::Path::new(&archive_path)
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let paths: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.to_lowercase().contains(&needle))
            .cloned()
            .collect();
        for path in paths {
            let lower = path.to_lowercase();
            let blob = match archive.read_path(&path) {
                Ok(blob) => blob,
                Err(e) => {
                    println!("refused\t{label}\t{path}\t{e}");
                    continue;
                }
            };
            if lower.ends_with(".rcsmodel") {
                census_model(&label, &path, &blob);
            } else if lower.ends_with(".gxt") {
                match gxt::Gxt::parse(&blob) {
                    Ok(g) => census_texture(
                        &label,
                        &path,
                        g.textures.len(),
                        g.textures.first().map(|t| {
                            (
                                u32::from(t.width),
                                u32::from(t.height),
                                t.mip_levels,
                                format!("{:?}", t.format()),
                            )
                        }),
                    ),
                    Err(e) => println!("refused\t{label}\t{path}\t{e}"),
                }
            } else if lower.ends_with(".gtf") {
                match gtf::Gtf::parse(&blob) {
                    Ok(g) => census_texture(
                        &label,
                        &path,
                        g.textures.len(),
                        g.textures.first().map(|t| {
                            (
                                u32::from(t.width),
                                u32::from(t.height),
                                t.mip_levels,
                                format!("{:?}", t.format),
                            )
                        }),
                    ),
                    Err(e) => println!("refused\t{label}\t{path}\t{e}"),
                }
            } else if lower.ends_with(".gnf") {
                match gnf::Texture::parse(&blob) {
                    Ok(t) => {
                        let mips = t.last_mip_level.saturating_add(1);
                        census_texture(
                            &label,
                            &path,
                            1,
                            Some((t.width, t.height, mips, format!("{:?}", t.surface_format))),
                        );
                    }
                    Err(e) => println!("refused\t{label}\t{path}\t{e}"),
                }
            }
        }
    }
}

fn census_model(label: &str, path: &str, blob: &[u8]) {
    if blob.len() >= 4 && blob[..4] == psp2::MAGIC.to_le_bytes() {
        match psp2::parse(blob) {
            Ok(model) => {
                let vertices: usize = model.submeshes.iter().map(|s| s.positions.len()).sum();
                let triangles: usize = model.submeshes.iter().map(|s| s.indices.len() / 3).sum();
                println!(
                    "model\t{label}\t{path}\t{}\t{}\t{vertices}\t{triangles}\t{}",
                    model.sections.len(),
                    model.submeshes.len(),
                    model.materials.len()
                );
            }
            Err(e) => println!("refused\t{label}\t{path}\t{e}"),
        }
        return;
    }
    match rcsmodel::Model::parse(blob) {
        Ok(model) => {
            let (mut meshes, mut submeshes, mut vertices, mut triangles) = (0, 0, 0, 0);
            let mut stack: Vec<&rcsmodel::Mesh> = model.meshes.iter().collect();
            while let Some(mesh) = stack.pop() {
                meshes += 1;
                for s in &mesh.submeshes {
                    submeshes += 1;
                    vertices += s.vertex_count;
                    triangles += s.index_count / 3;
                }
                stack.extend(mesh.extra_surfaces.iter());
            }
            println!(
                "model\t{label}\t{path}\t{meshes}\t{submeshes}\t{vertices}\t{triangles}\t{}",
                model.materials.len()
            );
        }
        Err(e) => println!("refused\t{label}\t{path}\t{e}"),
    }
}

fn census_texture(label: &str, path: &str, count: usize, first: Option<(u32, u32, u8, String)>) {
    match first {
        Some((w, h, mips, format)) => {
            println!("texture\t{label}\t{path}\t{count}\t{w}\t{h}\t{mips}\t{format}");
        }
        None => println!("refused\t{label}\t{path}\tno texture in container"),
    }
}
