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
//! - **Upper corners, below the HUD's readouts:** the airbrakes
//!   ([`Button::L`], [`Button::R`]) and absorb ([`Button::Circle`]); pause
//!   ([`Button::Start`]) and camera ([`Button::Select`]) at top centre.
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
pub const GO_ZONE_SIDE: f32 = 0.34;

/// The share of GO's height, from the bottom, that carries the brake corners.
pub const GO_ZONE_BOTTOM: f32 = 0.34;

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
    fn contains(self, at: (f32, f32), slop: f32) -> bool {
        at.0 >= self.x - slop
            && at.0 <= self.x + self.w + slop
            && at.1 >= self.y - slop
            && at.1 <= self.y + self.h + slop
    }

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

    /// The abstract button this zone adds.
    #[must_use]
    pub fn button(self) -> Button {
        match self {
            Self::Left => Button::L,
            Self::Right => Button::R,
        }
    }

    fn bit(self) -> u8 {
        match self {
            Self::Left => 1,
            Self::Right => 2,
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
}

impl Control {
    /// Every control, in the order [`layout`] lists them.
    pub const ALL: [Self; 7] = [
        Self::Accelerate,
        Self::Fire,
        Self::Absorb,
        Self::AirbrakeLeft,
        Self::AirbrakeRight,
        Self::Pause,
        Self::Camera,
    ];

    /// The abstract button this control presses.
    #[must_use]
    pub fn button(self) -> Button {
        match self {
            Self::Accelerate => Button::Cross,
            Self::Fire => Button::Square,
            Self::Absorb => Button::Circle,
            Self::AirbrakeLeft => Button::L,
            Self::AirbrakeRight => Button::R,
            Self::Pause => Button::Start,
            Self::Camera => Button::Select,
        }
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

/// Where every control sits in a window of `size` pixels. Everything scales
/// off the height, so the buttons keep their size on a wider phone.
///
/// **Chosen, not measured.** Placed for Pulse's HUD at a phone's 19.5:9: the
/// four corners hold lap, record, best/current time and speed/shield, so the
/// buttons keep to the free mid-height of each side and the top centre, and
/// GO's bottom ends above the speed bar.
#[must_use]
pub fn layout(size: (f32, f32)) -> [(Control, Rect); 7] {
    let (w, h) = size;
    let right = w / h - SAFE_X;
    let r = |x: f32, y: f32, rw: f32, rh: f32| Rect {
        x: x * h,
        y: y * h,
        w: rw * h,
        h: rh * h,
    };
    let go = (0.36, 0.36);
    let go_y = 0.40;
    let fire = 0.22;
    let brake = (0.30, 0.16);
    let brake_y = 0.18;
    let absorb = 0.20;
    [
        (Control::Accelerate, r(right - go.0, go_y, go.0, go.1)),
        (
            Control::Fire,
            r(right - go.0 - 0.03 - fire, go_y + go.1 - fire, fire, fire),
        ),
        (
            Control::Absorb,
            r(
                right - brake.0 - 0.03 - absorb,
                brake_y - 0.02,
                absorb,
                absorb,
            ),
        ),
        (Control::AirbrakeLeft, r(SAFE_X, brake_y, brake.0, brake.1)),
        (
            Control::AirbrakeRight,
            r(right - brake.0, brake_y, brake.0, brake.1),
        ),
        (Control::Pause, r(w / h / 2.0 - 0.19, SAFE_Y, 0.17, 0.11)),
        (Control::Camera, r(w / h / 2.0 + 0.02, SAFE_Y, 0.17, 0.11)),
    ]
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
    /// Whether GO's brake corners are live.
    go_zones: bool,
    /// GO zones under a finger, one bit each ([`GoZone`]).
    zones: u8,
}

impl Touches {
    /// Switches GO's brake corners on or off (`false` makes GO thrust only).
    pub fn set_go_zones(&mut self, on: bool, size: (f32, f32)) {
        self.go_zones = on;
        self.refresh(size);
    }

    /// Whether `zone` has a finger in it now.
    #[must_use]
    pub fn zone_down(&self, zone: GoZone) -> bool {
        self.zones & zone.bit() != 0
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
        let role = if control_at(at, size).is_some() {
            Role::Buttons
        } else if in_stick_zone {
            Role::Stick { origin: at }
        } else {
            Role::Idle
        };
        self.fingers.push(Finger { id, at, role });
        self.refresh(size);
        if let Some(control) = control_at(at, size).filter(|_| role == Role::Buttons) {
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
            .then(|| control_at(at, size))
            .flatten();
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
    }

    /// Whether any finger is on the overlay.
    #[must_use]
    pub fn any(&self) -> bool {
        !self.fingers.is_empty()
    }

    fn refresh(&mut self, size: (f32, f32)) {
        self.down = 0;
        self.zones = 0;
        let go = layout(size)[Control::Accelerate.index()].1;
        for finger in &self.fingers {
            if finger.role == Role::Buttons
                && let Some(control) = control_at(finger.at, size)
            {
                self.down |= 1 << control.index();
                if control == Control::Accelerate
                    && self.go_zones
                    && let Some(zone) = GoZone::at(go.fraction(finger.at))
                {
                    self.zones |= zone.bit();
                }
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
            .map(Control::button)
            .collect()
    }

    /// What the overlay holds right now, for a window of `size`. The momentary
    /// controls are absent: they arrive through [`Self::take_taps`].
    #[must_use]
    pub fn reading(&self, size: (f32, f32)) -> Reading {
        let mut reading = Reading::default();
        for control in Control::ALL {
            if self.is_down(control) && !control.is_momentary() {
                reading.buttons |= control.button().bit();
            }
        }
        for zone in [GoZone::Left, GoZone::Right] {
            if self.zone_down(zone) {
                reading.buttons |= zone.button().bit();
            }
        }
        if let Some((origin, at)) = self.stick() {
            let radius = (size.1 * STICK_RADIUS).max(1.0);
            reading.stick_x = ((at.0 - origin.0) / radius).clamp(-1.0, 1.0);
            // Screen y grows downward; up is positive.
            reading.stick_y = (-(at.1 - origin.1) / radius).clamp(-1.0, 1.0);
        }
        reading
    }
}

/// The control under `at`, nearest centre first when two slops overlap.
#[must_use]
pub fn control_at(at: (f32, f32), size: (f32, f32)) -> Option<Control> {
    let slop = size.1 * SLOP;
    layout(size)
        .into_iter()
        .filter(|(_, rect)| rect.contains(at, slop))
        .min_by(|(_, a), (_, b)| {
            let d = |rect: &Rect| {
                let c = rect.centre();
                (c.0 - at.0).powi(2) + (c.1 - at.1).powi(2)
            };
            d(a).total_cmp(&d(b))
        })
        .map(|(control, _)| control)
}

#[cfg(test)]
mod tests;
