//! Validates the [`pvs`](oag_vex::pvs) decoder against every `section` node
//! on the Pulse PSP and PS2 discs.
//!
//! **`#[ignore]`d, needs a disc image** (`just test-data`; ADR-0006). Skips when it
//! is absent; `OAG_REQUIRE_GAME_DATA=1` makes absence a failure.
//!
//! # What this is for
//!
//! `docs/formats/track.md` records the `section` layout at confidence 90 for the
//! mask and 85 for the bounding box, reasoning from the PSP load path and the
//! 64-section cap hit exactly on `01_Track`: an argument about one binary and one
//! file. These tests make it one about every track file on two discs.
//!
//! # The mask offset, established by measurement rather than assumed
//!
//! The obvious check, *every set bit names a section this file declares*, **is
//! false on shipped data, and finding that out is the point**: around one bit in
//! fifty names a section the file lacks, on both platforms (35 of 40 PSP tracks
//! declare fewer than 64 sections, so a mask outliving a deleted section is
//! authoring slop; the original only tests bits for sections it iterates).
//!
//! So [`stray_fraction`] measures the rate and
//! [`the_documented_offset_beats_its_neighbours`] discriminates: reading the low
//! mask word at the documented `+0x08` must give **fewer** stray bits than four
//! bytes either side. It does, on both platforms: 2.2 % against 7.7 % and 13.3 %
//! on PSP, 4.0 % against 8.7 % and 22.9 % on PS2 (a wrong offset reads the
//! never-initialised pad at `+0x02..0x08` or the bounding box's first float).
//! Self-contained: three readings ranked, no threshold to tune.
//!
//! The other checks a misreading cannot pass:
//!
//! - **Spline control points name sections in the same id namespace**, an
//!   independent route through `WO Track`. Exact on PSP (all 34,261 points name a
//!   declared section), nearly exact on PS2 (a handful do not, the same slop);
//!   asserted exactly where exact, bounded where not.
//! - **The payload closes**: fixed block, bounding box, a name padded to the
//!   16-byte node alignment, every byte after the terminator zero.
//! - **No index reaches the 64 cap.**
//!
//! # Four things this found that the docs page did not have
//!
//! - **The payload does not end at the bounding box**: a NUL-terminated name
//!   follows, padded to alignment, so a shipped `section` is 0x40, 0x50 or 0x60
//!   bytes, never the 0x30 the struct accounts for. Nothing reads it at runtime
//!   and [`TrackPvs`] drops it, but a parser assuming 0x30 is wrong on every disc.
//! - **The fourth float of each bbox corner is `w`, not zero padding**, and the
//!   platforms disagree (PSP `0.0` on every corner, PS2 `1.0` on nearly all).
//!   Either way not a coordinate, which a parser reading corners three floats
//!   wide would pick up as the next axis.
//! - **`has_bounds` is set on every section on both discs**; the flag-clear
//!   branch is real in the loader but unexercised, so [`TrackPvs::bounds_of`]
//!   returning `None` is a path no track takes.
//! - **One id is authored three times over** on 2 of 40 PSP and 2 of 59 PS2
//!   tracks with byte-identical masks; a parser treating a repeated index as
//!   corrupt refuses four shipped tracks. [`TrackPvs`] unions them.
//! - **Pure does not share Pulse's class-ID numbering**, so [`vex::CLASS_SECTION`]
//!   finds nothing there ([`pure_does_not_share_pulses_class_numbering`] pins it).
//!
//! Mask symmetry is only *measured*: a potentially-visible set need not be
//! symmetric, so the figure is printed, not asserted.

use std::collections::BTreeMap;
use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::wad::{self, Compression, Directory};
use oag_vex::pvs::{self, MAX_SECTIONS, TrackPvs};
use oag_vex::track;
use oag_vex::vex;

/// Fixed block: index, flag, padding, and the two mask words.
const FIXED_LEN: usize = 0x10;

/// Bounding box: two corners of four floats each.
const BOUNDS_LEN: usize = 0x20;

/// Node payloads are padded to this, so the name block's length varies, not the
/// name itself.
const ALIGN: usize = 0x10;

/// Where the low mask word is documented to sit.
const MASK_LO: usize = 0x08;

/// `section` nodes across the 40 PSP Pulse track files.
///
/// Pinned so a change in the tree walk or class id shows as a moved count.
const PSP_PULSE_SECTIONS: usize = 2272;

/// `section` nodes across the 59 PS2 Pulse track files.
const PS2_PULSE_SECTIONS: usize = 3049;

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

/// One decoded `.vex` file's worth of raw bytes.
struct VexFile {
    label: String,
    bytes: Vec<u8>,
    tree: Vec<vex::Node>,
}

/// Finds an archive by its full path, not by suffix: `ends_with("Data.wad")` also
/// matches `BEData.wad` and `FEData.wad`, the first of which in disc order holds
/// no tracks (reading as "this disc has no sections").
fn archive_path(disc: &mut DiscImage, name: &str, label: &str) -> String {
    disc.entries()
        .expect("entries")
        .iter()
        .map(|e| e.path.clone())
        .find(|p| p.as_str() == name || p.ends_with(&format!("/{name}")))
        .unwrap_or_else(|| panic!("{label}: {name} not on the disc"))
}

/// Every `.vex` file in one archive, decompressed and walked.
fn vex_files(disc: &mut DiscImage, archive_path: &str) -> Vec<VexFile> {
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
    for (index, entry) in dir.entries.iter().enumerate() {
        if entry.size == 0 {
            continue;
        }
        let raw = disc
            .read_entry_range(&archive, u64::from(entry.offset), u64::from(entry.size))
            .expect("blob");
        let bytes = match entry.compression {
            Compression::None => raw,
            Compression::Lzss => {
                oag_formats::lzss::decompress(&raw, entry.size_uncompressed as usize).expect("lzss")
            }
            Compression::Zlib => panic!("{archive_path} entry {index}: unexpected zlib entry"),
        };
        if !vex::has_magic(&bytes) {
            continue;
        }
        let Ok(tree) = vex::nodes(&bytes) else {
            continue;
        };
        out.push(VexFile {
            label: format!("{archive_path} entry {index}"),
            bytes,
            tree,
        });
    }
    out
}

fn sections_of(file: &VexFile) -> Vec<&vex::Node> {
    file.tree
        .iter()
        .filter(|n| n.class_id == vex::CLASS_SECTION)
        .collect()
}

/// Bits that name a section the file does not declare, over bits set, reading
/// the low mask word at `offset`.
///
/// See the module docs: the measurement establishing the field position by being
/// smallest at the documented offset.
fn stray_fraction(files: &[VexFile], offset: usize) -> f64 {
    let mut set = 0u64;
    let mut stray = 0u64;
    for file in files {
        let sections = sections_of(file);
        if sections.is_empty() {
            continue;
        }
        let declared = sections.iter().fold(0u64, |mask, node| {
            mask | (1u64 << file.bytes[node.payload().start])
        });
        for node in sections {
            let at = node.payload().start + offset;
            let word = |i: usize| -> u64 {
                u64::from(u32::from_le_bytes(
                    file.bytes[at + i..at + i + 4]
                        .try_into()
                        .expect("four bytes"),
                ))
            };
            let mask = (word(4) << 32) | word(0);
            set += u64::from(mask.count_ones());
            stray += u64::from((mask & !declared).count_ones());
        }
    }
    if set == 0 {
        0.0
    } else {
        stray as f64 / set as f64
    }
}

#[derive(Default)]
struct Survey {
    /// `.vex` files carrying at least one `section` node.
    files: usize,
    nodes: usize,
    /// Payload lengths seen, by node.
    payload_lens: BTreeMap<usize, usize>,
    /// Declared section counts, by file.
    counts: BTreeMap<usize, usize>,
    /// Highest index seen anywhere.
    max_index: u8,
    /// Files whose highest index is at or above their declared count - the
    /// sparse case `09_Track` reversed was found in.
    sparse_files: usize,
    /// Set bits summed over every mask, for a mean.
    visible_total: u64,
    /// Ordered pairs where A sees B but B does not see A.
    asymmetric_pairs: usize,
    /// The fourth float of each bbox corner, by value.
    corner_w: BTreeMap<String, usize>,
    /// Control points checked against the section ids they name.
    points_checked: usize,
    /// Section nodes beyond the first claiming an already-claimed id.
    repeated_ids: usize,
    /// Control points naming a section their own file does not declare.
    points_undeclared: usize,
}

impl Survey {
    fn report(&self, label: &str) {
        let mean = if self.nodes == 0 {
            0.0
        } else {
            self.visible_total as f64 / self.nodes as f64
        };
        println!("== {label}");
        println!("  {} track files, {} section nodes", self.files, self.nodes);
        println!("  payload lengths, by node: {:?}", self.payload_lens);
        println!("  sections per file, by file: {:?}", self.counts);
        println!(
            "  highest index anywhere: {}; files with sparse ids: {}",
            self.max_index, self.sparse_files
        );
        println!("  mean sections visible from a section: {mean:.1} of {MAX_SECTIONS}");
        println!("  one-way visibility pairs: {}", self.asymmetric_pairs);
        println!(
            "  fourth float of a bbox corner, by value: {:?}",
            self.corner_w
        );
        println!("  spline control points checked: {}", self.points_checked);
        println!(
            "  section nodes repeating an already-claimed id: {}",
            self.repeated_ids
        );
        println!(
            "  control points naming an undeclared section: {}",
            self.points_undeclared
        );
    }
}

fn survey(files: &[VexFile], label: &str) -> Survey {
    let mut into = Survey::default();

    for file in files {
        let sections = sections_of(file);
        if sections.is_empty() {
            continue;
        }
        let where_ = &file.label;
        let bytes = &file.bytes;

        for node in &sections {
            *into.payload_lens.entry(node.data_size).or_default() += 1;
            let payload = node.payload();

            // The fixed block and box are there; what follows is a whole number of
            // alignment units.
            assert!(
                bytes[payload.start + 1] != 0,
                "{where_}: has_bounds is clear, which no shipped section does - \
                 the module docs record that branch as unexercised"
            );
            assert!(
                node.data_size >= FIXED_LEN + BOUNDS_LEN,
                "{where_}: payload is {} bytes, too short for a bounded section",
                node.data_size
            );
            let tail = node.data_size - (FIXED_LEN + BOUNDS_LEN);
            assert!(
                tail > 0 && tail.is_multiple_of(ALIGN),
                "{where_}: {tail} bytes after the bounding box is not a whole \
                 number of {ALIGN}-byte alignment units"
            );

            // The tail is one NUL-terminated name then zeroes; a misplaced field
            // boundary lands non-zero bytes past the terminator.
            let name = &bytes[payload.start + FIXED_LEN + BOUNDS_LEN..payload.end];
            let end = name
                .iter()
                .position(|&b| b == 0)
                .unwrap_or_else(|| panic!("{where_}: the name block has no terminator"));
            assert!(
                name[..end]
                    .iter()
                    .all(|&b| b.is_ascii_graphic() || b == b' '),
                "{where_}: the name block is not printable ASCII"
            );
            assert!(
                name[end..].iter().all(|&b| b == 0),
                "{where_}: the name block has non-zero bytes after its terminator"
            );

            // The fourth float of each corner: not asserted equal to one value (the
            // platforms disagree) but not a coordinate, which the layout rests on.
            for at in [payload.start + 0x1c, payload.start + 0x2c] {
                let w = f32::from_bits(u32::from_le_bytes(
                    bytes[at..at + 4].try_into().expect("four bytes"),
                ));
                *into.corner_w.entry(format!("{w}")).or_default() += 1;
                assert!(
                    w == 0.0 || w == 1.0,
                    "{where_}: the fourth float at {at:#x} is {w}, which is neither \
                     a zero pad nor a homogeneous w - it may be a coordinate"
                );
            }
        }

        // The parser refuses an index at or above the cap, so this covers it.
        let pvs = TrackPvs::from_nodes(bytes, &file.tree)
            .unwrap_or_else(|e| panic!("{where_}: parsing sections: {e}"));

        into.files += 1;
        into.nodes += pvs.len();
        assert!(
            pvs.len() <= sections.len(),
            "{where_}: more ids than nodes, which cannot happen"
        );
        into.repeated_ids += sections.len() - pvs.len();
        *into.counts.entry(pvs.len()).or_default() += 1;

        let ids: Vec<u8> = pvs.ids().collect();
        let highest = *ids.last().expect("at least one section");
        into.max_index = into.max_index.max(highest);
        if usize::from(highest) >= pvs.len() {
            into.sparse_files += 1;
        }

        for &id in &ids {
            let mask = pvs.visible_from(id);
            into.visible_total += u64::from(mask.count_ones());
            assert!(
                mask & (1u64 << id) != 0,
                "{where_}: section {id} cannot see itself"
            );
            for seen in pvs::set_bits(mask) {
                if pvs.declares(seen) && pvs.visible_from(seen) & (1u64 << id) == 0 {
                    into.asymmetric_pairs += 1;
                }
            }

            let bounds = pvs
                .bounds_of(id)
                .unwrap_or_else(|| panic!("{where_}: section {id} has no bounds"));
            for axis in 0..3 {
                assert!(
                    bounds.min[axis] <= bounds.max[axis],
                    "{where_}: section {id} has an inside-out box on axis {axis}"
                );
            }
        }

        // The spline and section nodes share one id namespace, and the adjacency the
        // renderer pads with stays inside it. By the id this file's own version word
        // implies (as `track::find_node` does); this walk wants every node, so it
        // asks for the id itself.
        let wo_track = vex::classes_of(bytes)
            .expect("a version with a class table")
            .wo_track
            .expect("a version whose WO Track id is recovered");
        for node in file.tree.iter().filter(|n| n.class_id == wo_track) {
            let payload = node.payload();
            let Ok(ai) = track::parse(&bytes[payload]) else {
                continue;
            };
            let adjacency = pvs::SectionAdjacency::from_track(&ai, 2);
            for path in &ai.paths {
                for point in &path.points {
                    into.points_checked += 1;
                    if !pvs.declares(point.section_id) {
                        into.points_undeclared += 1;
                    }
                    // Padding must always contain the section it pads, or a
                    // craft would cull away the ground under itself.
                    assert!(
                        adjacency.near(point.section_id) & (1u64 << point.section_id) != 0,
                        "{where_}: section {} is not its own neighbour",
                        point.section_id
                    );
                }
            }
        }
    }

    into.report(label);
    assert!(into.nodes > 0, "{label}: no section nodes found at all");
    into
}

fn open(image_name: &str, archive: &str, label: &str) -> Option<Vec<VexFile>> {
    let path = image(image_name)?;
    let mut disc = DiscImage::open(&path).expect("open");
    let found = archive_path(&mut disc, archive, label);
    Some(vex_files(&mut disc, &found))
}

#[test]
#[ignore = "needs data/images; run with `just test-data`"]
fn psp_pulse_sections_parse_and_close() {
    let Some(files) = open("pulse-psp-usa.chd", "Data.wad", "Pulse PSP") else {
        return;
    };
    let survey = survey(&files, "Pulse PSP");
    assert_eq!(survey.files, 40, "the PSP disc has 40 track files");
    assert_eq!(survey.nodes + survey.repeated_ids, PSP_PULSE_SECTIONS);
    assert_eq!(
        survey.repeated_ids, 4,
        "two PSP tracks each author one id three times over"
    );
    assert_eq!(
        survey.points_undeclared, 0,
        "every PSP control point names a section its own file declares"
    );
    assert!(
        survey.sparse_files > 0,
        "no file has sparse ids, yet 09_Track reversed is documented as having them"
    );
    // PSP writes zero in each corner's fourth slot and PS2 one; if either mixes,
    // the field is not what the page says.
    assert_eq!(
        survey.corner_w.keys().collect::<Vec<_>>(),
        vec!["0"],
        "PSP is documented as writing zero in every corner's fourth slot"
    );
}

#[test]
#[ignore = "needs data/images; run with `just test-data`"]
fn ps2_pulse_sections_parse_and_close() {
    let Some(files) = open("pulse-ps2-eu.chd", "WADS2.WAD", "Pulse PS2") else {
        return;
    };
    let survey = survey(&files, "Pulse PS2");
    assert_eq!(survey.files, 59, "the PS2 disc has 59 track files");
    assert_eq!(survey.nodes + survey.repeated_ids, PS2_PULSE_SECTIONS);
    assert_eq!(
        survey.repeated_ids, 4,
        "two PS2 tracks each author one id three times over"
    );
    // Unlike PSP, a few PS2 control points name an undeclared section. Bounded, and
    // harmless: an undeclared id looks up as ALL_VISIBLE (the fallback earning its
    // keep on real data).
    let rate = survey.points_undeclared as f64 / survey.points_checked as f64;
    assert!(
        rate < 0.01,
        "{} of {} PS2 control points name an undeclared section ({rate:.4}), \
         which is too many to call authoring slop",
        survey.points_undeclared,
        survey.points_checked
    );
}

/// The evidence for the mask's field position, on both platforms.
///
/// See the module docs. The low word at the documented `+0x08` must scatter fewer
/// bits into undeclared sections than four bytes either side (the pad and the
/// bounding box's first float).
#[test]
#[ignore = "needs data/images; run with `just test-data`"]
fn the_documented_offset_beats_its_neighbours() {
    for (image_name, archive, label) in [
        ("pulse-psp-usa.chd", "Data.wad", "Pulse PSP"),
        ("pulse-ps2-eu.chd", "WADS2.WAD", "Pulse PS2"),
    ] {
        let Some(files) = open(image_name, archive, label) else {
            continue;
        };
        let documented = stray_fraction(&files, MASK_LO);
        let early = stray_fraction(&files, MASK_LO - 4);
        let late = stray_fraction(&files, MASK_LO + 4);
        println!(
            "== {label}: stray bits at +{:#04x} {:.2}%, at +{:#04x} {:.2}%, at +{:#04x} {:.2}%",
            MASK_LO - 4,
            early * 100.0,
            MASK_LO,
            documented * 100.0,
            MASK_LO + 4,
            late * 100.0,
        );
        assert!(
            documented < early && documented < late,
            "{label}: the documented offset {MASK_LO:#04x} is not the best reading \
             ({documented:.4} against {early:.4} and {late:.4})"
        );
        assert!(
            documented < 0.10,
            "{label}: even the best reading scatters {documented:.4} of its bits \
             into undeclared sections, which is too many to call authoring slop"
        );
    }
}

/// Pure is the earlier game on the same engine, and its `.vex` class IDs are
/// **not** Pulse's.
///
/// Its track files use classes in `0x36f..0x393` where Pulse uses `0x3b9..0x3e9`,
/// so [`vex::CLASS_SECTION`] matches nothing on that disc. Pinned as an
/// observation: an empty result means the constant does not apply to Pure, not
/// that Pure has no visibility partition (recovering its numbering is separate
/// work).
#[test]
#[ignore = "needs data/images; run with `just test-data`"]
fn pure_does_not_share_pulses_class_numbering() {
    let Some(files) = open("pure-psp-usa.chd", "Data.wad", "Pure PSP") else {
        return;
    };
    let sections: usize = files.iter().map(|f| sections_of(f).len()).sum();
    let pulse_track_nodes: usize = files
        .iter()
        .flat_map(|f| f.tree.iter())
        .filter(|n| n.class_id == vex::CLASS_WO_TRACK)
        .count();
    println!(
        "== Pure PSP: {} vex files, {sections} nodes matching Pulse's section class, \
         {pulse_track_nodes} matching Pulse's WO Track class",
        files.len()
    );
    assert_eq!(
        (sections, pulse_track_nodes),
        (0, 0),
        "Pure now matches Pulse's class numbering, which would be a real finding - \
         see this test's doc comment before changing it"
    );
}
