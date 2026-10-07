use super::*;

#[derive(Debug)]
struct Bytes(Vec<u8>);

impl Source for Bytes {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<()> {
        let end = offset as usize + buf.len();
        buf.copy_from_slice(
            self.0
                .get(offset as usize..end)
                .ok_or_else(|| bad(std::path::Path::new("mem"), "past the end"))?,
        );
        Ok(())
    }
}

const BLOCK: usize = 0x1000;

/// A PFSC with one stored sector, one inflated sector and one all-zero sector.
fn pfsc(data_len: u64) -> Vec<u8> {
    let stored: Vec<u8> = (0..BLOCK).map(|i| i as u8).collect();
    let plain = vec![7u8; BLOCK];
    let deflated = miniz_oxide::deflate::compress_to_vec(&plain, 6);
    let mut packed = vec![0x78, 0x9c];
    packed.extend_from_slice(&deflated);
    packed.extend_from_slice(&[0, 0, 0, 0]);
    let map_at = 0x30u64;
    let data_at = map_at + 4 * 8;
    let offsets = [
        data_at,
        data_at + BLOCK as u64,
        data_at + BLOCK as u64 + packed.len() as u64,
        data_at + BLOCK as u64 + packed.len() as u64 + BLOCK as u64 + 1,
    ];
    let mut out = Vec::new();
    out.extend_from_slice(b"PFSC");
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&6u32.to_le_bytes());
    out.extend_from_slice(&(BLOCK as u32).to_le_bytes());
    out.extend_from_slice(&(BLOCK as u64).to_le_bytes());
    out.extend_from_slice(&map_at.to_le_bytes());
    out.extend_from_slice(&data_at.to_le_bytes());
    out.extend_from_slice(&data_len.to_le_bytes());
    for o in offsets {
        out.extend_from_slice(&o.to_le_bytes());
    }
    out.extend_from_slice(&stored);
    out.extend_from_slice(&packed);
    out.extend_from_slice(&vec![0xAA; BLOCK + 1]);
    out
}

#[test]
fn pfsc_reads_stored_inflated_and_hole_sectors_across_a_boundary() {
    let image = pfsc(3 * BLOCK as u64);
    let len = image.len() as u64;
    let mut reader = PfscSource::open(Bytes(image), len, 4).unwrap();
    let mut buf = vec![0u8; 2 * BLOCK];
    reader.read_at(BLOCK as u64 / 2, &mut buf).unwrap();
    assert_eq!(buf[0], (BLOCK / 2) as u8, "stored sector, mid-way in");
    assert!(
        buf[BLOCK / 2..BLOCK / 2 + BLOCK].iter().all(|&b| b == 7),
        "inflated sector"
    );
    assert!(
        buf[BLOCK + BLOCK / 2..].iter().all(|&b| b == 0),
        "a span past a sector is a hole"
    );
    assert!(reader.read_at(3 * BLOCK as u64, &mut [0u8; 1]).is_err());
}

#[test]
fn pfsc_refuses_a_length_that_is_not_whole_sectors_by_name() {
    let image = pfsc(3 * BLOCK as u64 - 5);
    let len = image.len() as u64;
    let error = PfscSource::open(Bytes(image), len, 4)
        .expect_err("refused")
        .to_string();
    assert!(error.contains("not a multiple"), "{error}");
}

#[test]
fn pfsc_refuses_a_missing_magic() {
    let mut image = pfsc(3 * BLOCK as u64);
    image[0] = b'X';
    let len = image.len() as u64;
    assert!(PfscSource::open(Bytes(image), len, 4).is_err());
}

#[test]
fn extents_follow_a_block_list_across_scattered_blocks() {
    let mut raw = vec![0u8; 4 * 0x1000];
    for (i, b) in raw.iter_mut().enumerate() {
        *b = (i / 0x1000) as u8;
    }
    let mut extents = Extents {
        inner: Bytes(raw),
        block_size: 0x1000,
        start: 0,
        blocks: Some(vec![3, 1]),
        len: 0x2000,
    };
    let mut buf = [0u8; 4];
    extents.read_at(0x0FFE, &mut buf).unwrap();
    assert_eq!(buf, [3, 3, 1, 1]);
}
