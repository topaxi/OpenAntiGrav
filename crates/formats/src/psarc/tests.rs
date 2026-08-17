//! A synthetic archive, built to the layout `docs/formats/psarc.md` records.
//!
//! The real corpus is checked in `crates/formats/tests/psarc_ground_truth.rs`,
//! which needs a disc image. These tests exist so the block walk, the
//! block-width probe and the digest can fail on a machine that has none.

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

/// The width is probed, and the probe is only ever *sound* one way round.
///
/// A table of an odd number of bytes cannot be pairs, so a 3-byte width is
/// recovered. An even one can always be read as pairs, so a 4-byte width is
/// genuinely undecidable from the table alone - narrowest wins, and this test
/// pins that rather than pretending otherwise. Nothing on the one disc this
/// project holds exercises either branch: all seven archives are 2.
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

#[test]
fn the_manifest_tolerates_crlf_and_blank_lines() {
    assert_eq!(
        parse_manifest(b"/a.txt\r\n/b.txt\r\n\r\n"),
        vec!["/a.txt".to_string(), "/b.txt".to_string()]
    );
}
