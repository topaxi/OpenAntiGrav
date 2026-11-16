//! A minimal PNG writer, for looking at decoded assets.
//!
//! Dependency-free: zlib permits **stored** deflate blocks, so a valid file
//! needs only CRC-32 and Adler-32. Output is around 5% larger than a real
//! encoder's. This makes assets *visible*; it is not an image library.
//!
//! A file can carry **extra chunks** ([`encode_rgba_with`]) and be read back chunk by chunk
//! ([`chunks`]): a screenshot's metadata travels in an `iTXt` text chunk ([`itxt`]) and a
//! private ancillary one.

/// Largest payload in one stored deflate block.
const MAX_STORED_BLOCK: usize = 0xFFFF;

/// Encodes RGBA8888 pixels as a PNG.
///
/// `pixels` must be `width * height * 4` bytes, row-major from the top left.
///
/// # Panics
///
/// Panics if `pixels` is the wrong length, or either dimension is zero.
#[must_use]
pub fn encode_rgba(width: u32, height: u32, pixels: &[u8]) -> Vec<u8> {
    encode_rgba_with(width, height, pixels, &[])
}

/// [`encode_rgba`], with `extra` chunks written between `IDAT` and `IEND` as `(type, data)`.
///
/// A reader that does not know a chunk skips it when its type is ancillary (first letter
/// lowercase), which is what every extra chunk here must be.
///
/// # Panics
///
/// As [`encode_rgba`]; and on an `extra` type whose first letter is uppercase, which a decoder
/// would be bound to refuse.
#[must_use]
pub fn encode_rgba_with(
    width: u32,
    height: u32,
    pixels: &[u8],
    extra: &[([u8; 4], Vec<u8>)],
) -> Vec<u8> {
    assert!(width > 0 && height > 0, "zero-sized image");
    assert_eq!(
        pixels.len(),
        width as usize * height as usize * 4,
        "pixel buffer does not match the given dimensions"
    );

    let mut out = Vec::with_capacity(pixels.len() + 1024);
    out.extend(b"\x89PNG\r\n\x1a\n");

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend(width.to_be_bytes());
    ihdr.extend(height.to_be_bytes());
    ihdr.extend([
        8, // bit depth
        6, // colour type: RGBA
        0, // deflate
        0, // adaptive filtering
        0, // no interlace
    ]);
    chunk(&mut out, b"IHDR", &ihdr);

    // Each scanline is prefixed with its filter type; 0 (none) keeps the writer trivial.
    let mut raw = Vec::with_capacity(pixels.len() + height as usize);
    for row in pixels.chunks_exact(width as usize * 4) {
        raw.push(0);
        raw.extend_from_slice(row);
    }

    chunk(&mut out, b"IDAT", &zlib_stored(&raw));
    for (kind, data) in extra {
        assert!(
            kind[0].is_ascii_lowercase(),
            "an extra chunk must be ancillary"
        );
        chunk(&mut out, kind, data);
    }
    chunk(&mut out, b"IEND", &[]);
    out
}

/// The `iTXt` chunk's type.
pub const ITXT: [u8; 4] = *b"iTXt";

/// An uncompressed, untranslated UTF-8 `iTXt` chunk's data: `keyword`, then `text`.
///
/// # Panics
///
/// On a keyword that is empty, over 79 bytes, or holds a NUL.
#[must_use]
pub fn itxt(keyword: &str, text: &str) -> Vec<u8> {
    assert!(
        !keyword.is_empty() && keyword.len() <= 79 && !keyword.contains('\0'),
        "a PNG text keyword is 1 to 79 bytes with no NUL"
    );
    let mut data = Vec::with_capacity(keyword.len() + text.len() + 5);
    data.extend(keyword.as_bytes());
    // NUL, compression flag 0 (text is stored as it stands), method 0, an empty language tag,
    // an empty translated keyword.
    data.extend([0, 0, 0, 0, 0]);
    data.extend(text.as_bytes());
    data
}

/// The text of an uncompressed `iTXt` chunk whose keyword is `keyword`, from [`chunks`]'s data.
#[must_use]
pub fn itxt_text<'a>(data: &'a [u8], keyword: &str) -> Option<&'a str> {
    let rest = data.strip_prefix(keyword.as_bytes())?.strip_prefix(&[0])?;
    let (&[compressed, _method], rest) = rest.split_first_chunk::<2>()?;
    if compressed != 0 {
        return None;
    }
    let rest = &rest[rest.iter().position(|&b| b == 0)? + 1..];
    let rest = &rest[rest.iter().position(|&b| b == 0)? + 1..];
    std::str::from_utf8(rest).ok()
}

/// Why [`chunks`] stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkError {
    /// Not a PNG: the eight signature bytes are missing.
    Signature,
    /// A chunk runs past the end of the file.
    Truncated,
    /// A chunk's CRC does not match its bytes.
    Crc,
}

impl std::fmt::Display for ChunkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Signature => "not a PNG (bad signature)",
            Self::Truncated => "the PNG is cut short",
            Self::Crc => "a PNG chunk fails its checksum",
        })
    }
}

impl std::error::Error for ChunkError {}

/// A chunk read back: its type and data.
pub type Chunk<'a> = ([u8; 4], &'a [u8]);

/// Every chunk of `png` in order as `(type, data)`, each checked against its CRC.
///
/// # Errors
///
/// [`ChunkError`] for a file that is not a whole PNG; nothing is returned of a damaged one.
pub fn chunks(png: &[u8]) -> Result<Vec<Chunk<'_>>, ChunkError> {
    let mut rest = png
        .strip_prefix(b"\x89PNG\r\n\x1a\n")
        .ok_or(ChunkError::Signature)?;
    let mut out = Vec::new();
    while !rest.is_empty() {
        let (head, after) = rest.split_first_chunk::<8>().ok_or(ChunkError::Truncated)?;
        let len = u32::from_be_bytes([head[0], head[1], head[2], head[3]]) as usize;
        let kind = [head[4], head[5], head[6], head[7]];
        if after.len() < len + 4 {
            return Err(ChunkError::Truncated);
        }
        let (data, after) = after.split_at(len);
        let (stored, after) = after
            .split_first_chunk::<4>()
            .ok_or(ChunkError::Truncated)?;
        let mut crc = Crc32::new();
        crc.update(&kind);
        crc.update(data);
        if crc.finish().to_be_bytes() != *stored {
            return Err(ChunkError::Crc);
        }
        out.push((kind, data));
        rest = after;
    }
    Ok(out)
}

/// `png` with every ancillary chunk left out: the picture alone, so two files of the same pixels
/// are the same bytes whatever description each carries. A file that is not a whole PNG comes
/// back as it was.
#[must_use]
pub fn picture_only(png: &[u8]) -> Vec<u8> {
    let Ok(found) = chunks(png) else {
        return png.to_vec();
    };
    let mut out = png[..8].to_vec();
    for (kind, data) in found {
        if kind[0].is_ascii_uppercase() {
            chunk(&mut out, &kind, data);
        }
    }
    out
}

/// Writes a length-prefixed, CRC-suffixed PNG chunk.
fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend((data.len() as u32).to_be_bytes());
    out.extend(kind);
    out.extend(data);

    let mut crc = Crc32::new();
    crc.update(kind);
    crc.update(data);
    out.extend(crc.finish().to_be_bytes());
}

/// Wraps `data` in a zlib stream of stored deflate blocks.
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + data.len() / MAX_STORED_BLOCK * 5 + 16);
    out.extend([0x78, 0x01]); // deflate, 32 KiB window, no preset dictionary

    if data.is_empty() {
        out.extend([0x01, 0x00, 0x00, 0xff, 0xff]);
    } else {
        for (i, block) in data.chunks(MAX_STORED_BLOCK).enumerate() {
            let last = (i + 1) * MAX_STORED_BLOCK >= data.len();
            out.push(u8::from(last));
            let len = block.len() as u16;
            out.extend(len.to_le_bytes());
            out.extend((!len).to_le_bytes());
            out.extend(block);
        }
    }

    out.extend(adler32(data).to_be_bytes());
    out
}

/// CRC-32 as PNG uses it: reflected, polynomial `0xEDB88320`, initialised to
/// all ones.
///
/// Differs from `oag_formats::wad::hash_name` only in its initial value.
struct Crc32(u32);

impl Crc32 {
    fn new() -> Self {
        Self(0xFFFF_FFFF)
    }

    fn update(&mut self, data: &[u8]) {
        for &byte in data {
            let mut v = (self.0 ^ u32::from(byte)) & 0xff;
            for _ in 0..8 {
                v = if v & 1 != 0 {
                    (v >> 1) ^ 0xEDB8_8320
                } else {
                    v >> 1
                };
            }
            self.0 = v ^ (self.0 >> 8);
        }
    }

    fn finish(&self) -> u32 {
        !self.0
    }
}

/// Adler-32, as the zlib stream trailer.
fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in data {
        a = (a + u32::from(byte)) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_a_well_formed_header() {
        let png = encode_rgba(2, 2, &[0u8; 16]);
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(&png[16..20], &2u32.to_be_bytes());
        assert_eq!(&png[20..24], &2u32.to_be_bytes());
        assert!(png.ends_with(b"IEND\xae\x42\x60\x82"), "IEND CRC is fixed");
    }

    #[test]
    fn matches_the_known_crc32_test_vector() {
        // "123456789" -> 0xCBF43926 is the standard CRC-32 check value.
        let mut c = Crc32::new();
        c.update(b"123456789");
        assert_eq!(c.finish(), 0xCBF4_3926);
    }

    #[test]
    fn matches_the_known_adler32_test_vector() {
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
        assert_eq!(adler32(b""), 1);
    }

    #[test]
    fn stored_blocks_split_at_the_deflate_limit() {
        // Just over one block, so the stream must contain two.
        let data = vec![0u8; MAX_STORED_BLOCK + 10];
        let z = zlib_stored(&data);
        assert_eq!(z[2], 0x00, "first block is not final");
        let second = 2 + 5 + MAX_STORED_BLOCK;
        assert_eq!(z[second], 0x01, "second block is final");
    }

    #[test]
    fn a_single_block_is_marked_final() {
        let z = zlib_stored(&[1, 2, 3]);
        assert_eq!(z[2], 0x01);
    }

    #[test]
    fn extra_chunks_round_trip_and_the_picture_still_decodes() {
        let pixels: Vec<u8> = (0..16u8).collect();
        let text = itxt("oag", "title = \"x\"\n");
        let png = encode_rgba_with(
            2,
            2,
            &pixels,
            &[(ITXT, text.clone()), (*b"oaGs", vec![1, 2, 3])],
        );
        let found = chunks(&png).expect("a whole file");
        let kinds: Vec<_> = found.iter().map(|(k, _)| *k).collect();
        assert_eq!(
            kinds,
            [*b"IHDR", *b"IDAT", ITXT, *b"oaGs", *b"IEND"],
            "extras sit between the picture and the end"
        );
        assert_eq!(found[2].1, &text[..]);
        assert_eq!(itxt_text(found[2].1, "oag"), Some("title = \"x\"\n"));
        assert_eq!(itxt_text(found[2].1, "other"), None);
        assert_eq!(found[3].1, &[1, 2, 3]);
        let image = crate::replacer::decode_png(&png).expect("an ordinary decoder takes it");
        assert_eq!(image.rgba, pixels, "the extras leave the pixels alone");
    }

    #[test]
    fn the_picture_alone_is_the_same_bytes_whatever_the_description() {
        let plain = encode_rgba(2, 2, &[7; 16]);
        let described = encode_rgba_with(2, 2, &[7; 16], &[(ITXT, itxt("oag", "a = 1"))]);
        assert_ne!(plain, described);
        assert_eq!(picture_only(&described), plain);
        assert_eq!(picture_only(b"not a png"), b"not a png");
    }

    #[test]
    fn a_damaged_chunk_is_refused() {
        let whole = encode_rgba_with(1, 1, &[0; 4], &[(*b"oaGs", vec![9; 8])]);
        assert_eq!(
            chunks(&whole[..whole.len() - 3]),
            Err(ChunkError::Truncated)
        );
        let mut png = whole;
        let at = png.windows(4).position(|w| w == b"oaGs").unwrap() + 5;
        png[at] ^= 1;
        assert_eq!(chunks(&png), Err(ChunkError::Crc));
        assert_eq!(chunks(b"nope"), Err(ChunkError::Signature));
    }

    #[test]
    #[should_panic(expected = "ancillary")]
    fn a_critical_extra_chunk_is_refused() {
        let _ = encode_rgba_with(1, 1, &[0; 4], &[(*b"OAGS", vec![])]);
    }

    #[test]
    #[should_panic(expected = "does not match")]
    fn rejects_a_mismatched_pixel_buffer() {
        let _ = encode_rgba(4, 4, &[0u8; 16]);
    }
}
