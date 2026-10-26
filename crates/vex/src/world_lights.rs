//! `wopoint` `0x3cf` and `wospot` `0x3ce`: the track's point and spot lights
//! that the hull's GE light list takes its fourth slots from.
//!
//! Not `PointLight` `0x132` (see [`crate::lighting`]), which the original never
//! registers: these are the two classes `World_CollectMarkerLists` gathers into
//! the context's `+0x4c`/`+0x8c` and `+0x50`/`+0x3ac` lists and
//! `SceneLight_BuildLightingList` reads. Read live on Tech de Ra
//! (`docs/ghidra/functions/psp-pulse-usa/scene-light.md`, "The point and spot
//! lights"): the runtime object's colour and radii are the payload's own words.
//!
//! ```text
//! wopoint, 32 bytes:
//!   +0x00  f32[4]  colour (r, g, b, a), a 1.0 on every sample
//!   +0x10  f32     inner radius: full strength inside it
//!   +0x14  f32     outer radius: none beyond it
//!   +0x18  u32[2]  zero on every sample
//! wospot, 32 bytes:
//!   +0x00  f32[4]  colour
//!   +0x10  f32     inner radius
//!   +0x14  f32     outer radius
//!   +0x18  f32     inner cone angle (radians; the runtime halves it)
//!   +0x1c  f32     outer cone angle
//! ```
//!
//! The position is the node's world matrix translation. A spot's axis is the
//! **negated row 1** of that matrix: the runtime object's `+0x60` read
//! `(0.4865, -0.621, -0.6146)` against a node matrix row 1 of
//! `(-0.486, 0.621, 0.615)`.
//!
//! **A light under an `Anim Transform` moves.** The class's init (`0x0892ca04`)
//! walks the node's ancestors for one, sets the object's `+0x64` byte if it finds
//! it, and its update slot (`0x0892cd00`) copies the node's world translation into
//! `+0x50` every frame; the hull's list builder re-tests a light with the byte set
//! on every call instead of waiting out its countdown. Arc Prime authors two such
//! lights, the white pair that sweep with its floor spotlights; no other Pulse
//! circuit has one. [`placed_lights`] keeps the chain, [`PlacedLight::at`] reads
//! the light where the clock has put it.

use crate::vex::{self, Node, NodeChain};
use oag_formats::ByteOrder;

/// Class ID of a `wopoint` node.
pub const CLASS_WOPOINT: u32 = 0x3cf;
/// Class ID of a `wospot` node.
pub const CLASS_WOSPOT: u32 = 0x3ce;

/// Length of a `wopoint` or `wospot` payload.
pub const PAYLOAD_LEN: usize = 0x20;

/// One `wopoint` or `wospot`: a light with a position, a colour and two radii.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldLight {
    /// Where it sits: the translation of the node's world matrix.
    pub position: [f32; 3],
    /// Linear RGB as authored; the fourth word is unused.
    pub colour: [f32; 3],
    /// Distance inside which the light is at full strength.
    pub inner_radius: f32,
    /// Distance beyond which it does nothing.
    pub outer_radius: f32,
    /// `Some` for a spot: its axis (a unit vector) and the inner and outer cone
    /// angles in radians. `None` for a point.
    pub cone: Option<Cone>,
}

/// A `wospot`'s cone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cone {
    /// The direction the spot points: the negated row 1 of its world matrix.
    pub axis: [f32; 3],
    /// Inner cone angle, radians: full strength inside half of it.
    pub inner_angle: f32,
    /// Outer cone angle, radians: nothing outside half of it.
    pub outer_angle: f32,
}

impl WorldLight {
    fn parse(payload: &[u8], to_world: [f32; 16], order: ByteOrder, spot: bool) -> Option<Self> {
        if payload.len() < PAYLOAD_LEN {
            return None;
        }
        let f = |at: usize| f32::from_bits(order.u32(payload, at));
        let cone = spot.then(|| Cone {
            axis: [-to_world[4], -to_world[5], -to_world[6]],
            inner_angle: f(0x18),
            outer_angle: f(0x1c),
        });
        Some(Self {
            position: [to_world[12], to_world[13], to_world[14]],
            colour: [f(0), f(4), f(8)],
            inner_radius: f(0x10),
            outer_radius: f(0x14),
            cone,
        })
    }
}

/// A `wopoint` or `wospot` as the circuit places it: where it rests, and how it
/// moves if an `Anim Transform` is above it.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedLight {
    rest: WorldLight,
    motion: Option<Motion>,
}

/// What re-places a light at a time: its payload and the chain above it.
#[derive(Debug, Clone, PartialEq)]
struct Motion {
    payload: [u8; PAYLOAD_LEN],
    order: ByteOrder,
    spot: bool,
    chain: NodeChain,
}

impl PlacedLight {
    /// A light that does not move, standing as `light` says.
    #[must_use]
    pub fn fixed(light: WorldLight) -> Self {
        Self {
            rest: light,
            motion: None,
        }
    }

    /// Whether an `Anim Transform` sits above the light: the original's `+0x64`
    /// byte, and the only lights whose position depends on the clock.
    #[must_use]
    pub fn moves(&self) -> bool {
        self.motion.is_some()
    }

    /// The light at `seconds` into the scenery clock: position, and a spot's axis,
    /// from its node's world matrix then. A light that does not move is the same
    /// at every time.
    #[must_use]
    pub fn at(&self, seconds: f32) -> WorldLight {
        let Some(motion) = &self.motion else {
            return self.rest;
        };
        WorldLight::parse(
            &motion.payload,
            motion.chain.world_at(seconds),
            motion.order,
            motion.spot,
        )
        .unwrap_or(self.rest)
    }
}

/// Every `wopoint` then every `wospot` the file authors, each in node order, with
/// the chain of each that moves. The classes are the file's own table's
/// (`0x3cf`/`0x3ce` in version 6, `0x381`/`0x380` in Pure's version 4).
///
/// That is the order the original's two lists are walked in (points first),
/// which decides which light a tie keeps.
#[must_use]
pub fn placed_lights(data: &[u8], nodes: &[Node]) -> Vec<PlacedLight> {
    let Ok(classes) = vex::classes_of(data) else {
        return Vec::new();
    };
    let chain = vex::world_transforms(data, nodes);
    let order = vex::byte_order(data);
    let mut out = Vec::new();
    for (class, spot) in [(classes.wopoint, false), (classes.wospot, true)] {
        let Some(class) = class else { continue };
        out.extend(
            nodes
                .iter()
                .enumerate()
                .filter(|(_, node)| node.class_id == class)
                .filter_map(|(index, node)| {
                    let payload = data.get(node.payload())?;
                    let to_world = chain.get(index).copied()?;
                    let rest = WorldLight::parse(payload, to_world, order, spot)?;
                    let chain = NodeChain::of(data, nodes, index, vex::IDENTITY);
                    let motion = chain.moves().then(|| Motion {
                        payload: payload[..PAYLOAD_LEN].try_into().expect("parse checked it"),
                        order,
                        spot,
                        chain,
                    });
                    Some(PlacedLight { rest, motion })
                }),
        );
    }
    out
}

#[cfg(test)]
mod tests;
