//! A whole texture decoded to RGBA8, block-compressed formats through
//! `oag_texture::bcn`.

use oag_texture::bcn;
use oag_unity::assets::Assets;
use oag_unity::classes::TextureFormat;

use crate::error::{Error, Result};
use crate::track::ObjectRef;

/// A decoded image: `width * height` RGBA8 texels, **top row first**.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    /// Width in texels.
    pub width: u32,
    /// Height in texels.
    pub height: u32,
    /// RGBA8, row-major, top row first.
    pub rgba: Vec<u8>,
}

/// Decodes the base mip of the `Texture2D` at `at`. A 0 by 0 texture (Unity's
/// dynamic-font atlas) is an empty image; a format with no decoder is refused
/// by name.
pub fn decode(assets: &Assets, at: ObjectRef) -> Result<Image> {
    let (_, v) = assets
        .read(at.file, at.pptr)?
        .ok_or(Error::Dangling("a texture"))?;
    let t = assets.texture(&v)?;
    if t.is_empty() {
        return Ok(Image {
            width: 0,
            height: 0,
            rgba: Vec::new(),
        });
    }
    let (w, h) = (t.width as usize, t.height as usize);
    let rgba = match t.format {
        TextureFormat::Dxt1 => blocks::<8>(&t.data, w, h, bcn::dxt1)?,
        TextureFormat::Dxt3 => blocks::<16>(&t.data, w, h, bcn::dxt23)?,
        TextureFormat::Dxt5 => blocks::<16>(&t.data, w, h, bcn::dxt45)?,
        _ => t
            .base_level_rgba8()
            .ok_or(Error::Unsupported("a texture format with no decoder"))?,
    };
    Ok(Image {
        width: t.width,
        height: t.height,
        rgba,
    })
}

fn blocks<const N: usize>(
    data: &[u8],
    w: usize,
    h: usize,
    block: fn(&[u8; N]) -> [[u8; 4]; 16],
) -> Result<Vec<u8>> {
    let (bw, bh) = (w.div_ceil(4), h.div_ceil(4));
    let base = data
        .get(..bw * bh * N)
        .ok_or(Error::Unsupported("a texture shorter than its base level"))?;
    let mut out = vec![0u8; w * h * 4];
    for by in 0..bh {
        for bx in 0..bw {
            let at = (by * bw + bx) * N;
            let texels = block(base[at..at + N].try_into().expect("N bytes"));
            for (i, px) in texels.iter().enumerate() {
                let (x, y) = (bx * 4 + i % 4, by * 4 + i / 4);
                if x < w && y < h {
                    // Unity stores the bottom row first; the image is top row first.
                    let row = h - 1 - y;
                    out[(row * w + x) * 4..][..4].copy_from_slice(px);
                }
            }
        }
    }
    Ok(out)
}
