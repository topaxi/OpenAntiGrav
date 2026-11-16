//! Every vertex of a built Pulse circuit carries its batch's authored depth bias.
//!
//! **`#[ignore]`d and never run in CI.** It needs a Pulse disc image in `data/images`. Run it
//! with `just test-data`.
//!
//! `13_Track` authors glows, light cones and trees with a bias of 1 to 9
//! (`docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`, "The per-batch depth bias is file
//! data"). The model's vertices are checked against a second walk over the file's own batches:
//! the biased ones are found, each one's vertices carry `Batch::depth_bias` and no other
//! vertex carries anything. Remove the copy in `build_class` and the first check fails.

use oag_mesh::mesh::{self, batch_placements};
use oag_vex::vex;

#[test]
#[ignore = "needs a Pulse disc image in data/images"]
fn a_circuits_vertices_carry_their_batchs_authored_depth_bias() {
    let Some(image) = oag_testdata::image("pulse-psp-eu.chd") else {
        return;
    };
    let mut archive =
        oag_assets::Archive::open(&format!("{}:PSP_GAME/USRDIR/Data.wad", image.display()))
            .expect("opening the archive");
    let name = r"Data\Environments\13_Track\track.vex";
    let data = archive.read_name(name).expect("the .vex reads");
    let model = mesh::build_with_textures(name, &data, None).expect("builds");
    let placements = batch_placements(&data, |c| c.mesh).expect("placements");

    let nodes = vex::nodes(&data).expect("nodes");
    let class = vex::classes_of(&data)
        .expect("classes")
        .mesh
        .expect("mesh class");
    let mut biases = Vec::new();
    for node in nodes.iter().filter(|n| n.class_id == class) {
        let payload = &data[node.payload()];
        for list in [0u8, 1] {
            for batch in vex::mesh_batches(payload, list).expect("batches") {
                biases.push(batch.depth_bias);
            }
        }
    }
    assert_eq!(biases.len(), placements.len(), "the same walk");

    let mut biased_vertices = 0usize;
    for (bias, placement) in biases.iter().zip(&placements) {
        for vertex in
            &model.vertices[placement.vertices.start as usize..placement.vertices.end as usize]
        {
            assert!(
                (vertex.depth_bias - f32::from(*bias)).abs() < f32::EPSILON,
                "a vertex of a batch authored with bias {bias} carries {}",
                vertex.depth_bias
            );
            biased_vertices += usize::from(*bias > 0);
        }
    }
    println!(
        "{} batches, {} biased, {biased_vertices} biased vertices",
        biases.len(),
        biases.iter().filter(|b| **b > 0).count()
    );
    assert!(biased_vertices > 0, "13_Track authors biased batches");
}
