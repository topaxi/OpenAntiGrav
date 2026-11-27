//! HD's Detonator cannon bolt: `hd_detonator_cannonbolt_halo`, the one material of
//! `detonator_cannonbolt`.
//!
//! Read off the fogged lit block the race resolves (`HalfBrightAmbientSunSpot0SVC0`, fragment
//! `@0x1b70`, vertex `@0x1a10`; `scripts/ps3-microcode.py fp-file`, `vp-file`;
//! `docs/rendering/hd-unlit-programs.md`, "The Detonator cannon bolt"). The vertex program splits
//! one `float4` attribute: `.xy` (`uv0`) to `TC0.w`/`TC1.w`, `.zw` (`uv1`) to `TC3`. With `t` the
//! engine clock, unit 0 `hd_detonator_cannonbolt` (a blue band, a cloud in its alpha), unit 1
//! `hd_detonator_cannonbolt_glow` (a 4x128 strip whose alpha is eight rings, brightest at `v = 0`)
//! and `fB` the model's `fresnelBase`:
//!
//! ```text
//! noise = tex0(uv1).a                                   @0x12
//! c     = tex0(uv1 + 0.8 noise + 5 t)                   @0x13, @0x1a, @0x23 - both axes
//! ring  = tex1(uv0).a                                   @0x1e
//! rim   = sat(1 - N.V);  F = fB + (1 - fB) rim^5        @0x0b..@0x1f
//! rgb   = c.rgb + (1 - F)^3 (0.0619202, 0.744629, 1)   @0x25..@0x2e; fogged, then `/2`
//! a     = 5 c.a ring                                    @0x21, @0x2b, @0x31
//! ```
//!
//! Matched by what the program declares and the literals it authors, never by the material's
//! name. [`PROGRAM_CANNON_BOLT`] draws it; the entry's offset is `fB` and its tint the
//! program's. 2048's and Omega's programs are `super::psp2`'s `cannon_bolt`.

use std::sync::Arc;

use oag_rcs::rcsmaterial::{self, fragment::Program};
use oag_rcs::rcsmodel;

use crate::mesh::{Emissive, ModelTexture, PROGRAM_CANNON_BOLT, TextureSlots, slots};

use super::emissive::EMISSIVE_LIMIT;
use super::skin::decode_texture;
use super::{Report, Textures};

/// `~crc32("fresnelBase")`.
pub const FRESNEL_BASE: u32 = 0x743e_a80b;
/// `~crc32("fogColour")`.
const FOG_COLOUR: u32 = 0x3dc3_1258;
/// `~crc32("globalAlphaScaler")`.
const GLOBAL_ALPHA_SCALER: u32 = 0x4c13_d3af;
/// `~crc32("time")`.
const TIME: u32 = 0x906b_67ba;

/// The literals block `@0x1b70` authors, in program order: `1 - x` of the rim, the noise's
/// displacement, the Fresnel exponent, `log2(e)` of the fog, the clock's rate, `1 - F`, the tint
/// and the alpha's gain.
pub const LITERALS: [[f32; 4]; 8] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.8, 0.0, 0.0, 0.0],
    [5.0, 0.0, 0.0, 0.0],
    [0.0, 1.442_694_9, 0.0, 0.0],
    [5.0, 0.0, 0.0, 0.0],
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 0.061_920_2, 0.744_629, 1.0],
    [5.0, 0.0, 0.0, 0.0],
];

/// Whether `program` is the bolt's: the four parameters it declares, two samplers and its own
/// literals, each within `1e-6` (the listing prints the tint to six places).
fn is_cannon_bolt(declared: &rcsmaterial::Declared, program: &Program) -> bool {
    let mut parameters = declared.parameters.clone();
    parameters.sort_unstable();
    let mut want = [FOG_COLOUR, GLOBAL_ALPHA_SCALER, FRESNEL_BASE, TIME];
    want.sort_unstable();
    let literals = super::rim_glow::literals(declared, program);
    declared.samplers.len() == 2
        && parameters == want
        && literals.len() == LITERALS.len()
        && literals
            .iter()
            .zip(&LITERALS)
            .all(|(got, want)| got.iter().zip(want).all(|(g, w)| (g - w).abs() < 1.0e-6))
}

/// Whether `roles` names an entry of [`PROGRAM_CANNON_BOLT`] in `table`.
pub(super) fn is_bound(roles: u32, table: &[Emissive]) -> bool {
    slots::material_index(roles)
        .checked_sub(1)
        .and_then(|i| table.get(i as usize))
        .is_some_and(|entry| entry.program[0] == PROGRAM_CANNON_BOLT)
}

/// One coordinate pair per vertex.
type Coordinates = Vec<[f32; 2]>;

/// `(texcoords, lightmap_texcoords)` of a bolt chunk: `uv1` (unit 0's) and `uv0` (unit 1's), or
/// `None` for a chunk not bound to this program.
pub(super) fn route(
    roles: u32,
    table: &[Emissive],
    (mesh, model_blob, submesh, stride): (&rcsmodel::Mesh, &[u8], &rcsmodel::SubMesh, usize),
) -> Option<(Coordinates, Coordinates)> {
    if !is_bound(roles, table) {
        return None;
    }
    let (uv0, uv1) = mesh.inline_uv_pairs(model_blob, submesh, stride).ok()?;
    Some((uv1, uv0))
}

/// Binds each bolt material's two pictures (unit 0 as the albedo, unit 1 second) and gives it an
/// entry of [`PROGRAM_CANNON_BOLT`] holding its `fresnelBase`.
pub(super) fn cannon_bolt(
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
        if !is_cannon_bolt(&declared, &program) {
            continue;
        }
        let mut unit = |unit: u32| -> Option<Arc<ModelTexture>> {
            let hash = declared.samplers.iter().find(|s| s.1 == unit)?.0;
            let path = material
                .samplers
                .iter()
                .find(|(h, p)| *h == hash && p.is_some())
                .and_then(|(_, p)| p.clone())?;
            textures(&format!("/{path}"))
                .and_then(|bytes| decode_texture(&path, &bytes))
                .map(Arc::new)
        };
        let fresnel = material
            .parameters
            .iter()
            .find(|p| p.hash == FRESNEL_BASE)
            .map(|p| p.value[0]);
        let (Some(unit0), Some(unit1), Some(fresnel), Some(word)) =
            (unit(0), unit(1), fresnel, packed.get_mut(slot))
        else {
            continue;
        };
        if table.len() + 1 >= EMISSIVE_LIMIT {
            continue;
        }
        table.push(Emissive {
            tint: [LITERALS[6][1], LITERALS[6][2], LITERALS[6][3]],
            offset: fresnel,
            scale: 0.0,
            rate: 0.0,
            program: [PROGRAM_CANNON_BOLT, 0.0],
        });
        let Ok(index) = u32::try_from(table.len()) else {
            continue;
        };
        skins[slot] = Some(unit0);
        seconds[slot] = Some(unit1);
        *word &= slots::ROLE_MASK
            & !(slots::ADD_SECOND
                | slots::SECOND_IS_LIGHTMAP
                | slots::ALBEDO_FROM_SECOND
                | slots::ALPHA_FROM_SECOND);
        *word |= index << slots::MATERIAL_SHIFT;
        super::census::claim(slot, "cannon_bolt", super::census::Outcome::Drawn);
        report.cannon_bolt_bound += 1;
    }
}
