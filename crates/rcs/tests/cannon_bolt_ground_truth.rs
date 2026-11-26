//! Every `HD_Detonator_CannonBolt_Halo` material 2048 and Omega ship reads as the bolt's program.
//!
//! **`#[ignore]`d**: needs `data/extracted/vita/PCSF00007` and `data/extracted/ps4`.
//! `docs/rendering/hd-unlit-programs.md`, "The Detonator cannon bolt on 2048 and Omega".

use std::path::PathBuf;

use oag_assets::psarc::Archive;

/// `(archive, entries)`: every copy of the material the archive holds.
const OMEGA: [(&str, usize); 3] = [
    ("ps4/omega-eu/uroot/data00.psarc", 6),
    ("ps4/omega-eu-patch/uroot/data05.psarc", 6),
    ("ps4/omega-eu-patch/uroot/data08.psarc", 6),
];
const VITA: [(&str, usize); 3] = [
    ("vita/PCSF00007/base/PSP2/data.psarc", 2),
    ("vita/PCSF00007/patch-v104/PSP2/data1.psarc", 2),
    ("vita/PCSF00007/patch-v104/PSP2/data2.psarc", 2),
];

fn path(rel: &str) -> Option<PathBuf> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/extracted")
        .join(rel);
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

/// Each bolt material of the archive and its reading.
fn read(
    rel: &str,
    reader: fn(&[u8]) -> Option<[f32; 3]>,
) -> Option<Vec<(String, Option<[f32; 3]>)>> {
    let path = path(rel)?;
    let mut archive = Archive::open(&path.display().to_string()).expect("archive");
    let names: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| {
            let p = p.to_ascii_lowercase();
            p.contains("hd_detonator_cannonbolt_halo") && p.ends_with(".rcsmaterial")
        })
        .cloned()
        .collect();
    Some(
        names
            .into_iter()
            .map(|name| {
                let bytes = archive.read_path(&name).expect("entry");
                let read = reader(&bytes);
                (name, read)
            })
            .collect(),
    )
}

/// Whether `got` is `want` within the half-float step 2048's literals are stored to.
fn close(got: [f32; 3], want: [f32; 3]) -> bool {
    got.iter().zip(want).all(|(g, w)| (g - w).abs() < 1.0e-5)
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn every_omega_bolt_material_reads_as_the_bolt_program_with_omegas_own_tint() {
    for (rel, count) in OMEGA {
        let Some(found) = read(rel, oag_rcs::gcn::cannon_bolt) else {
            continue;
        };
        assert_eq!(found.len(), count, "{rel}");
        let mut plain = 0;
        for (name, tint) in found {
            let stem = name.trim_end_matches(".rcsmaterial");
            // The `_1`/`_2` files are the PS4 Pro compiles (the tint through VOP3P mixes), which
            // no model references: refused.
            if stem.ends_with("_1") || stem.ends_with("_2") {
                assert_eq!(tint, None, "{rel}: {name}");
                continue;
            }
            plain += 1;
            let tint = tint.unwrap_or_else(|| panic!("{rel}: {name}: refused"));
            assert!(
                close(tint, [0.061_907_5, 0.744_53, 1.0]),
                "{rel}: {name}: {tint:?}"
            );
        }
        assert_eq!(plain, 2, "{rel}: one per weapons directory");
    }
}

#[test]
#[ignore = "needs data/extracted/vita"]
fn every_2048_bolt_material_reads_as_the_bolt_program() {
    for (rel, count) in VITA {
        let Some(found) = read(rel, oag_rcs::gxp::cannon_bolt::cannon_bolt) else {
            continue;
        };
        assert_eq!(found.len(), count, "{rel}");
        for (name, tint) in found {
            let tint = tint.unwrap_or_else(|| panic!("{rel}: {name}: refused"));
            assert!(
                close(tint, [0.061_92, 0.744_628_9, 1.0]),
                "{rel}: {name}: {tint:?}"
            );
        }
    }
}
