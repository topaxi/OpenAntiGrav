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
                stats.classes[index].name,
                Some(class),
                "{name}: slot {index} holds {:?}",
                stats.classes[index].name
            );
            assert_eq!(
                stats.class(class).expect("Pulse has all four rungs").name,
                Some(class),
                "{name}: {class} lookup"
            );
        }

        // Four *distinct* blocks. Reading the same one four times would satisfy
        // every check above and nothing here.
        for (i, a) in stats.classes.iter().enumerate() {
            for b in &stats.classes[i + 1..] {
                assert_ne!(
                    (a.engine, a.turning, a.physical),
                    (b.engine, b.turning, b.physical),
                    "{name}: {:?} and {:?} are identical parameter sets",
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
        assert!(
            stats.misc.easyshield.is_some_and(|v| v > 0.0),
            "{name}: easy shield pool"
        );

        for class in SpeedClass::ALL {
            let block = stats.class(class).expect("Pulse has all four rungs");
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

/// What `<Handling>`'s children actually are, on both releases.
///
/// `Handling_ParseStats` (`0x0883a2f0`) walks them looking for two names -
/// `<Global>`, which it hands to `Xml_ReadGlobalSettings`, and `<Stats>`. Whether
/// the shipped files carry the first is a question about the discs rather than
/// about the decoder, and this is where a question like that gets answered.
///
/// Deliberately a **report, not an assertion** on `<Global>`: it names what it
/// found so the answer is in the test output either way. Only `<Stats>` is
/// asserted, because the decoder already requires it.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn which_top_level_elements_handlingstats_carries() {
    for (image, archive) in [
        ("pulse-psp-usa.chd", PSP_ARCHIVE),
        ("pulse-ps2-eu.chd", PS2_ARCHIVE),
    ] {
        let Some(blobs) = ship_blobs(image, archive) else {
            continue;
        };

        for (name, blob) in &blobs {
            let expanded = if oag_formats::fexml::is_fexml(blob) {
                oag_formats::fexml::expand(blob).expect("expands")
            } else {
                String::from_utf8_lossy(blob).into_owned()
            };
            let root = oag_formats::fexml::parse(&expanded);

            fn find<'a>(
                node: &'a oag_formats::fexml::Node,
                want: &str,
            ) -> Option<&'a oag_formats::fexml::Node> {
                if node.name.eq_ignore_ascii_case(want) {
                    return Some(node);
                }
                node.children.iter().find_map(|c| find(c, want))
            }

            let handling = find(&root, "Handling").expect("a <Handling> root");
            let children: Vec<&str> = handling.children.iter().map(|c| c.name.as_str()).collect();
            println!("{image} {name}: <Handling> holds {children:?}");

            assert!(
                children.iter().any(|c| c.eq_ignore_ascii_case("Stats")),
                "{image} {name}: no <Stats>"
            );

            // The finding this test exists to pin: a per-team file carries
            // `<Stats>` and nothing else. `<Global>` goes through the same parser
            // in the original, but it lives in `handling::GLOBAL_ENTRY`
            // (`Data\XML\HandlingStats.xml`), which is a different file passed by
            // a different caller. Sixteen files agree - eight teams on each disc.
            //
            // If this ever fails, `handling::parse` should start looking for
            // `<Global>` again and the module docs are wrong.
            assert!(
                !children.iter().any(|c| c.eq_ignore_ascii_case("Global")),
                "{image} {name}: carries <Global>, which was measured to live only \
                 in {}",
                handling::GLOBAL_ENTRY
            );
            assert_eq!(
                handling::parse_global(&expanded),
                Ok(None),
                "{image} {name}: a per-team file yielded Zone settings"
            );
        }
    }
}

/// Reads one named entry out of one archive on one disc, decompressing if it
/// needs it.
///
/// [`ship_blobs`] does the same walk for the eight per-team files; this exists
/// because the engine-wide `<Global>` file is a different entry in the same
/// archive and duplicating the walk is cheaper than generalising it.
fn global_blob(image_name: &str, archive_path: &str) -> Option<Vec<u8>> {
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

    let hash = wad::hash_name(handling::GLOBAL_ENTRY);
    let entry = dir
        .entries
        .iter()
        .find(|e| e.name_hash == hash)
        .unwrap_or_else(|| {
            panic!(
                "{image_name}: {} is not in {archive_path}",
                handling::GLOBAL_ENTRY
            )
        });

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

/// The engine-wide file must decode on both releases, with a complete set of
/// speed-pad tunables.
///
/// The decoder requires every attribute, so a successful parse *is* the
/// completeness claim - the same argument [`check_every_team`] rests on. What is
/// added here is that the two releases agree on the **schema** for a file only
/// one caller ever opens, which is the check the per-team tests cannot make.
///
/// No value is asserted or printed, per
/// `docs/architecture/adr/0006-no-copyrighted-content.md`. What *is* asserted is
/// that the four blocks are four distinct slots, so a reader that filled one and
/// copied it cannot pass.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_global_file_carries_a_complete_set_of_speed_pad_tunables() {
    let mut checked = 0;
    for (image_name, archive) in [
        ("pulse-psp-usa.chd", PSP_ARCHIVE),
        ("pulse-ps2-eu.chd", PS2_ARCHIVE),
    ] {
        let Some(blob) = global_blob(image_name, archive) else {
            continue;
        };
        let global = handling::global_from_blob(&blob)
            .unwrap_or_else(|e| panic!("{image_name}: {e}"))
            .unwrap_or_else(|| panic!("{image_name}: the global file carries no <Global>"));

        for class in SpeedClass::ALL {
            let pads = global.speedup_pads(class);
            assert!(
                pads.amount > 0.0,
                "{image_name} {class}: a speed pad with no magnitude does nothing"
            );
            assert!(
                pads.time > 0.0,
                "{image_name} {class}: a boost with no duration never applies"
            );
        }
        // Not all four distinct - the discs need not tune every class apart -
        // but not all four *identical* either, or the per-class table is being
        // filled from one block and the `VECTOR` skip could be shifting slots
        // without anything noticing.
        let all_same = SpeedClass::ALL
            .into_iter()
            .all(|c| global.speedup_pads(c) == global.speedup_pads(SpeedClass::Venom));
        assert!(
            !all_same,
            "{image_name}: every class got the same tunables, so the index is suspect"
        );

        // `<GravityMul>` comes out of the same `<GlobalClass>` blocks. A zero
        // would leave a grounded craft weightless, and a negative one would push
        // it off the track, so the sign is worth asserting even though the
        // magnitude stays off the page.
        for class in SpeedClass::ALL {
            assert!(
                global.gravity_mul(class).airborne > 0.0,
                "{image_name} {class}: a non-positive gravity scale is not a ship"
            );
        }
        // The finding this decode exists to correct: two physics pages recorded
        // the table as `1.0` for every class. It is not.
        let gravity_all_same = SpeedClass::ALL
            .into_iter()
            .all(|c| global.gravity_mul(c) == global.gravity_mul(SpeedClass::Venom));
        assert!(
            !gravity_all_same,
            "{image_name}: every class got the same gravity scale, which is what \
             docs/physics/README.md used to claim and this test exists to refute"
        );

        // `<WeaponPad>` is the third element in the same blocks. Unlike the two
        // above, **every class really is authored the same here**, so there is
        // no all-same check to make - see `handling::WeaponPad`, which records
        // that rather than treating it as suspicious.
        for class in SpeedClass::ALL {
            let pad = global.weapon_pads(class);
            assert!(
                pad.refresh_time > 0.0,
                "{image_name} {class}: a zero cooldown lets one crossing grant a \
                 pickup on every tick the hull is inside the volume"
            );
            // A relation rather than a value, per ADR-0006. It is the whole of
            // what having two tables is for: Eliminator re-arms a pad sooner.
            if let Some(elimination) = pad.elimination_refresh_time {
                assert!(
                    elimination > 0.0 && elimination < pad.refresh_time,
                    "{image_name} {class}: Eliminator's cooldown should be \
                     positive and shorter than the ordinary one"
                );
            }
        }
        checked += 1;
    }
    println!("{checked} release(s) carry a complete <Global> block");
}

/// The finding `oag_formats::handling::global_classes` is shaped around: the
/// shipped file authors a **fifth** `<GlobalClass>`, named `VECTOR`, and it is
/// **first**.
///
/// Both halves matter. That it exists is why an unknown name must not be an
/// error. That it is first is why skipping it is equivalent to what the original
/// does, which leaves its numbers in a stale slot that the four following blocks
/// then overwrite - see `docs/formats/handling-stats.md`. If this ever fails
/// because the order changed, the equivalence argument on that page is void.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_global_file_authors_a_fifth_class_and_authors_it_first() {
    for (image_name, archive) in [
        ("pulse-psp-usa.chd", PSP_ARCHIVE),
        ("pulse-ps2-eu.chd", PS2_ARCHIVE),
    ] {
        let Some(blob) = global_blob(image_name, archive) else {
            continue;
        };
        let expanded = if oag_formats::fexml::is_fexml(&blob) {
            oag_formats::fexml::expand(&blob).expect("expands")
        } else {
            String::from_utf8_lossy(&blob).into_owned()
        };

        let names: Vec<String> = expanded
            .match_indices("<GlobalClass")
            .filter_map(|(at, _)| {
                let rest = &expanded[at..];
                let open = rest.find("name=\"")? + 6;
                let close = rest[open..].find('"')? + open;
                Some(rest[open..close].to_string())
            })
            .collect();

        println!("{image_name}: <GlobalClass> names {names:?}");
        assert_eq!(
            names.len(),
            SpeedClass::ALL.len() + 1,
            "{image_name}: expected the four speed classes plus one more"
        );
        assert!(
            SpeedClass::from_name(&names[0]).is_none(),
            "{image_name}: the first block is a speed class, so the skip is no longer harmless"
        );
        for class in SpeedClass::ALL {
            assert!(
                names.iter().any(|n| n.eq_ignore_ascii_case(class.as_str())),
                "{image_name}: no <GlobalClass> for {class}"
            );
        }
    }
}
