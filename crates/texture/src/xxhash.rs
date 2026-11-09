//! XXH32 and XXH64, the two hashes PPSSPP's texture replacement computes.
//!
//! Written out here rather than taken from a crate: both are about forty lines,
//! and `oag-texture` has no dependency but `oag-formats`. The tests pin them to
//! the reference vectors (`xxhash` 0.8, seeds and lengths that exercise the
//! 4-lane bulk loop, the 8/4/1-byte tails and the empty input).

const P32_1: u32 = 0x9E37_79B1;
const P32_2: u32 = 0x85EB_CA77;
const P32_3: u32 = 0xC2B2_AE3D;
const P32_4: u32 = 0x27D4_EB2F;
const P32_5: u32 = 0x1656_67B1;

const P64_1: u64 = 0x9E37_79B1_85EB_CA87;
const P64_2: u64 = 0xC2B2_AE3D_27D4_EB4F;
const P64_3: u64 = 0x1656_67B1_9E37_79F9;
const P64_4: u64 = 0x85EB_CA77_C2B2_AE63;
const P64_5: u64 = 0x27D4_EB2F_1656_67C5;

fn read32(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn read64(b: &[u8], at: usize) -> u64 {
    let mut a = [0u8; 8];
    a.copy_from_slice(&b[at..at + 8]);
    u64::from_le_bytes(a)
}

/// XXH32 of `data` with `seed`.
#[must_use]
pub fn xxh32(data: &[u8], seed: u32) -> u32 {
    let round = |acc: u32, lane: u32| {
        acc.wrapping_add(lane.wrapping_mul(P32_2))
            .rotate_left(13)
            .wrapping_mul(P32_1)
    };
    let mut at = 0;
    let mut h = if data.len() >= 16 {
        let mut v = [
            seed.wrapping_add(P32_1).wrapping_add(P32_2),
            seed.wrapping_add(P32_2),
            seed,
            seed.wrapping_sub(P32_1),
        ];
        while at + 16 <= data.len() {
            for (i, lane) in v.iter_mut().enumerate() {
                *lane = round(*lane, read32(data, at + i * 4));
            }
            at += 16;
        }
        v[0].rotate_left(1)
            .wrapping_add(v[1].rotate_left(7))
            .wrapping_add(v[2].rotate_left(12))
            .wrapping_add(v[3].rotate_left(18))
    } else {
        seed.wrapping_add(P32_5)
    };
    h = h.wrapping_add(data.len() as u32);
    while at + 4 <= data.len() {
        h = h
            .wrapping_add(read32(data, at).wrapping_mul(P32_3))
            .rotate_left(17)
            .wrapping_mul(P32_4);
        at += 4;
    }
    while at < data.len() {
        h = h
            .wrapping_add(u32::from(data[at]).wrapping_mul(P32_5))
            .rotate_left(11)
            .wrapping_mul(P32_1);
        at += 1;
    }
    h ^= h >> 15;
    h = h.wrapping_mul(P32_2);
    h ^= h >> 13;
    h = h.wrapping_mul(P32_3);
    h ^ (h >> 16)
}

/// XXH64 of `data` with `seed`.
#[must_use]
pub fn xxh64(data: &[u8], seed: u64) -> u64 {
    let round = |acc: u64, lane: u64| {
        acc.wrapping_add(lane.wrapping_mul(P64_2))
            .rotate_left(31)
            .wrapping_mul(P64_1)
    };
    let merge = |acc: u64, lane: u64| {
        (acc ^ round(0, lane))
            .wrapping_mul(P64_1)
            .wrapping_add(P64_4)
    };
    let mut at = 0;
    let mut h = if data.len() >= 32 {
        let mut v = [
            seed.wrapping_add(P64_1).wrapping_add(P64_2),
            seed.wrapping_add(P64_2),
            seed,
            seed.wrapping_sub(P64_1),
        ];
        while at + 32 <= data.len() {
            for (i, lane) in v.iter_mut().enumerate() {
                *lane = round(*lane, read64(data, at + i * 8));
            }
            at += 32;
        }
        let mut h = v[0]
            .rotate_left(1)
            .wrapping_add(v[1].rotate_left(7))
            .wrapping_add(v[2].rotate_left(12))
            .wrapping_add(v[3].rotate_left(18));
        for lane in v {
            h = merge(h, lane);
        }
        h
    } else {
        seed.wrapping_add(P64_5)
    };
    h = h.wrapping_add(data.len() as u64);
    while at + 8 <= data.len() {
        h = (h ^ round(0, read64(data, at)))
            .rotate_left(27)
            .wrapping_mul(P64_1)
            .wrapping_add(P64_4);
        at += 8;
    }
    if at + 4 <= data.len() {
        h = (h ^ u64::from(read32(data, at)).wrapping_mul(P64_1))
            .rotate_left(23)
            .wrapping_mul(P64_2)
            .wrapping_add(P64_3);
        at += 4;
    }
    while at < data.len() {
        h = (h ^ u64::from(data[at]).wrapping_mul(P64_5))
            .rotate_left(11)
            .wrapping_mul(P64_1);
        at += 1;
    }
    h ^= h >> 33;
    h = h.wrapping_mul(P64_2);
    h ^= h >> 29;
    h = h.wrapping_mul(P64_3);
    h ^ (h >> 32)
}

#[cfg(test)]
mod tests;
