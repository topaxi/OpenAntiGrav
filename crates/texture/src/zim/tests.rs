use super::*;

fn header(width: u32, height: u32, flags: u32) -> Vec<u8> {
    let mut out = MAGIC.to_vec();
    for word in [width, height, flags] {
        out.extend_from_slice(&word.to_le_bytes());
    }
    out
}

/// A zstd frame of one raw block holding `content` (under 256 bytes, single segment).
fn zstd_raw_frame(content: &[u8]) -> Vec<u8> {
    let mut out = vec![0x28, 0xb5, 0x2f, 0xfd, 0x20, content.len() as u8];
    let block = (content.len() as u32) << 3 | 1;
    out.extend_from_slice(&block.to_le_bytes()[..3]);
    out.extend_from_slice(content);
    out
}

fn zlib(content: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(content).unwrap();
    encoder.finish().unwrap()
}

const TEXELS: [u8; 16] = [255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 1, 2, 3, 4];

#[test]
fn raw_zlib_and_zstd_agree() {
    for (flags, body) in [
        (0, TEXELS.to_vec()),
        (ZLIB, zlib(&TEXELS)),
        (ZSTD, zstd_raw_frame(&TEXELS)),
    ] {
        let mut file = header(2, 2, flags);
        file.extend_from_slice(&body);
        let image = decode(&file).unwrap_or_else(|why| panic!("flags {flags:#x}: {why}"));
        assert_eq!((image.width, image.height), (2, 2), "flags {flags:#x}");
        assert_eq!(image.rgba, TEXELS, "flags {flags:#x}");
    }
}

#[test]
fn only_level_zero_is_read_of_a_chain() {
    // 2x2 and its 1x1 level, in one zlib stream, flagged `0x10` (a chain).
    let mut chain = TEXELS.to_vec();
    chain.extend_from_slice(&[9, 9, 9, 9]);
    let mut file = header(2, 2, ZLIB | 0x10);
    file.extend_from_slice(&zlib(&chain));
    assert_eq!(decode(&file).unwrap().rgba, TEXELS);
}

#[test]
fn the_sixteen_bit_formats_widen_to_full_range() {
    // RGBA4444: red F, green 0, blue 8, alpha F.
    let mut file = header(1, 1, 1);
    file.extend_from_slice(&0xf08fu16.to_le_bytes());
    assert_eq!(decode(&file).unwrap().rgba, [255, 0, 136, 255]);
    // RGB565: red 31, green 63, blue 0.
    let mut file = header(1, 1, 2);
    file.extend_from_slice(&0xffe0u16.to_le_bytes());
    assert_eq!(decode(&file).unwrap().rgba, [255, 255, 0, 255]);
}

#[test]
fn what_is_not_a_picture_is_refused_by_name() {
    assert_eq!(decode(b"PNG"), Err(Error::NotZim));
    assert_eq!(decode(&header(0, 2, 0)), Err(Error::Size(0, 2)));
    assert_eq!(decode(&header(2, 20000, 0)), Err(Error::Size(2, 20000)));
    assert_eq!(decode(&header(2, 2, 3)), Err(Error::Format(3)));
    assert_eq!(decode(&header(2, 2, ZLIB | ZSTD)), Err(Error::BothCodecs));
    let mut short = header(2, 2, 0);
    short.extend_from_slice(&TEXELS[..8]);
    assert!(matches!(decode(&short), Err(Error::Data(_))));
    let mut bad = header(2, 2, ZSTD);
    bad.extend_from_slice(&[1, 2, 3, 4]);
    assert!(matches!(decode(&bad), Err(Error::Data(_))));
}
