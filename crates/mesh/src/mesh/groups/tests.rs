use super::*;

/// A node with just the fields the group walk reads.
fn node(name: Option<&str>, parent: Option<usize>, depth: usize) -> vex::Node {
    vex::Node {
        class_id: 0,
        offset: 0,
        header_size: 0,
        data_size: 0,
        child_count: 0,
        unk_0x0e: 0,
        name: name.map(str::to_string),
        depth,
        parent,
    }
}

/// Wipeout HD's own flare tree, to the depth that decides the split.
fn flare_tree() -> Vec<vex::Node> {
    vec![
        node(Some("world"), None, 0),
        node(Some("root"), Some(0), 1),
        node(Some("EF_Boost"), Some(1), 2),
        node(Some("Joint_BoostLeft"), Some(2), 3),
        node(Some("ef_BoostLeftShape"), Some(3), 4),
        node(Some("EF_Main"), Some(1), 2),
        node(Some("ef_OuterShape"), Some(5), 3),
    ]
}

/// A shape two levels under a group still belongs to it: `ef_BoostLeftShape`
/// hangs off `Joint_BoostLeft`, not off `EF_Boost` directly, and a walk that
/// only looked at the immediate parent would drop half the plume.
#[test]
fn a_grandchild_belongs_to_its_group() {
    let owner = owners(&flare_tree(), &["EF_Main", "EF_Boost"]);
    assert_eq!(owner[4], Some(1), "ef_BoostLeftShape is EF_Boost's");
    assert_eq!(owner[6], Some(0), "ef_OuterShape is EF_Main's");
}

/// Everything above the groups belongs to neither, so geometry authored
/// outside them is left undrawn rather than folded into whichever group came
/// first.
#[test]
fn nodes_above_the_groups_belong_to_none() {
    let owner = owners(&flare_tree(), &["EF_Main", "EF_Boost"]);
    assert_eq!(owner[0], None);
    assert_eq!(owner[1], None);
}

/// A model with one draw per node index, so the filter can be checked without
/// a `.vex` or a GPU.
fn model_with_nodes(nodes: &[u32]) -> Model {
    let mut model = Model::none("flare");
    for (i, &n) in nodes.iter().enumerate() {
        let start = u32::try_from(i * 3).unwrap();
        model.vertices.extend((0..3).map(|k| GpuVertex {
            position: [n as f32, k as f32, 0.0],
            normal: [0.0, 1.0, 0.0],
            colour: [1.0; 4],
            texcoord: [0.0; 2],
            lit: 1.0,
            anim: 0,
            lightmap_texcoord: [0.0; 2],
            xform: 0,
            sun_mask: 1.0,
            slots: 0,
            specular_exponent: crate::mesh::DEFAULT_SPECULAR_EXPONENT,
            glow: 0.0,
            texcoord2: [0.0, 0.0],
        }));
        model.indices.extend(start..start + 3);
        model.draws.push(DrawCall {
            moving: false,
            chunk: None,
            alpha_test_ref: None,
            range: start..start + 3,
            texture: None,
            bounds: Bounds {
                centre: [0.0; 3],
                radius: 0.0,
            },
            culled: false,
            blend: None,
            blend_state: None,
            layer: 0,
            node: Some(n),
        });
    }
    model
}

/// The split keeps each group's own draws and nothing else, and both parts
/// still index the one shared buffer pair.
#[test]
fn each_part_keeps_only_its_own_draws() {
    let owner = owners(&flare_tree(), &["EF_Main", "EF_Boost"]);
    let model = model_with_nodes(&[4, 6, 1]);
    let main = part(&model, &owner, 0, "EF_Main");
    let boost = part(&model, &owner, 1, "EF_Boost");
    assert_eq!(main.draws.len(), 1);
    assert_eq!(main.draws[0].node, Some(6));
    assert_eq!(boost.draws.len(), 1);
    assert_eq!(boost.draws[0].node, Some(4));
    // The draw for node 1 (`root`) is in neither.
    assert_eq!(main.vertices.len(), model.vertices.len());
    assert_eq!(boost.indices.len(), model.indices.len());
}

/// A group the file authors nothing under is an **empty part**, not a missing
/// one - the caller reports "this craft authors no boost geometry", which a
/// dropped entry could not say.
#[test]
fn an_unauthored_group_is_empty_rather_than_absent() {
    let owner = owners(&flare_tree(), &["EF_Main", "EF_Nothing"]);
    let model = model_with_nodes(&[6]);
    let empty = part(&model, &owner, 1, "EF_Nothing");
    assert!(empty.draws.is_empty());
    assert_eq!(empty.radius, 0.0);
    assert_eq!(empty.mesh_count, 0);
}

/// The part's sphere is over the vertices its own ranges reach, not over the
/// shared buffer - or every part of a flare would be centred on the whole
/// flare.
#[test]
fn the_sphere_is_the_parts_own() {
    let owner = owners(&flare_tree(), &["EF_Main", "EF_Boost"]);
    let model = model_with_nodes(&[4, 6]);
    let main = part(&model, &owner, 0, "EF_Main");
    assert!(
        (main.centre[0] - 6.0).abs() < 1e-6,
        "EF_Main's own vertices sit at x = 6, not at the pair's mean: {:?}",
        main.centre
    );
}
