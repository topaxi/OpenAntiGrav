//! Vertex layouts: the stride and offsets derived from a vertex type, the
//! types refused, and a decoded vertex scaled into model units.
//!
//! Split out of `vex.rs`'s `#[cfg(test)] mod tests`, which was 1,108
//! lines - past the 200 an inline test module may hold, and past the 1,000
//! a file may. See `scripts/check-file-size.py`, which is the rule as a
//! gate.

use crate::vex::*;

/// Every combination the game's stride calculator can reach.
/// Values from the recovered layout table; see `docs/formats/vex.md`.
#[test]
fn derives_every_reachable_layout() {
    let cases: &[(u16, usize, usize)] = &[
        (0x100, 6, 0),
        (0x101, 8, 2),
        (0x103, 16, 8),
        (0x11c, 12, 4),
        (0x11d, 16, 8),
        (0x11f, 20, 12),
        (0x120, 10, 4),
        (0x121, 12, 6),
        (0x123, 20, 12),
        (0x13c, 16, 8),
        (0x13d, 20, 12),
        (0x13f, 24, 16),
        // 16-bit colour. `0x139` and `0x13b` are both observed in the
        // shipped tracks and both validated by the bounding-box check; the
        // three colour-only rows are the same rule applied, not sightings.
        (0x110, 8, 2),   // BGR5650
        (0x114, 8, 2),   // ABGR5551
        (0x118, 8, 2),   // ABGR4444
        (0x139, 14, 8),  // u8 texcoord, ABGR4444, s8 normal
        (0x13b, 20, 14), // f32 texcoord, ABGR4444, s8 normal
    ];

    for &(vertex_type, stride, position) in cases {
        let layout = VertexLayout::from_vertex_type(vertex_type)
            .unwrap_or_else(|e| panic!("{vertex_type:#06x}: {e}"));
        assert_eq!(layout.stride, stride, "stride for {vertex_type:#06x}");
        assert_eq!(layout.position, position, "position for {vertex_type:#06x}");
    }
}

#[test]
fn identifies_present_components() {
    let full = VertexLayout::from_vertex_type(0x13f).unwrap();
    assert_eq!(full.texcoord, Some((0, TexcoordFormat::F32)));
    assert_eq!(full.colour, Some((8, ColourFormat::Abgr8888)));
    assert_eq!(full.normal, Some(12));
    assert_eq!(full.position, 16);

    let bare = VertexLayout::from_vertex_type(0x100).unwrap();
    assert_eq!(bare.texcoord, None);
    assert_eq!(bare.colour, None);
    assert_eq!(bare.normal, None);
}

/// 0x13b is the type that made a whole track refuse to decode: f32
/// texcoords, ABGR4444 colour, s8 normals, s16 position.
#[test]
fn decodes_the_track_vertex_type() {
    let layout = VertexLayout::from_vertex_type(0x13b).expect("0x13b");
    assert_eq!(layout.texcoord, Some((0, TexcoordFormat::F32)));
    assert_eq!(layout.colour, Some((8, ColourFormat::Abgr4444)));
    assert_eq!(layout.normal, Some(10));
    assert_eq!(layout.position, 14);
    assert_eq!(layout.stride, 20);
}

/// Widening a 4-bit or 5-bit channel by shifting leaves white looking grey.
#[test]
fn sixteen_bit_colour_widens_to_full_range() {
    assert_eq!(ColourFormat::Abgr4444.to_rgba(0xffff), [255, 255, 255, 255]);
    assert_eq!(ColourFormat::Abgr4444.to_rgba(0x0000), [0, 0, 0, 0]);
    assert_eq!(ColourFormat::Abgr5551.to_rgba(0xffff), [255, 255, 255, 255]);
    assert_eq!(ColourFormat::Bgr5650.to_rgba(0xffff), [255, 255, 255, 255]);
    // Alpha is one bit in 5551, so it is either off or fully on.
    assert_eq!(ColourFormat::Abgr5551.to_rgba(0x7fff)[3], 0);
    // Channel order: red is the low bits in every ABGR format.
    assert_eq!(ColourFormat::Abgr4444.to_rgba(0x000f), [255, 0, 0, 0]);
    assert_eq!(ColourFormat::Abgr8888.to_rgba(0x0000_00ff), [255, 0, 0, 0]);
}

#[test]
fn rejects_undefined_colour_formats() {
    // Bits 2-4 of 1, 2 and 3 are not GU colour formats.
    for vertex_type in [0x104u16, 0x108, 0x10c] {
        assert!(
            matches!(
                VertexLayout::from_vertex_type(vertex_type),
                Err(Error::UnsupportedVertexType { .. })
            ),
            "{vertex_type:#06x} should be refused"
        );
    }
}

#[test]
fn rejects_u16_texcoords() {
    // The game's own calculator falls through this case without advancing
    // the offset. Guessing would shift every following field.
    assert!(matches!(
        VertexLayout::from_vertex_type(0x102),
        Err(Error::UnsupportedVertexType { .. })
    ));
}

#[test]
fn rejects_non_s16_positions() {
    // The `+ 6` in the stride formula only holds for s16 positions.
    for vertex_type in [0x000, 0x080, 0x180] {
        assert!(
            matches!(
                VertexLayout::from_vertex_type(vertex_type),
                Err(Error::UnsupportedVertexType { .. })
            ),
            "{vertex_type:#06x} should be refused"
        );
    }
}

#[test]
fn rejects_weights_and_morphs() {
    assert!(VertexLayout::from_vertex_type(0x0500).is_err());
}

#[test]
fn scales_positions_into_model_units() {
    let layout = VertexLayout::from_vertex_type(0x100).unwrap();
    let mut data = vec![0u8; 16];
    // Full-scale positive, full-scale negative, zero.
    data[0..2].copy_from_slice(&i16::MAX.to_le_bytes());
    data[2..4].copy_from_slice(&(-32768i16).to_le_bytes());
    data[4..6].copy_from_slice(&0i16.to_le_bytes());

    let v = decode_vertex(&data, 0, &layout, 100.0);
    assert!((v.position[0] - 99.997).abs() < 0.01, "{:?}", v.position);
    assert!((v.position[1] + 100.0).abs() < 0.001, "{:?}", v.position);
    assert_eq!(v.position[2], 0.0);
}

#[test]
fn a_missing_scale_would_be_obvious() {
    // Guards the mistake this format invites: forgetting the scale leaves
    // every model in the unit cube.
    let layout = VertexLayout::from_vertex_type(0x100).unwrap();
    let mut data = vec![0u8; 16];
    data[0..2].copy_from_slice(&16384i16.to_le_bytes());

    let scaled = decode_vertex(&data, 0, &layout, 250.0);
    assert!((scaled.position[0] - 125.0).abs() < 0.001);
}
