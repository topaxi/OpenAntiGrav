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
) -> (usize, usize, psp2::UnboundDraws) {
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
    for (i, path) in &tracks {
        let blob = archives[*i].read_path(path).expect("the model reads");
        let name = path.rsplit('/').nth(1).unwrap_or(path).to_string();
        let (model, report) = psp2::build(&name, &blob, None, &mut |p| {
            archives.iter_mut().find_map(|a| a.read_path(p).ok())
        })
        .expect("builds");
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
    (tracks.len(), faulty, unbound)
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
/// water edge and `temp_diffuse` on Square and Subway. The counts may shrink
/// further and must never grow.
const VITA_FAULTS: usize = 10;

/// One Omega circuit directory, both directions, against its known gap.
fn omega_circuit(dir: &str, known: usize) {
    let Some(root) = root() else { return };
    let mut archives: Vec<_> = ARCHIVES
        .iter()
        .filter_map(|a| oag_assets::psarc::Archive::open_file(&root.join(a)).ok())
        .collect();
    let marker = format!("/{dir}/track");
    let (n, faulty, unbound) = every_circuit_binds(&mut archives, 0, |p| {
        p.contains(&marker) && p.ends_with(".final.rcsmodel") && !p.contains(".final.pvs")
    });
    println!("{dir}: {n} model(s), {faulty} drawn colour slot(s) that are not pictures");
    assert!(n >= 1, "{dir} was found");
    assert!(faulty <= known, "{dir}: {faulty} > {known}");
    only_by_design(dir, &unbound);
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
omega_circuit_test!(omega_altima_binds_no_more_lookups_than_known, "altima", 0);
omega_circuit_test!(omega_mall_binds_no_more_lookups_than_known, "mall", 0);
omega_circuit_test!(omega_tower_binds_no_more_lookups_than_known, "tower", 0);
omega_circuit_test!(omega_arena_binds_no_more_lookups_than_known, "arena", 1);
omega_circuit_test!(omega_park_binds_no_more_lookups_than_known, "park", 0);
omega_circuit_test!(omega_square_binds_no_more_lookups_than_known, "square", 1);
omega_circuit_test!(omega_subway_binds_no_more_lookups_than_known, "subway", 1);

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
    let (n, faulty, unbound) = every_circuit_binds(&mut archives, 0, |p| {
        p.contains("/environments/") && p.ends_with("/track.rcsmodel")
            || p.contains("/environments/") && p.ends_with("/track_reversed.rcsmodel")
    });
    println!("{n} circuit model(s), {faulty} drawn colour slot(s) that are not pictures");
    assert!(n > 4, "2048's circuits were found: {n}");
    assert!(faulty <= VITA_FAULTS, "{faulty} > {VITA_FAULTS}");
    only_by_design("2048", &unbound);
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
    let (n, _, unbound) = every_circuit_binds(&mut archives, 1, |p| {
        p.contains("/environments/") && p.ends_with("/track.rcsmodel")
            || p.contains("/environments/") && p.ends_with("/track_reversed.rcsmodel")
    });
    assert!(n >= 4, "the DLC circuits were found: {n}");
    only_by_design("2048 DLC and patch", &unbound);
}
