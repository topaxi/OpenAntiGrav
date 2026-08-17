//! What the disc says about a `.rcsmodel`'s material table.
//!
//! The third of the `.rcsmodel` ground-truth binaries, beside
//! `rcsmodel_ground_truth.rs` (the container) and
//! `rcsmodel_vertex_ground_truth.rs` (what a vertex holds). Split for the same
//! reason they are: one file per claim family keeps each under the size ratchet
//! and keeps a failure legible.
//!
//! `#[ignore]`d because it needs `data/images/hdfury-ps3-eu-dec.iso`; run with
//! `just test-data`.

mod rcsmodel_common;

use std::collections::{BTreeMap, BTreeSet};

use oag_formats::rcsmodel::{self, Blend, Transparency};
use oag_formats::vex;
use rcsmodel_common::{PAIRS, image, pair};

/// Every archive on the disc that holds `.rcsmodel` files.
const ARCHIVES: &[&str] = &[
    "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
];

/// Every material on the disc, as `(transparency, src, dst, name)`.
fn every_material() -> Vec<(Option<Transparency>, u16, u16, String)> {
    let image = image().expect("checked by the caller");
    let mut out = Vec::new();
    for archive in ARCHIVES {
        let spec = format!("{}:PS3_GAME/USRDIR/{archive}.PSARC", image.display());
        let mut open = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmodel"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(bytes) = open.read_path(&path) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&bytes) else {
                continue;
            };
            for m in &model.materials {
                out.push((
                    m.transparency(),
                    m.src_factor,
                    m.dst_factor,
                    m.name
                        .rsplit('/')
                        .next()
                        .unwrap_or_default()
                        .trim_end_matches(".rcsmaterial")
                        .to_ascii_lowercase(),
                ));
            }
        }
    }
    out
}

/// **The claim the whole decode rests on, and it is structural rather than a
/// reading of material names.**
///
/// The low two bits of a material's `+0x10` word gate whether its blend factor
/// pair is consumed. The evidence is that the pair is *constant* where those
/// bits are zero and *varied* where they are not: nothing writes seven distinct
/// equations into a field a renderer ignores, and nothing leaves 13,183 of
/// 13,188 records holding one default pair in a field it reads.
///
/// The bits take only three of their four values on the entire disc, which is
/// why `Transparency` has three members and `transparency()` returns an
/// `Option`.
#[test]
#[ignore]
fn the_low_two_state_bits_are_what_gate_the_blend_factors() {
    if image().is_none() {
        return;
    }
    let materials = every_material();
    assert_eq!(
        materials.len(),
        15_762,
        "the corpus this test's counts were measured on changed"
    );

    let mut pairs: BTreeMap<Option<Transparency>, BTreeMap<(u16, u16), usize>> = Default::default();
    for (mode, src, dst, _) in &materials {
        *pairs
            .entry(*mode)
            .or_default()
            .entry((*src, *dst))
            .or_default() += 1;
    }

    for (mode, by_pair) in &pairs {
        let total: usize = by_pair.values().sum();
        println!("  {mode:?}: {total} material(s), {} pair(s)", by_pair.len());
    }

    // 1. The fourth encoding is unused, so no material reads as `None`.
    assert!(
        !pairs.contains_key(&None),
        "a material outside the three known modes turned up: {:?}",
        pairs.keys().collect::<Vec<_>>()
    );

    // 2. Opaque: one pair, and it is the alpha-over default nothing consumes.
    let opaque = &pairs[&Some(Transparency::Opaque)];
    assert_eq!(opaque.values().sum::<usize>(), 13_188);
    assert_eq!(
        opaque[&(0x0302, 0x0303)],
        13_183,
        "the pair 13,183 of 13,188 opaque materials hold"
    );
    assert!(
        opaque.len() <= 3,
        "an opaque material's factor pair barely varies, which is the point: {opaque:?}"
    );

    // 3. Blended: the same field ranges widely, over both blend families.
    let blended = &pairs[&Some(Transparency::Blended)];
    assert_eq!(blended.values().sum::<usize>(), 2_362);
    assert!(
        blended.len() >= 7,
        "a blended material's factor pair is authored per effect: {blended:?}"
    );
    assert!(
        blended.keys().filter(|(_, dst)| *dst == 0x0001).count() >= 3,
        "three source factors appear against an additive destination, which is why \
         `Material::blend` keys on the destination alone: {blended:?}"
    );

    // 4. Mode 2 is see-through too, and is **not** told apart by its equation:
    //    211 of its 212 carry the same alpha-over pair blended materials use.
    let mode2 = &pairs[&Some(Transparency::Mode2)];
    assert_eq!(mode2.values().sum::<usize>(), 212);
    assert_eq!(mode2[&(0x0302, 0x0303)], 211);
}

/// The classification against the material *names*, which is where it was found
/// and so is a regression pin rather than fresh evidence.
///
/// It is still worth pinning: a shift in what these bits select would move
/// `glass_texture` or `track_surface` across the line, and that is legible in a
/// way a count is not.
#[test]
#[ignore]
fn the_named_glass_is_see_through_and_the_named_road_is_not() {
    if image().is_none() {
        return;
    }
    let materials = every_material();
    let mut modes: BTreeMap<&str, BTreeSet<bool>> = Default::default();
    for (mode, _, _, name) in &materials {
        let see_through = matches!(mode, Some(Transparency::Blended | Transparency::Mode2));
        modes.entry(name.as_str()).or_default().insert(see_through);
    }

    for name in [
        "glass_texture",
        "glass_texture_n",
        "basicalpha",
        "fence_alpha",
        "nr_crowd_bustle",
        "clouds",
        "scanlinebillboard",
        "hd_enginetrail",
    ] {
        assert_eq!(
            modes
                .get(name)
                .map(|m| m.iter().copied().collect::<Vec<_>>()),
            Some(vec![true]),
            "{name} should be see-through everywhere it appears"
        );
    }
    for name in [
        "track_surface",
        "track_wall",
        "weapon_pads",
        "glasstestnoalpha",
        "tunnel_fx_noalpha",
        "diffuse_with_specular_from_alpha",
    ] {
        assert_eq!(
            modes
                .get(name)
                .map(|m| m.iter().copied().collect::<Vec<_>>()),
            Some(vec![false]),
            "{name} should be opaque everywhere it appears - note the last three, \
             whose names carry a see-through word and whose state word does not"
        );
    }
}

/// Every factor pair the disc uses maps onto a blend class, bar a handful that
/// this module deliberately leaves unmapped.
#[test]
#[ignore]
fn the_unmapped_factor_pairs_are_few_and_named() {
    if image().is_none() {
        return;
    }
    let mut unmapped: BTreeMap<(u16, u16), BTreeSet<String>> = Default::default();
    let mut mapped = 0usize;
    for (mode, src, dst, name) in every_material() {
        let material = rcsmodel::Material {
            name: name.clone(),
            state: match mode {
                Some(Transparency::Opaque) => 0,
                Some(Transparency::Blended) => 1,
                Some(Transparency::Mode2) => 2,
                None => 3,
            },
            src_factor: src,
            dst_factor: dst,
        };
        match material.blend() {
            Blend::Opaque | Blend::Class(_) => mapped += 1,
            Blend::Unmapped { src, dst } => {
                unmapped.entry((src, dst)).or_default().insert(name);
            }
        }
    }
    println!("unmapped: {unmapped:?}");
    let count: usize = unmapped.values().map(BTreeSet::len).sum();
    assert!(
        count <= 5,
        "at most a handful of distinct materials use an equation this module does \
         not map: {unmapped:?}"
    );
    assert!(mapped >= 15_700, "{mapped} of 15,762 materials map");
}

/// `chunk +0x20` indexes the material table, checked by name on a craft whose
/// mesh nodes say what each surface is.
///
/// **Named, not merely in range.** Any small field would pass a range check; a
/// windscreen resolving to `glass_texture_n` and a hull to
/// `diffuse_with_specular_from_alpha_n_vcol` is what makes it the material
/// index and not some other per-chunk count.
#[test]
#[ignore]
fn a_chunks_material_index_names_the_surface_it_belongs_to() {
    let (archive, vex_path, model_path) = PAIRS[0];
    let Some((blob, bytes)) = pair(archive, vex_path, model_path) else {
        return;
    };
    let model = rcsmodel::Model::parse(&bytes).expect("the .rcsmodel parses");
    assert_eq!(model.materials.len(), 4, "Assegai carries four materials");

    let order = vex::byte_order(&blob);
    let classes = vex::classes_of(&blob).expect("a class table");
    let mut by_node: BTreeMap<String, String> = Default::default();
    for node in vex::nodes_by_class(
        &vex::nodes(&blob).expect("the node tree walks"),
        classes.mesh.expect("version 6 numbers Mesh"),
    ) {
        let payload = &blob[node.payload()];
        if payload.len() < 0x34 {
            continue;
        }
        let Some(mesh) = model.mesh(order.u32(payload, 0x30)) else {
            continue;
        };
        let Some(material) = model.material_of(mesh) else {
            continue;
        };
        by_node.insert(
            node.name.clone().unwrap_or_default(),
            material
                .name
                .rsplit('/')
                .next()
                .unwrap_or_default()
                .trim_end_matches(".rcsmaterial")
                .to_string(),
        );
    }
    println!("{by_node:?}");
    for (node, material) in [
        ("WindscreenShape", "glass_texture_n"),
        ("cockpit_screenShape", "screen_test"),
        ("FlashybitsShape", "emissive_bloom"),
        ("ShipShape", "diffuse_with_specular_from_alpha_n_vcol"),
    ] {
        assert_eq!(
            by_node.get(node).map(String::as_str),
            Some(material),
            "{node} should resolve to {material}"
        );
    }

    // And the classification follows from it: the windscreen is glass, the hull
    // is not.
    for (node, see_through) in [("WindscreenShape", true), ("ShipShape", false)] {
        let hash = vex::nodes_by_class(
            &vex::nodes(&blob).expect("nodes"),
            classes.mesh.expect("mesh"),
        )
        .find(|n| n.name.as_deref() == Some(node))
        .map(|n| order.u32(&blob[n.payload()], 0x30))
        .expect("the node is there");
        let mesh = model.mesh(hash).expect("its chunk is there");
        assert_eq!(
            model
                .material_of(mesh)
                .expect("its material is there")
                .is_see_through(),
            see_through,
            "{node}"
        );
    }
}

/// Every `/data/environments/` directory the disc ships, from
/// `rcsmodel_ground_truth.rs`'s own sweep.
const CIRCUITS: &[&str] = &[
    "01_vineta_k",
    "02_track",
    "03_track",
    "04_chenghou_project",
    "05_ubermall",
    "10_sebenco_climb",
    "12_sol_2",
    "15_anulpha_pass",
    "amphiseum",
    "modesto_heights",
    "talons_junction",
    "tech_de_ra",
    "zone_1",
    "zone_2",
    "zone_3",
    "zone_4",
];

/// The same split as [`a_third_of_a_circuits_chunks_are_see_through`], over all
/// 16 circuits.
///
/// **This is what makes not drawing a see-through chunk safe to do by default.**
/// One circuit measured is an anecdote: if some track ran 70 % see-through,
/// skipping those chunks would delete the level rather than uncover it. The
/// measured range is **4.3 % (`zone_4`) to 35.9 % (`talons_junction`)**, so the
/// behaviour `oag_render::mesh::rcs` settled on holds everywhere and not only
/// where it was looked at.
#[test]
#[ignore]
fn no_circuit_is_mostly_see_through() {
    let Some(image) = image() else {
        return;
    };
    // The 16 are spread over the archives, so each is looked for in all of
    // them: `01_vineta_k` is not in DATA00 and `talons_junction` is.
    let mut archives: Vec<oag_assets::psarc::Archive> = ARCHIVES
        .iter()
        .map(|a| {
            let spec = format!("{}:PS3_GAME/USRDIR/{a}.PSARC", image.display());
            oag_assets::psarc::Archive::open(&spec).expect("the archive opens")
        })
        .collect();

    let mut worst = 0.0f32;
    let mut measured = 0usize;
    for circuit in CIRCUITS {
        let path = format!("/data/environments/{circuit}/track.rcsmodel");
        let Some(bytes) = archives.iter_mut().find_map(|a| a.read_path(&path).ok()) else {
            continue;
        };
        let model = rcsmodel::Model::parse(&bytes).expect("the .rcsmodel parses");
        let (mut see_through, mut total) = (0usize, 0usize);
        for mesh in &model.meshes {
            total += 1;
            if model
                .material_of(mesh)
                .is_some_and(rcsmodel::Material::is_see_through)
            {
                see_through += 1;
            }
        }
        let share = see_through as f32 / total as f32;
        println!(
            "  {circuit:<20} {see_through:4} of {total:4} chunks, {:.1} %",
            100.0 * share
        );
        worst = worst.max(share);
        measured += 1;
    }
    assert_eq!(measured, CIRCUITS.len(), "every circuit was measured");
    assert!(
        worst < 0.4,
        "no circuit is mostly see-through; the worst is {:.1} %",
        100.0 * worst
    );
}

/// How much of a circuit's own geometry the table calls see-through.
///
/// The number that says why this matters: a third of Talon's Junction's
/// world-space chunks are surfaces the original does not draw solid, and this
/// project draws every one of them solid because it has no alpha for them.
#[test]
#[ignore]
fn a_third_of_a_circuits_chunks_are_see_through() {
    let (archive, vex_path, model_path) = PAIRS[2];
    let Some((blob, bytes)) = pair(archive, vex_path, model_path) else {
        return;
    };
    let model = rcsmodel::Model::parse(&bytes).expect("the .rcsmodel parses");
    assert_eq!(model.materials.len(), 442);

    let order = vex::byte_order(&blob);
    let classes = vex::classes_of(&blob).expect("a class table");
    let placed: BTreeSet<u32> = vex::nodes_by_class(
        &vex::nodes(&blob).expect("the node tree walks"),
        classes.mesh.expect("version 6 numbers Mesh"),
    )
    .filter_map(|n| {
        let p = &blob[n.payload()];
        (p.len() >= 0x34).then(|| order.u32(p, 0x30))
    })
    .collect();

    let (mut see_through, mut opaque) = (0usize, 0usize);
    for mesh in &model.meshes {
        // The world-space half: the road, the walls and the scenery, which is
        // what `oag_render::mesh::rcs::build_scene` draws without a node.
        if placed.contains(&mesh.hash) || mesh.solve_stride_without_a_box(&bytes).is_none() {
            continue;
        }
        match model
            .material_of(mesh)
            .is_some_and(rcsmodel::Material::is_see_through)
        {
            true => see_through += 1,
            false => opaque += 1,
        }
    }
    println!("{see_through} see-through, {opaque} opaque");
    assert_eq!((see_through, opaque), (257, 560));
    assert!(
        see_through * 4 > opaque,
        "this is not a rounding error on the picture"
    );
}
