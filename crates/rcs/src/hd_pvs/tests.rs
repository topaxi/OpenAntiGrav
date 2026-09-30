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
    let width = chunks / 8 + 1;
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
fn the_bitmap_is_chunks_over_eight_plus_one_bytes_wide() {
    // 20 chunks is three bytes, and 16 would be three as well: the loader
    // computes `(chunks >> 3) + 1`, so a count that divides by eight gets a
    // whole spare byte. A reader that used `ceil` would run the cells into
    // each other on exactly those counts.
    for (chunks, want) in [(20usize, 3usize), (16, 3), (8, 2), (7, 1)] {
        let blob = build(3, chunks, &[[0.0; 3]; 3], |cell, chunk| chunk == cell, 0);
        let pvs = Pvs::parse(&blob).unwrap();
        assert_eq!(pvs.bitmap_bytes(), want, "{chunks} chunk(s)");
        assert_eq!(pvs.cell_bits(0).unwrap().len(), want);
        for cell in 0..3 {
            assert!(pvs.visible(cell, cell));
            assert_eq!(pvs.visible_count(cell), 1);
        }
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

/// Builds a [`Dialect::Psp2`] file: little-endian, word 2 is `1`, a bitmap is
/// `ceil(chunks / 8)` bytes and a cell record's fourth float is zero.
fn build_psp2(cells: usize, chunks: usize, set: impl Fn(usize, usize) -> bool) -> Vec<u8> {
    let width = chunks.div_ceil(8);
    let mut out = Vec::new();
    for word in [cells as u32, chunks as u32, 1, 0xdead_beef] {
        out.extend_from_slice(&word.to_le_bytes());
    }
    for cell in 0..cells {
        for axis in [cell as f32, 2.0, 3.0, 0.0] {
            out.extend_from_slice(&axis.to_le_bytes());
        }
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
    out
}

#[test]
fn the_psp2_dialect_is_little_endian_with_a_tight_bitmap() {
    // 16 chunks is exactly two bytes here. HD's rule would want three and run
    // every cell into the next: the four Omega files whose chunk count divides
    // by eight are what settled it.
    for (chunks, want) in [(20usize, 3usize), (16, 2), (8, 1), (7, 1)] {
        let blob = build_psp2(3, chunks, |cell, chunk| chunk == cell);
        let pvs = Pvs::parse_as(&blob, Dialect::Psp2).unwrap();
        assert_eq!(pvs.dialect(), Dialect::Psp2);
        assert_eq!(pvs.bitmap_bytes(), want, "{chunks} chunk(s)");
        assert_eq!(pvs.trailing(), 0, "{chunks} chunk(s) fill the file exactly");
        for cell in 0..3 {
            assert!(pvs.visible(cell, cell));
            assert_eq!(pvs.visible_count(cell), 1);
        }
        assert_eq!(pvs.position(2), Some([2.0, 2.0, 3.0]));
    }
}

#[test]
fn detection_reads_header_word_two_and_never_the_title() {
    let psp2 = build_psp2(2, 24, |_, chunk| chunk == 5);
    assert_eq!(Pvs::parse_detect(&psp2).unwrap().dialect(), Dialect::Psp2);
    let ps3 = build(2, 24, &[[0.0; 3]; 2], |_, chunk| chunk == 5, 0);
    assert_eq!(Pvs::parse_detect(&ps3).unwrap().dialect(), Dialect::Ps3);
    // Each dialect refuses the other's file rather than misreading it, and
    // HD's own entry point never accepts a little-endian one.
    assert!(matches!(
        Pvs::parse(&psp2),
        Err(Error::UnknownLayout { .. })
    ));
    assert!(matches!(
        Pvs::parse_as(&ps3, Dialect::Psp2),
        Err(Error::UnknownLayout { .. })
    ));
    let mut neither = psp2.clone();
    neither[8..12].copy_from_slice(&7u32.to_le_bytes());
    assert!(matches!(
        Pvs::parse_detect(&neither),
        Err(Error::UnknownLayout { .. })
    ));
}
