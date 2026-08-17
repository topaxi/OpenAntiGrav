//! The camera rigs and the airbrake's graphics block, from `handlingstats.xml`.
//!
//! Split out of `handling.rs` for size alone; the schema and its evidence are
//! documented on the parent module and in `docs/formats/handling-stats.md`.
//! These are the elements that describe how a craft is *shown* rather than how
//! it moves, which is why they are the ones that can differ between titles
//! without any of the physics noticing - `Camera::headtilt` being the case in
//! point.

use super::{Node, Result, number, optional_number};

/// A cockpit camera rig. `<InternalCamera/>` and `<BackwardCamera/>`.
///
/// The two share a shape: the backward camera is the same rig looking the other
/// way, which is why it carries a `headtilt` the bonnet camera does not.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Camera {
    /// Field of view.
    pub fov: f32,
    /// How far the view tilts with the ship's roll.
    ///
    /// `None` where the document omits it, which is a fact about the file's
    /// generation rather than a defect in it: **Wipeout HD's team files drop
    /// the attribute** where its own mode ships still carry it, and both parse.
    /// Nothing in this project applies it anyway - see
    /// `oag_render::camera::internal`, which carries the value and documents
    /// why it does not roll the view by it - so an absent one costs nothing
    /// beyond having to say so here.
    pub headtilt: Option<f32>,
    /// Eye offset along the ship's up axis.
    pub height: f32,
    /// Eye offset along the ship's forward axis.
    pub length: f32,
    /// Fixed pitch applied to the rig.
    pub pitch: f32,
}

impl Camera {
    pub(super) fn from_node(node: &Node, element: &'static str) -> Result<Self> {
        Ok(Self {
            fov: number(node, element, "fov")?,
            headtilt: optional_number(node, element, "headtilt")?,
            height: number(node, element, "height")?,
            length: number(node, element, "length")?,
            pitch: number(node, element, "pitch")?,
        })
    }
}

/// The bonnet camera. `<BonnetCamera fov height length pitch/>`.
///
/// Deliberately not a [`Camera`]: it has no `headtilt`, and folding the two
/// together would mean inventing a default for an attribute the document does
/// not have.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BonnetCamera {
    /// Field of view.
    pub fov: f32,
    /// Eye offset along the ship's up axis.
    pub height: f32,
    /// Eye offset along the ship's forward axis.
    pub length: f32,
    /// Fixed pitch applied to the rig.
    pub pitch: f32,
}

impl BonnetCamera {
    pub(super) const ELEMENT: &'static str = "BonnetCamera";

    pub(super) fn from_node(node: &Node) -> Result<Self> {
        let e = Self::ELEMENT;
        Ok(Self {
            fov: number(node, e, "fov")?,
            height: number(node, e, "height")?,
            length: number(node, e, "length")?,
            pitch: number(node, e, "pitch")?,
        })
    }
}

/// A chase camera. `<ExternalCameraFar/>` and `<ExternalCameraClose/>`.
///
/// Two spring constants rather than one, split horizontal and vertical, which is
/// what lets the camera lag behind in a corner without bouncing over crests.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ExternalCamera {
    /// Field of view.
    pub fov: f32,
    /// Height of the point the camera aims at, relative to the ship.
    pub lookat_height: f32,
    /// Distance along the ship's forward axis of the point it aims at.
    pub lookat_length: f32,
    /// Height of the camera itself, relative to the ship.
    pub pos_height: f32,
    /// Distance behind the ship.
    pub pos_length: f32,
    /// Spring stiffness in the horizontal plane.
    pub spring_horiz: f32,
    /// Spring stiffness vertically.
    pub spring_vert: f32,
}

impl ExternalCamera {
    pub(super) fn from_node(node: &Node, element: &'static str) -> Result<Self> {
        Ok(Self {
            fov: number(node, element, "fov")?,
            lookat_height: number(node, element, "lookat_height")?,
            lookat_length: number(node, element, "lookat_length")?,
            pos_height: number(node, element, "pos_height")?,
            pos_length: number(node, element, "pos_length")?,
            spring_horiz: number(node, element, "spring_horiz")?,
            spring_vert: number(node, element, "spring_vert")?,
        })
    }
}

/// How far and how fast the airbrake flaps move. `<AirbrakeGraphics/>`.
///
/// Presentation only: the force law reads [`Airbrake`], not this.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AirbrakeGraphics {
    /// Deflection at full input.
    pub amount: f32,
    /// Rate the flap returns at.
    pub down_speed: f32,
    /// Rate the flap deploys at.
    pub up_speed: f32,
}

impl AirbrakeGraphics {
    pub(super) const ELEMENT: &'static str = "AirbrakeGraphics";

    pub(super) fn from_node(node: &Node) -> Result<Self> {
        let e = Self::ELEMENT;
        Ok(Self {
            amount: number(node, e, "amount")?,
            down_speed: number(node, e, "down_speed")?,
            up_speed: number(node, e, "up_speed")?,
        })
    }
}
