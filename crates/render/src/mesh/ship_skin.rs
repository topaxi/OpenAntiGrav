//! Overlays an [`oag_formats::ship_skin::Skin`] onto an already-built
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

use std::sync::Arc;

use oag_formats::ship_skin;

use super::{Model, ModelTexture};

/// A slot's own label, lower-cased, in [`ship_skin::Skin::blocks`] order.
///
/// `ship-skin.md`'s `g_skin_texture_slot_names` table, spelled the way
/// [`super::ModelTexture::label`] holds it: the shipped models spell the
/// runtime path in lower case, and the match is case-insensitive either way.
const SLOT_NAMES: [&str; 4] = [
    "texture1.tga",
    "texture2.tga",
    "texture3.tga",
    "texture4.tga",
];

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
        let Some(index) = SLOT_NAMES
            .iter()
            .position(|name| existing.label.eq_ignore_ascii_case(name))
        else {
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

#[cfg(test)]
mod tests;
