//! `gridCamera` `0x3dd`: the circuit's authored pre-race flyby.
//!
//! Every Wipeout Pulse circuit directory carries a `start_grid.vex`, a Maya
//! camera scene: a `World`, two `Anim Transform`s (`camera1_group`,
//! `grid_camera1`) and one `gridCamera` leaf (`grid_cameraShape1`).
//! `grid_camera1`'s keyed translation and rotation **are the flyby** the original
//! plays before a race, under the track-description panel
//! ([`docs/gameplay/race-intro.md`](../../../docs/gameplay/race-intro.md)).
//!
//! ```text
//! world
//! `- camera1_group   Anim Transform, one key: the identity, in effect
//!    `- grid_camera1  Anim Transform, the flyby: 9 to 230 translation keys, 112 to 215 rotation
//!                     keys; attributes AnimEnd (key units), GridPosition 1, Relative 0
//!       `- grid_cameraShape1   gridCamera, 48 bytes
//! ```
//!
//! # What is read
//!
//! - **The pose** is the leaf's world matrix with the animation evaluated at a
//!   time ([`super::vex::world_transforms_at`]'s entry for the leaf): row 3 the
//!   eye, rows 0 to 2 the camera's right, up and back. Checked frame by frame
//!   against the running original on `16_Track` and `03_Track`: to `0.1` world
//!   units over the whole 25 seconds, no sign flip or transpose.
//! - **The length** is `AnimEnd` on `grid_camera1`, in key units: the original's
//!   progress test (`GridCamera_Progress`, `0x08908a70`) divides the animation's
//!   clock by it and leaves the camera at one. `1500` (25.0 s) on nine circuits,
//!   `1620` (27.0 s) on `02_Track`, `1080` (18.0 s) on `01_Track`, `1078` on
//!   `14_Track`.
//! - **The cuts** are the translation channel's one-frame key gaps (`360`/`361`
//!   on `16_Track`): the artist's way of jumping between shots, since a key pair
//!   a frame apart blends across one tick. On `16_Track` the original's three
//!   large jumps land on keys `360`, `720` and `1080`, each such a pair. Smaller
//!   one-frame gaps (`1002`) are reported as cuts too: a cut here only means the
//!   picture may jump, which the temporal upscaler needs to know.
//!
//! # What is not read
//!
//! - **The field of view.** The leaf's payload is byte-identical on all twelve
//!   circuits but for its aim point (`+0x10`, as a `Camera`'s), and the original's
//!   field was `54.309` vertical degrees on both circuits captured. Nothing in
//!   the payload derives it, so [`FOV_DEGREES`] is a measured constant, not a
//!   reading.
//! - `GridPosition` and `Relative`, the two other attributes on `grid_camera1`.

use crate::vex::{self, Node};

/// Class ID of a `gridCamera` node.
pub const CLASS_GRID_CAMERA: u32 = 0x3dd;

/// The vertical field of view, degrees: the original's render view carried
/// through every frame of the flyby (`0x42593c69` as an `f32`). Measured on
/// `16_Track` and `03_Track`; **not** derived from the file (see the module docs).
pub const FOV_DEGREES: f32 = 54.308_994;

/// A circuit's flyby: the `start_grid.vex` scene, ready to be sampled.
#[derive(Debug, Clone)]
pub struct GridCamera {
    data: Vec<u8>,
    nodes: Vec<Node>,
    camera: usize,
    /// `AnimEnd`, in seconds.
    length: f32,
    /// The frame (key units) of the second key of every one-frame translation key pair,
    /// ascending: the keys a cut lands on.
    cut_frames: Vec<u32>,
}

/// Where the camera is at one instant, in the world's own axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    /// The eye.
    pub eye: [f32; 3],
    /// The camera's right, up and back axes, one row each - a rotation matrix whose third row
    /// points **away** from what the camera sees.
    pub rows: [[f32; 3]; 3],
}

impl GridCamera {
    /// Reads a circuit's `start_grid.vex`; `None` when the file does not parse, has
    /// no `gridCamera`, or the camera's parent is not an `Anim Transform` that
    /// decodes (never a flyby that would start somewhere arbitrary).
    #[must_use]
    pub fn read(data: &[u8]) -> Option<Self> {
        let nodes = vex::nodes(data).ok()?;
        let camera = nodes
            .iter()
            .position(|node| node.class_id == CLASS_GRID_CAMERA)?;
        let parent = nodes.get(camera)?.parent?;
        let anim = vex::anim_transform_of(data, nodes.get(parent)?)?;
        let attributes = vex::node_attributes(data, nodes.get(parent)?);
        let anim_end = attributes
            .iter()
            .find(|(name, _)| name == "AnimEnd")
            .map(|(_, value)| *value)?;
        let unit = anim.seconds_per_key;
        let length = anim_end * unit;
        if !(length.is_finite() && length > 0.0) {
            return None;
        }
        let cut_frames = anim
            .translation
            .times
            .windows(2)
            .filter(|pair| pair[1] == pair[0] + 1)
            .map(|pair| u32::from(pair[1]))
            .collect();
        Some(Self {
            data: data.to_vec(),
            nodes,
            camera,
            length,
            cut_frames,
        })
    }

    /// How long the flyby plays, seconds: `AnimEnd`.
    #[must_use]
    pub fn length(&self) -> f32 {
        self.length
    }

    /// The camera's pose `seconds` into the animation.
    #[must_use]
    pub fn pose_at(&self, seconds: f32) -> Pose {
        let world = vex::world_transforms_at(&self.data, &self.nodes, seconds);
        let m = world[self.camera];
        Pose {
            eye: [m[12], m[13], m[14]],
            rows: [[m[0], m[1], m[2]], [m[4], m[5], m[6]], [m[8], m[9], m[10]]],
        }
    }

    /// Whether the picture jumps to another shot between `from` and `to` seconds: whether a
    /// one-frame key gap of the translation channel has its second key in `(from, to]`.
    #[must_use]
    pub fn cuts_between(&self, from: f32, to: f32) -> bool {
        // Key times are frames of 1/60 s.
        let frame = |seconds: f32| (seconds * 60.0 + 1e-3).floor();
        let (a, b) = (frame(from), frame(to));
        self.cut_frames
            .iter()
            .any(|&key| (key as f32) > a && (key as f32) <= b)
    }

    /// The key frames at which the picture jumps, for the report line and the tests.
    #[must_use]
    pub fn cut_frames(&self) -> &[u32] {
        &self.cut_frames
    }
}

#[cfg(test)]
mod tests;
