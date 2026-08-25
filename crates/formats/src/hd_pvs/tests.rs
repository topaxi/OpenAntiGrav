use super::*;

/// Builds a `.pvs` with `cells` cells over `chunks` chunks, each cell's bitmap
/// filled by `set(cell, chunk)`, plus `trailing` unread bytes.
fn build(
    cells: usize,
    chunks: usize,
    positions: &[[f32; 3]],
    set: impl Fn(usize, usize) -> bool,
    trailing: usize,
) -> Vec<u8> {
    let width = chunks.div_ceil(8);
    let mut out = Vec::new();
    for word in [cells as u32, chunks as u32, CELLS_AT as u32, 0xdead_beef] {
        out.extend_from_slice(&word.to_be_bytes());
    }
    for cell in 0..cells {
        let p = positions.get(cell).copied().unwrap_or([0.0; 3]);
        for axis in p {
            out.extend_from_slice(&axis.to_be_bytes());
        }
        out.extend_from_slice(&p[2].to_be_bytes());
    }
    for cell in 0..cells {
        let mut map = vec![0u8; width];
        for chunk in 0..chunks {
            if set(cell, chunk) {
                map[chunk >> 3] |= 1 << (chunk & 7);
            }
        }
        out.extend_from_slice(&map);
    }
    out.extend(std::iter::repeat_n(0xa5u8, trailing));
    out
}

#[test]
fn a_bit_is_read_lsb_first_over_chunk_index() {
    // Chunk 0 and chunk 9 only: byte 0 bit 0, byte 1 bit 1.
    let blob = build(1, 20, &[[0.0; 3]], |_, chunk| chunk == 0 || chunk == 9, 0);
    let pvs = Pvs::parse(&blob).unwrap();
    assert!(pvs.visible(0, 0));
    assert!(!pvs.visible(0, 1));
    assert!(!pvs.visible(0, 8));
    assert!(pvs.visible(0, 9));
    assert_eq!(pvs.visible_count(0), 2);
}

#[test]
fn the_bitmap_is_ceil_chunks_over_eight_bytes_wide() {
    // 20 chunks is three bytes: a reader using two or four would run the
    // cells into each other, which is what the disc's padding-bit test rules
    // out.
    let blob = build(3, 20, &[[0.0; 3]; 3], |cell, chunk| chunk == cell, 0);
    let pvs = Pvs::parse(&blob).unwrap();
    assert_eq!(pvs.cell_bits(0).unwrap().len(), 3);
    for cell in 0..3 {
        assert!(pvs.visible(cell, cell));
        assert_eq!(pvs.visible_count(cell), 1);
    }
}

#[test]
fn a_cell_is_found_by_its_own_position() {
    let positions = [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [20.0, 0.0, 0.0]];
    let blob = build(3, 8, &positions, |_, _| true, 0);
    let pvs = Pvs::parse(&blob).unwrap();
    assert_eq!(pvs.nearest_cell([9.0, 0.0, 0.0]), Some(1));
    assert_eq!(pvs.nearest_cell([-100.0, 0.0, 0.0]), Some(0));
    assert_eq!(pvs.nearest_cell([1e6, 0.0, 0.0]), Some(2));
    assert_eq!(pvs.position(1), Some([10.0, 0.0, 0.0]));
}

#[test]
fn trailing_bytes_are_reported_and_not_read() {
    let with = build(2, 8, &[[0.0; 3]; 2], |_, chunk| chunk == 3, 1234);
    let without = build(2, 8, &[[0.0; 3]; 2], |_, chunk| chunk == 3, 0);
    let a = Pvs::parse(&with).unwrap();
    let b = Pvs::parse(&without).unwrap();
    assert_eq!(a.trailing(), 1234);
    assert_eq!(b.trailing(), 0);
    assert!(a.visible(0, 3) && b.visible(0, 3));
    assert!(!a.visible(1, 4) && !b.visible(1, 4));
}

#[test]
fn an_out_of_range_lookup_draws_rather_than_hides() {
    let blob = build(1, 8, &[[0.0; 3]], |_, _| false, 0);
    let pvs = Pvs::parse(&blob).unwrap();
    assert!(!pvs.visible(0, 0), "a declared chunk answers its own bit");
    assert!(pvs.visible(0, 8), "past the declared chunk count");
    assert!(pvs.visible(9, 0), "past the declared cell count");
    assert!(allows(&[], 0), "an empty bitmap draws");
}

#[test]
fn a_file_shorter_than_its_own_table_is_refused() {
    let blob = build(4, 40, &[[0.0; 3]; 4], |_, _| true, 0);
    for cut in [0, CELLS_AT, blob.len() - 1] {
        assert!(matches!(
            Pvs::parse(&blob[..cut]),
            Err(Error::TooShort { .. })
        ));
    }
}

#[test]
fn a_header_this_parser_does_not_recognise_is_refused() {
    let mut blob = build(1, 8, &[[0.0; 3]], |_, _| true, 0);
    blob[8..12].copy_from_slice(&32u32.to_be_bytes());
    assert_eq!(Pvs::parse(&blob), Err(Error::UnknownLayout { word: 32 }));

    let empty = build(0, 8, &[], |_, _| true, 0);
    assert!(matches!(Pvs::parse(&empty), Err(Error::Empty { .. })));
}
