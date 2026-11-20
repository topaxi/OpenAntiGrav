//! The formats of the offscreen targets the scene pass draws into and the
//! post chain reads.
//!
//! Defined here rather than beside the pass that owns each, because the other
//! side of the pair needs the value in real code (`oag-mesh` gates its bloom
//! output on [`SCENE_FORMAT`]; `oag-post`'s FSR 3 readback samples
//! [`VELOCITY_FORMAT`]) and neither crate may depend on the other.

/// The screen-space velocity target's format. `Rg16Float` is enough for a uv
/// delta and is multisample-renderable at 4x - pinned by
/// `rg16float_is_multisample_renderable_at_4x_on_this_adapter` in
/// `oag_post::motion_blur`'s tests, with `Rgba16Float` as the documented
/// fallback should an adapter ever fail that pin.
pub const VELOCITY_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg16Float;

/// The offset target blend class 8 draws into and the Omega composite reads.
///
/// **Chosen, not measured.** The executable's target is `R8G8_SNORM` (Gnm word
/// `0x22c103`, `docs/ghidra/functions/ps4-omega-eu/heat-haze.md`, confidence
/// 75) and WebGPU cannot render to `rg8snorm`, so the offsets are summed in
/// half floats and the reader clamps what it reads to the `[-1, 1]` an SNORM
/// write would have left.
pub const DISTORTION_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg16Float;

/// The float scene target's format: linear, and wide enough for the bloom gate to
/// see luminance above 1.0. HD's scene is in [`SCENE_FORMAT_8BIT`] by default. Drawing into it is what tells every mesh pipeline
/// to output linear light - see `oag_mesh::mesh_render::is_linear_target`.
pub const SCENE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// The 8-bit scene target's format, as the race draws into it: Wipeout HD's own
/// `A8R8G8B8` (`docs/rendering/hd-track-tone.md`, "HD's blends combine encoded
/// values"). Only a scene that stores encoded values ([`SceneStore::Encoded`]) is
/// drawn into it: every draw writes the encoded byte, a blend combines encoded
/// bytes and the attachment clamps each to `[0, 1]`, as the console's does. The
/// format alone says nothing about colour space here (every gamma target is
/// `Rgba8Unorm` too), so the store goes with it: see `is_scene_target`.
pub const SCENE_FORMAT_8BIT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// The view the post chain reads the 8-bit scene through: the texture unit decodes
/// each texel with the sRGB curve before it filters, as the console's units do on a
/// fetch with gamma nibble 7, so a bilinear tap averages light in one hardware tap.
pub const SCENE_READ_8BIT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// The format a race's scene target is drawn in: [`SCENE_FORMAT_8BIT`] when the scene
/// stores encoded values and the player keeps the original's precision, else
/// [`SCENE_FORMAT`].
#[must_use]
pub fn scene_format(store: SceneStore, eight_bit: bool) -> wgpu::TextureFormat {
    if eight_bit && store == SceneStore::Encoded {
        SCENE_FORMAT_8BIT
    } else {
        SCENE_FORMAT
    }
}

/// Whether a pipeline drawing into `format` with `store` draws into a race's scene
/// target, and so computes in linear light: [`SCENE_FORMAT`] whatever it stores, or
/// [`SCENE_FORMAT_8BIT`] holding encoded values.
#[must_use]
pub fn is_scene_target(format: wgpu::TextureFormat, store: SceneStore) -> bool {
    format == SCENE_FORMAT || (format == SCENE_FORMAT_8BIT && store == SceneStore::Encoded)
}

/// What a target stores of a draw's linear-light output: a [`SCENE_FORMAT`] one keeps
/// it or its encoding, and a card keeps it rounded to its few bits.
///
/// The format says the draws compute in linear light; this says whether the
/// target then keeps that value or its display encoding. It matters only to a
/// blended draw, which combines with what is stored: Wipeout HD's console packs
/// every scene draw to sRGB before the blend unit sees it (`SET_SHADER_PACKER`
/// 1 into an `A8R8G8B8` target, `docs/rendering/hd-track-tone.md`, "HD's blends
/// combine encoded values"), so an add or a mix there runs on encoded values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SceneStore {
    /// The value as computed: Omega's float scene, and any gamma target.
    #[default]
    Computed,
    /// Clamped to `[0, 1]` and encoded with the piecewise sRGB curve, the
    /// console's packer, which the HD chain decodes with where it reads the
    /// scene. A draw into it also decodes its gamma-authored texels with that
    /// curve, as the console's texture units do (`docs/rendering/hd-track-tone.md`,
    /// "The texture unit's decode is the sRGB curve").
    Encoded,
    /// The value as computed, in the gamma-2 space Wipeout 2048's programs
    /// light in: a draw decodes a gamma-authored colour by squaring it, not with
    /// the `2.2` power, and the race composite's square root is the encode
    /// (`docs/ghidra/functions/vita-2048-eu-v104/exposure.md`).
    Squared,
    /// The value as computed, with a gamma-authored texel decoded by the piecewise sRGB curve,
    /// as the texture unit decodes an sRGB-typed texture: Omega's float scene, whose circuit
    /// pictures are `ChannelType::Srgb` GNF (`docs/rendering/hd-track-tone.md`, "Omega's texels
    /// decode with the sRGB curve").
    SrgbTexels,
    /// A 5-6-5 colour card, not a scene: Wipeout HD's 512 x 256 advert and gantry
    /// cards hold linear light rounded to 31, 63 and 31 levels
    /// (`docs/rendering/start-gantry.md`, "The card is 5-6-5 in linear light"). Drawn
    /// into a gamma target, which keeps the rounded value encoded.
    Packed565,
    /// A 5-6-5 colour card holding gamma-encoded values, truncated to its bits the way the
    /// PSP's GE writes a 16-bit target: Wipeout Pulse's 128 x 128 advert and gantry card
    /// (`docs/rendering/start-gantry.md`, "Pulse's card is 5-6-5, truncated").
    Packed565Gamma,
}

impl SceneStore {
    /// A gamma-authored texel value on `0..=1` as the light the scene's shaders
    /// compute with: the CPU twin of `target.wesl`'s `decode`, which is the
    /// piecewise sRGB curve when the target stores encoded values, the square
    /// in 2048's gamma-2 space, and the `2.2` power everywhere else.
    #[must_use]
    pub fn texel_decode(self, v: f32) -> f32 {
        let v = v.clamp(0.0, 1.0);
        match self {
            Self::Encoded | Self::SrgbTexels => {
                if v <= 0.04045 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            }
            Self::Squared => v * v,
            _ => v.powf(2.2),
        }
    }

    /// The inverse of [`Self::texel_decode`]: the texel value a program-computed
    /// colour has to be baked as so that the shader's `decode` reads it back as
    /// that colour. A bake that used another curve than the shader reads
    /// tints the darks, which is where the curves differ most.
    #[must_use]
    pub fn texel_encode(self, light: f32) -> f32 {
        let c = light.clamp(0.0, 1.0);
        // White stays exactly 1.0: the sRGB arm computes 0.99999994 there, and a neutral tint must not
        // read as a tint.
        if c >= 1.0 {
            return 1.0;
        }
        self.encode_curve(c)
    }

    /// [`Self::texel_encode`] for a gain above 1: the factor a gamma-domain multiply of the
    /// texel needs so the light it decodes to is about `gain` times the texel's own. The curve
    /// is not a power, so the match is exact at a middle texel and a few percent off in the
    /// darkest ones (1.51 reads 1.2 % high at a texel of 0.5, 5 % low at 0.2).
    #[must_use]
    pub fn texel_gain(self, gain: f32) -> f32 {
        let g = gain.max(0.0);
        if g <= 1.0 {
            self.texel_encode(g)
        } else {
            self.encode_curve(g)
        }
    }

    fn encode_curve(self, c: f32) -> f32 {
        match self {
            Self::Encoded | Self::SrgbTexels => {
                if c <= 0.003_130_8 {
                    c * 12.92
                } else {
                    1.055 * c.powf(1.0 / 2.4) - 0.055
                }
            }
            Self::Squared => c.sqrt(),
            _ => c.powf(1.0 / 2.2),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SceneStore;

    const STORES: [SceneStore; 4] = [
        SceneStore::Encoded,
        SceneStore::Computed,
        SceneStore::Squared,
        SceneStore::SrgbTexels,
    ];

    /// The piecewise sRGB EOTF written out here independently of the code under test.
    fn srgb_to_light(v: f32) -> f32 {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    }

    /// A gain above 1 encoded for the gamma-domain multiply decodes back to about the gain at a
    /// middle texel, never above it, and a gain of 1 or less is the clamped encode.
    #[test]
    fn a_texel_gain_decodes_to_about_the_gain() {
        for store in [SceneStore::Encoded, SceneStore::SrgbTexels] {
            for gain in [1.51_f32, 2.264, 3.0] {
                let factor = store.texel_gain(gain);
                assert!(factor > 1.0, "{gain}: {factor}");
                let light = store.texel_decode((0.5 * factor).min(1.0)) / store.texel_decode(0.5);
                assert!((light / gain - 1.0).abs() < 0.08, "{gain}: {light}");
                assert!(light <= gain * 1.0001, "{gain}: {light}");
            }
            assert_eq!(store.texel_gain(1.0), 1.0);
            assert_eq!(store.texel_gain(0.5), store.texel_encode(0.5));
        }
    }

    /// A texel byte decoded and encoded again comes back within one level on
    /// every title's curve. Measured in the byte domain: the decode's slope near
    /// white turns half a level of rounding into more than 1/255 of light.
    #[test]
    fn a_texel_round_trips_on_every_titles_curve() {
        for store in STORES {
            for byte in 0..=255u32 {
                let light = store.texel_decode(byte as f32 / 255.0);
                let back = (store.texel_encode(light) * 255.0).round();
                assert!(
                    (back - byte as f32).abs() <= 1.0,
                    "{store:?}: {byte} -> {back}"
                );
            }
        }
    }

    /// HD decodes with the sRGB curve, so the `2.2` power re-encode the CPU bakes used
    /// to apply lands several levels off in the darks. Fails with the power encode.
    #[test]
    fn hds_encode_is_the_srgb_curve_not_the_power() {
        let light = srgb_to_light(10.0 / 255.0);
        let power = (light.powf(1.0 / 2.2) * 255.0).round();
        let encoded = (SceneStore::Encoded.texel_encode(light) * 255.0).round();
        assert!((power - 10.0).abs() > 4.0, "the power lands at {power}");
        assert_eq!(encoded, 10.0);
    }
}
