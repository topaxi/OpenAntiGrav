//! PS2 batches: the VIF command stream decoded into vertices, and the packets
//! refused.

use crate::vex::*;

/// Builds a PS2 batch: a 0x40 header, then the region header, DMA tag and VIF
/// command stream a real one carries. Hand-built (a real batch is game data), in
/// the shape `crates/vex/tests/vex_ps2_ground_truth.rs` checks against every
/// model on the disc, keeping the same paths covered in CI.
fn ps2_batch(
    vertex_type: u16,
    primitive_type: u8,
    chunks: &[Vec<[f32; 3]>],
    with_normal: bool,
) -> Vec<u8> {
    let code = |command: u8, num: u8, immediate: u16| -> [u8; 4] {
        (u32::from(command) << 24 | u32::from(num) << 16 | u32::from(immediate)).to_le_bytes()
    };

    let mut stream = Vec::new();
    for positions in chunks {
        let count = u8::try_from(positions.len()).expect("fixture chunk fits in NUM");
        // The setup quadword, whose low 15 bits are the chunk's count.
        stream.extend(code(0x01, 0, 0x0101));
        stream.extend(code(0x6c, 1, 0xc000));
        stream.extend(u32::from(0x8000 | u16::from(count)).to_le_bytes());
        stream.extend([0u8; 12]);
        stream.extend(code(0x01, 0, 0x0104));
        // Colour, then texture coordinates, then position, then normal:
        // the order a real packet uses.
        stream.extend(code(0x6e, count, 0xc005));
        for i in 0..positions.len() {
            stream.extend([64, 64, 64, u8::try_from(i % 128).unwrap()]);
        }
        stream.extend(code(0x74, count, 0xc006));
        for (i, _) in positions.iter().enumerate() {
            stream.extend((i as f32).to_le_bytes());
            stream.extend((-(i as f32)).to_le_bytes());
        }
        stream.extend(code(0x78, count, 0xc004));
        for p in positions {
            for c in p {
                stream.extend(c.to_le_bytes());
            }
        }
        if with_normal {
            stream.extend(code(0x78, count, 0xc007));
            for _ in positions {
                stream.extend(0.0f32.to_le_bytes());
                stream.extend(1.0f32.to_le_bytes());
                stream.extend(0.0f32.to_le_bytes());
            }
        }
        stream.extend(code(0x17, 0, 0));
    }
    // The packet is the tag quadword plus the stream, which the tag's two
    // command words are already part of.
    while (stream.len() + 8) % 16 != 0 {
        stream.extend(code(0x00, 0, 0));
    }
    let packet_len = 16 + stream.len() - 8;
    let qwc = (packet_len - 16) / 16;

    let declared: usize = chunks.iter().map(Vec::len).sum::<usize>()
        - 2 * chunks.len().saturating_sub(1) * usize::from(primitive_type == PRIM_TRIANGLE_STRIP);

    let mut header = vec![0u8; 0x40];
    header[0..2].copy_from_slice(&1u16.to_le_bytes()); // pass_mask, list A
    header[4..6].copy_from_slice(&(declared as u16).to_le_bytes());
    header[8] = primitive_type;
    header[0x0a..0x0c].copy_from_slice(&vertex_type.to_le_bytes());
    header[0x0c..0x0e].copy_from_slice(&((16 + packet_len) as u16).to_le_bytes());
    header[0x10..0x14].copy_from_slice(&1.0f32.to_le_bytes());
    // The f32 bounding box, wide enough for the fixture's positions.
    for i in 0..3 {
        header[0x20 + i * 4..0x24 + i * 4].copy_from_slice(&(-1000.0f32).to_le_bytes());
        header[0x30 + i * 4..0x34 + i * 4].copy_from_slice(&1000.0f32.to_le_bytes());
    }

    let mut out = header;
    out.extend((packet_len as u32).to_le_bytes());
    out.extend(u32::from(vertex_type).to_le_bytes());
    out.extend(1u32.to_le_bytes()); // pass mask again
    out.extend(0u32.to_le_bytes());
    out.extend((0x6000_0000u32 | qwc as u32).to_le_bytes()); // DMA tag
    out.extend(0u32.to_le_bytes());
    out.extend(stream);
    out
}

/// Wraps batches into a mesh payload: the header, then batch list A.
fn ps2_mesh(batches: &[Vec<u8>]) -> Vec<u8> {
    let mut payload = vec![0u8; 0x30];
    payload[4..8].copy_from_slice(&0x30u32.to_le_bytes()); // list A offset
    payload[8..12].copy_from_slice(&0x30u32.to_le_bytes());
    for batch in batches {
        payload.extend(batch);
    }
    // A terminator: a batch header whose pass mask has neither list bit.
    payload.extend(vec![0u8; 0x40]);
    payload
}

#[test]
fn a_vif_batch_is_selected_by_its_position_width() {
    // The PSP's twelve types are not VIF batches; the PS2's eight are.
    for psp in [0x100u16, 0x121, 0x139, 0x13d, 0x13f] {
        assert!(!is_vif_batch(psp), "{psp:#06x}");
    }
    for ps2 in [0x181u16, 0x183, 0x199, 0x19b, 0x1a1, 0x1a3, 0x1b9, 0x1bb] {
        assert!(is_vif_batch(ps2), "{ps2:#06x}");
    }
}

#[test]
fn decodes_a_ps2_batch_into_vertices() {
    let positions = vec![[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]];
    let payload = ps2_mesh(&[ps2_batch(
        0x1b9,
        PRIM_TRIANGLES,
        std::slice::from_ref(&positions),
        true,
    )]);
    let batches = mesh_batches(&payload, 0).expect("decodes");
    assert_eq!(batches.len(), 1);
    let batch = &batches[0];

    assert_eq!(batch.vertex_type, 0x1b9);
    assert_eq!(batch.vertices.len(), 3);
    assert_eq!(batch.declared_vertex_count, 3);
    for (vertex, expected) in batch.vertices.iter().zip(&positions) {
        assert_eq!(vertex.position, *expected);
    }
    // The bounding box is the f32 pair at +0x20 and +0x30, not the PSP's
    // s16 pair scaled: reading the wrong one gives zeros here.
    assert_eq!(batch.bounds, ([-1000.0; 3], [1000.0; 3]));
    assert_eq!(batch.vertices[1].texcoord, Some([1.0, -1.0]));
    assert_eq!(batch.vertices[0].normal, Some([0.0, 1.0, 0.0]));
    // 64 of 128 is half intensity, which widens to half of 255.
    assert_eq!(batch.vertices[0].colour, Some([127, 127, 127, 0]));
}

/// A strip split across chunks repeats two vertices, so the decoded count
/// exceeds the declared one. Concatenating them is what makes the repeats
/// zero-area triangles rather than a hole.
#[test]
fn a_split_strip_keeps_both_chunks() {
    let first: Vec<[f32; 3]> = (0..6).map(|i| [i as f32, 0.0, 0.0]).collect();
    let second: Vec<[f32; 3]> = (4..9).map(|i| [i as f32, 0.0, 0.0]).collect();
    let payload = ps2_mesh(&[ps2_batch(
        0x199,
        PRIM_TRIANGLE_STRIP,
        &[first.clone(), second.clone()],
        false,
    )]);
    let batch = &mesh_batches(&payload, 0).expect("decodes")[0];

    assert_eq!(batch.vertices.len(), first.len() + second.len());
    assert_eq!(
        usize::from(batch.declared_vertex_count),
        first.len() + second.len() - 2,
        "the header counts the strip, not the repeats"
    );
    assert_eq!(batch.vertices[6].position, [4.0, 0.0, 0.0]);
    assert!(
        batch.vertices[0].normal.is_none(),
        "0x199 declares no normal, so none may be decoded"
    );
}

#[test]
fn a_packet_whose_length_disagrees_with_the_batch_is_refused() {
    let mut payload = ps2_mesh(&[ps2_batch(
        0x1b9,
        PRIM_TRIANGLES,
        &[vec![[0.0, 0.0, 0.0]; 3]],
        true,
    )]);
    // Shorten the packet's own declared length by one quadword.
    let size = u32_at(&payload, 0x30 + 0x40) - 16;
    payload[0x30 + 0x40..0x30 + 0x44].copy_from_slice(&size.to_le_bytes());
    assert!(matches!(
        mesh_batches(&payload, 0),
        Err(Error::Packet { .. })
    ));
}

#[test]
fn a_missing_normal_array_is_refused_when_the_type_declares_one() {
    // The one attribute whose presence the vertex type really does predict.
    let payload = ps2_mesh(&[ps2_batch(
        0x1b9,
        PRIM_TRIANGLES,
        &[vec![[0.0, 0.0, 0.0]; 3]],
        false,
    )]);
    assert!(matches!(
        mesh_batches(&payload, 0),
        Err(Error::Packet { .. })
    ));
}

#[test]
fn colour_and_texcoords_are_taken_as_found_whatever_the_type_says() {
    // 0x181 declares no colour at all, and every real PS2 chunk carries one
    // anyway. Refusing it here would reject 930 batches on the disc.
    let payload = ps2_mesh(&[ps2_batch(
        0x181,
        PRIM_TRIANGLES,
        &[vec![[0.0, 0.0, 0.0]; 3]],
        false,
    )]);
    let batch = &mesh_batches(&payload, 0).expect("decodes")[0];
    assert!(batch.vertices[0].colour.is_some());
    assert!(batch.vertices[0].texcoord.is_some());
}

#[test]
fn rejects_a_short_file() {
    assert_eq!(version(&[0u8; 4]), Err(Error::TooShort { got: 4 }));
    assert_eq!(nodes(&[0u8; 4]), Err(Error::TooShort { got: 4 }));
}
