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
    assert!(
        !model.alpha_tested_draws.is_empty(),
        "{}",
        report.describe()
    );
    assert_eq!(model.alpha_test_ref, Some(0.5));
    assert!(model.draws.len() > model.transparent_draws.len());
}
