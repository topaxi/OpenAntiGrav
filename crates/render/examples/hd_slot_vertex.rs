//! Scratch probe: the resolved vertex program of one material slot - which
//! attribute registers it declares and every instruction that writes a
//! texture-coordinate output, so a flip the reading misses can be seen.
//!
//! ```sh
//! cargo run -p oag-render --example hd_slot_vertex -- <image> /data/environments/amphiseum/track.vex 501
//! ```

use oag_assets::Container;
use oag_mesh::mesh;
use oag_rcs::{rcsmaterial, rcsmodel};

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let image = args.next().unwrap();
    let name = args.next().unwrap();
    let slot: usize = args.next().unwrap().parse()?;
    let spec = format!("{image}:PS3_GAME/USRDIR/DATA00.PSARC");
    let data = mesh::read_blob(&spec, &name)?;
    let geometry = mesh::rcs::sibling_geometry(&spec, &name, &data)
        .ok_or_else(|| anyhow::anyhow!("no sibling model"))?;
    let model = rcsmodel::Model::parse(&geometry)?;
    let mut container = Container::open(&spec)?;
    let decl = model
        .meshes
        .iter()
        .flat_map(rcsmodel::Mesh::surfaces)
        .find(|m| m.material as usize == slot)
        .and_then(|m| m.decl.clone())
        .ok_or_else(|| anyhow::anyhow!("no chunk"))?;
    let material = &model.materials[slot];
    println!("slot {slot}: {}", material.name);
    let blob = container.read_entry(&format!("/{}", material.name))?;
    let parsed = rcsmaterial::RcsMaterial::parse(&blob)?;
    let word = rcsmaterial::Features::chunk_word(rcsmaterial::LIT_RACE_PASS, Some(&decl));
    let key = rcsmaterial::Features::from_pass_word(word);
    let variant = parsed
        .variant(rcsmaterial::Class::Static, key)
        .ok_or_else(|| anyhow::anyhow!("no variant"))?;
    println!(
        "variant vertex {:#x} fragment {:#x}",
        variant.vertex.offset, variant.fragment.offset
    );
    let program = rcsmaterial::vertex::Program::of(&blob, variant.vertex)
        .ok_or_else(|| anyhow::anyhow!("vertex program does not parse"))?;
    println!("attributes (hash, register): {:x?}", program.attributes);
    println!(
        "diffuse attribute {:#x?}",
        decl.diffuse_texcoord().map(|a| a.name_hash)
    );
    for (k, i) in program.instructions.iter().enumerate() {
        println!(
            "  {k:>3} vec {:#04x} input {} out {} dest {:>2} src {:05x?}",
            i.vec_op, i.input, i.dest_is_output, i.dest, i.sources
        );
    }
    Ok(())
}
