//! A minimal PNG writer, for looking at decoded assets.
//!
//! Dependency-free: zlib permits **stored** deflate blocks, so a valid file
//! needs only CRC-32 and Adler-32. Output is around 5% larger than a real
//! encoder's. This makes assets *visible*; it is not an image library.

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
    chunk(&mut out, b"IEND", &[]);
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
    #[should_panic(expected = "does not match")]
    fn rejects_a_mismatched_pixel_buffer() {
        let _ = encode_rgba(4, 4, &[0u8; 16]);
    }
}
