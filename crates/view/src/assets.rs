//! Collects viewable assets out of a disc image.
//!
//! Kept separate from the renderer so it can be tested without a GPU, and so
//! the viewer never learns anything about disc layout.

use anyhow::{Context, Result, bail};
use oag_assets::Archive;
use oag_formats::wad;
use oag_texture::ps2_texture;
use oag_texture::texture::Texture;

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
    let mut archive = Archive::open(spec)?;
    let entries = archive.directory().entries.clone();
    let names = names.map(load_names).transpose()?.unwrap_or_default();

    let mut assets = Vec::new();
    for (index, wad_entry) in entries.iter().enumerate() {
        if wad_entry.size == 0 {
            continue;
        }

        // One bad or undecodable entry should not stop the viewer opening.
        let Ok(data) = archive.read(index) else {
            continue;
        };

        // Two unrelated texture formats live on these discs, and the check for
        // each is the blob's own size arithmetic, so trying both cannot
        // misclassify: a PSP `.mip` is never the size a GS upload packet is.
        let decoded = Texture::parse(&data)
            .map(|t| (u32::from(t.width), u32::from(t.height), t.to_rgba()))
            .ok()
            .or_else(|| {
                ps2_texture::parse(&data)
                    .map(|t| (u32::from(t.width), u32::from(t.height), t.to_rgba()))
                    .ok()
            });
        let Some((width, height, rgba)) = decoded else {
            continue;
        };

        assets.push(Asset {
            label: names
                .get(&wad_entry.name_hash)
                .cloned()
                .unwrap_or_else(|| format!("#{index} {:08x}", wad_entry.name_hash)),
            width,
            height,
            rgba,
        });
    }

    if assets.is_empty() {
        bail!(
            "{} contains no textures this build can decode",
            archive.label()
        );
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
