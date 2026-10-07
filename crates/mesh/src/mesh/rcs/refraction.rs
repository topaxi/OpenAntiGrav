//! Vineta K's tunnel glass: a surface that reads the behind-the-glass target.
//!
//! `mt_tunnelrefraction` and `cl_tunnelrefraction` declare a sampler,
//! `0x88a0df95`, that no `.rcsmaterial` record names a texture for - the
//! engine binds it - and sample it at a screen coordinate the program builds
//! (clip `xy / w`, flipped, perturbed by a normal map). It is a copy of the
//! picture the engine renders itself, **not** an environment probe the disc
//! fails to ship: the one input is something this renderer draws too. Read off the
//! disassembly (`scripts/ps3-microcode.py`, `mt_tunnelrefraction` block
//! `@0x1fb0`, `cl_tunnelrefraction` block `@0x18b0`), the colour is
//!
//! ```text
//! out = fog(C + grab * W)
//! C   = diffuse * lit + specular      (what every lit material already is)
//! W   = diffuse.a * 0x78575769        (mt: the picture's alpha, [0.2, 0.6, 0.6])
//! W   = 2 * 0x78575769                (cl: no picture; [1, 1, 1])
//! ```
//!
//! and the output alpha is a constant, so this is not a blended surface in the
//! authored sense: it **replaces** the pixel with something that includes the
//! pixel. `0x78575769` is authored in the record, so the teal is data, not a
//! guess. `fog(x) = f x + (1 - f) fogColour`, which is why the two terms below
//! each carry `f`.
//!
//! # How it is drawn
//!
//! One opaque draw, depth written, as the original draws it (blend off,
//! depth `LEQUAL` with write on, on every glass draw of Vineta K's RPCS3
//! capture). The grab is **not the frame behind the glass**: the engine binds
//! its behind-the-glass target, a 640x360 picture of the sky and the chunks
//! flagged `rcsmodel::RENDER_BEHIND_GLASS`, drawn under a projection 4/3 wider
//! in tangent than the main view's (`oag_raceplay`'s `scene::behind_glass`,
//! `mesh::rcs::View`). The program projects a point through that target's own
//! matrix (`refractProject`, `0x590bc10e`, [`crate::mesh_render::Scene::refraction`])
//! and reads it there; `mesh.wesl`'s `refraction::opaque_colour` is the
//! equation. Read off the live programs of the capture
//! (`scripts/ps3-fp-live.py`), confidence 85:
//!
//! - the point is the surface's own position plus its (normal-mapped) normal,
//!   one world unit: the engine's `distortion` (`0x9fc59444`) is
//!   `(0, 0, -1, 1)` live, its `z` negating a negated normal and its `xy` the
//!   coordinate bias, zero;
//! - `refractProject`'s rows are the target matrix's `x + w`, `y - w`, `z`,
//!   `w`, and the program samples at `(x/w, -y/w)` of that, which taken
//!   literally reads `(1 + x/w, 1 - y/w)`, twice the target's own coordinate.
//!   **What is drawn is the target's own coordinate**, `0.5 + 0.5 (x/w, -y/w)`
//!   of its matrix, because the captured picture agrees with that and not with
//!   the literal reading: over 7,341 pane pixels of the capture, 90 % of those
//!   whose red is zero land on a target texel whose red is zero under it,
//!   none under the literal one. Where the factor of two goes is not read.
//!
//! **Not drawn: the normal map's perturbation of the normal**, so the glass
//! reads the target at the vertex normal's offset; it is a shimmer over the
//! sea, and the point it moves is one unit away. Chosen, not measured.
//!
//! # What it reaches
//!
//! The hash is declared by six slots disc-wide, all Vineta K (both
//! directions): `hd_refraction_census.rs @0x88a0df95`.

use oag_rcs::rcsmaterial::fragment::Program;
use oag_rcs::{rcsmaterial, rcsmodel};

use crate::mesh::{Emissive, TextureSlots, slots};

use super::emissive::EMISSIVE_LIMIT;
use super::{Report, Textures};

/// The engine-bound screen-colour sampler.
pub(super) const SCREEN_GRAB: u32 = 0x88a0_df95;
/// `DiffuseTexture`.
const DIFFUSE: u32 = 0x11cb_4f74;
/// The weight `W` of the grab: a float3 in the record.
const WEIGHT: u32 = 0x7857_5769;
/// `DiffuseColour`, which `cl_tunnelrefraction` multiplies its lit term by.
const DIFFUSE_COLOUR: u32 = 0x512f_8e65;

/// Whether a resolved program is a screen-grab refraction: it declares the
/// engine-bound grab, the record binds no texture to it, and the program
/// samples it.
fn reads_the_screen(
    material: &rcsmodel::Material,
    declared: &rcsmaterial::Declared,
    program: &Program,
) -> bool {
    let Some(&(_, unit)) = declared.samplers.iter().find(|&&(h, _)| h == SCREEN_GRAB) else {
        return false;
    };
    material.samplers.iter().all(|&(h, _)| h != SCREEN_GRAB)
        && program
            .instructions
            .iter()
            .any(|i| i.is_texture() && u32::from(i.unit) == unit)
}

/// Marks every screen-grab material's slot and gives it its table entry.
pub(super) fn refraction(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    textures: Textures<'_>,
    packed: &mut [u32],
    (skins, seconds): (&mut TextureSlots, &mut TextureSlots),
    table: &mut Vec<Emissive>,
    report: &mut Report,
) {
    for (slot, material) in model.materials.iter().enumerate() {
        let Some(variant) = variants.get(slot).copied().flatten() else {
            continue;
        };
        let Some(blob) = textures(&format!("/{}", material.name)) else {
            continue;
        };
        let (Some(declared), Some(program)) = (
            rcsmaterial::Declared::parse(&blob, variant.fragment.offset),
            Program::parse(&blob, variant.fragment.offset),
        ) else {
            continue;
        };
        if !reads_the_screen(material, &declared, &program) {
            continue;
        }
        let value_of = |hash: u32| material.parameters.iter().find(|p| p.hash == hash);
        let Some(weight) = value_of(WEIGHT) else {
            report.refraction_unread += 1;
            continue;
        };
        // A program that samples a picture weights the grab by the picture's
        // alpha; one that does not takes the grab doubled and tints its own
        // lit term by `DiffuseColour` instead.
        let picture = declared.samplers.iter().any(|&(h, _)| h == DIFFUSE);
        let layer = Emissive {
            tint: [weight.value[0], weight.value[1], weight.value[2]],
            offset: if picture {
                1.0
            } else {
                value_of(DIFFUSE_COLOUR).map_or(1.0, |p| p.value[0])
            },
            scale: if picture { 1.0 } else { 2.0 },
            rate: f32::from(u8::from(picture)),
        };
        let index = table.iter().position(|seen| *seen == layer).or_else(|| {
            (table.len() + 1 < EMISSIVE_LIMIT).then(|| {
                table.push(layer);
                table.len() - 1
            })
        });
        let Some(index) = index else {
            report.refraction_unread += 1;
            continue;
        };
        if !picture {
            skins[slot] = None;
            seconds[slot] = None;
        }
        if let Some(word) = packed.get_mut(slot) {
            *word &= slots::ROLE_MASK & !slots::ADD_SECOND;
            *word |= slots::REFRACTION;
            *word |= u32::try_from(index + 1).unwrap_or(0) << slots::MATERIAL_SHIFT;
        }
        report.refraction_bound += 1;
    }
}
