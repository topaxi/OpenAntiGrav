//! ZIM: PPSSPP's own picture format, what a later texture pack ships beside (or instead of) PNG.
//!
//! Layout (`docs/formats/ppsspp-texture-pack.md`, "ZIM images"), little-endian:
//!
//! ```text
//! +0x00  4    "ZIMG"
//! +0x04  u32  width
//! +0x08  u32  height
//! +0x0c  u32  flags
//! +0x10  ...  the levels' pixels back to back: raw, zlib or zstd as a whole
//! ```
//!
//! Only level 0 is read: a pack's picture is the replacement of one texture, and the renderer
//! makes its own levels. Reading stops at that level's bytes, so a stream that holds the whole
//! chain is never expanded past the first.

use std::io::Read;

use crate::replacer::Image;

/// The four bytes a `.zim` starts with.
pub const MAGIC: &[u8; 4] = b"ZIMG";

const HEADER_LEN: usize = 16;
const FORMAT_MASK: u32 = 0x0f;
const ZLIB: u32 = 0x100;
const ZSTD: u32 = 0x1000;

/// The longest side a picture may declare: beyond it the allocation alone is a decompression
/// bomb (16384 x 16384 x 4 is 1 GiB).
const MAX_SIDE: u32 = 16384;

/// Why a `.zim` is not decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Not "ZIMG", or shorter than its header.
    NotZim,
    /// A side of 0 or beyond [`MAX_SIDE`].
    Size(u32, u32),
    /// The pixel format in the flags' low nibble is none of 0 (RGBA8888), 1 (RGBA4444) and
    /// 2 (RGB565).
    Format(u32),
    /// Both the zlib and the zstd flag are set: the loader would take the first it tests.
    BothCodecs,
    /// The data ends before level 0 does, or the compressed stream will not decode.
    Data(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotZim => f.write_str("not a ZIM picture"),
            Self::Size(w, h) => write!(f, "a {w}x{h} picture is outside 1..={MAX_SIDE} a side"),
            Self::Format(n) => write!(f, "pixel format {n} is not 0, 1 or 2"),
            Self::BothCodecs => f.write_str("flagged both zlib and zstd"),
            Self::Data(why) => write!(f, "level 0 does not decode: {why}"),
        }
    }
}

impl std::error::Error for Error {}

/// Whether `bytes` begin a `.zim`.
#[must_use]
pub fn is_zim(bytes: &[u8]) -> bool {
    bytes.starts_with(MAGIC)
}

/// Level 0 of a `.zim` as RGBA8888, top row first.
///
/// # Errors
///
/// [`Error`] for a header, size, format or stream this reader refuses.
pub fn decode(bytes: &[u8]) -> Result<Image, Error> {
    if !is_zim(bytes) || bytes.len() < HEADER_LEN {
        return Err(Error::NotZim);
    }
    let word =
        |at: usize| u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
    let (width, height, flags) = (word(4), word(8), word(12));
    if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
        return Err(Error::Size(width, height));
    }
    let format = flags & FORMAT_MASK;
    let bytes_per_pixel: usize = match format {
        0 => 4,
        1 | 2 => 2,
        other => return Err(Error::Format(other)),
    };
    if flags & ZLIB != 0 && flags & ZSTD != 0 {
        return Err(Error::BothCodecs);
    }
    let wanted = width as usize * height as usize * bytes_per_pixel;
    let body = &bytes[HEADER_LEN..];
    let mut level0 = Vec::with_capacity(wanted);
    let read = if flags & ZLIB != 0 {
        flate2::read::ZlibDecoder::new(body)
            .take(wanted as u64)
            .read_to_end(&mut level0)
            .map_err(|why| why.to_string())
    } else if flags & ZSTD != 0 {
        ruzstd::decoding::StreamingDecoder::new(body)
            .map_err(|why| why.to_string())
            .and_then(|decoder| {
                decoder
                    .take(wanted as u64)
                    .read_to_end(&mut level0)
                    .map_err(|why| why.to_string())
            })
    } else {
        level0.extend_from_slice(&body[..wanted.min(body.len())]);
        Ok(level0.len())
    };
    read.map_err(Error::Data)?;
    if level0.len() < wanted {
        return Err(Error::Data(format!("{} of {wanted} bytes", level0.len())));
    }
    let rgba = match format {
        0 => level0,
        1 => level0
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|pixel| rgba4444(u16::from_le_bytes([pixel[0], pixel[1]])))
            .collect(),
        _ => level0
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|pixel| rgb565(u16::from_le_bytes([pixel[0], pixel[1]])))
            .collect(),
    };
    Ok(Image {
        width,
        height,
        rgba,
    })
}

/// A 4-bit channel widened to 8 bits, 0xf to 0xff.
fn widen4(nibble: u16) -> u8 {
    (nibble as u8) * 17
}

/// RGBA4444: red in the high nibble.
fn rgba4444(texel: u16) -> [u8; 4] {
    [
        widen4(texel >> 12),
        widen4((texel >> 8) & 0xf),
        widen4((texel >> 4) & 0xf),
        widen4(texel & 0xf),
    ]
}

/// RGB565: red in the high five bits; every texel opaque.
fn rgb565(texel: u16) -> [u8; 4] {
    let wide = |value: u16, bits: u32| {
        let value = u32::from(value);
        ((value << (8 - bits)) | (value >> (2 * bits - 8))) as u8
    };
    [
        wide(texel >> 11, 5),
        wide((texel >> 5) & 0x3f, 6),
        wide(texel & 0x1f, 5),
        255,
    ]
}

#[cfg(test)]
mod tests;
