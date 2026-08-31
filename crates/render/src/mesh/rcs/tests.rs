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
        strays: 2,
        unreferenced: 639,
        weapon_pads: 9,
        see_through: 257,
        cutout: 41,
        cutout_unread: 0,
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
        isolated: 0,
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
    assert!(
        line.contains("9 weapon pad chunk(s), drawn separately and gated by mode"),
        "the split has to be visible in the load report, not just in the model: {line}"
    );
}
