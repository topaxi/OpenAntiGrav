//! Validates [`weapons::ai`](oag_tables::weapons::ai) against the real
//! `Data\XML\WeaponAIstats.xml`.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! `docs/ghidra/functions/psp-pulse-usa/ai-stats.md` records the file as
//! nearly uniform - every weapon authors `absorb="1.0"`, every weapon but the
//! Plasma and the Quake authors one `useAgainstAI`/`useAgainstPlayer` pair, and
//! those two author a different, shared pair of their own. That is a
//! **relational** claim ("exactly these two differ, and only from each other"),
//! and it is what this test checks - never a literal value, which ADR-0006
//! forbids committing.
//!
//! It is also the check that `weapons::ai`'s own name-to-[`Weapon`] table
//! landed on the right rows: `ai-stats.md` found the file's own element order
//! is a *fourth* id
//! space, distinct from `craft+0x1bc` and from [`Weapon::ALL`]'s pool order, at
//! exactly the three weapons (Turbo/Shield/Cannon) this test does not rely on
//! being in any particular order. If keying by name ever slipped and pulled in
//! that numeric id space instead, one of Plasma or Quake would silently read a
//! neighbour's row and stop matching the other - the shared-pair assertion
//! below would catch that even without knowing what the numbers are.

use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::wad::{self, Compression, Directory};
use oag_tables::weapons::{self, Weapon};

const PSP_IMAGE: &str = "pulse-psp-usa.chd";
const PSP_ARCHIVE: &str = "PSP_GAME/USRDIR/Data.wad";

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

/// `Data\XML\WeaponAIstats.xml`, off the disc.
fn blob() -> Option<Vec<u8>> {
    let path = image(PSP_IMAGE)?;
    let mut disc = DiscImage::open(&path).expect("open");
    let archive = disc
        .entries()
        .expect("entries")
        .iter()
        .find(|e| e.path == PSP_ARCHIVE)
        .expect("Data.wad present")
        .clone();

    let header = disc
        .read_entry_range(&archive, 0, wad::HEADER_LEN as u64)
        .expect("header");
    let count = Directory::peek_entry_count(&header).expect("entry count");
    let dir_bytes = disc
        .read_entry_range(&archive, 0, Directory::directory_len(count))
        .expect("directory");
    let dir = Directory::parse(&dir_bytes, Some(archive.size)).expect("parse directory");

    let hash = wad::hash_name(weapons::ai::ENTRY);
    let entry = dir
        .entries
        .iter()
        .find(|e| e.name_hash == hash)
        .unwrap_or_else(|| panic!("{} is not in {PSP_ARCHIVE}", weapons::ai::ENTRY));
    let raw = disc
        .read_entry_range(&archive, u64::from(entry.offset), u64::from(entry.size))
        .expect("blob");
    Some(match entry.compression {
        Compression::None => raw,
        Compression::Lzss => {
            oag_formats::lzss::decompress(&raw, entry.size_uncompressed as usize).expect("lzss")
        }
        Compression::Zlib => panic!("unexpected zlib entry"),
    })
}

/// Every weapon `WeaponAi_Update`'s own dispatch switch reaches - the thirteen
/// on `docs/ghidra/functions/psp-pulse-usa/weapon-ai.md`'s direction-flag
/// table. The Disruptor is Pure's, not Pulse's, so it is not expected here.
const ALL_THIRTEEN: [Weapon; 13] = [
    Weapon::Rocket,
    Weapon::Missile,
    Weapon::Quake,
    Weapon::Turbo,
    Weapon::Shield,
    Weapon::Cannon,
    Weapon::Autopilot,
    Weapon::Plasma,
    Weapon::Bomb,
    Weapon::Mine,
    Weapon::LeachBeam,
    Weapon::Repulser,
    Weapon::Shuriken,
];

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_weapon_is_authored_and_absorb_is_uniform() {
    let Some(blob) = blob() else { return };
    let stats = weapons::ai::from_blob(&blob).expect("parses");
    let mut absorbs = Vec::new();
    for weapon in ALL_THIRTEEN {
        let odds = stats
            .get(weapon)
            .unwrap_or_else(|| panic!("{weapon:?} is not authored"));
        assert!(
            odds.use_against_player >= 0.0 && odds.use_against_ai >= 0.0 && odds.absorb >= 0.0,
            "{weapon:?} reads {odds:?}"
        );
        absorbs.push(odds.absorb);
    }
    // Not pinned to a literal - ADR-0006 - but a claim about shape:
    // `weapon-ai.md` records every one of the thirteen agreeing.
    assert!(
        absorbs.windows(2).all(|w| w[0] == w[1]),
        "absorb is not uniform across the thirteen weapons: {absorbs:?}"
    );
    println!("absorb (uniform): {}", absorbs[0]);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn exactly_plasma_and_quake_share_a_pair_no_other_weapon_has() {
    let Some(blob) = blob() else { return };
    let stats = weapons::ai::from_blob(&blob).expect("parses");

    let plasma = stats.get(Weapon::Plasma).expect("Plasma is authored");
    let quake = stats.get(Weapon::Quake).expect("Quake is authored");
    assert_eq!(
        (plasma.use_against_player, plasma.use_against_ai),
        (quake.use_against_player, quake.use_against_ai),
        "Plasma and Quake are documented as sharing one pair, and read {plasma:?} / {quake:?}"
    );

    let common: Vec<_> = ALL_THIRTEEN
        .into_iter()
        .filter(|w| !matches!(w, Weapon::Plasma | Weapon::Quake))
        .map(|w| {
            let odds = stats.get(w).expect("authored");
            (odds.use_against_player, odds.use_against_ai)
        })
        .collect();
    assert!(
        common.windows(2).all(|w| w[0] == w[1]),
        "the other eleven weapons do not all share one pair: {common:?}"
    );
    assert_ne!(
        common[0],
        (plasma.use_against_player, plasma.use_against_ai),
        "Plasma/Quake's pair should differ from the other eleven's shared pair"
    );
    println!(
        "eleven weapons share {:?}; Plasma/Quake share {:?}",
        common[0],
        (plasma.use_against_player, plasma.use_against_ai)
    );
}
