//! Disc-wide check: wherever one `.rcsmaterial` ships both a `Static` and a
//! `RigidBody` (or `StaticQuake`) row at the **same** feature hash, is the
//! fragment block always the same program - so that falling back from
//! `Class::Static` to another class in `skin::variants()` can never change
//! *shading*, only which class's key happened to match?
use oag_assets::psarc;
use oag_rcs::rcsmaterial::{Class, RcsMaterial};

const ARCHIVES: &[&str] = &[
    "PS3_GAME/USRDIR/DATA00.PSARC",
    "PS3_GAME/USRDIR/DATA01.PSARC",
    "PS3_GAME/USRDIR/DATA02.PSARC",
    "PS3_GAME/USRDIR/DATA03.PSARC",
    "PS3_GAME/USRDIR/DATA04.PSARC",
    "PS3_GAME/USRDIR/DATA05.PSARC",
    "PS3_GAME/USRDIR/DATA06.PSARC",
];

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());

    let mut files = 0usize;
    let mut pairs_checked = 0usize;
    let mut mismatches = 0usize;
    for archive in ARCHIVES {
        let spec = format!("{image}:{archive}");
        let Ok(mut ar) = psarc::Archive::open(&spec) else {
            continue;
        };
        let paths: Vec<String> = ar
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmaterial"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(data) = ar.read_path(&path) else {
                continue;
            };
            let Ok(parsed) = RcsMaterial::parse(&data) else {
                continue;
            };
            files += 1;
            for &a in &Class::ALL {
                for &b in &Class::ALL {
                    if a >= b {
                        continue;
                    }
                    for va in parsed.variants.iter().filter(|v| v.class == Some(a)) {
                        let Some(vb) = parsed
                            .variants
                            .iter()
                            .find(|v| v.class == Some(b) && v.feature_hash == va.feature_hash)
                        else {
                            continue;
                        };
                        pairs_checked += 1;
                        if va.fragment.program_hash != vb.fragment.program_hash {
                            mismatches += 1;
                            println!(
                                "MISMATCH {path}: {a:?} fragment {:#x} != {b:?} fragment {:#x} \
                                 at feature {:#010x}",
                                va.fragment.program_hash, vb.fragment.program_hash, va.feature_hash,
                            );
                        }
                    }
                }
            }
        }
    }
    println!(
        "{files} .rcsmaterial file(s), {pairs_checked} same-feature cross-class pair(s) checked, \
         {mismatches} fragment mismatch(es)"
    );
    Ok(())
}
