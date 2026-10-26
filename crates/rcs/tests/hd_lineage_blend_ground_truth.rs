//! `psp2::lineage_blend::INHERITED` against the three discs it is read from.
//!
//! The table holds Wipeout HD's authored factor pair for every material name that
//! Omega or 2048 draws in mode 1 (blended), where HD authors one non-default pair
//! for that name and only one. This test rebuilds it from HD's `.rcsmodel` files and
//! from Omega's and 2048's state words, so a row that is dropped, added or edited
//! fails here, and so does a new name on either disc.
//!
//! `#[ignore]`d and never run in CI; `just test-data` runs it.

use crate::rcsmodel_common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use oag_rcs::rcsmodel::{
    self, psp2,
    psp2::lineage_blend::{EQUIVALENT, INHERITED},
};
use rcsmodel_common::image;

const DEFAULT: (u16, u16) = (0x0302, 0x0303);

fn stem(path: &str) -> String {
    path.rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .trim_end_matches(".rcsmaterial")
        .to_ascii_lowercase()
}

/// HD's blended materials: name to the set of pairs it authors.
fn hd_pairs() -> BTreeMap<String, BTreeSet<(u16, u16)>> {
    let image = image().expect("checked by the caller");
    let mut out: BTreeMap<String, BTreeSet<(u16, u16)>> = BTreeMap::new();
    for archive in 0..7 {
        let spec = format!("{}:PS3_GAME/USRDIR/DATA0{archive}.PSARC", image.display());
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
            for m in model.materials.iter().filter(|m| m.state & 3 == 1) {
                out.entry(stem(&m.name))
                    .or_default()
                    .insert((m.src_factor, m.dst_factor));
            }
        }
    }
    out
}

/// What an [`EQUIVALENT`] name's materials are, wherever a model carries one: its name, whether
/// it is blended, and its first picture's file stem.
type Family = BTreeSet<(String, bool, String)>;

/// Names a `psp2` archive's models author in mode 1, and the [`EQUIVALENT`] family beside them.
fn blended_names(path: &Path) -> (BTreeSet<String>, Family) {
    let mut archive = oag_assets::psarc::Archive::open_file(path).expect("the archive opens");
    let models: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".rcsmodel"))
        .cloned()
        .collect();
    let mut out = BTreeSet::new();
    let mut family = Family::new();
    for model in models {
        let Ok(blob) = archive.read_path(&model) else {
            continue;
        };
        let Ok(parsed) = psp2::parse(&blob) else {
            continue;
        };
        for m in &parsed.materials {
            let blended = m.mode() == Some(psp2::material::Mode::Blended);
            if blended {
                out.insert(stem(&m.name));
            }
            if EQUIVALENT.iter().any(|row| row.0 == stem(&m.name)) {
                let picture = m.textures.first().map_or_else(String::new, |t| {
                    let leaf = t.rsplit(['/', '\\']).next().unwrap_or_default();
                    leaf.split('.')
                        .next()
                        .unwrap_or_default()
                        .to_ascii_lowercase()
                });
                family.insert((stem(&m.name), blended, picture));
            }
        }
    }
    (out, family)
}

#[test]
#[ignore = "needs the HD, Omega and 2048 images"]
fn the_inherited_table_is_hds_pair_for_every_blended_name_the_two_titles_share() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/extracted");
    let sources: Vec<_> = (0..5)
        .map(|i| root.join(format!("ps4/omega-eu/uroot/data0{i}.psarc")))
        .chain([root.join("vita/PCSF00007/base/PSP2/data.psarc")])
        .collect();
    if image().is_none() || sources.iter().any(|p| !p.exists()) {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but an image is missing"
        );
        return;
    }
    let hd = hd_pairs();
    let mut names = BTreeSet::new();
    let mut family = Family::new();
    for source in &sources {
        let (blended, found) = blended_names(source);
        names.extend(blended);
        family.extend(found);
    }
    // Every material of the start-grid laser's name, on 2048's base package and Omega's five
    // base archives, is blended and carries the `start_grid_laser_da` picture: the census that
    // lets the whole name take HD's `cf_startbeam_glow` pair.
    let laser = (
        "fc01_effects_vscroll_emissive_alpha_glow".to_string(),
        true,
        "start_grid_laser_da".to_string(),
    );
    assert_eq!(family, Family::from([laser]));
    let derived: Vec<(String, u16, u16)> = names
        .into_iter()
        .filter_map(|name| {
            let pairs = hd.get(&name)?;
            let mut it = pairs.iter();
            let (&pair, None) = (it.next()?, it.next()) else {
                return None;
            };
            (pair != DEFAULT).then_some((name, pair.0, pair.1))
        })
        .collect();
    let table: Vec<(String, u16, u16)> = INHERITED
        .iter()
        .map(|&(n, s, d)| (n.to_string(), s, d))
        .collect();
    assert_eq!(table, derived);
    assert!(table.len() >= 60, "{} rows", table.len());
}
