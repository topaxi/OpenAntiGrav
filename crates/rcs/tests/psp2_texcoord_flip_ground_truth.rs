//! Which of 2048's material files write `1 - v` into their diffuse coordinate, over every
//! `.rcsmaterial` of the base package and both DLC packages.
//!
//! **`#[ignore]`d and never run in CI.** It needs the Vita packages; run it with
//! `just test-data`. See `oag_rcs::gxp::texcoord_flip` and `docs/formats/2048-material-params.md`,
//! "The track wall that writes `1 - v`".
//!
//! Exactly four files do, all in `dlc2.psarc`, and the reader finds the same four: drop it (or
//! widen it to a negation that is not the flip) and the set changes.

use std::collections::BTreeSet;
use std::path::Path;

use oag_rcs::gxp::texcoord_flip::flips_v;

/// Circuit directory and file stem of every material file whose vertex programs flip.
const FLIPPING: [(&str, &str); 4] = [
    ("amphiseum", "track_surface"),
    ("amphiseum", "track_surface_no_emissive"),
    ("modesto_heights", "track_wall"),
    ("tech_de_ra", "track_surface"),
];

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn four_material_files_flip_the_diffuse_coordinate() {
    let mut found: BTreeSet<(String, String)> = BTreeSet::new();
    let mut files = 0usize;
    for (package, archive) in [
        ("base", "data.psarc"),
        ("dlc1", "dlc1.psarc"),
        ("dlc2", "dlc2.psarc"),
    ] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/extracted/vita/PCSF00007")
            .join(package)
            .join("PSP2")
            .join(archive);
        if !path.exists() {
            assert!(
                std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
                "OAG_REQUIRE_GAME_DATA is set but {} is missing",
                path.display()
            );
            return;
        }
        let mut open =
            oag_assets::psarc::Archive::open(path.to_str().expect("utf-8")).expect("opens");
        let entries: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.to_ascii_lowercase().ends_with(".rcsmaterial"))
            .cloned()
            .collect();
        for entry in entries {
            files += 1;
            let blob = open.read_path(&entry).expect("reads");
            if flips_v(&blob) {
                let parts: Vec<&str> = entry.split('/').collect();
                let stem = parts[parts.len() - 1].trim_end_matches(".rcsmaterial");
                found.insert((
                    parts[parts.len() - 3].to_ascii_lowercase(),
                    stem.to_ascii_lowercase(),
                ));
            }
        }
    }
    println!("{files} material files, flipping: {found:?}");
    let want: BTreeSet<(String, String)> = FLIPPING
        .iter()
        .map(|(c, s)| ((*c).to_string(), (*s).to_string()))
        .collect();
    assert_eq!(found, want);
}
