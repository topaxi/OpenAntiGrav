//! HD's `Team Selection` craft, framed as the screen's own code frames it.
//!
//! **The widget's authored pose is not what the original draws.**
//! `Team_Selection_Definition.xml`'s `<Model name="ShipModel">` authors
//! `OriginX=1220 OriginY=412`, `z=-24`, `RotX=0.4 RotY=-0.5`, and the screen's
//! update writes over the rotation, the depth, the lens and the origin every
//! frame ([`oag_title::ShipFit`];
//! `docs/ghidra/functions/ps3-hdfury-eu/team-selection-model.md`). What stays
//! the widget's is `nearZ` and `ObeySafeZone`. A title with a fit is drawn by
//! [`fitted`]; one without (Omega, on a pose it borrows) by [`matrices`].
//!
//! The craft is drawn at its **raw model-space coordinates** either way:
//! nothing re-centres or rescales it.

use oag_core::math::{Mat4, Vec3, camera};
use oag_display::space::Space;
use oag_title::{ShipControl, ShipFit};
use oag_ui_screens::picker::hd::ShipModel;

pub mod control;

use control::{Craft, CraftControl, CraftInput};

/// The far plane of a widget's own pose: Omega's update writes `100` into the item.
pub const FAR: f32 = 100.0;
/// The near plane the HD widget authors.
pub const NEAR: f32 = 1.0;

/// The view-projection and model matrices of the craft as the widget poses it: its rotation,
/// position, scale and lens through its `nearZ`, zoomed and slid to its origin. `safe_area` is the
/// title's `Safe Area Setting` as a scale, which the picture and the origin take when the widget
/// authors `ObeySafeZone`.
#[must_use]
pub fn matrices(widget: &ShipModel, craft: &Craft, space: Space, safe_area: f32) -> (Mat4, Mat4) {
    let safe_area = if widget.obey_safe_zone {
        safe_area
    } else {
        1.0
    };
    let projection = camera::perspective(
        (widget.fov_degrees + craft.fov_delta).to_radians(),
        space.display_aspect,
        widget.near,
        FAR,
    );
    let model = Mat4::from_translation(Vec3::new(
        widget.position[0] + craft.pan[0],
        widget.position[1] + craft.pan[1],
        widget.z + craft.push,
    )) * Mat4::from_rotation_x(widget.rot_x + craft.turn[0])
        * Mat4::from_rotation_y(widget.rot_y + craft.turn[1])
        * Mat4::from_scale(Vec3::splat(widget.scale));
    let picture = safe_area * widget.zoom;
    (
        Mat4::from_translation(grid_shift(widget.origin, space) * safe_area)
            * Mat4::from_scale(Vec3::new(picture, picture, 1.0))
            * projection,
        model,
    )
}

/// Where a grid point sits in normalised device coordinates, as a slide of the
/// finished picture.
fn grid_shift(origin: [f32; 2], space: Space) -> Vec3 {
    Vec3::new(
        2.0 * origin[0] / space.size.0 - 1.0,
        1.0 - 2.0 * origin[1] / space.size.1,
        0.0,
    )
}

/// Where the fit holds a model at rest: how far in front of the lens, and how
/// far the widget's origin is slid, in the screen's grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Framing {
    pub depth: f32,
    pub slide: [f32; 2],
}

/// One frame of the fit at `depth`: the slide that centres the box, and whether
/// the centred box is inside the window.
fn measure(
    fit: &ShipFit,
    turned: [f32; 2],
    aspect: f32,
    box_z: [f32; 2],
    depth: f32,
) -> ([f32; 2], bool) {
    let turn = Mat4::from_rotation_x(fit.rotation[0] + turned[0])
        * Mat4::from_rotation_y(fit.rotation[1] + turned[1]);
    let lens = 1.0 / fit.fit_half_fov.tan();
    // The original's running minimum and maximum start at zero, not at the
    // first corner.
    let (mut low, mut high) = ([0.0_f32; 2], [0.0_f32; 2]);
    for corner in 0..8 {
        let at = turn.transform_point3(Vec3::new(
            if corner & 2 == 0 {
                -fit.half_box[0]
            } else {
                fit.half_box[0]
            },
            if corner & 1 == 0 {
                -fit.half_box[1]
            } else {
                fit.half_box[1]
            },
            box_z[corner >> 2],
        ));
        let w = depth - at.z;
        let on_screen = [
            at.x * lens / aspect / w * fit.unit[0],
            -(at.y * lens / w) * fit.unit[1],
        ];
        for axis in 0..2 {
            low[axis] = low[axis].min(on_screen[axis]);
            high[axis] = high[axis].max(on_screen[axis]);
        }
    }
    let slide = [-0.5 * (low[0] + high[0]), -0.5 * (low[1] + high[1])];
    let inside = (0..2).all(|axis| {
        low[axis] + slide[axis] >= -fit.window[axis] && high[axis] + slide[axis] <= fit.window[axis]
    });
    (slide, inside)
}

/// The depth the fit moves to from `depth` in one frame: nearer while the box
/// is inside the window, farther while it is not, never nearer than the floor.
fn stepped(fit: &ShipFit, depth: f32, inside: bool) -> f32 {
    let moved = if inside {
        depth - fit.step
    } else {
        depth + fit.step
    };
    moved.max(fit.floor)
}

/// Where the fit comes to rest for a box `box_z` fore and aft.
///
/// The original steps the depth every frame and never stops: at rest it
/// alternates between the last depth the box fits at and the one a step
/// nearer, and the drawn depth and slide ease towards whichever is current, so
/// they sit between the two. This is the mean of that pair.
#[must_use]
pub fn rest(fit: &ShipFit, aspect: f32, box_z: [f32; 2]) -> Framing {
    // Far enough for any depth a model's length can ask for: the fit moves one
    // step a frame, and a box as long as the far plane is deep rests inside it.
    let frames = (2.0 * fit.far / fit.step) as usize;
    let mut depth = fit.start;
    let mut seen = [(depth, [0.0; 2]); 2];
    for frame in 0..frames {
        let (slide, inside) = measure(fit, [0.0; 2], aspect, box_z, depth);
        seen[frame % 2] = (depth, slide);
        let next = stepped(fit, depth, inside);
        // Back where it was two frames ago: the rest pair, or the floor.
        if frame > 0 && (next - seen[(frame + 1) % 2].0).abs() < 0.5 * fit.step {
            break;
        }
        depth = next;
    }
    Framing {
        depth: 0.5 * (seen[0].0 + seen[1].0),
        slide: [
            0.5 * (seen[0].1[0] + seen[1].1[0]),
            0.5 * (seen[0].1[1] + seen[1].1[1]),
        ],
    }
}

/// The seconds of one frame of the screen's update: the original counts frames
/// at sixty a second.
pub const FRAME_SECONDS: f32 = 1.0 / 60.0;

/// Frames that bring the fit to rest from its opening state for any model on the
/// disc: the target walks 2 units at most (a hundred frames at most), and the
/// ease then leaves under a millionth of it.
const SETTLED_FRAMES: usize = 600;

/// The fit's state across frames: the depth and slide it is moving to and the
/// ones it draws, which follow them.
///
/// The original keeps this on the screen and updates it once a frame, whether
/// or not the craft is drawn, and a step to another model resets none of it
/// (`docs/ghidra/functions/ps3-hdfury-eu/team-selection-model.md`, "What the
/// update writes over the widget"). A model that rests at another depth is
/// therefore drawn from the last one's depth, while the target walks to its own
/// at one step a frame and the drawn depth follows it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FitMotion {
    target: f32,
    depth: f32,
    slide_target: [f32; 2],
    slide: [f32; 2],
}

impl FitMotion {
    /// The screen as it opens: the target at the fit's start. The drawn depth
    /// is unread, and is placed on the target; the craft is absent for the
    /// first 0.9 s (`hull_swap::ENTRY_SECONDS`), which is 54 frames of the ease.
    #[must_use]
    pub fn open(fit: &ShipFit) -> Self {
        Self {
            target: fit.start,
            depth: fit.start,
            slide_target: [0.0; 2],
            slide: [0.0; 2],
        }
    }

    /// One frame of the screen's update for a selected model whose box is
    /// `box_z`: the fit measures the box at the target and steps it, and the
    /// drawn depth and slide ease toward the new targets.
    ///
    /// The order inside a frame (fit, then ease) is chosen, a frame's difference
    /// at most.
    pub fn frame(&mut self, fit: &ShipFit, aspect: f32, box_z: [f32; 2]) {
        self.frame_turned(fit, aspect, box_z, [0.0; 2]);
    }

    /// [`Self::frame`] for a craft the player has turned `turned` radians about X and Y: the
    /// fit's box is measured through the turned rotation, as the original's projector
    /// (`TeamSelectionScreen_ProjectFitCorner`) adds the same terms to the rest rotation.
    pub fn frame_turned(&mut self, fit: &ShipFit, aspect: f32, box_z: [f32; 2], turned: [f32; 2]) {
        let (slide, inside) = measure(fit, turned, aspect, box_z, self.target);
        self.slide_target = slide;
        self.target = stepped(fit, self.target, inside);
        self.depth += (self.target - self.depth) * fit.depth_ease;
        for axis in 0..2 {
            self.slide[axis] += (self.slide_target[axis] - self.slide[axis]) * fit.slide_ease;
        }
    }

    /// The screen settled on the model whose box is `from`, then `frames` frames
    /// after a step to the one whose box is `to`: how a capture stands where a
    /// film would.
    #[must_use]
    pub fn stepped(
        fit: &ShipFit,
        aspect: f32,
        from: [f32; 2],
        to: [f32; 2],
        frames: usize,
    ) -> Self {
        let mut motion = Self::open(fit);
        for _ in 0..SETTLED_FRAMES {
            motion.frame(fit, aspect, from);
        }
        for _ in 0..frames {
            motion.frame(fit, aspect, to);
        }
        motion
    }

    /// The depth the fit is moving to.
    #[must_use]
    pub fn target(&self) -> f32 {
        self.target
    }

    /// Where the craft is drawn this frame.
    #[must_use]
    pub fn framing(&self) -> Framing {
        Framing {
            depth: self.depth,
            slide: self.slide,
        }
    }
}

/// The craft's whole per-frame state on `Team Selection`, driven by a fixed tick of any length:
/// one frame of the screen's update for each [`FRAME_SECONDS`] that has passed. The fit, where
/// the title has one, and the player's own control of the craft, where it has that.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShipPlayer {
    motion: Option<FitMotion>,
    control: CraftControl,
    law: Option<ShipControl>,
}

impl ShipPlayer {
    /// A screen as it opens: the fit at its start where the title has one, the craft at rest.
    #[must_use]
    pub fn open(fit: Option<&ShipFit>, law: Option<ShipControl>) -> Self {
        Self {
            motion: fit.map(FitMotion::open),
            control: CraftControl::default(),
            law,
        }
    }

    /// Advances by `dt` seconds with `fit` (and the aspect and `box_z` of the model selected
    /// now) where the title fits its craft, and what the devices said.
    pub fn tick(&mut self, fit: Option<(&ShipFit, f32, [f32; 2])>, input: &CraftInput, dt: f32) {
        let motion = &mut self.motion;
        self.control.advance(self.law.as_ref(), input, dt, |pose| {
            if let (Some(motion), Some((fit, aspect, box_z))) = (motion.as_mut(), fit) {
                motion.frame_turned(fit, aspect, box_z, pose.turn);
            }
        });
    }

    /// Where the fit holds the craft now, where the title has one.
    #[must_use]
    pub fn framing(&self) -> Option<Framing> {
        self.motion.map(|motion| motion.framing())
    }

    /// Whether the craft is zoomed now, whichever input holds it.
    #[must_use]
    pub fn zoomed(&self) -> bool {
        self.law
            .as_ref()
            .is_some_and(|law| self.control.zoomed(law))
    }

    /// What the player has done to the craft now.
    #[must_use]
    pub fn craft(&self) -> Craft {
        self.law
            .as_ref()
            .map_or_else(Craft::default, |law| self.control.pose().of(law))
    }
}

/// The fore and aft extent of the fit's box: the mode's box where the screen
/// draws the mode's own craft (`own_craft`: Detonator's, else Zone's), else the
/// selected model's own length each way. `None`
/// where neither is known, and the craft is then not fitted.
#[must_use]
pub fn box_z(
    fit: &ShipFit,
    own_craft: Option<oag_race::Mode>,
    length: Option<f32>,
) -> Option<[f32; 2]> {
    match own_craft {
        Some(oag_race::Mode::Detonator) => Some(fit.detonator_box),
        Some(_) => Some(fit.zone_box),
        None => length.map(|length| [-length, length]),
    }
}

/// The view-projection and model matrices of the craft as the screen's fit
/// frames it at `framing`: [`rest`] for a still, [`FitMotion::framing`] for the
/// screen as it plays. `safe_area` is the title's `Safe Area Setting`
/// as a scale, `1.0` for a widget that does not obey it: the original scales the
/// finished picture and the origin's distance from the middle of the screen by it.
#[must_use]
pub fn fitted(
    fit: &ShipFit,
    framing: Framing,
    craft: &Craft,
    space: Space,
    safe_area: f32,
) -> (Mat4, Mat4) {
    let projection = camera::perspective(
        (fit.fov_degrees + craft.fov_delta).to_radians(),
        space.display_aspect,
        NEAR,
        fit.far,
    );
    let model = Mat4::from_translation(Vec3::new(
        craft.pan[0],
        craft.pan[1],
        craft.push - framing.depth,
    )) * Mat4::from_rotation_x(fit.rotation[0] + craft.turn[0])
        * Mat4::from_rotation_y(fit.rotation[1] + craft.turn[1]);
    let origin = [
        fit.origin[0] + framing.slide[0],
        fit.origin[1] + framing.slide[1],
    ];
    (
        Mat4::from_translation(grid_shift(origin, space) * safe_area)
            * Mat4::from_scale(Vec3::new(safe_area, safe_area, 1.0))
            * projection,
        model,
    )
}

/// The matrices the screen draws its craft with: the title's fit when it has one and the
/// framing is known, else the widget's own pose. `safe_area` is the title's `Safe Area Setting`
/// as a scale, which the picture takes when the widget authors `ObeySafeZone`.
#[must_use]
pub fn framed(
    widget: &ShipModel,
    fit: Option<(&ShipFit, Framing)>,
    craft: &Craft,
    space: Space,
    safe_area: f32,
) -> (Mat4, Mat4) {
    match fit {
        Some((fit, framing)) => {
            let safe_area = if widget.obey_safe_zone {
                safe_area
            } else {
                1.0
            };
            fitted(fit, framing, craft, space, safe_area)
        }
        None => matrices(widget, craft, space, safe_area),
    }
}

#[cfg(test)]
mod tests;
