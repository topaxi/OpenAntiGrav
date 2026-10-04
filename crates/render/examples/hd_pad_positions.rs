//! Where a circuit's pads are, for aiming a `--camera-pose` at one: every
//! `Speedup Pad` and `Weapon Pad` node's vertex centre and mean vertex
//! normal, in world space (a pad chunk is baked there).
//!
//! `cargo run -p oag-render --example hd_pad_positions -- <archive> <vex>`
//!
//! For a PSP/PS2 Pulse archive (anything not ending `.PSARC`) it reads the
//! `.vex` straight out of the WAD and builds the pad models through
//! `mesh::build_pads`/`build_weapon_pads`, which is how a Pulse pad is aimed
//! at to check it did not move.

use oag_render::mesh;

fn main() -> anyhow::Result<()> {
    let spec = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA02.PSARC".into());
    let name = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "/data/environments/12_sol_2/track.vex".into());
    if !spec.ends_with(".PSARC") {
        let mut archive = oag_assets::Archive::open(&spec)?;
        let data = archive.read_name(&name)?;
        for label in ["Speedup Pad", "Weapon Pad"] {
            let model = if label == "Speedup Pad" {
                mesh::build_pads(&name, &data, None)?
            } else {
                mesh::build_weapon_pads(&name, &data, None)?
            };
            print_nodes(label, &model);
        }
        return Ok(());
    }
    let data = mesh::read_blob(&spec, &name)?;
    let model_blob = mesh::read_blob(&spec, &mesh::rcs::sibling_name(&name).unwrap())?;
    let mut read = |path: &str| mesh::read_blob(&spec, path).ok();
    for label in ["Speedup Pad", "Weapon Pad"] {
        let (model, _) = if label == "Speedup Pad" {
            mesh::rcs::build_pads(&name, &data, &model_blob, &mut read)?
        } else {
            mesh::rcs::build_weapon_pads(&name, &data, &model_blob, &mut read)?
        };
        print_nodes(label, &model);
    }
    Ok(())
}

fn print_nodes(label: &str, model: &mesh::Model) {
    for (i, range) in model.node_vertex_ranges.iter().enumerate() {
        let vs = &model.vertices[range.start as usize..range.end as usize];
        if vs.is_empty() {
            continue;
        }
        let n = vs.len() as f32;
        let mut c = [0.0f32; 3];
        let mut nn = [0.0f32; 3];
        for v in vs {
            for k in 0..3 {
                c[k] += v.position[k] / n;
                nn[k] += v.normal[k] / n;
            }
        }
        let l = (nn[0] * nn[0] + nn[1] * nn[1] + nn[2] * nn[2])
            .sqrt()
            .max(1e-9);
        println!(
            "{label} #{i}: centre {:.1},{:.1},{:.1} normal {:.3},{:.3},{:.3}",
            c[0],
            c[1],
            c[2],
            nn[0] / l,
            nn[1] / l,
            nn[2] / l
        );
    }
}
