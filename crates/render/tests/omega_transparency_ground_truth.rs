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

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn tech_de_ras_glass_and_cutouts_leave_the_opaque_list() {
    let Some(root) = root() else { return };
    let mut archives: Vec<_> = ARCHIVES
        .iter()
        .filter_map(|a| oag_assets::psarc::Archive::open_file(&root.join(a)).ok())
        .collect();
    let path = "Data/environments/tech_de_ra/track.final.rcsmodel";
    let blob = archives
        .iter_mut()
        .find_map(|a| a.read_path(path).ok())
        .expect("the circuit model is in a base archive");
    let (model, report) = psp2::build("tech_de_ra", &blob, None, &mut |p| {
        archives.iter_mut().find_map(|a| a.read_path(p).ok())
    })
    .expect("builds");
    assert!(model.transparent_draws.len() > 50, "{}", report.describe());
    assert_eq!(report.blended_draws, model.transparent_draws.len());
    assert!(
        model
            .transparent_draws
            .iter()
            .all(|d| d.blend_state.is_some() && d.texture.is_some())
    );
    // The blend is HD's authored pair where HD has one for the name: some draws
    // are not alpha-over, and the count in the report is the draws that are not.
    let default = Some(oag_mesh::mesh_render::TRANSPARENT_BLEND);
    let off_default = model
        .transparent_draws
        .iter()
        .filter(|d| d.blend_state != default)
        .count();
    assert!(off_default > 0, "{}", report.describe());
    assert_eq!(report.inherited_blend_draws, off_default);
    assert!(
        !model.alpha_tested_draws.is_empty(),
        "{}",
        report.describe()
    );
    assert_eq!(model.alpha_test_ref, Some(0.5));
    assert!(model.draws.len() > model.transparent_draws.len());
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

/// Builds every circuit model in `archives` whose path matches `is_track` and
/// returns how many there were and how many drawn colour slots are not
/// pictures (`track_albedo`). psp2 resolves no shader variant (the Vita and PS4
/// programs are unread), so there is no variant clause on these two titles, and
/// its `unresolved_draws` is not ratcheted: Amphiseum alone reports about 2,100,
/// which this census did not take apart.
fn every_circuit_binds_pictures(
    archives: &mut [oag_assets::psarc::Archive],
    is_track: impl Fn(&str) -> bool,
) -> (usize, usize) {
    let mut tracks: Vec<(usize, String)> = Vec::new();
    for (i, a) in archives.iter().enumerate() {
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
        let (model, _) = psp2::build(&name, &blob, None, &mut |p| {
            archives.iter_mut().find_map(|a| a.read_path(p).ok())
        })
        .expect("builds");
        let faults = crate::track_albedo::faults(&model, true);
        if !faults.is_empty() {
            println!("{path}: {}", crate::track_albedo::describe(&faults));
        }
        faulty += faults.len();
    }
    (tracks.len(), faulty)
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
/// (`oag_rcs::rcsmodel::psp2::lighting_only`): 223 slots fell to 149. The counts
/// may shrink further and must never grow.
const VITA_FAULTS: usize = 41;

/// One Omega circuit directory, both directions, against its known gap.
fn omega_circuit(dir: &str, known: usize) {
    let Some(root) = root() else { return };
    let mut archives: Vec<_> = ARCHIVES
        .iter()
        .filter_map(|a| oag_assets::psarc::Archive::open_file(&root.join(a)).ok())
        .collect();
    let marker = format!("/{dir}/track");
    let (n, faulty) = every_circuit_binds_pictures(&mut archives, |p| {
        p.contains(&marker) && p.ends_with(".final.rcsmodel") && !p.contains(".final.pvs")
    });
    println!("{dir}: {n} model(s), {faulty} drawn colour slot(s) that are not pictures");
    assert!(n >= 1, "{dir} was found");
    assert!(faulty <= known, "{dir}: {faulty} > {known}");
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
    28
);
omega_circuit_test!(
    omega_metropia_binds_no_more_lookups_than_known,
    "02_metropia",
    20
);
omega_circuit_test!(
    omega_moa_therma_binds_no_more_lookups_than_known,
    "03_moa_therma",
    10
);
omega_circuit_test!(
    omega_chenghou_binds_no_more_lookups_than_known,
    "04_chenghou_project",
    8
);
omega_circuit_test!(
    omega_ubermall_binds_no_more_lookups_than_known,
    "05_ubermall",
    27
);
omega_circuit_test!(
    omega_sebenco_binds_no_more_lookups_than_known,
    "10_sebenco_climb",
    7
);
omega_circuit_test!(omega_sol_2_binds_no_more_lookups_than_known, "12_sol_2", 10);
omega_circuit_test!(
    omega_anulpha_binds_no_more_lookups_than_known,
    "15_anulpha_pass",
    8
);
omega_circuit_test!(
    omega_amphiseum_binds_no_more_lookups_than_known,
    "amphiseum",
    4
);
omega_circuit_test!(
    omega_modesto_binds_no_more_lookups_than_known,
    "modesto_heights",
    4
);
omega_circuit_test!(
    omega_talons_binds_no_more_lookups_than_known,
    "talons_junction",
    4
);
omega_circuit_test!(
    omega_tech_de_ra_binds_no_more_lookups_than_known,
    "tech_de_ra",
    16
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
    let (n, faulty) = every_circuit_binds_pictures(&mut archives, |p| {
        p.contains("/environments/") && p.ends_with("/track.rcsmodel")
            || p.contains("/environments/") && p.ends_with("/track_reversed.rcsmodel")
    });
    println!("{n} circuit model(s), {faulty} drawn colour slot(s) that are not pictures");
    assert!(n > 4, "2048's circuits were found: {n}");
    assert!(faulty <= VITA_FAULTS, "{faulty} > {VITA_FAULTS}");
}
