//! Counts, per circuit model, the submeshes whose vertex declaration names `uv1_uvDirt` and no
//! `Uv1` (the loader's diffuse-coordinate fallback then takes the first half pair), and the
//! materials they draw with.
//!
//! ```sh
//! cargo run -p oag-rcs --example psp2_uv_census -- <archive.psarc>...
//! ```

use std::collections::BTreeMap;
use std::path::Path;

use oag_rcs::rcsmaterial::name_hash;
use oag_rcs::rcsmodel::psp2::{self, vertex_decl};

fn main() {
    let archives: Vec<String> = std::env::args().skip(1).collect();
    let uv_dirt = name_hash("uv1_uvDirt");
    let mut totals: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for a in &archives {
        let Ok(mut archive) = oag_assets::psarc::Archive::open_file(Path::new(a)) else {
            continue;
        };
        let paths: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| {
                p.ends_with("track.final.rcsmodel") || p.ends_with("track_reversed.final.rcsmodel")
            })
            .cloned()
            .collect();
        for path in paths {
            let blob = archive.read_path(&path).expect("reads");
            let model = psp2::parse(&blob).expect("parses");
            let cpu = model.sections[0];
            let cpu_bytes = &blob[cpu.at..cpu.at + cpu.len];
            let decls = vertex_decl::find_by_header(cpu_bytes);
            let mut hit = 0;
            let mut tris = 0;
            let mut materials: BTreeMap<String, usize> = BTreeMap::new();
            for s in &model.submeshes {
                let at = s.record + vertex_decl::PS4_DECLARATION_POINTER;
                let Some(word) = cpu_bytes.get(at..at + 8) else {
                    continue;
                };
                let target = u64::from_le_bytes(word.try_into().unwrap()) as usize;
                let Some(decl) = decls.get(&target).filter(|d| d.stride == s.stride) else {
                    continue;
                };
                if decl.attribute(uv_dirt).is_some() && decl.diffuse_texcoord().is_none() {
                    hit += 1;
                    tris += s.triangle_count();
                    let m = s.material.map_or("-".to_string(), |m| {
                        model.materials[m]
                            .name
                            .rsplit('/')
                            .next()
                            .unwrap_or("")
                            .to_string()
                    });
                    *materials.entry(m).or_default() += 1;
                }
            }
            println!(
                "{path}: {} submesh(es), {hit} with uv1_uvDirt and no Uv1, {tris} tri(s) {materials:?}",
                model.submeshes.len()
            );
            let e = totals.entry(path).or_default();
            e.0 += hit;
            e.1 += tris;
        }
    }
    let all: usize = totals.values().map(|t| t.0).sum();
    println!("total {all} submesh(es) over {} model(s)", totals.len());
}
