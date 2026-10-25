/// Reflected CRC-32 polynomial, as used by the game's table builder.
pub const CRC32_POLY: u32 = 0xEDB8_8320;

/// Bare CRC-32 (the reflected, `0xedb88320` variant), uninverted.
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &b in bytes {
        crc = crc32_step(crc, b);
    }
    !crc
}

/// One byte of reflected CRC-32.
///
/// Computed, not table-driven.
pub fn crc32_step(crc: u32, byte: u8) -> u32 {
    let mut v = (crc ^ u32::from(byte)) & 0xff;
    for _ in 0..8 {
        v = if v & 1 != 0 {
            (v >> 1) ^ CRC32_POLY
        } else {
            v >> 1
        };
    }
    v ^ (crc >> 8)
}
