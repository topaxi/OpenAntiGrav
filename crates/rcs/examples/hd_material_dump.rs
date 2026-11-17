//! Lists the materials an HD (PS3) `.rcsmodel` uses, in the line shape of
//! `psp2_material_dump`, so one filter reads both lineages.
//!
//! ```sh
//! cargo run -p oag-rcs --example hd_material_dump -- <entry> <archive.psarc>...
//! ```
//!
//! For each material a chunk draws with: slot, path, state word, chunk and
//! triangle counts, the textures by sampler hash and the authored parameters,
//! after one line counting the chunks that set each render flag bit and, for
//! bits 1 to 3, the materials of the chunks that set it.
//! `docs/rendering/water.md`'s HD census is this output filtered by name.

use std::collections::BTreeMap;
use std::path::Path;

use oag_rcs::rcsmodel::Model;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [entry, archives @ ..] = args.as_slice() else {
        eprintln!("usage: hd_material_dump <entry> <archive.psarc>...");
        std::process::exit(2);
    };
    let blob = archives
        .iter()
        .find_map(|a| {
            oag_assets::psarc::Archive::open_file(Path::new(a))
                .ok()?
                .read_path(entry)
                .ok()
        })
        .expect("entry in an archive");
    let model = Model::parse(&blob).expect("parses");
    let mut use_of: BTreeMap<u32, (usize, usize)> = BTreeMap::new();
    for mesh in &model.meshes {
        let e = use_of.entry(mesh.material).or_default();
        e.0 += 1;
        e.1 += mesh
            .submeshes
            .iter()
            .map(|s| s.index_count / 3)
            .sum::<usize>();
    }
    let mut bits = [0usize; 16];
    for mesh in &model.meshes {
        for (bit, count) in bits.iter_mut().enumerate() {
            if mesh.render_flags & (1 << bit) != 0 {
                *count += 1;
            }
        }
    }
    println!(
        "render flag bits 0..16 over {} chunks: {bits:?}",
        model.meshes.len()
    );
    for bit in 1..4u16 {
        let mut by_material: BTreeMap<&str, usize> = BTreeMap::new();
        for mesh in model
            .meshes
            .iter()
            .filter(|m| m.render_flags & (1 << bit) != 0)
        {
            let name = model
                .material_of(mesh)
                .map_or("?", |m| m.name.rsplit('/').next().unwrap_or(&m.name));
            *by_material.entry(name).or_default() += 1;
        }
        if !by_material.is_empty() {
            println!("render flag bit {bit} by material: {by_material:?}");
        }
    }
    for (slot, (chunks, tris)) in &use_of {
        let Some(mat) = model.materials.get(*slot as usize) else {
            continue;
        };
        println!(
            "#{slot} {} | state {:#x} | {chunks} submesh(es) {tris} tri(s)",
            mat.name, mat.state
        );
        let textures: Vec<&str> = mat
            .samplers
            .iter()
            .filter_map(|(_, path)| path.as_deref())
            .collect();
        println!("    textures {textures:?}");
        println!("    samplers {:08x?}", mat.samplers);
        for p in &mat.parameters {
            println!("    param {p:x?}");
        }
    }
}
