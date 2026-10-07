//! Wipeout HD's `Anim Transform` nodes, from the disc through to a drawn,
//! moving batch.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(hd_scenery_animation_ground_truth)'
//! ```
//!
//! # What this measures
//!
//! `vex::anim_transform` read its payload little-endian until 2026-08-31,
//! because the module it lives in states - correctly, for the geometry
//! decoders around it - that no big-endian file reaches them. HD keeps the
//! class: **5,518 nodes across all seven archives**, every one of which
//! decoded to `None` and fell back to the identity, dropping the node's
//! *placement* along with its motion. That is the Pulse defect of 2026-08-18
//! repeated on a disc twelve times the size, and
//! `docs/rendering/scenery-animation.md` records the Pulse half.
//!
//! Five claims, each against something a self-consistent misread could not
//! satisfy:
//!
//! 1. **Every node decodes, and its six key arrays tile its payload.** The
//!    counts and the six offsets are separate fields, so a wrong key width
//!    leaves a gap rather than a plausible animation.
//! 2. **A widened key form exists and 97 nodes use it**, flagged by the
//!    payload's own `+0x34` word - the field Pulse leaves zero on all 393 of
//!    its nodes.
//! 3. **The circuits' geometry moves**, and where it lands is not where the
//!    identity fallback put it.
//! 4. **The drawn model carries it**: batches under a node get a non-zero
//!    `xform`, and sampling the table at two times gives two matrices.
//! 5. **Something uploads that table.** The first four all stop at the CPU,
//!    and geometry baked in anchor space against an all-identity table on the
//!    GPU draws at the anchor's origin - worse than the defect this fixed.
//!    Only two frames of pixels answer it.

mod archive_cache;

use std::path::{Path, PathBuf};

use oag_mesh::mesh;
use oag_vex::vex;

/// The decrypted PS3 image.
const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

/// All seven archives, because the class is not confined to one.
const ARCHIVES: usize = 7;

/// `Anim Transform` nodes across every `.vex` on the disc.
const EXPECTED_NODES: usize = 5518;

/// Of those, how many carry a non-zero `+0x34` word - the widened key form.
const EXPECTED_WIDE: usize = 97;

/// The word all 97 of them carry: [`vex::TRANSLATION_IS_FLOAT`] and
/// [`vex::ROTATION_IS_QUATERNION`] together, and never either alone.
const WIDE_FLAGS: u32 = vex::TRANSLATION_IS_FLOAT | vex::ROTATION_IS_QUATERNION;

/// The busiest single file, which is what `mesh::NODE_ANIM_LIMIT` has to clear.
const BUSIEST_FILE: (&str, usize) = ("/data/environments/modesto_heights/track.vex", 259);

/// A circuit with plenty of moving scenery, for the end-to-end half.
const CIRCUIT: (&str, &str) = (
    "DATA00.PSARC",
    "/data/environments/talons_junction/track.vex",
);

fn image() -> Option<PathBuf> {
    oag_testdata::image(PS3_IMAGE)
}

/// Claims 1 and 2: every node on the disc decodes, its arrays tile, and the
/// widened form is where and only where the flag word says.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_anim_transform_on_the_disc_decodes_and_tiles_its_payload() {
    let Some(image) = image() else { return };
    let mut nodes_seen = 0usize;
    let mut wide = 0usize;
    let mut busiest = (String::new(), 0usize);
    let mut worst_padding = 0usize;

    for n in 0..ARCHIVES {
        let spec = format!("{}:PS3_GAME/USRDIR/DATA0{n}.PSARC", image.display());
        let mut archive = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
        let paths: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.ends_with(".vex"))
            .cloned()
            .collect();
        for path in paths {
            let blob = archive.read_path(&path).expect("the .vex reads");
            let (Ok(classes), Ok(nodes)) = (vex::classes_of(&blob), vex::nodes(&blob)) else {
                continue;
            };
            let Some(anim_class) = classes.anim_transform else {
                continue;
            };
            let here = vex::nodes_by_class(&nodes, anim_class).count();
            if here > busiest.1 {
                busiest = (path.clone(), here);
            }
            for node in nodes.iter().filter(|n| n.class_id == anim_class) {
                nodes_seen += 1;
                let anim = vex::anim_transform_of(&blob, node)
                    .unwrap_or_else(|| panic!("{path}: {:?} does not decode", node.name));
                assert!(
                    (anim.seconds_per_key - 1.0 / 60.0).abs() < 1e-6,
                    "{path}: {:?} keys are not 60 Hz frames",
                    node.name
                );
                if anim.flags != 0 {
                    wide += 1;
                    assert_eq!(
                        anim.flags, WIDE_FLAGS,
                        "{path}: {:?} sets a flag bit this decode does not account for",
                        node.name
                    );
                    // A channel with a count of zero comes back empty and so
                    // carries no width at all - `camera1_group` authors no
                    // rotation. The flag still selects the stride the *stored*
                    // key occupies, which is what `trailing_padding` checks.
                    assert!(anim.translation.is_empty() || anim.translation.wide);
                    assert!(anim.rotation.is_empty() || anim.rotation.wide);
                    assert_eq!(anim.rotation.w.len(), anim.rotation.values.len());
                }
                worst_padding = worst_padding.max(trailing_padding(&blob, node, &anim));
            }
        }
    }

    println!("{nodes_seen} nodes, {wide} widened, worst padding {worst_padding} bytes");
    assert_eq!(
        nodes_seen, EXPECTED_NODES,
        "Anim Transform nodes on the disc"
    );
    assert_eq!(wide, EXPECTED_WIDE, "nodes using the widened key form");
    assert_eq!(
        (busiest.0.as_str(), busiest.1),
        BUSIEST_FILE,
        "the busiest file, which NODE_ANIM_LIMIT has to clear"
    );
    assert!(
        BUSIEST_FILE.1 < mesh::NODE_ANIM_LIMIT,
        "NODE_ANIM_LIMIT is {} and the disc's busiest file authors {}, so {} of \
         its nodes would freeze",
        mesh::NODE_ANIM_LIMIT,
        BUSIEST_FILE.1,
        BUSIEST_FILE.1 + 1 - mesh::NODE_ANIM_LIMIT
    );
    // HD aligns a key array to four bytes where Pulse leaves no gap at all, so
    // the tiling is contiguous rather than exact. Anything wider than an
    // alignment gap would be a field this reading has not accounted for.
    assert!(
        worst_padding <= 15,
        "trailing padding of {worst_padding} bytes"
    );
}

/// How many bytes of a node's payload the six key arrays leave over.
///
/// Panics if they do not tile it contiguously from `0x50`, which is what pins
/// the field map: the counts and the six offsets are independent fields, so a
/// wrong key width shows up here rather than as a plausible-looking animation.
fn trailing_padding(data: &[u8], node: &vex::Node, anim: &vex::AnimTransform) -> usize {
    let payload = &data[node.payload()];
    let order = vex::byte_order(data);
    let u16_at = |at: usize| usize::from(order.u16(payload, at));
    let u32_at = |at: usize| order.u32(payload, at) as usize;
    // Off the flag word rather than off the channel, because an empty channel
    // still stores one key and that key still takes the flagged width.
    let width = |bit: u32, w: usize| if anim.flags & bit != 0 { w } else { 6 };
    let (t, r, s) = (
        u16_at(0x02).max(1),
        u16_at(0x04).max(1),
        u16_at(0x06).max(1),
    );
    let mut spans = [
        (u32_at(0x0c), t * 2),
        (u32_at(0x2c), t * width(vex::TRANSLATION_IS_FLOAT, 12)),
        (u32_at(0x08), r * 2),
        (u32_at(0x1c), r * width(vex::ROTATION_IS_QUATERNION, 16)),
        (u32_at(0x38), s * 2),
        (u32_at(0x40), s * 6),
    ];
    spans.sort_unstable();
    let mut at = 0x50;
    for (start, len) in spans {
        assert!(
            start >= at && start - at <= 3,
            "{:?}: key array at {start:#x} leaves a gap after {at:#x}",
            node.name
        );
        at = start + len;
    }
    assert!(
        at <= payload.len(),
        "{:?}: keys run past the payload",
        node.name
    );
    payload.len() - at
}

/// Claims 3 and 4: a circuit's scenery lands somewhere new, and the built model
/// carries the table that moves it.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn a_circuits_anchored_scenery_is_placed_and_moves() {
    let Some(image) = image() else { return };
    let (archive, path) = CIRCUIT;
    let spec = format!("{}:PS3_GAME/USRDIR/{archive}", image.display());
    let data = archive_cache::read(&spec, path).expect("the .vex reads");
    let geometry = mesh::rcs::sibling_geometry(&spec, path, &data).expect("the .rcsmodel is there");

    let nodes = vex::nodes(&data).expect("the tree walks");
    let anchors = vex::anim_anchors(&data, &nodes);
    let anchored = anchors.iter().filter(|a| a.anchor.is_some()).count();
    assert!(anchored > 0, "{path} anchors nothing to an Anim Transform");

    // Claim 3, on the file rather than on the model: the anchors' own world
    // matrices at time zero are not the identity the failed decode fell back
    // to, and they change with time.
    let at_zero = vex::anchor_world(&data, &nodes, 0.0);
    let later = vex::anchor_world(&data, &nodes, 1.0);
    let placed = at_zero
        .iter()
        .flatten()
        .filter(|m| **m != vex::IDENTITY)
        .count();
    let moved = at_zero
        .iter()
        .zip(&later)
        .filter(|(a, b)| a.is_some() && a != b)
        .count();
    println!("{path}: {anchored} anchored nodes, {placed} placed, {moved} moving");
    assert!(placed > 0, "every anchor still resolves to the identity");
    assert!(moved > 0, "no anchor's matrix changes between 0 s and 1 s");

    // Claim 4, end to end through the builder the race uses.
    let (model, report) = mesh::rcs::build_scene(path, &data, &geometry, &mut |name| {
        archive_cache::read(&spec, name)
    })
    .expect("the scene builds");
    println!("{report:?}");
    assert!(
        !model.anim_nodes.is_empty(),
        "the model carries no Anim Transform table"
    );
    let moving = model.vertices.iter().filter(|v| v.xform != 0).count();
    assert!(
        moving > 0,
        "no vertex is baked in an anchor's space, so nothing can move"
    );
    let a = model.sample_anim_nodes(0.0);
    let b = model.sample_anim_nodes(1.0);
    assert_ne!(a, b, "the node table is the same at 0 s and 1 s");
    assert!(
        model
            .draws
            .iter()
            .chain(&model.transparent_draws)
            .any(|d| d.moving),
        "no draw call is marked moving, so the frustum test would cull on a \
         time-zero sphere"
    );
}

/// Claim 5, and the only one that proves anything *uploads* the table: two
/// renders of the same model at two clocks are two different pictures.
///
/// Every claim above stops at the CPU. `Model::sample_anim_nodes` producing
/// two tables says nothing about whether a drawable writes one, and a model
/// whose vertices are baked in **anchor** space against an all-identity table
/// on the GPU draws at the anchor's local origin - which would be *worse* than
/// the placement defect this work fixed, not better. Only a pixel answers that.
///
/// Offscreen, because this machine's windowed wgpu path screenshots black
/// while the headless one works - `mesh_render::tests` and `zone_recolour`
/// render on a real device the same way.
///
/// **The billboard is the discriminating target and the circuit is not.** A
/// `track.vex` framed by its own bounding sphere puts every mover at a few
/// pixels: Talon's Junction's 79 nodes move **4 of 76,800**, which is a real
/// signal and a fragile assertion. `piranha_billboard_01` is 19 nodes over
/// 1,174 vertices and is *all* mover, so it moves 1.5 % of the frame. Both are
/// checked, with the threshold on the one that can carry it.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn two_animation_clocks_render_two_different_frames() {
    let Some(image) = image() else { return };
    let mut checked = 0usize;
    for (path, floor) in MOVING_MODELS {
        let Some((model, from)) = build_anywhere(&image, path) else {
            continue;
        };
        checked += 1;
        let shot = |seconds: f32| {
            oag_mesh::mesh_render::capture_pixels_from(
                &model,
                320,
                240,
                0.0,
                0.3,
                oag_mesh::mesh_render::Anisotropy::default(),
                seconds,
            )
            .expect("the offscreen capture runs")
        };
        let a = shot(0.0);
        let b = shot(1.5);
        let moved = a
            .as_chunks::<4>()
            .0
            .iter()
            .zip(b.as_chunks::<4>().0)
            .filter(|(x, y)| x != y)
            .count();
        println!(
            "{from}{path}: {} anim nodes, {moved} of {} pixels differ at 1.5 s",
            model.anim_nodes.len(),
            a.len() / 4
        );
        assert!(
            moved >= *floor,
            "{path} moved {moved} pixels, under the {floor} this target carries - \
             either nothing uploads the node table or the geometry is not anchored"
        );
    }
    assert_eq!(checked, MOVING_MODELS.len(), "every target was reachable");
}

/// Models whose geometry hangs off an `Anim Transform`, with the pixel count
/// each is expected to move at 1.5 seconds. See the test's own doc comment for
/// why the floors differ by two orders of magnitude.
const MOVING_MODELS: &[(&str, usize)] = &[
    (CIRCUIT.1, 1),
    (
        "/data/billboards/hd_adverts/piranha/piranha_billboard_01.vex",
        500,
    ),
    (
        "/data/billboards/hd_adverts/icaras/looping_background.vex",
        100,
    ),
];

/// Builds a `.vex` and its `.rcsmodel` out of whichever archive carries them.
///
/// The billboards are not in the circuits' archive, and which one holds a given
/// advert is not worth pinning here - `oag_assets::Archives` exists for exactly
/// this and needs a title package this test does not otherwise want.
fn build_anywhere(image: &Path, path: &str) -> Option<(oag_mesh::mesh::Model, String)> {
    for n in 0..ARCHIVES {
        let archive = format!("PS3_GAME/USRDIR/DATA0{n}.PSARC");
        let spec = format!("{}:{archive}", image.display());
        let Some(data) = archive_cache::read(&spec, path) else {
            continue;
        };
        let geometry = mesh::rcs::sibling_geometry(&spec, path, &data)?;
        let (model, _) = mesh::rcs::build_scene(path, &data, &geometry, &mut |name| {
            archive_cache::read(&spec, name)
        })
        .expect("the scene builds");
        return Some((model, format!("DATA0{n} ")));
    }
    None
}
