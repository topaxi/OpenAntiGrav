//! On-screen racing controls: where the controls are, which finger is on
//! which, and the [`Reading`] they add up to.
//!
//! **Chosen, not measured.** Nothing here is recovered from an original: no
//! Studio Liverpool title has a touch scheme, and the PSP's own controls are
//! the abstract buttons [`oag_gameplay::input::Button`] already names. The
//! layout, sizes and the labels the renderer draws are this project's own.
//!
//! The scheme, for a phone held in landscape:
//!
//! - **Left, a floating stick.** A finger that lands in the left 40% of the
//!   window below the top strip becomes the stick: where it landed is
//!   centre, and sliding it [`STICK_RADIUS`] of the window's height off
//!   centre is full lock. Horizontal is steering, vertical is pitch.
//! - **Right, buttons.** Accelerate ([`Button::Cross`]) is the big one under
//!   the right thumb, with fire ([`Button::Square`]) at its left in the
//!   thumb's arc.
//! - **Dynamic GO.** Where the finger is *within* GO, tracked while held, adds
//!   an airbrake: the bottom-left corner is [`Button::L`], the bottom-right is
//!   [`Button::R`], the centre column and everything above is thrust alone
//!   (see [`GoZone`]). Switched by [`Touches::set_go_zones`].
//! - **Upper left:** pause ([`Button::Start`]) and camera ([`Button::Select`]),
//!   with BRAKE L beside them in Standard when GO's zones are off, because
//!   Wipeout HD draws its shield meter at the top centre.
//!
//! Sizes are fractions of the window's height, so a button is the same
//! physical size on every phone: GO is 0.36 of the height (about 24 mm on a
//! 6.2 in phone), the small buttons 0.16 to 0.22 (11 to 15 mm), none under
//! the 9 to 12 mm a thumb needs. Everything is kept inside [`SAFE_X`] and
//! [`SAFE_Y`] of the edges for rounded corners and a camera cutout, and
//! clear of the HUD's four corner readouts.
//!
//! Every finger is tracked, so steering and accelerating at once is two
//! fingers and nothing else. A button finger re-reads what is under it as it
//! slides, so a thumb can roll from accelerate onto fire without lifting.
//!
//! Pure geometry and state, with no window and no winit type, so every rule
//! is a unit test. The renderer draws [`layout`]'s rectangles; it never
//! decides anything.

use oag_gameplay::controls::novice_airbrakes;
use oag_gameplay::input::Button;

use crate::pad::Reading;

/// How far a finger slides, as a fraction of the window's height, for the
/// stick to read full.
pub const STICK_RADIUS: f32 = 0.16;

/// The left share of the window's width where a finger becomes the stick.
pub const STICK_ZONE_WIDTH: f32 = 0.40;

/// The top strip, as a fraction of the height, that belongs to the airbrakes
/// and not to the stick.
pub const TOP_STRIP: f32 = 0.30;

/// How far past a button's drawn edge a finger still counts, as a fraction of
/// the window's height. A thumb is bigger than the rectangle under it.
pub const SLOP: f32 = 0.025;

/// How far in from the left and right edges the controls stay, as a fraction
/// of the window's height: a rounded corner or a landscape camera cutout.
/// **Chosen, not measured** - Android's real cutout is not queried.
pub const SAFE_X: f32 = 0.05;

/// How far in from the top and bottom edges the controls stay, as a fraction
/// of the height.
pub const SAFE_Y: f32 = 0.03;

/// The share of GO's width, from each side, that is a brake corner.
pub const GO_ZONE_SIDE: f32 = 0.42;

/// The share of GO's height, from the bottom, that carries the brake corners.
pub const GO_ZONE_BOTTOM: f32 = 0.50;

/// How deep into a brake zone, as a share of its extent, the pull already
/// reads full: a thumb need not reach GO's very edge, which is where ABSORB
/// starts. **Chosen, not measured.**
pub const GO_FULL_DEPTH: f32 = 0.85;

/// The share of GO's height, from the bottom, that is Easy's single BRAKE
/// band. **Chosen, not measured.**
pub const EASY_BAND: f32 = 0.40;

/// Easy's stick radius as a fraction of the window's height: bigger than
/// [`STICK_RADIUS`] so the brake band at the rim is reachable.
pub const EASY_STICK_RADIUS: f32 = 0.20;

/// Where Easy's steering reaches full lock, as a share of the stick's
/// radius; past it the same-side airbrake ramps in to the rim.
pub const EASY_STEER_FULL: f32 = 0.6;

/// Which on-screen scheme is live. **Chosen, not measured**; the original
/// novice scheme it mimics is `oag_gameplay::controls::novice_airbrakes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scheme {
    /// Separate stick, GO with analogue L/R corners, FIRE and ABSORB.
    #[default]
    Standard,
    /// The stick blends into the airbrake on its rim, and GO carries one
    /// BRAKE zone that picks its side off the steering.
    Easy,
}

impl Scheme {
    /// The setting token.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Easy => "easy",
        }
    }

    /// The scheme a setting token names.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "standard" => Some(Self::Standard),
            "easy" => Some(Self::Easy),
            _ => None,
        }
    }
}

/// The scheme and whether GO carries its brake zone(s).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Setup {
    /// The scheme.
    pub scheme: Scheme,
    /// `[controls] touch_go_zones`.
    pub zones: bool,
}

impl Setup {
    /// A setup.
    #[must_use]
    pub const fn new(scheme: Scheme, zones: bool) -> Self {
        Self { scheme, zones }
    }

    /// The stick's full-lock distance, as a fraction of the window's height.
    #[must_use]
    pub fn stick_radius(self) -> f32 {
        match self.scheme {
            Scheme::Standard => STICK_RADIUS,
            Scheme::Easy => EASY_STICK_RADIUS,
        }
    }
}

/// Easy's stick: `u` is the drag along the axis over the stick's radius, -1
/// to 1. Returns the steering and the airbrake **on the dragged side**: steering
/// is linear to full lock at [`EASY_STEER_FULL`], and past it the airbrake ramps
/// from 0 to 1 at the rim. **Chosen, not measured.**
#[must_use]
pub fn easy_curve(u: f32) -> (f32, f32) {
    let u = u.clamp(-1.0, 1.0);
    let steer = (u / EASY_STEER_FULL).clamp(-1.0, 1.0);
    let brake = ((u.abs() - EASY_STEER_FULL) / (1.0 - EASY_STEER_FULL)).clamp(0.0, 1.0);
    (steer, brake)
}

/// Under the simulation's own Novice scheme only `airbrake_right` is read
/// (it picks the side off the steering) and `airbrake_left` is ignored, so an
/// Easy reading's pull moves onto the right axis.
pub fn fold_for_novice_sim(reading: &mut Reading) {
    reading.airbrake_right = reading.airbrake_left.max(reading.airbrake_right);
    reading.airbrake_left = 0.0;
}

fn band_depth(fraction_y: f32, band: f32) -> f32 {
    ((fraction_y - (1.0 - band)) / (band * GO_FULL_DEPTH)).clamp(0.0, 1.0)
}

/// A rectangle in window pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
}

impl Rect {
    /// Where `at` falls in the rectangle, `(0, 0)` top left to `(1, 1)`
    /// bottom right, clamped: a finger in the slop reads as the nearest edge.
    #[must_use]
    pub fn fraction(self, at: (f32, f32)) -> (f32, f32) {
        (
            ((at.0 - self.x) / self.w.max(1.0)).clamp(0.0, 1.0),
            ((at.1 - self.y) / self.h.max(1.0)).clamp(0.0, 1.0),
        )
    }

    /// The rectangle's centre.
    #[must_use]
    pub fn centre(self) -> (f32, f32) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
}

/// Which airbrake corner of GO a finger is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoZone {
    /// The bottom-left corner: [`Button::L`].
    Left,
    /// The bottom-right corner: [`Button::R`].
    Right,
}

impl GoZone {
    /// The zone at `fraction` of GO's rectangle (see [`Rect::fraction`]):
    /// the bottom [`GO_ZONE_BOTTOM`] split left and right, with a dead
    /// centre column. **Chosen, not measured.**
    #[must_use]
    pub fn at(fraction: (f32, f32)) -> Option<Self> {
        if fraction.1 < 1.0 - GO_ZONE_BOTTOM {
            None
        } else if fraction.0 <= GO_ZONE_SIDE {
            Some(Self::Left)
        } else if fraction.0 >= 1.0 - GO_ZONE_SIDE {
            Some(Self::Right)
        } else {
            None
        }
    }

    /// How hard the airbrake is pulled with a finger at `fraction` of GO's
    /// rectangle, 0 to 1: **analogue, chosen, not measured**. Zero on the
    /// zone's inner edge (the dead centre column) and on its top, one in the
    /// outer bottom corner, the geometric mean of the depth on each axis so
    /// sliding down and sliding out both strengthen it.
    #[must_use]
    pub fn strength(self, fraction: (f32, f32)) -> f32 {
        let across = match self {
            Self::Left => GO_ZONE_SIDE - fraction.0,
            Self::Right => fraction.0 - (1.0 - GO_ZONE_SIDE),
        } / (GO_ZONE_SIDE * GO_FULL_DEPTH);
        let down = band_depth(fraction.1, GO_ZONE_BOTTOM);
        (across.clamp(0.0, 1.0) * down).sqrt()
    }

    fn bit(self) -> u8 {
        match self {
            Self::Left => 1,
            Self::Right => 2,
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
        }
    }
}

/// One on-screen button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    /// Thrust, [`Button::Cross`].
    Accelerate,
    /// The weapon, [`Button::Square`].
    Fire,
    /// Absorb, [`Button::Circle`].
    Absorb,
    /// The left airbrake, [`Button::L`].
    AirbrakeLeft,
    /// The right airbrake, [`Button::R`].
    AirbrakeRight,
    /// Pause, [`Button::Start`].
    Pause,
    /// The camera cycle, [`Button::Select`].
    Camera,
    /// Easy with GO's zone off: the bar's BRAKE half, which brakes the
    /// steered side (no button of its own).
    Brake,
}

impl Control {
    /// Every control, in the order [`layout`] lists them.
    pub const ALL: [Self; 8] = [
        Self::Accelerate,
        Self::Fire,
        Self::Absorb,
        Self::AirbrakeLeft,
        Self::AirbrakeRight,
        Self::Pause,
        Self::Camera,
        Self::Brake,
    ];

    /// The abstract button this control presses, `None` for Easy's BRAKE, which
    /// is an airbrake axis only.
    #[must_use]
    pub fn button(self) -> Option<Button> {
        Some(match self {
            Self::Accelerate => Button::Cross,
            Self::Fire => Button::Square,
            Self::Absorb => Button::Circle,
            Self::AirbrakeLeft => Button::L,
            Self::AirbrakeRight => Button::R,
            Self::Pause => Button::Start,
            Self::Camera => Button::Select,
            Self::Brake => return None,
        })
    }

    /// The short label the renderer draws on it. Chosen, plain words.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Accelerate => "GO",
            Self::Fire => "FIRE",
            Self::Absorb => "ABSORB",
            Self::AirbrakeLeft => "BRAKE L",
            Self::AirbrakeRight => "BRAKE R",
            Self::Pause => "PAUSE",
            Self::Camera => "VIEW",
            Self::Brake => "BRAKE",
        }
    }

    /// Whether this control is a press and not a hold: pause and the camera
    /// cycle act on the edge, so they reach the game only as a tap (see
    /// [`Touches::take_taps`]) and never as a held bit that could fire twice.
    #[must_use]
    pub fn is_momentary(self) -> bool {
        matches!(self, Self::Pause | Self::Camera)
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|c| *c == self).unwrap_or(0)
    }
}

/// Where every control sits with the separate airbrake buttons showing:
/// Standard with GO's zones off, the full set of seven.
///
/// **Chosen, not measured.** Placed for Pulse's HUD at a phone's 19.5:9: the
/// four corners hold lap, record, best/current time and speed/shield, so the
/// buttons keep to the free mid-height of each side.
#[must_use]
pub fn layout(size: (f32, f32)) -> Vec<(Control, Rect)> {
    layout_for(size, Setup::new(Scheme::Standard, false))
}

/// The controls on screen and under a finger. **Chosen, not measured.**
///
/// The right column is one width: FIRE above GO, a bar below it. The bar is
/// ABSORB, or in Easy with GO's zone off split into BRAKE on the left and
/// ABSORB on the right. Standard with the zone off keeps the separate BRAKE
/// L/R buttons (BRAKE L upper left with PAUSE and VIEW beside it, BRAKE R in
/// the thumb's arc left of GO);
/// otherwise those two sit in the top-left corner.
#[must_use]
pub fn layout_for(size: (f32, f32), setup: Setup) -> Vec<(Control, Rect)> {
    let h = size.1;
    let right = size.0 / h - SAFE_X;
    let r = |x: f32, y: f32, rw: f32, rh: f32| Rect {
        x: x * h,
        y: y * h,
        w: rw * h,
        h: rh * h,
    };
    let (go, go_y, fire, bar, gap) = (0.36, 0.34, 0.18, 0.11, 0.03);
    let x = right - go;
    let bar_y = go_y + go + gap;
    let (brake, brake_y, small) = (0.17, 0.20, 0.13);
    let separate = setup.scheme == Scheme::Standard && !setup.zones;
    let mut out = vec![
        (Control::Accelerate, r(x, go_y, go, go)),
        (Control::Fire, r(x, go_y - 0.02 - fire, go, fire)),
    ];
    if setup.scheme == Scheme::Easy && !setup.zones {
        let half = (go - 0.01) / 2.0;
        out.push((Control::Brake, r(x, bar_y, half, bar)));
        out.push((Control::Absorb, r(x + half + 0.01, bar_y, half, bar)));
    } else {
        out.push((Control::Absorb, r(x, bar_y, go, bar)));
    }
    let pause_x = if separate {
        SAFE_X + brake + gap
    } else {
        SAFE_X
    };
    let pause_y = if separate {
        brake_y + (brake - small) / 2.0
    } else {
        brake_y
    };
    out.push((Control::Pause, r(pause_x, pause_y, small, small)));
    out.push((
        Control::Camera,
        r(pause_x + small + gap, pause_y, small, small),
    ));
    if separate {
        out.push((Control::AirbrakeLeft, r(SAFE_X, brake_y, brake, brake)));
        out.push((
            Control::AirbrakeRight,
            r(x - gap - brake, go_y + (go - brake) / 2.0, brake, brake),
        ));
    }
    out
}

/// GO's rectangle.
#[must_use]
pub fn go_rect(size: (f32, f32)) -> Rect {
    layout_for(size, Setup::default())[0].1
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Role {
    /// The steering finger, with where it first landed.
    Stick { origin: (f32, f32) },
    /// On the buttons: whichever one is under it right now.
    Buttons,
    /// Landed on nothing, and stays nothing.
    Idle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Finger {
    id: u64,
    at: (f32, f32),
    role: Role,
    /// Set once the finger has been on GO: thrust then lasts until it lifts,
    /// wherever it slides.
    go: bool,
}

/// Every finger on the overlay, and what the overlay makes of them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Touches {
    fingers: Vec<Finger>,
    /// Controls pressed since the last [`Self::take_taps`], so a tap shorter
    /// than a tick is still a press.
    tapped: u8,
    /// Controls under a finger at the last [`Self::update`], one bit each.
    down: u8,
    /// The scheme and whether GO's brake zone(s) are live.
    setup: Setup,
    /// Easy's BRAKE pull, 0 to 1: GO's band or the bar's BRAKE half.
    brake: f32,
    /// GO zones under a finger, one bit each ([`GoZone`]).
    zones: u8,
    /// How hard each zone's airbrake is pulled, by [`GoZone::strength`].
    pulled: [f32; 2],
}

impl Touches {
    /// Sets the scheme and whether GO's brake zone(s) are live.
    pub fn set_setup(&mut self, setup: Setup, size: (f32, f32)) {
        self.setup = setup;
        self.refresh(size);
    }

    /// Easy's BRAKE pull, 0 to 1.
    #[must_use]
    pub fn brake_pull(&self) -> f32 {
        self.brake
    }

    /// Whether `zone` has a finger in it now.
    #[must_use]
    pub fn zone_down(&self, zone: GoZone) -> bool {
        self.zones & zone.bit() != 0
    }

    /// How hard `zone`'s airbrake is pulled, 0 to 1.
    #[must_use]
    pub fn zone_strength(&self, zone: GoZone) -> f32 {
        self.pulled[zone.index()]
    }

    /// A finger landed at `at` (window pixels) in a window of `size`.
    pub fn down(&mut self, id: u64, at: (f32, f32), size: (f32, f32)) {
        self.fingers.retain(|f| f.id != id);
        let stick_taken = self
            .fingers
            .iter()
            .any(|f| matches!(f.role, Role::Stick { .. }));
        let in_stick_zone =
            at.0 < size.0 * STICK_ZONE_WIDTH && at.1 > size.1 * TOP_STRIP && !stick_taken;
        let role = if control_at(at, size, self.setup).is_some() {
            Role::Buttons
        } else if in_stick_zone {
            Role::Stick { origin: at }
        } else {
            Role::Idle
        };
        let under = control_at(at, size, self.setup).filter(|_| role == Role::Buttons);
        let go = under == Some(Control::Accelerate);
        self.fingers.push(Finger { id, at, role, go });
        self.refresh(size);
        if let Some(control) = under {
            self.tapped |= 1 << control.index();
        }
    }

    /// A finger moved.
    pub fn moved(&mut self, id: u64, at: (f32, f32), size: (f32, f32)) {
        let Some(finger) = self.fingers.iter_mut().find(|f| f.id == id) else {
            return;
        };
        finger.at = at;
        let slid_onto = (finger.role == Role::Buttons)
            .then(|| control_at(at, size, self.setup))
            .flatten();
        finger.go |= slid_onto == Some(Control::Accelerate);
        self.refresh(size);
        if let Some(control) = slid_onto {
            self.tapped |= 1 << control.index();
        }
    }

    /// A finger lifted or was cancelled.
    pub fn up(&mut self, id: u64, size: (f32, f32)) {
        self.fingers.retain(|f| f.id != id);
        self.refresh(size);
    }

    /// Lets go of every finger, for a suspend or when the overlay hides.
    pub fn release_all(&mut self) {
        self.fingers.clear();
        self.tapped = 0;
        self.down = 0;
        self.zones = 0;
        self.pulled = [0.0; 2];
        self.brake = 0.0;
    }

    /// Whether any finger is on the overlay.
    #[must_use]
    pub fn any(&self) -> bool {
        !self.fingers.is_empty()
    }

    fn refresh(&mut self, size: (f32, f32)) {
        self.down = 0;
        self.zones = 0;
        self.pulled = [0.0; 2];
        self.brake = 0.0;
        let go = go_rect(size);
        for finger in &self.fingers {
            if finger.role != Role::Buttons {
                continue;
            }
            if finger.go {
                self.down |= 1 << Control::Accelerate.index();
            }
            let Some(control) = control_at(finger.at, size, self.setup) else {
                continue;
            };
            self.down |= 1 << control.index();
            let fraction = go.fraction(finger.at);
            match (control, self.setup) {
                (Control::Brake, _) => self.brake = 1.0,
                (
                    Control::Accelerate,
                    Setup {
                        zones: true,
                        scheme: Scheme::Easy,
                    },
                ) => {
                    if fraction.1 >= 1.0 - EASY_BAND {
                        self.brake = self.brake.max(band_depth(fraction.1, EASY_BAND));
                    }
                }
                (
                    Control::Accelerate,
                    Setup {
                        zones: true,
                        scheme: Scheme::Standard,
                    },
                ) => {
                    if let Some(zone) = GoZone::at(fraction) {
                        self.zones |= zone.bit();
                        let pull = zone.strength(fraction);
                        self.pulled[zone.index()] = self.pulled[zone.index()].max(pull);
                    }
                }
                _ => {}
            }
        }
    }

    /// Whether `control` has a finger on it now.
    #[must_use]
    pub fn is_down(&self, control: Control) -> bool {
        self.down & (1 << control.index()) != 0
    }

    /// The stick finger's centre and where it is now, for drawing the stick.
    #[must_use]
    pub fn stick(&self) -> Option<((f32, f32), (f32, f32))> {
        self.fingers.iter().find_map(|f| match f.role {
            Role::Stick { origin } => Some((origin, f.at)),
            _ => None,
        })
    }

    /// The controls pressed since the last call, as the buttons they press,
    /// and clears the latch. Fed to `Controls::tap` so a press shorter than
    /// one tick is not lost, exactly as a key's is.
    pub fn take_taps(&mut self) -> Vec<Button> {
        let tapped = std::mem::take(&mut self.tapped);
        Control::ALL
            .into_iter()
            .filter(|c| tapped & (1 << c.index()) != 0)
            .filter_map(Control::button)
            .collect()
    }

    /// What the overlay holds right now, for a window of `size`. The momentary
    /// controls are absent: they arrive through [`Self::take_taps`].
    #[must_use]
    pub fn reading(&self, size: (f32, f32)) -> Reading {
        let mut reading = Reading::default();
        for control in Control::ALL {
            if self.is_down(control)
                && !control.is_momentary()
                && let Some(button) = control.button()
            {
                reading.buttons |= button.bit();
            }
        }
        let easy = self.setup.scheme == Scheme::Easy;
        let mut rim = (0.0, 0.0);
        if let Some((origin, at)) = self.stick() {
            let radius = (size.1 * self.setup.stick_radius()).max(1.0);
            let (u, v) = ((at.0 - origin.0) / radius, -(at.1 - origin.1) / radius);
            // Screen y grows downward; up is positive.
            reading.stick_y = v.clamp(-1.0, 1.0);
            if easy {
                let (steer, brake) = easy_curve(u);
                reading.stick_x = steer;
                rim = if u < 0.0 { (brake, 0.0) } else { (0.0, brake) };
            } else {
                reading.stick_x = u.clamp(-1.0, 1.0);
            }
        }
        if easy {
            let (left, right) = novice_airbrakes(self.brake, reading.stick_x);
            reading.airbrake_left = left.max(rim.0);
            reading.airbrake_right = right.max(rim.1);
        } else {
            reading.airbrake_left = self.zone_strength(GoZone::Left);
            reading.airbrake_right = self.zone_strength(GoZone::Right);
        }
        reading
    }
}

/// The control under `at`: the one whose rectangle is nearest, a finger
/// inside a rectangle beating one only in another's slop.
#[must_use]
pub fn control_at(at: (f32, f32), size: (f32, f32), setup: Setup) -> Option<Control> {
    let slop = size.1 * SLOP;
    let off = |rect: &Rect| {
        let dx = (rect.x - at.0).max(at.0 - (rect.x + rect.w)).max(0.0);
        let dy = (rect.y - at.1).max(at.1 - (rect.y + rect.h)).max(0.0);
        dx.hypot(dy)
    };
    layout_for(size, setup)
        .into_iter()
        .filter(|(_, rect)| off(rect) <= slop)
        .min_by(|(_, a), (_, b)| {
            let d = |rect: &Rect| {
                let c = rect.centre();
                (c.0 - at.0).powi(2) + (c.1 - at.1).powi(2)
            };
            off(a).total_cmp(&off(b)).then(d(a).total_cmp(&d(b)))
        })
        .map(|(control, _)| control)
}

#[cfg(test)]
mod tests;
