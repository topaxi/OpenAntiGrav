//! Wipeout 2048's scenery animation against Wipeout HD's, on the circuits
//! 2048 re-ships from HD.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! HD's scenery animation is decoded and validated (`Anim Transform`,
//! `hd_scenery_animation_ground_truth.rs`). 2048's DLC re-ships twelve HD
//! circuits and four Zone ones with the same scenery, so HD's evaluator is
//! an oracle for 2048's `.rcsskeleton`/`.rcsanimclip` rig: for every HD
//! `Anim Transform` that has a 2048 node of the same name,
//!
//! - **the node's world matrix at four times** - composed by the shipped
//!   builder, pivots and parent chain included - lands on HD's own
//!   `anchor_world` at the same time, and
//! - **the node's meshes** have vertex boxes inside HD's authored box for
//!   the same mesh *in its anchor's space*, which is what says the vertices
//!   are node-local rather than world-space.
//!
//! Only the nodes 2048 kept HD's animation for are compared - a track whose
//! loop is HD's `LoopEnd` - because 2048 re-authored some (Anulpha Pass's
//! trains loop at 15 s where HD's loop at 7.5 s, Chenghou Project's cars
//! at 38.3 s against 33.3 s) and those say nothing about the composition.
//! Neither check is asserted at 100 % even so, and each circuit prints what
//! missed. See `docs/formats/2048-animation.md` for the measured numbers.

use std::path::PathBuf;

use oag_rcs::rcsmaterial::name_hash;
use oag_render::mesh::rcs::psp2::{self, Animation};
use oag_vex::vex;

const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

/// The times HD's evaluator is sampled at: on a key, between keys, and
/// past the first few 0.2 s keys.
const TIMES: [f32; 4] = [0.0, 0.3, 1.0, 2.5];

/// How many of HD's `Anim Transform`s a circuit must keep by name for the
/// pairing to count as the same circuit.
const MATCHED_FLOOR: usize = 8;

/// World-matrix tolerance, in world units and basis-vector units alike -
/// or one percent of the node's distance from the origin, whichever is
/// larger: a 2048 flying vehicle a kilometre out lands within ten units
/// of HD's, its keys re-baked from the same curves at a different cadence,
/// while a composition error is hundreds.
const WORLD_TOLERANCE: f32 = 2.0;
const WORLD_TOLERANCE_RELATIVE: f32 = 0.01;

/// Vertex-box tolerance: HD's authored box is conservative by up to nine
/// units on the largest rings, so this is not a float comparison.
const BOX_TOLERANCE: f32 = 12.0;

/// A circuit both titles ship: the 2048 package and environment directory,
/// then HD's archive and environment directory.
struct Pair {
    package: &'static str,
    vita: &'static str,
    archive: &'static str,
    hd: &'static str,
    /// The share of compared placements that must land on HD's, in
    /// percent - **per circuit, because the misses are content**. Where
    /// 2048 kept HD's loop length it did not always keep its phase: Talons
    /// Junction's `Mining_Ship1_Ctrl2` starts half a loop along HD's path,
    /// so every one of its samples is thousands of units out while its
    /// composition is exactly HD's. Each floor sits a few points under what
    /// the circuit measured on 2026-09-16, so a composition regression
    /// (hundreds of units on every node) fails all twelve and a re-authored
    /// vehicle fails none.
    placement_floor: u8,
}

fn package(name: &str) -> Option<PathBuf> {
    oag_testdata::exact(&format!("data/extracted/vita/PCSF00007/{name}"))
}

struct Loaded {
    hd_data: Vec<u8>,
    model: oag_render::mesh::Model,
    plan_xform: Vec<u32>,
    animation: Animation,
    decoded: oag_rcs::rcsmodel::psp2::Model,
}

fn load(pair: &Pair) -> Option<Loaded> {
    let image = oag_testdata::image(PS3_IMAGE)?;
    let package = package(pair.package)?;
    let spec = format!("{}:PS3_GAME/USRDIR/{}", image.display(), pair.archive);
    let mut hd = oag_assets::psarc::Archive::open(&spec).expect("the HD archive opens");
    let hd_data = hd
        .read_path(&format!("/data/environments/{}/track.vex", pair.hd))
        .expect("HD track.vex");
    let mut vita = oag_assets::psarc::Archive::open(package.to_str().expect("utf-8"))
        .expect("the package opens");
    let base = format!("data/art/published/DLC1/environments/{}/track", pair.vita);
    let model_blob = vita
        .read_path(&format!("{base}.rcsmodel"))
        .expect("2048 track.rcsmodel");
    let skeleton = vita
        .read_path(&format!("{base}.rcsskeleton"))
        .expect("2048 track.rcsskeleton");
    let clip = vita
        .read_path(&format!("{base}.rcsanimclip"))
        .expect("2048 track.rcsanimclip");
    let animation = Animation::parse(&skeleton, Some(&clip)).expect("the rig parses");
    let decoded = oag_rcs::rcsmodel::psp2::parse(&model_blob).expect("the model parses");
    let plan = psp2::placement::plan(&decoded.scene, Some(&animation));
    let plan_xform = plan.placements.iter().map(|p| p.xform).collect();
    let (model, _) =
        psp2::build(pair.vita, &model_blob, Some(&animation), &mut |_| None).expect("builds");
    Some(Loaded {
        hd_data,
        model,
        plan_xform,
        animation,
        decoded,
    })
}

/// HD `Anim Transform` node index -> 2048 model node index, by the `~crc32`
/// of the node's Maya path under whichever namespaces the 2048 file uses.
fn matches(loaded: &Loaded, nodes: &[vex::Node], anim_class: u32) -> Vec<(usize, usize)> {
    let scene = &loaded.decoded.scene;
    // Every namespace prefix a node-bound mesh carries, plus none.
    let mut prefixes: Vec<String> = scene
        .meshes
        .iter()
        .filter(|m| m.node.is_some())
        .filter_map(|m| m.name.rfind(':').map(|at| m.name[..=at].to_string()))
        .collect();
    prefixes.push(String::new());
    prefixes.sort();
    prefixes.dedup();
    let mut out = Vec::new();
    for (i, node) in nodes.iter().enumerate() {
        if node.class_id != anim_class {
            continue;
        }
        let Some(name) = &node.name else { continue };
        let found = prefixes.iter().find_map(|p| {
            let hash = name_hash(&format!("{p}{name}"));
            scene.nodes.iter().position(|n| n.name_hash == hash)
        });
        if let Some(v) = found {
            out.push((i, v));
        }
    }
    out
}

fn max_abs_diff(a: &[f32; 16], b: &[f32; 16]) -> f32 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f32::max)
}

fn check(pair: &Pair) {
    let Some(loaded) = load(pair) else { return };
    let nodes = vex::nodes(&loaded.hd_data).expect("HD nodes");
    let classes = vex::classes_of(&loaded.hd_data).expect("HD classes");
    let anim_class = classes.anim_transform.expect("HD authors Anim Transform");
    let mesh_class = classes.mesh.expect("HD authors Mesh");
    let order = vex::byte_order(&loaded.hd_data);
    let anchors = vex::anim_anchors(&loaded.hd_data, &nodes);
    let matched = matches(&loaded, &nodes, anim_class);
    let hd_total = nodes.iter().filter(|n| n.class_id == anim_class).count();
    // A floor that catches a wrong pairing (zero names in common), not a
    // claim that 2048 kept everything: Chenghou Project keeps 52 of HD's
    // 136 - the blimp, the taxis and the cameras went.
    assert!(
        matched.len() >= MATCHED_FLOOR,
        "{}: only {} of HD's {hd_total} Anim Transforms have a 2048 node of the same name",
        pair.vita,
        matched.len()
    );

    // **Only the nodes 2048 kept HD's animation for are compared.** 2048
    // re-authored some (Chenghou Project's cars run 38.3 s laps where HD's
    // run 33.3 s; Anulpha Pass's trains 15 s where HD's run 7.5 s), and a
    // re-authored node says nothing about the composition. The tell is the
    // loop: a track whose duration is HD's `LoopEnd` is HD's animation
    // resampled, and one whose duration differs is not.
    let hd_transforms = vex::anim_transforms(&loaded.hd_data, &nodes);
    let same_loop = |hd: usize, v: usize| -> bool {
        let id = loaded.decoded.scene.nodes[v].id;
        let track = loaded.animation.clip.as_ref().and_then(|c| c.track(id));
        match (hd_transforms[hd].as_ref(), track) {
            // The same loop - or no `LoopEnd` on HD's side at all, where
            // the node plays its keys once and holds, which the first few
            // seconds sampled here cannot tell from 2048's own loop.
            (Some(hd), Some(track)) => {
                // 2048 sometimes bakes two or three of HD's loops into one
                // track (Amphiseum's 138.33 s tracks are two 69.17 s loops)
                // - the same animation, sampled over a longer span.
                let ratio = track.duration / hd.loop_seconds;
                let whole =
                    (ratio - ratio.round()).abs() < 0.003 && (1.0..=4.0).contains(&ratio.round());
                whole || hd.loop_seconds >= vex::DEFAULT_LOOP_SECONDS
            }
            // No keys on either side: the node is static in both.
            (Some(hd), None) => hd.loop_seconds >= vex::DEFAULT_LOOP_SECONDS,
            _ => false,
        }
    };
    let kept: Vec<(usize, usize)> = matched
        .iter()
        .copied()
        .filter(|&(hd, v)| same_loop(hd, v))
        .collect();
    println!(
        "{}: {} of {} shared nodes keep HD's loop length; {} are re-authored and not compared",
        pair.vita,
        kept.len(),
        matched.len(),
        matched.len() - kept.len()
    );
    assert!(
        kept.len() >= MATCHED_FLOOR,
        "{}: {} nodes keep HD's animation",
        pair.vita,
        kept.len()
    );

    // World matrices at each time, through the shipped builder's table.
    let mut placed = 0usize;
    let mut compared = 0usize;
    let mut missed: Vec<(String, f32)> = Vec::new();
    let plan = psp2::placement::plan(&loaded.decoded.scene, Some(&loaded.animation));
    for &t in &TIMES {
        let hd_world = vex::anchor_world(&loaded.hd_data, &nodes, t);
        let table = loaded.model.sample_anim_nodes(t);
        for &(hd, v) in &kept {
            let Some(expected) = hd_world[hd] else {
                continue;
            };
            let xform = loaded.plan_xform[v];
            let actual = if xform == 0 {
                // A node nothing moves: its static world, which the plan
                // baked into the vertices and does not table.
                plan.placements[v].to_world
            } else {
                table[xform as usize - 1]
            };
            compared += 1;
            let err = max_abs_diff(&actual, &expected);
            let distance =
                (expected[12].powi(2) + expected[13].powi(2) + expected[14].powi(2)).sqrt();
            if err <= WORLD_TOLERANCE.max(distance * WORLD_TOLERANCE_RELATIVE) {
                placed += 1;
            } else {
                missed.push((
                    format!("{} @ {t}s", nodes[hd].name.clone().unwrap_or_default()),
                    err,
                ));
            }
        }
    }
    missed.sort_by(|a, b| b.1.total_cmp(&a.1));
    println!(
        "{}: {placed} of {compared} node placements within tolerance of HD's; worst: {:?}",
        pair.vita,
        &missed[..missed.len().min(8)]
    );
    assert!(
        placed * 100 >= compared * usize::from(pair.placement_floor),
        "{}: {placed} of {compared} placements match HD, under the {} % floor",
        pair.vita,
        pair.placement_floor
    );

    // Vertex boxes, in anchor space.
    let by_short: std::collections::HashMap<&str, usize> = loaded
        .decoded
        .scene
        .meshes
        .iter()
        .enumerate()
        .filter(|(_, m)| m.node.is_some())
        .map(|(i, m)| (m.name.rsplit(':').next().unwrap_or(&m.name), i))
        .collect();
    let mut boxes = 0usize;
    let mut boxed = 0usize;
    let mut worst: Vec<(String, f32)> = Vec::new();
    for (i, node) in nodes.iter().enumerate() {
        if node.class_id != mesh_class {
            continue;
        }
        let Some(anchor) = anchors[i].anchor else {
            continue;
        };
        if !kept.iter().any(|&(hd, _)| hd == anchor) {
            continue;
        }
        let Some(name) = &node.name else { continue };
        let Some(&mesh) = by_short.get(name.as_str()) else {
            continue;
        };
        let payload = &loaded.hd_data[node.payload()];
        if payload.len() < 0x34 {
            continue;
        }
        let read3 =
            |at: usize| -> [f32; 3] { std::array::from_fn(|k| order.f32(payload, at + k * 4)) };
        let (min, max) = (read3(0x10), read3(0x20));
        // HD's box through the mesh node's own matrix to its anchor.
        let local = oag_core::math::Mat4::from_cols_array(&anchors[i].local);
        let mut hd_lo = [f32::MAX; 3];
        let mut hd_hi = [f32::MIN; 3];
        for c in 0..8 {
            let corner = oag_core::math::Vec3::new(
                if c & 1 == 0 { min[0] } else { max[0] },
                if c & 2 == 0 { min[1] } else { max[1] },
                if c & 4 == 0 { min[2] } else { max[2] },
            );
            let p = local.transform_point3(corner);
            for a in 0..3 {
                hd_lo[a] = hd_lo[a].min(p[a]);
                hd_hi[a] = hd_hi[a].max(p[a]);
            }
        }
        // 2048's vertices, as authored.
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        let mut any = false;
        for submesh in loaded
            .decoded
            .submeshes
            .iter()
            .filter(|s| s.mesh == Some(mesh))
        {
            for p in &submesh.positions {
                any = true;
                for a in 0..3 {
                    lo[a] = lo[a].min(p[a]);
                    hi[a] = hi[a].max(p[a]);
                }
            }
        }
        if !any {
            continue;
        }
        boxes += 1;
        // Inside HD's box, rather than equal to it: 2048 trims some meshes
        // (Chenghou Project's ship blocks keep the same min corner and lose
        // the far ships), and a trimmed mesh in the right space still sits
        // inside the box its untrimmed twin was authored with. A mesh in the
        // wrong space - world instead of anchor - lands hundreds of units
        // outside it.
        let inside = |hd_lo: [f32; 3], hd_hi: [f32; 3]| -> f32 {
            (0..3)
                .map(|a| (hd_lo[a] - lo[a]).max(hi[a] - hd_hi[a]).max(0.0))
                .fold(0.0f32, f32::max)
        };
        // Inside HD's box in the anchor's space - or in the mesh node's own,
        // where 2048 gave a `Mesh` that HD parents through a static
        // `Transform` its own node instead (Moa Therma's speedboat wakes).
        let err = inside(hd_lo, hd_hi).min(inside(min, max));
        if err <= BOX_TOLERANCE {
            boxed += 1;
        } else {
            worst.push((name.clone(), err));
        }
    }
    worst.sort_by(|a, b| b.1.total_cmp(&a.1));
    println!(
        "{}: {boxed} of {boxes} mesh boxes sit inside HD's in anchor space; worst: {:?}",
        pair.vita,
        &worst[..worst.len().min(6)]
    );
    // Sol 2 and Talons Junction keep HD's animated nodes under names 2048
    // renamed their meshes away from, so there is nothing to compare there
    // and the placement check above is the whole of it.
    if boxes > 0 {
        assert!(
            boxed * 100 >= boxes * 85,
            "{}: {boxed} of {boxes} mesh boxes match HD's anchor-space box",
            pair.vita
        );
    }
}

macro_rules! circuit {
    ($name:ident, $package:literal, $vita:literal, $archive:literal, $hd:literal, $floor:literal) => {
        #[test]
        #[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and data/extracted/vita/PCSF00007"]
        fn $name() {
            check(&Pair {
                package: $package,
                vita: $vita,
                archive: $archive,
                hd: $hd,
                placement_floor: $floor,
            });
        }
    };
}

circuit!(
    anulpha_pass,
    "base/PSP2/data.psarc",
    "Anulpha_Pass",
    "DATA02.PSARC",
    "15_anulpha_pass",
    88
);
circuit!(
    chenghou_project,
    "base/PSP2/data.psarc",
    "Chenghou_Project",
    "DATA02.PSARC",
    "04_chenghou_project",
    95
);
circuit!(
    vineta_k,
    "base/PSP2/data.psarc",
    "Vineta_K",
    "DATA02.PSARC",
    "01_vineta_k",
    80
);
// HD's two unnamed directories, told apart by which 2048 circuit shares
// their node names: `02_track` shares 35 with Metropia and none with Moa
// Therma, `03_track` 67 with Moa Therma and none with Metropia.
circuit!(
    metropia,
    "dlc1/PSP2/dlc1.psarc",
    "Metropia",
    "DATA02.PSARC",
    "02_track",
    80
);
circuit!(
    moa_therma,
    "base/PSP2/data.psarc",
    "Moa_Therma",
    "DATA02.PSARC",
    "03_track",
    65
);
circuit!(
    ubermall,
    "dlc1/PSP2/dlc1.psarc",
    "Ubermall",
    "DATA02.PSARC",
    "05_ubermall",
    72
);
circuit!(
    sebenco_climb,
    "dlc1/PSP2/dlc1.psarc",
    "Sebenco_Climb",
    "DATA02.PSARC",
    "10_sebenco_climb",
    95
);
circuit!(
    sol_2,
    "dlc1/PSP2/dlc1.psarc",
    "Sol_2",
    "DATA02.PSARC",
    "12_sol_2",
    70
);
circuit!(
    amphiseum,
    "dlc2/PSP2/dlc2.psarc",
    "amphiseum",
    "DATA00.PSARC",
    "amphiseum",
    90
);
circuit!(
    modesto_heights,
    "dlc2/PSP2/dlc2.psarc",
    "modesto_heights",
    "DATA00.PSARC",
    "modesto_heights",
    72
);
circuit!(
    talons_junction,
    "dlc2/PSP2/dlc2.psarc",
    "talons_junction",
    "DATA00.PSARC",
    "talons_junction",
    30
);
circuit!(
    tech_de_ra,
    "dlc2/PSP2/dlc2.psarc",
    "tech_de_ra",
    "DATA00.PSARC",
    "tech_de_ra",
    45
);
