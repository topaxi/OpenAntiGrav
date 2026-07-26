//! Collects viewable assets out of a disc image.
//!
//! Kept separate from the renderer so it can be tested without a GPU, and so
//! the viewer never learns anything about disc layout.

use anyhow::{Context, Result, bail};
use oag_disc::DiscImage;
use oag_formats::lzss;
use oag_formats::texture::Texture;
use oag_formats::wad::{self, Compression, Directory};

/// One decoded texture, ready to upload.
pub struct Asset {
    /// How to refer to it: a resolved name if known, otherwise index and hash.
    pub label: String,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// RGBA8888, row-major from the top left.
    pub rgba: Vec<u8>,
}

/// Loads every texture from an archive inside a disc image.
///
/// `spec` is `<image>:<path-on-disc>`, matching `oag-wad`.
pub fn load(spec: &str, names: Option<&std::path::Path>) -> Result<Vec<Asset>> {
    let (image, inner) = spec
        .rsplit_once(':')
        .filter(|(image, inner)| image.len() >= 2 && !inner.is_empty())
        .context("expected <image>:<path-on-disc>")?;

    let mut disc = DiscImage::open(image).with_context(|| format!("opening {image}"))?;
    let entry = disc
        .entries()?
        .iter()
        .find(|e| !e.is_directory && e.path.eq_ignore_ascii_case(inner))
        .cloned()
        .with_context(|| format!("{inner} is not on {image}"))?;

    let header = disc.read_entry_range(&entry, 0, wad::HEADER_LEN as u64)?;
    let count =
        Directory::peek_entry_count(&header).map_err(|e| anyhow::anyhow!("{inner}: {e}"))?;
    let dir_bytes = disc.read_entry_range(&entry, 0, Directory::directory_len(count))?;
    let dir = Directory::parse(&dir_bytes, Some(entry.size))
        .map_err(|e| anyhow::anyhow!("{inner}: {e}"))?;

    let names = names.map(load_names).transpose()?.unwrap_or_default();

    let mut assets = Vec::new();
    for (index, wad_entry) in dir.entries.iter().enumerate() {
        if wad_entry.size == 0 {
            continue;
        }

        let raw = disc.read_entry_range(
            &entry,
            u64::from(wad_entry.offset),
            u64::from(wad_entry.size),
        )?;

        let data = match wad_entry.compression {
            Compression::None => raw,
            Compression::Lzss => {
                match lzss::decompress(&raw, wad_entry.size_uncompressed as usize) {
                    Ok(d) => d,
                    // One bad entry should not stop the viewer opening.
                    Err(_) => continue,
                }
            }
            Compression::Zlib => continue,
        };

        let Ok(texture) = Texture::parse(&data) else {
            continue;
        };

        assets.push(Asset {
            label: names
                .get(&wad_entry.name_hash)
                .cloned()
                .unwrap_or_else(|| format!("#{index} {:08x}", wad_entry.name_hash)),
            width: u32::from(texture.width),
            height: u32::from(texture.height),
            rgba: texture.to_rgba(),
        });
    }

    if assets.is_empty() {
        bail!("{inner} contains no textures this build can decode");
    }
    Ok(assets)
}

fn load_names(path: &std::path::Path) -> Result<std::collections::HashMap<u32, String>> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;

    let mut map = std::collections::HashMap::new();
    for line in text.lines() {
        let name = line.trim();
        if !name.is_empty() && !name.starts_with('#') {
            map.entry(wad::hash_name(name))
                .or_insert_with(|| name.to_string());
        }
    }
    Ok(map)
}
