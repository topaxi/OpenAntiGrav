//! Arm's astcenc (through `ctt-astcenc`) as the encoder `oag_mesh::mesh_render::astc` is given:
//! 4x4 blocks, the linear LDR profile, the `medium` preset
//! (`docs/rendering/texture-compression.md`, "The encoder", "Choosing the preset").

use oag_mesh::mesh_render::astc::{BLOCK, level_len};

/// The astcenc quality preset, on its 0 to 100 scale (`medium` is 60), chosen by measurement
/// against `fast` and `thorough` on HD's Vineta K textures.
pub const QUALITY: f32 = ctt_astcenc::bindings::ASTCENC_PRE_MEDIUM;

/// The encoder as the cache is configured with it.
#[must_use]
pub fn encoder() -> oag_mesh::mesh_render::astc::Encoder {
    oag_mesh::mesh_render::astc::Encoder {
        name: "ctt-astcenc 0.6.1",
        quality: QUALITY,
        encode: encode_level,
    }
}

thread_local! {
    /// One context per thread that encodes (the cache's one encoder thread), made on first use.
    static CONTEXT: std::cell::RefCell<Option<Encoder>> = const { std::cell::RefCell::new(None) };
}

fn encode_level(width: u32, height: u32, rgba: &[u8]) -> Option<Vec<u8>> {
    CONTEXT.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_none() {
            *slot = Some(Encoder::new()?);
        }
        slot.as_mut()?.encode(width, height, rgba)
    })
}

/// An astcenc context at [`BLOCK`] and [`QUALITY`], linear LDR.
pub struct Encoder {
    context: ctt_astcenc::Context,
}

impl std::fmt::Debug for Encoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Encoder")
    }
}

impl Encoder {
    /// A single-threaded context; `None` if astcenc refuses the configuration.
    #[must_use]
    pub fn new() -> Option<Self> {
        Self::with_quality(QUALITY)
    }

    /// A context at another preset, for measuring one against [`QUALITY`].
    #[must_use]
    pub fn with_quality(quality: f32) -> Option<Self> {
        let config = ctt_astcenc::config_init(
            ctt_astcenc::Profile::Ldr,
            BLOCK,
            BLOCK,
            1,
            ctt_astcenc::Preset::Custom(quality),
            ctt_astcenc::Flags::empty(),
        )
        .map_err(|why| log::warn!("astc: {why}"))
        .ok()?;
        let context = ctt_astcenc::Context::new(&config)
            .map_err(|why| log::warn!("astc: {why}"))
            .ok()?;
        Some(Self { context })
    }

    /// One level of RGBA8 texels as ASTC blocks.
    pub fn encode(&mut self, width: u32, height: u32, rgba: &[u8]) -> Option<Vec<u8>> {
        if rgba.len() != (width * height * 4) as usize {
            return None;
        }
        let mut texels = rgba.to_vec();
        let mut planes = [texels.as_mut_ptr().cast::<std::ffi::c_void>()];
        let mut image = ctt_astcenc::bindings::astcenc_image {
            dim_x: width,
            dim_y: height,
            dim_z: 1,
            data_type: ctt_astcenc::bindings::astcenc_type_ASTCENC_TYPE_U8,
            data: planes.as_mut_ptr(),
        };
        let mut out = vec![0u8; level_len(width, height)];
        let encoded = self
            .context
            .compress(&mut image, ctt_astcenc::Swizzle::IDENTITY, &mut out);
        self.context.compress_reset().ok()?;
        encoded.map_err(|why| log::warn!("astc: {why}")).ok()?;
        Some(out)
    }

    /// ASTC blocks of one level back to RGBA8, as a GPU would sample them.
    pub fn decode(&mut self, width: u32, height: u32, blocks: &[u8]) -> Option<Vec<u8>> {
        let mut texels = vec![0u8; (width * height * 4) as usize];
        let mut planes = [texels.as_mut_ptr().cast::<std::ffi::c_void>()];
        let mut image = ctt_astcenc::bindings::astcenc_image {
            dim_x: width,
            dim_y: height,
            dim_z: 1,
            data_type: ctt_astcenc::bindings::astcenc_type_ASTCENC_TYPE_U8,
            data: planes.as_mut_ptr(),
        };
        let decoded = self
            .context
            .decompress(blocks, &mut image, ctt_astcenc::Swizzle::IDENTITY);
        self.context.decompress_reset().ok()?;
        decoded.ok()?;
        Some(texels)
    }
}

/// How far a decoded encode is from the texels it was encoded from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Error {
    /// Peak signal-to-noise ratio over every channel, in dB (infinite for none).
    pub psnr: f64,
    /// The largest difference of one channel, in levels of 255.
    pub max: u8,
    /// The share of texels with any channel more than 2 levels off.
    pub over_two: f64,
}

/// The [`Error`] of `decoded` against `source`, both RGBA8 of the same size.
#[must_use]
pub fn error(source: &[u8], decoded: &[u8]) -> Error {
    let mut squared = 0u64;
    let mut max = 0u8;
    let mut over = 0usize;
    for (a, b) in source
        .as_chunks::<4>()
        .0
        .iter()
        .zip(decoded.as_chunks::<4>().0)
    {
        let mut texel_max = 0u8;
        for (x, y) in a.iter().zip(b) {
            let d = x.abs_diff(*y);
            squared += u64::from(d) * u64::from(d);
            texel_max = texel_max.max(d);
        }
        max = max.max(texel_max);
        over += usize::from(texel_max > 2);
    }
    let texels = (source.len() / 4).max(1);
    let mse = squared as f64 / (texels * 4) as f64;
    Error {
        psnr: if mse == 0.0 {
            f64::INFINITY
        } else {
            10.0 * (255.0f64 * 255.0 / mse).log10()
        },
        max,
        over_two: over as f64 / texels as f64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An RGBA8 picture ramping red across and green down, opaque.
    fn gradient(width: u32, height: u32) -> Vec<u8> {
        let mut texels = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                texels.extend_from_slice(&[
                    (x * 255 / width.max(2).saturating_sub(1)) as u8,
                    (y * 255 / height.max(2).saturating_sub(1)) as u8,
                    128,
                    255,
                ]);
            }
        }
        texels
    }

    #[test]
    fn an_encode_decodes_back_close_to_its_source() {
        let mut encoder = Encoder::new().expect("astcenc configures");
        let source = gradient(16, 16);
        let blocks = encoder.encode(16, 16, &source).expect("encodes");
        assert_eq!(blocks.len(), level_len(16, 16));
        let back = encoder.decode(16, 16, &blocks).expect("decodes");
        let error = error(&source, &back);
        // Measured, not a byte hash: astcenc's output is ISA-dependent
        // (`ASTCENC_NO_INVARIANCE`), its quality is not.
        assert!(
            error.psnr > 40.0,
            "a smooth gradient survives 4x4: {error:?}"
        );

        let flat = vec![90u8; 8 * 8 * 4];
        let blocks = encoder.encode(8, 8, &flat).expect("encodes");
        assert_eq!(
            encoder.decode(8, 8, &blocks).expect("decodes"),
            flat,
            "a constant picture is exact"
        );

        // A level smaller than a block still fills one.
        assert_eq!(
            encode_level(2, 2, &gradient(2, 2)).map(|b| b.len()),
            Some(16)
        );
    }
}
