//! [`MaterialSetup`]: one `.rcsmodel`'s whole material table, read once before
//! any geometry is emitted.
//!
//! Split out of `mesh/rcs.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use oag_rcs::rcsmodel;

use super::skin::{self, flips, picks, roles, skin, variants};
use super::{Report, TextureSlots, Textures, curve_track, cutout, emissive, slots};

/// One `.rcsmodel`'s whole material setup: every texture, lightmap, slot
/// role, specular exponent, additive-glow table and alpha-test reference a
/// caller's [`crate::mesh::Model`] needs before it emits a single vertex.
///
/// Shared by [`super::build_with_options`] and [`super::pads::build_pad_class`] -
/// both read the same per-material tables off the same file, keyed the same
/// way, and a caller that re-derived them by hand would drift from this one
/// the moment either grew a term. It has grown five already
/// (`material_specular_exponent`, `emissive`, `alpha_test_ref`, the `FLIP_V`
/// fold, `material_anim`) since the reading this was still one inline block.
pub(super) struct MaterialSetup {
    pub(super) textures: TextureSlots,
    pub(super) lightmaps: TextureSlots,
    pub(super) material_slots: Vec<u32>,
    pub(super) material_specular_exponent: Vec<f32>,
    pub(super) material_variants: Vec<Option<oag_rcs::rcsmaterial::Variant>>,
    pub(super) emissive: Vec<crate::mesh::Emissive>,
    pub(super) alpha_test_ref: Option<f32>,
    /// The magstrip emissive pictures and waves - see [`super::mag_wave`].
    pub(super) mag_emissive: TextureSlots,
    pub(super) wave_maps: TextureSlots,
    /// Which of [`crate::mesh::Model::anim_tracks`] each material slot
    /// drives - see [`curve_track::material_anim_tracks`].
    pub(super) material_anim: Vec<u32>,
    /// The tracks themselves, for a caller to append to
    /// [`crate::mesh::Model::anim_tracks`].
    pub(super) anim_tracks: Vec<crate::mesh::AnimTrack>,
}

pub(super) fn material_setup(
    model: &rcsmodel::Model,
    model_blob: &[u8],
    textures: Textures<'_>,
    report: &mut Report,
) -> MaterialSetup {
    // **The variant first**, because which sampler entry each of this
    // renderer's two bindings comes from is a property of the shader the
    // lit-race key resolves to, not of the entry's position - see
    // `skin::picks`.
    let material_variants = variants(model, textures, report);
    let picks = picks(model, &material_variants, textures);
    let (mut skins, mut seconds) = skin(model, &picks, textures, report);
    // After the variants, because the roles are read off the resolved one.
    let skin::Roles {
        packed: mut material_slots,
        specular_exponent: material_specular_exponent,
    } = roles(
        model,
        &material_variants,
        &picks,
        &seconds,
        textures,
        report,
    );
    // **The coordinate's orientation, off the resolved *vertex* block** rather
    // than the fragment one the roles come from, and folded into the same word
    // because it is the same kind of statement: what this material's own
    // microcode says. See `skin::flips`.
    for (packed, flipped) in
        material_slots
            .iter_mut()
            .zip(flips(model, &material_variants, textures))
    {
        if flipped {
            *packed |= slots::FLIP_V;
        }
    }
    // The additive glow, read off the same resolved variant the roles are -
    // and after the flip, because it writes into the same word. See
    // `mesh::slots::ADD_SECOND`.
    let emissive = emissive::emissive(
        model,
        &material_variants,
        &picks,
        &seconds,
        &mut material_slots,
        textures,
        report,
    );
    // After the glow table, because it clears a slot's `ADD_SECOND` and
    // writes its own entry into the same table.
    let mut emissive = emissive;
    let (mag_emissive, wave_maps) = super::mag_wave::mag_wave(
        model,
        &material_variants,
        textures,
        &mut material_slots,
        (&mut skins, &mut seconds),
        &mut emissive,
        report,
    );
    // The light cone's own combine claims its slots after the glow table is
    // final, for the same reason: it writes an entry into it.
    super::light_cone::light_cone(
        model,
        &material_variants,
        textures,
        &mut material_slots,
        (&mut skins, &mut seconds),
        &mut emissive,
        report,
    );
    // The disc's own alpha-test reference, for a caller's cutout draws - see
    // `cutout`, which reports a comparison this shader cannot reproduce
    // rather than drawing one wrongly.
    let alpha_test_ref = cutout::reference(model, report);
    // Every material's own animated curve, if it carries one - generic across
    // every `.rcsmodel` this crate builds, not gantry-specific. See
    // `curve_track`'s own module doc for why this is not fitted to one file.
    let (mut material_anim, mut anim_tracks) = curve_track::material_anim_tracks(model, model_blob);
    // After the curves: a material that authors one keeps it, and the vertex
    // scroll takes only the slots nothing else animates.
    super::vertex_scroll::vertex_scroll(
        model,
        &material_variants,
        textures,
        &mut material_anim,
        &mut anim_tracks,
        report,
    );
    MaterialSetup {
        textures: skins,
        lightmaps: seconds,
        material_slots,
        material_specular_exponent,
        material_variants,
        emissive,
        alpha_test_ref,
        mag_emissive,
        wave_maps,
        material_anim,
        anim_tracks,
    }
}
