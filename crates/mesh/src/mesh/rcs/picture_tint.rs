//! The picture multiplier a `lambert_spec_mult` surface carries.
//!
//! `lambert_spec_mult.rcsmaterial`'s lit and prelit blocks (Tech de Ra
//! `@0x1cf0` block #2, `@0x1f50` block #3; `scripts/ps3-microcode.py fp-file`)
//! each read `TEX H, Texture1` then `MUL H.xyz, H, C` with `C` patched from
//! parameter `0x6d0178af`, and everything after that (light, specular, fog)
//! works on the product. The authored instances hold `(1, 1, 1)` on 37 of
//! Tech de Ra's 76 slots and a tint on the rest: `(0.078, 0.1296, 0.26)` on
//! `whitehexagon_colour_spec`, `(0.096, 0.1408, 0.32)` on
//! `bluewhite_band_colour_spec`, `0.8` and `1.8` greys. Without it the
//! hexagon panels behind the stand glass drew `(124, 133, 160)` where the
//! original draws `(55, 62, 87)`.
//!
//! Applied the way [`super::skin`] already applies a program constant to a
//! picture (`scale_alpha`): the product is baked into a picture of the same
//! size, with the constant encoded by `skin::raw_to_texel`'s rule so the
//! renderer's multiply by the light sees the value the program does. A value
//! above 1 clips at a texel of 255, which the original's half-float register
//! does not; no authored texel is near that on the one `1.8` instance read
//! (`startdarkouter01`, a dark texture).

use oag_rcs::rcsmodel;

use super::super::ModelTexture;

/// Families whose program multiplies `Texture1` by a parameter, with its hash.
/// Only `lambert_spec_mult` is read block by block; its `_scroll`, `_reflect`
/// and `_emissive` siblings carry the same parameter and hold `(1, 1, 1)` on
/// every Tech de Ra instance, and stay unlisted until one of them does not.
const PICTURE_TINT: &[(&str, u32)] = &[("lambert_spec_mult", 0x6d01_78af)];

/// The tint a material's picture is multiplied by, or `None` where it is `1`.
pub(super) fn tint_of(material: &rcsmodel::Material) -> Option<[f32; 3]> {
    let family = super::emissive::family(material);
    let hash = PICTURE_TINT
        .iter()
        .find_map(|&(name, hash)| (name == family).then_some(hash))?;
    let value = material.parameters.iter().find(|p| p.hash == hash)?.value;
    let tint = [value[0], value[1], value[2]];
    (tint != [1.0; 3]).then_some(tint)
}

/// `map` times `tint`, as a picture the renderer binds like any other.
pub(super) fn tinted(map: &ModelTexture, tint: [f32; 3]) -> Option<ModelTexture> {
    let texels = map.to_rgba()?;
    let gain = tint.map(|c| c.max(0.0).powf(1.0 / 2.2));
    let mut out = Vec::with_capacity(texels.len());
    for texel in texels.as_chunks::<4>().0 {
        for (byte, g) in texel[..3].iter().zip(gain) {
            out.push((f32::from(*byte) * g).round().min(255.0) as u8);
        }
        out.push(texel[3]);
    }
    Some(crate::mesh_render::offer_to_texture_sink(
        ModelTexture::rgba8(
            format!("picture tint {}", map.label),
            map.width,
            map.height,
            out,
            None,
        ),
    ))
}
