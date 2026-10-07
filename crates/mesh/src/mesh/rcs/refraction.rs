//! Vineta K's tunnel glass: a surface that reads the screen behind it.
//!
//! `mt_tunnelrefraction` and `cl_tunnelrefraction` declare a sampler,
//! `0x88a0df95`, that no `.rcsmaterial` record names a texture for - the
//! engine binds it - and sample it at a screen coordinate the program builds
//! (clip `xy / w`, flipped, perturbed by a normal map). It is a copy of the
//! frame behind the glass, **not** an environment probe the disc fails to
//! ship: the one input is something this renderer holds itself. Read off the
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
//! No scene-copy pass exists, and none is needed to draw this: the equation is
//! linear in the grab, so a chunk is emitted twice (`emit`), both blended and
//! with depth write off like every transparent draw:
//!
//! 1. [`GRAB_BLEND`], `dst = dst * src`, whose shader output is `f * W`;
//! 2. [`ADD_BLEND`], `dst = dst + src`, whose shader output is the lit `C`.
//!
//! **Not drawn: the normal map's offset of the grab coordinate.** The grab is
//! read at the pixel's own position. The offset is `normal.xy` times the
//! engine's screen scale (`0x9fc59444`), a few pixels of shimmer over a
//! uniform teal sea; leaving it out moves no edge. Chosen, not measured.
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

/// The pass that scales what is behind the glass: `dst = dst * src`.
pub(super) const GRAB_BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::Zero,
        dst_factor: wgpu::BlendFactor::Src,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: KEEP_ALPHA,
};

/// The pass that adds the glass's own lit colour: `dst = dst + src`.
pub(super) const ADD_BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: KEEP_ALPHA,
};

/// Neither pass touches the target's alpha, which is the bloom's glow mask.
const KEEP_ALPHA: wgpu::BlendComponent = wgpu::BlendComponent {
    src_factor: wgpu::BlendFactor::Zero,
    dst_factor: wgpu::BlendFactor::One,
    operation: wgpu::BlendOperation::Add,
};

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
