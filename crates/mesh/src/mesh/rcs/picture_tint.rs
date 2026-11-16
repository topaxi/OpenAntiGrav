//! The picture multiplier a `lambert_spec_mult` surface carries.
//!
//! `lambert_spec_mult.rcsmaterial`'s lit and prelit blocks (Tech de Ra
//! `@0x1cf0` block #2, `@0x1f50` block #3; `scripts/ps3-microcode.py fp-file`)
//! each read `TEX H, Texture1` then `MUL H.xyz, H, C` with `C` patched from
//! parameter `0x6d0178af`, and everything after that (light, specular, fog)
//! works on the product. The authored instances hold `(1, 1, 1)` on 37 of
//! Tech de Ra's 68 slots and a tint on the rest: `(0.078, 0.1296, 0.26)` on
//! `whitehexagon_colour_spec`, `(0.096, 0.1408, 0.32)` on
//! `bluewhite_band_colour_spec`, `0.8` and `1.8` greys. Without it the
//! hexagon panels behind the stand glass drew `(124, 133, 160)` where the
//! original draws `(55, 62, 87)`.
//!
//! Applied the way [`super::skin`] already applies a program constant to a
//! picture (`scale_alpha`): the product is baked into a picture of the same
//! size: each texel is decoded with the title's curve, multiplied by the
//! constant, and encoded again, which is what the program does (`TEX` then
//! `MUL`) and what the renderer's decode then reads back. A value
//! above 1 clips at a texel of 255, which the original's half-float register
//! does not; the `1.8` instances bind `startdarkouter01` and
//! `startdarkline01`, whose brightest texel is `(132, 125, 132)`, so nothing clips.
//! Of the 16 HD circuits only Tech de Ra carries the family.

use oag_gpu::formats::SceneStore;
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
pub(super) fn tinted(
    map: &ModelTexture,
    tint: [f32; 3],
    store: SceneStore,
) -> Option<ModelTexture> {
    let texels = map.to_rgba()?;
    let mut out = Vec::with_capacity(texels.len());
    for texel in texels.as_chunks::<4>().0 {
        for (byte, t) in texel[..3].iter().zip(tint) {
            let light = store.texel_decode(f32::from(*byte) / 255.0) * t.max(0.0);
            out.push((store.texel_encode(light) * 255.0).round() as u8);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn srgb_to_light(v: f32) -> f32 {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    }

    fn light_to_srgb(c: f32) -> f32 {
        if c <= 0.003_130_8 {
            c * 12.92
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        }
    }

    fn baked(byte: u8, tint: f32, store: SceneStore) -> u8 {
        let map = ModelTexture::rgba8("t".into(), 1, 1, vec![byte, byte, byte, 255], None);
        let out = tinted(&map, [tint; 3], store).expect("an RGBA picture tints");
        out.to_rgba().expect("RGBA out")[0]
    }

    /// On HD the renderer decodes the picture with the sRGB curve, so the bake is
    /// `encode(decode(texel) * tint)` with that curve. Fails with the old `byte *
    /// tint^(1/2.2)` gain, which lands a dark texel several levels off.
    #[test]
    fn hd_bakes_the_product_in_srgb_light() {
        for (byte, tint) in [(30u8, 0.25f32), (100, 0.2), (200, 0.5), (255, 1.0)] {
            let want =
                (light_to_srgb(srgb_to_light(f32::from(byte) / 255.0) * tint) * 255.0).round();
            let got = f32::from(baked(byte, tint, SceneStore::Encoded));
            assert!(
                (got - want).abs() <= 1.0,
                "{byte} x {tint}: {got} against {want}"
            );
        }
    }

    /// Whatever the title's curve, the shader's decode of the baked byte is the
    /// decoded texel times the tint, to within a level.
    #[test]
    fn the_baked_picture_decodes_to_texel_times_tint_on_every_title() {
        for store in [
            SceneStore::Encoded,
            SceneStore::Computed,
            SceneStore::Squared,
        ] {
            for (byte, tint) in [(60u8, 0.3f32), (128, 0.5), (220, 0.8)] {
                let out = baked(byte, tint, store);
                let want = store.texel_decode(f32::from(byte) / 255.0) * tint;
                let lo = store.texel_decode((f32::from(out) - 1.0) / 255.0);
                let hi = store.texel_decode((f32::from(out) + 1.0) / 255.0);
                assert!(
                    (lo..=hi).contains(&want),
                    "{store:?} {byte} x {tint}: {out}"
                );
            }
        }
    }
}
