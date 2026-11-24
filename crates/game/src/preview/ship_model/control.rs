//! The player's own turn, zoom and pan of the craft on `Team Selection`.
//!
//! The screen's update keeps six terms beside the fit (`st+0xfe8`, `+0xfec`, `+0xff0`, `+0xff4`,
//! `+0xff8`, `+0x1024`) that the right stick, the zoom button and the left stick push, each eased
//! toward its target, and writes them over the widget every frame
//! (`docs/ghidra/functions/ps3-hdfury-eu/team-selection-model.md`, "The player's own turn, pan
//! and zoom"). All are zero at rest. The turn is also what the depth fit measures its box
//! through, so a turned craft is refitted as the original refits it.
//!
//! The law is [`oag_title::ShipControl`]'s data; this is one frame of it, plus the three inputs
//! the original has no counterpart of: a pointer drag, the wheel and a held pan. **Ours, chosen,
//! not measured**: a drag turns the craft (or pans it while it is zoomed, as the original's pan
//! only exists while its zoom button is held), the wheel pulls the zoom, and what they set walks
//! back to rest on the same idle count as the stick's turn.

use oag_title::ShipControl;

use super::FRAME_SECONDS;

/// The direction a stick's up and right push the turn and the pan: the original's pad is read as
/// a signed float whose sign convention is unread, so **chosen, not measured**: pushing the
/// right stick right swings the craft's nose to the player's right, pushing it up tips the nose
/// up, and the left stick moves the craft the way it is pushed.
const TURN_SIGN: [f32; 2] = [-1.0, 1.0];
const PAN_SIGN: [f32; 2] = [1.0, 1.0];

/// Radians of turn one grid unit of pointer travel makes. **Chosen, not measured**: a drag across
/// the craft's own width (about 500 units) is about a third of a turn.
pub const DRAG_TURN: f32 = 0.004;

/// Grid units of pointer travel that carry the craft the stick's whole reach while it is
/// zoomed. **Chosen, not measured**: about the craft frame's half width.
pub const DRAG_REACH: f32 = 330.0;

/// How much of a full zoom pull one wheel detent is. **Chosen, not measured**: five detents.
pub const WHEEL_PULL: f32 = 0.2;

/// What the devices say for one tick. The sticks and the pull are levels, held every frame the
/// tick makes; `grab`, `drag` and `wheel` are things that happened once and are spent by the
/// tick's first frame.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CraftInput {
    /// The right stick before any dead band, right and up positive: turns the craft.
    pub right: (f32, f32),
    /// The left stick, right and up positive: pans the craft while it is zoomed.
    pub left: (f32, f32),
    /// The zoom button's pull, 0 to 1.
    pub zoom: f32,
    /// A pointer is dragging the craft: its travel this tick in grid units, right and down.
    pub drag: Option<(f32, f32)>,
    /// Wheel detents this tick, positive toward the player (zooms out).
    pub wheel: i32,
}

impl CraftInput {
    /// Whether the tick says nothing, which is every tick of a player with their hands off.
    #[must_use]
    pub fn is_idle(&self) -> bool {
        *self == Self::default()
    }
}

/// Where the terms stand: what the widget is written this frame.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CraftPose {
    /// The turn about X and about Y, radians, on top of the rest rotation (`st+0xfe8`, `+0xfec`).
    pub turn: [f32; 2],
    /// The zoom term (`st+0x1024`): the lens is `fov_per_zoom` degrees narrower per unit of it.
    pub zoom: f32,
    /// How far the craft is moved across and down, model units (`st+0xff4`, `+0xff8`).
    pub pan: [f32; 2],
    /// The push term (`st+0xff0`): nothing in the update drives it, so it is zero.
    pub push: f32,
}

/// What the player's terms do to the picture: the rotation and position added to the pose the
/// screen draws at rest, the lens narrowed and the craft pushed in depth. All zero at rest.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Craft {
    /// Radians about X and Y added to the rest rotation.
    pub turn: [f32; 2],
    /// Model units across and down added to the craft's position.
    pub pan: [f32; 2],
    /// Degrees added to the lens (negative: narrower, closer).
    pub fov_delta: f32,
    /// Model units added to the craft's depth, toward the lens.
    pub push: f32,
}

impl CraftPose {
    /// The picture's share of the terms under `law`.
    #[must_use]
    pub fn of(&self, law: &ShipControl) -> Craft {
        Craft {
            turn: self.turn,
            pan: self.pan,
            fov_delta: law.fov_per_zoom * self.zoom,
            push: law.push_scale * self.push,
        }
    }
}

/// The six terms, the accumulators and the idle count, and the clock that makes a tick of any
/// length the original's frames.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CraftControl {
    /// What the right stick has pushed the turn to (`st+0xfe0`, `+0xfe4`); the drawn turn
    /// follows it.
    accumulated: [f32; 2],
    pose: CraftPose,
    /// A pointer's pull on the zoom, held until the idle count runs out.
    wheel_pull: f32,
    /// A pointer's pan, held the same way.
    grabbed_pan: [f32; 2],
    /// Frames since anything moved (`st+0xffc`).
    idle: u32,
    owed: f32,
}

impl CraftControl {
    /// Whether the craft is zoomed, by the button or by the wheel: the left stick and the d-pad
    /// are then the pan's and not the list's.
    #[must_use]
    pub fn zoomed(&self, law: &ShipControl) -> bool {
        self.wheel_pull > 0.0 || self.pose.zoom > law.zoom_active
    }

    /// The terms as they stand.
    #[must_use]
    pub fn pose(&self) -> CraftPose {
        self.pose
    }

    /// Advances by `dt` seconds, calling `frame` with the pose after each of the original's
    /// frames. Without a law (`None`: a title with no such control) the terms stay at rest.
    pub fn advance(
        &mut self,
        law: Option<&ShipControl>,
        input: &CraftInput,
        dt: f32,
        mut frame: impl FnMut(&CraftPose),
    ) {
        self.owed += dt;
        let mut first = true;
        // A tick of exactly a frame is a frame, whatever the float rounding.
        while self.owed >= FRAME_SECONDS * 0.999 {
            if let Some(law) = law {
                self.frame(law, input, first);
            }
            first = false;
            frame(&self.pose);
            self.owed -= FRAME_SECONDS;
        }
        self.owed = self.owed.max(0.0);
    }

    /// One frame of the update's control half. `spend` is whether this frame takes the tick's
    /// one-off events (the drag and the wheel).
    fn frame(&mut self, law: &ShipControl, input: &CraftInput, spend: bool) {
        let shaped = |value: f32| {
            let value = value.clamp(-1.0, 1.0);
            if value > law.deadzone {
                value - law.deadzone
            } else if value < -law.deadzone {
                value + law.deadzone
            } else {
                0.0
            }
        };
        let (right_x, right_y) = (shaped(input.right.0), shaped(input.right.1));
        let mut turn_push = [
            TURN_SIGN[0] * right_y * law.turn_rate,
            TURN_SIGN[1] * right_x * law.turn_rate,
        ];
        let mut touched = right_x != 0.0 || right_y != 0.0;
        if spend {
            if input.wheel != 0 {
                // A wheel detent toward the player zooms out.
                self.wheel_pull =
                    (self.wheel_pull - input.wheel as f32 * WHEEL_PULL).clamp(0.0, 1.0);
                touched = true;
            }
            if let Some((across, down)) = input.drag
                && (across != 0.0 || down != 0.0)
            {
                if self.pose.zoom > law.zoom_active {
                    self.grab_pan(law, across, down);
                } else {
                    turn_push[0] += TURN_SIGN[0] * -down * DRAG_TURN;
                    turn_push[1] += TURN_SIGN[1] * across * DRAG_TURN;
                }
                touched = true;
            }
        }
        for axis in 0..2 {
            self.accumulated[axis] += turn_push[axis];
        }
        let pull = input.zoom.clamp(0.0, 1.0).max(self.wheel_pull);
        let target = pull * law.zoom_scale;
        // Only the button holds the idle count off: a wheel's pull is a touch once, and is then
        // let go on the idle count like the turn.
        if input.zoom.clamp(0.0, 1.0) * law.zoom_scale > law.zoom_active {
            touched = true;
        }
        // A pan with the stick while the wheel holds the zoom is the player at work too.
        if self.wheel_pull > 0.0 && input.left != (0.0, 0.0) {
            touched = true;
        }
        if touched {
            self.idle = 0;
        }

        for axis in 0..2 {
            let drawn = &mut self.pose.turn[axis];
            *drawn += (self.accumulated[axis] - *drawn) * law.turn_ease;
            if self.accumulated[axis] > law.wrap {
                self.accumulated[axis] -= 2.0 * law.wrap;
                *drawn -= 2.0 * law.wrap;
            }
            if self.accumulated[axis] < -law.wrap {
                self.accumulated[axis] += 2.0 * law.wrap;
                *drawn += 2.0 * law.wrap;
            }
        }
        self.pose.push *= law.push_decay;
        self.pose.zoom += (target - self.pose.zoom) * law.zoom_ease;
        let stick = [
            PAN_SIGN[0] * input.left.0 * pull * law.pan_scale[0],
            PAN_SIGN[1] * input.left.1 * pull * law.pan_scale[1],
        ];
        for axis in 0..2 {
            // A pointer's pan is the zoomed craft's: it lets go as the zoom does.
            let zoomed = (self.pose.zoom / law.zoom_scale).clamp(0.0, 1.0);
            let aim = stick[axis] + self.grabbed_pan[axis] * zoomed;
            self.pose.pan[axis] += (aim - self.pose.pan[axis]) * law.pan_ease;
        }

        if self.idle < law.idle_frames {
            self.idle += 1;
        } else {
            for value in &mut self.accumulated {
                walk(value, law.idle_step);
            }
            self.wheel_pull = (self.wheel_pull - law.idle_step).max(0.0);
            for value in &mut self.grabbed_pan {
                *value -= value.clamp(-law.idle_step, law.idle_step);
            }
        }
    }

    /// Holds the craft `across` and `down` grid units further along, within the stick's own
    /// reach `law.pan_scale`: [`DRAG_REACH`] grid units of travel is all of it.
    fn grab_pan(&mut self, law: &ShipControl, across: f32, down: f32) {
        let reach = law.pan_scale;
        self.grabbed_pan[0] = (self.grabbed_pan[0] + PAN_SIGN[0] * across * reach[0] / DRAG_REACH)
            .clamp(-reach[0], reach[0]);
        self.grabbed_pan[1] = (self.grabbed_pan[1] - PAN_SIGN[1] * down * reach[1] / DRAG_REACH)
            .clamp(-reach[1], reach[1]);
    }
}

/// The original's idle step on one accumulator: it moves `step` toward zero and, if that crossed
/// zero, moves `step` back, so a value inside one step of zero stays where it is. **Read, not
/// designed**: the update's two comparisons both run on the same frame.
fn walk(value: &mut f32, step: f32) {
    if *value > 0.0 {
        *value -= step;
    }
    if *value < 0.0 {
        *value += step;
    }
}

#[cfg(test)]
mod tests;
