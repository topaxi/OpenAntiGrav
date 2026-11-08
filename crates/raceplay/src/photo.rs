//! Photo mode's camera: the one view every title shares.
//!
//! The race stands still, as in pause (`Race::tick` is not called), and this
//! flies the camera about a chosen craft. Nothing here reaches the simulation or
//! its hash: the pose goes out as the same [`CameraOverride`] a `--camera-pose`
//! run and the spectator director use. **The one exception is the rewind and the
//! shot's frames ([`rewind`], [`shot`])**, which move the race on screen through
//! its past with [`Race::tick`] and put its simulation back to the bit when the
//! mode closes.
//!
//! The laws are read off Pulse and Wipeout HD/Fury (`docs/ui/photo-mode.md`);
//! where they differ the better one is taken, and a value one title measured is
//! used on the rest unchanged:
//!
//! | Rule | Source |
//! | --- | --- |
//! | three lenses: `internal` (5 ahead of the nose), `close` (12 behind, 3 above, [`craft_view`]), `track` (the circuit's authored `Camera` node nearest the craft) | Pulse and HD, measured |
//! | `close` orbits on two free axes at [`ORBIT_RATE`] a tick, each wrapped at `+-2 pi` | HD; Pulse's pitch clamp is dropped, its eye went under the road anyway |
//! | `close` field of view `[10, 100]` from 65, `track` `[3, 40]` from 20 | HD, and 2048 for the limits |
//! | tilt at [`TILT_RATE`] a tick, pan within `+-5` | HD, read and not seen moving |
//! | the craft is chosen with a wrap, skipping craft that are out of the race | HD |
//! | no shot for [`GUARD_TICKS`] after entering | HD, 2048 |
//! | no collision with the road | Pulse and HD |
//!
//! **Chosen, not measured:** the zoom and pan rates, the bindings ([`Pad`]) and which lens
//! the mode opens on when a title's original has none.

use super::*;

pub mod effects;
pub mod rewind;
pub mod shot;

use effects::Effects;
use finish_camera::Subject;
use oag_render::camera::craft_view;
use oag_render::camera::destroy::Destroy;

/// Ticks after entering in which a shot is refused: `0.15` s at 60 Hz. It stops the button
/// that opened the mode from taking a picture.
pub const GUARD_TICKS: u32 = 9;

/// Radians per tick at full stick deflection, for the orbit and the tilt.
pub const ORBIT_RATE: f32 = 0.05;
/// Radians per tick the tilt turns at full deflection.
pub const TILT_RATE: f32 = 0.05;
/// Degrees of field per tick at full deflection.
pub const ZOOM_RATE: f32 = 0.6;
/// Units per tick the pan moves at full deflection.
pub const PAN_RATE: f32 = 0.1;
/// How far the pan may go either way, in units.
pub const PAN_LIMIT: f32 = 5.0;

/// The `close` lens's vertical field, in degrees: `(least, greatest, opening)`.
pub const CLOSE_FOV: (f32, f32, f32) = (10.0, 100.0, 65.0);
/// The `track` lens's vertical field, in degrees: `(least, greatest, opening)`.
pub const TRACK_FOV: (f32, f32, f32) = (3.0, 40.0, 20.0);

/// The stick deflection that counts as a push on the effects panel.
const PUSH: f32 = 0.5;
/// Ticks a push is held before it repeats, and between repeats once it does.
const REPEAT_AFTER: u32 = 14;
const REPEAT_EVERY: u32 = 3;

/// The turn at which an angle wraps.
const WRAP: f32 = std::f32::consts::TAU;

/// Which of the three cameras is up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lens {
    /// Five units ahead of the nose, looking where the craft flies.
    Internal,
    /// Twelve behind and three above, free to orbit, zoom, tilt and pan.
    Close,
    /// The circuit's own camera node, aimed at the craft.
    Track,
}

/// What the player's devices said this tick, already boiled down to what the mode reads.
///
/// **Bindings, chosen, not measured**: the abstract input has one stick, so the original's
/// two-stick layout is folded onto it with two modifiers, and every one is a button a keyboard
/// has. The stick (or the arrow keys) orbits `close` and zooms `track`; with
/// [`Self::select`] held it zooms (up and down) and tilts (left and right); with
/// [`Self::square`] held it pans. Either button pressed and released without moving the
/// stick is a tap: SELECT changes the lens, SQUARE the HUD. A pad's two sticks are the layout
/// ADR-0088 names, [`roles`]: the right orbits, the left pans and zooms.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Pad {
    /// The stick, `x` right and `y` up, each in `-1..=1`: the arrow keys and the d-pad, and the
    /// whole snapshot stick where no pad is attached.
    pub stick: (f32, f32),
    /// A pad's own left stick, apart from [`Self::stick`]. On `close` it pans (left and right)
    /// and zooms (up and down).
    pub left: (f32, f32),
    /// The right stick: it orbits `close`.
    pub orbit: (f32, f32),
    /// SELECT is down.
    pub select: bool,
    /// SQUARE is down.
    pub square: bool,
    /// TRIANGLE went down this tick: put everything back.
    pub reset: bool,
    /// L went down this tick: the previous craft.
    pub previous: bool,
    /// R went down this tick: the next craft.
    pub next: bool,
    /// START went down this tick: hide or show the help.
    pub help: bool,
    /// CROSS went down this tick: take the shot.
    pub shot: bool,
    /// CIRCLE went down this tick: leave.
    pub leave: bool,
}

/// What a tick of the mode asks of whoever runs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Only the camera moved, or nothing.
    None,
    /// Save the picture.
    Shot,
    /// Resume the race.
    Leave,
}

/// The mode's state: which lens, where it has been turned, which craft, what is shown.
#[derive(Debug, Clone, PartialEq)]
pub struct Photo {
    lens: Lens,
    /// Radians the `close` rig is turned about the craft's up. Wrapped at `+-2 pi`.
    yaw: f32,
    /// Radians the `close` rig is turned about the craft's right; positive lifts the eye.
    pitch: f32,
    /// Radians the view is rolled about its axis.
    tilt: f32,
    close_fov: f32,
    track_fov: f32,
    /// How far the `close` eye is moved sideways and up, in view space.
    pan: (f32, f32),
    subject: usize,
    hud: bool,
    help: bool,
    /// Ticks since entering, saturating.
    age: u32,
    /// Whether a circuit node is there to put the `track` lens on.
    track_available: bool,
    /// The picture controls, and whether their panel is up (the stick then moves its cursor
    /// and the camera stands still).
    effects: Effects,
    panel: bool,
    /// Ticks the stick has been pushed the same way on the panel, per axis: `(x, y)`.
    held: (i32, i32),
    held_ticks: (u32, u32),
    /// The zoom ring's numbers, for the speed blur: HD's, handed in by the composition root.
    zoom: Option<oag_post::hd_zoom::Tuning>,
    select_down: bool,
    select_used: bool,
    square_down: bool,
    square_used: bool,
    /// How many times the picture has jumped, for the temporal upscaler.
    cuts: u32,
    /// The override that was in force when the mode was entered.
    restore: Option<CameraOverride>,
    /// What the title's shot accumulates, when it does (`oag_title::photo::Accumulation`).
    accumulation: Option<oag_title::photo::Accumulation>,
    /// The eye moved across the lens for one frame of an accumulated depth of field.
    shift: Option<LensShift>,
}

/// The eye moved by `(dx, dy)` in view space and aimed at the point `focus` ahead of where it
/// was: one sample of the lens, so that the focus plane stays where it is and the rest of the
/// picture smears about it (an accumulated depth of field).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LensShift {
    /// Sideways, in world units.
    pub dx: f32,
    /// Up, in world units.
    pub dy: f32,
    /// The distance of the plane that stays sharp, in world units.
    pub focus: f32,
}

impl LensShift {
    /// `view` (world to camera, looking down `-Z`) for the shifted eye.
    #[must_use]
    pub fn apply(self, view: Mat4) -> Mat4 {
        let aim = Vec3::new(-self.dx, -self.dy, -self.focus).normalize();
        let turn = Quat::from_rotation_arc(aim, Vec3::NEG_Z);
        Mat4::from_quat(turn) * Mat4::from_translation(Vec3::new(-self.dx, -self.dy, 0.0)) * view
    }
}

impl Photo {
    /// The mode on `subject`, on the `internal` lens, with the HUD off and the help up.
    /// `track_available` is whether the circuit authors a camera node. `restore` is the
    /// camera override to put back on leaving.
    #[must_use]
    pub fn new(subject: usize, track_available: bool, restore: Option<CameraOverride>) -> Self {
        Self {
            lens: Lens::Internal,
            yaw: 0.0,
            pitch: 0.0,
            tilt: 0.0,
            close_fov: CLOSE_FOV.2,
            track_fov: TRACK_FOV.2,
            pan: (0.0, 0.0),
            subject,
            hud: false,
            help: true,
            age: 0,
            track_available,
            effects: Effects::new(),
            panel: false,
            held: (0, 0),
            held_ticks: (0, 0),
            zoom: None,
            select_down: false,
            select_used: false,
            square_down: false,
            square_used: false,
            cuts: 0,
            restore,
            accumulation: None,
            shift: None,
        }
    }

    /// The mode on a title whose shot accumulates frames, with the race able to rewind or not:
    /// the blur and rewind rows are on offer.
    #[must_use]
    pub fn offering(
        mut self,
        accumulation: Option<oag_title::photo::Accumulation>,
        rewinds: bool,
    ) -> Self {
        self.accumulation = accumulation;
        self.effects = self.effects.offering(accumulation.is_some(), rewinds);
        self
    }

    /// What the title's shot accumulates.
    #[must_use]
    pub fn accumulation(&self) -> Option<oag_title::photo::Accumulation> {
        self.accumulation
    }

    /// The lens sample the frame is drawn with, if an accumulated shot is under way.
    #[must_use]
    pub fn shift(&self) -> Option<LensShift> {
        self.shift
    }

    /// The mode as it opens with SELECT and SQUARE already down: their release is not a tap.
    #[must_use]
    pub fn holding(mut self, select: bool, square: bool) -> Self {
        self.select_down = select;
        self.select_used = select;
        self.square_down = square;
        self.square_used = square;
        self
    }

    /// The mode with the zoom ring's numbers the speed blur plays. Without them the row does
    /// nothing.
    #[must_use]
    pub fn with_zoom(mut self, tuning: Option<oag_post::hd_zoom::Tuning>) -> Self {
        self.zoom = tuning;
        self
    }

    /// The picture controls.
    #[must_use]
    pub fn effects(&self) -> &Effects {
        &self.effects
    }

    /// The picture controls, to change.
    pub fn effects_mut(&mut self) -> &mut Effects {
        &mut self.effects
    }

    /// Whether the effects panel is up.
    #[must_use]
    pub fn panel(&self) -> bool {
        self.panel
    }

    /// Shows or hides the effects panel, showing the help it sits in.
    pub fn toggle_panel(&mut self) {
        self.panel = !self.panel;
        if self.panel {
            self.help = true;
        }
        self.held = (0, 0);
        self.held_ticks = (0, 0);
    }

    /// START: help, then the effects panel, then neither.
    fn cycle_help(&mut self) {
        if !self.help {
            self.help = true;
        } else if !self.panel {
            self.toggle_panel();
        } else {
            self.help = false;
            self.panel = false;
        }
    }

    /// The zoom ring's numbers and the pulse the speed blur holds, when the row is above zero.
    #[must_use]
    pub fn speed_blur(&self) -> Option<(oag_post::hd_zoom::Tuning, oag_post::hd_zoom::Frame)> {
        let blur = self.effects.speed_blur();
        let tuning = self.zoom?;
        (blur > 0.0).then(|| {
            (
                tuning,
                oag_post::hd_zoom::Frame {
                    boost: blur,
                    // The original writes the damage pulse too, which tints the streak red;
                    // a speed blur does not read as damage here.
                    damage: 0.0,
                    size: [1.0, 1.0],
                    // A tick of the mode's own, far from any the race counts: the ring's
                    // history advances once per tick of it, and the race is not stepping.
                    tick: u64::MAX / 2 + u64::from(self.age),
                },
            )
        })
    }

    /// The lens that is up.
    #[must_use]
    pub fn lens(&self) -> Lens {
        self.lens
    }

    /// The craft in the picture, by grid slot.
    #[must_use]
    pub fn subject(&self) -> usize {
        self.subject
    }

    /// Whether the HUD is in the picture.
    #[must_use]
    pub fn hud(&self) -> bool {
        self.hud
    }

    /// Whether the help is up. Off, the screen shows the picture and nothing else.
    #[must_use]
    pub fn help(&self) -> bool {
        self.help
    }

    /// Whether a shot is allowed yet ([`GUARD_TICKS`]).
    #[must_use]
    pub fn armed(&self) -> bool {
        self.age >= GUARD_TICKS
    }

    /// How many times the picture has jumped to a different shot.
    #[must_use]
    pub fn cuts(&self) -> u32 {
        self.cuts
    }

    /// The `close` rig's turn: `(yaw, pitch)`.
    #[must_use]
    pub fn orbit(&self) -> (f32, f32) {
        (self.yaw, self.pitch)
    }

    /// The vertical field of view the lens draws with, in degrees.
    #[must_use]
    pub fn fov(&self) -> f32 {
        match self.lens {
            Lens::Internal => craft_view::FOV_DEGREES,
            Lens::Close => self.close_fov,
            Lens::Track => self.track_fov,
        }
    }

    /// The tilt, in radians.
    #[must_use]
    pub fn tilt(&self) -> f32 {
        self.tilt
    }

    /// The pan, in view-space units.
    #[must_use]
    pub fn pan(&self) -> (f32, f32) {
        self.pan
    }

    /// The override that was in force when the mode was entered.
    #[must_use]
    pub fn restore(&self) -> Option<CameraOverride> {
        self.restore
    }

    /// Whether `track` is a lens on this circuit.
    #[must_use]
    pub fn track_available(&self) -> bool {
        self.track_available
    }

    /// Takes the next lens in `internal, close, track`, leaving `track` out where there
    /// is no node. The picture is a different shot, so the upscaler's history is dropped.
    pub fn cycle_lens(&mut self) {
        self.lens = match self.lens {
            Lens::Internal => Lens::Close,
            Lens::Close if self.track_available => Lens::Track,
            Lens::Close | Lens::Track => Lens::Internal,
        };
        self.cuts = self.cuts.wrapping_add(1);
    }

    /// Turns the HUD in the picture on or off.
    pub fn toggle_hud(&mut self) {
        self.hud = !self.hud;
    }

    /// Puts the orbit, the tilt, the pan and both fields back to where entering left them.
    pub fn reset(&mut self) {
        self.yaw = 0.0;
        self.pitch = 0.0;
        self.tilt = 0.0;
        self.pan = (0.0, 0.0);
        self.close_fov = CLOSE_FOV.2;
        self.track_fov = TRACK_FOV.2;
        self.cuts = self.cuts.wrapping_add(1);
    }

    /// The craft before or after the one shown among `live`, wrapping, or the same one when
    /// no other is left. A subject no longer in `live` goes to the first that is.
    fn step_subject(&mut self, forward: bool, live: &[usize]) {
        let Some(next) = step_among(live, self.subject, forward) else {
            return;
        };
        if next != self.subject {
            self.subject = next;
            self.cuts = self.cuts.wrapping_add(1);
        }
    }

    /// One tick of the mode. `live` holds the grid slots of the craft still in the race, in
    /// order.
    pub fn step(&mut self, pad: Pad, live: &[usize]) -> Action {
        self.age = self.age.saturating_add(1);
        if pad.leave {
            return Action::Leave;
        }
        if pad.help {
            self.cycle_help();
        }
        if pad.reset {
            self.reset();
        }
        if pad.previous {
            self.step_subject(false, live);
        }
        if pad.next {
            self.step_subject(true, live);
        }
        if !live.contains(&self.subject) {
            self.step_subject(true, live);
        }
        if self.panel {
            self.push((
                crate::photo::larger(pad.stick.0, pad.left.0),
                crate::photo::larger(pad.stick.1, pad.left.1),
            ));
        } else {
            self.drive(pad);
        }
        self.taps(pad);
        if pad.shot && self.armed() {
            return Action::Shot;
        }
        Action::None
    }

    /// What the stick does on the effects panel: up and down move the cursor, left and right
    /// change the row. A push repeats while held.
    fn push(&mut self, (x, y): (f32, f32)) {
        let direction = |v: f32| {
            if v > PUSH {
                1
            } else if v < -PUSH {
                -1
            } else {
                0
            }
        };
        let (dx, dy) = (direction(x), direction(y));
        let fires = |dir: i32, held: &mut i32, ticks: &mut u32| {
            if dir == 0 || dir != *held {
                *held = dir;
                *ticks = 0;
                return dir != 0;
            }
            *ticks += 1;
            *ticks >= REPEAT_AFTER && (*ticks - REPEAT_AFTER).is_multiple_of(REPEAT_EVERY)
        };
        if fires(dy, &mut self.held.1, &mut self.held_ticks.1) {
            // Stick up is positive and the list reads downward.
            self.effects.step_row(-dy);
        }
        if fires(dx, &mut self.held.0, &mut self.held_ticks.0) {
            self.effects.adjust(dx);
        }
    }

    /// What the sticks do under the buttons held.
    fn drive(&mut self, pad: Pad) {
        let used = |(x, y): (f32, f32)| x.abs() > f32::EPSILON || y.abs() > f32::EPSILON;
        let moved = used(pad.stick) || used(pad.left);
        if moved && pad.select {
            self.select_used = true;
        }
        if moved && pad.square {
            self.square_used = true;
        }
        let Roles {
            orbit,
            pan,
            zoom,
            tilt,
        } = roles(pad, self.lens);
        match self.lens {
            Lens::Internal => {}
            Lens::Close => {
                self.pan.0 = (self.pan.0 + pan.0 * PAN_RATE).clamp(-PAN_LIMIT, PAN_LIMIT);
                self.pan.1 = (self.pan.1 + pan.1 * PAN_RATE).clamp(-PAN_LIMIT, PAN_LIMIT);
                self.close_fov =
                    (self.close_fov - zoom * ZOOM_RATE).clamp(CLOSE_FOV.0, CLOSE_FOV.1);
                self.tilt = wrap(self.tilt + tilt * TILT_RATE);
                self.yaw = wrap(self.yaw - orbit.0 * ORBIT_RATE);
                self.pitch = wrap(self.pitch + orbit.1 * ORBIT_RATE);
            }
            Lens::Track => {
                self.track_fov =
                    (self.track_fov - zoom * ZOOM_RATE).clamp(TRACK_FOV.0, TRACK_FOV.1);
                self.tilt = wrap(self.tilt + tilt * TILT_RATE);
            }
        }
    }

    /// SELECT and SQUARE let go without having been used as a modifier.
    fn taps(&mut self, pad: Pad) {
        if self.select_down && !pad.select && !self.select_used {
            self.cycle_lens();
        }
        if self.square_down && !pad.square && !self.square_used {
            self.toggle_hud();
        }
        if !pad.select {
            self.select_used = false;
        }
        if !pad.square {
            self.square_used = false;
        }
        self.select_down = pad.select;
        self.square_down = pad.square;
    }

    /// Where the camera is for `craft`, with `node` the circuit camera nearest it.
    ///
    /// `close` at rest is exactly [`craft_view::chase`]'s pose; the orbit turns the whole rig
    /// about the craft, so the eye keeps its distance at every angle. `track` falls back to
    /// `close` where `node` is `None`.
    #[must_use]
    pub fn pose(&self, craft: &Subject, node: Option<Vec3>) -> CameraOverride {
        let roll = Quat::from_rotation_z(self.tilt);
        let (eye, orientation) = match (self.lens, node) {
            (Lens::Internal, _) => {
                let pose = craft_view::nose(craft.position, craft.orientation);
                (pose.eye, pose.orientation * roll)
            }
            (Lens::Track, Some(node)) => {
                let aim = Destroy {
                    eye: node,
                    focus: craft.position,
                    focus_target: craft.position,
                    fov: self.track_fov,
                    focus_rate: 1.0,
                };
                let (_, orientation, _) = aim.to_world().to_scale_rotation_translation();
                (node, orientation * roll)
            }
            (Lens::Close | Lens::Track, _) => {
                let rig = craft.orientation
                    * Quat::from_rotation_y(self.yaw)
                    * Quat::from_rotation_x(-self.pitch);
                let offset = Vec3::new(0.0, craft_view::CHASE_UP, craft_view::CHASE_BEHIND);
                let orientation = rig * roll;
                let pan = orientation * Vec3::new(self.pan.0, self.pan.1, 0.0);
                (craft.position + rig * offset + pan, orientation)
            }
        };
        CameraOverride {
            eye,
            orientation,
            fov_deg: Some(self.fov()),
        }
    }
}

/// What the sticks mean on a lens, per tick at full deflection: the one place a stick is given a
/// role. **Chosen, not measured** (ADR-0088: the right stick orbits and the left pans and zooms;
/// HD's measured layout is the left stick orbiting and the right stick's Y zooming,
/// `docs/ghidra/functions/ps3-hdfury-eu/photo-mode.md`). The arrow keys and the d-pad keep the
/// one-stick fold they always had: they orbit, and SELECT or SQUARE held turn them into zoom and
/// tilt, or pan.
struct Roles {
    /// Yaw (left and right) and pitch (up and down).
    orbit: (f32, f32),
    /// Pan across and up.
    pan: (f32, f32),
    /// Zoom in (positive).
    zoom: f32,
    /// Tilt.
    tilt: f32,
}

fn roles(pad: Pad, lens: Lens) -> Roles {
    let (x, y) = pad.stick;
    let mut roles = Roles {
        orbit: (0.0, 0.0),
        pan: (0.0, 0.0),
        zoom: 0.0,
        tilt: 0.0,
    };
    match lens {
        Lens::Internal => {}
        Lens::Close if pad.square => roles.pan = (x, y),
        Lens::Close if pad.select => (roles.zoom, roles.tilt) = (y, x),
        Lens::Close => roles.orbit = (x, y),
        Lens::Track => (roles.zoom, roles.tilt) = (y, x),
    }
    match lens {
        Lens::Internal => {}
        Lens::Close => {
            roles.orbit = (roles.orbit.0 + pad.orbit.0, roles.orbit.1 + pad.orbit.1);
            roles.pan.0 += pad.left.0;
            roles.zoom += pad.left.1;
        }
        Lens::Track => {
            roles.zoom += pad.left.1;
            roles.tilt += pad.left.0;
        }
    }
    roles
}

/// Whichever is further from centre, sign kept.
fn larger(a: f32, b: f32) -> f32 {
    if b.abs() > a.abs() { b } else { a }
}

/// The craft before or after `current` among `live` (the grid slots still in the race, in
/// order), wrapping; the first of them when `current` is no longer there; `None` when none is.
/// Photo mode's rule, and the replay viewer's.
#[must_use]
pub fn step_among(live: &[usize], current: usize, forward: bool) -> Option<usize> {
    let &first = live.first()?;
    Some(
        match (live.iter().position(|&slot| slot == current), forward) {
            (Some(i), true) => live[(i + 1) % live.len()],
            (Some(i), false) => live[(i + live.len() - 1) % live.len()],
            (None, _) => first,
        },
    )
}

/// Wraps an angle into `[-2 pi, 2 pi]`.
fn wrap(angle: f32) -> f32 {
    if angle > WRAP {
        angle - WRAP
    } else if angle < -WRAP {
        angle + WRAP
    } else {
        angle
    }
}

impl Race {
    /// Opens photo mode on the player's craft. The world is not touched; the pose is imposed
    /// as a camera override, and whatever override was in force is put back by
    /// [`Self::photo_end`]. `select` and `square` say which of the two modifier buttons are down
    /// as the mode opens, so letting go of them is not read as a tap. `zoom` is the ring the
    /// speed blur row plays (HD's, handed in by the composition root); without it the row does
    /// nothing. `false` in a split screen or on a race with more than one view.
    pub fn photo_begin(
        &mut self,
        select: bool,
        square: bool,
        zoom: Option<&'static oag_title::ZoomRing>,
    ) -> bool {
        if self.viewport_count() > 1 || self.windowed() || self.view.photo.is_some() {
            return false;
        }
        let track = self
            .view
            .finish_camera
            .as_ref()
            .is_some_and(finish_camera::FinishCamera::has_nodes);
        let restore = self.view.camera_override;
        let subject = self.player_slot();
        self.view.photo = Some(
            Photo::new(subject, track, restore)
                .holding(select, square)
                .with_zoom(zoom.map(crate::zoom::tuning)),
        );
        self.photo_pose();
        true
    }

    /// Offers the blur and rewind rows to the mode that is open (`Photo::offering`).
    pub fn photo_offer(
        &mut self,
        accumulation: Option<oag_title::photo::Accumulation>,
        rewinds: bool,
    ) {
        let live = self.sim.world.tick;
        if let Some(photo) = self.view.photo.take() {
            let mut photo = photo.offering(accumulation, rewinds);
            photo.effects_mut().set_live(live);
            self.view.photo = Some(photo);
        }
    }

    /// Moves the lens sample the next draw is made with; `None` is the camera as it is.
    pub fn photo_set_shift(&mut self, shift: Option<LensShift>) {
        if let Some(photo) = self.view.photo.as_mut() {
            photo.shift = shift;
        }
    }

    /// Imposes the mode's pose for the craft positions the race stands at now: after the race
    /// has been moved to another tick.
    pub fn photo_repose(&mut self) {
        self.photo_pose();
    }

    /// Imposes `pose` as the camera, whatever the mode would put there: a ship blur holds the
    /// camera at the pose of the tick the shot is taken at.
    pub fn photo_hold(&mut self, pose: CameraOverride) {
        if self.view.photo.is_some() {
            self.view.camera_override = Some(pose);
        }
    }

    /// The camera the mode puts at the tick the race stands at now.
    #[must_use]
    pub fn photo_camera(&self) -> Option<CameraOverride> {
        let photo = self.view.photo.as_ref()?;
        let slot = photo.subject();
        let ship = &self.sim.world.ships[slot];
        let craft = Subject {
            slot,
            position: ship.physics.body.position,
            orientation: ship.physics.body.orientation,
        };
        let node = self
            .view
            .finish_camera
            .as_ref()
            .and_then(|director| director.node_eye_near(craft.position));
        Some(photo.pose(&craft, node))
    }

    /// Whether photo mode is open.
    #[must_use]
    pub fn photo(&self) -> Option<&Photo> {
        self.view.photo.as_ref()
    }

    /// Takes the next lens, as a click on the screen's camera chip does.
    pub fn photo_cycle_lens(&mut self) {
        if let Some(photo) = self.view.photo.as_mut() {
            photo.cycle_lens();
        }
    }

    /// Turns the HUD in the picture on or off, as a click on the screen's HUD chip does.
    pub fn photo_toggle_hud(&mut self) {
        if let Some(photo) = self.view.photo.as_mut() {
            photo.toggle_hud();
        }
    }

    /// Shows or hides the effects panel, as a click on the screen's EFFECTS chip does.
    pub fn photo_toggle_panel(&mut self) {
        if let Some(photo) = self.view.photo.as_mut() {
            photo.toggle_panel();
        }
    }

    /// Changes the picture controls, as a click on the panel does.
    pub fn photo_effects(&mut self, change: impl FnOnce(&mut effects::Effects)) {
        if let Some(photo) = self.view.photo.as_mut() {
            change(photo.effects_mut());
        }
    }

    /// The replay viewer's L1 and R1: follows the craft before or after the one the view follows
    /// among those still in the race, as photo mode's do (`step_among`). `false` when there was
    /// nothing to change to.
    pub fn step_followed(&mut self, forward: bool) -> bool {
        let live: Vec<usize> = self.spectator_subjects().iter().map(|s| s.slot).collect();
        let now = self.player_slot();
        match step_among(&live, now, forward) {
            Some(next) if next != now => {
                self.follow(next);
                true
            }
            _ => false,
        }
    }

    /// Makes the picture follow `slot`: its camera, HUD, sound and tags ([`Self::set_seat`], view
    /// only; the player's own craft is no seat at all). The jump is a camera cut, so the motion
    /// blur and the temporal history do not smear the old craft across the new one.
    pub fn follow(&mut self, slot: usize) {
        let before = self.player_slot();
        self.set_seat((slot != self.sim.world.primary_slot()).then_some(slot));
        if self.player_slot() != before {
            #[cfg(feature = "view")]
            {
                self.view.restores = self.view.restores.wrapping_add(1);
            }
        }
    }

    /// One tick of photo mode, and the pose it leaves. [`Action::None`] when it is not open.
    pub fn photo_step(&mut self, pad: Pad) -> Action {
        let live: Vec<usize> = self.spectator_subjects().iter().map(|s| s.slot).collect();
        let Some(photo) = self.view.photo.as_mut() else {
            return Action::None;
        };
        let action = photo.step(pad, &live);
        self.photo_pose();
        action
    }

    /// What the picture controls ask of the frame: `None` when the mode is closed or every
    /// control stands where it opened.
    #[must_use]
    pub fn photo_fx(&self, near: f32, far: f32) -> Option<oag_post::photo_fx::Params> {
        let photo = self.view.photo.as_ref()?;
        let mut params = photo.effects().params(near, far);
        // An accumulated depth of field frame keeps half of the single-frame pass's blur: the
        // other half is the lens sample it is drawn through (`shot`).
        if photo.shift().is_some()
            && let Some(dof) = params.dof.as_mut()
        {
            dof.aperture *= 0.5;
        }
        params.active().then_some(params)
    }

    /// Closes photo mode and puts the camera back as it was.
    pub fn photo_end(&mut self) {
        if let Some(photo) = self.view.photo.take() {
            self.view.camera_override = photo.restore();
        }
    }

    /// Imposes the mode's pose for this tick's craft positions.
    fn photo_pose(&mut self) {
        if self.view.photo.is_some() {
            self.view.camera_override = self.photo_camera();
        }
    }
}

#[cfg(test)]
mod tests;
