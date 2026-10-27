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
    pub(super) material_glow: Vec<f32>,
    pub(super) material_colour_factor: Vec<bool>,
    pub(super) material_variants: Vec<Option<oag_rcs::rcsmaterial::Variant>>,
    /// Per material slot: whether the material ships a `StaticQuake` vertex
    /// program, so the Quake's displacement reaches it - see
    /// [`crate::mesh::GpuVertex::quake`].
    pub(super) material_quake: Vec<bool>,
    /// Per material slot: whether the hull's chunks carry their tangent - see
    /// [`super::hull_normal`].
    pub(super) material_tangent: Vec<bool>,
    pub(super) emissive: Vec<crate::mesh::Emissive>,
    pub(super) alpha_test_ref: Option<f32>,
    /// The magstrip emissive pictures and waves - see [`super::mag_wave`].
    pub(super) mag_emissive: TextureSlots,
    pub(super) wave_maps: TextureSlots,
    /// The fifth and sixth pictures of a program that samples more than four
    /// units - see [`crate::mesh::Model::fifth_maps`].
    pub(super) fifth_maps: TextureSlots,
    pub(super) sixth_maps: TextureSlots,
    /// Which of [`crate::mesh::Model::anim_tracks`] each material slot
    /// drives - see [`curve_track::material_anim_tracks`].
    pub(super) material_anim: Vec<u32>,
    /// The tracks themselves, for a caller to append to
    /// [`crate::mesh::Model::anim_tracks`].
    pub(super) anim_tracks: Vec<crate::mesh::AnimTrack>,
}

impl MaterialSetup {
    /// The setup a model was already built with, read back off it: what a
    /// second build of the same file reuses instead of decoding every texture
    /// again (`super::scene::build_scene_views`). Every slot table is cloned,
    /// and a texture slot is a shared pointer, so this copies no pixels.
    pub(super) fn of(model: &crate::mesh::Model) -> Self {
        Self {
            textures: model.textures.clone(),
            lightmaps: model.lightmaps.clone(),
            material_slots: model.material_slots.clone(),
            material_specular_exponent: model.material_specular_exponent.clone(),
            material_glow: model.material_glow.clone(),
            material_colour_factor: model.material_colour_factor.clone(),
            material_variants: model.material_variants.clone(),
            material_quake: model.material_quake.clone(),
            material_tangent: model.material_tangent.clone(),
            emissive: model.emissive.clone(),
            alpha_test_ref: model.alpha_test_ref,
            mag_emissive: model.pad_masks.clone(),
            wave_maps: model.wave_maps.clone(),
            fifth_maps: model.fifth_maps.clone(),
            sixth_maps: model.sixth_maps.clone(),
            material_anim: model.material_anim.clone(),
            anim_tracks: model.anim_tracks.clone(),
        }
    }
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
    let (material_variants, material_quake) = variants(model, textures, report);
    let picks = picks(model, &material_variants, textures);
    let (mut skins, mut seconds) = skin(model, &picks, textures, report);
    // After the variants, because the roles are read off the resolved one.
    let skin::Roles {
        packed: mut material_slots,
        specular_exponent: material_specular_exponent,
        colour_factor: material_colour_factor,
        glow: material_glow,
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
    // The billboard and the stand glass keep `ADD_SECOND` and swap the shared
    // additive entry for their program's own.
    super::billboard_glass::billboard_glass(
        model,
        &material_variants,
        textures,
        &mut material_slots,
        &seconds,
        &mut emissive,
        report,
    );
    let (mut mag_emissive, mut wave_maps) = super::mag_wave::mag_wave(
        model,
        &material_variants,
        textures,
        &mut material_slots,
        (&mut skins, &mut seconds),
        &mut emissive,
        report,
    );
    // The Zone hull's light cells ride the magstrips' two bindings and a glow entry.
    let (cell_blues, cell_masks) = super::damage_lights::damage_lights(
        model,
        &material_variants,
        textures,
        &mut material_slots,
        &mut emissive,
        report,
    );
    super::mag_wave::merge(&mut mag_emissive, cell_blues);
    super::mag_wave::merge(&mut wave_maps, cell_masks);
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
    // The holographic billboard writes an entry too, and binds its second
    // layer third, beside the magstrips' and the pads' masks.
    let holo_layers = super::holo_billboard::holo_billboard(
        model,
        &material_variants,
        textures,
        &mut material_slots,
        (&mut skins, &mut seconds),
        &mut emissive,
        report,
    );
    super::mag_wave::merge(&mut mag_emissive, holo_layers);
    // A screen-grab refraction claims its slots last, for the same reason:
    // it writes an entry into the table.
    super::refraction::refraction(
        model,
        &material_variants,
        textures,
        &mut material_slots,
        (&mut skins, &mut seconds),
        &mut emissive,
        report,
    );
    // The glass sheen's `c` rides in the table too.
    super::glass_sheen::bias(model, &mut material_slots, &mut emissive);
    // Its lightmapped variant binds the grid third, the atlas holding `lightmap`.
    let glass_grids =
        super::glass_sheen::lightmapped_grids(model, &material_variants, textures, report);
    super::mag_wave::merge(&mut mag_emissive, glass_grids);
    // The Sebenco ice pool writes three entries into the table and binds its
    // pond mask third, beside what the magstrips and pads bind there.
    let ice_masks = super::ice::ice(
        model,
        &material_variants,
        textures,
        &mut material_slots,
        &mut skins,
        &mut emissive,
        report,
    );
    super::mag_wave::merge(&mut mag_emissive, ice_masks);
    // Sebenco's snow-capped rock binds six pictures: its mask third, the snow's normal
    // fourth, the rock and its normal fifth and sixth, and three entries.
    let snow = super::snow_rock::snow_rock(
        model,
        &material_variants,
        textures,
        &mut material_slots,
        &mut skins,
        &mut emissive,
        report,
    );
    super::mag_wave::merge(&mut mag_emissive, snow.mask);
    super::mag_wave::merge(&mut wave_maps, snow.snow_normal);
    // `nr_twinblend` binds its wipe third and its bars fourth, and writes one entry.
    let twin = super::twin_blend::twin_blend(
        model,
        &material_variants,
        textures,
        &mut material_slots,
        (&mut skins, &mut seconds),
        &mut emissive,
        report,
    );
    super::mag_wave::merge(&mut mag_emissive, twin.wipe);
    super::mag_wave::merge(&mut wave_maps, twin.bars);
    // `dc_hologramwithstatic` binds its static third and writes one entry.
    let statics = super::static_hologram::static_hologram(
        model,
        &material_variants,
        textures,
        &mut material_slots,
        (&mut skins, &mut seconds),
        &mut emissive,
        report,
    );
    super::mag_wave::merge(&mut mag_emissive, statics);
    // A hull's normal map rides the same third binding, and its materials are
    // none of the families above.
    let (hull_maps, mut material_tangent) =
        super::hull_normal::hull_normal(model, &material_variants, textures, &mag_emissive, report);
    for (tangent, snow) in material_tangent.iter_mut().zip(&snow.tangent) {
        *tangent |= snow;
    }
    super::mag_wave::merge(&mut mag_emissive, hull_maps);
    // Water reads its picture as a normal map and has none to paint.
    super::water::water(
        model,
        &material_variants,
        textures,
        &mut skins,
        &mut material_slots,
        &mut emissive,
        report,
    );
    // The hulls that read the reflection probe claim a slot's glow index last: ice ORs its
    // index into whatever the slot holds, so a tag laid earlier would be corrupted, and every
    // other family above keeps the slot it took (a tagged slot is counted, not overwritten).
    super::probe_lit::probe_lit(
        model,
        &material_variants,
        textures,
        &mut material_slots,
        &mut emissive,
        report,
    );
    // Scenery glass that reads the sky reflection claims its slot after the same families.
    // Its glow masks ride the third binding, read at the second coordinate set.
    let window_masks = super::window_glass::window_glass(
        model,
        &material_variants,
        textures,
        &mut material_slots,
        &mut emissive,
        report,
    );
    super::mag_wave::merge(&mut mag_emissive, window_masks);
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
    // After the curves, because it appends a track of its own (the outline's offset is its
    // material's curve driving a parameter), and after every glow-index family, because it
    // only takes a slot none of them did.
    let (chevron_emasks, chevron_waves) = super::chevron_pulse::chevron_pulse(
        model,
        model_blob,
        &material_variants,
        textures,
        &mut material_slots,
        &mut emissive,
        &mut anim_tracks,
        report,
    );
    let (mut fifth_maps, mut sixth_maps) = (snow.rock, snow.rock_normal);
    super::mag_wave::merge(&mut fifth_maps, chevron_emasks);
    super::mag_wave::merge(&mut sixth_maps, chevron_waves);
    MaterialSetup {
        textures: skins,
        lightmaps: seconds,
        material_slots,
        material_specular_exponent,
        material_glow,
        material_colour_factor,
        material_variants,
        material_quake,
        material_tangent,
        emissive,
        alpha_test_ref,
        mag_emissive,
        wave_maps,
        fifth_maps,
        sixth_maps,
        material_anim,
        anim_tracks,
    }
}
