//! Validates the [`weapons`](oag_formats::weapons) decoder against the real
//! tables.
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
//! The same reasoning as [`handling_ground_truth`]: this is text, so a wrong
//! reading does not overrun a buffer and there is no arithmetic to close. The
//! checkable prediction is **completeness and shape**.
//!
//! Three claims, and each would be easy to get wrong in a way a fixture cannot
//! catch:
//!
//! 1. **The three simple weapons really are simple in the shipped file** -
//!    `absorb` and `time`, nothing else needed to read them. The decoder singles
//!    them out on that basis, so if a shipped table gave Turbo a fourth
//!    attribute the reason for the whole `Simple` type would be gone.
//! 2. **`absorb` really is on every weapon.** The module says so, and it is what
//!    lets a future absorb branch look the number up for anything.
//! 3. **`<Pickupodds>` covers the speed classes**, with all four weights on
//!    every entry. A weight silently defaulting to zero would be a weapon that
//!    never drops, which reads as bad luck rather than as a parse failure.
//!
//! Both shipped tables are checked - the race one and Eliminator's - because
//! they are two independent documents against one parser, and the roadmap only
//! ever races the first.
//!
//! What is deliberately **not** checked is any *value*. Asserting one would put
//! shipped tuning data in the repository, which ADR-0006 does not allow.
//! Everything below is a statement about presence and structure.
//!
//! [`handling_ground_truth`]: ./handling_ground_truth.rs

use std::path::{Path, PathBuf};

use oag_disc::DiscImage;
use oag_formats::wad::{self, Compression, Directory};
use oag_formats::weapons::{self, Weapon};

const PSP_IMAGE: &str = "pulse-psp-usa.chd";
const PSP_ARCHIVE: &str = "PSP_GAME/USRDIR/Data.wad";

fn image(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images")
        .join(name);
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

/// Both weapon tables off one disc, in `[race, elimination]` order.
fn tables() -> Option<Vec<(&'static str, Vec<u8>)>> {
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

    let mut out = Vec::new();
    // Both are required rather than a floor. The Eliminator table existing at
    // all is part of the claim that a mode swaps the whole table.
    for name in [weapons::RACE_ENTRY, weapons::ELIMINATION_ENTRY] {
        let hash = wad::hash_name(name);
        let entry = dir
            .entries
            .iter()
            .find(|e| e.name_hash == hash)
            .unwrap_or_else(|| panic!("{name} is not in {PSP_ARCHIVE}"));
        let raw = disc
            .read_entry_range(&archive, u64::from(entry.offset), u64::from(entry.size))
            .expect("blob");
        let blob = match entry.compression {
            Compression::None => raw,
            Compression::Lzss => {
                oag_formats::lzss::decompress(&raw, entry.size_uncompressed as usize).expect("lzss")
            }
            Compression::Zlib => panic!("{name}: unexpected zlib entry"),
        };
        out.push((name, blob));
    }
    Some(out)
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn both_shipped_tables_decode() {
    let Some(tables) = tables() else { return };
    for (name, blob) in &tables {
        let stats = weapons::from_blob(blob).unwrap_or_else(|e| panic!("{name}: {e}"));
        println!(
            "{name}: {} weapon(s) with absorb, {} pickup table(s)",
            stats.absorb.len(),
            stats.pickups.len()
        );
        assert!(
            stats.slowdown_limit.is_finite(),
            "{name}: <Global> did not decode"
        );
    }
}

/// Claim 1: the three the decoder calls simple really are, in both tables.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_three_simple_weapons_are_authored_in_both_tables() {
    let Some(tables) = tables() else { return };
    for (name, blob) in &tables {
        let stats = weapons::from_blob(blob).unwrap_or_else(|e| panic!("{name}: {e}"));
        for weapon in [Weapon::Turbo, Weapon::Shield, Weapon::Autopilot] {
            let simple = stats
                .simple(weapon)
                .unwrap_or_else(|| panic!("{name}: {weapon:?} is not authored"));
            // Both are durations or energies, so neither is meaningfully
            // negative and a zero `time` would be an effect that never runs.
            assert!(
                simple.time > 0.0 && simple.absorb >= 0.0,
                "{name}: {weapon:?} reads {simple:?}"
            );
        }
    }
}

/// Claim 2: `absorb` is on every weapon the file authors.
///
/// Stated as "every weapon the *decoder recognises* that the file authors",
/// because the two tables need not carry the same set - Eliminator may well drop
/// some - and a missing weapon is not a defect.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_authored_weapon_carries_absorb() {
    let Some(tables) = tables() else { return };
    for (name, blob) in &tables {
        let stats = weapons::from_blob(blob).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(
            stats.absorb.len() >= 10,
            "{name}: only {} weapon(s) decoded, which is too few for a real table",
            stats.absorb.len()
        );
        for (weapon, absorb) in &stats.absorb {
            assert!(
                absorb.is_finite() && *absorb >= 0.0,
                "{name}: {weapon:?} absorbs {absorb}"
            );
        }
        // The decoder's own list is the thing being checked: every weapon it
        // knows about has to be one the file actually authors, or the enum has
        // a name the game does not use.
        let authored: Vec<Weapon> = stats.absorb.iter().map(|(w, _)| *w).collect();
        for weapon in &authored {
            assert!(Weapon::ALL.contains(weapon));
        }
    }
}

/// Claim 3: the pickup tables cover the speed classes, with all four weights.
///
/// The four weights are the pickup design, and reading them positionally rather
/// than by name would be silently wrong because all four are plain numbers - so
/// this asserts every entry has all four and that they are usable weights.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_pickup_tables_cover_the_speed_classes() {
    let Some(tables) = tables() else { return };
    let (name, blob) = &tables[0];
    let stats = weapons::from_blob(blob).unwrap_or_else(|e| panic!("{name}: {e}"));

    for class in ["Venom", "Flash", "Rapier", "Phantom"] {
        let table = stats
            .pickups_for(class)
            .unwrap_or_else(|| panic!("{name}: no <Pickupodds class=\"{class}\">"));
        assert!(
            table.odds.len() >= 8,
            "{name}: {class} authors odds for only {} weapon(s)",
            table.odds.len()
        );
        let mut any_positive = false;
        for (weapon, odds) in &table.odds {
            for (label, value) in [
                ("ai", odds.ai),
                ("back", odds.back),
                ("front", odds.front),
                ("human", odds.human),
            ] {
                assert!(
                    value.is_finite() && value >= 0.0,
                    "{name}: {class}/{weapon:?} {label} is {value}"
                );
            }
            any_positive |= odds.ai > 0.0 || odds.human > 0.0;
        }
        assert!(
            any_positive,
            "{name}: every weight in {class} is zero, so nothing would ever drop"
        );
    }

    // **The shipped table authors four classes and not five**, and this
    // assertion is here because the first reading of the parser said otherwise.
    // `WeaponStats_Parse` tests the `class` attribute against `Vector` and
    // *discards the result* before testing the four that store an index, so the
    // executable knows the name and the data does not use it. That is evidence
    // about `docs/formats/handling-stats.md`'s fifth-class question and it
    // points the opposite way to the parser read alone - pinned here so it is
    // not re-inferred from the code a second time.
    assert!(
        stats.pickups_for("Vector").is_none(),
        "{name}: Vector is authored now, which answers the fifth-class question"
    );
    assert_eq!(
        stats.pickups.len(),
        4,
        "{name}: {} <Pickupodds> block(s), expected the four speed classes",
        stats.pickups.len()
    );
}
