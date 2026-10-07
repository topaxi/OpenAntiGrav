//! What [`super`] is asserted to do.
//!
//! Moved out of `mesh/rcs.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;

#[test]
fn a_sibling_is_the_same_path_with_the_other_extension() {
    assert_eq!(
        sibling_name("/data/ships/assegai/ship.vex").as_deref(),
        Some("/data/ships/assegai/ship.rcsmodel")
    );
    assert_eq!(
        sibling_name(r"Data\Ships\Assegai\Ship.VEX").as_deref(),
        Some(r"Data\Ships\Assegai\Ship.rcsmodel"),
        "both containers match a name case-insensitively"
    );
    assert_eq!(sibling_name("Data\\Tex\\thing.mip"), None);
    assert_eq!(sibling_name(".vex").as_deref(), Some(".rcsmodel"));
}

/// The Omega Collection's circuits spell the pairing with a `.final` infix.
/// Both spellings are a rewrite of the same `.vex` name, and the skeleton and
/// clip follow whichever `.rcsmodel` was actually found.
#[test]
fn a_cooked_sibling_carries_the_final_infix_and_its_animation_follows() {
    let track = r"Data\environments\tech_de_ra\track.vex";
    assert_eq!(
        sibling_name_cooked(track).as_deref(),
        Some(r"Data\environments\tech_de_ra\track.final.rcsmodel")
    );
    assert_eq!(
        sibling_name_cooked(r"Data\environments\tech_de_ra\track_reversed.vex").as_deref(),
        Some(r"Data\environments\tech_de_ra\track_reversed.final.rcsmodel")
    );
    assert_eq!(sibling_name_cooked("Data\\Tex\\thing.mip"), None);
    assert_eq!(
        psp2::animation_names_beside(r"Data\environments\tech_de_ra\track.final.rcsmodel"),
        Some((
            r"Data\environments\tech_de_ra\track.final.rcsskeleton".to_string(),
            r"Data\environments\tech_de_ra\track.final.rcsanimclip".to_string()
        ))
    );
    // The plain spelling is untouched, which is what keeps every other title's
    // lookup exactly what it was.
    assert_eq!(
        psp2::animation_names(track),
        psp2::animation_names_beside(&sibling_name(track).unwrap())
    );
    assert_eq!(psp2::animation_names_beside("track.vex"), None);
}

/// An empty model frames at the origin rather than dividing by zero.
#[test]
fn a_model_with_no_vertices_still_has_a_usable_radius() {
    let (centre, radius) = bounding_sphere(&[]);
    assert_eq!(centre, [0.0; 3]);
    assert_eq!(radius, 0.0);
}

/// A frame built with the diagnostic filter on says so in its own report,
/// because a mutilated picture read as the real one is the mistake
/// `isolate` exists to prevent.
#[test]
fn an_isolation_render_says_it_is_one() {
    let plain = Report {
        nodes: 4,
        addressed: 4,
        drawn: 4,
        ..Report::default()
    };
    assert!(
        !plain.describe().contains("DIAGNOSTIC"),
        "{}",
        plain.describe()
    );
    let filtered = Report {
        isolated: 968,
        ..plain
    };
    let line = filtered.describe();
    assert!(line.contains("DIAGNOSTIC"), "{line}");
    assert!(line.contains("968 chunk(s)"), "{line}");
    assert!(line.contains("not the picture"), "{line}");
}

/// A pad pass whose nodes name no chunk reports the world-pass chunks that are
/// the pad, not "0 of N (0 triangle(s))", which reads as an absent picture.
#[test]
fn a_pad_pass_with_no_addressed_node_reports_the_chunks_the_scene_draws() {
    let report = Report {
        nodes: 18,
        routed_chunks: 18,
        routed_triangles: 5_382,
        ..Report::default()
    };
    let line = report.describe();
    assert!(line.starts_with("18 node(s) address no chunk"), "{line}");
    assert!(line.contains("18 chunk(s) on a pad material"), "{line}");
    assert!(line.contains("(5382 triangle(s))"), "{line}");
    assert!(!line.contains("(0 triangle(s))"), "{line}");
    let drawn = Report {
        drawn: 15,
        addressed: 15,
        nodes: 16,
        triangles: 3_684,
        routed_chunks: 1,
        ..Report::default()
    };
    assert!(drawn.describe().starts_with("15 of 16 mesh node(s)"));
}

/// The report says what is missing, in the two ways it can be missing.
#[test]
fn the_report_names_both_kinds_of_absence() {
    let report = Report {
        nodes: 126,
        addressed: 70,
        world_baked: 5,
        drawn: 59,
        no_stride: 11,
        triangles: 4_000,
        routed_chunks: 0,
        routed_triangles: 0,
        strays: 2,
        unreferenced: 639,
        see_through: 257,
        cutout: 41,
        cutout_unread: 0,
        track_surface: 124,
        untextured: 3,
        lightmapped: 4,
        lightmap_undecoded: 0,
        second_texture_loaded: 6,
        second_texture_unread: 0,
        no_texcoord: 9,
        authored_normals: 531_904,
        variants_resolved: 40,
        variants_unshipped: 2,
        materials_unread: 1,
        variant_chunks: 300,
        variant_chunks_missed: 4,
        specular_exponent_unresolved: 12,
        emissive_surface_map_excluded: 0,
        emissive_role_unresolved: 0,
        pad_ne_bound: 0,
        pad_ne_unread: 0,
        mag_wave_bound: 0,
        mag_wave_unread: 0,
        vertex_scrolls: 0,
        light_cone_bound: 0,
        refraction_bound: 0,
        refraction_unread: 0,
        ice_bound: 0,
        ice_unread: 0,
        water_lit_colour: 0,
        water_glint_only: 0,
        isolated: 0,
        behind_glass: 0,
    };
    let line = report.describe();
    assert!(line.contains("59 of 126"), "{line}");
    assert!(line.contains("51 addressed no chunk"), "{line}");
    assert!(
        line.contains("5 baked in world space despite a node naming them"),
        "{line}"
    );
    assert!(line.contains("257 chunk(s) drawn see-through"), "{line}");
    assert!(
        line.contains("41 chunk(s) drawn as an alpha-test cutout"),
        "a cutout is a different GPU feature from a blend, and this crate drew \
         every one of them as the other one until the state word was read: {line}"
    );
    assert!(
        line.contains("124 chunk(s) flagged track surface for Zone's Track set"),
        "the file's own Scene/Track split is worth naming in the load report: {line}"
    );
    assert!(
        line.contains("40 of 42 drawn material(s) resolved"),
        "{line}"
    );
    assert!(line.contains("covering 300 of 304 chunk(s)"), "{line}");
    assert!(
        line.contains("3 material(s) whose .gtf did not paint"),
        "a draw with no texture binds the white 1x1 and paints a sheet, which \
         is what a working surface looks like at a glance: {line}"
    );
    assert!(
        line.contains("9 chunk(s) declaring no texture coordinate"),
        "a chunk with no coordinate paints at the origin of its texture, \
         which is an absence worth naming: {line}"
    );
    assert!(
        line.contains("11 had no recoverable vertex stride"),
        "{line}"
    );
    assert!(
        line.contains("6 second texture(s) loaded but not drawn (role unread)"),
        "{line}"
    );
    assert!(line.contains("2 submesh(es) dropped as strays"), "{line}");
    assert!(line.contains("531904 authored vertex normal(s)"), "{line}");
    assert!(
        line.contains("12 material(s) with no specular_exponent chain read (default used)"),
        "{line}"
    );
    // And the other way round: a model whose vertices carry no normal says
    // that the shading is this project's derivation, not the disc's data.
    let derived = Report {
        authored_normals: 0,
        ..report
    };
    assert!(
        derived
            .describe()
            .contains("lit off face normals computed from the triangles"),
        "{}",
        derived.describe()
    );
    assert!(
        line.contains("639 chunk(s) no node references"),
        "a circuit is mostly this, so the line has to say it: {line}"
    );
}
