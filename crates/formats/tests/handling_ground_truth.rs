//! Validates the [`handling`](oag_formats::handling) decoder against real ships.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! The test skips with a printed message when the disc image is absent. Set
//! `OAG_REQUIRE_GAME_DATA=1` to turn absence into a failure, which is what a
//! release check wants: a skipped ground-truth test is green and proves nothing.
//!
//! # What this is for
//!
//! `handlingstats.xml` has no arithmetic to close the way a binary format does:
//! it is text, and a wrong reading of text does not overrun a buffer. The
//! checkable prediction for this format is **completeness** instead.
//!
//! Eight teams, four speed classes each, and every attribute the schema lists
//! present in every block. That is 32 parameter sets and 174 attributes per file,
//! and the decoder treats an absent one as an error rather than a zero
//! specifically so this test can be the thing that proves none is absent. A
//! `mass` that quietly arrived as `0.0` would not crash; it would produce a ship
//! that behaves oddly, which reads as a tuning problem and gets looked for in the
//! force law rather than in the loader.
//!
//! Both shipped releases are checked, because
//! [the rubric](../../../docs/reverse-engineering/confidence-rubric.md) prices a
//! second binary above a second reading of the first: PSP stores this file as
//! shortened XML and PS2 as plain text, and the two agreeing on the element and
//! attribute names is much stronger evidence for the schema than either alone.
//!
//! What is deliberately **not** checked is whether the two ship the same
//! *values*. Answering that would mean putting a comparison or a fingerprint of
//! shipped design data in the repository, which
//! `docs/architecture/adr/0006-no-copyrighted-content.md` does not allow. It
//! stays an open question on `docs/formats/handling-stats.md`.

use std::path::{Path, PathBuf};

use oag_disc::DiscImage;
use oag_formats::handling::{self, SpeedClass};
use oag_formats::wad::{self, Compression, Directory};

/// Where each release keeps the archive holding the ship data.
///
/// The PSP name comes from the ISO 9660 tree; the PS2 one is inside a numbered
/// directory that is part of that disc's layout, not something derived.
const PSP_ARCHIVE: &str = "PSP_GAME/USRDIR/Data.wad";
/// See [`PSP_ARCHIVE`].
const PS2_ARCHIVE: &str = "54748/WADS2.WAD";

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

/// Reads every team's `handlingstats.xml` out of one archive on one disc.
///
/// Returns the blobs so each platform's own test can assert how its release
/// *stores* them, which is the one thing the two do not agree on.
fn ship_blobs(image_name: &str, archive_path: &str) -> Option<Vec<(String, Vec<u8>)>> {
    let path = image(image_name)?;
    let mut disc = DiscImage::open(&path).expect("open");
    let archive = disc
        .entries()
        .expect("entries")
        .iter()
        .find(|e| e.path == archive_path)
        .unwrap_or_else(|| panic!("{archive_path} present"))
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
    for team in handling::TEAMS {
        let name = handling::entry_name(team);
        let hash = wad::hash_name(&name);
        // All eight are required, not a floor: the interesting half of the claim
        // is that no team is missing, and a "checked at least N" test throws that
        // half away. A team spelled differently on one release is a finding, so
        // the panic names it rather than skipping it.
        let entry = dir
            .entries
            .iter()
            .find(|e| e.name_hash == hash)
            .unwrap_or_else(|| panic!("{image_name}: {name} is not in {archive_path}"));

        let raw = disc
            .read_entry_range(&archive, u64::from(entry.offset), u64::from(entry.size))
            .expect("blob");
        let blob = match entry.compression {
            Compression::None => raw,
            Compression::Lzss => {
                oag_formats::lzss::decompress(&raw, entry.size_uncompressed as usize).expect("lzss")
            }
            // The flag exists in the game but no shipped archive uses it, so
            // there is nothing to decode against; a hit here would be a finding.
            Compression::Zlib => panic!("{name}: unexpected zlib entry"),
        };
        out.push((name, blob));
    }

    Some(out)
}

/// The completeness check, run against one release's blobs.
///
/// Every documented attribute is required by the decoder, so a successful parse
/// *is* the completeness assertion: nothing here defaults, so nothing can be
/// quietly absent. What is left to check is that the four classes are exactly the
/// four classes, and that each parameter set is one a force law could use.
fn check_every_team(blobs: &[(String, Vec<u8>)]) {
    for (name, blob) in blobs {
        let stats = handling::from_blob(blob).unwrap_or_else(|e| panic!("{name}: {e}"));

        assert!(!stats.team.is_empty(), "{name}: <Stats> has no team");

        // The type guarantees four slots; this pins each slot to its own class,
        // so a mapping that read one block into two places cannot pass.
        for (index, class) in SpeedClass::ALL.into_iter().enumerate() {
            assert_eq!(
                stats.classes[index].name, class,
                "{name}: slot {index} holds {}",
                stats.classes[index].name
            );
            assert_eq!(stats.class(class).name, class, "{name}: {class} lookup");
        }

        // Four *distinct* blocks. Reading the same one four times would satisfy
        // every check above and nothing here.
        for (i, a) in stats.classes.iter().enumerate() {
            for b in &stats.classes[i + 1..] {
                assert_ne!(
                    (a.engine, a.turning, a.physical),
                    (b.engine, b.turning, b.physical),
                    "{name}: {} and {} are identical parameter sets",
                    a.name,
                    b.name
                );
            }
        }

        // Physical sanity, not tuning: the properties a parameter set has to have
        // for the force law to be defined at all. A zero mass divides by zero; a
        // zero hull extent degenerates contact generation. None of these names a
        // value, which is what keeps the test inside ADR-0006.
        assert!(stats.misc.height > 0.0, "{name}: hull height");
        assert!(stats.misc.length > 0.0, "{name}: hull length");
        assert!(stats.misc.width > 0.0, "{name}: hull width");
        assert!(stats.misc.shield > 0.0, "{name}: shield pool");
        assert!(stats.misc.easyshield > 0.0, "{name}: easy shield pool");

        for class in SpeedClass::ALL {
            let block = stats.class(class);
            assert!(block.physical.mass > 0.0, "{name} {class}: mass");
            assert!(
                block.antigrav.ride_height > 0.0,
                "{name} {class}: hover probe length"
            );
            assert!(
                (0.0..=100.0).contains(&block.airbrake.slidegrip),
                "{name} {class}: slidegrip is documented as a percentage"
            );
        }
    }

    assert_eq!(
        blobs.len(),
        handling::TEAMS.len(),
        "every team must have been read"
    );
    println!("{} teams, {} parameter sets", blobs.len(), blobs.len() * 4);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_psp_team_carries_a_complete_set_of_four_speed_classes() {
    let Some(blobs) = ship_blobs("pulse-psp-usa.chd", PSP_ARCHIVE) else {
        return;
    };

    // PSP stores these shortened, with a per-file `<code>` dictionary. A file
    // beginning `<?xml` here would mean the name hash found a different kind of
    // document than expected.
    for (name, blob) in &blobs {
        assert!(
            oag_formats::fexml::is_fexml(blob),
            "{name} is not shortened front-end XML on PSP"
        );
    }

    check_every_team(&blobs);
}

/// The same claim against the PS2 release, which is what makes it a schema rather
/// than one binary's habit.
///
/// PS2 stores the file as plain text, so this also exercises the other half of
/// [`handling::from_blob`]'s dispatch. Worth knowing before grepping an expanded
/// file: PS2 capitalises `<Pitch>` where PSP writes `<pitch>`, which costs nothing
/// because element lookup is case-insensitive, and would cost an afternoon if it
/// were not.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_ps2_team_carries_the_same_complete_set() {
    let Some(blobs) = ship_blobs("pulse-ps2-eu.chd", PS2_ARCHIVE) else {
        return;
    };

    for (name, blob) in &blobs {
        assert!(
            !oag_formats::fexml::is_fexml(blob),
            "{name} is shortened on PS2, which PSP's storage form was thought to be alone in"
        );
        // Leading whitespace is real: the PS2 files begin with a space, and the
        // declaration after it is malformed (`encoding=utf-81"?>`, unquoted and
        // with a stray digit). Neither matters, because a declaration is a
        // processing instruction and `fexml::parse` skips those, but a stricter
        // XML reader would reject all eight files outright.
        assert!(
            blob.trim_ascii_start().starts_with(b"<?xml"),
            "{name} on PS2 does not begin as plain XML"
        );
    }

    check_every_team(&blobs);
}
