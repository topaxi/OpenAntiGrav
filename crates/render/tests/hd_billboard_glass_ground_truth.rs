//! HD's lit billboard and Tech de Ra's stand glass reach the shader as the
//! programs they are (`oag_mesh::mesh::rcs::billboard_glass`), not as an
//! additive glow.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content. Run with
//! `just test-data`, or only this file:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'test(/^hd_billboard_glass_ground_truth::/)'
//! ```

use crate::archive_cache;

use oag_mesh::mesh::{self, PROGRAM_BILLBOARD, PROGRAM_BW_GLASS, slots};

fn build(archive: &str, path: &str) -> Option<(mesh::Model, mesh::rcs::Report)> {
    let image = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    let spec = format!("{}:PS3_GAME/USRDIR/{archive}", image.display());
    let data = archive_cache::read(&spec, path)?;
    let geometry = mesh::rcs::sibling_geometry(&spec, path, &data)?;
    Some(
        mesh::rcs::build_scene(path, &data, &geometry, &mut |name| {
            archive_cache::read(&spec, name)
        })
        .expect("the scene builds"),
    )
}

/// The glow entries of every vertex that keeps `ADD_SECOND` and names `mode`.
fn entries(model: &mesh::Model, mode: f32) -> Vec<mesh::Emissive> {
    let mut out: Vec<mesh::Emissive> = Vec::new();
    for v in &model.vertices {
        let index = slots::material_index(v.slots) as usize;
        if v.slots & slots::ADD_SECOND == 0 || index == 0 {
            continue;
        }
        let entry = model.emissive[index - 1];
        if entry.program[0] == mode && !out.contains(&entry) {
            out.push(entry);
        }
    }
    out
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn vineta_ks_billboards_multiply_their_glow_with_the_authored_scales() {
    let Some((model, report)) = build("DATA02.PSARC", "/data/environments/01_vineta_k/track.vex")
    else {
        return;
    };
    assert!(report.billboard_multiplied > 0, "{}", report.describe());
    let layers = entries(&model, PROGRAM_BILLBOARD);
    // The harimau card (slot 330): u scale 1.5, v scale 1.0095, k 1.
    assert!(
        layers
            .iter()
            .any(|e| e.program[1] == 1.5 && e.scale == 1.0095 && e.tint == [1.0; 3]),
        "{layers:?}"
    );
    // A different board authors a different u scale (4.0): the scales are read, not typed.
    assert!(layers.iter().any(|e| e.program[1] == 4.0), "{layers:?}");
    assert!(layers.iter().all(|e| e.rate == 0.0), "no clock on a card");
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn tech_de_ras_stand_glass_multiplies_bwgradient_read_at_the_second_uvs() {
    let Some((model, report)) = build("DATA00.PSARC", "/data/environments/tech_de_ra/track.vex")
    else {
        return;
    };
    assert!(report.glass_bw_multiplied > 0, "{}", report.describe());
    assert_eq!(entries(&model, PROGRAM_BW_GLASS).len(), 1);
    let glass: Vec<_> = model
        .vertices
        .iter()
        .filter(|v| {
            let i = slots::material_index(v.slots) as usize;
            v.slots & slots::ADD_SECOND != 0
                && i > 0
                && model.emissive[i - 1].program[0] == PROGRAM_BW_GLASS
        })
        .collect();
    assert!(!glass.is_empty());
    eprintln!(
        "{} {:?} {:?}",
        glass.len(),
        glass[0].texcoord2,
        glass[glass.len() / 2].texcoord2
    );
    assert!(
        glass.iter().any(|v| v.texcoord2 != [0.0, 0.0]),
        "Uv2 reached the vertices"
    );
}
