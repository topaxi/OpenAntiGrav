//! Writing a mod folder as an `.oagmod`: a zip whose entries are compressed
//! with zstd (zip method 93), stored when the data is already compressed.

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::{Error, MANIFEST, Mod};

/// Extensions whose bytes are already compressed: stored, since compressing
/// them again costs time and gains nothing.
const STORED: &[&str] = &[
    "png", "ogg", "jpg", "jpeg", "webp", "mp3", "opus", "zip", "zst",
];

/// What [`pack`] wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packed {
    /// Entries written with zstd.
    pub zstd: usize,
    /// Entries stored as they are.
    pub stored: usize,
    /// Bytes read from the folder.
    pub bytes_in: u64,
    /// Bytes of the finished file.
    pub bytes_out: u64,
}

/// Checks the folder's manifest, then writes `out` with every file under it.
///
/// # Errors
///
/// The folder is not a valid mod, or a file will not read or write.
pub fn pack(folder: &Path, out: &Path) -> Result<Packed, Error> {
    let module = Mod::open(folder)?;
    let origin = folder.display().to_string();
    let io = |source| Error::Io {
        origin: origin.clone(),
        source,
    };
    let zip = |source| Error::Zip {
        origin: origin.clone(),
        source,
    };
    let mut writer = ZipWriter::new(File::create(out).map_err(io)?);
    let mut packed = Packed {
        zstd: 0,
        stored: 0,
        bytes_in: 0,
        bytes_out: 0,
    };
    let mut names = module.names();
    // The manifest first, so a reader finds it without scanning.
    names.sort_by_key(|name| (name != MANIFEST, name.clone()));
    for name in names {
        let mut bytes = Vec::new();
        File::open(folder.join(&name))
            .and_then(|mut file| file.read_to_end(&mut bytes))
            .map_err(io)?;
        let already = Path::new(&name)
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| STORED.iter().any(|s| s.eq_ignore_ascii_case(ext)));
        let method = if already {
            packed.stored += 1;
            CompressionMethod::Stored
        } else {
            packed.zstd += 1;
            CompressionMethod::Zstd
        };
        let mut options = SimpleFileOptions::default().compression_method(method);
        if method == CompressionMethod::Zstd {
            options = options.compression_level(Some(19));
        }
        writer.start_file(name, options).map_err(zip)?;
        writer.write_all(&bytes).map_err(io)?;
        packed.bytes_in += bytes.len() as u64;
    }
    writer.finish().map_err(zip)?;
    packed.bytes_out = std::fs::metadata(out).map_err(io)?.len();
    Ok(packed)
}
