//! The GE's block swizzle, and the flag that says a blob is in it.
//!
//! Byte layout, not pixel format: where a texel *is*, never its colour. It sits
//! beside [`crate::byte_order`] rather than in `oag-texture` because `.mip`
//! textures, `.fnt` glyph atlases and `.vex` embedded textures all store texels
//! this way and are decoded in three crates. See
//! [ADR-0050](../../../docs/architecture/adr/0050-format-crates-split-by-format-family.md).

/// Bit 0 of `+0x07`: the pixels are stored swizzled for the GE.
///
/// `Texture_BindEmbeddedData` in the PSP executable copies this bit into the
/// texture node's own flags, and swizzles at load **only** when it is clear.
/// See `docs/formats/psp-texture.md`.
pub const FLAG_SWIZZLED: u8 = 1;

pub const SWIZZLE_BLOCK_BYTES: usize = 16;

pub const SWIZZLE_BLOCK_ROWS: usize = 8;

/// Undoes the GE's 16-byte by 8-row block swizzle.
///
/// `row_bytes` is the image's width in bytes. This is the exact inverse of
/// `Texture_SwizzleForGe` (`0x08926da8`), which walks block rows, then block
/// columns, then the 8 rows within a block, copying 16 bytes at a time out of a
/// linear image.
///
/// A `row_bytes` of 16 or a `height` under 8 makes the swizzle the identity,
/// which is why the disc's one 4bpp texture cannot tell the two readings apart.
#[must_use]
pub fn unswizzle(src: &[u8], row_bytes: usize, height: usize) -> Vec<u8> {
    let mut dst = vec![0u8; src.len()];
    if !row_bytes.is_multiple_of(SWIZZLE_BLOCK_BYTES) || row_bytes == 0 {
        dst.copy_from_slice(src);
        return dst;
    }
    let mut read = 0;
    for block_row in 0..height / SWIZZLE_BLOCK_ROWS {
        for block_col in 0..row_bytes / SWIZZLE_BLOCK_BYTES {
            for row in 0..SWIZZLE_BLOCK_ROWS {
                let at = (block_row * SWIZZLE_BLOCK_ROWS + row) * row_bytes
                    + block_col * SWIZZLE_BLOCK_BYTES;
                let (Some(chunk), true) = (
                    src.get(read..read + SWIZZLE_BLOCK_BYTES),
                    at + SWIZZLE_BLOCK_BYTES <= dst.len(),
                ) else {
                    return dst;
                };
                dst[at..at + SWIZZLE_BLOCK_BYTES].copy_from_slice(chunk);
                read += SWIZZLE_BLOCK_BYTES;
            }
        }
    }
    // A height that is not a multiple of 8 leaves a tail the block walk never
    // reaches; copy it through rather than leaving it black.
    let done = (height / SWIZZLE_BLOCK_ROWS) * SWIZZLE_BLOCK_ROWS * row_bytes;
    if done < src.len() {
        dst[done..].copy_from_slice(&src[done..]);
    }
    dst
}

/// The GE's 16-byte by 8-row block swizzle: `Texture_SwizzleForGe`, the inverse
/// of [`unswizzle`].
///
/// `row_bytes` is the image's row stride in bytes. A row that is not a whole
/// number of 16-byte blocks, or a `height` under 8, is left as it is, exactly as
/// [`unswizzle`] does; rows past the last whole block row are copied through.
#[must_use]
pub fn swizzle(src: &[u8], row_bytes: usize, height: usize) -> Vec<u8> {
    let mut dst = vec![0u8; src.len()];
    if !row_bytes.is_multiple_of(SWIZZLE_BLOCK_BYTES) || row_bytes == 0 {
        dst.copy_from_slice(src);
        return dst;
    }
    let mut write = 0;
    for block_row in 0..height / SWIZZLE_BLOCK_ROWS {
        for block_col in 0..row_bytes / SWIZZLE_BLOCK_BYTES {
            for row in 0..SWIZZLE_BLOCK_ROWS {
                let at = (block_row * SWIZZLE_BLOCK_ROWS + row) * row_bytes
                    + block_col * SWIZZLE_BLOCK_BYTES;
                let (Some(chunk), true) = (
                    src.get(at..at + SWIZZLE_BLOCK_BYTES),
                    write + SWIZZLE_BLOCK_BYTES <= dst.len(),
                ) else {
                    return dst;
                };
                dst[write..write + SWIZZLE_BLOCK_BYTES].copy_from_slice(chunk);
                write += SWIZZLE_BLOCK_BYTES;
            }
        }
    }
    let done = (height / SWIZZLE_BLOCK_ROWS) * SWIZZLE_BLOCK_ROWS * row_bytes;
    if done < src.len() {
        dst[done..].copy_from_slice(&src[done..]);
    }
    dst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swizzle_inverts_unswizzle() {
        for (row_bytes, height) in [(16, 8), (32, 16), (64, 24), (48, 20), (8, 8), (128, 4)] {
            let linear: Vec<u8> = (0..row_bytes * height)
                .map(|i| (i * 31 % 251) as u8)
                .collect();
            let swizzled = swizzle(&linear, row_bytes, height);
            assert_eq!(unswizzle(&swizzled, row_bytes, height), linear);
            assert_eq!(
                swizzle(&unswizzle(&linear, row_bytes, height), row_bytes, height),
                linear
            );
        }
    }

    #[test]
    fn a_block_is_eight_rows_of_sixteen_bytes() {
        let row_bytes = 32;
        let linear: Vec<u8> = (0..row_bytes * 8).map(|i| i as u8).collect();
        let swizzled = swizzle(&linear, row_bytes, 8);
        assert_eq!(&swizzled[..16], &linear[..16]);
        assert_eq!(&swizzled[16..32], &linear[32..48]);
        assert_eq!(&swizzled[128..144], &linear[16..32]);
    }
}
