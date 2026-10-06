//! `AmbientLight` `0x12c`, `DirectionalLight` `0x131` and `PointLight` `0x132`:
//! the track's authored light rig.
//!
//! All three are small, fixed-length payloads - 16 bytes for the first two, 32
//! for `PointLight` - and none of them is a [`vex::Mesh`](vex::CLASS_MESH)
//! payload the way [`Skycube`](vex::CLASS_SKYCUBE) or
//! [`Speedup Pad`](vex::CLASS_SPEEDUP_PAD) turned out to be: both are far
//! shorter than [`vex::mesh_materials`]'s `len >= 0x30` precondition, so there
//! is no batch-decoder reuse trick here, only three small standalone parsers.
//! See [`crate::pads`] and `docs/formats/skycube.md` for the classes where reuse
//! *did* apply, and `docs/formats/lighting.md` for the evidence and confidence
//! scores behind everything in this module.
//!
//! ```text
//! AmbientLight / DirectionalLight, 16 bytes:
//!   +0x00  f32[3]  colour, linear RGB
//!   +0x0c  f32     intensity
//!
//! PointLight, 32 bytes:
//!   +0x00  f32[3]  colour, linear RGB
//!   +0x0c  f32     range
//!   +0x10  u32[4]  trailer, undecoded; see below for what each disc holds
//! ```
//!
//! Every field is in the file's own byte order, from
//! [`vex::byte_order`](crate::vex::byte_order).
//!
//! # Placement is the transform chain, not the payload
//!
//! None of these three payloads carries a matrix, unlike a locator class such
//! as [`Engine Flare`](vex::CLASS_ENGINE_FLARE) or
//! [`Start Position`](vex::CLASS_START_POSITION). So a light's `to_world` comes
//! from [`vex::world_transforms`] - the full per-node chain, which contributes
//! the identity for any node that is not itself a
//! [`Transform`](vex::CLASS_TRANSFORM) - the same source
//! [`pads::volumes`](crate::pads::volumes) uses for exactly the same reason.
//!
//! **Not [`vex::class_world_transforms`].** That helper reads a matching
//! node's *own* payload as the 64-byte matrix, which is right for a locator
//! class and wrong here: handed a 16- or 32-byte light payload, its
//! `vex::transform` call sees a non-empty payload under 64 bytes, returns
//! `None`, and the light is silently `filter_map`'d out of the result. The
//! failure mode is not a wrong matrix, it is an empty `Vec` - every light on
//! every track missing at once, which is exactly the trap
//! [`pads::volumes`](crate::pads::volumes)'s own doc comment already names for
//! the mesh-shaped pad payload.
//!
//! # Wipeout HD authors these too, and read little-endian they are denormals
//!
//! 17 `AmbientLight`, 17 `DirectionalLight` and **1,160 `PointLight`** nodes
//! across HD's 742 `.vex` files - so this module is not a Pulse-only one and
//! reading a `f32` little-endian here was a live bug, not a latent one.
//!
//! It is the quiet kind. `start_grid.vex`'s point light is `3f 80 00 00` four
//! times over: white at range 1.0 big-endian, and `4.6006e-41` in all four
//! fields little-endian. A denormal, not a `NaN` and not an error - it would
//! have travelled all the way to a shader as a light that contributes nothing
//! visible. The matrix beside it came out *right* the whole time, because
//! [`vex::world_transforms`] has always sniffed the file's order, which is what
//! made the wrong half look plausible.
//!
//! The order is sniffed once per file by the three collectors and handed to
//! `parse`, the shape [`pads::volumes`](crate::pads::volumes) already uses.
//!
//! # The trailer and the intensity range are both open
//!
//! `PointLight`'s second 16 bytes decode as four `u32` and read `{1, 0, 0, 0}`
//! on every Pulse sample seen so far. Nothing here reads what they mean; they
//! are carried on [`PointLight::trailer`] rather than dropped, the same
//! judgement call [`pads::PadVolume::disabled`](crate::pads::PadVolume::disabled)
//! already makes for a field that is provably present and not provably inert.
//!
//! HD's 1,160 point lights split `{0, 0, 0, 0}` on 1,151 and
//! `{0x0001_0000, 0, 0, 0}` on 9. **If the first word were a `u16` pair rather
//! than a `u32`, those nine would read 1 - the value Pulse's carry.** That is
//! noted and deliberately not acted on: nine samples against one alternative
//! reading is a hypothesis, the field is undecoded either way, and re-typing it
//! on that basis would turn a coincidence into a claim. `u32` is what both
//! discs are read as until something in the runtime says otherwise.
//!
//! Intensities up to 5.7854 have been observed, so `colour`/`intensity` and
//! `colour`/`range` are **not** clamped to `0..=1` here the way
//! [`fog::FogParams::colour`](crate::fog::FogParams::colour) is - see
//! `docs/formats/lighting.md`'s Open section for the tension with an 8-bit GE
//! colour register that this raises and does not resolve.

use crate::vex::{self, Node};
use oag_formats::ByteOrder;

/// Length of an `AmbientLight`/`DirectionalLight` payload.
pub const AMBIENT_OR_DIRECTIONAL_PAYLOAD_LEN: usize = 0x10;

/// Length of a `PointLight` payload.
pub const POINT_PAYLOAD_LEN: usize = 0x20;

/// Reads the four `f32` at the front of `payload`, in the file's own order.
///
/// Shared by [`AmbientLight::parse`], [`DirectionalLight::parse`] and
/// [`PointLight::parse`], all of which start with the same four-float shape
/// before their payload lengths diverge.
fn read_four_f32(payload: &[u8], order: ByteOrder) -> Option<[f32; 4]> {
    if payload.len() < 0x10 {
        return None;
    }
    let f = |i: usize| f32::from_bits(order.u32(payload, i * 4));
    Some([f(0), f(1), f(2), f(3)])
}

/// A flat colour added everywhere, with no position or direction of its own.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AmbientLight {
    /// Linear RGB, as authored. Not clamped to `0..=1` - see the module docs.
    pub colour: [f32; 3],
    /// Scale on `colour`, applied at an unrecovered point in the original's
    /// pipeline. Not clamped.
    pub intensity: f32,
    /// This node's own world transform, from [`vex::world_transforms`].
    ///
    /// The payload carries no matrix, so this is placement in name only for an
    /// ambient light's own colour contribution - but it is what would
    /// disambiguate one ambient light from another if a track ever authored
    /// more than one, and every other light class here carries the same field.
    pub to_world: [f32; 16],
}

impl AmbientLight {
    /// Decodes one `AmbientLight` payload against a world matrix from its
    /// transform chain.
    ///
    /// Returns `None` for a payload shorter than
    /// [`AMBIENT_OR_DIRECTIONAL_PAYLOAD_LEN`].
    #[must_use]
    pub fn parse(payload: &[u8], to_world: [f32; 16], order: ByteOrder) -> Option<Self> {
        if payload.len() < AMBIENT_OR_DIRECTIONAL_PAYLOAD_LEN {
            return None;
        }
        let f = read_four_f32(payload, order)?;
        Some(Self {
            colour: [f[0], f[1], f[2]],
            intensity: f[3],
            to_world,
        })
    }
}

/// A parallel light with no position, only a direction.
///
/// Byte-identical payload shape to [`AmbientLight`]; only the class ID and,
/// presumably, the runtime's handling of `to_world` differ. **Whether the
/// direction the original draws with comes from this matrix's rotation basis
/// or its translation is not settled here** - see `docs/formats/lighting.md`'s
/// Open section and the direction-basis survey in
/// `crates/vex/tests/lighting_ground_truth.rs`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectionalLight {
    /// Linear RGB, as authored. Not clamped to `0..=1` - see the module docs.
    pub colour: [f32; 3],
    /// Scale on `colour`. Not clamped.
    pub intensity: f32,
    /// This node's own world transform, from [`vex::world_transforms`]. Which
    /// part of it the runtime reads as "direction" is open; see above.
    pub to_world: [f32; 16],
}

impl DirectionalLight {
    /// Decodes one `DirectionalLight` payload against a world matrix from its
    /// transform chain.
    ///
    /// Returns `None` for a payload shorter than
    /// [`AMBIENT_OR_DIRECTIONAL_PAYLOAD_LEN`].
    #[must_use]
    pub fn parse(payload: &[u8], to_world: [f32; 16], order: ByteOrder) -> Option<Self> {
        if payload.len() < AMBIENT_OR_DIRECTIONAL_PAYLOAD_LEN {
            return None;
        }
        let f = read_four_f32(payload, order)?;
        Some(Self {
            colour: [f[0], f[1], f[2]],
            intensity: f[3],
            to_world,
        })
    }
}

/// A light that falls off with distance from its position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointLight {
    /// Linear RGB, as authored. Not clamped to `0..=1` - see the module docs.
    pub colour: [f32; 3],
    /// Falloff distance. Not clamped; expected finite and positive, checked in
    /// the ground-truth test rather than enforced here, the same trade
    /// [`fog::FogVolume::parse`](crate::fog::FogVolume::parse) makes for its own
    /// edge length.
    pub range: f32,
    /// This node's own world transform, from [`vex::world_transforms`]. The
    /// light's position is this matrix's translation, row 3.
    pub to_world: [f32; 16],
    /// The payload's second sixteen bytes, four `u32` in the file's order.
    ///
    /// `{1, 0, 0, 0}` on every shipped sample seen so far. Nothing here reads
    /// what they mean; carried rather than dropped because a field that is
    /// provably present should be visible in the reimplementation, even
    /// undecoded.
    pub trailer: [u32; 4],
}

impl PointLight {
    /// Decodes one `PointLight` payload against a world matrix from its
    /// transform chain.
    ///
    /// Returns `None` for a payload shorter than [`POINT_PAYLOAD_LEN`].
    #[must_use]
    pub fn parse(payload: &[u8], to_world: [f32; 16], order: ByteOrder) -> Option<Self> {
        if payload.len() < POINT_PAYLOAD_LEN {
            return None;
        }
        let f = read_four_f32(payload, order)?;
        let u = |i: usize| order.u32(payload, 0x10 + i * 4);
        Some(Self {
            colour: [f[0], f[1], f[2]],
            range: f[3],
            to_world,
            trailer: [u(0), u(1), u(2), u(3)],
        })
    }
}

/// Every `AmbientLight` a `.vex` file authors, in node order.
///
/// Placement is [`vex::world_transforms`], **not**
/// [`vex::class_world_transforms`] - see the module docs for why the latter
/// silently drops every light rather than misreading one.
#[must_use]
pub fn ambient_lights(data: &[u8], nodes: &[Node]) -> Vec<AmbientLight> {
    let chain = vex::world_transforms(data, nodes);
    let order = vex::byte_order(data);
    nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.class_id == vex::CLASS_AMBIENT_LIGHT)
        .filter_map(|(index, node)| {
            let to_world = chain.get(index).copied()?;
            AmbientLight::parse(data.get(node.payload())?, to_world, order)
        })
        .collect()
}

/// Every `DirectionalLight` a `.vex` file authors, in node order.
#[must_use]
pub fn directional_lights(data: &[u8], nodes: &[Node]) -> Vec<DirectionalLight> {
    let chain = vex::world_transforms(data, nodes);
    let order = vex::byte_order(data);
    nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.class_id == vex::CLASS_DIRECTIONAL_LIGHT)
        .filter_map(|(index, node)| {
            let to_world = chain.get(index).copied()?;
            DirectionalLight::parse(data.get(node.payload())?, to_world, order)
        })
        .collect()
}

/// Every `PointLight` a `.vex` file authors, in node order.
#[must_use]
pub fn point_lights(data: &[u8], nodes: &[Node]) -> Vec<PointLight> {
    let chain = vex::world_transforms(data, nodes);
    let order = vex::byte_order(data);
    nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.class_id == vex::CLASS_POINT_LIGHT)
        .filter_map(|(index, node)| {
            let to_world = chain.get(index).copied()?;
            PointLight::parse(data.get(node.payload())?, to_world, order)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `{0.25, 0.5, 0.75, 2.0}` as an `AmbientLight`/`DirectionalLight`
    /// payload: distinct values in every slot so a transposed read shows up.
    fn ambient_or_directional_payload() -> Vec<u8> {
        let mut p = vec![0u8; AMBIENT_OR_DIRECTIONAL_PAYLOAD_LEN];
        let mut put = |at: usize, v: f32| p[at..at + 4].copy_from_slice(&v.to_le_bytes());
        put(0x00, 0.25);
        put(0x04, 0.5);
        put(0x08, 0.75);
        put(0x0c, 2.0);
        p
    }

    /// `{0.1, 0.2, 0.3, 40.0}` then `{1, 0, 0, 0}`, the trailer every shipped
    /// sample carries.
    fn point_payload() -> Vec<u8> {
        let mut p = vec![0u8; POINT_PAYLOAD_LEN];
        let mut putf = |at: usize, v: f32| p[at..at + 4].copy_from_slice(&v.to_le_bytes());
        putf(0x00, 0.1);
        putf(0x04, 0.2);
        putf(0x08, 0.3);
        putf(0x0c, 40.0);
        let mut putu = |at: usize, v: u32| p[at..at + 4].copy_from_slice(&v.to_le_bytes());
        putu(0x10, 1);
        putu(0x14, 0);
        putu(0x18, 0);
        putu(0x1c, 0);
        p
    }

    #[test]
    fn an_ambient_light_reads_colour_then_intensity() {
        let light = AmbientLight::parse(
            &ambient_or_directional_payload(),
            vex::IDENTITY,
            ByteOrder::Little,
        )
        .expect("parses");
        assert_eq!(light.colour, [0.25, 0.5, 0.75]);
        assert_eq!(light.intensity, 2.0);
        assert_eq!(light.to_world, vex::IDENTITY);
    }

    #[test]
    fn a_directional_light_reads_the_same_shape_as_ambient() {
        let light = DirectionalLight::parse(
            &ambient_or_directional_payload(),
            vex::IDENTITY,
            ByteOrder::Little,
        )
        .expect("parses");
        assert_eq!(light.colour, [0.25, 0.5, 0.75]);
        assert_eq!(light.intensity, 2.0);
    }

    #[test]
    fn a_short_ambient_or_directional_payload_is_rejected() {
        let short = vec![0u8; AMBIENT_OR_DIRECTIONAL_PAYLOAD_LEN - 1];
        assert_eq!(
            AmbientLight::parse(&short, vex::IDENTITY, ByteOrder::Little),
            None
        );
        assert_eq!(
            DirectionalLight::parse(&short, vex::IDENTITY, ByteOrder::Little),
            None
        );
    }

    #[test]
    fn a_point_light_reads_colour_range_then_trailer() {
        let light =
            PointLight::parse(&point_payload(), vex::IDENTITY, ByteOrder::Little).expect("parses");
        assert_eq!(light.colour, [0.1, 0.2, 0.3]);
        assert_eq!(light.range, 40.0);
        assert_eq!(light.trailer, [1, 0, 0, 0]);
    }

    #[test]
    fn a_short_point_payload_is_rejected() {
        let short = vec![0u8; POINT_PAYLOAD_LEN - 1];
        assert_eq!(
            PointLight::parse(&short, vex::IDENTITY, ByteOrder::Little),
            None
        );
    }

    #[test]
    fn a_point_lights_position_is_its_world_matrixs_translation() {
        let mut m = vex::IDENTITY;
        m[12] = 10.0;
        m[13] = 20.0;
        m[14] = 30.0;
        let light = PointLight::parse(&point_payload(), m, ByteOrder::Little).expect("parses");
        assert_eq!(
            [light.to_world[12], light.to_world[13], light.to_world[14]],
            [10.0, 20.0, 30.0]
        );
    }
}
