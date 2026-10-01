//! `Camera` `0xf7`: the camera an artist left in a scene.
//!
//! Every Wipeout HD front-end flyer (`Data/FE/Flyers/<name>/flyer.vex` and
//! its `flyer_back.vex`) carries one, a `Transform` named `camera1` parenting a
//! `Camera` named `cameraShape1`, exported straight out of Maya. Billboards, a
//! few ships, the front-end scene and the circuits' own `track.vex` author some
//! too (121 of the disc's `.vex` files in all); only the flyers' are read.
//!
//! ```text
//! Camera, 48 bytes, big-endian on a PS3 file:
//!   +0x00  u32   0x0001_0000  (the same on every flyer front)
//!   +0x04  u32   0x20
//!   +0x08  u32   0x22
//!   +0x0c  f32   1/60         (0.016667, the same on every flyer front)
//!   +0x10  f32[3] 0 on a flyer; on a circuit's `track.vex` the camera's AIM POINT, world space
//!   +0x1c  f32   1.5380 on the eight base grids, 1.5389 on the eight Fury
//!                grids, 1.0833 on `Fury_Campaign` and `HD_Campaign`
//!   +0x20  u32   0x4f15, 0x4a2c, 0x3621 respectively
//!   +0x24  u32[3] 0
//! ```
//!
//! # What is read and what is not
//!
//! **Placement is the transform chain**, the way [`crate::lighting`]'s is: the
//! payload carries no matrix, so [`Camera::to_world`] is the node's entry of
//! [`vex::world_transforms`]. On the 18 flyers' front files it is a pure
//! translation `(0, 0, z)` with `z` `92.4957` on the eight base grids and
//! `12.0` on the other ten, and that translation is what this module is for:
//! it is the distance the artist framed the card from. (A `flyer_back.vex`
//! frames itself from a slightly different distance, 97.15 on `01_uplift`.)
//!
//! **`+0x1c` and `+0x20` are not decoded.** `+0x1c` equals each card's own
//! width over its height to four digits (the base body is 115 by 74.8 units,
//! 1.5374; the campaign cards 22.1 by 20.4, 1.083), which reads as an aspect
//! ratio. Read as a horizontal field of view at 16:9 it also comes out at
//! 0.997 rad, within 0.3% of the vertical field of view of 1.0 rad
//! `Flyer_Item`'s render function hard-codes (`tanf(0.5)`) - a coincidence, or
//! a second meaning: the reading holds on the base and Fury grids and not on
//! the campaign cards (0.653 rad against the same constant). `+0x20` moves with
//! how much of the camera's image a flyer's card shows: the two windows fitted
//! on the base and Fury grids are in the ratio 1.081, the two words in the
//! ratio 1.066. Both are carried ([`Camera::value_1c`], [`Camera::value_20`])
//! and neither is interpreted. `docs/ui/campaign-screens.md`, "The flyer
//! behind `Grid Selection`".

use crate::vex::{self, Node};
use oag_formats::ByteOrder;

/// Class ID of a `Camera` node.
pub const CLASS_CAMERA: u32 = 0xf7;

/// Length of a `Camera` payload.
pub const PAYLOAD_LEN: usize = 0x30;

/// One `Camera` node.
#[derive(Debug, Clone, PartialEq)]
pub struct Camera {
    /// The node's name, `cameraShape1` on every shipped file.
    pub name: Option<String>,
    /// The node's own world transform, row-major with the translation in row
    /// 3, from [`vex::world_transforms`].
    pub to_world: [f32; 16],
    /// The three `f32`s at `+0x10`: **zero on a flyer, the camera's aim point in world
    /// space on a circuit's `track.vex`**. Read live 2026-10-01 off the original's own
    /// camera node list (`cam+0x40`, ten nodes on `16_Track`): the engine keeps it at
    /// `node+0xa0` and picks the spectator camera's node by how near the subject is to
    /// it - the node's eye is `to_world`'s translation and sits elsewhere. See
    /// `docs/ghidra/functions/psp-pulse-usa/race-finish.md`.
    pub target: [f32; 3],
    /// The `f32` at `+0x1c`. Not interpreted - see the module docs.
    pub value_1c: f32,
    /// The `u32` at `+0x20`. Not interpreted - see the module docs.
    pub value_20: u32,
}

impl Camera {
    /// Decodes one payload against a world matrix from its transform chain.
    ///
    /// `None` for a payload shorter than [`PAYLOAD_LEN`].
    #[must_use]
    pub fn parse(
        name: Option<String>,
        payload: &[u8],
        to_world: [f32; 16],
        order: ByteOrder,
    ) -> Option<Self> {
        if payload.len() < PAYLOAD_LEN {
            return None;
        }
        Some(Self {
            name,
            to_world,
            target: [
                f32::from_bits(order.u32(payload, 0x10)),
                f32::from_bits(order.u32(payload, 0x14)),
                f32::from_bits(order.u32(payload, 0x18)),
            ],
            value_1c: f32::from_bits(order.u32(payload, 0x1c)),
            value_20: order.u32(payload, 0x20),
        })
    }

    /// Where the camera sits, `to_world`'s translation row.
    #[must_use]
    pub fn position(&self) -> [f32; 3] {
        [self.to_world[12], self.to_world[13], self.to_world[14]]
    }
}

/// Every `Camera` in a `.vex`, in tree order.
///
/// A payload that will not decode is left out rather than guessed at; a file
/// with no camera answers an empty list.
#[must_use]
pub fn cameras(data: &[u8]) -> Vec<Camera> {
    let Ok(nodes) = vex::nodes(data) else {
        return Vec::new();
    };
    let order = vex::byte_order(data);
    let world = vex::world_transforms(data, &nodes);
    nodes
        .iter()
        .zip(&world)
        .filter(|(node, _)| node.class_id == CLASS_CAMERA)
        .filter_map(|(node, to_world)| parse_node(data, node, *to_world, order))
        .collect()
}

fn parse_node(data: &[u8], node: &Node, to_world: [f32; 16], order: ByteOrder) -> Option<Camera> {
    Camera::parse(
        node.name.clone(),
        data.get(node.payload())?,
        to_world,
        order,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(value: f32) -> Vec<u8> {
        let mut bytes = vec![0u8; PAYLOAD_LEN];
        bytes[0x1c..0x20].copy_from_slice(&value.to_be_bytes());
        bytes[0x20..0x24].copy_from_slice(&0x4f15u32.to_be_bytes());
        bytes
    }

    #[test]
    fn reads_the_value_at_0x1c_in_the_files_byte_order() {
        let camera = Camera::parse(
            Some("cameraShape1".to_string()),
            &payload(1.538),
            vex::matrix::IDENTITY,
            ByteOrder::Big,
        )
        .expect("a full payload");
        assert_eq!(camera.value_1c, 1.538);
        assert_eq!(camera.value_20, 0x4f15);
        assert_eq!(camera.position(), [0.0, 0.0, 0.0]);
    }

    #[test]
    fn the_aim_point_is_the_three_floats_at_0x10() {
        let mut bytes = payload(1.5);
        for (index, value) in [-233.88_f32, -74.62, 53.65].into_iter().enumerate() {
            bytes[0x10 + 4 * index..0x14 + 4 * index].copy_from_slice(&value.to_be_bytes());
        }
        let camera =
            Camera::parse(None, &bytes, vex::matrix::IDENTITY, ByteOrder::Big).expect("parses");
        assert_eq!(camera.target, [-233.88, -74.62, 53.65]);
        let flyer = Camera::parse(None, &payload(1.5), vex::matrix::IDENTITY, ByteOrder::Big)
            .expect("parses");
        assert_eq!(flyer.target, [0.0; 3], "a flyer authors zeros there");
    }

    #[test]
    fn a_short_payload_is_refused() {
        assert!(
            Camera::parse(
                None,
                &[0u8; PAYLOAD_LEN - 1],
                vex::matrix::IDENTITY,
                ByteOrder::Big
            )
            .is_none()
        );
    }

    #[test]
    fn the_position_is_the_translation_row() {
        let mut to_world = vex::matrix::IDENTITY;
        to_world[14] = 92.4957;
        let camera = Camera::parse(None, &payload(0.0), to_world, ByteOrder::Big).expect("parses");
        assert_eq!(camera.position(), [0.0, 0.0, 92.4957]);
    }
}
