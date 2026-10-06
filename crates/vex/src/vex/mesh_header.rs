//! The mesh payload's own header: which materials a mesh carries, and which
//! **render layer** it is drawn in.
//!
//! Split out of [`super`]: these are the fields `Mesh_InitFromPayload`
//! (`0x0890e998`) reads at load, and the layer derivation is the first thing in
//! this crate answering a question about *ordering* rather than geometry.
//!
//! ```text
//! +0x00  u16   mesh flags
//! +0x02  u16   material_count
//! +0x0c  u16   a second flag word - only its `0x8` bit is read, by `mesh_layer`
//! +0x30  material[material_count], stride 0x14
//! ```

use super::u16_at;
use super::u32_at;

/// One material of a mesh, from the stride-`0x14` array at `+0x30`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Material {
    /// The `u16` at `+0x00`: render state, not an animation switch.
    ///
    /// Surveyed across every material of all 12 PSP circuits: richly varied (19
    /// distinct values on `07_Track` alone), correlated with the artists' naming,
    /// but it does **not** separate animated from static surfaces
    /// (`flicker1nonalpha_GLOW` and the static `hub_banner_GLOW` both carry
    /// `0x91`). Two bits are legible:
    ///
    /// - `0x0080` accompanies the `_GLOW`/additive naming convention.
    /// - `0x2000` lands on exactly the `*_shinemap` textures, corroborating the
    ///   "extra pass" reading of the same bit in a batch's `pass_mask` and of
    ///   [`Material::second_texture`].
    ///
    /// Nothing consumes it yet; recorded so the census is reproducible from the
    /// parser. See `docs/formats/vex.md`.
    pub flags: u16,
    /// The `u32` at `+0x04`: an index into the model's texture array, from
    /// [`textures`].
    pub texture: u32,
    /// The `u32` at `+0x08`: the second texture of the `0x2000` extra pass. Zero
    /// on every material of `07_Track`; a Pulse hull's materials name
    /// `envtest4bit.tga` (or `envmap_stripe2.tga`) here, which `oag_render::shine`
    /// draws.
    pub second_texture: u32,
}

/// Materials of one mesh payload, stride 0x14 from `+0x30`.
///
/// Positional as [`textures`] is: a batch selects a material by index, so one
/// running past the payload comes back as `None` rather than renumbering the rest.
///
/// The remaining `+0x0c..0x14` is **proven zero** on every material of every PSP
/// circuit, ruling out an authored per-surface UV scroll rate and forcing
/// texture-keyed animation.
#[must_use]
pub fn mesh_materials(payload: &[u8]) -> Vec<Option<Material>> {
    if payload.len() < 0x30 {
        return Vec::new();
    }
    let count = usize::from(u16_at(payload, 2));
    (0..count)
        .map(|i| {
            let at = 0x30 + i * 0x14;
            (at + 0x0c <= payload.len()).then(|| Material {
                flags: u16_at(payload, at),
                texture: u32_at(payload, at + 4),
                second_texture: u32_at(payload, at + 8),
            })
        })
        .collect()
}

/// The ordinary scene layer, and the only model key the derivation acts on:
/// `Vex_LoadModel`'s third argument. A model loaded with it has each mesh re-keyed
/// by [`mesh_layer`]; any other key is kept (so `<Team>boost.vex` stays wholly on
/// [`LAYER_EXHAUST`]).
pub const LAYER_SCENE: u32 = 0x4500_0000;

/// The layer a mesh gets when no other branch claims it. **The default, not a
/// special case** (see [`mesh_layer`]); it sorts *after* [`LAYER_SCENE`], so a
/// `0x45` mesh draws first.
pub const LAYER_DEFAULT: u32 = 0x4a00_0000;

/// The layer for a mesh whose payload `+0x0c` has `0x8` set: the lowest of the
/// three the derivation produces, drawn first.
pub const LAYER_EARLY: u32 = 0x3100_0000;

/// The layer `ExhaustFlare_Submit` enqueues at, the only one whose items carry a
/// depth in the low twenty bits. Not derived: it is the model key
/// `<Team>boost.vex` is loaded with, named so the ordering table has all rows.
pub const LAYER_EXHAUST: u32 = 0x4d00_0000;

/// First of the eight planar-reflection layers, `0x01500000`; each further one
/// is `0x1000000` above the last.
///
/// A mesh is diverted here when any material has `(material+0x03) & 0xfc`
/// non-zero, that field `>> 2` being the index. **Not derived by [`mesh_layer`]**:
/// the byte is inside a *batch's* material reference, not the payload header, and
/// no caller needs it yet. Named because these sort below every other layer (so
/// `0x31` is not the first thing drawn).
pub const LAYER_REFLECTION_FIRST: u32 = 0x0150_0000;

/// Which render layer a mesh is drawn in, or `None` when `model_layer` is not
/// [`LAYER_SCENE`] and the mesh therefore keeps its model's key.
///
/// # The rule, and it is a read rather than an inference
///
/// `Mesh_InitFromPayload` (`0x0890e998`, confidence 85) re-derives the layer from
/// two words of the payload header, only when the model was loaded with
/// [`LAYER_SCENE`]:
///
/// ```text
/// payload[+0x0c] & 0x8                      -> 0x31000000
/// else flags & 0x0080                       -> 0x4a000000
/// else flags & 0x2000 && flags & 0x0040     -> 0x4a000000
/// else flags & 0x1000                       -> 0x45000000
/// else                                      -> 0x4a000000
/// ```
///
/// Three of the five branches land on [`LAYER_DEFAULT`], so **the narrow case is
/// `0x45`**: `0x1000` set, `0x0080` clear, and not both `0x2000` and `0x0040`.
///
/// # What the layer is for
///
/// The top twelve bits of the sort key every drawable submits to the one render
/// queue, which `Gfx_FlushRenderManager` (`0x0891e3c0`) sorts ascending: **lower
/// draws first.** The low twenty bits are a back-to-front depth that mesh
/// geometry does not compute (a batch set enqueues with its bare layer key), so
/// within a layer the order is submission order. See
/// `docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`.
#[must_use]
pub fn mesh_layer(payload: &[u8], model_layer: u32) -> Option<u32> {
    if payload.len() < 0x0e {
        return None;
    }
    if model_layer != LAYER_SCENE {
        return Some(model_layer);
    }
    let flags = u16_at(payload, 0);
    if u16_at(payload, 0x0c) & 0x8 != 0 {
        return Some(LAYER_EARLY);
    }
    if flags & 0x0080 != 0 {
        return Some(LAYER_DEFAULT);
    }
    if flags & 0x2000 != 0 && flags & 0x0040 != 0 {
        return Some(LAYER_DEFAULT);
    }
    if flags & 0x1000 != 0 {
        return Some(LAYER_SCENE);
    }
    Some(LAYER_DEFAULT)
}

/// The GU vertex type of a mesh payload's first batch, in either list.
///
/// **`Some(0)` is the answer that matters**: a batch declaring no vertex format
/// has its vertices elsewhere. Wipeout HD and 2048 export that way, geometry in a
/// `.rcsmodel` beside the `.vex` and batch headers left behind. See
/// `oag_mesh::mesh::geometry_is_external`.
///
/// `None` when the payload is too short for a batch list or neither list has a
/// batch: says nothing either way, so not folded into `Some(0)`.
#[must_use]
pub fn mesh_first_vertex_type(payload: &[u8]) -> Option<u16> {
    if payload.len() < 0x30 {
        return None;
    }
    for batch_list in [0u8, 1u8] {
        let at = u32_at(payload, if batch_list == 0 { 4 } else { 8 }) as usize;
        let terminator = if batch_list == 0 { 1u16 } else { 2 };
        if at + 0x40 > payload.len() || u16_at(payload, at) & terminator == 0 {
            continue;
        }
        return Some(u16_at(payload, at + 0x0a));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A payload header with the given flag word and `+0x0c`.
    fn header(flags: u16, at_0c: u16) -> Vec<u8> {
        let mut out = vec![0u8; 0x30];
        out[0..2].copy_from_slice(&flags.to_le_bytes());
        out[0x0c..0x0e].copy_from_slice(&at_0c.to_le_bytes());
        out
    }

    /// The one mesh whose flag word is written down elsewhere:
    /// `docs/ghidra/functions/psp-pulse-usa/mesh-draw.md` measures
    /// `bflare1Shape`/`bflare2Shape` at `mesh_flags = 0x1232` off
    /// `Data\Ships\Assegai\shipboost.vex`. It exercises the narrow branch.
    const PLUME_FLAGS: u16 = 0x1232;

    #[test]
    fn the_plumes_own_flag_word_takes_the_narrow_branch() {
        assert_eq!(
            mesh_layer(&header(PLUME_FLAGS, 0), LAYER_SCENE),
            Some(LAYER_SCENE)
        );
    }

    /// **And the plume is not actually drawn there**: its model is loaded with
    /// `0x4d000000` and the derivation only fires on [`LAYER_SCENE`], so the
    /// exhaust does not sort with the scenery.
    #[test]
    fn a_model_outside_the_scene_layer_keeps_its_own_key() {
        assert_eq!(
            mesh_layer(&header(PLUME_FLAGS, 0), LAYER_EXHAUST),
            Some(LAYER_EXHAUST)
        );
    }

    #[test]
    fn the_0x0c_bit_outranks_every_flag() {
        for flags in [0x0000, PLUME_FLAGS, 0x0080, 0x2040] {
            assert_eq!(
                mesh_layer(&header(flags, 0x8), LAYER_SCENE),
                Some(LAYER_EARLY),
                "flags {flags:#06x}"
            );
        }
    }

    /// Three of the five branches land here, which makes `0x45` the narrow case.
    #[test]
    fn the_default_branch_is_the_wide_one() {
        for flags in [0x0000, 0x0080, 0x1080, 0x2040, 0x3040] {
            assert_eq!(
                mesh_layer(&header(flags, 0), LAYER_SCENE),
                Some(LAYER_DEFAULT),
                "flags {flags:#06x}"
            );
        }
    }

    /// `0x2000` alone does not divert; it needs `0x0040` beside it (the branch a
    /// reader is most likely to simplify away).
    #[test]
    fn the_extra_pass_bit_needs_its_partner() {
        assert_eq!(
            mesh_layer(&header(0x3000, 0), LAYER_SCENE),
            Some(LAYER_SCENE)
        );
        assert_eq!(
            mesh_layer(&header(0x3040, 0), LAYER_SCENE),
            Some(LAYER_DEFAULT)
        );
    }

    #[test]
    fn a_payload_too_short_to_hold_the_header_is_not_guessed_at() {
        assert_eq!(mesh_layer(&[0u8; 8], LAYER_SCENE), None);
    }

    /// The layers are named in draw order; an ascending sort is what the original
    /// applies.
    #[test]
    fn the_constants_are_in_draw_order() {
        let order = [
            LAYER_REFLECTION_FIRST,
            LAYER_EARLY,
            LAYER_SCENE,
            LAYER_DEFAULT,
            LAYER_EXHAUST,
        ];
        let mut sorted = order;
        sorted.sort_unstable();
        assert_eq!(
            order, sorted,
            "the constants are declared out of draw order"
        );
    }
}
