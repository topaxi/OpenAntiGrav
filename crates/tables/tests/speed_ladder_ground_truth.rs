//! Which speed classes each title's per-team `handlingstats.xml` authors, read
//! off the files, and that `SpeedClass` names every one of them.
//!
//! `#[ignore]`d: needs the extracted 2048 archives under `data/extracted/vita/`
//! (the `.pkg`s unpacked).

mod common;

use oag_tables::handling::{self, SpeedClass};

const BASES: [&str; 2] = [
    "data/extracted/vita/PCSF00007/base/PSP2/data.psarc",
    "data/extracted/vita/PCSA00015/base/PSP2/data.psarc",
];

/// `(path, class names in document order)` for every per-team
/// `handlingstats.xml` in the archive that carries a `<Stats>` element.
fn ladders(base: &str) -> Option<Vec<(String, Vec<String>)>> {
    let mut data = common::loose(base)?;
    let paths: Vec<String> = data
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with("/handlingstats.xml"))
        .cloned()
        .collect();
    let mut out = Vec::new();
    for path in paths {
        let blob = data.read_path(&path).expect("read");
        let Ok(stats) = handling::from_blob(&blob) else {
            continue;
        };
        out.push((
            path,
            stats.classes.iter().map(|c| c.raw_name.clone()).collect(),
        ));
    }
    Some(out)
}

/// 2048 ships two rosters: the five native teams (`data/HandlingStats/`) author
/// five classes, `SUPERPHANTOM` above Phantom; the twelve HD teams it reships
/// (`data/art/published/hdships/`) author four. The Zone craft (`Zone`,
/// `Zone Battle`, `Detonator`) author no `<Class>` at all.
#[test]
#[ignore = "needs the extracted 2048 archives in data/"]
fn wipeout_2048_native_teams_author_five_classes_and_the_hd_teams_four() {
    let native = ["VENOM", "FLASH", "RAPIER", "PHANTOM", "SUPERPHANTOM"];
    let hd = ["VENOM", "FLASH", "RAPIER", "PHANTOM"];
    let mut seen = 0;
    for base in BASES {
        let Some(all) = ladders(base) else { continue };
        for (path, names) in &all {
            let want: &[&str] = if path.starts_with("data/HandlingStats/") {
                &native
            } else if path.contains("/hdships/") {
                if names.is_empty() {
                    continue;
                }
                &hd
            } else {
                continue;
            };
            assert_eq!(names, want, "{base}: {path}");
            for name in names {
                assert!(
                    SpeedClass::named(name).is_some(),
                    "{path}: {name} is not a rung SpeedClass names"
                );
            }
            seen += 1;
        }
        assert!(
            all.iter()
                .filter(|(p, _)| p.contains("/hdships/Zone") || p.contains("/hdships/Detonator"))
                .all(|(_, names)| names.is_empty()),
            "{base}: the Zone craft author no <Class>"
        );
    }
    if seen == 0 {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but no 2048 archive was found"
        );
    }
}
