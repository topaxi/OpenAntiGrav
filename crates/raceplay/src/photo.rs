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
//! A fourth lens, `free`, is ours: no original has one (`docs/ui/photo-mode.md`, "The free
//! lens"). **Chosen, not measured:** the zoom and pan rates, the free lens's speeds, bounds and
//! bindings, the bindings ([`Pad`]) and which lens the mode opens on when a title's original has
//! none.

use super::*;

pub mod effects;
pub mod mount;
pub mod rewind;
pub mod shot;
pub mod touch;

use effects::Effects;
use finish_camera::Subject;
use oag_render::camera::craft_view;
use oag_render::camera::destroy::Destroy;
use oag_render::camera::free::{Free, Motion};

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
/// The vertical field the free lens can start on, in degrees: wide enough for the picture of
/// either other lens, so choosing `free` from one starts on its field.
const FREE_START_FOV: (f32, f32) = (TRACK_FOV.0.min(CLOSE_FOV.0), TRACK_FOV.1.max(CLOSE_FOV.1));

/// Units per tick the free lens moves at full deflection before it has been held.
pub const FREE_WALK_RATE: f32 = 0.4;
/// How many times faster the free lens moves once a direction has been held [`FREE_RAMP_TICKS`].
pub const FREE_WALK_BOOST: f32 = 4.0;
/// Ticks of held movement over which the free lens speeds up to [`FREE_WALK_BOOST`].
pub const FREE_RAMP_TICKS: u32 = 90;
/// Radians per tick the free lens looks around at full right stick.
pub const FREE_LOOK_RATE: f32 = ORBIT_RATE;
/// Radians of look per grid unit a mouse or a finger drags.
pub const FREE_DRAG_LOOK: f32 = 0.005;
/// Units of movement per grid unit dragged, with the drag set to move.
pub const FREE_DRAG_MOVE: f32 = 0.1;
/// Units a wheel detent dollies the free lens.
pub const FREE_DOLLY: f32 = 2.0;
/// How far the free lens's eye may be from the craft in the picture, in units.
pub const FREE_RANGE: f32 = 400.0;

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
    /// A camera the player flies: it starts where the last lens left it and goes anywhere
    /// within [`FREE_RANGE`] of the craft.
    Free,
}

/// The free lens's camera: [`Free::position`] is the eye **relative to the craft in the picture**
/// (so a rewind or a ship change keeps the craft framed), the angles are the world's.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Rig {
    cam: Free,
    fov: f32,
    /// The least field a zoom reaches: the `close` lens's, or the seed's own when it started
    /// narrower (a `track` lens zoomed in).
    least_fov: f32,
    /// The seed's own orientation while its pitch is past what the rig can hold, drawn until the
    /// first turn so the picture does not jump when `free` is chosen.
    pinned: Option<Quat>,
    /// How far the eye may be from the craft: [`FREE_RANGE`], or the seed's own distance when
    /// the lens it was seeded from stood further out.
    reach: f32,
}

/// What the player's devices said this tick, already boiled down to what the mode reads.
///
/// **Bindings, chosen, not measured**: the abstract input has one stick, so the original's
/// two-stick layout is folded onto it with two modifiers, and every one is a button a keyboard
/// has. The stick (or the arrow keys) orbits `close` and zooms `track`; with
/// [`Self::select`] held it zooms (up and down) and tilts (left and right); with
/// [`Self::square`] held it pans. Either button pressed and released without moving the
/// stick is a tap: SELECT changes the lens, SQUARE the HUD. A pad's left stick is that stick; its
/// right stick zooms and tilts ([`roles`]).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Pad {
    /// The stick, `x` right and `y` up, each in `-1..=1`: the arrow keys and the d-pad, and the
    /// whole snapshot stick where no pad is attached.
    pub stick: (f32, f32),
    /// The right stick: up and down zoom, left and right tilt, on `close` and `track`.
    pub right: (f32, f32),
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
    /// How far a mouse or a finger dragged this tick, in grid units (`y` down): looks around on
    /// the free lens, or moves it with [`Photo::drag_moves`].
    pub drag: (f32, f32),
    /// Wheel detents this tick, positive toward the player: dollies the free lens.
    pub scroll: f32,
    /// What the fingers did this tick (`touch`).
    pub touch: touch::Touch,
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
    /// The free lens's camera, `None` until it is seeded from the picture on screen, and the
    /// seed TRIANGLE returns to.
    free: Option<Rig>,
    free_home: Option<Rig>,
    /// Whether a drag moves the free lens instead of turning it.
    drag_moves: bool,
    /// Ticks a free movement has been held, for the speed-up.
    walk_ticks: u32,
}

/// The eye moved by `(dx, dy)` in view space, with the frustum sheared so the plane `focus`
/// ahead stays where it was on the picture: one sample of a thin lens (an off-axis frustum),
/// so the focus plane holds across the whole frame and the rest smears about it (an
/// accumulated depth of field).
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
    /// `view` (world to camera, looking down `-Z`) for the shifted eye. A point at depth `z`
    /// gains `-(dx, dy) * z / focus` across the picture: on the plane `z = -focus` that undoes
    /// the eye's move exactly, at every point of the frame. A turn toward the focus point would
    /// hold only a sphere through it, sharp on the axis alone. Depth is left as it was, so the
    /// depth buffer still reads as distance from the eye.
    #[must_use]
    pub fn apply(self, view: Mat4) -> Mat4 {
        let focus = self.focus.max(f32::EPSILON);
        let shear = Mat4::from_cols(
            oag_core::math::Vec4::X,
            oag_core::math::Vec4::Y,
            oag_core::math::Vec4::new(-self.dx / focus, -self.dy / focus, 1.0, 0.0),
            oag_core::math::Vec4::W,
        );
        shear * Mat4::from_translation(Vec3::new(-self.dx, -self.dy, 0.0)) * view
    }
}

/// The zoom ring's pulse a speed blur of `effects` holds, with the ring's numbers, or `None` with
/// the row at zero. `age` is the mode's own tick: the ring's history advances once per tick of
/// it, and the race is not stepping. The scene stands still, so each tick's history draw is
/// repeated until it has settled (`oag_post::hd_zoom::SETTLE_PASSES`): a shot sees the history
/// the preview of the same scene has, however few frames the mode drew before it.
fn speed_blur_pulse(
    effects: &Effects,
    tuning: oag_post::hd_zoom::Tuning,
    age: u64,
) -> Option<(oag_post::hd_zoom::Tuning, oag_post::hd_zoom::Frame)> {
    let blur = effects.speed_blur();
    (blur > 0.0).then(|| {
        (
            tuning,
            oag_post::hd_zoom::Frame {
                boost: blur,
                // The original writes the damage pulse too, which tints the streak red; a speed
                // blur does not read as damage here.
                damage: 0.0,
                size: [1.0, 1.0],
                // Far from any tick the race counts.
                tick: u64::MAX / 2 + age,
                settle: oag_post::hd_zoom::SETTLE_PASSES,
            },
        )
    })
}

/// The picture controls of a frame drawn without photo mode open: a saved shot drawn again
/// (`oag-game --from-png`), which has the controls the shot was taken with and none of the mode's
/// camera or panel.
#[derive(Debug, Clone, Copy)]
pub struct FixedPicture {
    effects: Effects,
    zoom: Option<oag_post::hd_zoom::Tuning>,
    /// The eye moved across the lens for one frame of an accumulated shot.
    shift: Option<LensShift>,
}

impl FixedPicture {
    /// The controls `effects` holds, with the zoom ring the speed blur plays (none: the row does
    /// nothing, as in the mode).
    #[must_use]
    pub fn new(effects: Effects, zoom: Option<&'static oag_title::ZoomRing>) -> Self {
        Self {
            effects,
            zoom: zoom.map(crate::zoom::tuning),
            shift: None,
        }
    }

    /// The controls.
    #[must_use]
    pub fn effects(&self) -> &Effects {
        &self.effects
    }

    /// The lens sample the frame is drawn with, if an accumulated shot is being drawn again.
    #[must_use]
    pub fn shift(&self) -> Option<LensShift> {
        self.shift
    }

    /// The speed blur's ring and pulse, when the row is above zero.
    #[must_use]
    pub fn speed_blur(&self) -> Option<(oag_post::hd_zoom::Tuning, oag_post::hd_zoom::Frame)> {
        speed_blur_pulse(&self.effects, self.zoom?, 0)
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
            free: None,
            free_home: None,
            drag_moves: false,
            walk_ticks: 0,
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

    /// The mode opened on the free lens at `pose`, the camera the player was looking through
    /// when the pause began, with `craft` the position of the craft in the picture: the first
    /// frame is the one the pause left.
    #[must_use]
    pub fn opening_on(mut self, pose: CameraOverride, craft: Vec3) -> Self {
        self.lens = Lens::Free;
        self.seed_free(pose, craft);
        self
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
        self.panel.toggle();
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
        speed_blur_pulse(&self.effects, self.zoom?, u64::from(self.age))
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
            Lens::Free => self.free.map_or(self.close_fov, |rig| rig.fov),
        }
    }

    /// Whether the free lens is up and waiting for the picture it starts from
    /// ([`Self::seed_free`]).
    #[must_use]
    pub fn needs_free_seed(&self) -> bool {
        self.lens == Lens::Free && self.free.is_none()
    }

    /// Starts the free lens from `pose`, the camera on screen, with `craft` the position of the
    /// craft in the picture: the picture does not move. Also what TRIANGLE returns to.
    pub fn seed_free(&mut self, pose: CameraOverride, craft: Vec3) {
        let cam = Free::from_orientation(pose.eye - craft, pose.orientation);
        let fov = pose
            .fov_deg
            .unwrap_or(craft_view::FOV_DEGREES)
            .clamp(FREE_START_FOV.0, FREE_START_FOV.1);
        let rig = Rig {
            cam,
            fov,
            least_fov: CLOSE_FOV.0.min(fov),
            pinned: (cam.orientation().dot(pose.orientation).abs() < 1.0 - 1e-6)
                .then_some(pose.orientation),
            reach: FREE_RANGE.max(cam.position.length()),
        };
        self.free = Some(rig);
        self.free_home = Some(rig);
        self.walk_ticks = 0;
    }

    /// The free lens's eye relative to the craft, and its yaw, pitch and roll, once seeded.
    #[must_use]
    pub fn free_pose(&self) -> Option<(Vec3, f32, f32, f32)> {
        self.free
            .map(|rig| (rig.cam.position, rig.cam.yaw, rig.cam.pitch, rig.cam.roll))
    }

    /// Whether a drag moves the free lens (otherwise it turns it).
    #[must_use]
    pub fn drag_moves(&self) -> bool {
        self.drag_moves
    }

    /// Sets what a drag does on the free lens.
    pub fn toggle_drag_moves(&mut self) {
        self.drag_moves.toggle();
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

    /// Takes the next lens in `internal, close, track, free`, leaving `track` out where there
    /// is no node. The picture is a different shot, so the upscaler's history is dropped.
    pub fn cycle_lens(&mut self) {
        self.lens = match self.lens {
            Lens::Internal => Lens::Close,
            Lens::Close if self.track_available => Lens::Track,
            Lens::Close | Lens::Track => Lens::Free,
            Lens::Free => Lens::Internal,
        };
        if self.lens == Lens::Free {
            self.free = None;
            self.free_home = None;
        }
        self.cuts = self.cuts.wrapping_add(1);
    }

    /// Turns the HUD in the picture on or off.
    pub fn toggle_hud(&mut self) {
        self.hud.toggle();
    }

    /// Puts the orbit, the tilt, the pan and both fields back to where entering left them.
    pub fn reset(&mut self) {
        self.yaw = 0.0;
        self.pitch = 0.0;
        self.tilt = 0.0;
        self.pan = (0.0, 0.0);
        self.close_fov = CLOSE_FOV.2;
        self.track_fov = TRACK_FOV.2;
        self.free = self.free_home;
        self.walk_ticks = 0;
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
            self.push(pad.stick);
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
        let moved = used(pad.stick);
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
            walk,
            lift,
            look,
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
            Lens::Free => self.fly(pad, walk, lift, look, zoom, tilt),
        }
        self.apply_touch(pad.touch);
    }

    /// One tick of the free lens: the sticks in `roles`, then the pointer's drag and wheel.
    fn fly(
        &mut self,
        pad: Pad,
        walk: (f32, f32),
        lift: f32,
        look: (f32, f32),
        zoom: f32,
        tilt: f32,
    ) {
        let Some(mut rig) = self.free else {
            return;
        };
        let asked = walk.0 != 0.0 || walk.1 != 0.0 || lift != 0.0;
        self.walk_ticks = if asked {
            self.walk_ticks.saturating_add(1)
        } else {
            0
        };
        let ramp = (self.walk_ticks as f32 / FREE_RAMP_TICKS as f32).min(1.0);
        let speed = FREE_WALK_RATE * (1.0 + (FREE_WALK_BOOST - 1.0) * ramp);
        let (mut strafe, mut advance) = (walk.0 * speed, walk.1 * speed);
        let (mut yaw, mut pitch) = (look.0 * FREE_LOOK_RATE, look.1 * FREE_LOOK_RATE);
        let (dx, dy) = pad.drag;
        if self.drag_moves {
            strafe += dx * FREE_DRAG_MOVE;
            advance -= dy * FREE_DRAG_MOVE;
        } else {
            yaw += dx * FREE_DRAG_LOOK;
            pitch -= dy * FREE_DRAG_LOOK;
        }
        advance -= pad.scroll * FREE_DOLLY;
        if yaw != 0.0 || pitch != 0.0 || tilt != 0.0 {
            rig.pinned = None;
        }
        rig.cam = rig.cam.advance(
            &Motion {
                advance,
                strafe,
                lift: lift * speed,
                yaw,
                pitch,
                // A positive tilt rolls as the close lens's does, which turns the horizon
                // clockwise.
                roll: -tilt * TILT_RATE,
            },
            1.0,
        );
        rig.cam.position = rig.cam.position.clamp_length_max(rig.reach);
        rig.fov = (rig.fov - zoom * ZOOM_RATE).clamp(rig.least_fov, CLOSE_FOV.1);
        self.free = Some(rig);
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
        if let (Lens::Free, Some(rig)) = (self.lens, self.free) {
            return CameraOverride {
                eye: craft.position + rig.cam.position,
                orientation: rig.pinned.unwrap_or_else(|| rig.cam.orientation()),
                fov_deg: Some(rig.fov),
            };
        }
        let roll = Quat::from_rotation_z(self.tilt);
        let (eye, orientation) = match (self.lens, node) {
            (Lens::Internal, _) => {
                let pose = craft_view::nose(craft.position, craft.orientation);
                (pose.eye, pose.orientation * roll)
            }
            (Lens::Track, Some(node)) => {
                let aim = Destroy::posed(node, craft.position, self.track_fov, 1.0);
                let (_, orientation, _) = aim.to_world().to_scale_rotation_translation();
                (node, orientation * roll)
            }
            (Lens::Close | Lens::Track | Lens::Free, _) => {
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
/// role. HD's measured layout (`docs/ghidra/functions/ps3-hdfury-eu/photo-mode.md`, "Controls"):
/// the left stick, the arrow keys and the d-pad orbit `close` and, with SQUARE held, pan it; the
/// right stick's vertical axis zooms. **Chosen, not measured:** the right stick's horizontal axis
/// tilts (HD tilts on L2 and R2, which scrub the rewind row here), SELECT held turns the left
/// stick into zoom and tilt for a pad with no right stick, and `track` zooms and tilts on the
/// left stick too.
struct Roles {
    /// Yaw (left and right) and pitch (up and down).
    orbit: (f32, f32),
    /// Pan across and up.
    pan: (f32, f32),
    /// Zoom in (positive).
    zoom: f32,
    /// Tilt.
    tilt: f32,
    /// Free lens: across (right positive) and forward.
    walk: (f32, f32),
    /// Free lens: up the camera's own up.
    lift: f32,
    /// Free lens: turn right and look up.
    look: (f32, f32),
}

fn roles(pad: Pad, lens: Lens) -> Roles {
    let (x, y) = pad.stick;
    let mut roles = Roles {
        orbit: (0.0, 0.0),
        pan: (0.0, 0.0),
        zoom: 0.0,
        tilt: 0.0,
        walk: (0.0, 0.0),
        lift: 0.0,
        look: (0.0, 0.0),
    };
    match lens {
        Lens::Internal => return roles,
        Lens::Free => {
            // The left stick walks, SQUARE held slides and lifts, SELECT held zooms and rolls
            // (the close lens's modifiers); the right stick looks.
            if pad.select {
                (roles.zoom, roles.tilt) = (y, x);
            } else if pad.square {
                (roles.walk.0, roles.lift) = (x, y);
            } else {
                roles.walk = (x, y);
            }
            roles.look = pad.right;
            return roles;
        }
        Lens::Close if pad.square => roles.pan = (x, y),
        Lens::Close if pad.select => (roles.zoom, roles.tilt) = (y, x),
        Lens::Close => roles.orbit = (x, y),
        Lens::Track => (roles.zoom, roles.tilt) = (y, x),
    }
    roles.zoom += pad.right.1;
    roles.tilt += pad.right.0;
    roles
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
        if self.view.viewport_count() > 1 || self.windowed() || self.view.photo.is_some() {
            return false;
        }
        let track = self
            .view
            .finish_camera
            .as_ref()
            .is_some_and(finish_camera::FinishCamera::has_nodes);
        let restore = self.view.camera_override;
        let subject = self.core.player_slot();
        // The first frame is the one the pause left: the lens it opens on is the camera on
        // screen, free to be flown from there.
        let on_screen = self.on_screen_camera();
        let craft = self.core.sim.world.ships[subject].physics.body.position;
        self.view.photo = Some(
            Photo::new(subject, track, restore)
                .opening_on(on_screen, craft)
                .holding(select, square)
                .with_zoom(zoom.map(crate::zoom::tuning)),
        );
        self.photo_pose();
        true
    }

    /// The camera the picture is drawn from at the tick the race stands at: an imposed override
    /// as it is, else the view the race draws, with the field it authors (the speed widen is
    /// added again by the draw, so it is not part of this).
    fn on_screen_camera(&self) -> CameraOverride {
        if let Some(over) = self.view.camera_override {
            return over;
        }
        let (_, orientation, eye) = self
            .view
            .tick_view_unshaken(&self.core)
            .inverse()
            .to_scale_rotation_translation();
        let authored = if self.view.camera_view == oag_display::display::CameraView::Internal {
            self.view.internal_params.fov
        } else {
            self.view.chase_params.fov
        };
        CameraOverride {
            eye,
            orientation,
            fov_deg: Some(authored),
        }
    }

    /// Offers the blur and rewind rows to the mode that is open (`Photo::offering`).
    pub fn photo_offer(
        &mut self,
        accumulation: Option<oag_title::photo::Accumulation>,
        rewinds: bool,
    ) {
        let live = self.core.sim.world.tick;
        let records = self.recording();
        if let Some(photo) = self.view.photo.take() {
            let mut photo = photo.offering(accumulation, rewinds);
            photo.effects_mut().set_recording(records);
            photo.effects_mut().set_live(live);
            self.view.photo = Some(photo);
        }
    }

    /// The shot the open mode asks for at `tick`, or `None` when it is the one frame on screen
    /// ([`shot::Plan::new`]), with its lens ring sized in the field the frames are drawn with at
    /// `aspect` and the player's `setting`.
    #[must_use]
    pub fn photo_plan(
        &self,
        tick: u64,
        resimulates: bool,
        aspect: f32,
        setting: oag_display::display::Fov,
    ) -> Option<shot::Plan> {
        shot::Plan::new(
            self.photo()?,
            tick,
            resimulates,
            self.drawn_fov_degrees(aspect, setting),
        )
    }

    /// Moves the lens sample the next draw is made with; `None` is the camera as it is.
    pub fn photo_set_shift(&mut self, shift: Option<LensShift>) {
        if let Some(photo) = self.view.photo.as_mut() {
            photo.shift = shift;
        } else if let Some(picture) = self.view.picture.as_mut() {
            picture.shift = shift;
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
        let (craft, node) = self.craft_and_node(photo.subject())?;
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
        self.photo_seed_free();
    }

    /// Switches what a drag does on the free lens, as a click on the screen's drag chip does.
    pub fn photo_toggle_drag(&mut self) {
        if let Some(photo) = self.view.photo.as_mut() {
            photo.toggle_drag_moves();
        }
    }

    /// Starts the free lens, if it was just chosen, from the camera on screen, so the picture
    /// does not move.
    fn photo_seed_free(&mut self) {
        let Some(photo) = self.view.photo.as_ref().filter(|p| p.needs_free_seed()) else {
            return;
        };
        let craft = self.core.sim.world.ships[photo.subject()]
            .physics
            .body
            .position;
        let Some(pose) = self.view.camera_override.or_else(|| self.photo_camera()) else {
            return;
        };
        if let Some(photo) = self.view.photo.as_mut() {
            photo.seed_free(pose, craft);
        }
    }

    /// Turns the HUD in the picture on or off, as a click on the screen's HUD chip does.
    pub fn photo_toggle_hud(&mut self) {
        if let Some(photo) = self.view.photo.as_mut() {
            photo.toggle_hud();
        }
    }

    /// Hides or shows the help and the panel, as a tap on the empty picture does.
    pub fn photo_toggle_chrome(&mut self) {
        if let Some(photo) = self.view.photo.as_mut() {
            photo.toggle_chrome();
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
        let live: Vec<usize> = self
            .core
            .sim
            .spectator_subjects()
            .iter()
            .map(|s| s.slot)
            .collect();
        let now = self.core.player_slot();
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
        let before = self.core.player_slot();
        self.set_seat((slot != self.core.sim.world.primary_slot()).then_some(slot));
        if self.core.player_slot() != before {
            #[cfg(feature = "view")]
            {
                self.view.restores = self.view.restores.wrapping_add(1);
            }
        }
    }

    /// One tick of photo mode, and the pose it leaves. [`Action::None`] when it is not open.
    pub fn photo_step(&mut self, pad: Pad) -> Action {
        let live: Vec<usize> = self
            .core
            .sim
            .spectator_subjects()
            .iter()
            .map(|s| s.slot)
            .collect();
        let Some(photo) = self.view.photo.as_mut() else {
            return Action::None;
        };
        let action = photo.step(pad, &live);
        self.photo_seed_free();
        self.photo_pose();
        action
    }

    /// What the picture controls ask of the frame: `None` when the mode is closed or every
    /// control stands where it opened.
    #[must_use]
    pub fn photo_fx(&self, near: f32, far: f32) -> Option<oag_post::photo_fx::Params> {
        // An accumulated depth of field frame is drawn with the preview's own pass: the lens
        // sample only refines it (`shot`, "Depth of field").
        let effects = match (&self.view.photo, &self.view.picture) {
            (Some(photo), _) => photo.effects(),
            (None, Some(fixed)) => fixed.effects(),
            (None, None) => return None,
        };
        let params = effects.params(near, far);
        params.active().then_some(params)
    }

    /// Draws every frame with `picture`'s controls, as a saved shot was taken with them.
    pub fn set_picture(&mut self, picture: FixedPicture) {
        self.view.picture = Some(picture);
    }

    /// Whether every frame is drawn with a saved shot's controls (`set_picture`).
    #[must_use]
    pub fn has_fixed_picture(&self) -> bool {
        self.view.picture.is_some()
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
