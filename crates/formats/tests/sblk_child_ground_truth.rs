//! The child-sound grain, against every bank on every disc this project reads.
//!
//! **`#[ignore]`d, never run in CI**: needs game content; see
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-formats --test sblk_child_ground_truth --run-ignored all
//! ```
//!
//! # What this is for
//!
//! `docs/formats/psp-audio.md` recorded HD's `.COLLISIONS` as binding no
//! waveform because "all four of its commands are among the 43 unread opcodes",
//! a dead end: two of the four are `0x08`, which plays **another cue**, and the
//! record holds the child's name in ASCII.
//!
//! That `.COLLISIONS` reads legibly proves little (a wrong offset can do that
//! once). The evidence is that across the corpus the field is **either** an
//! in-range cue index **or** `0xffffffff` beside a name the bank's own name
//! table holds, almost never anything else; a wrong stride manufactures neither.

use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::sblk::{self, Bank};
use oag_formats::wad;

/// The PSP, PS2 and Pure discs, with the archives that hold banks on each.
const DISCS: [(&str, &[&str]); 5] = [
    (
        "pulse-psp-usa.chd",
        &["PSP_GAME/USRDIR/FE.wad", "PSP_GAME/USRDIR/Data.wad"],
    ),
    (
        "pulse-psp-eu.chd",
        &["PSP_GAME/USRDIR/FE.wad", "PSP_GAME/USRDIR/Data.wad"],
    ),
    ("pulse-ps2-eu.chd", &["54748/WADS2.WAD"]),
    (
        "pure-psp-usa.chd",
        &[
            "PSP_GAME/USRDIR/Data.wad",
            "PSP_GAME/USRDIR/FE.wad",
            "PSP_GAME/USRDIR/FEData.wad",
        ],
    ),
    (
        "pure-psp-eu.chd",
        &[
            "PSP_GAME/USRDIR/Data.wad",
            "PSP_GAME/USRDIR/FE.wad",
            "PSP_GAME/USRDIR/FEData.wad",
        ],
    ),
];

/// The Wipeout HD disc, and its seven PSARC archives.
const HD_IMAGE: &str = "hdfury-ps3-eu-dec.iso";
const HD_ARCHIVES: usize = 7;

/// Child grains across the whole corpus. Fewer means the walk stopped early.
const CHILD_GRAINS: usize = 1461;

/// Of those, the ones carrying an in-range cue index.
const BY_INDEX: usize = 1153;

/// Of those, the ones carrying a name their own bank holds.
const BY_NAME: usize = 300;

/// Grains that resolve to no child at all: two faults worth keeping apart,
/// [`NEITHER_FIELD`] plus [`OUT_OF_RANGE_INDEX`].
const MALFORMED: usize = NEITHER_FIELD + OUT_OF_RANGE_INDEX;

/// Grains storing `0xffffffff` with no name to fall back on: six, all in
/// `speech_results.bnk`. The record says "resolve me by name" and carries none.
const NEITHER_FIELD: usize = 6;

/// Grains storing an index past the end of their own cue table: one, in
/// `weapons_det.bnk`, which holds 65 in a 55-cue bank.
///
/// **Not the same fault as [`NEITHER_FIELD`]**: this record does store an
/// index, so a "neither field is set" check misses it; the raw byte check
/// separates them. It is what SCREAM's `snd_SFX_GRAIN_TYPE_BRANCH invalid sound
/// index %d` prints.
const OUT_OF_RANGE_INDEX: usize = 1;

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

/// Every `SBlk` blob in one WAD, decompressed.
fn banks_in(disc: &mut DiscImage, archive_path: &str) -> Vec<Vec<u8>> {
    let Some(archive) = disc
        .entries()
        .expect("entries")
        .iter()
        .find(|e| e.path == archive_path)
        .cloned()
    else {
        panic!("{archive_path} present");
    };
    let header = disc
        .read_entry_range(&archive, 0, wad::HEADER_LEN as u64)
        .expect("header");
    let count = wad::Directory::peek_entry_count(&header).expect("entry count");
    let dir_bytes = disc
        .read_entry_range(&archive, 0, wad::Directory::directory_len(count))
        .expect("directory");
    let dir = wad::Directory::parse(&dir_bytes, Some(archive.size)).expect("parse directory");

    let mut out = Vec::new();
    for entry in &dir.entries {
        if entry.size == 0 {
            continue;
        }
        let raw = disc
            .read_entry_range(&archive, u64::from(entry.offset), u64::from(entry.size))
            .expect("blob");
        let blob = match entry.compression {
            wad::Compression::None => raw,
            wad::Compression::Lzss => {
                oag_formats::lzss::decompress(&raw, entry.size_uncompressed as usize).expect("lzss")
            }
            wad::Compression::Zlib => continue,
        };
        if sblk::looks_like_bank(&blob) {
            out.push(blob);
        }
    }
    out
}

/// Every bank on every disc present, tagged with where it came from.
fn every_bank() -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for (name, archives) in DISCS {
        let Some(path) = image(name) else { continue };
        let mut disc = DiscImage::open(&path).expect("open");
        for archive in archives {
            for blob in banks_in(&mut disc, archive) {
                out.push((format!("{name}:{archive}"), blob));
            }
        }
    }
    if let Some(iso) = image(HD_IMAGE) {
        for n in 0..HD_ARCHIVES {
            let spec = format!("{}:PS3_GAME/USRDIR/DATA0{n}.PSARC", iso.display());
            let Ok(mut archive) = oag_assets::psarc::Archive::open(&spec) else {
                continue;
            };
            let paths: Vec<String> = archive
                .paths()
                .iter()
                .filter(|p| p.ends_with(".bnk"))
                .cloned()
                .collect();
            for path in paths {
                if let Ok(blob) = archive.read_path(&path) {
                    out.push((format!("hd:DATA0{n}:{path}"), blob));
                }
            }
        }
    }
    out
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_child_grain_is_an_index_or_a_name_and_almost_never_neither() {
    let banks = every_bank();
    if banks.is_empty() {
        return;
    }

    let mut grains = 0;
    let mut by_index = 0;
    let mut by_name = 0;
    let mut foreign = 0;
    let mut malformed = Vec::new();
    let mut indexed_and_named = 0;
    let mut raw_both = 0;
    let mut raw_neither = 0;
    let mut raw_out_of_range = 0;

    for (label, blob) in &banks {
        let bank = Bank::parse(blob).unwrap_or_else(|e| panic!("{label}: {e}"));
        let known: Vec<String> = bank.sound_names().into_iter().map(|n| n.name).collect();
        for cue in bank.cues().iter().filter(|c| c.plays()) {
            for child in bank.cue_children(cue) {
                grains += 1;

                // **Straight off the bytes, not off `Child`**: classifying by the
                // struct under test would make the exclusivity a property of the
                // parser. These two reads go to the record itself.
                let at = child.record as usize;
                let record = &bank.block[at..at + 32];
                let stored = bank.order.u32(record, 0x0c);
                let has_index = stored != u32::MAX;
                let has_name = record[0x10] != 0;
                match (has_index, has_name) {
                    (true, true) => raw_both += 1,
                    (false, false) => raw_neither += 1,
                    _ => {}
                }
                if has_index && u64::from(stored) >= u64::from(bank.cue_count) {
                    raw_out_of_range += 1;
                }

                match (child.cue, child.name.is_empty()) {
                    (Some(_), true) => by_index += 1,
                    (None, false) if known.contains(&child.name) => by_name += 1,
                    // A name resolved against every loaded bank, not this one.
                    (None, false) => foreign += 1,
                    (Some(_), false) => indexed_and_named += 1,
                    (None, true) => malformed.push(label.clone()),
                }
            }
        }
    }

    println!("child grains   {grains}");
    println!("by index       {by_index}");
    println!("by name        {by_name}");
    println!("cross-bank     {foreign}");
    println!("malformed      {}", malformed.len());

    assert_eq!(grains, CHILD_GRAINS);
    assert_eq!(by_index, BY_INDEX);
    assert_eq!(by_name, BY_NAME);
    // The one cross-bank grain: `env0_det.bnk` asks for `".COLLISIONS"`, which
    // `shiphd.bnk` holds.
    assert_eq!(foreign, 1);
    assert_eq!(malformed.len(), MALFORMED);
    // **The two forms are exclusive**: the assertion that the record is read at
    // the right offsets rather than plausibly (an index and a name in one record
    // would mean the fields are not what they are taken to be).
    //
    // Raw bytes first, since that form can fail. `Child`'s fields are checked
    // second, so a change to `cue_children` that derived one field from the
    // other shows as a disagreement between the counts.
    println!("raw both       {raw_both}");
    println!("raw neither    {raw_neither}");
    println!("raw bad index  {raw_out_of_range}");
    assert_eq!(raw_both, 0, "a record stored an index and a name");
    assert_eq!(indexed_and_named, 0, "a record carried both forms");

    // The two faults kept apart: `weapons_det.bnk`'s grain *does* store an index
    // (past the end of the bank), so "neither field is set" counts six, not
    // seven; the raw read exposed that.
    assert_eq!(raw_neither, NEITHER_FIELD);
    assert_eq!(raw_out_of_range, OUT_OF_RANGE_INDEX);
    assert_eq!(
        raw_neither + raw_out_of_range,
        malformed.len(),
        "the two raw faults account for every unresolvable grain"
    );

    // The split by platform, a fact about two library generations, not this
    // reader: every PSP and PS2 grain is indexed, every named one is on HD.
    assert!(
        by_index > by_name,
        "the corpus is mostly the older, indexed form"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn wipeout_hd_collisions_reaches_the_ship_and_wall_trees() {
    let Some(iso) = image(HD_IMAGE) else {
        return;
    };
    let spec = format!("{}:PS3_GAME/USRDIR/DATA01.PSARC", iso.display());
    let mut archive = oag_assets::psarc::Archive::open(&spec).expect("DATA01");
    let blob = archive.read_path("/data/sound/shiphd.bnk").expect("shiphd");
    let bank = Bank::parse(&blob).expect("parse");

    let root = bank.cue_named(".COLLISIONS").expect(".COLLISIONS");
    // Four grains, none a key-on: `cue_sounds` is empty and the cue is not broken.
    assert_eq!(root.commands, 4);
    assert!(bank.cue_sounds(&root).is_empty());

    let children = bank.cue_children(&root);
    let named: Vec<&str> = children.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(named, ["c_CShipShip", "c_CShipWall"]);
    assert!(
        children.iter().all(oag_formats::sblk::Child::is_named),
        "Wipeout HD names its children rather than indexing them"
    );

    // Each splits three ways by a suffix not wired to anything; see
    // `oag_sound::sfx`.
    for child in &children {
        let cue = bank.resolve_child(child).expect("a child of .COLLISIONS");
        let leaves: Vec<String> = bank
            .cue_children(&cue)
            .iter()
            .map(|c| c.name.clone())
            .collect();
        assert!(
            leaves.iter().all(|n| n.starts_with(&child.name)),
            "{:?} does not name {child:?}'s own leaves",
            leaves
        );
        assert!(leaves.len() >= 3, "{leaves:?}");
    }

    // The tree reaches real audio.
    let sounds = bank.cue_tree_sounds(&root);
    assert_eq!(sounds.len(), 112);
    assert!(
        sounds.iter().all(oag_formats::sblk::Sound::is_adpcm),
        "a collision leaf is in Wipeout HD's unidentified second codec"
    );
    assert!(
        sounds.iter().all(|s| bank.waveform(s).is_some()),
        "a collision leaf reaches outside the waveform section"
    );
}
