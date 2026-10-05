//! The lock-on reticle: where its five pieces go, and when it says "locked".
//!
//! **Recovered whole** from `HudSight_Update` (`0x0881dbcc`), confidence 88 -
//! evidence and per-claim confidences on
//! `docs/ghidra/functions/psp-pulse-usa/lock-sight.md`. Everything in this file
//! is the original's arithmetic; the short list of what is not is under
//! [What is ours](#what-is-ours).
//!
//! The reticle is four corner brackets around a box plus one closed box in the
//! middle. The brackets sit at `±extent` from a *smoothed* centre that chases
//! the locked craft's projected position; the inner one leads them, offset
//! toward the true position by at most `0.4` of the extent. That lag is the
//! whole visual: the brackets are seen closing.
//!
//! # What is ours
//!
//! - **The gate.** The original guards its projection block with a condition
//!   this project has read but not understood - see the page's "The gate is the
//!   one part not read". `oag_game::race::Race` drives this from "the held weapon
//!   locks and something is lockable" instead, which is what the weapon plays
//!   like, and says so.
//! - **The screen-space sign convention.** The original computes
//!   `136 + 136 * y/w` against a clip space whose `y` runs down; this engine's
//!   runs up, so [`project`] flips it. Same number, our axis.
//! - **Nothing else.** The extents, the rates, the `0.8`, the `0.7`, the four
//!   rotations, the `0.4` inner clamp, the `250.0` range and the `w > 0` guard
//!   are all read out of the function.
//!
//! # Why this lives in a gameplay crate
//!
//! It draws nothing. It is arithmetic over a view-projection matrix and a world
//! position, and the pixels are `oag_hud::sight_draw`'s. Nothing here
//! reaches `oag_render`, `oag_audio`, `oag_input`, `winit` or `wgpu`, which is
//! what let it move down out of `oag-game` on 2026-09-09 - and moving it is
//! what breaks the `hud -> race` module cycle that had `oag_hud`
//! reaching back up into `oag_game::race` for these five types.

use oag_core::math::{Mat4, Vec3, Vec4};

/// The screen the PSP titles author every HUD number for.
///
/// **A default, not the only answer.** Wipeout HD authors its HUD - the sights
/// included - in 1920x1080, and the reticle has to project into whichever grid
/// the layout it is drawn beside uses or it lands in the top-left corner of it.
/// [`Sight::new`] takes the real one off `oag_hud::Assets::space`; this is
/// what [`Sight::default`] uses and what the PSP titles pass.
///
/// **The placement law does not change with it.** The original's `240` and `136`
/// are `screen/2` on its own screen, and every sight widget on every title is
/// authored at a placeholder whose centre is `(-w/2, +h/2)` - so the arithmetic
/// is the same and only the number differs. See
/// `docs/ghidra/functions/psp-pulse-usa/lock-sight.md`.
pub const SCREEN: [f32; 2] = [480.0, 272.0];

/// How far the reticle may be from the camera and still be drawn, in world
/// units.
///
/// **Recovered, confidence 90** - `HudSight_Update` tests
/// `length(eye_space) < 250.0` before it projects. It is a range on the *sight*
/// and not on the lock, and the two are not comparable numbers: this is a
/// straight-line distance in eye space, while `Ship_AcquireLock`'s window is
/// **longitudinal**, so a craft inside either weapon's authored far bound can
/// still be further than this away and have no reticle drawn over it.
///
/// **It is a code literal, not an authored one**, which is worth saying here
/// because the LeachBeam's block authors a `range` attribute at the very same
/// figure. That is a coincidence of tuning, not the same number twice, and
/// `oag_tables::weapons::LeachBeamStats` deliberately does not decode it.
pub const DRAW_RANGE: f32 = 250.0;

/// How long a target must stay on screen before the lock takes, in seconds.
///
/// **Recovered, confidence 90**, and it is the answer to a question
/// `missile.md` recorded as open for months: `HudSight_Update` accumulates `dt`
/// into a hold timer whenever the target projects on screen, resets it to zero
/// the moment it does not, and only sets the lock flag - `entity+0x860 & 1`,
/// the one `Ship_FireHeldWeapon` gates on - when that timer is past `0.8` **and**
/// the reticle has caught up with the target this frame.
///
/// So a lock is not a property of geometry alone. Two craft in identical
/// positions differ by how long one of them has been held there.
///
/// **Not HD's own hold time.** `docs/ghidra/functions/ps3-hdfury-eu/hud-sight.md`
/// reads HD's shared hold constant directly out of memory as `0.5`, not the
/// PSP's `0.8` above - a real, measured divergence this project has not
/// adopted: it is shared, title-blind engine code judged against the PSP's
/// own recovered law, and changing it for HD alone is a simulation-behaviour
/// change and a determinism-hash move, out of scope for a presentation-only
/// lane. Left at `0.8` on every title, deliberately.
pub const HOLD_SECONDS: f32 = 0.8;

/// The reticle's half-extent when it has nothing, in screen pixels.
///
/// **Recovered, confidence 88.** The brackets sit this far out and wait.
pub const EXTENT_OPEN: f32 = 30.0;

/// The half-extent while a target is on screen but not yet locked.
pub const EXTENT_SEEKING: f32 = 9.6;

/// The half-extent once the lock is taken. The brackets close this far.
pub const EXTENT_LOCKED: f32 = 6.0;

/// How fast the extent eases toward its target, in pixels a second.
///
/// **Recovered, confidence 85** - `dt * 50.0`, taken [`EXTENT_SNAP`] times
/// faster while the current extent is above the one this frame is easing
/// toward, which is the direction that snaps shut rather than the one that
/// opens.
pub const EXTENT_RATE: f32 = 50.0;

/// The extra rate applied while the extent is shrinking toward its target.
///
/// **The comparison is against the frame's own target**, not against
/// [`EXTENT_SEEKING`]: the original tests the current extent against the value
/// it is heading for, so the threshold is `6.0` once locked and `9.6` before.
pub const EXTENT_SNAP: f32 = 1.4;

/// The base speed the reticle centre chases at, in pixels a second.
///
/// **Recovered, confidence 88** - `step = dt * 30.0`, then multiplied by the
/// rate below. Split into two constants because the original does: one is a
/// distance a frame, the other is unitless.
pub const CHASE_STEP: f32 = 30.0;

/// How much the vertical counts for, when deciding whether the reticle arrived.
///
/// **Recovered, confidence 90** - the literal `0x3f333333` = `0.7`, applied to
/// `dy` before the distance and **divided back out** of the step afterwards.
/// The two are not the same operation once the step is clamped, which is why
/// both halves are reproduced rather than cancelled.
pub const VERTICAL_WEIGHT: f32 = 0.7;

/// How much of the extent the inner box may lead the brackets by.
///
/// **Recovered, confidence 88** - the offset from the smoothed centre to the
/// true projected point, clamped to `0.4 * extent`.
pub const INNER_LEAD: f32 = 0.4;

/// The four rotations the corner brackets are drawn at, in radians.
///
/// **Recovered, confidence 92**, and they are *rotations* rather than mirrors:
/// the original writes an angle to `widget+0xac` for each of the four, as the
/// literals `0x3fc90fdb`, `0x40490fdb`, `0` and `0x4096cbe4`. One corner
/// bracket makes four corners no other way.
///
/// Indexed the way the corners below are: `[-e,-e]`, `[+e,-e]`, `[-e,+e]`,
/// `[+e,+e]`.
pub const BRACKET_ROTATIONS: [f32; 4] = [
    std::f32::consts::FRAC_PI_2,
    std::f32::consts::PI,
    0.0,
    std::f32::consts::PI + std::f32::consts::FRAC_PI_2,
];

/// The alpha a near target's reticle is drawn at, out of 255.
pub const ALPHA_NEAR: f32 = 255.0;

/// The alpha a far target's reticle is drawn at.
///
/// **Recovered, confidence 80 on the value; the "far" test itself is not a
/// tuning constant at all.** Read further 2026-09-16: `near` compares the
/// projected depth against `g_hud_sight_depth_sample`
/// (`0x08ab0a4c`), which is not a fixed global - it is written once a frame by
/// `Hud_SampleSightDepth` (`0x08819b24`) from a live read of the rendered
/// frame's own Z-buffer at the reticle's *previous* on-screen position (the
/// call site, `FUN_0890906c` at `0x0890906c`, reuses an existing per-frame
/// depth-buffer sampling pass shared with an unidentified post-process effect,
/// so "near" is really "is the target's own depth closer than whatever
/// geometry the reticle was sitting over last frame" - a one-frame-lagged
/// screen-space occlusion test, not a softer range band on top of
/// [`DRAW_RANGE`] as this constant's own name suggests). See
/// `docs/ghidra/functions/psp-pulse-usa/lock-sight.md#the-far-target-alpha-step-is-a-live-depth-buffer-sample-not-a-tuning-constant`.
///
/// **Still not reproduced, now for a structural reason rather than a missing
/// read.** Reproducing it needs the rendered frame's own depth buffer, which
/// `oag_race` cannot reach - rule 1 in `CLAUDE.md`'s two dependency rules
/// forbids a gameplay crate depending on `oag-render`. [`Sight::update`] keeps
/// treating every drawn target as near, unchanged.
pub const ALPHA_FAR: f32 = 96.0;

/// How fast alpha eases, in units of 255 a second.
pub const ALPHA_RATE: f32 = 1000.0;

/// What the dim half of the seeking blink multiplies the colour by.
///
/// **Recovered, confidence 85** - `value * 0xc0 >> 8`, which is `0.75`, written
/// into all three colour channels on that phase where the other phase writes
/// the unscaled value. See [`Sight::tint`] for what is and is not ported of it.
pub const BLINK_TINT: f32 = 0.75;

/// How long each half of the seeking blink lasts, in seconds.
///
/// **Recovered, confidence 85** - a `0.1` accumulator toggling a flag, so 5 Hz
/// on and off, 10 toggles a second. It runs the whole time and is only *spent*
/// while the reticle is not locked.
pub const BLINK_PERIOD: f32 = 0.1;

/// The four corner-bracket widgets of the Missile's reticle, in the order the
/// original's slot run binds them.
///
/// **Recovered** from `HudSight_Bind` (`0x0881b604`), which looks each up by
/// name under the `"HUD->"` path. All four instance one model - see
/// [`is_sight_widget`].
pub const MISSILE_BRACKETS: [&str; 4] = [
    "missile_sight_1",
    "missile_sight_2",
    "missile_sight_3",
    "missile_sight_4",
];

/// The closed box at the middle of the Missile's reticle.
pub const MISSILE_INNER: &str = "missile_sight_inner";

/// The LeachBeam's four, which are hollow arrowheads rather than brackets.
///
/// **Four instances of one model, exactly as the Missile's four are** -
/// `Data\HUD\leachbeam_sight.vex` on all four, at the same `(-240, 136)`
/// placeholder, measured off `pulse-psp-usa.chd`'s own `Arcade_HUD.xml`.
///
/// **There is no LeachBeam inner.** `HudSight_Bind` (`0x0881b604`) binds nine
/// widgets over three models and only the Missile gets a closed box; the
/// LeachBeam's reticle is four arrowheads pointing inward and nothing in the
/// middle.
///
/// # Their law is their own, read 2026-10-01
///
/// `HudSight_Update` (`0x0881dbcc`) writes the **Missile's** five widgets. The
/// LeachBeam's four are written by `FUN_0881e8c8`, which `Hud_Update` runs when
/// the first returns no lock: a different law - spinning, no hold timer - and it
/// is [`leach`]'s. The four take [`BRACKET_ROTATIONS`] **plus the figure's spin**
/// (`piece+0xac = spin + {pi/2, pi, 0, 3pi/2}`), which is what the original
/// writes; a title that does not run that law (Wipeout HD, whose reticle is
/// concentric rings) keeps the quarter turns alone.
pub const LEACHBEAM_BRACKETS: [&str; 4] = [
    "leachbeam_sight_1",
    "leachbeam_sight_2",
    "leachbeam_sight_3",
    "leachbeam_sight_4",
];

/// Which of the two lockable weapons the reticle is being drawn for.
///
/// **Recovered as a distinction the original makes, not one added here.**
/// `Ship_AcquireLock` (`0x08844784`) switches on the held weapon id to pick
/// which pair of `<Stats>` offsets its window comes from - `+0x50`/`+0x54` for
/// the Missile, `+0x114`/`+0x118` for the LeachBeam - and `HudSight_Bind`
/// (`0x0881b604`) binds a separate set of four widgets for each. Everything
/// between those two ends is shared, which is why this is an enum on one
/// [`Sight`] rather than two sights.
///
/// It is what picks the *art*: [`MISSILE_BRACKETS`] plus [`MISSILE_INNER`], or
/// [`LEACHBEAM_BRACKETS`] and no inner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Held {
    /// Four corner brackets around a closed box.
    Missile,
    /// Four hollow arrowheads pointing inward, and nothing in the middle.
    LeachBeam,
}

/// Whether a `<Mode3D><Model>` widget is one of the nine lock-on sights.
///
/// By name, the way the original binds them, rather than by the model each
/// names: four widgets share `missile_sight_outer.vex` and four share
/// `leachbeam_sight.vex`, so the model is not what tells them apart.
#[must_use]
pub fn is_sight_widget(name: &str) -> bool {
    MISSILE_BRACKETS.contains(&name) || name == MISSILE_INNER || LEACHBEAM_BRACKETS.contains(&name)
}

/// Where a target is on screen, and how far away it is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projected {
    /// Screen position in the 480x272 space, `y` down.
    pub screen: [f32; 2],
    /// Distance from the camera in world units, which drives the zoom.
    pub distance: f32,
}

/// Projects a world position into the HUD's screen space.
///
/// Returns `None` for a target behind the camera or past [`DRAW_RANGE`].
///
/// **`w > 0` is the original's own guard**, not one added here, and it is the
/// one that matters: a craft behind the camera divides by a negative `w` and
/// lands at a mirrored on-screen point that looks entirely plausible in a still
/// frame. The `0.9` lock cone makes that rare rather than impossible.
#[must_use]
pub fn project(
    view: Mat4,
    view_projection: Mat4,
    world: Vec3,
    screen: [f32; 2],
) -> Option<Projected> {
    let eye = view * Vec4::new(world.x, world.y, world.z, 1.0);
    let distance = Vec3::new(eye.x, eye.y, eye.z).length();
    if distance >= DRAW_RANGE {
        return None;
    }
    let clip = view_projection * Vec4::new(world.x, world.y, world.z, 1.0);
    if clip.w <= 0.0 {
        return None;
    }
    let x = screen[0] * 0.5 + screen[0] * 0.5 * (clip.x / clip.w);
    // The original's `136 + 136 * y/w` against a clip space whose `y` runs
    // down. Ours runs up, so the sign flips and the number does not.
    let y = screen[1] * 0.5 - screen[1] * 0.5 * (clip.y / clip.w);
    Some(Projected {
        screen: [x, y],
        distance,
    })
}

/// What the reticle is doing, and what the tone plays because of it.
///
/// **Recovered**: the original publishes exactly these three values into
/// `DAT_002aca54`, which `HudSight_UpdateTone` (`0x0881b34c`) turns into one
/// `~ROCKLOCK` voice with a parameter. See `lock-sight.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum State {
    /// Nothing to lock, or nothing on screen. The voice is stopped.
    #[default]
    Absent,
    /// A target is on screen and the brackets are closing. Parameter `0`.
    Seeking,
    /// The lock is taken. Parameter `1`.
    Locked,
}

/// One of the five pieces of the reticle, placed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Piece {
    /// Centre in screen space.
    pub centre: [f32; 2],
    /// Rotation in radians, clockwise about that centre. Zero for the inner.
    pub rotation: f32,
}

/// The reticle's live state for one player.
///
/// Held on `oag_game::race::Race` rather than in `World`, for the reason
/// `Race::exhaust` is: it is what the screen shows, it must not move a
/// determinism hash, and the simulation does not know a renderer exists.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sight {
    /// The smoothed centre the brackets are drawn around (`hud+0xc0`/`+0xc4`).
    centre: [f32; 2],
    /// The half-extent (`hud+0x280`).
    extent: f32,
    /// How long the target has been on screen (`hud+0xe8` - the slot the widget
    /// bind skips, which is what made it look like a tenth widget).
    hold: f32,
    /// Whether the lock is taken this frame (`hud+0xc8`).
    locked: bool,
    /// Where the target actually projected, kept so the inner can lead.
    target: Option<[f32; 2]>,
    /// The previous frame's projection, for the original's `2*new - old`
    /// on-screen test.
    previous: Option<[f32; 2]>,
    /// Eased alpha, 0..255.
    alpha: f32,
    /// The 5 Hz blink accumulator and its flag.
    blink_timer: f32,
    blink: bool,
    /// The grid this reticle's coordinates are in.
    ///
    /// The title's own, off `oag_hud::Assets::space` - see [`SCREEN`] for
    /// why it is a field rather than a constant.
    screen: [f32; 2],
    /// Which weapon's art the reticle is wearing.
    ///
    /// **Sticky on purpose.** The brackets are drawn while the extent is still
    /// easing back open after a target is lost - the original's own rule, see
    /// [`Self::visible`] - and swapping the art out from under that would make
    /// the closing arrowheads finish as brackets. So this holds its last value
    /// until another lockable weapon is picked up, and [`Self::visible`] is what
    /// decides whether anything is drawn at all.
    held: Held,
    /// The figure's turn angle, radians (`hud+0x104`) - the LeachBeam's own law
    /// only. See [`leach`].
    spin: f32,
    /// Whether the previous frame held a target on screen (`view+0xf1`), which
    /// is what makes a first sighting close in from open. The LeachBeam's law.
    leach_seen: bool,
    /// Whether a held LeachBeam runs its own law rather than the Missile's.
    leach_law: bool,
}

impl Default for Sight {
    /// A reticle on the PSP's own screen. See [`Sight::new`].
    fn default() -> Self {
        Self::new(SCREEN)
    }
}

impl Sight {
    /// A reticle in the grid `screen` names.
    #[must_use]
    pub fn new(screen: [f32; 2]) -> Self {
        Self {
            screen,
            centre: [screen[0] * 0.5, screen[1] * 0.5],
            extent: EXTENT_OPEN,
            hold: 0.0,
            locked: false,
            target: None,
            previous: None,
            alpha: 0.0,
            blink_timer: 0.0,
            blink: false,
            held: Held::Missile,
            spin: 0.0,
            leach_seen: false,
            leach_law: false,
        }
    }

    /// The grid this reticle's coordinates are in.
    #[must_use]
    pub fn screen(&self) -> [f32; 2] {
        self.screen
    }

    /// Which weapon's art the reticle is wearing.
    ///
    /// The draw path picks its widget set off this and **not** off whatever the
    /// sight happens to be pointing at: the reticle belongs to the weapon in the
    /// player's slot, and a target is a target whichever weapon found it.
    #[must_use]
    pub fn held(&self) -> Held {
        self.held
    }

    /// Records which lockable weapon the reticle is now being drawn for.
    ///
    /// Called once a tick from `Race::update_sight`, and only when the held
    /// weapon is one of the two that lock - see [`Sight::held`] for why it is
    /// sticky the rest of the time.
    pub fn set_held(&mut self, held: Held) {
        self.held = held;
    }

    /// One frame of the reticle, given where its target projected.
    ///
    /// `target` is `None` when nothing is lockable, when the held weapon does
    /// not lock, or when [`project`] refused - all three are the same thing to
    /// this function, which is the original's arrangement.
    ///
    /// Returns what the tone should be doing.
    ///
    /// # The order is the original's and two steps of it look wrong
    ///
    /// 1. **The on-screen test is run against `2 * new - old`**, a one-frame
    ///    extrapolation, and what is *drawn* is the smoothed centre. So a target
    ///    about to leave the screen stops counting a frame early.
    /// 2. **The lock is taken only on the frame the reticle arrives.** Being
    ///    held past [`HOLD_SECONDS`] is necessary and not sufficient: the
    ///    brackets have to have caught up in the same frame.
    pub fn update(&mut self, dt: f32, target: Option<Projected>) -> State {
        if self.runs_leach_law() {
            return self.update_leach(dt, target);
        }
        self.leach_seen = false;
        self.blink_timer -= dt;
        while self.blink_timer < 0.0 {
            self.blink_timer += BLINK_PERIOD;
            self.blink = !self.blink;
        }

        // The extrapolated point is what decides "on screen"; the raw one is
        // what everything else uses.
        let visible = target.and_then(|p| {
            let previous = self.previous.unwrap_or(p.screen);
            let extrapolated = [
                2.0 * p.screen[0] - previous[0],
                2.0 * p.screen[1] - previous[1],
            ];
            let on_screen = (0.0..self.screen[0]).contains(&extrapolated[0])
                && (0.0..self.screen[1]).contains(&extrapolated[1]);
            on_screen.then_some((extrapolated, p.distance))
        });
        self.previous = target.map(|p| p.screen);

        self.hold = if visible.is_some() {
            self.hold + dt
        } else {
            0.0
        };

        let was_locked = self.locked;
        self.locked = false;
        let (aim, zoom) = match visible {
            Some((screen, distance)) => (screen, (CHASE_STEP / distance).clamp(1.0, 5.0)),
            // The centre is left where it is and the brackets open; the
            // original does not recentre.
            None => (self.centre, 1.0),
        };
        self.target = visible.map(|(screen, _)| screen);

        let step = dt * CHASE_STEP;
        let dx = aim[0] - self.centre[0];
        let dy = (aim[1] - self.centre[1]) * VERTICAL_WEIGHT;
        let distance = (dx * dx + dy * dy).sqrt();

        let mut rate = 1.0;
        if visible.is_some() {
            rate = (distance * 0.12 + 1.5) * 1.1;
            if self.hold > HOLD_SECONDS && was_locked {
                rate = 15.0;
            }
            rate *= zoom;
        }

        if distance < step * rate {
            self.centre = aim;
            if visible.is_some() && self.hold > HOLD_SECONDS {
                self.locked = true;
            }
        } else {
            let t = step * rate / distance;
            self.centre[0] += dx * t;
            self.centre[1] += dy * t / VERTICAL_WEIGHT;
        }

        let unzoomed = if visible.is_some() {
            if self.locked {
                EXTENT_LOCKED
            } else {
                EXTENT_SEEKING
            }
        } else {
            EXTENT_OPEN
        };
        // **Against this frame's own target, not against [`EXTENT_SEEKING`].**
        // The original compares the current extent with the value it is easing
        // toward, taken *before* the zoom - so a locked reticle snaps 1.4x
        // faster anywhere above six, and one still seeking only above 9.6.
        let ease = dt
            * EXTENT_RATE
            * if self.extent > unzoomed {
                EXTENT_SNAP
            } else {
                1.0
            };
        self.extent = ease_toward(self.extent, unzoomed * zoom, ease);

        let alpha_target = if visible.is_some() { ALPHA_NEAR } else { 0.0 };
        self.alpha = ease_toward(self.alpha, alpha_target, dt * ALPHA_RATE);

        match (visible.is_some(), self.locked) {
            (false, _) => State::Absent,
            (true, false) => State::Seeking,
            (true, true) => State::Locked,
        }
    }

    /// Whether anything is drawn at all.
    ///
    /// **Recovered**: the original shows all five while the target is visible
    /// *or* while the extent is still easing, so the brackets are watched
    /// opening again after a target is lost rather than vanishing with it.
    #[must_use]
    pub fn visible(&self) -> bool {
        self.target.is_some() || (self.extent - EXTENT_OPEN).abs() > 0.01
    }

    /// Whether the lock is taken.
    #[must_use]
    pub fn locked(&self) -> bool {
        self.locked
    }

    /// The smoothed centre the brackets are drawn around.
    #[must_use]
    pub fn centre(&self) -> [f32; 2] {
        self.centre
    }

    /// Where the target actually is on screen, or `None` when there is none.
    ///
    /// The point [`Self::centre`] is chasing, after the original's one-frame
    /// extrapolation. The two are equal on exactly the frames the reticle
    /// arrives - which is also the only kind of frame a lock is taken on, so
    /// `locked() && centre() != aim()` is impossible by construction.
    #[must_use]
    pub fn aim(&self) -> Option<[f32; 2]> {
        self.target
    }

    /// The eased alpha, 0..1.
    #[must_use]
    pub fn alpha(&self) -> f32 {
        (self.alpha / 255.0).clamp(0.0, 1.0)
    }

    /// How far into the hold window the target has been continuously visible,
    /// 0..1 - `0` the moment a target is (re)acquired, `1` once held past
    /// [`HOLD_SECONDS`].
    ///
    /// **For Wipeout HD's progressive LeachBeam reveal.** HD's own
    /// `Hud_UpdateLeachBeamSight` shows one of its three outer rings at a time,
    /// switching at exact quarters of *its own* `0.5` s hold constant - see
    /// `docs/ghidra/functions/ps3-hdfury-eu/hud-sight.md#the-leachbeams-four-reveal-one-at-a-time-gated-by-hold-time-not-distance`.
    /// This engine keeps the PSP's shared [`HOLD_SECONDS`] (`0.8`) rather than
    /// adopting HD's `0.5` - a separate, unadopted divergence recorded on
    /// [`HOLD_SECONDS`]'s own doc - so what this exposes is the *shape*, a
    /// hold window divided into quarters, scaled onto whichever hold constant
    /// is active rather than HD's own absolute second marks.
    #[must_use]
    pub fn hold_progress(&self) -> f32 {
        (self.hold / HOLD_SECONDS).clamp(0.0, 1.0)
    }

    /// What the reticle's colour is multiplied by this frame, as a brightness
    /// alone - no hue.
    ///
    /// **For Wipeout HD's own-coloured concentric widgets.** HD authors each
    /// ring's colour itself (a red outer, a green inner - see
    /// `oag_title::hud::Sights::Concentric`), so multiplying by [`Self::tint`]'s
    /// recovered PSP hue here would repaint HD's own art rather than dim it.
    /// Nothing has read whether HD's own sight code spends a blink the same
    /// way that PSP reading (see [`Self::tint`]'s doc) does, so this keeps
    /// the brightness-only behaviour this engine already had before the hue
    /// below was recovered, unchanged for this dialect.
    ///
    /// **The blink is a tint and never a hide**, which is worth stating because
    /// the obvious reading of "blink" is the wrong one and this file had it
    /// wrong: `HudSight_Update` writes a colour on *both* phases and drops the
    /// draw on neither. While the reticle is seeking, one phase takes the full
    /// value and the other takes it scaled by `0xc0 >> 8` - [`BLINK_TINT`],
    /// recovered as a literal. Once locked the blink stops being spent and the
    /// colour holds.
    #[must_use]
    pub fn brightness(&self) -> f32 {
        if self.locked || !self.blink {
            1.0
        } else {
            BLINK_TINT
        }
    }

    /// What colour the PSP dialect's white corner-bracket models are tinted
    /// this frame - `[r, g, b]`, multiplied by [`Self::alpha`] at the draw site.
    ///
    /// **Recovered, confidence 88.** `HudSight_Update` (`0x0881dbcc`) builds a
    /// packed colour word and hands it to `Image_SetVertexColours`
    /// (`0x089122b4`); this project's own established byte order for that
    /// word - byte0 = R, byte1 = G, byte2 = B, byte3 = A, the same one
    /// `Loading_DrawWave`'s `0xff000000` -> `0xff808080` ramp already fixed
    /// (`crates/render/src/loading.rs`, `docs/formats/psp-texture.md`) - reads
    /// the literal construction as:
    ///
    /// - **Locked**: pure red (`R` = the eased alpha byte, `G` = `B` = `0`).
    /// - **Seeking, blink phase off**: yellow at full brightness (`R` = `G` =
    ///   the alpha byte, `B` = `0`).
    /// - **Seeking, blink phase on**: white at [`BLINK_TINT`] the brightness
    ///   (`R` = `G` = `B` = the alpha byte scaled by `0xc0 >> 8`).
    ///
    /// The colour word's own alpha byte is always `0xff` - the eased value this
    /// project already carries as [`Self::alpha`] scales the RGB channels
    /// instead, which is the same operation under the models' `Additive` blend
    /// (`src.rgb * src.a`) as scaling `src.a` alone would be with `rgb = 1`, so
    /// nothing here is a second, uncounted fade. See
    /// `docs/ghidra/functions/psp-pulse-usa/lock-sight.md#colour-and-blink-resolved-the-byte-order-and-the-two-tints`.
    #[must_use]
    pub fn tint(&self) -> [f32; 3] {
        if self.runs_leach_law() {
            // `0xff` and `0xffff` under the same byte order: red when locked,
            // yellow while seeking, and no blink.
            return if self.locked {
                [1.0, 0.0, 0.0]
            } else {
                [1.0, 1.0, 0.0]
            };
        }
        if self.locked {
            [1.0, 0.0, 0.0]
        } else if self.blink {
            [BLINK_TINT, BLINK_TINT, BLINK_TINT]
        } else {
            [1.0, 1.0, 0.0]
        }
    }

    /// The four corner brackets, in the order [`BRACKET_ROTATIONS`] indexes.
    #[must_use]
    pub fn brackets(&self) -> [Piece; 4] {
        if self.runs_leach_law() {
            return self.leach_pieces();
        }
        let e = self.extent;
        let [cx, cy] = self.centre;
        let corners = [
            [cx - e, cy - e],
            [cx + e, cy - e],
            [cx - e, cy + e],
            [cx + e, cy + e],
        ];
        std::array::from_fn(|i| Piece {
            centre: corners[i],
            rotation: BRACKET_ROTATIONS[i],
        })
    }

    /// The inner box, which leads the brackets toward the true position.
    #[must_use]
    pub fn inner(&self) -> Piece {
        let [cx, cy] = self.centre;
        let limit = INNER_LEAD * self.extent;
        let Some(target) = self.target else {
            return Piece {
                centre: self.centre,
                rotation: 0.0,
            };
        };
        let mut dx = target[0] - cx;
        let mut dy = target[1] - cy;
        let length = (dx * dx + dy * dy).sqrt();
        if length > limit && length > 0.0 {
            let t = limit / length;
            dx *= t;
            dy *= t;
        }
        Piece {
            centre: [cx + dx, cy + dy],
            rotation: 0.0,
        }
    }
}

/// Moves `current` toward `wanted` by at most `step`.
///
/// The original writes this out twice, once for the extent and once for the
/// alpha, in the same shape both times: assign the target, then walk back to
/// the clamped value if the step did not cover the gap.
fn ease_toward(current: f32, wanted: f32, step: f32) -> f32 {
    if current < wanted {
        (current + step).min(wanted)
    } else {
        (current - step).max(wanted)
    }
}

mod leach;
pub use leach::{
    LEACH_EXTENT_CLOSED, LEACH_OPEN_RATE, LEACH_SPIN_LOCKED, LEACH_SPIN_SEEKING,
    LEACH_UNHELD_REFERENCE,
};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod leach_tests;
