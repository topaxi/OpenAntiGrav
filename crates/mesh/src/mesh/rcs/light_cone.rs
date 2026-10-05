//! HD's light cone: `dc_lightcone.rcsmaterial`, the soft shafts that fall from
//! a circuit's lamps, drawn by the combine the disc's own fragment program
//! computes rather than as a grey picture.
//!
//! # What the program does
//!
//! Read off the resolved fogged variant (Talon's Junction `@0x1550`, Amphiseum
//! `@0x1520`; `scripts/ps3-microcode.py fp-file`). With `N` the vertex normal
//! (carried in the `w` of three interpolators), `V` the vector to the eye,
//! `noise` the `Texture1` tap at the surface's own UV and `ramp` the unit-1
//! tap:
//!
//! ```text
//! cos    = (N . V) / sqrt(|N|^2 |V|^2)          DP3 / DP3 / DIVSQ
//! colour = fog-lerp(noise.x * K)                MAD R2.xyz, H2.xxxx, K, -fog
//! alpha  = noise.x * s * ramp(cos).x            MUL H1.w, H2.xxxx, s ; MUL H0.w, H1, H2.xxxx
//! ```
//!
//! The picture the renderer drew before this module was `noise` as colour and
//! the texture's own alpha channel as coverage, which on the disc's grey
//! single-channel noise is a **fully opaque grey wedge**: the original's shafts
//! are translucent, and on Talon's Junction `K` is `100` (parameter
//! `0x60eaf40d`), so every texel above 1 % saturates to white and the shaft is
//! a white glow whose edges the ramp fades. Amphiseum's variant has no `K`
//! multiply (`K` = 1) and `s` = 0.496.
//!
//! # The ramp tap is predicated on one of the two variants
//!
//! Talon's Junction's `@0x1550` runs `FENCT R63, R0, R0` (`R0` = `f[TC1]`)
//! before `TEX H2.x, R2.wwww unit1 [NE(wwww)]`; Amphiseum's `@0x1520` has the
//! same tap with no predicate. `H2.x` still holds the noise from the first
//! tap, so where the predicated `TEX` does not execute the alpha is
//! `s * noise * noise`. `f[TC1].w` is the vertex normal's `y` and every
//! authored cone's normal is the constant `(0, 0, 1)` (raw vertex bytes
//! `7f 80 00 00` in a `CMP` attribute, `hd_unlit_probe`), so on the gated
//! variant the tap is skipped everywhere. **That the condition register is
//! set from `R0` by `FENCT` is a hypothesis**, not documented behaviour: the
//! opcode writes no register (every one of 59,256 uses names register 63,
//! `docs/formats/rcsmaterial.md`). Confidence 60: the alternative reading
//! (ramp always taken) renders the shafts as an opaque white wall where the
//! original's are translucent streaks over the blue tunnel, one matched
//! frame (`talons-matched/03`, upper left), and the skipped-ramp reading
//! reproduces that look.
//!
//! # Population
//!
//! `crates/render/examples/hd_light_cone_census.rs` over the four PS3
//! archives: exactly **two** materials resolve to this shape, both named
//! `dc_lightcone` (Talon's Junction slot 440, Amphiseum slot 610). Every
//! other material that declares `{Texture1, 0xa2d555b9}` reads a *mixed*
//! colour lane and is refused by the colour-lane test below.
//!
//! # What is approximated
//!
//! `ramp` is sampled at `dot(V, N)` with the interpolated normal, exactly the
//! way the glass sheen already does; the program normalises both vectors in
//! the fragment stage and so does the shader. The fog lerp is the fog the
//! shader applies after every material (`fogged`), not a second one.

use oag_rcs::rcsmaterial::fragment::{Program, Texel};
use oag_rcs::{rcsmaterial, rcsmodel};

use crate::mesh::{Emissive, TextureSlots, slots};

use super::emissive::EMISSIVE_LIMIT;
use super::skin::decode_texture;
use super::{Report, Textures};

/// `~crc32("Texture1")`: the noise, at the unit the program samples for colour.
const TEXTURE1: u32 = 0x3bdc_0403;
/// The cone's facing ramp (`dc_gradient_e.gtf`).
const RAMP: u32 = 0xa2d5_55b9;
/// `K`: what the noise is multiplied by on the way to the colour. Absent on
/// Amphiseum's variant, which means one.
const INTENSITY: u32 = 0x60ea_f40d;
/// The condition field of an unconditional instruction (`TR` on every lane).
const UNCONDITIONAL: u16 = 0x727;
/// `s`: what the noise is multiplied by on the way to the alpha.
const SCALE: u32 = 0x7611_a2d8;

/// Whether a resolved program is the cone's, by what it declares and traces:
/// exactly the noise and the ramp, the alpha scale, and a colour lane that is
/// one unit's red - the noise's - while the alpha mixes both units.
fn is_cone(declared: &rcsmaterial::Declared, program: &Program) -> bool {
    let mut hashes: Vec<u32> = declared.samplers.iter().map(|s| s.0).collect();
    hashes.sort_unstable();
    if hashes != [TEXTURE1.min(RAMP), TEXTURE1.max(RAMP)] || !declared.parameters.contains(&SCALE) {
        return false;
    }
    let Some(noise_unit) = declared
        .samplers
        .iter()
        .find(|&&(h, _)| h == TEXTURE1)
        .map(|&(_, u)| u)
    else {
        return false;
    };
    let texels = program.output_texels();
    let colour_is_noise = texels[..3].iter().all(|t| {
        matches!(
            *t,
            Texel::Unit { unit, channel: Some(0) } if u32::from(unit) == noise_unit
        )
    });
    colour_is_noise && texels[3] == Texel::Mixed
}

/// Binds every light cone's ramp and noise and writes [`slots::LIGHT_CONE`]
/// with its glow-table index into `packed`.
pub(super) fn light_cone(
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
        let Some(declared) = rcsmaterial::Declared::parse(&blob, variant.fragment.offset) else {
            continue;
        };
        let Some(program) = Program::parse(&blob, variant.fragment.offset) else {
            continue;
        };
        if !is_cone(&declared, &program) {
            continue;
        }
        let path_of = |hash: u32| {
            material
                .samplers
                .iter()
                .find(|(h, p)| *h == hash && p.is_some())
                .and_then(|(_, p)| p.clone())
        };
        let value_of = |hash: u32| material.parameters.iter().find(|p| p.hash == hash);
        let (Some(noise_path), Some(ramp_path), Some(scale)) =
            (path_of(TEXTURE1), path_of(RAMP), value_of(SCALE))
        else {
            continue;
        };
        let mut decode = |path: &str| {
            textures(&format!("/{path}"))
                .and_then(|bytes| decode_texture(path, &bytes))
                .map(std::sync::Arc::new)
        };
        let (Some(noise), Some(ramp)) = (decode(&noise_path), decode(&ramp_path)) else {
            continue;
        };
        let intensity =
            value_of(INTENSITY).map_or([1.0; 3], |p| [p.value[0], p.value[1], p.value[2]]);
        // The ramp tap is predicated on Talon's Junction's variant and not on
        // Amphiseum's: `rate` carries which, `1.0` for gated.
        let ramp_unit = declared
            .samplers
            .iter()
            .find(|&&(h, _)| h == RAMP)
            .map(|&(_, u)| u);
        let gated = program.instructions.iter().any(|i| {
            i.name() == Some("TEX")
                && Some(u32::from(i.unit)) == ramp_unit
                && i.cond != UNCONDITIONAL
        });
        let layer = Emissive {
            tint: intensity,
            offset: 0.0,
            scale: scale.value[0],
            rate: f32::from(u8::from(gated)),
        };
        let index = table.iter().position(|seen| *seen == layer).or_else(|| {
            (table.len() + 1 < EMISSIVE_LIMIT).then(|| {
                table.push(layer);
                table.len() - 1
            })
        });
        let Some(index) = index else {
            continue;
        };
        skins[slot] = Some(ramp);
        seconds[slot] = Some(noise);
        if let Some(word) = packed.get_mut(slot) {
            *word &= slots::ROLE_MASK & !slots::ADD_SECOND;
            *word |= slots::LIGHT_CONE;
            *word |= u32::try_from(index + 1).unwrap_or(0) << slots::MATERIAL_SHIFT;
        }
        report.light_cone_bound += 1;
    }
}
