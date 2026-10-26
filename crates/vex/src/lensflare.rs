//! `lensflare` nodes: the sun's anchor and tint for the lens flare Pure draws.
//!
//! One node per circuit that has a flare (`lensflare_`, plus a child
//! `lensflare_shape_`). Only the anchor and the colour are authored; the ring
//! count, sizes, spacing, texture cells and the occlusion test are the
//! executable's (`docs/rendering/lens-flare.md`).
//!
//! ```text
//! lensflare payload, 80 bytes, Pure (class 0x390):
//!   +0x00  f32[16]  4x4 matrix, row-major; only row 3 (+0x30) is read
//!   +0x40  u32      not read by the importer (187 on Vineta K)
//!   +0x44  ..       12 bytes, not read
//! header attributes (f32, matched without case): ColourR, ColourG, ColourB
//! ```
//!
//! The importer (`Lensflare_ImportNode`, `0x0888c444`) normalises row 3 into a
//! direction; the node's transform chain is not applied, so the anchor is a
//! direction from the camera, not a point in the world.

use crate::vex::{self, Node};

/// Length of a `lensflare` payload on Pure.
pub const PAYLOAD_LEN: usize = 0x50;

/// Byte offset of the matrix's translation row.
const ROW_3: usize = 0x30;

/// A circuit's sun lens flare.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lensflare {
    /// Payload row 3 as authored, before normalising.
    pub anchor: [f32; 3],
    /// `anchor` scaled to length 1; the all-zero anchor stays zero.
    pub direction: [f32; 3],
    /// `ColourR`, `ColourG`, `ColourB`; an attribute the node lacks reads 0, as
    /// in the importer.
    pub colour: [f32; 3],
}

impl Lensflare {
    /// Decodes one payload and its header attributes.
    ///
    /// Returns `None` for a payload that is not [`PAYLOAD_LEN`] bytes.
    #[must_use]
    pub fn parse(
        payload: &[u8],
        attributes: &[(String, f32)],
        order: oag_formats::ByteOrder,
    ) -> Option<Self> {
        if payload.len() != PAYLOAD_LEN {
            return None;
        }
        let f = |at: usize| f32::from_bits(order.u32(payload, at));
        let anchor = [f(ROW_3), f(ROW_3 + 4), f(ROW_3 + 8)];
        let length = (anchor[0] * anchor[0] + anchor[1] * anchor[1] + anchor[2] * anchor[2]).sqrt();
        let direction = if length == 0.0 {
            [0.0; 3]
        } else {
            [anchor[0] / length, anchor[1] / length, anchor[2] / length]
        };
        let channel = |name: &str| {
            attributes
                .iter()
                .find(|(n, _)| n.eq_ignore_ascii_case(name))
                .map_or(0.0, |(_, v)| *v)
        };
        Some(Self {
            anchor,
            direction,
            colour: [channel("ColourR"), channel("ColourG"), channel("ColourB")],
        })
    }
}

/// Every `lensflare` node of a file, in node order. Empty where the file's
/// class table has no `lensflare` id or the file authors none.
#[must_use]
pub fn lensflares(data: &[u8], nodes: &[Node]) -> Vec<Lensflare> {
    let Some(class) = vex::classes_of(data).ok().and_then(|c| c.lensflare) else {
        return Vec::new();
    };
    let order = vex::byte_order(data);
    nodes
        .iter()
        .filter(|n| n.class_id == class)
        .filter_map(|n| {
            let payload = data.get(n.payload())?;
            Lensflare::parse(payload, &vex::node_attributes(data, n), order)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_formats::ByteOrder;

    fn payload(row3: [f32; 3]) -> Vec<u8> {
        let mut out = vec![0_u8; PAYLOAD_LEN];
        for (i, v) in row3.into_iter().enumerate() {
            out[ROW_3 + i * 4..ROW_3 + i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        out
    }

    #[test]
    fn the_anchor_is_row_three_and_the_direction_is_unit_length() {
        let flare = Lensflare::parse(&payload([3.0, 0.0, 4.0]), &[], ByteOrder::Little)
            .expect("80 bytes parse");
        assert_eq!(flare.anchor, [3.0, 0.0, 4.0]);
        assert_eq!(flare.direction, [0.6, 0.0, 0.8]);
    }

    #[test]
    fn colour_attributes_match_without_case_and_default_to_zero() {
        let attrs = vec![("colourr".to_string(), 0.5), ("ColourB".to_string(), 0.25)];
        let flare = Lensflare::parse(&payload([1.0, 0.0, 0.0]), &attrs, ByteOrder::Little).unwrap();
        assert_eq!(flare.colour, [0.5, 0.0, 0.25]);
    }

    #[test]
    fn a_payload_of_another_length_is_refused() {
        assert!(Lensflare::parse(&[0; 64], &[], ByteOrder::Little).is_none());
    }
}
