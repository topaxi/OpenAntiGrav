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

use oag_rcs::rcsmodel::{self, Blend, Transparency};
use oag_vex::vex;
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

/// Every factor the disc uses is one of the four `Factor` names, so **no**
/// material anywhere on the disc comes back unmapped.
///
/// This used to allow a handful of unmapped pairs, because the mapping was onto
/// `vex::BlendClass` - Pulse's three recovered classes - and `hologram`,
/// `dg_zonelights1` and `zone_death_electricity` are equations Pulse has no
/// member for. `Material::blend` carries the authored pair itself now, so the
/// only way to be unmapped is a *factor value* outside `0x0001`, `0x0300`,
/// `0x0302` and `0x0303`, and there is none. The assertion is the stronger one
/// accordingly: zero, not five.
#[test]
#[ignore]
fn every_factor_the_disc_uses_is_one_of_the_four_named_ones() {
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
            alpha_func: 0,
            alpha_ref: 0.0,
            texture: String::new(),
            second_texture: None,
            texture_sampler: 0,
            second_texture_sampler: None,
            samplers: Vec::new(),
            parameters: Vec::new(),
            curve: None,
        };
        match material.blend() {
            Blend::Opaque | Blend::Factors { .. } => mapped += 1,
            // **Mode 2 does not consume the pair at all** - it is an alpha
            // test with blending off, see `Blend::AlphaTest` - so `blend()`
            // has nothing to say about its factors and the 212 materials in
            // it would drop out of this census silently. The question here is
            // about the *value in the record*, which those records still
            // carry, so they are asked directly instead.
            Blend::AlphaTest => match (
                rcsmodel::Factor::from_rsx(src),
                rcsmodel::Factor::from_rsx(dst),
            ) {
                (Some(_), Some(_)) => mapped += 1,
                _ => {
                    unmapped.entry((src, dst)).or_default().insert(name);
                }
            },
            Blend::Unmapped { src, dst } => {
                unmapped.entry((src, dst)).or_default().insert(name);
            }
        }
    }
    println!("unmapped: {unmapped:?}");
    let count: usize = unmapped.values().map(BTreeSet::len).sum();
    assert_eq!(
        count, 0,
        "every factor on the disc is one of the four named values: {unmapped:?}"
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

/// **A lightmap and a `lightmapUV` come together, on every circuit, with no
/// exceptions either way.**
///
/// The discriminator behind `Material::lightmap`, and the reason that method is
/// a reading rather than a preference for the second texture slot. The material
/// table and the vertex declaration are decoded independently out of two
/// different parts of the file, and they agree exactly: a chunk whose material
/// names one of the circuit's own `lmaps/*-lmap.gtf` **always** declares a
/// `lightmapUV` attribute, and a chunk whose material names anything else there
/// never does *because of* the material.
///
/// The second half is the one that would fail loudly if the reading were wrong.
/// A chunk with an `lmaps/` second texture and no coordinates to sample it
/// through would mean either that the path does not identify a lightmap or that
/// the attribute name does not - and this project would be multiplying a
/// circuit by an atlas it cannot address.
///
/// The converse is *not* asserted and is not true: plenty of chunks declare a
/// `lightmapUV` whose material's second slot is an emissive map, a normal map
/// or a mask. Those are the three uses of the slot that stay unsampled - see
/// `docs/formats/rcsmaterial.md`.
#[test]
#[ignore]
fn a_lightmap_and_a_lightmap_coordinate_come_together() {
    let Some(image) = image() else {
        return;
    };
    let mut archives: Vec<oag_assets::psarc::Archive> = ARCHIVES
        .iter()
        .map(|a| {
            let spec = format!("{}:PS3_GAME/USRDIR/{a}.PSARC", image.display());
            oag_assets::psarc::Archive::open(&spec).expect("the archive opens")
        })
        .collect();

    let (mut lightmapped, mut orphans, mut measured, mut chunks) = (0usize, 0usize, 0usize, 0usize);
    let mut named: BTreeSet<String> = BTreeSet::new();
    for circuit in CIRCUITS {
        let path = format!("/data/environments/{circuit}/track.rcsmodel");
        let Some(bytes) = archives.iter_mut().find_map(|a| a.read_path(&path).ok()) else {
            continue;
        };
        let model = rcsmodel::Model::parse(&bytes).expect("the .rcsmodel parses");
        let (mut here, mut missing) = (0usize, 0usize);
        for mesh in &model.meshes {
            chunks += 1;
            let Some(lightmap) = model
                .material_of(mesh)
                .and_then(rcsmodel::Material::lightmap)
            else {
                continue;
            };
            named.insert(lightmap.to_string());
            here += 1;
            if mesh
                .decl
                .as_ref()
                .and_then(rcsmodel::VertexDecl::lightmap_texcoord)
                .is_none()
            {
                missing += 1;
                println!(
                    "  {circuit}: chunk {:#010x} has {lightmap} and no lightmapUV",
                    mesh.hash
                );
            }
        }
        println!("  {circuit:<20} {here:4} lightmapped chunk(s), {missing} without coordinates");
        lightmapped += here;
        orphans += missing;
        measured += 1;
    }
    println!(
        "{measured} circuit(s), {chunks} chunks, {lightmapped} lightmapped, \
         {} distinct atlas(es)",
        named.len()
    );
    assert_eq!(measured, CIRCUITS.len(), "every circuit was measured");
    assert_eq!(
        orphans, 0,
        "a chunk carrying a lightmap with no coordinates to sample it through, \
         which would mean the path or the attribute name does not identify one"
    );
    assert!(
        lightmapped > 500,
        "only {lightmapped} lightmapped chunks disc-wide, so either the reading \
         narrowed or the corpus did"
    );
}

/// How much of a circuit's own geometry the table calls see-through.
///
/// The number that says why this matters: a third of Talon's Junction's
/// world-space chunks are surfaces the original does not draw solid, and this
/// project draws every one of them solid because it has no alpha for them.
///
/// **The pinned pair moved from `(257, 560)` to `(251, 553)` on 2026-09-02,
/// and it is the stride search getting more honest, not less accurate.**
/// `rcsmodel::STRIDES` widened from three candidates to the disc's own seven
/// on 2026-08-25 (`docs/formats/rcsmodel.md`, "there are more strides than
/// three"), and that is the cause - confirmed rather than assumed: setting
/// `STRIDES` back to `[14, 18, 22]` locally and re-running this test
/// reproduces `(257, 560)` exactly. The likely mechanism, from reading
/// [`solve_stride_without_a_box`]'s three rules rather than from tracing every
/// changed chunk: each picks a winner by its margin over the runner-up among
/// `STRIDES`, and four more candidates give a few chunks that used to win by
/// default against two rivals a closer contender to lose or tie against,
/// so a few more chunks correctly come back `None` instead of a guess. The
/// share itself barely moved (31.4% to 31.2%), which reads as the same disc
/// measured more carefully rather than a different disc.
///
/// [`solve_stride_without_a_box`]: rcsmodel::Mesh::solve_stride_without_a_box
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
    assert_eq!((see_through, opaque), (251, 553));
    assert!(
        see_through * 4 > opaque,
        "this is not a rounding error on the picture"
    );
}

/// Every material names a `.gtf`, and the second slot is **not** one thing.
///
/// The first sample said "normal map" - Assegai's only two second textures are
/// `assegai_n.gtf` and `assegai_glass_n.gtf`. Four more materials refute it, and
/// this test pins the refutation so the name reading cannot come back.
#[test]
#[ignore]
fn every_material_names_a_texture_and_the_second_slot_is_not_one_thing() {
    let (_, ship) = match pair(PAIRS[0].0, PAIRS[0].1, PAIRS[0].2) {
        Some(p) => p,
        None => return,
    };
    let (_, track) = pair(PAIRS[2].0, PAIRS[2].1, PAIRS[2].2).expect("checked above");

    for (label, blob, count) in [("assegai", &ship, 4usize), ("talons_junction", &track, 442)] {
        let model = rcsmodel::Model::parse(blob).expect("the .rcsmodel parses");
        assert_eq!(model.materials.len(), count);
        let named = model
            .materials
            .iter()
            .filter(|m| m.texture.ends_with(".gtf"))
            .count();
        let second = model
            .materials
            .iter()
            .filter(|m| m.second_texture.is_some())
            .count();
        println!("  {label}: {named} of {count} name a texture, {second} name a second");
        assert_eq!(named, count, "{label}: every material paints with a .gtf");
    }

    // The refutation itself, by name on the circuit: one slot, four uses.
    let model = rcsmodel::Model::parse(&track).expect("parses");
    let of = |leaf: &str| {
        model
            .materials
            .iter()
            .find(|m| {
                m.name
                    .rsplit('/')
                    .next()
                    .unwrap_or_default()
                    .trim_end_matches(".rcsmaterial")
                    == leaf
            })
            .map(|m| {
                (
                    m.texture.clone(),
                    m.second_texture.clone().unwrap_or_default(),
                )
            })
    };
    for (material, first, second) in [
        ("clouds", "clouds_new.gtf", "cloud mask.gtf"),
        (
            "tunnel_fx_noalpha",
            "tunnel_fx_diffuse.gtf",
            "tunnel_fx_emissive.gtf",
        ),
    ] {
        let (a, b) = of(material).unwrap_or_else(|| panic!("{material} is on this circuit"));
        assert!(a.ends_with(first), "{material}: {a}");
        assert!(b.ends_with(second), "{material}: {b}");
    }
    let (_, lightmap) = of("diffusewithalphachannel").expect("the circuit has one");
    assert!(
        lightmap.contains("lmap"),
        "a fourth use of the same slot, a lightmap: {lightmap}"
    );
}

/// The declared coordinate is a texture coordinate.
///
/// **What confirms it is a picture.** Sampling Assegai's own `.gtf` through
/// these coordinates renders the words "ASSEGAI DEVELOPMENTS" legibly along the
/// hull - lettering a wrong reading cannot produce, since any other offset or
/// packing smears the atlas. What is asserted here is the statistic behind
/// that: the pair lands in the unit square on essentially all of a craft's
/// vertices, and mostly on a circuit's, where tiling legitimately runs outside
/// it.
///
/// **The bars were raised on 2026-08-18 and the reason is the finding.** This
/// used to read the last four bytes of every vertex and pass at 75 % of a
/// circuit's coordinates in the unit square and 97 % finite - which was the
/// honest bar for a reading that was, on a quarter of the chunks, sampling a
/// lightmap's atlas coordinates or a colour set. Reading where
/// `rcsmodel::VertexDecl` says clears far more, and a bar that a wrong reading
/// also clears is not measuring anything.
#[test]
#[ignore]
fn the_declared_coordinate_is_a_texture_coordinate() {
    let Some((_, ship)) = pair(PAIRS[0].0, PAIRS[0].1, PAIRS[0].2) else {
        return;
    };
    let (_, track) = pair(PAIRS[2].0, PAIRS[2].1, PAIRS[2].2).expect("checked above");

    // **An exact count of non-finite coordinates, not a percentage bar.** The
    // circuit's used to be 1.5 % - around 9,000 vertices - and is 9: almost
    // every one of them was a `tangent` or a colour set read as a coordinate,
    // because the reader took the last four bytes of a vertex. What survives on
    // both models is a different and still-unexplained thing, three orders of
    // magnitude smaller, and it is pinned exactly rather than covered by a bar
    // so that whatever it is cannot grow unnoticed.
    for (label, blob, unit_bar, non_finite) in [
        ("assegai", &ship, 0.95, 6),
        ("talons_junction", &track, 0.75, 9),
    ] {
        let model = rcsmodel::Model::parse(blob).expect("the .rcsmodel parses");
        let (mut total, mut unit, mut finite) = (0usize, 0usize, 0usize);
        for mesh in &model.meshes {
            let Some(stride) = mesh.solve_stride_without_a_box(blob) else {
                continue;
            };
            for sub in &mesh.submeshes {
                let Ok(uvs) = mesh.texcoords(blob, sub, stride) else {
                    continue;
                };
                for [u, v] in uvs {
                    total += 1;
                    if !u.is_finite() || !v.is_finite() {
                        continue;
                    }
                    finite += 1;
                    if (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v) {
                        unit += 1;
                    }
                }
            }
        }
        let share = |k: usize| k as f32 / total as f32;
        println!(
            "  {label}: {total} vertices, {:.1} % finite, {:.1} % in the unit square",
            100.0 * share(finite),
            100.0 * share(unit)
        );
        assert_eq!(
            total - finite,
            non_finite,
            "{label}: non-finite coordinates moved"
        );
        assert!(
            share(unit) >= unit_bar,
            "{label}: {:.3} in the unit square",
            share(unit)
        );
    }
}

/// **The source factor is not constant, which is why dropping it was a bug.**
///
/// `Material::blend` used to key on the destination alone, so every see-through
/// surface reached a renderer as one of Pulse's three classes and was drawn with
/// a source factor of `GL_SRC_ALPHA` whatever the file said. This is the
/// measurement that says how much that costs: over every see-through material on
/// the disc, how many author a source that is *not* `GL_SRC_ALPHA`.
///
/// Two families are wrong under the old reading and both are here.
/// `0001`/`0001` is unweighted additive, drawn as `SrcAlpha`/`One` - every texel
/// scaled by its own alpha before being added. `0001`/`0303` is premultiplied
/// alpha, drawn as straight alpha - the colour multiplied by alpha a second
/// time, which darkens exactly the semi-transparent edge of a glass panel.
#[test]
#[ignore]
fn a_see_through_material_does_not_always_author_a_source_of_src_alpha() {
    if image().is_none() {
        return;
    }
    let mut pairs: BTreeMap<(u16, u16), usize> = BTreeMap::new();
    for (mode, src, dst, _) in every_material() {
        if matches!(mode, Some(Transparency::Opaque) | None) {
            continue;
        }
        *pairs.entry((src, dst)).or_default() += 1;
    }
    println!("see-through factor pairs disc-wide: {pairs:?}");
    let total: usize = pairs.values().sum();
    let other_source: usize = pairs
        .iter()
        .filter(|((src, _), _)| *src != rcsmodel::material::FACTOR_SRC_ALPHA)
        .map(|(_, count)| count)
        .sum();
    assert!(
        other_source >= 300,
        "{other_source} of {total} see-through materials author a source other than \
         GL_SRC_ALPHA; folding them onto Pulse's classes redraws every one of them"
    );
    // The two the old reading got wrong, named so a regression says which.
    for pair in [
        (
            rcsmodel::material::FACTOR_ONE,
            rcsmodel::material::FACTOR_ONE,
        ),
        (
            rcsmodel::material::FACTOR_ONE,
            rcsmodel::material::FACTOR_ONE_MINUS_SRC_ALPHA,
        ),
    ] {
        assert!(
            pairs.contains_key(&pair),
            "{pair:?} is on the disc and is neither of Pulse's equations"
        );
    }
}

/// A reversed circuit's atlases live in a differently-named directory, and the
/// predicate has to see all of them.
///
/// The regression this exists for: [`rcsmodel::Material::lightmap`] used to
/// require the literal `/lmaps/`, which is where a **forward** circuit keeps
/// its atlases. A reversed one keeps them in `lmaps_rev/` and Fury's in
/// `lmaps_dlc/`, so the predicate answered `None` for every reversed chunk and
/// those surfaces would have bound the no-lightmap placeholder and lit without
/// their bake. It was latent - nothing loads a reversed circuit yet - which is
/// exactly why it needed a test rather than a note: the day one loads, the
/// wrong picture would have looked like a shading bug rather than a path bug.
#[test]
#[ignore]
fn a_reversed_circuit_finds_its_own_lightmap_atlases() {
    let Some(image) = image() else {
        return;
    };
    let mut archives: Vec<oag_assets::psarc::Archive> = ARCHIVES
        .iter()
        .map(|a| {
            let spec = format!("{}:PS3_GAME/USRDIR/{a}.PSARC", image.display());
            oag_assets::psarc::Archive::open(&spec).expect("the archive opens")
        })
        .collect();

    let (mut measured, mut lightmapped, mut orphans) = (0usize, 0usize, 0usize);
    let mut dirs: BTreeSet<String> = BTreeSet::new();
    for circuit in CIRCUITS {
        let path = format!("/data/environments/{circuit}/track_reversed.rcsmodel");
        let Some(bytes) = archives.iter_mut().find_map(|a| a.read_path(&path).ok()) else {
            continue;
        };
        let model = rcsmodel::Model::parse(&bytes).expect("the .rcsmodel parses");
        measured += 1;
        let mut here = 0usize;
        for mesh in &model.meshes {
            let Some(atlas) = model
                .material_of(mesh)
                .and_then(rcsmodel::Material::lightmap)
            else {
                continue;
            };
            here += 1;
            if let Some(leaf) = atlas
                .rsplit_once('/')
                .and_then(|(dir, _)| dir.rsplit_once('/'))
                .map(|(_, leaf)| leaf)
            {
                dirs.insert(leaf.to_string());
            }
            if mesh
                .decl
                .as_ref()
                .and_then(rcsmodel::VertexDecl::lightmap_texcoord)
                .is_none()
            {
                orphans += 1;
            }
        }
        println!("  {circuit:<20} {here:4} lightmapped chunk(s) reversed");
        lightmapped += here;
    }
    println!("{measured} reversed circuit(s), {lightmapped} lightmapped, dirs {dirs:?}");
    assert!(
        measured > 0,
        "no reversed circuit was found on the disc at all"
    );
    assert!(
        lightmapped > 100,
        "only {lightmapped} lightmapped chunks across {measured} reversed \
         circuit(s), so the atlas directory the predicate accepts has narrowed \
         again - it must cover lmaps/, lmaps_rev/ and lmaps_dlc/"
    );
    assert_eq!(
        orphans, 0,
        "a reversed chunk carrying a lightmap with no coordinates to sample it \
         through"
    );
}

/// The per-instance **parameter table**, disc-wide, and the numbers the engine
/// flare's material authors.
///
/// **This is where a shipped shader's constants live**, and until
/// [`rcsmodel::material::parameters`] existed it was the unread tail of a
/// record: the `.rcsmaterial` declares that a program takes a `power1`, and the
/// value is here, per model.
///
/// The layout claim is `+0x30` count, `+0x34` table, `0x20`-byte entries,
/// `+0x04` kind (`0x8001` sampler, `0` parameter), `+0x18` value offset,
/// `+0x1c` quad count. A wrong walk reads arbitrary bytes as floats, so the
/// disc-wide half asserts that **every** value on the disc is a finite number
/// of ordinary magnitude - 20,445 of them across 9,757 materials, which is what
/// the confidence in `engine-flare.md` rests on.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_material_parameter_on_the_disc_is_a_plausible_number() {
    let Some(image) = image() else {
        return;
    };
    let mut total = 0usize;
    let mut with_table = 0usize;
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
            for material in &model.materials {
                if material.parameters.is_empty() {
                    continue;
                }
                with_table += 1;
                for parameter in &material.parameters {
                    total += 1;
                    assert!(
                        parameter
                            .value
                            .iter()
                            .all(|v| v.is_finite() && v.abs() < 1.0e6),
                        "{path}: {} parameter {:#010x} reads {:?}, which is not a \
                         number a shader takes - the table walk is off",
                        material.name,
                        parameter.hash,
                        parameter.value
                    );
                }
            }
        }
    }
    assert!(
        with_table > 9_000 && total > 20_000,
        "only {with_table} material(s) with a table and {total} parameter(s); the \
         census was 9,757 and 20,445"
    );
}

/// Feisar's engine flare, parameter by parameter.
///
/// The five the flame's fragment program patches into inline constants, plus
/// the `Speed` its unimplemented scroll takes. Named where a `~crc32` preimage
/// was found - see `docs/ghidra/functions/ps3-hdfury-eu/engine-flare.md`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_engine_flares_material_authors_the_numbers_its_program_patches() {
    let Some(image) = image() else {
        return;
    };
    let spec = format!("{}:PS3_GAME/USRDIR/DATA02.PSARC", image.display());
    let mut open = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
    let bytes = open
        .read_path("/data/ships/feisar/engineflare.rcsmodel")
        .expect("Feisar's engine flare");
    let model = rcsmodel::Model::parse(&bytes).expect("it parses");
    let material = model.materials.first().expect("its one material");
    assert!(
        material.name.ends_with("engines/flame_test.rcsmaterial"),
        "{}",
        material.name
    );
    let value = |hash: u32| {
        material
            .parameters
            .iter()
            .find(|p| p.hash == hash)
            .unwrap_or_else(|| panic!("{hash:#010x} is not in the table"))
            .value[0]
    };
    let name_hash = oag_rcs::rcsmaterial::name_hash;
    assert_eq!(value(name_hash("power1")), 10.0);
    assert_eq!(value(name_hash("scale1")), 0.3);
    assert_eq!(value(name_hash("min1")), 0.45);
    assert_eq!(value(name_hash("Speed")), 2.0);
    // The two with no preimage, read by the code slot they patch rather than by
    // a name: the alpha term's scale and the colour's.
    assert_eq!(value(0x92fc_84bf), 2.0);
    assert_eq!(value(0x17d9_b3d3), 1.0);
}
