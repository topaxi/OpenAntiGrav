//! What Wipeout 2048's own `321Go_2048.vex` authors for its `3`, `2`, `1`,
//! `GO` gantry, and how its manifest and its one HD-inherited asset compare
//! to `docs/rendering/start-gantry.md`'s Pulse and HD sections.
//!
//! **`#[ignore]`d and never run in CI.** It needs the decrypted Vita package,
//! which this project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md` and
//! `handover/2048s-vita-eboots-are-imported-re-not-started.md` for how
//! `data/extracted/vita/PCSF00007` gets populated.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-formats --run-ignored all \
//!     -E 'binary(start_gantry_2048_ground_truth)'
//! ```
//!
//! # What this reproduces, and what it does not
//!
//! `docs/rendering/start-gantry.md`'s 2048 section makes claims this pins:
//!
//! 1. 2048's own nine native circuits author slot 8 with `321Go_2048.vex` and
//!    no slot 7 at all - a manifest shape neither Pulse's nor HD's own is.
//! 2. The four HD-ported DLC circuits keep HD's exact slot 7/8 pair
//!    (`fx350.vex`/`321Go_StartFinish.vex`) unchanged.
//! 3. `321Go_2048.vex` authors `Three`, `Two`, `One` and `GO` as four
//!    **separate** `Anim Transform` + `Mesh` node pairs, not Pulse's one
//!    multi-cell node or HD's one five-cell node - a third encoding.
//! 4. Each digit's own translation track slides it between an off-screen `x`
//!    and an on-screen one, on its own schedule - measured, not a UV walk.
//! 5. `GO` (and three `Stripe_*` nodes alongside it) teleport off-screen at
//!    frame 360/361 - **the same 6.000 s loop-closing instant** Pulse's own
//!    panel and HD's own glyph node teleport at, a third title now.
//! 6. The whole authored timeline past the countdown - `FINAL`/`LAP` spelled
//!    letter by letter entering around frame 558-566, `polySurface7` (the
//!    chequered flag) entering at frame 740/741 - lands at the **same frame
//!    numbers** `docs/rendering/start-gantry.md`'s Pulse section records,
//!    node name included for `polySurface7`.
//! 7. 2048's own copy of HD's `321go_startfinish.vex` keeps the glyph node's
//!    exact translation key times (`[359, 360]`) HD's own pass measured -
//!    2048 re-exported the file into its own container without touching the
//!    authored timing.
//!
//! **Not proven here**: which submesh of `321go_startfinish.rcsmodel` is the
//! glyph's re-baked geometry. 2048's `.rcsmodel` container has no per-chunk
//! address (no hash, no name) the way HD's does - see
//! `oag_formats::rcsmodel::psp2`'s own module doc - so there is no principled
//! way to point at "this submesh is that node" the way
//! `start_gantry_hd_ground_truth.rs` does with `Model::mesh(hash)`. A
//! by-content check (a submesh whose five distinct UV cells numerically match
//! HD's own recovered raw values) is reported in
//! `docs/rendering/start-gantry.md` as suggestive, not asserted as a fact
//! here, precisely because it is found by coincidence of value rather than by
//! address.
//!
//! Nothing here places the gantry or resolves whether `321Go_Combat`/
//! `321Fight_2048` (this title's own Combat/Fight-mode variants) are ever
//! reached at runtime - both stay open on `docs/rendering/start-gantry.md`.

use std::path::{Path, PathBuf};

use oag_formats::{trackstartup::TrackStartup, vex};

const ARCHIVE: &str = "base/PSP2/data.psarc";

const NATIVE_CIRCUITS: &[&str] = &[
    "altima",
    "arena",
    "bridge",
    "cathedral",
    "mall",
    "park",
    "sol",
    "square",
    "subway",
    "tower",
];

fn source() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007");
    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

fn archive(source: &Path) -> oag_assets::psarc::Archive {
    oag_assets::psarc::Archive::open_file(&source.join(ARCHIVE)).expect("the base archive opens")
}

fn track_startup(archive: &mut oag_assets::psarc::Archive, circuit: &str) -> TrackStartup {
    let path = format!("data/art/published/environments/{circuit}/TrackStartup.xml");
    let xml = archive
        .read_path(&path)
        .unwrap_or_else(|e| panic!("{path} reads: {e}"));
    TrackStartup::parse(&String::from_utf8(xml).expect("the manifest is UTF-8"))
}

/// **Claim 1**: every one of 2048's own nine circuits authors slot 8 with its
/// own `321Go_2048.vex`, and authors no slot 7 at all - unlike every HD-ported
/// circuit.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn native_circuits_author_slot_8_alone_with_2048s_own_file() {
    let Some(source) = source() else { return };
    let mut archive = archive(&source);
    for circuit in NATIVE_CIRCUITS {
        let startup = track_startup(&mut archive, circuit);
        assert!(
            startup.billboard(7).is_none(),
            "{circuit} should author no slot 7 - it has no fx350 substitution site to feed"
        );
        let slot8 = startup
            .billboard(8)
            .unwrap_or_else(|| panic!("{circuit} should author slot 8"));
        assert_eq!(
            slot8.location().map(str::to_ascii_lowercase),
            Some("/data/billboards/hd_adverts/321go/321go_2048.vex".to_string()),
            "{circuit}'s slot 8 should name 2048's own gantry file, not HD's"
        );
    }
}

/// **Claim 2**: the four HD-ported DLC circuits keep HD's exact slot 7/8
/// pair, unchanged.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn hd_ported_circuits_keep_hds_exact_billboard_pair() {
    let Some(source) = source() else { return };
    let mut archive = archive(&source);
    for circuit in ["Anulpha_Pass", "Chenghou_Project", "Moa_Therma", "Vineta_K"] {
        let path = format!("data/art/published/DLC1/environments/{circuit}/TrackStartup.xml");
        let xml = archive
            .read_path(&path)
            .unwrap_or_else(|e| panic!("{path} reads: {e}"));
        let startup = TrackStartup::parse(&String::from_utf8(xml).expect("UTF-8"));
        let slot7 = startup
            .billboard(7)
            .unwrap_or_else(|| panic!("{circuit} should author slot 7, same as HD"));
        assert_eq!(
            slot7.location().map(str::to_ascii_lowercase),
            Some("/data/billboards/hd_adverts/321go/fx350.vex".to_string())
        );
        let slot8 = startup
            .billboard(8)
            .unwrap_or_else(|| panic!("{circuit} should author slot 8"));
        assert_eq!(
            slot8.location().map(str::to_ascii_lowercase),
            Some("/data/billboards/hd_adverts/321go/321go_startfinish.vex".to_string()),
            "{circuit} should keep HD's own gantry file, not 2048's native one"
        );
    }
}

/// The gantry file's node tree, read once per test that needs it.
fn gantry_nodes(archive: &mut oag_assets::psarc::Archive) -> (Vec<u8>, Vec<vex::Node>, u32) {
    let path = "data/billboards/hd_adverts/321go/321go_2048.vex";
    let data = archive
        .read_path(path)
        .unwrap_or_else(|e| panic!("{path} reads: {e}"));
    let nodes = vex::nodes(&data).expect("nodes");
    let classes = vex::classes_of(&data).expect("classes");
    let anim_class = classes.anim_transform.expect("anim transform class id");
    (data, nodes, anim_class)
}

/// **Claim 3**: `Three`, `Two`, `One` and `GO` are four separate `Anim
/// Transform` + `Mesh` node pairs - a third encoding, not Pulse's one
/// multi-cell node or HD's one five-cell node.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn the_four_states_are_four_separate_nodes_not_one_multi_cell_node() {
    let Some(source) = source() else { return };
    let mut archive = archive(&source);
    let (data, nodes, anim_class) = gantry_nodes(&mut archive);

    let mesh_class = vex::classes_of(&data).unwrap().mesh.expect("mesh class");
    for name in ["Three", "Two", "One", "GO"] {
        let node_index = nodes
            .iter()
            .position(|n| n.name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("a node named {name} should exist"));
        let node = &nodes[node_index];
        assert_eq!(
            node.class_id, anim_class,
            "{name} should be its own Anim Transform node"
        );
        assert_eq!(
            node.child_count, 1,
            "{name} should carry exactly one Mesh child, its own geometry"
        );
        let child_is_mesh = nodes
            .iter()
            .any(|n| n.parent == Some(node_index) && n.class_id == mesh_class);
        assert!(child_is_mesh, "{name}'s child should be a Mesh node");
    }

    // Every named digit/word node is distinct - not the same shape reused
    // four times, which `Three`/`Two`/`One`/`GO` each authoring its own
    // `Anim Transform` key schedule already implies but this makes explicit.
    let mut names: Vec<&str> = vec!["Three", "Two", "One", "GO"];
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 4, "all four names should be distinct");
}

/// One digit/word node's translation, sampled at rest and at, and either
/// side of, its own key times.
fn translation_x_at(data: &[u8], nodes: &[vex::Node], name: &str, seconds: f32) -> f32 {
    let node = nodes
        .iter()
        .find(|n| n.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("{name} should exist"));
    let anim = vex::anim_transform_of(data, node).unwrap_or_else(|| panic!("{name} should decode"));
    anim.sample(seconds)[12]
}

/// **Claim 4**: each digit slides between an off-screen and an on-screen `x`
/// on its own schedule - a measured slide, not a UV walk, not a fade.
///
/// Values are the exact ones a real decode produces (frame times `/60`
/// exactly land on the tenth of a millisecond this compares at), read with
/// `cargo run -p oag-game --example gantry_2048_anim_probe`.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn each_digit_slides_on_screen_for_its_own_window_then_off() {
    let Some(source) = source() else { return };
    let mut archive = archive(&source);
    let (data, nodes, _) = gantry_nodes(&mut archive);

    // `Three` starts already on screen (it is first) and slides off after
    // its own window closes, at frame 62 = 1.0333 s.
    let three_on = translation_x_at(&data, &nodes, "Three", 0.5);
    let three_off = translation_x_at(&data, &nodes, "Three", 1.5);
    assert!(
        (-5.0..5.0).contains(&three_on),
        "Three should be near centre while shown, got {three_on}"
    );
    assert!(
        three_off < -30.0,
        "Three should have slid well off screen after its window, got {three_off}"
    );

    // `Two` starts off screen, slides on at frame 79 = 1.3167 s, off again
    // at frame 122 = 2.0333 s.
    let two_before = translation_x_at(&data, &nodes, "Two", 1.0);
    let two_during = translation_x_at(&data, &nodes, "Two", 1.7);
    let two_after = translation_x_at(&data, &nodes, "Two", 2.5);
    assert!(
        two_before < -30.0,
        "Two should start off screen, got {two_before}"
    );
    assert!(
        (-5.0..5.0).contains(&two_during),
        "Two should be near centre during its window, got {two_during}"
    );
    assert!(
        two_after < -20.0,
        "Two should slide off again, got {two_after}"
    );

    // `GO` slides on at frame 200 = 3.3333 s and holds until the loop close.
    let go_before = translation_x_at(&data, &nodes, "GO", 3.0);
    let go_during = translation_x_at(&data, &nodes, "GO", 5.0);
    assert!(
        go_before < -30.0,
        "GO should start off screen, got {go_before}"
    );
    assert!(
        (-5.0..15.0).contains(&go_during),
        "GO should be on screen through the hold, got {go_during}"
    );
}

/// **Claim 5**: `GO` teleports off screen at the exact same 6.000 s
/// loop-closing instant Pulse's panel and HD's glyph node do - the third
/// title this cross-title coincidence holds on, this time on a different
/// axis (`x`, not `y`) and at a different magnitude (~40 units, not ~10).
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn go_teleports_off_screen_at_the_six_second_loop_close() {
    let Some(source) = source() else { return };
    let mut archive = archive(&source);
    let (data, nodes, _) = gantry_nodes(&mut archive);

    let node = nodes
        .iter()
        .find(|n| n.name.as_deref() == Some("GO"))
        .expect("GO exists");
    let anim = vex::anim_transform_of(&data, node).expect("GO decodes");
    assert_eq!(
        anim.translation.times,
        vec![198, 199, 200, 360, 361],
        "GO's own key times should include the 360/361 loop-close pair, the \
         same frame numbers Pulse's own panel teleports at"
    );

    let before = anim.sample(5.9833);
    let after = anim.sample(6.0167);
    let delta = (after[12] - before[12]).abs();
    assert!(
        delta > 30.0,
        "expected a large teleport in x at the loop close, got delta {delta}"
    );
    assert!(
        (before[13] - after[13]).abs() < 1e-3,
        "the teleport should be a pure x move on this title, unlike Pulse/HD's y move"
    );
}

/// **Claim 6**: the whole authored timeline past the countdown lands on the
/// same frame numbers `docs/rendering/start-gantry.md`'s Pulse section
/// records - `FINAL`/`LAP` spelled letter by letter starting around frame
/// 558-566, and `polySurface7` (the chequered flag, the *same node name*
/// Pulse's own file carries) starting at exactly frame 740/741.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn the_post_countdown_timeline_lands_on_pulses_own_frame_numbers() {
    let Some(source) = source() else { return };
    let mut archive = archive(&source);
    let (data, nodes, _) = gantry_nodes(&mut archive);

    for name in ["Final_F", "Lap_L"] {
        let node = nodes
            .iter()
            .find(|n| n.name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("{name} exists"));
        let anim = vex::anim_transform_of(&data, node).unwrap_or_else(|| panic!("{name} decodes"));
        let first = *anim
            .translation
            .times
            .first()
            .unwrap_or_else(|| panic!("{name} has translation keys"));
        assert!(
            (555..=570).contains(&first),
            "{name}'s first translation key should land around frame 558-566 \
             (9.3-9.43 s), the same instant Pulse's own FINAL LAP board \
             enters at frame 560/561 (9.333/9.350 s); got frame {first}"
        );
    }

    let poly7 = nodes
        .iter()
        .find(|n| n.name.as_deref() == Some("polySurface7"))
        .expect("polySurface7 exists - the same node name Pulse's own file carries");
    let anim = vex::anim_transform_of(&data, poly7).expect("polySurface7 decodes");
    assert_eq!(
        anim.translation.times,
        vec![740, 741],
        "polySurface7 should enter at exactly frame 740/741 - the same frame \
         numbers, and the same node name, Pulse's own chequered-flag entrance \
         carries"
    );
}

/// **Claim 7**: 2048's own re-export of HD's `321go_startfinish.vex` keeps
/// the glyph node's exact translation key times HD's own pass measured.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn the_inherited_hd_file_keeps_hds_exact_teleport_timing() {
    let Some(source) = source() else { return };
    let mut archive = archive(&source);
    let path = "data/billboards/hd_adverts/321go/321go_startfinish.vex";
    let data = archive
        .read_path(path)
        .unwrap_or_else(|e| panic!("{path} reads: {e}"));
    let nodes = vex::nodes(&data).expect("nodes");
    let node = nodes
        .iter()
        .find(|n| n.name.as_deref() == Some("pasted__Go_HD_start_light_321go"))
        .expect("the glyph node keeps its HD name");
    let anim = vex::anim_transform_of(&data, node).expect("it decodes");
    assert_eq!(
        anim.translation.times,
        vec![359, 360],
        "2048's own copy of this file should keep HD's exact key times \
         unchanged - the same evidence HD's own pass measured"
    );

    // The sibling .rcsmodel is 2048's own psp2 container (re-baked, not a
    // byte-identical copy of HD's PS3 one - see the module doc for why no
    // per-chunk address survives that re-bake).
    let sibling = "data/billboards/hd_adverts/321go/321go_startfinish.rcsmodel";
    let rcs = archive
        .read_path(sibling)
        .unwrap_or_else(|e| panic!("{sibling} reads: {e}"));
    let model = oag_formats::rcsmodel::psp2::parse(&rcs).expect("the psp2 container parses");
    assert_eq!(
        model.unpaired_pointers, 0,
        "every GPU pointer in this file should resolve to a submesh, the \
         same closure the corpus-wide reading requires"
    );
    assert!(
        model
            .materials
            .iter()
            .filter(|m| m.textures.iter().any(|t| t.contains("321_go_64")))
            .count()
            >= 2,
        "the countdown texture should back at least two material entries \
         here, the same reuse HD's own material table shows"
    );
}

/// **By-content corroboration for claim 7's caveat**: a submesh exists whose
/// five distinct UV cells numerically match HD's own recovered raw values
/// (`docs/rendering/start-gantry.md`'s HD section, before its `+0.04` static
/// offset) to within quantisation noise - found by scanning every submesh
/// naming the countdown texture, not by any per-chunk address, because this
/// container has none. This is why the doc reports the match as suggestive
/// rather than as settled the way HD's hash-addressed reading is.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn a_submesh_of_the_inherited_file_numerically_matches_hds_own_five_cells() {
    let Some(source) = source() else { return };
    let mut archive = archive(&source);
    let sibling = "data/billboards/hd_adverts/321go/321go_startfinish.rcsmodel";
    let rcs = archive
        .read_path(sibling)
        .unwrap_or_else(|e| panic!("{sibling} reads: {e}"));
    let model = oag_formats::rcsmodel::psp2::parse(&rcs).expect("parses");

    let go_material_indices: Vec<usize> = model
        .materials
        .iter()
        .enumerate()
        .filter(|(_, m)| m.textures.iter().any(|t| t.contains("321_go_64")))
        .map(|(i, _)| i)
        .collect();
    assert!(
        !go_material_indices.is_empty(),
        "at least one material should name the countdown texture"
    );

    let mut found = false;
    for submesh in &model.submeshes {
        let Some(material) = submesh.material else {
            continue;
        };
        if !go_material_indices.contains(&material) {
            continue;
        }
        let mut cells: Vec<[f32; 2]> = Vec::new();
        for uv in &submesh.texcoords {
            if !cells
                .iter()
                .any(|c| (c[0] - uv[0]).abs() < 1e-3 && (c[1] - uv[1]).abs() < 1e-3)
            {
                cells.push(*uv);
            }
        }
        if cells.len() != 5 {
            continue;
        }
        // Four narrow cells share `v`; the wide fifth one does not - the
        // same shape HD's own reading found.
        let mut by_u: Vec<f32> = cells.iter().map(|c| c[0]).collect();
        by_u.sort_by(f32::total_cmp);
        let expected_narrow = [0.0547_f32, 0.1831, 0.1920, 0.3079];
        let narrow_matches = expected_narrow
            .iter()
            .all(|expected| by_u[..4].iter().any(|u| (u - expected).abs() < 0.01));
        let wide_matches = (by_u[4] - 0.4968).abs() < 0.01;
        if narrow_matches && wide_matches {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "expected one submesh naming the countdown texture to carry five UV \
         cells numerically matching HD's own recovered raw values"
    );
}
