//! A 2048 model decodes each texture file once, however many materials name it.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. Run it with `just test-data`.
//!
//! Altima's track resolves 523 material slots to 164 distinct `.gxt` files.
//! Keyed by material, `psp2::build` read and decoded all 523; keyed by path it
//! reads each file once and the materials that share one share its decode
//! (`docs/architecture/load-time.md`). Key the cache by material again and
//! this fails.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use oag_mesh::mesh::rcs::psp2::{self, Animation};

fn package() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007/base/PSP2/data.psarc");
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
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn altima_reads_each_texture_file_once_and_shares_its_decode() {
    let Some(package) = package() else {
        return;
    };
    let mut archive =
        oag_assets::psarc::Archive::open(package.to_str().expect("utf-8")).expect("opens");
    let base = "data/art/published/environments/altima/track";
    let model = archive
        .read_path(&format!("{base}.rcsmodel"))
        .expect("model");
    let skeleton = archive.read_path(&format!("{base}.rcsskeleton")).ok();
    let clip = archive.read_path(&format!("{base}.rcsanimclip")).ok();
    let animation = skeleton
        .as_deref()
        .and_then(|s| Animation::parse(s, clip.as_deref()).ok());

    let mut asked: HashMap<String, usize> = HashMap::new();
    let (model, report) = psp2::build("altima", &model, animation.as_ref(), &mut |p| {
        *asked.entry(p.to_string()).or_default() += 1;
        archive.read_path(p).ok()
    })
    .expect("builds");

    let repeated: Vec<_> = asked
        .iter()
        .filter(|(path, n)| path.ends_with(".gxt") && **n > 1)
        .collect();
    assert!(repeated.is_empty(), "read more than once: {repeated:?}");

    let slots: Vec<_> = model.textures.iter().flatten().collect();
    let mut distinct: Vec<*const _> = slots.iter().map(|t| std::sync::Arc::as_ptr(t)).collect();
    distinct.sort_unstable();
    distinct.dedup();
    println!(
        "{} texture slot(s), {} distinct decode(s); {}",
        slots.len(),
        distinct.len(),
        report.describe()
    );
    assert_eq!(
        report.textures,
        slots.len(),
        "one slot per material, as before"
    );
    assert!(
        distinct.len() < slots.len(),
        "materials that share a file share its decode"
    );
}
