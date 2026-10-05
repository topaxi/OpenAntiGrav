//! Overlays an [`oag_texture::ship_skin::Skin`] onto an already-built
//! [`super::Model`]'s texture slots.
//!
//! **What selects a skin for a race is not decided yet**: nothing collects a
//! team's `PI_ModelSkin` paths, and no caller reads an Alternative or
//! Eliminator pick. This is the other
//! half, decoupled from that question: given a skin, put its four blocks
//! where the original's own `Skin_ApplyToModel` would. Nothing here reads a
//! `.dat` off an archive or a `PI_ModelSkin`'s `Unlock`; a caller that has
//! both bytes in hand supplies them.
//!
//! # Matching a block to a slot needs no second parse of the model
//!
//! `ship-skin.md` traces the original matching each block against the last
//! path component of the hull's own texture name, case-insensitively -
//! `\TEXTURE1.TGA` against `texture1.tga`. [`super::build_with_textures`]
//! already keeps exactly that string as [`super::ModelTexture::label`] (the
//! runtime asset path's own last component, the same rule this reads), so
//! matching against the already-built [`super::Model::textures`] needs no
//! second look at the source `.vex` bytes.
//!
//! # The PS2 build swaps one whole atlas instead
//!
//! A PS2 hull has no `\TEXTUREn.TGA` slots; it names one 256x256
//! `ALL_Textures.tga` atlas. The PS2 port's `Skin_SwapAtlasSibling` never reads
//! the `.dat` at all: it maps the skin's file name to a sibling atlas entry
//! beside the hull's own and copies that over the atlas whole. [`is_ps2_atlas`],
//! [`ps2_atlas_path`], [`ps2_atlas_sibling`] and [`apply_ps2_atlas`] are that
//! path. See `docs/ghidra/functions/ps2-pulse-eu/ship-skin.md`.

use std::sync::Arc;

use oag_texture::ship_skin;
use oag_vex::vex;

use super::{Model, ModelTexture};

/// A slot's own label, lower-cased, in [`ship_skin::Skin::blocks`] order.
///
/// `ship-skin.md`'s `g_skin_texture_slot_names` table, spelled the way
/// [`super::ModelTexture::label`] holds it: the shipped models spell the
/// runtime path in lower case, and the match is case-insensitive either way.
pub const SLOT_NAMES: [&str; 4] = [
    "texture1.tga",
    "texture2.tga",
    "texture3.tga",
    "texture4.tga",
];

/// Which [`ship_skin::Skin::blocks`] index a texture slot's own label takes,
/// if any.
///
/// Public because the size check a *caller* wants needs the same pairing this
/// module already does: a skin file declares no dimensions at all, so whether
/// a block matches the slot it lands in is only answerable by holding the two
/// side by side, and doing that positionally over a model's slot list pairs
/// the wrong ones on any hull that skips a slot.
#[must_use]
pub fn slot_of(label: &str) -> Option<usize> {
    SLOT_NAMES
        .iter()
        .position(|name| label.eq_ignore_ascii_case(name))
}

/// Replaces every texture slot of `hull` that `skin` names, in place.
///
/// Returns how many of `hull`'s slots were replaced, `0..=4`. A caller that
/// gets `0` back has been handed a skin file for a different hull, or a hull
/// with no `\TEXTUREn.TGA`-named slots at all - the front-end preview model
/// is the latter case, since it references only `texture1`..`texture3` and
/// so never matches block 4. Neither is treated as an error here: which
/// outcome is worth reporting is a caller decision, the same way a missing
/// hull or a missing sibling file already is in `crate::mesh` and
/// `oag_game::livery`.
///
/// **Only the stored fourth block, never the composite.** `ship-skin.md`
/// records that the original composes a distinct 64x64 atlas out of blocks
/// 1-3 when the applier's own flag argument is zero, and leaves its top-right
/// quadrant as uninitialised stack. Reproducing that is deliberately out of
/// scope - see the module docs on both this file and `ship_skin.rs` - so this
/// always uploads block 4 itself, the same as the applier's non-zero-flag
/// path.
pub fn apply(hull: &mut Model, skin: &ship_skin::Skin) -> usize {
    let mut applied = 0;
    for slot in &mut hull.textures {
        let Some(existing) = slot else { continue };
        let Some(index) = slot_of(&existing.label) else {
            continue;
        };
        let block = &skin.blocks[index];
        *slot = Some(Arc::new(ModelTexture::rgba8(
            existing.label.clone(),
            block.width as u32,
            block.height as u32,
            block.to_rgba(),
            None,
        )));
        applied += 1;
    }
    applied
}

/// The two spellings of a PS2 hull's one paintable atlas that
/// `Skin_SwapAtlasSibling` (`0x001dfb70` in `SCES_547.48`) matches, as a
/// file name. Feisar authors the second; the other eleven teams the first.
/// See `docs/ghidra/functions/ps2-pulse-eu/ship-skin.md`.
pub const PS2_ATLAS_NAMES: [&str; 2] = ["all_textures.tga", "textures_all.tga"];

/// Whether a texture slot's own label is a PS2 hull's paintable atlas.
/// Case-insensitive: the original upper-cases the name before it matches, and
/// the disc spells it three different ways.
#[must_use]
pub fn is_ps2_atlas(label: &str) -> bool {
    PS2_ATLAS_NAMES
        .iter()
        .any(|name| label.eq_ignore_ascii_case(name))
}

/// The declared runtime path of `data`'s PS2 atlas `Texture` node, if the
/// model has one - `Data\Ships\<Team>\Textures\ALL_Textures.tga` and its
/// spellings. The directory [`ps2_atlas_sibling`] builds in comes from here,
/// because that is where the original takes it from: the node's own name.
#[must_use]
pub fn ps2_atlas_path(data: &[u8]) -> Option<String> {
    let classes = vex::classes_of(data).ok()?;
    let texture = classes.texture?;
    let nodes = vex::nodes(data).ok()?;
    nodes
        .iter()
        .filter(|node| node.class_id == texture)
        .filter_map(|node| vex::texture_asset_path(data.get(node.payload())?))
        .find(|path| is_ps2_atlas(path.rsplit(['/', '\\']).next().unwrap_or(path)))
}

/// The sibling atlas a PS2 skin file names, built the way
/// `Skin_SwapAtlasSibling` builds it, with the `.mip` extension the original
/// appends (its `Texture_FindOrLoad` then rewrites that to `.pct`).
///
/// Only the skin's **file name** is consulted, never its bytes:
/// `ship_alt.dat` gives `<dir>\livery.mip`, `ship_eliminator.dat`
/// `<dir>\<stem>_eliminator.mip`, and `ship.dat` `<dir>\<stem>2.mip`. Any other
/// name is `None`, the original's own no-op. `atlas_path` is the atlas node's
/// declared path, see [`ps2_atlas_path`].
#[must_use]
pub fn ps2_atlas_sibling(atlas_path: &str, skin_entry: &str) -> Option<String> {
    let skin_file = skin_entry.rsplit(['/', '\\']).next().unwrap_or(skin_entry);
    let (dir, atlas_file) = atlas_path.rsplit_once(['/', '\\'])?;
    let stem = atlas_file
        .rsplit_once('.')
        .map_or(atlas_file, |(stem, _)| stem);
    let file = if skin_file.eq_ignore_ascii_case("ship_alt.dat") {
        "livery".to_string()
    } else if skin_file.eq_ignore_ascii_case("ship_eliminator.dat") {
        format!("{stem}_eliminator")
    } else if skin_file.eq_ignore_ascii_case("ship.dat") {
        format!("{stem}2")
    } else {
        return None;
    };
    Some(format!(r"{dir}\{file}.mip"))
}

/// Replaces every PS2 atlas slot of `hull` with `atlas`, whole, in place -
/// the original's `memcpy` of the sibling's image over the hull atlas's own.
/// Returns how many slots were replaced. A sibling whose size differs from the
/// slot it would land in is refused: the original copies the *hull* atlas's
/// length, which only means anything when the two are the same shape, and
/// every shipped sibling is.
pub fn apply_ps2_atlas(hull: &mut Model, width: u32, height: u32, rgba: &[u8]) -> usize {
    let mut applied = 0;
    for slot in &mut hull.textures {
        let Some(existing) = slot else { continue };
        if !is_ps2_atlas(&existing.label) || existing.width != width || existing.height != height {
            continue;
        }
        *slot = Some(Arc::new(ModelTexture::rgba8(
            existing.label.clone(),
            width,
            height,
            rgba.to_vec(),
            None,
        )));
        applied += 1;
    }
    applied
}

#[cfg(test)]
mod tests;
