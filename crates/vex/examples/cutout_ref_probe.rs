//! Scratch probe: which alpha-test reference one named `.vex` entry's batches
//! get, and what their textures' alpha actually looks like.
//!
//! The companion to `alpha_ref_census`, for the single files the docs name as
//! counterexamples - Wipeout Pure's `Speedup Pad` glow above all.
//!
//! ```sh
//! cargo run -q -p oag-vex --example cutout_ref_probe -- \
//!     'data/images/pure-psp-usa.chd:PSP_GAME/USRDIR/Data.wad' \
//!     'Data\Environments\01_Vineta_K\track.vex'
//! ```

use std::collections::BTreeMap;

use oag_vex::vex;

fn bucket(pass_mask: u16, header_flags: u8) -> &'static str {
    if header_flags & 0x10 != 0 {
        "no-test (header_flags & 0x10)"
    } else if pass_mask & 0x0700 != 0 {
        "ref 0 (transparent)"
    } else if pass_mask & 0x0800 == 0 {
        "no-test (ALWAYS)"
    } else if header_flags & 0x20 != 0 {
        "ref 0x10"
    } else if pass_mask & 0x0080 != 0 {
        "ref 0"
    } else {
        "ref 0x7f"
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let spec = std::env::args().nth(1).expect("image:archive spec");
    let mut archive = oag_assets::Archive::open(&spec)?;

    for entry in std::env::args().skip(2) {
        println!("== {entry} ==");
        let blob = match archive.read_name(&entry) {
            Ok(b) => b,
            Err(why) => {
                println!("  unavailable: {why}");
                continue;
            }
        };
        println!("  {} bytes, version {:?}", blob.len(), vex::version(&blob));

        #[allow(clippy::type_complexity)]
        let tex_alpha: Vec<Option<(BTreeMap<u8, u64>, Option<String>)>> = vex::textures(&blob)
            .map(|ts| {
                ts.into_iter()
                    .map(|t| {
                        t.map(|t| {
                            let rgba = t.to_rgba();
                            let mut h: BTreeMap<u8, u64> = BTreeMap::new();
                            for texel in rgba.as_chunks::<4>().0 {
                                *h.entry(texel[3]).or_default() += 1;
                            }
                            (h, t.asset_path.clone())
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let nodes = match vex::nodes(&blob) {
            Ok(n) => n,
            Err(why) => {
                println!("  nodes: {why}");
                continue;
            }
        };
        let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
        let mut cutout_textures: BTreeMap<(&'static str, String), (u64, u64)> = BTreeMap::new();
        let mut meshes = 0usize;
        let mut failures = 0usize;
        let mesh_class = vex::classes_of(&blob)
            .ok()
            .and_then(|c| c.mesh)
            .unwrap_or(vex::CLASS_MESH);
        println!("  mesh class {mesh_class:#x}");
        for node in vex::nodes_by_class(&nodes, mesh_class) {
            let payload = &blob[node.payload()];
            meshes += 1;
            let materials = vex::mesh_materials(payload);
            for list in 0..2 {
                let batches = match vex::mesh_batches(payload, list) {
                    Ok(b) => b,
                    Err(_) => {
                        failures += 1;
                        continue;
                    }
                };
                for b in &batches {
                    let k = bucket(b.pass_mask, b.header_flags);
                    *counts.entry(k).or_default() += 1;
                    let refv = match k {
                        "ref 0x7f" => 0x7fu8,
                        "ref 0x10" => 0x10,
                        _ => 0,
                    };
                    let Some(Some((hist, name))) = materials
                        .get(usize::from(b.material_index))
                        .copied()
                        .flatten()
                        .map(|m| m.texture as usize)
                        .and_then(|t| tex_alpha.get(t))
                    else {
                        continue;
                    };
                    let drawn: u64 = hist.iter().filter(|(a, _)| **a > 0).map(|(_, n)| n).sum();
                    let kept: u64 = hist
                        .iter()
                        .filter(|(a, _)| **a > refv)
                        .map(|(_, n)| n)
                        .sum();
                    let e = cutout_textures
                        .entry((k, name.clone().unwrap_or_else(|| "<unnamed>".into())))
                        .or_default();
                    e.0 += drawn;
                    e.1 += drawn - kept;
                }
            }
        }
        println!("  meshes {meshes}, batch-list failures {failures}");
        for (k, v) in &counts {
            println!("   {v:>6}  {k}");
        }
        println!("  textures by bucket (drawn texels -> newly discarded at the recovered ref):");
        for ((k, name), (drawn, discarded)) in &cutout_textures {
            println!("   {k:>20}  {name}: {drawn} -> {discarded} discarded");
        }
    }
    Ok(())
}
