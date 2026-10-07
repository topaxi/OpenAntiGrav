//! Validates the PSARC reader against the real PS3 disc.
//!
//! **`#[ignore]`d, never run in CI**: needs game content (`just test-data`);
//! see `docs/architecture/adr/0006-no-copyrighted-content.md`. Tests skip with
//! a printed message when the image is absent; `OAG_REQUIRE_GAME_DATA=1` turns
//! that into a failure, since a skipped ground-truth test proves nothing.
//!
//! # Which image
//!
//! `hdfury-ps3-eu.iso` is per-sector AES-128-CBC; `oag_disc` reads it in place
//! when the disc key sits beside it, and `ps3_crypt_ground_truth` proves that
//! byte-identical. These tests keep reading `hdfury-ps3-eu-dec.iso` (made by
//! `scripts/ps3iso.py decrypt`) so a machine without the key still runs them
//! (see `docs/formats/ps3-disc.md`); neither key nor image is committed.
//!
//! # What these are for
//!
//! The load-bearing one is [`every_entry_carries_md5_of_its_own_uppercased_path`],
//! one check tying three readings that could each be wrong alone: the manifest
//! parse (a path off by one line hashes to nothing), the entry ordering (nothing
//! states the manifest is in entry order) and the stride (30-byte entries read a
//! byte adrift shift every digest). Lowercase matches zero entries, so the
//! uppercasing is measured.
//!
//! The rest corroborate *decompression*, which no digest reaches: a deflate
//! stream does not survive a container misread, so inflated bytes carrying the
//! magic their extension predicts check the block walk and width probe together.

use std::path::PathBuf;

use oag_disc::{DiscImage, Entry};
use oag_formats::psarc::{self, Directory, Header};

/// The decrypted PS3 image. See the module docs for where it comes from.
const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

/// Archives on the disc, all under `PS3_GAME/USRDIR/`.
const ARCHIVE_COUNT: usize = 7;

/// Entries across all seven, the manifests excluded. From
/// `docs/formats/psarc.md`.
const ENTRY_COUNT: usize = 11_664;

/// A circuit whose `.vex` is the one `docs/formats/hd-status.md` measures.
const TRACK_VEX: &str = "/data/environments/talons_junction/track.vex";

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

/// One archive, read in place out of the image at the LBA the ISO walk reports.
/// Nothing is extracted: the header, table of contents and each entry's blocks
/// are ranges within the archive's extent, so listing 2.1 GiB takes seconds.
struct Archive {
    path: String,
    entry: Entry,
    directory: Directory,
    paths: Vec<String>,
}

impl Archive {
    fn open(disc: &mut DiscImage, entry: Entry) -> Self {
        let head = disc
            .read_entry_range(&entry, 0, psarc::HEADER_LEN as u64)
            .expect("header");
        let header = Header::parse(&head).unwrap_or_else(|e| panic!("{}: {e}", entry.path));
        let toc = disc
            .read_entry_range(&entry, 0, u64::from(header.toc_len))
            .expect("toc");
        let directory = Directory::parse(&toc).unwrap_or_else(|e| panic!("{}: {e}", entry.path));

        let mut archive = Self {
            path: entry.path.clone(),
            entry,
            directory,
            paths: Vec::new(),
        };
        let manifest = archive.read(0, disc);
        archive.paths = psarc::parse_manifest(&manifest);
        archive
    }

    fn read(&self, index: usize, disc: &mut DiscImage) -> Vec<u8> {
        let (offset, len) = self
            .directory
            .entry_range(index)
            .unwrap_or_else(|e| panic!("{}: entry {index}: {e}", self.path));
        let stored = disc
            .read_entry_range(&self.entry, offset, len)
            .expect("entry bytes");
        self.directory
            .read_entry(index, &stored)
            .unwrap_or_else(|e| panic!("{}: entry {index}: {e}", self.path))
    }
}

fn archives(disc: &mut DiscImage) -> Vec<Archive> {
    let found: Vec<Entry> = disc
        .entries()
        .expect("iso walk")
        .iter()
        .filter(|e| !e.is_directory && e.path.to_ascii_lowercase().ends_with(".psarc"))
        .cloned()
        .collect();
    found
        .into_iter()
        .map(|entry| Archive::open(disc, entry))
        .collect()
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_entry_carries_md5_of_its_own_uppercased_path() {
    let Some(path) = image(PS3_IMAGE) else { return };
    let mut disc = DiscImage::open(&path).expect("open image");
    let archives = archives(&mut disc);

    assert_eq!(
        archives.len(),
        ARCHIVE_COUNT,
        "the disc ships seven .psarc archives"
    );

    let mut total = 0usize;
    let mut matched = 0usize;
    let mut lowercase_matched = 0usize;
    for archive in &archives {
        assert_eq!(
            archive.paths.len(),
            archive.directory.len() - 1,
            "{}: the manifest names every entry but itself",
            archive.path
        );
        assert_eq!(
            archive.directory.entries[0].digest, [0u8; 16],
            "{}: the manifest has no path to hash",
            archive.path
        );

        for (n, entry_path) in archive.paths.iter().enumerate() {
            total += 1;
            let stored = archive.directory.entries[n + 1].digest;
            matched += usize::from(stored == psarc::path_digest(entry_path));
            lowercase_matched +=
                usize::from(stored == psarc::md5_digest(entry_path.to_lowercase().as_bytes()));
        }
    }

    assert_eq!(total, ENTRY_COUNT, "entries across all seven archives");
    assert_eq!(
        matched, total,
        "every entry's digest is MD5 of its own path, uppercased"
    );
    assert_eq!(
        lowercase_matched, 0,
        "and the lowercase spelling - which is how the paths are stored - matches none"
    );
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_archive_is_psar_1_3_zlib_with_64_kib_blocks() {
    let Some(path) = image(PS3_IMAGE) else { return };
    let mut disc = DiscImage::open(&path).expect("open image");

    for archive in archives(&mut disc) {
        let header = archive.directory.header;
        assert_eq!(
            (header.version_major, header.version_minor),
            (1, 3),
            "{}",
            archive.path
        );
        assert_eq!(header.compression_name(), "zlib", "{}", archive.path);
        assert_eq!(header.block_size, 65_536, "{}", archive.path);
        assert_eq!(
            archive.directory.block_width, 2,
            "{}: a 64 KiB block cannot deflate past 65,535 bytes",
            archive.path
        );
    }
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn an_inflated_entry_carries_the_magic_its_extension_predicts() {
    let Some(path) = image(PS3_IMAGE) else { return };
    let mut disc = DiscImage::open(&path).expect("open image");
    let archives = archives(&mut disc);

    let (archive, index) = archives
        .iter()
        .find_map(|a| {
            a.paths
                .iter()
                .position(|p| p == TRACK_VEX)
                .map(|n| (a, n + 1))
        })
        .expect("talons_junction/track.vex is on the disc");

    let declared = archive.directory.entries[index].size;
    let data = archive.read(index, &mut disc);

    assert_eq!(
        data.len() as u64,
        declared,
        "the inflated entry is exactly as long as its directory row says"
    );
    assert_eq!(
        &data[0x0c..0x10],
        b"XXEV",
        "a .vex, byte-reversed - which a wrong block walk would not produce"
    );
    assert_eq!(
        u32::from_be_bytes([data[0], data[1], data[2], data[3]]),
        6,
        "version 6, read big-endian"
    );
    assert_eq!(
        16 + u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as u64
            + u32::from_be_bytes([data[8], data[9], data[10], data[11]]) as u64,
        declared,
        "and its two declared section lengths close on the file length"
    );
}
