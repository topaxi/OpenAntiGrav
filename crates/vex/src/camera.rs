//! `Camera` `0xf7`: the camera an artist left in a scene.
//!
//! Every Wipeout HD front-end flyer (`Data/FE/Flyers/<name>/flyer.vex` and
//! `flyer_back.vex`) carries one: a `Transform` named `camera1` parenting a
//! `Camera` named `cameraShape1`, exported out of Maya. Billboards, a few ships,
//! the front-end scene and circuits' `track.vex` author some too (121 of the
//! disc's `.vex` files); only the flyers' are read.
//!
//! ```text
//! Camera, 48 bytes, big-endian on a PS3 file:
//!   +0x00  u32   0x0001_0000  (the same on every flyer front)
//!   +0x04  u32   0x20
//!   +0x08  u32   0x22
//!   +0x0c  f32   1/60         (0.016667, the same on every flyer front)
//!   +0x10  f32[3] 0 on a Wipeout HD flyer; the AIM POINT in world space on a
//!                Wipeout Pulse circuit (see "The aim point" below)
//!   +0x1c  f32   1.5380 on the eight base grids, 1.5389 on the eight Fury
//!                grids, 1.0833 on `Fury_Campaign` and `HD_Campaign`
//!   +0x20  u32   0x4f15, 0x4a2c, 0x3621 respectively
//!   +0x24  u32[3] 0
//! ```
//!
//! # The aim point
//!
//! **On a Wipeout Pulse circuit's `track.vex` the three floats at `+0x10` are a
//! world-space point the artist aimed the camera at** ([`Camera::aim`]); the
//! position beside it is the node's own transform translation. Read off Talon's
//! Junction (`16_Track`, ten cameras) and checked against the running original:
//! the camera fixed on a wrecked player craft (`FUN_0887fedc`) keeps its eye at
//! the node's world translation (`468.3436, -22.8052, -39.8696`, exactly) and
//! picks the one whose **aim point** is nearest the craft, the `+0xa0` of its
//! runtime object and this payload's `+0x10..+0x1c` (`344.1187, -43.1941,
//! -137.0839`, exactly). The aim is already in world space, not run through
//! [`Camera::to_world`]. See `docs/ghidra/functions/psp-pulse-usa/camera.md`,
//! "The destroy camera".
//!
//! # The field of view (2026-10-06, Pulse PSP)
//!
//! **`+0x00` is a key count, `+0x02` a flag byte (bit 0 set: orthographic), and
//! `+0x04` / `+0x08` are offsets into the payload of the key times and the key
//! values, both `u16`**; `VexCamera_BuildProjection` (`0x08901dc4`,
//! `docs/ghidra/functions/psp-pulse-usa/billboards.md`) evaluates the curve
//! and scales a value by `180 / 65535` to a **horizontal** field of view in
//! degrees. Every shipped billboard-advert camera has one key (offsets `0x20`,
//! `0x22`), so [`Camera::fov_degrees`] reads the value at the second offset and
//! answers `None` for any curve it would have to interpolate. Confirmed against
//! the GE projection matrix of the running original: `0x5080` gives 56.60
//! degrees and an x scale of `1/tan(28.30 deg)` = 1.857, the matrix word the
//! capture holds, with `+0x1c` = 2.0 as the y/x ratio (3.714).
//!
//! # What is read and what is not
//!
//! **Placement is the transform chain**, as in [`crate::lighting`]: the payload
//! has no matrix, so [`Camera::to_world`] is the node's entry of
//! [`vex::world_transforms`]. On the 18 flyers' front files it is a pure
//! translation `(0, 0, z)`, `z` `92.4957` on the eight base grids and `12.0` on
//! the other ten: the distance the artist framed the card from (a
//! `flyer_back.vex` frames from a slightly different one, 97.15 on `01_uplift`).
//!
//! **`+0x1c` and `+0x20` are not decoded.** `+0x1c` equals each card's width over
//! height to four digits (base body 115 by 74.8, 1.5374; campaign cards 22.1 by
//! 20.4, 1.083), reading as an aspect ratio. Read as a horizontal field of view
//! at 16:9 it is 0.997 rad, within 0.3% of the 1.0 rad vertical field of view
//! `Flyer_Item`'s render function hard-codes (`tanf(0.5)`): a coincidence or a
//! second meaning, holding on the base and Fury grids but not the campaign cards
//! (0.653 rad). `+0x20` moves with how much of the camera's image a card shows
//! (windows fitted on the base and Fury grids are in ratio 1.081, the two words
//! 1.066). Both are carried ([`Camera::value_1c`], [`Camera::value_20`]) and
//! neither interpreted; see `docs/ui/campaign-screens.md`, "The flyer behind
//! `Grid Selection`".

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
    /// The three `f32`s at `+0x10`: the point the camera was aimed at, in world
    /// space, on a Pulse circuit; zero on a Wipeout HD flyer. See the module docs.
    pub aim: [f32; 3],
    /// The `f32` at `+0x1c`. Not interpreted - see the module docs.
    pub value_1c: f32,
    /// The `u32` at `+0x20`. Not interpreted - see the module docs.
    pub value_20: u32,
    /// The curve's key count, the `u16` at `+0x00`.
    pub fov_keys: u16,
    /// The curve's flag byte at `+0x02`; bit 0 selects the orthographic branch.
    pub fov_flags: u8,
    /// The single key's value, when the curve has exactly one: the `u16` at the
    /// offset `+0x08` names. See the module docs.
    pub fov_value: Option<u16>,
}

impl Camera {
    /// Decodes one payload against a world matrix from its transform chain;
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
        let fov_keys = order.u16(payload, 0);
        let values_at = order.u32(payload, 0x08) as usize;
        let fov_value = (fov_keys == 1 && values_at + 2 <= payload.len())
            .then(|| order.u16(payload, values_at));
        Some(Self {
            fov_keys,
            fov_flags: payload[2],
            fov_value,
            name,
            to_world,
            aim: [
                f32::from_bits(order.u32(payload, 0x10)),
                f32::from_bits(order.u32(payload, 0x14)),
                f32::from_bits(order.u32(payload, 0x18)),
            ],
            value_1c: f32::from_bits(order.u32(payload, 0x1c)),
            value_20: order.u32(payload, 0x20),
        })
    }

    /// The horizontal field of view in degrees, for a one-key perspective
    /// curve; `None` for an orthographic camera or a curve with more than one
    /// key, which this does not interpolate.
    #[must_use]
    pub fn fov_degrees(&self) -> Option<f32> {
        if self.fov_flags & 1 != 0 {
            return None;
        }
        self.fov_value.map(|v| f32::from(v) * 180.0 / 65535.0)
    }

    /// The frustum's width over its height, the `f32` at `+0x1c`.
    #[must_use]
    pub fn aspect(&self) -> f32 {
        self.value_1c
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

    /// Talon's Junction's eighth camera: its payload's `+0x10` is the aim point
    /// the running original keeps at `+0xa0` of the camera it fixes on a wreck.
    #[test]
    fn the_aim_point_is_the_three_floats_at_0x10() {
        let mut bytes = payload(1.538);
        for (i, v) in [344.1187_f32, -43.1941, -137.0839].iter().enumerate() {
            bytes[0x10 + 4 * i..0x14 + 4 * i].copy_from_slice(&v.to_le_bytes());
        }
        let camera = Camera::parse(None, &bytes, vex::matrix::IDENTITY, ByteOrder::Little)
            .expect("a full payload");
        assert_eq!(camera.aim, [344.1187, -43.1941, -137.0839]);
    }

    /// The Auricom advert's camera as it ships on Pulse PSP: one key, value
    /// `0x5080`, aspect 2.0 - the GE projection of the running original has an x
    /// scale of 1.857 and a y scale of 3.714.
    #[test]
    fn a_one_key_curve_gives_the_horizontal_fov_the_projection_carries() {
        let mut bytes = vec![0u8; PAYLOAD_LEN];
        bytes[0..4].copy_from_slice(&1u32.to_le_bytes());
        bytes[4..8].copy_from_slice(&0x20u32.to_le_bytes());
        bytes[8..12].copy_from_slice(&0x22u32.to_le_bytes());
        bytes[0x1c..0x20].copy_from_slice(&2.0f32.to_le_bytes());
        bytes[0x22..0x24].copy_from_slice(&0x5080u16.to_le_bytes());
        let camera = Camera::parse(None, &bytes, vex::matrix::IDENTITY, ByteOrder::Little)
            .expect("a full payload");
        let fov = camera.fov_degrees().expect("one key");
        assert!((fov - 56.60).abs() < 0.01, "{fov}");
        let x_scale = 1.0 / (fov.to_radians() / 2.0).tan();
        assert!((x_scale - 1.857).abs() < 0.001, "{x_scale}");
        assert!((x_scale * camera.aspect() - 3.714).abs() < 0.002);
    }

    #[test]
    fn an_orthographic_or_multi_key_curve_has_no_fov() {
        let mut bytes = vec![0u8; PAYLOAD_LEN];
        bytes[0..4].copy_from_slice(&1u32.to_le_bytes());
        bytes[8..12].copy_from_slice(&0x22u32.to_le_bytes());
        bytes[2] = 1;
        let ortho = Camera::parse(None, &bytes, vex::matrix::IDENTITY, ByteOrder::Little).unwrap();
        assert_eq!(ortho.fov_degrees(), None);
        bytes[2] = 0;
        bytes[0..2].copy_from_slice(&2u16.to_le_bytes());
        let two = Camera::parse(None, &bytes, vex::matrix::IDENTITY, ByteOrder::Little).unwrap();
        assert_eq!(two.fov_degrees(), None);
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
