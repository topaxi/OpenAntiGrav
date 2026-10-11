//! The reads that are about an entry's neighbourhood in a WAD rather than its
//! name: a `.fnt` with its glyph atlas, and the entry before or after another.
//!
//! Split out of [`super`] under the 1,000-line rule; a move.

use super::{Archives, Held, Neighbour};
use crate::{Error, Result};

impl Archives {
    /// Reads a `.fnt` and returns it with its glyph atlas attached, wherever
    /// that atlas lives on this source.
    ///
    /// The two builds differ and the file itself says which is which: a PSP
    /// `.fnt` carries its atlas inside it, a PS2 `.fnt` stops where the atlas
    /// would begin and keeps the glyph sheet in the **next archive entry**, as
    /// a [`PSMT4`](oag_texture::ps2_texture::Layout::Psmt4) texture. So the
    /// embedded atlas is tried first and the following entry is only read when
    /// there is none, which means a PSP source never touches the fallback and
    /// this needs no platform test.
    ///
    /// # Errors
    ///
    /// [`Error::NoSuchEntry`] when `name` is not on this source, and
    /// [`Error::Font`] when the metrics, or the atlas the following entry is
    /// supposed to hold, do not decode.
    pub fn read_font(&mut self, name: &str) -> Result<oag_texture::fnt::Font> {
        use oag_texture::{fnt, ps2_texture};

        let blob = self.read_name(name)?;
        let metrics = fnt::Metrics::parse(&blob).map_err(|source| Error::Font {
            name: name.to_string(),
            source,
        })?;
        if metrics.has_embedded_atlas(&blob) {
            return fnt::Font::parse(&blob).map_err(|source| Error::Font {
                name: name.to_string(),
                source,
            });
        }

        let atlas = self.read_following(name)?;
        let texture = ps2_texture::parse(&atlas).map_err(|source| Error::FontAtlas {
            name: name.to_string(),
            source,
        })?;
        // The GS calls 128 fully opaque; `alpha_at` is read as 0-255 coverage.
        let palette = texture
            .palette
            .iter()
            .map(|&[r, g, b, a]| [r, g, b, a.saturating_mul(2)])
            .collect();
        fnt::Font::with_atlas(
            metrics,
            texture.width,
            texture.height,
            palette,
            texture.indices,
        )
        .map_err(|source| Error::Font {
            name: name.to_string(),
            source,
        })
    }

    /// Reads the entry immediately before `name`'s own entry, in whichever
    /// archive holds `name`.
    ///
    /// This is where a PS2 model's texture set sits: not addressed by any
    /// name or hash a model declares, but by directory position. Checked
    /// against every recovered ship team independently, not just one: each
    /// team's `Ship.vex` has the archive entry directly before it decode as a
    /// texture set with exactly as many entries as the model has `Texture`
    /// nodes. See `docs/formats/ps2-texture.md`.
    ///
    /// # Errors
    ///
    /// [`Error::NoSuchEntry`] when `name` itself is not found, or its entry is
    /// first in the directory and nothing precedes it.
    pub fn read_preceding(&mut self, name: &str) -> Result<Vec<u8>> {
        self.read_neighbour(name, Neighbour::Before)
    }

    /// Reads the entry immediately after `name`'s own entry, in whichever
    /// archive holds `name`.
    ///
    /// The other half of the PS2's directory-position addressing: a model's
    /// texture set sits *before* it ([`Self::read_preceding`]), and a font's
    /// glyph atlas sits *after* it. All five `.fnt` entries in `WADS2.WAD` are
    /// followed by a `PSMT4` texture whose dimensions cover that font's own
    /// glyph boxes; see `docs/formats/fnt.md`.
    ///
    /// # Errors
    ///
    /// [`Error::NoSuchEntry`] when `name` itself is not found, or its entry is
    /// last in the directory and nothing follows it.
    pub fn read_following(&mut self, name: &str) -> Result<Vec<u8>> {
        self.read_neighbour(name, Neighbour::After)
    }

    /// Directory position is only meaningful **within one archive**, so the
    /// neighbour is always taken from whichever archive held `name` - never
    /// from a flattened view across all of them, which would hand a model the
    /// last entry of the previous archive and decode it as a texture set.
    fn read_neighbour(&mut self, name: &str, which: Neighbour) -> Result<Vec<u8>> {
        let hash = oag_formats::wad::hash_name(name);
        let at = self.holder_of_archive(name).unwrap_or(Held::Data);
        // A PSARC's entry order is its manifest's and carries no meaning, so it
        // is refused here rather than answered with the entry that happens to
        // sit next to this one. See `Error::NotAWad`.
        let archive = self.held(at).as_wad_mut("the neighbouring entry")?;
        let index = archive
            .index_of_hash(hash)
            .ok_or_else(|| Error::NoSuchEntry {
                archive: archive.label().to_string(),
                hash,
                name: Some(name.to_string()),
            })?;
        let (neighbour, edge) = match which {
            Neighbour::Before => (index.checked_sub(1), "first"),
            Neighbour::After => (
                Some(index + 1).filter(|&next| next < archive.directory().entries.len()),
                "last",
            ),
        };
        let neighbour = neighbour.ok_or_else(|| Error::NoSuchEntry {
            archive: archive.label().to_string(),
            hash,
            name: Some(format!("{name} ({edge} in its directory)")),
        })?;
        archive.read(neighbour)
    }
}
