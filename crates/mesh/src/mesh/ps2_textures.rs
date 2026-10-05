//! Decoding a PlayStation 2 track's texture set, and resolving a model's
//! materials against it **by name**, the mechanism the original uses.
//!
//! Split out of `mesh.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`.
//!
//! See `docs/formats/ps2-texture.md`'s "how a model finds its texture set"
//! section: `Texture_FindOrLoad` (`0x0010c1e0` in `SCES_547.48`) is the only
//! texture-resolution primitive in the binary, and it is name-based -
//! `strcpy`, strip a build prefix, rewrite `.TGA`/`.MIP` to `.PCT`, hash with
//! `Wad_HashNameString`, look up in a hash-keyed cache. No ordinal-addressed
//! texture API exists anywhere in the executable. A flat, directory-order
//! index (this module's own shape before 2026-09-05) collapses whenever a
//! model's `Texture` nodes repeat a name, which the nested WAD then holds
//! once rather than once per node - 87 of `12_Track`'s 152 ordinals bound
//! their neighbour's texture this way, and one, the sky's, fell off the end
//! entirely and drew white. Resolving by name never sees that collapse: a
//! repeated name simply resolves to the one entry it always named.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use oag_formats::wad;
use oag_vex::vex;

use super::ModelTexture;

/// A PS2 model's texture set, keyed by each entry's own `name_hash` field -
/// the same hash [`wad::hash_name`] produces for a node's declared name, and
/// the same one the outer archive's own directory uses.
///
/// Built once per model and consulted by [`resolve_texture_slots`] for every
/// `Texture`-class node, rather than being indexed positionally.
#[derive(Debug)]
pub struct Ps2TextureSet {
    by_hash: HashMap<u32, Arc<ModelTexture>>,
    /// The nested WAD's own directory length, for a loader report - see
    /// [`Self::entry_count`].
    entry_count: usize,
}

impl Ps2TextureSet {
    /// Parses a nested texture-set WAD, decoding every entry it can and
    /// keying the result by the entry's own `name_hash` rather than by its
    /// position in the directory.
    ///
    /// An entry this build cannot decode (an unsupported compression, a
    /// `Zlib` blob, truncated bytes) is simply absent from the map - a lookup
    /// against its hash then misses the same way a name with no entry at all
    /// would, which is the honest outcome rather than a placeholder.
    pub fn parse(blob: &[u8]) -> Result<Self> {
        wad::Directory::peek_entry_count(blob).context("not a nested WAD")?;
        let directory = wad::Directory::parse(blob, Some(blob.len() as u64))
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("parsing the texture set directory")?;

        let entry_count = directory.entries.len();
        let mut by_hash = HashMap::with_capacity(entry_count);
        for entry in &directory.entries {
            let start = entry.offset as usize;
            let end = start + entry.size as usize;
            let Some(stored) = blob.get(start..end) else {
                continue;
            };
            let decompressed = match entry.compression {
                wad::Compression::None => stored.to_vec(),
                wad::Compression::Lzss => {
                    match oag_formats::lzss::decompress(stored, entry.size_uncompressed as usize) {
                        Ok(bytes) => bytes,
                        Err(_) => continue,
                    }
                }
                wad::Compression::Zlib => continue,
            };
            let Ok(texture) = oag_texture::ps2_texture::parse(&decompressed) else {
                continue;
            };
            by_hash.insert(
                entry.name_hash,
                Arc::new(ModelTexture::rgba8(
                    format!("{:08x}", entry.name_hash),
                    u32::from(texture.width),
                    u32::from(texture.height),
                    texture.to_rgba(),
                    None,
                )),
            );
        }
        Ok(Self {
            by_hash,
            entry_count,
        })
    }

    /// How many entries the nested WAD's own directory declared, whether or
    /// not each one decoded - for a loader report to say how much of the set
    /// came through. See [`Self::decoded_count`] for how many actually did.
    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.entry_count
    }

    /// How many distinct textures decoded, keyed by name hash.
    ///
    /// Can be smaller than [`Self::entry_count`] for two different reasons,
    /// and this count does not distinguish them: an entry this build cannot
    /// decode, or two entries sharing a name hash - the exact collapse this
    /// module exists to resolve correctly rather than to avoid.
    #[must_use]
    pub fn decoded_count(&self) -> usize {
        self.by_hash.len()
    }

    /// Resolves one node's own declared texture name against this set,
    /// applying the same canonicalisation `Texture_FindOrLoad` does: strip a
    /// leading build path, then rewrite a source-art extension to `.pct`,
    /// then hash.
    ///
    /// `None` on a miss - a name that resolves to nothing is left untextured,
    /// never substituted with another entry.
    #[must_use]
    pub fn resolve(&self, declared_name: &str) -> Option<Arc<ModelTexture>> {
        let stripped = wad::ps2_strip_build_prefix(declared_name);
        let canonical = wad::ps2_texture_name(&stripped).unwrap_or(stripped);
        self.by_hash.get(&wad::hash_name(&canonical)).cloned()
    }
}

/// Resolves a model's texture slots against a [`Ps2TextureSet`], one call
/// per `Texture`-class node in file order - the shape [`super::TextureSlots`]
/// has always had, one entry per node, but each slot now filled by the
/// node's own declared name rather than by its position among the set's
/// entries.
///
/// A node with no declared name, or whose name resolves to nothing in the
/// set, comes back `None` - drawn as nothing, per this project's rule
/// against standing in a neighbour's texture for a genuine miss.
///
/// **The returned slot's own [`ModelTexture::label`] is the node's declared
/// name, not [`Ps2TextureSet::resolve`]'s own `{name_hash:08x}` label.** A
/// WAD entry carries only a hash, never a string (`oag_formats::wad::Entry`
/// has no name field), so that hash is the only identity the *set* can give
/// a texture - but a consumer matching a slot by name, the way
/// `oag_game::livery::ship_skin::apply` matches `\TEXTUREn.TGA` and
/// `crate::gantry` matches a billboard name, needs the same
/// last-path-component label the PSP embedded path already produces (see
/// `super::build_class`'s `embedded` construction). Leaving the hash in
/// place is why a PS2 ship's alternative skin used to select and never draw:
/// `ship_skin::slot_of` compared `"3a1f2b4c"` against `"texture1.tga"` and
/// never matched, so every apply reported 0 of N slots repainted.
///
/// Relabeled once per distinct declared name rather than once per node, so a
/// track's hundreds of material slots naming a handful of distinct textures
/// still cost one clone per texture, the same dedup [`Ps2TextureSet::by_hash`]
/// already does by hash - not one clone per slot, which would undo the memory
/// sharing [`super::ModelTexture`]'s own doc measured.
#[must_use]
pub fn resolve_texture_slots(
    data: &[u8],
    nodes: &[vex::Node],
    texture_class: Option<u32>,
    set: &Ps2TextureSet,
) -> super::TextureSlots {
    let mut relabeled: HashMap<String, Arc<ModelTexture>> = HashMap::new();
    nodes
        .iter()
        .filter(|n| Some(n.class_id) == texture_class)
        .map(|node| {
            let payload = data.get(node.payload())?;
            let name = vex::texture_asset_path(payload)?;
            let texture = set.resolve(&name)?;
            let label = name.rsplit(['/', '\\']).next().unwrap_or(&name).to_string();
            Some(
                relabeled
                    .entry(label.clone())
                    .or_insert_with(|| {
                        Arc::new(ModelTexture {
                            label,
                            ..(*texture).clone()
                        })
                    })
                    .clone(),
            )
        })
        .collect()
}
