//! What the visibility placement in [`super`] is asserted to do: which section
//! governs a draw call, how the visible set is unioned and padded, and how
//! coincident swap sections resolve.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `pvs.rs`: the tests are 352 lines, past the 200 an inline test module
//! may hold. See `scripts/check-file-size.py`, which is the rule as a gate.

/// A minimal but honest `.vex` file header.
///
/// `TrackPvs::from_nodes` reads its class table out of the version word, so
/// a fixture's `data` has to be a file rather than a bare run of payloads -
/// which is what that parameter has always been documented as.
fn vex_header() -> Vec<u8> {
    let mut header = vec![0u8; oag_vex::vex::FILE_HEADER_LEN];
    header[0..4].copy_from_slice(&6u32.to_le_bytes());
    header[0x0c..0x10].copy_from_slice(oag_vex::vex::MAGIC);
    header
}

use super::*;
use oag_mesh::mesh::Bounds;

fn draw_at(centre: [f32; 3], radius: f32) -> DrawCall {
    DrawCall {
        moving: false,
        blend: None,
        blend_state: None,
        layer: oag_vex::vex::LAYER_DEFAULT,
        culled: false,
        range: 0..3,
        texture: None,
        bounds: Bounds { centre, radius },
        node: None,
        chunk: None,
        alpha_test_ref: None,
    }
}

/// Sections with boxes, built the way a `.vex` would hold them.
pub(super) fn pvs(boxes: &[(u8, [f32; 3], [f32; 3])]) -> TrackPvs {
    let mut data = vex_header();
    let mut nodes = Vec::new();
    for &(index, min, max) in boxes {
        nodes.push(oag_vex::vex::Node {
            class_id: oag_vex::vex::CLASS_SECTION,
            offset: data.len(),
            header_size: 0,
            data_size: 0x30,
            child_count: 0,
            unk_0x0e: 0,
            name: None,
            depth: 0,
            parent: None,
        });
        data.extend([index, 1, 0, 0, 0, 0, 0, 0]);
        data.extend(0u32.to_le_bytes());
        data.extend(0u32.to_le_bytes());
        for v in min {
            data.extend(v.to_le_bytes());
        }
        data.extend(0f32.to_le_bytes());
        for v in max {
            data.extend(v.to_le_bytes());
        }
        data.extend(0f32.to_le_bytes());
    }
    TrackPvs::from_nodes(&data, &nodes).expect("parse")
}

fn draw_of_node(node: Option<u32>) -> DrawCall {
    DrawCall {
        moving: false,
        chunk: None,
        blend: None,
        blend_state: None,
        layer: oag_vex::vex::LAYER_DEFAULT,
        culled: false,
        range: 0..3,
        texture: None,
        bounds: Bounds {
            centre: [0.0; 3],
            radius: 1.0,
        },
        node,
        alpha_test_ref: None,
    }
}

fn model_of(draws: Vec<DrawCall>) -> Model {
    Model {
        airbrakes: [None, None],
        node_vertex_ranges: Vec::new(),
        lod_groups: Default::default(),
        label: "test".into(),
        vertices: Vec::new(),
        indices: vec![0, 1, 2],
        draws,
        alpha_tested_draws: Vec::new(),
        transparent_draws: Vec::new(),
        textures: Vec::new(),
        lightmaps: Vec::new(),
        pad_masks: Vec::new(),
        wave_maps: Vec::new(),
        material_slots: Vec::new(),
        material_specular_exponent: Vec::new(),
        material_colour_factor: Vec::new(),
        material_variants: Vec::new(),
        material_anim: Vec::new(),
        shine_draws: Vec::new(),

        vertex_colour_is_light: false,
        stamps_glow: false,
        glow_by_texel: false,

        flame: None,
        absorb_shell: false,
        alpha_test_ref: None,
        centre: [0.0; 3],
        radius: 1.0,
        anim_tracks: Vec::new(),
        anim_nodes: Vec::new(),
        emissive: Vec::new(),
        mesh_count: 1,
    }
}

/// Placement is structural: a governed draw call gets its group's one
/// bit, whatever its bounds - which is exactly what lets a far-LOD copy
/// sitting *inside* the racing sections' boxes stay hidden while racing.
#[test]
fn a_governed_draw_call_gets_its_sections_single_bit() {
    let pvs = pvs(&[
        (0, [0.0, 0.0, 0.0], [10.0, 10.0, 10.0]),
        (3, [0.0, 0.0, 0.0], [10.0, 10.0, 10.0]),
    ]);
    // Nodes 0..3, where node 1 belongs to section 0's group and node 2 to
    // section 3's - the two groups' boxes overlap entirely.
    let governing = vec![None, Some(0), Some(3), None];
    let mut model = model_of(vec![draw_of_node(Some(1)), draw_of_node(Some(2))]);
    model.alpha_tested_draws.push(draw_of_node(Some(2)));

    let (sections, stats) = DrawSections::place(&model, &governing, &pvs);
    assert_eq!(sections.opaque, vec![1 << 0, 1 << 3]);
    assert_eq!(sections.alpha_tested, vec![1 << 3]);
    assert_eq!(sections.transparent, Vec::<u64>::new());
    assert_eq!(stats.placed, 3);
    assert_eq!(stats.unplaced, 0);
    assert_eq!(stats.total(), 3);
    assert!((stats.placed_fraction() - 1.0).abs() < 1e-6);
    assert_eq!(DrawSections::at(&sections.opaque, 99), ALWAYS);

    let sees_only_0 = VisibleSet { mask: 1 };
    assert!(sees_only_0.allows(sections.opaque[0]));
    assert!(
        !sees_only_0.allows(sections.opaque[1]),
        "the coincident group in the unseen section is hidden"
    );
}

/// The direction the error has to point, three ways: no node, no
/// governing section, and a governing section the track does not declare
/// all draw every frame rather than never.
#[test]
fn an_ungoverned_draw_call_always_draws() {
    let pvs = pvs(&[(0, [0.0, 0.0, 0.0], [10.0, 10.0, 10.0])]);
    let governing = vec![None, Some(0), Some(9)];
    let model = model_of(vec![
        draw_of_node(None),     // synthetic geometry
        draw_of_node(Some(0)),  // node no section governs
        draw_of_node(Some(2)),  // governed by undeclared section 9
        draw_of_node(Some(50)), // node index past the governance table
    ]);

    let (sections, stats) = DrawSections::place(&model, &governing, &pvs);
    assert_eq!(sections.opaque, vec![ALWAYS; 4]);
    assert_eq!(stats.placed, 0);
    assert_eq!(stats.unplaced, 4);
    assert_eq!(PlacementStats::default().placed_fraction(), 0.0);

    let hides_all_declared = VisibleSet { mask: 1 };
    assert!(
        hides_all_declared.allows(ALWAYS),
        "unplaced geometry survives any non-empty set"
    );
}

#[test]
fn an_unknown_section_makes_the_visible_set_everything() {
    let pvs = pvs(&[(0, [0.0; 3], [1.0; 3])]);
    let padding = SectionPadding::default();
    let set = VisibleSet::around(&pvs, &padding, &SwapConflicts::none(), UNPLACED, UNPLACED);
    assert!(set.is_everything(), "neither section is declared");
    assert_eq!(set.section_count(), 64);
    assert!(set.allows(1 << 40));
}

#[test]
fn the_visible_set_unions_the_craft_and_the_camera() {
    // Each section sees only itself.
    let pvs = pvs(&[(0, [0.0; 3], [1.0; 3]), (1, [0.0; 3], [1.0; 3])]);
    let padding = SectionPadding::default();

    let alone = VisibleSet::around(&pvs, &padding, &SwapConflicts::none(), 0, 0);
    assert_eq!(alone.mask(), 1, "just section 0");

    let split = VisibleSet::around(&pvs, &padding, &SwapConflicts::none(), 0, 1);
    assert_eq!(split.mask(), 0b11, "the camera's section is drawn too");
    assert!(split.allows(1) && split.allows(0b10) && !split.allows(0b100));
}

/// Padding pulls in the neighbours' *masks*, not merely the neighbours.
#[test]
fn padding_draws_the_neighbours_but_not_what_they_see() {
    use oag_vex::track::{AiTrack, Path, SplinePoint};
    let point = |section_id| SplinePoint {
        pos: [0.0; 3],
        tangent: [0.0, 0.0, 1.0],
        down: [0.0, -1.0, 0.0],
        lateral: [1.0, 0.0, 0.0],
        progress: 0.0,
        half_width_left: 1.0,
        half_width_right: 1.0,
        ai_bound_left: 1.0,
        ai_bound_right: 1.0,
        racing_line: 0.0,
        section_id,
        flags: 0,
        light_scale: [0xff; 4],
    };
    let track = AiTrack {
        version: 0x105,
        paths: vec![Path {
            points: vec![point(0), point(1)],
            max_spacing: 1.0,
            entry: None,
            exit: None,
        }],
        junctions: Vec::new(),
    };
    // Section 1's mask names a far-away section 40 - the LOD-swap shape:
    // what a neighbour sees is authored against *its* viewpoint, and
    // pulling it in early is how a far-LOD copy got drawn over the
    // detailed track it duplicates (Moa Therma's magstrip artifact,
    // second cause).
    let pvs = pvs_with_masks(&[(0, 0), (1, 1 << 40)]);
    let padding = SectionPadding::from_track(&track);
    assert!(padding.near(0) & 0b10 != 0, "1 is next door to 0");

    let set = VisibleSet::around(&pvs, &padding, &SwapConflicts::none(), 0, 0);
    assert_eq!(
        set.mask(),
        0b11,
        "the neighbour itself is drawn before it is entered, and what \
         only the neighbour can see is not"
    );

    // Once the craft is actually in section 1, its mask applies whole.
    let entered = VisibleSet::around(&pvs, &padding, &SwapConflicts::none(), 1, 1);
    assert!(entered.allows(1 << 40));
}

/// A LOD swap in miniature: sections 0 and 1 are ordinary neighbours
/// (each names the other), 2 is the detail and 3 the coincident copy -
/// named by disjoint viewpoints, never together. The draws of sections
/// 1, 2 and 3 share bit-identical vertex positions; section 0's geometry
/// is elsewhere.
fn swap_fixture() -> (TrackPvs, SwapConflicts) {
    let pvs = pvs_with_masks(&[
        (0, 0b0110), // sees 1 and the detail 2
        (1, 0b1001), // sees 0 and the copy 3
        (2, 0b0001),
        (3, 0b0010),
    ]);
    let governing = vec![Some(0), Some(1), Some(2), Some(3)];
    let mut model = model_of(vec![
        draw_of_node(Some(0)),
        draw_of_node(Some(1)),
        draw_of_node(Some(2)),
        draw_of_node(Some(3)),
    ]);
    let vertex = |x: f32| oag_mesh::mesh::GpuVertex {
        position: [x, 0.0, 0.0],
        normal: [0.0, 1.0, 0.0],
        colour: [1.0; 4],
        texcoord: [0.0; 2],
        lightmap_texcoord: [0.0, 0.0],
        lit: 1.0,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        slots: oag_mesh::mesh::slots::DEFAULT,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.0, 0.0],
    };
    model.indices.clear();
    for (draw, base_x) in [(0usize, 100.0f32), (1, 0.0), (2, 0.0), (3, 0.0)] {
        let start = model.vertices.len() as u32;
        for i in 0..SwapConflicts::SHARED_POSITIONS {
            model.vertices.push(vertex(base_x + i as f32));
        }
        let end = model.vertices.len() as u32;
        model.indices.extend(start..end);
        model.draws[draw].range = start..end;
    }
    let (sections, _) = DrawSections::place(&model, &governing, &pvs);
    let swaps = SwapConflicts::find(&pvs, &sections, &model);
    (pvs, swaps)
}

/// Only mask-exclusive pairs with overlapping geometry are swaps. Here
/// that is 2/3 alone: 1's geometry coincides with both, but mask 0 names
/// 1 with 2 and mask 1 names 1 with 3, so neither is exclusive - and 0
/// is exclusive with nothing that shares its space.
#[test]
fn only_coincident_exclusive_pairs_are_swaps() {
    let (_, swaps) = swap_fixture();
    assert_eq!(swaps.pair_count(), 1, "the detail and its copy");
    assert_eq!(swaps.partners_of(1 << 2), 1 << 3);
    assert_eq!(swaps.partners_of(1 << 3), 1 << 2);
    assert_eq!(swaps.partners_of(1 << 1), 0, "named together is not a swap");
    assert_eq!(swaps.partners_of(1 << 0), 0, "distant geometry never pairs");
    assert_eq!(SwapConflicts::none().partners_of(ALL_VISIBLE), 0);
    assert_eq!(
        swaps.partners_of(ALL_VISIBLE),
        0,
        "unknown draws everything"
    );
}

/// The straddle: craft already across the boundary (sees the detail),
/// camera still behind (sees the copy). The craft's mask wins and the
/// copy stays hidden; the rest of the camera's mask still contributes.
#[test]
fn the_crafts_mask_outranks_the_cameras_across_a_swap() {
    let (pvs, swaps) = swap_fixture();
    let padding = SectionPadding::default();

    let straddle = VisibleSet::around(&pvs, &padding, &swaps, 0, 1);
    assert!(straddle.allows(1 << 2), "the craft's detail is drawn");
    assert!(
        !straddle.allows(1 << 3),
        "the camera's copy of it is not - the authored exclusion holds"
    );
    assert!(
        straddle.allows(1 << 0) && straddle.allows(1 << 1),
        "the camera's non-conflicting geometry still contributes"
    );

    // The same straddle without the table is the bug this exists for.
    let unfiltered = VisibleSet::around(&pvs, &padding, &SwapConflicts::none(), 0, 1);
    assert!(unfiltered.allows(1 << 2) && unfiltered.allows(1 << 3));

    // And from the other side of the boundary the swap flips whole.
    let flipped = VisibleSet::around(&pvs, &padding, &swaps, 1, 0);
    assert!(flipped.allows(1 << 3) && !flipped.allows(1 << 2));
}

/// Sections without boxes but with authored visibility masks.
fn pvs_with_masks(sections: &[(u8, u64)]) -> TrackPvs {
    let mut data = vex_header();
    let mut nodes = Vec::new();
    for &(index, mask) in sections {
        nodes.push(oag_vex::vex::Node {
            class_id: oag_vex::vex::CLASS_SECTION,
            offset: data.len(),
            header_size: 0,
            data_size: 0x10,
            child_count: 0,
            unk_0x0e: 0,
            name: None,
            depth: 0,
            parent: None,
        });
        data.extend([index, 0, 0, 0, 0, 0, 0, 0]);
        data.extend((mask as u32).to_le_bytes());
        data.extend(((mask >> 32) as u32).to_le_bytes());
    }
    TrackPvs::from_nodes(&data, &nodes).expect("parse")
}

/// The ordering claim, made executable: with no frustum at all, the mask
/// alone decides, and geometry the mask excludes never reaches the frustum
/// test.
#[test]
fn the_mask_decides_before_the_frustum_is_consulted() {
    let draw = draw_at([0.0, 0.0, 0.0], 1.0);
    let hides_section_2 = VisibleSet { mask: !(1u64 << 2) };
    assert!(!visible(&draw, 1 << 2, Some(&hides_section_2), None, None));
    assert!(visible(&draw, 1 << 3, Some(&hides_section_2), None, None));
    assert!(
        visible(&draw, 1 << 2, None, None, None),
        "PVS off draws everything"
    );
    assert!(
        visible(&draw, ALWAYS, Some(&hides_section_2), None, None),
        "unplaced geometry is never excluded"
    );
}

/// A moving draw takes the section mask and skips the frustum: its bounds
/// are where it was at time zero, but its section is where the artists put it.
#[test]
fn a_moving_draw_is_hidden_by_its_section_but_never_by_its_bounds() {
    let mut draw = draw_at([1.0e6, 0.0, 0.0], 1.0);
    draw.moving = true;
    let hides_section_2 = VisibleSet { mask: !(1u64 << 2) };
    let looking_away = {
        use oag_core::math::{Vec3, camera};
        let projection = camera::perspective(core::f32::consts::FRAC_PI_2, 1.0, 1.0, 100.0);
        let view = camera::look_at(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
        Frustum::from_view_projection(projection * view)
    };
    assert!(
        !visible(&draw, 1 << 2, Some(&hides_section_2), None, None),
        "a section the set hides hides its moving draw"
    );
    assert!(
        visible(
            &draw,
            1 << 3,
            Some(&hides_section_2),
            None,
            Some(&looking_away)
        ),
        "a visible section draws it whatever its time-zero bounds say"
    );
    assert!(
        visible(&draw, 1 << 2, None, None, None),
        "PVS off draws everything"
    );
    assert!(
        visible(&draw, ALWAYS, Some(&hides_section_2), None, None),
        "an unplaced moving draw is never excluded"
    );
}

/// A `.pvs` fixture: `cells` cells at the given positions over `chunks`
/// chunks, each cell seeing exactly the chunks `sees` names for it.
fn hd_pvs(positions: &[[f32; 3]], chunks: usize, sees: impl Fn(usize) -> Vec<usize>) -> Vec<u8> {
    let width = chunks / 8 + 1;
    let mut out = Vec::new();
    for word in [positions.len() as u32, chunks as u32, 0x10, 0] {
        out.extend_from_slice(&word.to_be_bytes());
    }
    for p in positions {
        for axis in p {
            out.extend_from_slice(&axis.to_be_bytes());
        }
        out.extend_from_slice(&p[2].to_be_bytes());
    }
    for cell in 0..positions.len() {
        let mut map = vec![0u8; width];
        for chunk in sees(cell) {
            map[chunk >> 3] |= 1 << (chunk & 7);
        }
        out.extend_from_slice(&map);
    }
    out
}

fn chunk_draw(chunk: Option<u32>) -> DrawCall {
    DrawCall {
        chunk,
        ..draw_of_node(None)
    }
}

/// The PS3 tier decides on the chunk index, and a draw with no chunk index is
/// never the one it excludes.
#[test]
fn a_chunk_outside_the_cells_set_is_not_drawn() {
    let blob = hd_pvs(&[[0.0; 3], [1000.0, 0.0, 0.0]], 16, |cell| match cell {
        0 => vec![0, 1],
        _ => vec![15],
    });
    let pvs = oag_rcs::hd_pvs::Pvs::parse(&blob).unwrap();
    let set = ChunkSet::around(&pvs, Vec3::ZERO, Vec3::ZERO).expect("located");
    assert!(set.allows(&chunk_draw(Some(0))));
    assert!(set.allows(&chunk_draw(Some(1))));
    assert!(!set.allows(&chunk_draw(Some(15))));
    assert!(
        set.allows(&chunk_draw(None)),
        "a draw with no chunk index is unplaced, and unplaced always draws"
    );
    assert_eq!(set.chunk_count(), 2);
}

/// Both viewpoints contribute, which is what covers a chase camera that has
/// not reached the cell the craft is already in.
#[test]
fn the_craft_and_the_camera_are_unioned() {
    let far = [1000.0, 0.0, 0.0];
    let blob = hd_pvs(&[[0.0; 3], far], 16, |cell| match cell {
        0 => vec![0],
        _ => vec![9],
    });
    let pvs = oag_rcs::hd_pvs::Pvs::parse(&blob).unwrap();
    let together = ChunkSet::around(&pvs, Vec3::ZERO, Vec3::from_array(far)).expect("located");
    assert!(together.allows(&chunk_draw(Some(0))));
    assert!(together.allows(&chunk_draw(Some(9))));

    let craft_only = ChunkSet::around(&pvs, Vec3::ZERO, Vec3::ZERO).expect("located");
    assert!(!craft_only.allows(&chunk_draw(Some(9))));
}

/// The conservative fallback: a viewpoint nowhere near the authored partition
/// gets no first tier at all rather than a confidently wrong cell.
#[test]
fn a_viewpoint_off_the_partition_draws_everything() {
    let blob = hd_pvs(&[[0.0; 3]], 16, |_| vec![0]);
    let pvs = oag_rcs::hd_pvs::Pvs::parse(&blob).unwrap();
    let stray = Vec3::new(CHUNK_TRUST_RADIUS * 10.0, 0.0, 0.0);
    assert!(
        ChunkSet::around(&pvs, stray, stray).is_none(),
        "beyond CHUNK_TRUST_RADIUS the set is not built"
    );
    assert!(ChunkSet::around(&pvs, Vec3::ZERO, stray).is_some());
}

/// The neighbourhood padding, which covers a cell boundary crossed between two
/// frames the way `SectionPadding::HOPS` does on the PSP.
#[test]
fn a_neighbouring_cell_within_the_padding_is_unioned_in() {
    let inside = CHUNK_PAD * 0.5;
    let outside = CHUNK_PAD * 4.0;
    let blob = hd_pvs(
        &[[0.0; 3], [inside, 0.0, 0.0], [outside, 0.0, 0.0]],
        16,
        |cell| vec![cell],
    );
    let pvs = oag_rcs::hd_pvs::Pvs::parse(&blob).unwrap();
    let set = ChunkSet::around(&pvs, Vec3::ZERO, Vec3::ZERO).expect("located");
    assert!(set.allows(&chunk_draw(Some(0))));
    assert!(set.allows(&chunk_draw(Some(1))), "within CHUNK_PAD");
    assert!(!set.allows(&chunk_draw(Some(2))), "beyond CHUNK_PAD");
}
