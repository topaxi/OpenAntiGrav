//! Two HD scenery programs the additive-glow reading got wrong, because each
//! multiplies its second texture where the glow family adds it: the lit
//! advertising billboard (`cf_vex_billboard`) and Tech de Ra's stand glass
//! (`glasstest`).
//!
//! Both already carry [`slots::ADD_SECOND`], because `Program::accumulates`
//! sees a `MAD` whose addend is its own destination in each (one compile of the
//! billboard aside, below). Neither adds its second texture to the picture.
//! This module leaves the textures as the skin pass bound them and gives each
//! material its own glow-table entry with a [`Emissive::program`] mode, which
//! `shade.wesl` reads.
//!
//! # `cf_vex_billboard` ([`PROGRAM_BILLBOARD`])
//!
//! Lit, fogged variant (Vineta K `@0x1b40`, block #3; `scripts/ps3-microcode.py
//! fp-file`). With `t0` the unit-0 picture (`Texture1`), `g` the unit-1 glow
//! texture (`tv_glow`), `k` parameter `0x21db4cba`, `su` and `sv` parameters
//! `0xb17204e5` and `0x287b555f`, and `uv` the surface's coordinate pair:
//!
//! ```text
//! g     = k * tex1(uv.x * su, uv.y * sv)             TEX H0 unit1 ; MUL H0, H0, k
//! light = 0x81db67ea + sat(N.L) * 0x2dba643d         ambient + sun, no prelit, no sun mask
//! out   = t0 * (g.a + light * g.rgb)                 MUL H0.xyz, H1, H0 ; MUL H0.xyz, light, H0 ; MAD H0.xyz, H1, H0.w, H0
//! alpha = g.a
//! ```
//!
//! then the fog lerp and the final `/2` every lit HD program ends on. The
//! earlier reading of this page had `0xb17204e5` as `k`: it is the glow tap's
//! `u` scale (1.5 on the harimau card, 4.0 on one of Vineta K's side boards),
//! and `0x21db4cba` multiplies the glow on every lane.
//!
//! **Metropia's and Chenghou Project's copy is an older compile with no `k`**
//! (`@0x1360`, block #3, the start gantry's boards): the same taps and the same
//! light, `out = t0 * g.a + g.rgb * (t0 * light)`, the output alpha the literal
//! 1, and neither the parameter nor its multiply. Its last combine is `MAD
//! H1.xyz, H1, H4.wwww, H4`, whose addend is not its destination, so
//! `Program::accumulates` does not flag it and the slot arrives here without
//! [`slots::ADD_SECOND`]. It is the same program with `k = 1`, and takes the
//! bit here.
//!
//! # `glasstest` ([`PROGRAM_BW_GLASS`])
//!
//! Block #5 (Tech de Ra `@0x3150`), with `o` the unit-0 orange glass, `b` the
//! unit-1 `bwgradient` at the **second** coordinate set (`TC5.xy = v[8].zw`,
//! the `Uv2` half of the attribute whose `xy` half is the diffuse `Uv1`):
//!
//! ```text
//! lit  = TC0 + TC1 + TC5.z * sat(N.L) * sun          TC1 and TC2.xyz are written 0 by the paired vertex block #5
//! rgb  = b * (o.rgb * lit) + b * o.a * (TC5.z * pow(N.H, e) * sat(8 N.L) * sun)
//!        + probe reflection tinted by 0xd5814b74
//! a    = o.a * ratio(-N.V) * 1.25
//! ```
//!
//! **Drawn:** the multiply by `b` of the lit sum and of the specular. **Not
//! drawn:** the probe reflection (no dual-paraboloid probe on a world
//! surface), and the alpha's `ratio` ramp (unit 3, not bound), so the alpha is
//! `o.a`.
//!
//! # What picks them
//!
//! The declared interface alone: the sampler hashes and the parameters the
//! program names. `crates/render/examples/hd_billboard_glass_census.rs` lists
//! every material on the four PS3 archives that matches and which slot word it
//! holds.

use oag_rcs::{rcsmaterial, rcsmodel};

use crate::mesh::{
    Emissive, PROGRAM_BILLBOARD, PROGRAM_BW_GLASS, PROGRAM_SNOW, TextureSlots, slots,
};

use super::emissive::EMISSIVE_LIMIT;
use super::{Report, Textures};

/// `Texture1`, the billboard's picture at unit 0.
const BILLBOARD_PICTURE: u32 = 0x3bdc_0403;
/// The billboard's glow texture (`tv_glow`), unit 1.
const BILLBOARD_GLOW: u32 = 0xa2d5_55b9;
/// `aspect_ratio_U`: the `u` scale of the glow tap.
const GLOW_U_SCALE: u32 = 0xb172_04e5;
/// `aspect_ratio_V`: the `v` scale of the glow tap.
const GLOW_V_SCALE: u32 = 0x287b_555f;
/// `texture2overlaystrength`: what the glow sample is multiplied by, on every lane.
const GLOW_K: u32 = 0x21db_4cba;
/// The ambient colour the billboard's light starts from.
const AMBIENT: u32 = 0x81db_67ea;

/// The glass's four samplers: the orange picture, `bwgradient`, the probe and
/// the facing-ratio ramp.
const GLASS_SAMPLERS: [u32; 4] = [0x3e4c_34f2, 0x7bdf_9b36, 0x9edd_3243, 0xaad2_fd79];
/// The reflection tint, which only the glass program of this family declares.
const GLASS_REFLECTION_TINT: u32 = 0xd581_4b74;
/// The probe's own tint.
const GLASS_PROBE_TINT: u32 = 0x7480_de6d;

/// Which program a resolved material runs, and the entry it needs.
fn classify(
    declared: &rcsmaterial::Declared,
    parameters: &[rcsmodel::material::Parameter],
) -> Option<Emissive> {
    let mut samplers: Vec<u32> = declared.samplers.iter().map(|s| s.0).collect();
    samplers.sort_unstable();
    let has = |hash: u32| declared.parameters.contains(&hash);
    let value = |hash: u32| {
        parameters
            .iter()
            .find(|p| p.hash == hash)
            .map(|p| p.value[0])
    };
    let mut billboard = [BILLBOARD_PICTURE, BILLBOARD_GLOW];
    billboard.sort_unstable();
    let mut glass = GLASS_SAMPLERS;
    glass.sort_unstable();
    if samplers == billboard
        && [GLOW_U_SCALE, GLOW_V_SCALE, AMBIENT]
            .iter()
            .all(|&h| has(h))
    {
        // The compile with no `k` multiplies the glow by nothing.
        let k = if has(GLOW_K) { value(GLOW_K)? } else { 1.0 };
        return Some(Emissive {
            tint: [k; 3],
            offset: 0.0,
            scale: value(GLOW_V_SCALE)?,
            rate: 0.0,
            program: [PROGRAM_BILLBOARD, value(GLOW_U_SCALE)?],
        });
    }
    if samplers == glass && has(GLASS_REFLECTION_TINT) && has(GLASS_PROBE_TINT) {
        return Some(Emissive {
            tint: [1.0; 3],
            offset: 0.0,
            scale: 1.0,
            rate: 0.0,
            program: [PROGRAM_BW_GLASS, 0.0],
        });
    }
    None
}

/// Gives every billboard and every stand-glass material the glow entry of its
/// own program, in place of the shared additive one the emissive pass wrote.
pub(super) fn billboard_glass(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    textures: Textures<'_>,
    packed: &mut [u32],
    seconds: &TextureSlots,
    table: &mut Vec<Emissive>,
    report: &mut Report,
) {
    for (slot, material) in model.materials.iter().enumerate() {
        let Some(word) = packed.get_mut(slot) else {
            continue;
        };
        if !seconds.get(slot).is_some_and(Option::is_some) {
            continue;
        }
        // A lightmapped variant's second binding is the atlas, not the glow.
        if *word & slots::SECOND_IS_LIGHTMAP != 0 {
            continue;
        }
        let Some(variant) = variants.get(slot).copied().flatten() else {
            continue;
        };
        let Some(blob) = textures(&format!("/{}", material.name)) else {
            continue;
        };
        let Some(declared) = rcsmaterial::Declared::parse(&blob, variant.fragment.offset) else {
            continue;
        };
        let Some(layer) = classify(&declared, &material.parameters) else {
            continue;
        };
        // The glass is taken only where the generic pass flagged its accumulate; the
        // billboard's compile without `k` is not flagged and takes the bit below.
        let flagged = *word & slots::ADD_SECOND != 0;
        if !flagged && (layer.program[0] != PROGRAM_BILLBOARD || slots::material_index(*word) != 0)
        {
            continue;
        }
        let index = table.iter().position(|seen| *seen == layer).or_else(|| {
            (table.len() + 1 < EMISSIVE_LIMIT).then(|| {
                table.push(layer);
                table.len() - 1
            })
        });
        let Some(index) = index else {
            report.billboard_glass_unbound += 1;
            continue;
        };
        *word = (*word & slots::ROLE_MASK)
            | slots::ADD_SECOND
            | (u32::try_from(index + 1).unwrap_or(0) << slots::MATERIAL_SHIFT);
        if layer.program[0] == PROGRAM_BILLBOARD {
            report.billboard_multiplied += 1;
        } else {
            report.glass_bw_multiplied += 1;
        }
    }
}

/// A submesh's second coordinate set (`Uv2`) where a surface reads one: the
/// ice pool's and the Zone light cells' (`ice::second_uv`), or the stand
/// glass's, which reads `bwgradient` through it. `None` for every other
/// surface.
pub(super) fn second_uv(
    roles: u32,
    table: &[Emissive],
    mesh: &rcsmodel::Mesh,
    model_blob: &[u8],
    submesh: &rcsmodel::SubMesh,
    stride: usize,
) -> Option<Vec<[f32; 2]>> {
    super::ice::second_uv(roles, mesh, model_blob, submesh, stride).or_else(|| {
        let layer = slots::material_index(roles)
            .checked_sub(1)
            .and_then(|i| table.get(i as usize));
        let program = layer.map_or(0.0, |layer| layer.program[0]);
        // The stand glass's `bwgradient` and `sebenco_snow`'s mask (`super::snow_rock`).
        let glass = roles & slots::ADD_SECOND != 0 && program == PROGRAM_BW_GLASS;
        // A reflective window's glow mask (`super::window_glass`, `offset` 1).
        let window = layer.is_some_and(|layer| {
            layer.rate == crate::mesh::WINDOW_REFLECTION_RATE && layer.offset > 0.5
        });
        (glass || window || program == PROGRAM_SNOW)
            .then(|| mesh.second_texcoords(model_blob, submesh, stride).ok())
            .flatten()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declared(parameters: &[u32]) -> rcsmaterial::Declared {
        rcsmaterial::Declared {
            parameters: parameters.to_vec(),
            samplers: vec![(BILLBOARD_PICTURE, 0), (BILLBOARD_GLOW, 1)],
            ..Default::default()
        }
    }

    fn parameter(hash: u32, value: f32) -> rcsmodel::material::Parameter {
        rcsmodel::material::Parameter {
            hash,
            value: [value, 0.0, 0.0, 0.0],
            quads: 1,
        }
    }

    /// The compile Metropia and Chenghou Project ship declares the scales and the
    /// ambient and no `k`: the same billboard, its glow multiplied by nothing.
    #[test]
    fn a_billboard_compiled_without_k_is_the_billboard_with_k_one() {
        let scales = [parameter(GLOW_U_SCALE, 4.0), parameter(GLOW_V_SCALE, 2.0)];
        let layer = classify(&declared(&[GLOW_U_SCALE, GLOW_V_SCALE, AMBIENT]), &scales)
            .expect("the interface without k is still the billboard's");
        assert_eq!(layer.tint, [1.0; 3]);
        assert_eq!(layer.scale, 2.0);
        assert_eq!(layer.program, [PROGRAM_BILLBOARD, 4.0]);
    }

    /// Where the program declares `k`, the material's own value multiplies the glow,
    /// and a material that authors none is left unclassified.
    #[test]
    fn a_billboard_that_declares_k_takes_the_authored_value() {
        let with_k = declared(&[GLOW_U_SCALE, GLOW_V_SCALE, GLOW_K, AMBIENT]);
        let mut parameters = vec![parameter(GLOW_U_SCALE, 1.0), parameter(GLOW_V_SCALE, 1.0)];
        assert!(classify(&with_k, &parameters).is_none());
        parameters.push(parameter(GLOW_K, 0.5));
        let layer = classify(&with_k, &parameters).expect("the billboard");
        assert_eq!(layer.tint, [0.5; 3]);
    }

    /// Without the ambient the interface is another program's.
    #[test]
    fn two_samplers_alone_are_not_a_billboard() {
        assert!(classify(&declared(&[GLOW_U_SCALE, GLOW_V_SCALE]), &[]).is_none());
    }
}
