//! Omega's see-through materials, read off each material's own state word.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. Run it with `just test-data`.
//!
//! A PS4 material carries HD's state word at header `+0x22`; its low two bits
//! select blended (1) or alpha-tested (2). The build routes a textured draw of
//! either kind out of the opaque list. Remove `psp2::transparency::route` and
//! every assertion here fails: Tech De Ra's glass tubes and road panels are
//! 100 % opaque draws again.

use std::path::{Path, PathBuf};

use oag_mesh::mesh::rcs::psp2;
use oag_mesh::mesh::{Model, PROGRAM_EMISSIVE_ADD, PROGRAM_VERTEX_BLEND, slots};
use oag_rcs::rcsmodel::psp2::vertex_blend;

const ARCHIVES: [&str; 5] = [
    "data00.psarc",
    "data01.psarc",
    "data02.psarc",
    "data03.psarc",
    "data04.psarc",
];

fn root() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/ps4/omega-eu/uroot");
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

/// The Vita header carries the same word at `+0x12`, so 2048's circuits route
/// the same way.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn altimas_glass_and_cutouts_leave_the_opaque_list_on_the_vita_too() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007/base/PSP2/data.psarc");
    if !path.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            path.display()
        );
        return;
    }
    let mut archive =
        oag_assets::psarc::Archive::open(path.to_str().expect("utf-8")).expect("opens");
    let blob = archive
        .read_path("data/art/published/environments/altima/track.rcsmodel")
        .expect("model");
    let (model, report) =
        psp2::build("altima", &blob, None, &mut |p| archive.read_path(p).ok()).expect("builds");
    assert!(!model.transparent_draws.is_empty(), "{}", report.describe());
    assert_eq!(report.blended_draws, model.transparent_draws.len());
    assert!(model.draws.len() > model.transparent_draws.len());
}

/// Altima's sea draws under `original` and is the flat colour it was under `off`: the glow
/// table gains the sea's block and the third binding gains the normal map only under `original`.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn altimas_sea_draws_its_law_under_original_and_nothing_under_off() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007/base/PSP2/data.psarc");
    if !path.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            path.display()
        );
        return;
    }
    let mut archive =
        oag_assets::psarc::Archive::open(path.to_str().expect("utf-8")).expect("opens");
    let blob = archive
        .read_path("data/art/published/environments/altima/track.rcsmodel")
        .expect("model");
    let mut build = |law| {
        psp2::build_circuit(
            "altima",
            &blob,
            None,
            &mut |p| archive.read_path(p).ok(),
            law,
            psp2::circuit_pads::HidePads::None,
        )
        .expect("builds")
    };
    let (on, on_report) = build(oag_display::display::Water::Original);
    let (off, off_report) = build(oag_display::display::Water::Off);
    assert_eq!((on_report.sea.drawn, off_report.sea.drawn), (10, 0));
    assert_eq!(off_report.sea.submeshes, 10, "classified either way");
    assert_eq!(
        (on_report.sea.fc17_drawn, on_report.sea.edge_drawn),
        (5, 5),
        "the blended edge's five batches"
    );
    assert_eq!(
        on.emissive.len(),
        off.emissive.len() + 6,
        "two blocks of three"
    );
    let sea = on
        .emissive
        .iter()
        .find(|e| {
            e.program
                == [
                    oag_mesh::mesh::PROGRAM_WATER,
                    oag_mesh::mesh::rcs::VITA_SEA_KIND,
                ]
        })
        .expect("the sea's header entry");
    assert_eq!(sea.scale, 250.0, "Altima's SpecularPower");
    let edge = on
        .emissive
        .iter()
        .find(|e| {
            e.program
                == [
                    oag_mesh::mesh::PROGRAM_WATER,
                    oag_mesh::mesh::rcs::VITA_EDGE_KIND,
                ]
        })
        .expect("the edge's header entry");
    assert_eq!(
        edge.offset, 1.0,
        "the edge's Opacity rides the header's offset"
    );
    assert_eq!(edge.scale, 250.0, "the edge's SpecularPower");
    // The edge's own `Watercolour`, three halves; the pond's `ColourConstant` beside it differs
    // in the third digit (`0.1735`, `0.2195`).
    assert_eq!(
        edge.tint,
        [0.133_178_71, 0.172_607_42, 0.217_651_37],
        "the edge's authored Watercolour"
    );
    assert_eq!(
        sea.tint,
        [0.133_178_71, 0.173_461_91, 0.219_482_42],
        "the pond's ColourConstant"
    );
    assert_eq!(on.vertices.len(), off.vertices.len());
}

/// Builds every circuit model in `archives` whose path matches `is_track`
/// (`track_albedo` counts the colour slots that are not pictures). psp2 resolves no shader variant (the Vita and PS4
/// programs are unread), so there is no variant clause on these two titles, and
/// its `unresolved_draws` is not ratcheted: Amphiseum alone reports about 2,100,
/// which this census did not take apart.
/// Builds every model, and returns how many there were, how many drawn colour slots are not
/// pictures, and the draws left without a picture by the exit of `bind_textures` that left
/// them, summed over the models. Each model's line is printed: `--no-capture` reads the
/// per-circuit counts.
fn every_circuit_binds(
    archives: &mut [oag_assets::psarc::Archive],
    first_track_archive: usize,
    is_track: impl Fn(&str) -> bool,
) -> Walk {
    let mut unbound = psp2::UnboundDraws::default();
    let mut tracks: Vec<(usize, String)> = Vec::new();
    for (i, a) in archives.iter().enumerate().skip(first_track_archive) {
        tracks.extend(
            a.paths()
                .iter()
                .filter(|p| is_track(p))
                .map(|p| (i, p.clone())),
        );
    }
    tracks.sort();
    let mut faulty = 0;
    let mut seas = Vec::new();
    let mut blends = Vec::new();
    let mut track_emissive = Vec::new();
    for (i, path) in &tracks {
        let blob = archives[*i].read_path(path).expect("the model reads");
        let name = path.rsplit('/').nth(1).unwrap_or(path).to_string();
        let (model, report) = psp2::build_circuit(
            &name,
            &blob,
            None,
            &mut |p| archives.iter_mut().find_map(|a| a.read_path(p).ok()),
            oag_display::display::Water::Original,
            psp2::circuit_pads::HidePads::None,
        )
        .expect("builds");
        seas.push((path.clone(), report.sea.clone()));
        blends.push((path.clone(), Blend::of(&blob, &model, &report)));
        track_emissive.push((path.clone(), TrackEmissive::of(&blob, &model)));
        println!(
            "{path}: {} submesh(es), {} unresolved{}",
            report.submeshes,
            report.unresolved_draws,
            report.unbound.describe()
        );
        unbound.add(&report.unbound);
        let faults = crate::track_albedo::faults(&model, true);
        if !faults.is_empty() {
            println!("{path}: {}", crate::track_albedo::describe(&faults));
        }
        faulty += faults.len();
    }
    Walk {
        circuits: tracks.len(),
        faulty,
        unbound,
        seas,
        blends,
        track_emissive,
    }
}

/// What one model's `fc16_phong_alphaspec_normal_emissive` materials built to: the
/// `EmissiveMap` paths the file authors on them, and the third-binding pictures of the draws
/// whose vertices take a unit-tint [`PROGRAM_EMISSIVE_ADD`] entry, lower case, sorted.
struct TrackEmissive {
    authored: Vec<String>,
    bound: Vec<String>,
    draws: usize,
}

impl TrackEmissive {
    fn of(blob: &[u8], model: &Model) -> Self {
        let file = oag_rcs::rcsmodel::psp2::parse(blob).expect("the model parses");
        let mut authored: Vec<String> = file
            .materials
            .iter()
            .filter(|m| {
                let stem = m.name.rsplit(['/', '\\']).next().unwrap_or(&m.name);
                stem.eq_ignore_ascii_case("fc16_phong_alphaspec_normal_emissive.rcsmaterial")
            })
            .filter_map(|m| m.sampler(oag_rcs::rcsmaterial::name_hash("EmissiveMap")))
            .map(str::to_ascii_lowercase)
            .collect();
        let unit = |v: &oag_mesh::mesh::GpuVertex| {
            let index = slots::material_index(v.slots) as usize;
            index != 0
                && model.emissive[index - 1].program[0] == PROGRAM_EMISSIVE_ADD
                && model.emissive[index - 1].tint == [1.0; 3]
        };
        let mut bound = Vec::new();
        let mut draws = 0;
        for draw in model
            .draws
            .iter()
            .chain(&model.transparent_draws)
            .chain(&model.alpha_tested_draws)
        {
            let range = draw.range.start as usize..draw.range.end as usize;
            if !model.indices[range]
                .iter()
                .any(|&i| unit(&model.vertices[i as usize]))
            {
                continue;
            }
            draws += 1;
            if let Some(Some(t)) = draw.texture.and_then(|at| model.pad_masks.get(at)) {
                bound.push(t.label.to_ascii_lowercase());
            }
        }
        for list in [&mut authored, &mut bound] {
            list.sort();
            list.dedup();
        }
        Self {
            authored,
            bound,
            draws,
        }
    }
}

/// The 2048 track surface adds its `EmissiveMap` squared after the light
/// (`docs/formats/gxp.md`, "`fc16_phong_alphaspec_normal_emissive` adds its emissive map"):
/// on every circuit model whose file authors the material, its draws take a unit-tint mode-8
/// entry and bind that picture third. Drop the family from `psp2::emissive_add` and the bound
/// list empties on every such model.
fn the_track_surface_adds_its_emissive_map(walks: &[(String, TrackEmissive)]) {
    let mut models = 0;
    for (path, e) in walks {
        println!(
            "{path}: fc16 emissive {:?}, {} draw(s) bind {:?}",
            e.authored, e.draws, e.bound
        );
        if e.authored.is_empty() {
            continue;
        }
        models += 1;
        assert!(e.draws > 0, "{path}: no draw takes the term");
        assert_eq!(
            e.bound, e.authored,
            "{path}: the third binding against EmissiveMap"
        );
    }
    assert!(models > 0, "no circuit model authors the track surface");
}

/// What one model's `fc07_blend_vertred_lambert_2diff` materials built to, against what the
/// file itself authors (`oag_rcs::rcsmodel::psp2::vertex_blend`).
struct Blend {
    /// Materials the build admitted, and family materials it left to the generic reading.
    admitted: usize,
    unread: usize,
    /// Family materials in the file, by the reader's row.
    authored: usize,
    /// Per red value, the vertices the file authors on the family's submeshes and the vertices
    /// the build drew with that weight.
    reds_authored: Vec<usize>,
    reds_drawn: Vec<usize>,
    /// The `(DiffuseMap, DiffuseMap01)` pairs the file authors and the `(first, third binding)`
    /// pairs the blended draws bind, lower case, sorted.
    pairs_authored: Vec<(String, String)>,
    pairs_bound: Vec<(String, String)>,
    /// Every blended vertex is still lit: the mix replaces the albedo alone.
    lit: bool,
}

impl Blend {
    fn of(blob: &[u8], model: &Model, report: &psp2::Report) -> Self {
        let mut out = Self {
            admitted: report.vertex_blend_materials,
            unread: report.vertex_blend_unread,
            authored: 0,
            reds_authored: vec![0; 256],
            reds_drawn: vec![0; 256],
            pairs_authored: Vec::new(),
            pairs_bound: Vec::new(),
            lit: true,
        };
        let file = oag_rcs::rcsmodel::psp2::parse(blob).expect("the model parses");
        for (index, material) in file.materials.iter().enumerate() {
            let Some(named) = vertex_blend::of(material) else {
                continue;
            };
            out.authored += 1;
            out.pairs_authored.push((
                named.first.to_ascii_lowercase(),
                named.second.to_ascii_lowercase(),
            ));
            for submesh in file.submeshes.iter().filter(|s| s.material == Some(index)) {
                // Red is the colour's first byte, named here and not through the reader's
                // constant, so a reader moved to another lane fails below.
                for [red, ..] in &submesh.vita_vertex_colour1 {
                    out.reds_authored[usize::from(*red)] += 1;
                }
            }
        }
        let blended = |v: &oag_mesh::mesh::GpuVertex| {
            let index = slots::material_index(v.slots) as usize;
            index != 0 && model.emissive[index - 1].program[0] == PROGRAM_VERTEX_BLEND
        };
        for v in model.vertices.iter().filter(|v| blended(v)) {
            out.reds_drawn[(v.texcoord2[0] * 255.0).round() as usize] += 1;
            out.lit &= v.lit == 1.0 && v.slots & slots::NO_SUN == 0;
        }
        let label = |slots: &oag_mesh::mesh::TextureSlots, at: usize| {
            slots
                .get(at)
                .and_then(Option::as_ref)
                .map_or_else(String::new, |t| t.label.to_ascii_lowercase())
        };
        for draw in model
            .draws
            .iter()
            .chain(&model.transparent_draws)
            .chain(&model.alpha_tested_draws)
        {
            let range = draw.range.start as usize..draw.range.end as usize;
            let Some(at) = draw.texture.filter(|_| {
                model.indices[range]
                    .iter()
                    .any(|&i| blended(&model.vertices[i as usize]))
            }) else {
                continue;
            };
            out.pairs_bound
                .push((label(&model.textures, at), label(&model.pad_masks, at)));
        }
        for pairs in [&mut out.pairs_authored, &mut out.pairs_bound] {
            pairs.sort();
            pairs.dedup();
        }
        out
    }
}

/// `fc07_blend_vertred_lambert_2diff` on each model of a walk: every family material the file
/// authors is admitted, each blended draw binds its `DiffuseMap` first and its `DiffuseMap01`
/// third, and the weights the vertices carry are the file's `VertexColour1` reds, value for
/// value. `want` names the circuits that carry the family and how many materials each has;
/// every other model has none. Drop `psp2::vertex_blend::plan`, bind another sampler third or
/// read another lane of the colour and a count, a pair or a histogram here moves.
fn the_blend_family_mixes_its_second_picture_by_the_vertex_red(
    blends: &[(String, Blend)],
    want: &[(&str, usize)],
) {
    for (path, blend) in blends {
        let circuit = path.rsplit('/').nth(1).unwrap_or(path);
        let materials = want
            .iter()
            .find(|(name, _)| *name == circuit)
            .map_or(0, |&(_, materials)| materials);
        println!(
            "{path}: {} vertex-blend material(s) of {} authored, {} blended vertices",
            blend.admitted,
            blend.authored,
            blend.reds_drawn.iter().sum::<usize>()
        );
        assert_eq!(blend.admitted, materials, "{path}");
        assert_eq!(blend.unread, 0, "{path}");
        assert!(blend.lit, "{path}: a blended vertex is unlit");
        if materials == 0 {
            assert_eq!(blend.reds_drawn.iter().sum::<usize>(), 0, "{path}");
            assert!(blend.pairs_bound.is_empty(), "{path}");
            continue;
        }
        assert_eq!(blend.authored, materials, "{path}");
        assert_eq!(
            blend.pairs_bound, blend.pairs_authored,
            "{path}: (first, third) against (DiffuseMap, DiffuseMap01)"
        );
        assert_eq!(
            blend.reds_drawn, blend.reds_authored,
            "{path}: the weights drawn against VertexColour1's red"
        );
        let mixed: usize = blend.reds_drawn[1..255].iter().sum();
        assert!(
            blend.reds_drawn[0] > 0 && mixed > 0,
            "{path}: the family is painted, not constant: {} at 0, {mixed} between",
            blend.reds_drawn[0]
        );
    }
}

/// What `every_circuit_binds` found over its circuit models.
struct Walk {
    circuits: usize,
    faulty: usize,
    unbound: psp2::UnboundDraws,
    /// Each model's path and its 2048-sea census (`psp2::water`), the PS4 ones all zero.
    seas: Vec<(String, psp2::water::Census)>,
    /// Each model's path and its vertex-blend family (`psp2::vertex_blend`).
    blends: Vec<(String, Blend)>,
    /// Each model's path and its track-surface emissive add (`psp2::emissive_add`).
    track_emissive: Vec<(String, TrackEmissive)>,
}

/// Altima's road binds its `DiffuseMap`, not the first sampler listed.
///
/// `road_alphaspec_normal_emissive_panelmap1` names a 1-channel `R16F` panel
/// map ahead of its `DiffuseMap`; binding the first left the road with no
/// picture (flat pink under Omega's sun, 2026-10-19). Drop the named-sampler
/// choice in `Material::diffuse_texture` and 241 road draws are unresolved
/// again: 690 against the 449 below, which are the other causes (`fc01` flat
/// materials and the like, not taken apart). The count may shrink, never grow.
const ALTIMA_UNRESOLVED: usize = 449;

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn omega_altimas_road_binds_its_diffuse_map() {
    let Some(root) = root() else { return };
    let mut archives: Vec<_> = ARCHIVES
        .iter()
        .filter_map(|a| oag_assets::psarc::Archive::open_file(&root.join(a)).ok())
        .collect();
    let path = "Data/environments2048/altima/track.final.rcsmodel";
    let blob = archives
        .iter_mut()
        .find_map(|a| a.read_path(path).ok())
        .expect("altima's model is in a base archive");
    let (_, report) = psp2::build("altima", &blob, None, &mut |p| {
        archives.iter_mut().find_map(|a| a.read_path(p).ok())
    })
    .expect("builds");
    println!("altima: {}", report.describe());
    assert!(
        report.unresolved_draws <= ALTIMA_UNRESOLVED,
        "{} unresolved draws: {}",
        report.unresolved_draws,
        report.describe()
    );
}

/// Drawn colour slots of each Omega circuit that are not pictures, and 2048's:
/// **a ratchet over a known gap, not a claim of zero.**
///
/// 2048's count fell from 66 to 41 on 2026-10-19 (`normal-maps-as-colour`): the
/// build reads each material's own fragment programs
/// (`oag_rcs::gxp::sampler_use`) and binds no picture from a sampler they read
/// only for lighting, drawing the constant colour their ambient-only program
/// names instead. That took `constantdiffuse_specular_normal` (and its `New`),
/// `ds_booth_glass` and the water shaders out of the list. What is left is
/// `m_diffuse_normal_constantspecular` (its ambient-only program does sample
/// `Norm`/`Texture1`, so the programs do not call it lighting-only), the
/// Altima water edge (same reason), and `temp_diffuse` on Square and Subway.
/// Omega's PS4 programs are not read for sampler use, so it takes 2048's answer
/// only for the three families whose Omega instance record is 2048's exactly
/// (`oag_rcs::rcsmodel::psp2::lighting_only`): 223 slots fell to 149.
/// 2026-10-31 (`2048-normal-maps-re`): `m_diffuse_normal_constantspecular`
/// draws `Constant1` times its map's alpha (`mesh::rcs::psp2::alpha_product`,
/// read off both titles' programs): 2048 41 to 10, Omega 149 to 67. Left on
/// 2048: Chenghou's `diffuse_normal_specular_emmissivenew` (7), the Altima
/// water edge and `temp_diffuse` on Square and Subway. Omega's Altima counts one for its Zone
/// model's water edge, the slot 2048 counts: the census is of what the colour slot binds, and
/// the edge's entry reads that map for its alpha alone. The counts may shrink further and must
/// never grow.
const VITA_FAULTS: usize = 3;

/// One Omega circuit directory, both directions, against its known gap.
fn omega_circuit(dir: &str, known: usize) {
    let Some(root) = root() else { return };
    let mut archives: Vec<_> = ARCHIVES
        .iter()
        .filter_map(|a| oag_assets::psarc::Archive::open_file(&root.join(a)).ok())
        .collect();
    let marker = format!("/{dir}/track");
    let Walk {
        circuits: n,
        faulty,
        unbound,
        seas,
        blends,
        ..
    } = every_circuit_binds(&mut archives, 0, |p| {
        p.contains(&marker) && p.ends_with(".final.rcsmodel") && !p.contains(".final.pvs")
            // Altima's Zone model rides its circuit's walk: the one Zone model whose water
            // edge is pinned (`omegas_reflective_seas_are_the_two_tap_batches`).
            || dir == "altima" && p.ends_with("/altima/trackZone.rcsmodel")
    });
    println!("{dir}: {n} model(s), {faulty} drawn colour slot(s) that are not pictures");
    omegas_reflective_seas_are_the_two_tap_batches(dir, &seas);
    // Omega's copies of the family are not wired (`psp2::vertex_blend`): no PS4 model blends.
    the_blend_family_mixes_its_second_picture_by_the_vertex_red(&blends, &[]);
    assert!(n >= 1, "{dir} was found");
    assert!(faulty <= known, "{dir}: {faulty} > {known}");
    only_by_design(dir, &unbound);
}

/// Omega's reflective seas, by their PS4 programs' declared interface and taps
/// (`oag_rcs::gcn::sea`, `psp2::water`): the batches of `reflectplane_mid`, `_edge`, `_pond`,
/// `_dc_seawater_new` and `_dc_seawater_tow`, counted per circuit model and all drawn under
/// `original`. Drop the PS4 classification and every count falls to zero. Park's
/// `reflectplane_dc_seawater` and the HD circuits' seas carry HD's interface, the shore strip
/// (`reflectplane_rt_seawateredging`) reads no reflection: none is this law, and no PS4 model
/// matches 2048's GXP laws.
///
/// Altima's Zone model keeps 2048's blended edge (`oag_rcs::gcn::scrolled_edge`): its five
/// batches draw the alpha chain, and no circuit model has one.
fn omegas_reflective_seas_are_the_two_tap_batches(
    dir: &str,
    seas: &[(String, psp2::water::Census)],
) {
    // (circuit directory, batches, of which the blended edge)
    const WANT: [(&str, usize, usize); 5] = [
        ("altima", 10, 5),
        ("mall", 8, 0),
        ("tower", 5, 0),
        ("square", 7, 0),
        ("bridge", 13, 0),
    ];
    let (batches, edge) = WANT
        .iter()
        .find(|(name, ..)| *name == dir)
        .map_or((0, 0), |&(_, batches, edge)| (batches, edge));
    for (path, sea) in seas {
        println!("{path}: {sea:?}");
        let zone = path.ends_with("/trackZone.rcsmodel");
        let (batches, edge, zone_edge) = if zone { (0, 0, 5) } else { (batches, edge, 0) };
        assert_eq!(sea.reflective, batches, "{path}: {sea:?}");
        assert_eq!(sea.reflective_drawn, batches, "{path}: {sea:?}");
        assert_eq!(sea.reflective_edge_drawn, edge, "{path}: {sea:?}");
        assert_eq!(sea.zone_edge, zone_edge, "{path}: {sea:?}");
        assert_eq!(sea.zone_edge_drawn, zone_edge, "{path}: {sea:?}");
        assert_eq!(
            sea.submeshes, 0,
            "{path}: a PS4 model matches no GXP law: {sea:?}"
        );
    }
}

/// A draw that names a texture of its own never goes without it, and a texture that is
/// named is in the archive set and decodes: what is left unbound is a placeholder, a Zone
/// colour, glass, or a material that names no texture (`fc05_colour_lambert` and kin), which
/// draw flat on the disc's own terms.
fn only_by_design(what: &str, unbound: &psp2::UnboundDraws) {
    assert_eq!(unbound.no_diffuse_path, 0, "{what}: {unbound:?}");
    assert_eq!(unbound.archive_miss, 0, "{what}: {unbound:?}");
    assert_eq!(unbound.decode_refused, 0, "{what}: {unbound:?}");
}

macro_rules! omega_circuit_test {
    ($name:ident, $dir:literal, $known:expr) => {
        #[test]
        #[ignore = "needs data/extracted/ps4/omega-eu"]
        fn $name() {
            omega_circuit($dir, $known);
        }
    };
}

omega_circuit_test!(
    omega_vineta_k_binds_no_more_lookups_than_known,
    "01_vineta_k",
    14
);
omega_circuit_test!(
    omega_metropia_binds_no_more_lookups_than_known,
    "02_metropia",
    14
);
omega_circuit_test!(
    omega_moa_therma_binds_no_more_lookups_than_known,
    "03_moa_therma",
    2
);
omega_circuit_test!(
    omega_chenghou_binds_no_more_lookups_than_known,
    "04_chenghou_project",
    0
);
omega_circuit_test!(
    omega_ubermall_binds_no_more_lookups_than_known,
    "05_ubermall",
    20
);
omega_circuit_test!(
    omega_sebenco_binds_no_more_lookups_than_known,
    "10_sebenco_climb",
    0
);
omega_circuit_test!(omega_sol_2_binds_no_more_lookups_than_known, "12_sol_2", 2);
omega_circuit_test!(
    omega_anulpha_binds_no_more_lookups_than_known,
    "15_anulpha_pass",
    0
);
omega_circuit_test!(
    omega_amphiseum_binds_no_more_lookups_than_known,
    "amphiseum",
    0
);
omega_circuit_test!(
    omega_modesto_binds_no_more_lookups_than_known,
    "modesto_heights",
    0
);
omega_circuit_test!(
    omega_talons_binds_no_more_lookups_than_known,
    "talons_junction",
    0
);
omega_circuit_test!(
    omega_tech_de_ra_binds_no_more_lookups_than_known,
    "tech_de_ra",
    12
);
omega_circuit_test!(omega_altima_binds_no_more_lookups_than_known, "altima", 1);
omega_circuit_test!(omega_bridge_binds_no_more_lookups_than_known, "bridge", 0);
omega_circuit_test!(omega_mall_binds_no_more_lookups_than_known, "mall", 0);
omega_circuit_test!(omega_tower_binds_no_more_lookups_than_known, "tower", 0);
omega_circuit_test!(omega_arena_binds_no_more_lookups_than_known, "arena", 1);
omega_circuit_test!(omega_park_binds_no_more_lookups_than_known, "park", 0);
omega_circuit_test!(omega_square_binds_no_more_lookups_than_known, "square", 1);
omega_circuit_test!(omega_subway_binds_no_more_lookups_than_known, "subway", 1);

/// Omega's plain `diffuse_normal_specular_emmissive` adds `Texture2`'s alpha times `Constant1`
/// after the light, as its ambient pixel program does (`docs/formats/gxp.md`,
/// "`diffuse_normal_specular_emmissivenew`"): Chenghou's start gantry admits its slots.
#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn omega_chenghous_start_gantry_adds_its_emissive() {
    let Some(root) = root() else { return };
    let mut archives: Vec<_> = ARCHIVES
        .iter()
        .filter_map(|a| oag_assets::psarc::Archive::open_file(&root.join(a)).ok())
        .collect();
    let path = "Data/environments/04_chenghou_project/track.final.rcsmodel";
    let blob = archives
        .iter_mut()
        .find_map(|a| a.read_path(path).ok())
        .expect("chenghou's model is in a base archive");
    let (_, report) = psp2::build("chenghou", &blob, None, &mut |p| {
        archives.iter_mut().find_map(|a| a.read_path(p).ok())
    })
    .expect("builds");
    assert!(
        report.emissive_add_materials >= 6 && report.emissive_add_unread == 0,
        "{}",
        report.describe()
    );
}

/// Amphiseum's stands and pad strips drew white times light: `metallic` and
/// `diffuse_spec_constant` have no colour texture, and their GCN programs end
/// `light * constant + specular` (`docs/formats/rcsmaterial.md`, "Omega's constant diffuse
/// families"). The loader binds the constant, encoded; the `emissive` family (919 spectator
/// figures) is its `EmissiveColour`, unlit. Drop `psp2::flat_constant` and both counts fall to 0.
#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn omega_amphiseums_constant_families_draw_their_constants() {
    let Some(root) = root() else { return };
    let mut archives: Vec<_> = ARCHIVES
        .iter()
        .filter_map(|a| oag_assets::psarc::Archive::open_file(&root.join(a)).ok())
        .collect();
    let path = "Data/environments/amphiseum/track.final.rcsmodel";
    let blob = archives
        .iter_mut()
        .find_map(|a| a.read_path(path).ok())
        .expect("amphiseum's model is in a base archive");
    let (model, report) = psp2::build("amphiseum", &blob, None, &mut |p| {
        archives.iter_mut().find_map(|a| a.read_path(p).ok())
    })
    .expect("builds");
    // The 907 spectator placeholders (`emissive`, `EmissiveColour` 0.5566) are crowd-entity
    // shapes: each is replaced by a bind-pose rig, every entity name parses to a group.
    let decoded = oag_rcs::rcsmodel::psp2::parse(&blob).expect("parses");
    assert_eq!(
        super::omega_crowd_shadowed_check::crowd_entity_submeshes(&decoded).0,
        907,
        "Amphiseum's crowd-entity shapes"
    );
    super::omega_crowd_shadowed_check::check_crowd_entities_hidden(
        "amphiseum",
        &decoded,
        &model,
        &report,
    );
    assert_eq!(
        report.constant_diffuse_materials,
        29,
        "{}",
        report.describe()
    );
    assert_eq!(
        report.emissive_colour_submeshes,
        919,
        "{}",
        report.describe()
    );
    let bound = |label: &str| {
        model
            .textures
            .iter()
            .flatten()
            .any(|t| t.label == format!("constant diffuse {label}"))
    };
    assert!(bound("512f8e65"), "metallic's DiffuseColour");
    assert!(bound("515e298e"), "diffuse_spec_constant's diffuse");
}

/// 2048's circuits, the Vita package.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn the_2048_circuits_bind_no_more_lookups_than_the_known_gap() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007/base/PSP2/data.psarc");
    if !path.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            path.display()
        );
        return;
    }
    let mut archives =
        vec![oag_assets::psarc::Archive::open(path.to_str().expect("utf-8")).expect("opens")];
    let walk = every_circuit_binds(&mut archives, 0, |p| {
        p.contains("/environments/") && p.ends_with("/track.rcsmodel")
            || p.contains("/environments/") && p.ends_with("/track_reversed.rcsmodel")
    });
    let Walk {
        circuits: n,
        faulty,
        unbound,
        ..
    } = walk;
    println!("{n} circuit model(s), {faulty} drawn colour slot(s) that are not pictures");
    the_2048_seas_are_the_fc15_and_fc17_batches(&walk.seas);
    the_track_surface_adds_its_emissive_map(&walk.track_emissive);
    // (circuit directory, family materials), the census of `docs/formats/gxp.md`.
    the_blend_family_mixes_its_second_picture_by_the_vertex_red(
        &walk.blends,
        &[
            ("altima", 21),
            ("arena", 11),
            ("cathedral", 4),
            ("park", 68),
            ("square", 11),
            ("subway", 1),
            ("Moa_Therma", 1),
        ],
    );
    assert!(n > 4, "2048's circuits were found: {n}");
    assert!(faulty <= VITA_FAULTS, "{faulty} > {VITA_FAULTS}");
    only_by_design("2048", &unbound);
}

/// 2048's own seas, by the programs' declared interface (`psp2::water`): the batches of
/// `fc15_effects_vscroll_water` and of the two `fc17` materials, counted per circuit and all
/// drawn. Drop `Plan`'s classification or the `build_circuit` law and every count here falls to
/// zero. The other circuits' seas (`reflectplane_*`, pictures) are not these laws and stay 0.
fn the_2048_seas_are_the_fc15_and_fc17_batches(seas: &[(String, psp2::water::Census)]) {
    // (circuit, fc15 batches, fc17 batches of which the blended edge)
    const WANT: [(&str, usize, usize, usize); 5] = [
        ("altima", 5, 5, 5),
        ("mall", 3, 0, 0),
        ("tower", 4, 0, 0),
        ("square", 4, 0, 0),
        ("park", 0, 4, 0),
    ];
    for (path, sea) in seas {
        let circuit = path.rsplit('/').nth(1).unwrap_or(path);
        let (pond, fc17, edge) = WANT
            .iter()
            .find(|(name, ..)| {
                *name == circuit && path.contains("/environments/") && !path.contains("DLC")
            })
            .map_or((0, 0, 0), |&(_, pond, fc17, edge)| (pond, fc17, edge));
        assert_eq!(sea.submeshes, pond + fc17, "{path}: {sea:?}");
        assert_eq!(sea.drawn, pond + fc17, "{path}: {sea:?}");
        assert_eq!(sea.fc17_drawn, fc17, "{path}: {sea:?}");
        assert_eq!(sea.edge_drawn, edge, "{path}: {sea:?}");
        assert_eq!(sea.left_flat, 0, "{path}: {sea:?}");
    }
    assert_eq!(seas.iter().map(|(_, s)| s.drawn).sum::<usize>(), 25);
    assert_eq!(seas.iter().map(|(_, s)| s.fc17_drawn).sum::<usize>(), 9);
    assert_eq!(seas.iter().map(|(_, s)| s.edge_drawn).sum::<usize>(), 5);
}

/// 2048's DLC circuits and the patch's re-shipped ones, the two DLC packages and the two patch
/// archives over the base archive for the textures they borrow. Their materials are the ones the base circuits' were: no named texture is missing.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn the_2048_dlc_and_patch_circuits_bind_every_texture_they_name() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007");
    let paths = [
        "base/PSP2/data.psarc",
        "dlc1/PSP2/dlc1.psarc",
        "dlc2/PSP2/dlc2.psarc",
        "patch-v104/PSP2/data1.psarc",
        "patch-v104/PSP2/data2.psarc",
    ];
    if !root.join(paths[1]).exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            root.display()
        );
        return;
    }
    let mut archives: Vec<_> = paths
        .iter()
        .map(|p| {
            oag_assets::psarc::Archive::open(root.join(p).to_str().expect("utf-8")).expect("opens")
        })
        .collect();
    let Walk {
        circuits: n,
        unbound,
        seas,
        ..
    } = every_circuit_binds(&mut archives, 1, |p| {
        p.contains("/environments/") && p.ends_with("/track.rcsmodel")
            || p.contains("/environments/") && p.ends_with("/track_reversed.rcsmodel")
    });
    assert!(n >= 4, "the DLC circuits were found: {n}");
    only_by_design("2048 DLC and patch", &unbound);
    // The DLC circuits' seas are HD's `reflectplane_dc_seawater` and `dc_seawater`, two taps
    // each: not 2048's one-tap law, so none is classified as it. The patch re-ships Park, whose
    // four `fc17` batches are the base circuit's.
    let park = psp2::water::Census {
        submeshes: 4,
        drawn: 4,
        fc17_drawn: 4,
        ..Default::default()
    };
    assert!(
        seas.iter().all(|(path, sea)| {
            *sea == if path.contains("/environments/park/") {
                park.clone()
            } else {
                Default::default()
            }
        }),
        "{seas:?}"
    );
}
