//! Every `electricity.rcsmaterial` 2048 ships reads as the Detonator mine's visualiser program.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content; run with `just test-data`.
//!
//! The census (`docs/ghidra/functions/ps3-hdfury-eu/detonator-mine-draw.md`, "The visualiser,
//! wired", the 2048 line): `data/Weapons/` and `data/Weapons2048/` in the base `data.psarc` and
//! the v1.04 patch's `data1.psarc` and `data2.psarc`, EU and USA, twelve files of six fragment
//! programs each. Every one must read with the colour tap's shift `0.3` and the three squared
//! taps; only the entries are read, not the archives.

use std::path::Path;

use oag_rcs::gxp;

const ARCHIVES: [&str; 6] = [
    "vita/PCSF00007/base/PSP2/data.psarc",
    "vita/PCSF00007/patch-v104/PSP2/data1.psarc",
    "vita/PCSF00007/patch-v104/PSP2/data2.psarc",
    "vita/PCSA00015/base/PSP2/data.psarc",
    "vita/PCSA00015/patch-v104/PSP2/data1.psarc",
    "vita/PCSA00015/patch-v104/PSP2/data2.psarc",
];

const FILES: [&str; 2] = [
    "data/Weapons/Materials/electricity.rcsmaterial",
    "data/Weapons2048/Materials/electricity.rcsmaterial",
];

#[test]
#[ignore = "needs the extracted 2048 packages in data/extracted/vita/"]
fn every_2048_electricity_program_reads_as_the_visualiser() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/extracted");
    let mut files = 0;
    let mut programs = 0;
    for name in ARCHIVES {
        let path = root.join(name);
        let Ok(mut archive) = oag_assets::psarc::Archive::open(&path.display().to_string()) else {
            assert!(
                std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
                "OAG_REQUIRE_GAME_DATA is set but {} is missing",
                path.display()
            );
            println!("skipping: {} not present", path.display());
            return;
        };
        for file in FILES {
            let blob = archive
                .read_path(file)
                .unwrap_or_else(|e| panic!("{name}:{file} ships: {e}"));
            files += 1;
            for (at, parsed) in gxp::programs(&blob) {
                let p = parsed.unwrap_or_else(|e| panic!("{name}:{file} @{at:#x}: {e}"));
                if !p.is_fragment() || p.parameter(gxp::electricity::ELECTRICITY).is_none() {
                    continue;
                }
                let read = gxp::electricity::program(&blob[at..at + p.size], &p)
                    .unwrap_or_else(|| panic!("{name}:{file} @{at:#x} does not read"));
                assert_eq!(read.colour_tap_shift, 0.3, "{name}:{file} @{at:#x}");
                assert!(read.squared_taps, "{name}:{file} @{at:#x}");
                programs += 1;
            }
            assert!(
                gxp::electricity::electricity(&blob).is_some(),
                "{name}:{file}"
            );
        }
    }
    assert_eq!((files, programs), (12, 72));
}
