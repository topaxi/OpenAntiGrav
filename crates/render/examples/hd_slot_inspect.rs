//! What a material slot binds and what its chunks declare: the sampler hashes
//! with their paths, and per vertex-attribute name how many chunks carry it
//! (`lightmapUV`, the colour sets). The join key for an `OAG_TINT_MATERIALS`
//! frame read against `hd_slot_list`.
//!
//! ```sh
//! cargo run -p oag-render --example hd_slot_inspect -- <image>:<archive> /data/environments/15_anulpha_pass/track.vex 476,442
//! ```

use oag_mesh::mesh;
use oag_rcs::rcsmodel;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args.next().expect("<image>:<archive>");
    let name = args.next().expect("track .vex path");
    let slots: Vec<u32> = args
        .next()
        .expect("slots")
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    let data = mesh::read_blob(&spec, &name)?;
    let geometry = mesh::rcs::sibling_geometry(&spec, &name, &data)
        .ok_or_else(|| anyhow::anyhow!("no sibling .rcsmodel"))?;
    let model = rcsmodel::Model::parse(&geometry)?;
    for slot in slots {
        let Some(m) = model.materials.get(slot as usize) else {
            println!("slot {slot}: none");
            continue;
        };
        println!("slot {slot} {} state {:#x}", m.name, m.state);
        println!(
            "  texture {:?} sampler {:#010x}; second {:?} sampler {:?}",
            m.texture, m.texture_sampler, m.second_texture, m.second_texture_sampler
        );
        for (h, p) in &m.samplers {
            println!("  sampler {h:#010x} -> {p:?}");
        }
        let mut attrs: std::collections::BTreeMap<String, usize> = Default::default();
        let mut chunks = 0;
        for c in model.meshes.iter().filter(|c| c.material == slot) {
            chunks += 1;
            if let Some(d) = c.decl.as_ref() {
                for a in &d.attributes {
                    let n = a
                        .name()
                        .map_or_else(|| format!("{:#010x}", a.name_hash), str::to_string);
                    *attrs
                        .entry(format!("{n}/{}x{}", a.components, a.rsx_type))
                        .or_default() += 1;
                }
            }
        }
        println!("  {chunks} chunk(s); attrs {attrs:?}");
    }
    Ok(())
}
