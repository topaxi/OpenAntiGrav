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
//!   the right thumb, with fire ([`Button::Square`]) beside it and absorb
//!   ([`Button::Circle`]) above it.
//! - **Top, the airbrakes** at the two corners ([`Button::L`], [`Button::R`]),
//!   and pause ([`Button::Start`]) and camera ([`Button::Select`]) between.
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

    /// The rectangle's centre.
    #[must_use]
    pub fn centre(self) -> (f32, f32) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
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
#[must_use]
pub fn layout(size: (f32, f32)) -> [(Control, Rect); 7] {
    let (w, h) = size;
    let r = |x: f32, y: f32, rw: f32, rh: f32| Rect {
        x,
        y,
        w: rw * h,
        h: rh * h,
    };
    [
        (
            Control::Accelerate,
            r(w / h - 0.42, 0.62, 0.36, 0.34).anchored(h),
        ),
        (Control::Fire, r(w / h - 0.74, 0.70, 0.26, 0.26).anchored(h)),
        (
            Control::Absorb,
            r(w / h - 0.34, 0.30, 0.26, 0.26).anchored(h),
        ),
        (Control::AirbrakeLeft, r(0.04, 0.04, 0.36, 0.20).anchored(h)),
        (
            Control::AirbrakeRight,
            r(w / h - 0.40, 0.04, 0.36, 0.20).anchored(h),
        ),
        (
            Control::Pause,
            r(w / h / 2.0 - 0.21, 0.03, 0.19, 0.12).anchored(h),
        ),
        (
            Control::Camera,
            r(w / h / 2.0 + 0.02, 0.03, 0.19, 0.12).anchored(h),
        ),
    ]
}

impl Rect {
    /// `x` and `y` were given in units of the window's height; scale them.
    fn anchored(self, h: f32) -> Self {
        Self {
            x: self.x * h,
            y: self.y * h,
            ..self
        }
    }
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
}

impl Touches {
    /// A finger landed at `at` (window pixels) in a window of `size`.
    pub fn down(&mut self, id: u64, at: (f32, f32), size: (f32, f32)) {
        self.fingers.retain(|f| f.id != id);
        let stick_taken = self
            .fingers
            .iter()
            .any(|f| matches!(f.role, Role::Stick { .. }));
        let in_stick_zone =
            at.0 < size.0 * STICK_ZONE_WIDTH && at.1 > size.1 * TOP_STRIP && !stick_taken;
        let role = if in_stick_zone {
            Role::Stick { origin: at }
        } else if control_at(at, size).is_some() {
            Role::Buttons
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
    }

    /// Whether any finger is on the overlay.
    #[must_use]
    pub fn any(&self) -> bool {
        !self.fingers.is_empty()
    }

    fn refresh(&mut self, size: (f32, f32)) {
        self.down = 0;
        for finger in &self.fingers {
            if finger.role == Role::Buttons
                && let Some(control) = control_at(finger.at, size)
            {
                self.down |= 1 << control.index();
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
