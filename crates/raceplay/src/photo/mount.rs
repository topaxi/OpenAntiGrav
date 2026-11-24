//! How photo mode's camera is bound to the craft it frames: what a saved shot keeps so that a
//! track blur can be drawn again with the camera riding the craft as it did.
//!
//! The mode puts the camera again at every tick it moves the race through ([`Photo::pose`]), so
//! the camera follows the craft by the lens's own rule. A [`Mount`] is that rule with the numbers
//! of one shot, which puts the camera for the craft of any other tick:
//!
//! - `internal` and `close` (and `track` on a circuit with no camera node) are rigid in the
//!   craft's frame ([`Follow::Craft`]);
//! - `free` is the craft's position plus a world offset, at a world orientation
//!   ([`Follow::Offset`]): it does not turn with the craft;
//! - `track` stands on the circuit's camera node nearest the craft and aims at it
//!   ([`Follow::Node`]).

use super::*;

/// The rule a camera follows its craft by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Follow {
    /// Rigid in the craft's frame.
    Craft,
    /// The craft's position plus a fixed offset, at a fixed orientation.
    Offset,
    /// On the nearest circuit camera node, aimed at the craft.
    Node,
}

impl Follow {
    /// The word the shot's text spells it with.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Craft => "craft",
            Self::Offset => "offset",
            Self::Node => "node",
        }
    }

    /// The rule a word names.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        [Self::Craft, Self::Offset, Self::Node]
            .into_iter()
            .find(|follow| follow.name() == name)
    }
}

/// A camera bound to a craft.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mount {
    /// The rule.
    pub follow: Follow,
    /// The grid slot of the craft framed (photo mode can follow any craft in the race).
    pub subject: usize,
    /// [`Follow::Craft`]: the eye in the craft's frame. [`Follow::Offset`]: the eye relative to the
    /// craft, in world axes. [`Follow::Node`]: the node the shot stood on, for a craft with none
    /// near it.
    pub eye: Vec3,
    /// [`Follow::Craft`]: the camera's orientation in the craft's frame. [`Follow::Offset`]: its
    /// world orientation. [`Follow::Node`]: the roll about the view axis, on top of the aim.
    pub rotation: Quat,
    /// The vertical field in degrees.
    pub fov_deg: f32,
}

impl Mount {
    /// The mount of the camera `pose` that the lens `lens` put at `craft`, with `node` the
    /// circuit camera nearest it ([`Photo::pose`]'s own inputs).
    #[must_use]
    pub fn of(lens: Lens, pose: &CameraOverride, craft: &Subject, node: Option<Vec3>) -> Self {
        let fov_deg = pose.fov_deg.unwrap_or(craft_view::FOV_DEGREES);
        let (follow, eye, rotation) = match (lens, node) {
            (Lens::Free, _) => (Follow::Offset, pose.eye - craft.position, pose.orientation),
            (Lens::Track, Some(node)) => {
                let aim = aim_at(node, craft.position, fov_deg);
                (Follow::Node, node, aim.inverse() * pose.orientation)
            }
            _ => {
                let into = craft.orientation.inverse();
                (
                    Follow::Craft,
                    into * (pose.eye - craft.position),
                    into * pose.orientation,
                )
            }
        };
        Self {
            follow,
            subject: craft.slot,
            eye,
            rotation,
            fov_deg,
        }
    }

    /// The camera for `craft`, with `node` the circuit camera nearest it.
    #[must_use]
    pub fn pose(&self, craft: &Subject, node: Option<Vec3>) -> CameraOverride {
        let (eye, orientation) = match self.follow {
            Follow::Craft => (
                craft.position + craft.orientation * self.eye,
                craft.orientation * self.rotation,
            ),
            Follow::Offset => (craft.position + self.eye, self.rotation),
            Follow::Node => {
                let node = node.unwrap_or(self.eye);
                (
                    node,
                    aim_at(node, craft.position, self.fov_deg) * self.rotation,
                )
            }
        };
        CameraOverride {
            eye,
            orientation,
            fov_deg: Some(self.fov_deg),
        }
    }
}

/// The orientation of a camera at `eye` aimed at `focus`, as the `track` lens aims.
fn aim_at(eye: Vec3, focus: Vec3, fov: f32) -> Quat {
    let (_, orientation, _) = Destroy::posed(eye, focus, fov, 1.0)
        .to_world()
        .to_scale_rotation_translation();
    orientation
}

impl Photo {
    /// The mount of the camera the mode puts at `craft`, `node` the circuit camera nearest it.
    #[must_use]
    pub fn mount(&self, craft: &Subject, node: Option<Vec3>) -> Mount {
        // Until it is seeded the free lens is drawn as the close lens is (`Self::pose`).
        let lens = match self.lens {
            Lens::Free if self.free.is_none() => Lens::Close,
            lens => lens,
        };
        Mount::of(lens, &self.pose(craft, node), craft, node)
    }
}

impl Race {
    /// The mount of the camera photo mode puts at the tick the race stands at.
    #[must_use]
    pub fn photo_mount(&self) -> Option<Mount> {
        let photo = self.view.photo.as_ref()?;
        let (craft, node) = self.craft_and_node(photo.subject())?;
        Some(photo.mount(&craft, node))
    }

    /// Puts the camera where `mount` has it for the craft as it stands now: the camera of a saved
    /// shot riding its craft, at each tick it is drawn at.
    pub fn ride(&mut self, mount: &Mount) {
        if let Some((craft, node)) = self.craft_and_node(mount.subject) {
            self.view.camera_override = Some(mount.pose(&craft, node));
        }
    }

    pub(super) fn craft_and_node(&self, slot: usize) -> Option<(Subject, Option<Vec3>)> {
        let body = &self.core.sim.world.ships.get(slot)?.physics.body;
        let craft = Subject {
            slot,
            position: body.position,
            orientation: body.orientation,
        };
        let node = self
            .view
            .finish_camera
            .as_ref()
            .and_then(|director| director.node_eye_near(craft.position));
        Some((craft, node))
    }
}
