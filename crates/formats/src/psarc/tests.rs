//! A synthetic archive, built to the layout `docs/formats/psarc.md` records.
//!
//! The real corpus is `crates/formats/tests/psarc_ground_truth.rs`, which needs
//! a disc image. These tests let the block walk, width probe and digest fail on
//! a machine that has none.

use super::{
    Directory, ENTRY_LEN, Entry, Error, HEADER_LEN, Header, MAGIC, parse_manifest, path_digest,
};

/// One entry as the builder sees it, before offsets are known.
struct Planned {
    path: &'static str,
    body: Vec<u8>,
    /// Whether to store the blocks raw rather than deflate them.
    stored: bool,
}

/// Builds a whole archive, manifest included, and returns its bytes.
fn build(block_size: u32, block_width: usize, files: &[Planned]) -> Vec<u8> {
    let manifest: Vec<u8> = files
        .iter()
        .map(|f| f.path)
        .collect::<Vec<_>>()
        .join("\n")
        .into_bytes();

    // The manifest is entry 0 and is always deflated, as it is on the disc.
    let mut payloads: Vec<(Vec<u8>, bool)> = vec![(manifest, false)];
    payloads.extend(files.iter().map(|f| (f.body.clone(), f.stored)));

    let mut blocks: Vec<u32> = Vec::new();
    let mut bodies: Vec<Vec<u8>> = Vec::new();
    let mut plan: Vec<(u32, u64)> = Vec::new(); // first block, uncompressed size

    for (payload, stored) in &payloads {
        plan.push((blocks.len() as u32, payload.len() as u64));
        let mut body = Vec::new();
        for chunk in payload.chunks(block_size as usize) {
            if *stored {
                blocks.push(0);
                let mut padded = chunk.to_vec();
                padded.resize(block_size as usize, 0);
                body.extend_from_slice(&padded);
            } else {
                let deflated = miniz_oxide::deflate::compress_to_vec_zlib(chunk, 6);
                blocks.push(deflated.len() as u32);
                body.extend_from_slice(&deflated);
            }
        }
        bodies.push(body);
    }

    let toc_len = HEADER_LEN + payloads.len() * ENTRY_LEN + blocks.len() * block_width;

    let mut out = Vec::new();
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&3u16.to_be_bytes());
    out.extend_from_slice(b"zlib");
    out.extend_from_slice(&(toc_len as u32).to_be_bytes());
    out.extend_from_slice(&(ENTRY_LEN as u32).to_be_bytes());
    out.extend_from_slice(&(payloads.len() as u32).to_be_bytes());
    out.extend_from_slice(&block_size.to_be_bytes());
    out.extend_from_slice(&3u32.to_be_bytes());

    let mut offset = toc_len as u64;
    for (index, (first_block, size)) in plan.iter().enumerate() {
        let digest = if index == 0 {
            [0u8; 16]
        } else {
            path_digest(files[index - 1].path)
        };
        out.extend_from_slice(&digest);
        out.extend_from_slice(&first_block.to_be_bytes());
        out.extend_from_slice(&size.to_be_bytes()[3..8]);
        out.extend_from_slice(&offset.to_be_bytes()[3..8]);
        offset += bodies[index].len() as u64;
    }
    for block in &blocks {
        out.extend_from_slice(&block.to_be_bytes()[4 - block_width..]);
    }
    for body in &bodies {
        out.extend_from_slice(body);
    }
    out
}

fn planned(path: &'static str, body: Vec<u8>, stored: bool) -> Planned {
    Planned { path, body, stored }
}

/// Reads every entry back the way a caller does: ask for the range, hand back
/// exactly those bytes.
fn read_all(archive: &[u8]) -> (Directory, Vec<Vec<u8>>) {
    let header = Header::parse(archive).expect("header");
    let directory = Directory::parse(&archive[..header.toc_len as usize]).expect("directory");
    let mut out = Vec::new();
    for index in 0..directory.len() {
        let (offset, len) = directory.entry_range(index).expect("range");
        let stored = &archive[offset as usize..(offset + len) as usize];
        out.push(directory.read_entry(index, stored).expect("entry"));
    }
    (directory, out)
}

#[test]
fn a_deflated_archive_round_trips() {
    let long: Vec<u8> = (0..300u32).map(|i| (i % 251) as u8).collect();
    let archive = build(
        64,
        2,
        &[
            planned("/data/one.txt", b"hello psarc".to_vec(), false),
            planned("/data/two.bin", long.clone(), false),
        ],
    );

    let (directory, entries) = read_all(&archive);

    assert_eq!(directory.header.version_major, 1);
    assert_eq!(directory.header.version_minor, 3);
    assert_eq!(&directory.header.compression, b"zlib");
    assert_eq!(directory.block_width, 2);
    assert_eq!(directory.len(), 3, "two files plus the manifest");

    assert_eq!(
        parse_manifest(&entries[0]),
        vec!["/data/one.txt".to_string(), "/data/two.bin".to_string()],
        "entry n + 1 is manifest line n"
    );
    assert_eq!(entries[1], b"hello psarc");
    assert_eq!(entries[2], long, "an entry spanning five blocks");
}

#[test]
fn a_stored_block_is_a_full_block_of_raw_bytes() {
    let body: Vec<u8> = (0..100u32).map(|i| (i % 7) as u8).collect();
    let archive = build(32, 2, &[planned("/raw.bin", body.clone(), true)]);

    let (directory, entries) = read_all(&archive);

    assert!(
        directory.blocks[directory.entries[1].first_block as usize..]
            .iter()
            .all(|&b| b == 0),
        "a stored block declares size zero"
    );
    assert_eq!(entries[1], body, "and the trailing pad is truncated away");
}

/// A stored block whose first byte happens to be `0x78` is **still raw**.
///
/// A short block may be a deflate stream or a payload that did not shrink, and
/// `0x78` tells them apart; a **full-size** block is raw by definition (stored
/// length zero), and `0x78` is an ordinary byte in arbitrary content. It bit on
/// real content: `Data\Music\Exceeder\music_stereo.mp3` is `DATA01.PSARC` entry
/// 16 on `hdfury-ps3-eu-dec.iso`, its block 574 a raw MP3 block beginning `78`,
/// one track of fifteen silently missing from the soundtrack listing.
/// `scripts/psarc.py` never had the bug: it emits a full block without testing.
#[test]
fn a_full_stored_block_beginning_with_the_zlib_marker_is_not_inflated() {
    // Every body byte is `0x78`, so every block opens on the marker: the
    // pathological case, not a lucky one.
    let body = vec![super::ZLIB_CMF; 96];
    let archive = build(32, 2, &[planned("/raw.bin", body.clone(), true)]);

    let (_, entries) = read_all(&archive);
    assert_eq!(
        entries[1], body,
        "a full block is raw whatever it happens to start with"
    );
}

#[test]
fn every_entry_carries_md5_of_its_uppercased_path() {
    let archive = build(
        64,
        2,
        &[planned("/DATA/FE/FONTS/CHINESE.FNT", vec![1], false)],
    );
    let (directory, _) = read_all(&archive);

    assert_eq!(
        directory.entries[0].digest, [0u8; 16],
        "the manifest has no path to hash"
    );
    assert_eq!(
        directory.entries[1].digest,
        path_digest("/data/fe/fonts/chinese.fnt"),
        "the digest is over the uppercase spelling, whatever case the path is stored in"
    );
    assert_ne!(
        directory.entries[1].digest,
        super::md5_digest(b"/data/fe/fonts/chinese.fnt"),
        "the lowercase spelling matches nothing"
    );
}

/// The width is probed, and the probe is only *sound* one way round.
///
/// An odd table length cannot be pairs, so a 3-byte width is recovered. An even
/// one can always be read as pairs, so a 4-byte width is undecidable and the
/// narrowest wins; this pins that. No disc here exercises either branch: all
/// seven archives are 2.
#[test]
fn the_block_width_probe_recovers_an_odd_table_and_prefers_the_narrowest_otherwise() {
    // Five blocks at three bytes each is fifteen: not divisible by two.
    let archive = build(16, 3, &[planned("/w.bin", vec![9; 62], false)]);
    let (directory, entries) = read_all(&archive);
    assert_eq!(directory.blocks.len(), 5, "one manifest block plus four");
    assert_eq!(directory.block_width, 3);
    assert_eq!(entries[1], vec![9u8; 62]);

    let archive = build(16, 2, &[planned("/w.bin", vec![9; 40], false)]);
    let (directory, entries) = read_all(&archive);
    assert_eq!(directory.block_width, 2);
    assert_eq!(entries[1], vec![9u8; 40]);
}

#[test]
fn an_encrypted_image_reads_as_bad_magic_rather_than_as_a_guess() {
    let noise = [0x5au8; HEADER_LEN];
    assert_eq!(
        Header::parse(&noise),
        Err(Error::BadMagic { found: [0x5a; 4] })
    );
}

#[test]
fn a_truncated_header_says_what_it_needed() {
    assert_eq!(
        Header::parse(&MAGIC),
        Err(Error::TooShort {
            need: HEADER_LEN,
            got: 4
        })
    );
}

#[test]
fn an_unknown_entry_stride_is_refused() {
    let mut archive = build(64, 2, &[planned("/a", vec![1], false)]);
    archive[0x13] = 40;
    assert_eq!(
        Header::parse(&archive),
        Err(Error::UnknownEntryLen { entry_len: 40 })
    );
}

#[test]
fn an_entry_naming_a_block_past_the_table_is_refused() {
    let mut directory = read_all(&build(64, 2, &[planned("/a", vec![1], false)])).0;
    directory.entries.push(Entry {
        digest: [0; 16],
        first_block: 99,
        size: 1,
        offset: 0,
    });
    assert_eq!(
        directory.entry_range(2),
        Err(Error::BlockOutOfRange {
            index: 2,
            block: 99,
            blocks: directory.blocks.len(),
        })
    );
}

/// A single implausible `first_block` (real digest and size, a block index no
/// width could cover) does not take the whole directory down.
///
/// Pins the `omega-ps4-eu` fix: `read_block_table` excludes such an entry from
/// its `highest` computation and `Directory::parse` no longer validates every
/// block range, so the bad row fails only its own path.
#[test]
fn an_implausible_first_block_fails_its_own_entry_not_the_whole_directory() {
    let mut archive = build(
        64,
        2,
        &[
            planned("/a.bin", vec![1, 2, 3], false),
            planned("/b.bin", vec![4, 5, 6], false),
        ],
    );

    // Entry 1 ("/a.bin") is HEADER_LEN + 1 * ENTRY_LEN in; its first_block is
    // the four bytes at +0x10 of that row.
    let entry_1_first_block = HEADER_LEN + ENTRY_LEN + 0x10;
    archive[entry_1_first_block..entry_1_first_block + 4].copy_from_slice(&u32::MAX.to_be_bytes());

    let header = Header::parse(&archive).expect("header");
    let directory =
        Directory::parse(&archive[..header.toc_len as usize]).expect("directory still parses");

    assert!(
        directory.entry_range(1).is_err(),
        "the corrupted entry itself is unreadable"
    );
    let (offset, len) = directory
        .entry_range(2)
        .expect("the other entry is unaffected");
    let stored = &archive[offset as usize..(offset + len) as usize];
    assert_eq!(
        directory.read_entry(2, stored).expect("reads fine"),
        vec![4, 5, 6]
    );
}

#[test]
fn a_digest_shared_by_two_entries_matches_the_first_one_in_entry_order() {
    let live_path = "Data/art/ship.gnf";

    let live = Entry {
        digest: path_digest(live_path),
        first_block: 0,
        size: 64,
        offset: 128,
    };
    let stale_duplicate = Entry {
        digest: path_digest(live_path),
        first_block: 0,
        size: 0,
        offset: 0,
    };
    let entries = vec![
        Entry {
            digest: [0u8; 16],
            first_block: 0,
            size: 0,
            offset: 0,
        },
        live,
        stale_duplicate,
    ];

    let matches = super::match_paths_to_entries(&entries, &[live_path.to_string()]);
    assert_eq!(
        matches,
        vec![
            super::PathEntry {
                index: 1,
                path: live_path.to_string(),
            },
            super::PathEntry {
                index: 2,
                path: live_path.to_string(),
            },
        ],
        "both entries resolve to the path - entry order decides which one a \
         first-match lookup like Archive::index_of_path finds"
    );
}

/// `parse_manifest` takes no `Header`, because a version-based dispatch once
/// regressed every Vita-backed path lookup: `omega-ps4-eu`'s 1.4 archives are
/// NUL-delimited but Vita `2048`'s 1.4 `data.psarc` is newline-delimited with
/// no `\x00`. Real CRLF and no NUL is Vita's shape.
#[test]
fn the_manifest_tolerates_crlf_and_blank_lines() {
    assert_eq!(
        parse_manifest(b"/a.txt\r\n/b.txt\r\n\r\n"),
        vec!["/a.txt".to_string(), "/b.txt".to_string()]
    );
}

#[test]
fn a_manifest_with_no_newline_at_all_is_read_as_nul_delimited() {
    assert_eq!(
        parse_manifest(b"Data/a.gnf\x00Data/b.gnf\x00\x00\x00"),
        vec!["Data/a.gnf".to_string(), "Data/b.gnf".to_string()],
        "NUL-delimited, and a run of empty segments is dropped like a blank line"
    );
}

#[test]
fn a_zero_digest_placeholder_row_and_an_orphaned_manifest_path_are_both_dropped() {
    // Entry 1 is live, entry 2 a zeroed placeholder row, and the manifest lists
    // a path with no entry: the shape `omega-ps4-eu`'s archives are full of.
    let live_path = "Data/art/ship.gnf";
    let entries = vec![
        Entry {
            digest: [0u8; 16],
            first_block: 0,
            size: 0,
            offset: 0,
        },
        Entry {
            digest: path_digest(live_path),
            first_block: 0,
            size: 64,
            offset: 128,
        },
        Entry {
            digest: [0u8; 16],
            first_block: 0,
            size: 0,
            offset: 0,
        },
    ];
    let manifest_paths = vec![
        live_path.to_string(),
        "Data/art/orphaned_elsewhere.gnf".to_string(),
    ];

    let matches = super::match_paths_to_entries(&entries, &manifest_paths);
    assert_eq!(
        matches,
        vec![super::PathEntry {
            index: 1,
            path: live_path.to_string(),
        }],
        "the placeholder row and the orphaned manifest path are both dropped, not guessed at"
    );
}

/// The well-behaved case is a corollary of digest matching, not a separate
/// path: every digest matches its manifest line's, so matching by digest
/// reproduces the positional order PS3 and Vita archives hold.
#[test]
fn a_positionally_ordered_manifest_still_resolves_in_order_through_digest_matching() {
    let paths = ["/data/one.txt", "/data/two.bin", "/data/three.xml"];
    let entries: Vec<Entry> = std::iter::once(Entry {
        digest: [0u8; 16],
        first_block: 0,
        size: 0,
        offset: 0,
    })
    .chain(paths.iter().enumerate().map(|(n, p)| Entry {
        digest: path_digest(p),
        first_block: n as u32,
        size: 64,
        offset: 128 + n as u64 * 64,
    }))
    .collect();
    let manifest_paths: Vec<String> = paths.iter().map(|p| p.to_string()).collect();

    let matches = super::match_paths_to_entries(&entries, &manifest_paths);
    assert_eq!(
        matches,
        vec![
            super::PathEntry {
                index: 1,
                path: "/data/one.txt".to_string()
            },
            super::PathEntry {
                index: 2,
                path: "/data/two.bin".to_string()
            },
            super::PathEntry {
                index: 3,
                path: "/data/three.xml".to_string()
            },
        ],
        "entry n + 1 is manifest line n, recovered by digest rather than assumed"
    );
}
