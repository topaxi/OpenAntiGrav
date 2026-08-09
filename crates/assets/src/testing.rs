//! Hand-authored archives for this crate's own tests.
//!
//! Every byte here is written by these functions, per
//! [ADR-0006](../../../docs/architecture/adr/0006-no-copyrighted-content.md): a
//! test that needs an archive builds one rather than reading a player's disc.
//! The layout is the one in
//! [`docs/formats/wad.md`](../../../docs/formats/wad.md), which is also why
//! these are useful - if the writer here and the reader in `oag-formats` ever
//! disagree about the header, the tests stop passing.

use std::path::Path;

use oag_formats::wad::{BLOB_ALIGNMENT, ENTRY_LEN, HEADER_LEN, KNOWN_VERSION, hash_name};

/// A WAD holding one uncompressed blob per `(name, contents)` pair.
#[must_use]
pub fn wad(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let alignment = BLOB_ALIGNMENT as usize;
    let directory = HEADER_LEN + entries.len() * ENTRY_LEN;
    let mut offset = directory.div_ceil(alignment) * alignment;

    let count = u32::try_from(entries.len()).expect("a test archive of sane size");
    let mut out = Vec::new();
    out.extend_from_slice(&KNOWN_VERSION.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    for (name, blob) in entries {
        let len = u32::try_from(blob.len()).expect("a test blob of sane size");
        out.extend_from_slice(&hash_name(name).to_le_bytes());
        out.extend_from_slice(
            &u32::try_from(offset)
                .expect("a test archive of sane size")
                .to_le_bytes(),
        );
        // Stored, not compressed: the two size fields are equal, which is what
        // `Compression` reads as "stored" whatever bit 31 says.
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&len.to_le_bytes());
        offset += (blob.len().max(1)).div_ceil(alignment) * alignment;
    }

    for (_, blob) in entries {
        let start = out.len().div_ceil(alignment) * alignment;
        out.resize(start, 0);
        out.extend_from_slice(blob);
    }
    out
}

/// Writes [`wad`] to `dir/name` and returns the path as a spec string.
///
/// # Panics
///
/// If the file cannot be written, which in a test means the temporary
/// directory is not usable and every assertion after it would be noise.
#[must_use]
pub fn write_wad(dir: &Path, name: &str, entries: &[(&str, &[u8])]) -> String {
    let path = dir.join(name);
    std::fs::write(&path, wad(entries)).expect("writing a test archive");
    path.to_string_lossy().into_owned()
}

/// A directory of this test's own, cleared first so a rerun starts clean.
///
/// # Panics
///
/// If the directory cannot be created.
#[must_use]
pub fn temp_dir(name: &str) -> std::path::PathBuf {
    let directory = std::env::temp_dir().join(format!("oag-assets-{name}"));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("creating a test directory");
    directory
}
