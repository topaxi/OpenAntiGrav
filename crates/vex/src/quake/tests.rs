use super::*;

/// A node header at `offset` with `data_size` bytes of payload behind it.
fn node(offset: usize, data_size: usize) -> Node {
    Node {
        class_id: CLASS_QUAKE,
        offset,
        header_size: 0x20,
        data_size,
        child_count: 0,
        unk_0x0e: 0,
        name: None,
        depth: 0,
        parent: None,
    }
}

fn put_u16(file: &mut [u8], at: usize, value: u16) {
    file[at..at + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(file: &mut [u8], at: usize, value: u32) {
    file[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_f32(file: &mut [u8], at: usize, value: f32) {
    put_u32(file, at, value.to_bits());
}

/// One record at the payload's start, its parameters just past it, and a
/// "batch" before the node, reached by a negative self-relative offset as the
/// disc's own are.
#[test]
fn a_record_resolves_its_offsets_and_pins_its_sentinel_vertices() {
    let mut file = vec![0u8; 0x400];
    let node = node(0x100, HEADER + RECORD + 12);
    let payload = node.payload().start;
    put_u32(&mut file, payload, 1);
    let r = payload + HEADER;
    put_f32(&mut file, r + 4, -1.0);
    put_f32(&mut file, r + 0x14, -1.0);
    put_f32(&mut file, r + 0x20, 36.5);
    put_u16(&mut file, r + 0x30, 7);
    put_u16(&mut file, r + 0x32, NONE);
    put_u16(&mut file, r + 0x34, NONE);
    put_u16(&mut file, r + 0x36, NONE);
    put_u16(&mut file, r + 0x3c, 4);
    put_f32(&mut file, r + 0x40, 50.0);
    // Batch header at 0x40, its first position eight bytes past the header.
    put_u32(
        &mut file,
        r + 0x44,
        (0x88_i64 - (r + 0x44) as i64) as i32 as u32,
    );
    put_u32(&mut file, r + 0x48, (RECORD - 0x48) as u32);
    put_u32(
        &mut file,
        r + 0x4c,
        (0x40_i64 - (r + 0x4c) as i64) as i32 as u32,
    );
    put_u16(&mut file, r + 0x50, 3);
    put_u16(&mut file, r + 0x52, 1);
    put_f32(&mut file, r + 0x54, 0.25);
    put_f32(&mut file, r + 0x58, 0.5);
    let params = r + RECORD;
    put_f32(&mut file, params, 0.0);
    put_u32(&mut file, params + 4, u32::MAX);
    put_f32(&mut file, params + 8, 1.0);

    let spans = spans(&file, &node).expect("parses");
    assert_eq!(spans.len(), 1);
    let span = &spans[0];
    assert_eq!(span.batch, 0x40);
    assert_eq!(span.first_position, 0x88);
    assert_eq!(span.down_start, [0.0, -1.0, 0.0]);
    assert_eq!(span.forward, [Some((7, 36.5)), None]);
    assert_eq!(span.backward, [None, None]);
    assert_eq!(span.path, 1);
    assert_eq!((span.t_start, span.t_end, span.length), (0.25, 0.5, 50.0));
    assert_eq!(span.parameters, vec![Some(0.0), None, Some(1.0)]);
}

#[test]
fn a_record_count_past_the_file_is_an_error_not_a_short_table() {
    let mut file = vec![0u8; 0x100];
    let node = node(0x20, HEADER);
    put_u32(&mut file, node.payload().start, 2);
    assert!(matches!(
        spans(&file, &node),
        Err(Error::OutOfBounds { .. })
    ));
}

/// Two list-A batches, the second with the 0x80 header, then a list-B one:
/// the walk stops at the first header that does not carry the list's bit.
#[test]
fn batch_offsets_step_over_each_header_and_its_vertices() {
    let mut payload = vec![0u8; 0x200];
    put_u32(&mut payload, 4, 0x30);
    put_u16(&mut payload, 0x30, 1);
    put_u16(&mut payload, 0x30 + 0x0c, 0x10);
    let second = 0x30 + 0x40 + 0x10;
    put_u16(&mut payload, second, 1);
    payload[second + 3] = 0x40;
    put_u16(&mut payload, second + 0x0c, 0x08);
    let third = second + 0x80 + 0x08;
    put_u16(&mut payload, third, 2);
    assert_eq!(batch_offsets(&payload, 0), vec![0x30, second]);
}
