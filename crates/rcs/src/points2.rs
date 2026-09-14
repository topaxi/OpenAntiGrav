//! `.points2`: a ship's hull sampled to a point cloud, for the Fury front end.
//!
//! Wipeout HD's Fury style draws its menu backdrop from one of nineteen of
//! these under `Data/FE/Fury/` - a hull sampled to 51,336..55,000 points,
//! each a position, a normal and four random bytes, drawn as camera-facing
//! sprites and trailed. The file is the cloud object's first `0x80` bytes
//! written out verbatim, then the records `Points2_Load` copies straight into
//! video memory, and every field below is one the renderer's own vertex
//! bindings name: the three streams `PointCloud_DrawRaw` binds are
//! `SF x3 @0`, `UB x4 @6` and `UB x4 @10` on a 16-byte stride. See
//! [menu-backdrop.md](../../../docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md).
//!
//! ```text
//! +0x00  u32   0
//! +0x04  u32   count
//! +0x08  u32   stride, 16
//! +0x0c  u32   0
//! +0x10  f32*3 bounding-box min
//! +0x1c  f32*3 bounding-box max
//! +0x28  u32   data offset, 0x80
//! +0x2c  u32   0
//! +0x30  f32*3 0 0 0
//! +0x3c  4x4   identity
//! +0x80  records, `count * 16` bytes
//!
//! +0   f16*3  position
//! +6   u8*4   rand
//! +10  u8*3   normal, `v / 255 * 2 - 1`
//! +13  u8*3   0
//! ```
//!
//! Measured on all nineteen files (`tests/points2_ground_truth.rs`): every
//! position sits inside the header's box to within one half-float ulp, the
//! padding is zero, and every normal is unit length to within the byte
//! quantisation - except forty-six points across four files that carry the
//! zero vector's nearest encoding, a sampling artefact the test counts by
//! name. A caller displacing along the normal moves those points nowhere.
//!
//! Big-endian throughout, like everything else on the PS3.

use oag_formats::ByteOrder;

use crate::rcsmodel::unpack_half;

/// Where the records start, and how long the header this reads is.
pub const HEADER_SIZE: usize = 0x80;

/// One record's size; the header says so too, and the parser checks it.
pub const RECORD_SIZE: usize = 16;

/// One sampled point of the hull.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// In the cloud's own space, inside [`PointCloud::min`]..[`PointCloud::max`].
    pub position: [f32; 3],
    /// Four bytes of per-point noise, `0..=254` as shipped, read by the vertex
    /// programs as a `0..1` vector. Mode 2 uses `.x` alone, for the colour
    /// ramp's jitter.
    pub rand: [u8; 4],
    /// The hull's normal at the sample, unit length as shipped but for the
    /// degenerates the module docs count. The vertex programs displace along
    /// it by the music multiplier.
    pub normal: [f32; 3],
}

/// A whole cloud.
#[derive(Debug, Clone, PartialEq)]
pub struct PointCloud {
    /// The header's bounding box.
    pub min: [f32; 3],
    pub max: [f32; 3],
    /// Every record, in file order.
    pub points: Vec<Point>,
}

/// Why a blob is not a `.points2`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Shorter than the header.
    Truncated { len: usize },
    /// The header's stride or data offset is not this format's.
    Layout { stride: u32, data_offset: u32 },
    /// The header's count does not match the bytes after it.
    Count { count: u32, available: usize },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncated { len } => {
                write!(
                    f,
                    "{len} bytes is shorter than the {HEADER_SIZE}-byte header"
                )
            }
            Self::Layout {
                stride,
                data_offset,
            } => write!(
                f,
                "stride {stride} at data offset {data_offset:#x} is not {RECORD_SIZE} at {HEADER_SIZE:#x}"
            ),
            Self::Count { count, available } => write!(
                f,
                "{count} records need {} bytes, {available} follow the header",
                *count as usize * RECORD_SIZE
            ),
        }
    }
}

impl std::error::Error for Error {}

/// Reads a whole `.points2`.
///
/// # Errors
///
/// [`Error`] when the header is short, declares another layout, or promises
/// more records than the bytes carry. A file with fewer bytes than its count
/// is refused rather than truncated: a cloud missing its last points would
/// draw a hull with a hole in it and nothing would say so.
pub fn parse(bytes: &[u8]) -> Result<PointCloud, Error> {
    if bytes.len() < HEADER_SIZE {
        return Err(Error::Truncated { len: bytes.len() });
    }
    let count = ByteOrder::Big.u32(bytes, 0x04);
    let stride = ByteOrder::Big.u32(bytes, 0x08);
    let data_offset = ByteOrder::Big.u32(bytes, 0x28);
    if stride as usize != RECORD_SIZE || data_offset as usize != HEADER_SIZE {
        return Err(Error::Layout {
            stride,
            data_offset,
        });
    }
    let records = &bytes[HEADER_SIZE..];
    if records.len() < count as usize * RECORD_SIZE {
        return Err(Error::Count {
            count,
            available: records.len(),
        });
    }
    let vec3 = |at: usize| std::array::from_fn(|i| ByteOrder::Big.f32(bytes, at + i * 4));
    let points = records
        .as_chunks::<RECORD_SIZE>()
        .0
        .iter()
        .take(count as usize)
        .map(|record| Point {
            position: [
                unpack_half(ByteOrder::Big.u16(record, 0)),
                unpack_half(ByteOrder::Big.u16(record, 2)),
                unpack_half(ByteOrder::Big.u16(record, 4)),
            ],
            rand: [record[6], record[7], record[8], record[9]],
            normal: [
                unpack_normal(record[10]),
                unpack_normal(record[11]),
                unpack_normal(record[12]),
            ],
        })
        .collect();
    Ok(PointCloud {
        min: vec3(0x10),
        max: vec3(0x1c),
        points,
    })
}

/// A normal's byte, as the vertex binding declares it: unsigned, normalised,
/// then the program's `* 2 - 1`.
fn unpack_normal(byte: u8) -> f32 {
    f32::from(byte) / 255.0 * 2.0 - 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(count: u32, stride: u32, data_offset: u32) -> Vec<u8> {
        let mut out = vec![0u8; HEADER_SIZE];
        out[0x04..0x08].copy_from_slice(&count.to_be_bytes());
        out[0x08..0x0c].copy_from_slice(&stride.to_be_bytes());
        out[0x28..0x2c].copy_from_slice(&data_offset.to_be_bytes());
        for (at, value) in [
            (0x10, -1.0f32),
            (0x14, -2.0),
            (0x18, -3.0),
            (0x1c, 1.0),
            (0x20, 2.0),
            (0x24, 3.0),
        ] {
            out[at..at + 4].copy_from_slice(&value.to_be_bytes());
        }
        out
    }

    #[test]
    fn a_record_decodes_its_three_streams() {
        let mut bytes = header(1, 16, 0x80);
        // 1.0, -2.0, 0.5 as halves; rand 1 2 3 4; normal 255 0 127; pad.
        bytes.extend_from_slice(&[
            0x3c, 0x00, 0xc0, 0x00, 0x38, 0x00, 1, 2, 3, 4, 255, 0, 127, 0, 0, 0,
        ]);
        let cloud = parse(&bytes).expect("parses");
        assert_eq!(cloud.min, [-1.0, -2.0, -3.0]);
        assert_eq!(cloud.max, [1.0, 2.0, 3.0]);
        let point = cloud.points[0];
        assert_eq!(point.position, [1.0, -2.0, 0.5]);
        assert_eq!(point.rand, [1, 2, 3, 4]);
        assert_eq!(point.normal[0], 1.0);
        assert_eq!(point.normal[1], -1.0);
        assert!((point.normal[2] - (127.0 / 255.0 * 2.0 - 1.0)).abs() < 1e-6);
    }

    #[test]
    fn another_layout_is_refused_not_misread() {
        assert_eq!(
            parse(&header(0, 12, 0x80)),
            Err(Error::Layout {
                stride: 12,
                data_offset: 0x80
            })
        );
        assert_eq!(parse(&[0; 16]), Err(Error::Truncated { len: 16 }));
    }

    #[test]
    fn a_short_file_is_refused_not_truncated() {
        let mut bytes = header(2, 16, 0x80);
        bytes.extend_from_slice(&[0; 16]);
        assert_eq!(
            parse(&bytes),
            Err(Error::Count {
                count: 2,
                available: 16
            })
        );
    }
}
