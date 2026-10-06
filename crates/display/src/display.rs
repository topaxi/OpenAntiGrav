//! The vocabulary the picture is configured in: which screen, what shape, how
//! big, how bright.
//!
//! Every setting here is a separate question, which is why it is a separate type:
//!
//! - [`Monitor`] is *which* screen, when there is more than one.
//! - [`WindowMode`] is what the compositor is asked for.
//! - [`Size`] is how big a *windowed* window is, and means nothing in
//!   borderless, where the display decides.
//! - [`Aspect`] is the shape the game is drawn at **inside** whatever it got,
//!   with the leftover bars left black.
//! - [`Scale`] is how many pixels that shape is actually rendered with.
//! - [`Fov`] is how much of the world fits in it.
//! - [`BoostFovKick`] is how much [`Fov`] itself moves for a moment on a speed pad -
//!   **this project's own effect, not a reimplementation of the original's** (see its
//!   own doc, which records what the original does instead and why the two are not reconciled).
//! - [`CameraView`] is *which* camera the field of view belongs to: the cockpit or one
//!   of the two chase distances. Unlike everything else here it is also cycled from a
//!   button during a race, so the settings file is where the choice persists, not the only place it is set.
//! - [`Brightness`] and [`Gamma`] are what happens to the finished picture on
//!   its way to the surface, and [`FilterStrength`] is how much of a screen
//!   filter's simulated display it passes through on the way.
//!
//! **These are the types, not the menu pages.** The menus split them across
//! DISPLAY and GRAPHICS and the settings file across `[display]` and
//! `[graphics]`; this module is one place to define what each value may be and
//! how it is spelled, and that split is `oag_game::settings`'s business.
//!
//! The arithmetic is [`viewport`], a pure function with tests, because none of
//! this is checkable from the gate: it is all window and GPU, and a screenshot
//! per mode is the only empirical check there is. Keeping the fit in a function
//! means the part that can be wrong in a way nobody notices is the part that is
//! tested. The same reasoning puts [`Monitor::choose`] and [`Fov::apply`] here
//! rather than at their call sites.

use serde::{Deserialize, Serialize};

mod aspect;
mod hud_scale;
mod motion_blur;
mod msaa;
mod reconstruction;
mod screen_filter;
mod shadows;

pub use {
    aspect::Aspect, hud_scale::*, motion_blur::*, msaa::Msaa, reconstruction::Reconstruction,
    screen_filter::FilterStrength, shadows::Shadows,
};

/// What kind of window the game asks the compositor for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WindowMode {
    /// A window of [`Size`], with decorations.
    #[default]
    Windowed,
    /// A borderless window filling the monitor it is on.
    ///
    /// This is what "fullscreen" means on a modern desktop: no mode switch, no
    /// resolution change, and alt-tab does not black the screen. It also cannot
    /// fail, which exclusive fullscreen can.
    Borderless,
}

impl WindowMode {
    /// The spelling used in a settings file and on a menu row.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Windowed => "windowed",
            Self::Borderless => "borderless",
        }
    }

    /// Every mode this build offers.
    ///
    /// **Exclusive fullscreen is deliberately not here.** It needs a `VideoMode`
    /// enumerated off the monitor, which is a row of its own and a failure path
    /// of its own, and on every compositor this project is developed against it
    /// buys nothing over borderless. Half-wiring it would put a row on the menu
    /// that sometimes does nothing.
    pub const ALL: [Self; 2] = [Self::Windowed, Self::Borderless];
}

impl std::str::FromStr for WindowMode {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|mode| mode.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| format!("{text:?} is not a window mode; try windowed or borderless"))
    }
}

impl std::fmt::Display for WindowMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Which screen the game opens on: `default`, or a monitor by the name the
/// windowing system gives it.
///
/// A **name and not an index**, because an index is a promise the machine does
/// not keep: unplugging a screen, or a compositor that enumerates them in
/// whatever order it woke up in, silently moves the game somewhere else. A name
/// that is no longer there is instead a miss, and a miss falls back to the
/// default - see [`Monitor::choose`].
///
/// There is no value this cannot hold, which is why the conversion is `From`
/// and not `TryFrom`: a monitor name is whatever the platform says it is, and
/// this type has no grounds to refuse one.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub struct Monitor(Option<String>);

impl Monitor {
    /// The spelling for "whichever one the compositor would have picked".
    pub const DEFAULT: &'static str = "default";

    /// Letting the compositor decide, which is what a single-monitor machine
    /// wants and what a name it no longer has falls back to.
    #[must_use]
    pub const fn default_monitor() -> Self {
        Self(None)
    }

    /// Whether this is the default rather than a named screen.
    #[must_use]
    pub fn is_default(&self) -> bool {
        self.0.is_none()
    }

    /// The name this setting holds, or `None` for the default.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.0.as_deref()
    }

    /// The list a MONITOR row is offered, for the screens `available` names.
    ///
    /// [`Monitor::DEFAULT`] first and always, because it is the way back from a
    /// screen that has since been unplugged - and, on a machine with one
    /// screen, the only sensible row. A name already equal to it is not
    /// repeated, so a compositor that really does call a monitor "default"
    /// leaves the list one entry long rather than two identical ones.
    #[must_use]
    pub fn offered(available: &[String]) -> Vec<String> {
        let mut out = vec![Self::DEFAULT.to_string()];
        out.extend(
            available
                .iter()
                .filter(|name| !name.eq_ignore_ascii_case(Self::DEFAULT))
                .cloned(),
        );
        out
    }

    /// Which of `available` this setting picks, or `None` for the default and
    /// for a name this machine does not have.
    ///
    /// The two `None`s are deliberately the same answer: a settings file
    /// written at a desk with two screens, opened on a laptop with one, should
    /// open a window rather than report a problem the player did not cause.
    /// [`Monitor::is_default`] is what tells the caller which of the two
    /// happened, so a *miss* can still be worth a note.
    ///
    /// Matching is case-insensitive on the whole name. Nothing partial: two
    /// screens from one manufacturer share a prefix, and picking the wrong one
    /// of those is exactly the failure a name was chosen to avoid.
    #[must_use]
    pub fn choose(&self, available: &[String]) -> Option<usize> {
        let wanted = self.0.as_deref()?;
        available
            .iter()
            .position(|name| name.eq_ignore_ascii_case(wanted))
    }
}

impl std::str::FromStr for Monitor {
    type Err = std::convert::Infallible;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(text.to_string()))
    }
}

impl From<String> for Monitor {
    fn from(text: String) -> Self {
        let trimmed = text.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case(Self::DEFAULT) {
            Self(None)
        } else {
            Self(Some(trimmed.to_string()))
        }
    }
}

impl From<Monitor> for String {
    fn from(monitor: Monitor) -> Self {
        monitor.to_string()
    }
}

impl std::fmt::Display for Monitor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_deref().unwrap_or(Self::DEFAULT))
    }
}

/// Which adapter the game draws with: `default`, or one by the name
/// `oag_game::adapter::label` gives it.
///
/// [`Monitor`]'s shape, for [`Monitor`]'s reasons, against a different piece of
/// hardware - a name rather than an index, and a name this machine no longer
/// has is a miss that falls back rather than a refusal to start. Plugging in an
/// eGPU reorders the list exactly the way plugging in a screen does.
///
/// The one thing it does not share is that the names it holds are *this
/// project's* spelling and not the platform's: a driver reports a name that is
/// only unique within its backend, so `oag_game::adapter::label` qualifies it.
/// That matters here because `oag_game::menu::Menu::seed` matches the stored
/// string exactly and silently shows the first row when it does not - a
/// duplicate would put the player on an adapter they did not pick and persist
/// it on their first nudge.
///
/// **Held rather than applied**: the device is made once, at boot, so a change
/// lands on the next launch. See `docs/architecture/menus.md`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub struct Renderer(Option<String>);

impl Renderer {
    /// The spelling for "whichever one wgpu would have picked".
    pub const DEFAULT: &'static str = "default";

    /// Letting wgpu decide, which is what a single-GPU machine wants and what a
    /// name it no longer has falls back to.
    #[must_use]
    pub const fn default_renderer() -> Self {
        Self(None)
    }

    /// Whether this is the default rather than a named adapter.
    #[must_use]
    pub fn is_default(&self) -> bool {
        self.0.is_none()
    }

    /// The name this setting holds, or `None` for the default.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.0.as_deref()
    }

    /// The list a RENDERER row is offered, for the adapters `available` names.
    ///
    /// [`Renderer::DEFAULT`] first and always: it is the way back from an
    /// adapter that has since been unplugged or uninstalled, and on a machine
    /// with one GPU it is the only row that means anything.
    #[must_use]
    pub fn offered(available: &[String]) -> Vec<String> {
        let mut out = vec![Self::DEFAULT.to_string()];
        out.extend(
            available
                .iter()
                .filter(|name| !name.eq_ignore_ascii_case(Self::DEFAULT))
                .cloned(),
        );
        out
    }

    /// Which of `available` this setting picks, or `None` for the default and
    /// for a name this machine does not have.
    ///
    /// The two `None`s are the same answer for the same reason they are in
    /// [`Monitor::choose`]: a settings file written on a desktop with a
    /// discrete card, opened on a laptop without it, should start the game.
    /// [`Renderer::is_default`] tells the caller which of the two happened, so a
    /// *miss* is still worth a note.
    ///
    /// Case-insensitive on the whole name, nothing partial - two cards from one
    /// vendor share a prefix, and a driver version moving inside the name (as
    /// llvmpipe's does) would make a prefix match land on the wrong entry.
    #[must_use]
    pub fn choose(&self, available: &[String]) -> Option<usize> {
        let wanted = self.0.as_deref()?;
        available
            .iter()
            .position(|name| name.eq_ignore_ascii_case(wanted))
    }
}

impl std::str::FromStr for Renderer {
    type Err = std::convert::Infallible;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(text.to_string()))
    }
}

impl From<String> for Renderer {
    fn from(text: String) -> Self {
        let trimmed = text.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case(Self::DEFAULT) {
            Self(None)
        } else {
            Self(Some(trimmed.to_string()))
        }
    }
}

impl From<Renderer> for String {
    fn from(renderer: Renderer) -> Self {
        renderer.to_string()
    }
}

impl std::fmt::Display for Renderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_deref().unwrap_or(Self::DEFAULT))
    }
}

/// A window size, spelled `1440x816`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Size {
    /// Width in physical pixels.
    pub width: u32,
    /// Height in physical pixels.
    pub height: u32,
}

impl Size {
    /// The sizes the menus offer, at the PSP's own aspect.
    ///
    /// Multiples of 480x272 and then a few common desktop heights at the same
    /// ratio, so the default shape needs no letterboxing at any of them. A
    /// player on a different aspect gets bars, which is what [`Aspect`] is for.
    pub const OFFERED: [Self; 6] = [
        Self::new(960, 544),
        Self::new(1280, 720),
        Self::new(1440, 816),
        Self::new(1600, 900),
        Self::new(1920, 1080),
        Self::new(2560, 1440),
    ];

    /// A size, with both dimensions forced to at least one.
    ///
    /// Zero is what a minimised window reports and what a surface cannot be
    /// configured at, so it is clamped here rather than at each of the several
    /// places that would otherwise have to.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            width: if width == 0 { 1 } else { width },
            height: if height == 0 { 1 } else { height },
        }
    }
}

impl Default for Size {
    /// 1440x816: three times the PSP's own framebuffer, and what the window
    /// opened at before any of this was configurable.
    fn default() -> Self {
        Self::new(1440, 816)
    }
}

impl std::str::FromStr for Size {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (width, height) = text
            .split_once(['x', 'X'])
            .ok_or_else(|| format!("{text:?} is not a size; try 1440x816"))?;
        let parse = |value: &str, which: &str| {
            value
                .trim()
                .parse::<u32>()
                .map_err(|e| format!("{text:?}: {which} is not a number: {e}"))
        };
        let size = Self::new(parse(width, "width")?, parse(height, "height")?);
        if size.width < 2 || size.height < 2 {
            return Err(format!("{text:?} is too small to draw into"));
        }
        Ok(size)
    }
}

impl std::fmt::Display for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

impl TryFrom<String> for Size {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<Size> for String {
    fn from(size: Size) -> Self {
        size.to_string()
    }
}

/// Writes out the plumbing every percentage setting below shares: a default, a
/// range-checked `TryFrom<u32>`, `FromStr` that tolerates a trailing `%`, and
/// the `Display` those round-trip through.
///
/// A macro rather than one shared `Percent` type because the four must not be
/// interchangeable - a brightness handed to the projection would compile and be
/// wrong - and rather than four hand-written copies because copies are the kind
/// of thing that drifts one edit at a time. Each type still declares its own
/// `RANGE`, `OFFERED` and neutral value, which is all that actually differs.
///
/// Exported for `oag_sound::Volume`, which is the same shape and is not a
/// display setting: a percentage row is a percentage row wherever the value
/// ends up, and a second copy of this body in another module is exactly the
/// drift the macro exists to prevent. That cross-crate reader is why it is
/// `pub` rather than `pub(crate)` since the split.
#[macro_export]
macro_rules! percentage {
    ($type:ident, $neutral:ident, $what:literal) => {
        impl Default for $type {
            fn default() -> Self {
                Self::$neutral
            }
        }

        impl TryFrom<u32> for $type {
            type Error = String;

            fn try_from(percent: u32) -> Result<Self, Self::Error> {
                if Self::RANGE.contains(&percent) {
                    Ok(Self(percent))
                } else {
                    Err(format!(
                        concat!("a ", $what, " of {} is outside {}-{}"),
                        percent,
                        Self::RANGE.start(),
                        Self::RANGE.end()
                    ))
                }
            }
        }

        impl std::str::FromStr for $type {
            type Err = String;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                let percent: u32 = text
                    .trim()
                    .trim_end_matches('%')
                    .parse()
                    .map_err(|e| format!(concat!("{:?} is not a ", $what, ": {}"), text, e))?;
                Self::try_from(percent)
            }
        }

        impl From<$type> for u32 {
            fn from(value: $type) -> Self {
                value.0
            }
        }

        impl std::fmt::Display for $type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

/// How hard FSR 1's RCAS pass sharpens, in **tenths of a stop**.
///
/// Stops are upstream FidelityFX's own unit and they run backwards: zero is
/// maximum sharpening and each whole step halves it. That is worth carrying
/// rather than quietly rescaling, because every FSR document and every other
/// game's setting is in these units, and a value a player reads about elsewhere
/// should mean the same thing here.
///
/// Tenths, and an integer, so a menu row round-trips exactly. A bare `f32`
/// setting would have to format and re-parse to the same text every time or the
/// row would fail to find its own current value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "SharpnessRepr", into = "String")]
pub struct Sharpness(u32);

/// What a settings file is allowed to say, which includes what an older one
/// already says.
///
/// This shipped as a bare `f32` for one commit, so a file written by that build
/// holds `upscale_sharpness = 0.2` and would otherwise fail to load with
/// "invalid type: floating point". The same twenty lines, and the same
/// reasoning, as `oag_game::perf::Vsync`'s: the canonical rewrite normalises it
/// to a string on the next run, so the compatibility does not accumulate.
#[derive(Deserialize)]
#[serde(untagged)]
enum SharpnessRepr {
    Stops(f32),
    Name(String),
}

impl TryFrom<SharpnessRepr> for Sharpness {
    type Error = String;

    fn try_from(repr: SharpnessRepr) -> Result<Self, Self::Error> {
        match repr {
            SharpnessRepr::Stops(stops) => stops.to_string().parse(),
            SharpnessRepr::Name(name) => name.parse(),
        }
    }
}

impl Sharpness {
    /// Upstream's own default, 0.2 stops.
    pub const DEFAULT: Self = Self(2);

    /// The widest range upstream documents, zero to two stops.
    pub const RANGE: std::ops::RangeInclusive<u32> = 0..=20;

    /// The values the menus offer: maximum, upstream's default, then halving.
    pub const OFFERED: [Self; 5] = [Self(0), Self(2), Self(5), Self(10), Self(20)];

    /// The setting in stops, which is what the upscaler wants.
    #[must_use]
    pub fn stops(self) -> f32 {
        self.0 as f32 / 10.0
    }
}

impl Default for Sharpness {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl std::str::FromStr for Sharpness {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let stops: f32 = text
            .trim()
            .parse()
            .map_err(|e| format!("{text:?} is not a sharpness: {e}"))?;
        // Rounded rather than truncated so `0.25` lands on a value rather than
        // silently becoming `0.2`, and so the round trip is stable.
        let tenths = (stops * 10.0).round();
        if !tenths.is_finite() || tenths < 0.0 {
            return Err(format!("a sharpness of {stops} is not a number of stops"));
        }
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "checked finite and non-negative just above"
        )]
        let tenths = tenths as u32;
        if Self::RANGE.contains(&tenths) {
            Ok(Self(tenths))
        } else {
            Err(format!(
                "a sharpness of {stops} stops is outside {}-{}",
                *Self::RANGE.start() as f32 / 10.0,
                *Self::RANGE.end() as f32 / 10.0
            ))
        }
    }
}

impl std::fmt::Display for Sharpness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.0 / 10, self.0 % 10)
    }
}

impl From<Sharpness> for String {
    fn from(value: Sharpness) -> Self {
        value.to_string()
    }
}

/// How much of the viewport rectangle the game is actually rendered at, as a
/// percentage.
///
/// Below 100 this is the usual internal-resolution knob; above it, it is
/// supersampling. A percentage rather than an absolute resolution because a
/// percentage has no invalid values - see `oag_game::upscale` for the argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct Scale(u32);

impl Scale {
    /// Rendering at exactly the size it is displayed at.
    pub const FULL: Self = Self(100);

    /// The narrowest and widest a scale may be.
    ///
    /// Not preferences: below the floor a 480-wide viewport renders at 120
    /// pixels and the menus stop being readable at all, and above the ceiling
    /// the target runs past what an adapter will allocate on an ordinary
    /// window. `oag_game::upscale::target_size` clamps the actual pixels too,
    /// because the ceiling here is not a per-device answer.
    pub const RANGE: std::ops::RangeInclusive<u32> = 25..=200;

    /// The percentages the menus offer.
    pub const OFFERED: [Self; 6] = [
        Self(50),
        Self(75),
        Self(100),
        Self(125),
        Self(150),
        Self(200),
    ];

    /// The multiplier this percentage means.
    #[must_use]
    pub fn factor(self) -> f32 {
        self.0 as f32 / 100.0
    }

    /// The percentage itself.
    #[must_use]
    pub fn percent(self) -> u32 {
        self.0
    }
}

/// How much the finished picture is multiplied by on its way to the surface, as
/// a percentage. 100 is untouched.
///
/// A multiply and not a lift: adding a constant raises black off black and
/// leaves a race at night looking like a race in fog, and the thing a player
/// reaches for this setting to fix - a track they cannot see into - is a
/// midtone problem [`Gamma`] handles better anyway. Both are applied in the
/// blit pass, so both cover every stage; see `oag_game::upscale`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct Brightness(u32);

impl Brightness {
    /// Leaving the picture alone.
    pub const NEUTRAL: Self = Self(100);

    /// The narrowest and widest this may be.
    ///
    /// Bounded at both ends because both ends are a black screen a player
    /// cannot get back from with a menu they can no longer read.
    pub const RANGE: std::ops::RangeInclusive<u32> = 50..=200;

    /// The percentages the menus offer.
    pub const OFFERED: [Self; 7] = [
        Self(50),
        Self(75),
        Self(90),
        Self(100),
        Self(110),
        Self(125),
        Self(150),
    ];

    /// The multiplier this percentage means.
    #[must_use]
    pub fn factor(self) -> f32 {
        self.0 as f32 / 100.0
    }

    /// The percentage itself.
    #[must_use]
    pub fn percent(self) -> u32 {
        self.0
    }
}

/// The exponent curve applied to the finished picture, as a percentage of 1.0.
/// 100 is untouched; above it lifts the midtones without touching black or
/// white, below it deepens them.
///
/// **Applied to the values the blit samples, which are linear when the surface
/// is an sRGB one** - so this is not the display-gamma knob a CRT-era game
/// shipped, and it is not claimed to be. What it does is the thing that knob
/// was used for: making a dark corner of a track visible without washing the
/// rest of it out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct Gamma(u32);

impl Gamma {
    /// Leaving the picture alone.
    pub const NEUTRAL: Self = Self(100);

    /// The narrowest and widest this may be, bounded for the same reason
    /// [`Brightness::RANGE`] is.
    pub const RANGE: std::ops::RangeInclusive<u32> = 50..=200;

    /// The percentages the menus offer.
    pub const OFFERED: [Self; 7] = [
        Self(60),
        Self(80),
        Self(90),
        Self(100),
        Self(110),
        Self(120),
        Self(140),
    ];

    /// The exponent the shader raises the picture to, which is the reciprocal
    /// of the gamma this names: a gamma above 1 is an exponent below it, and
    /// that is what brightens.
    ///
    /// Computed here rather than in the shader so the reciprocal is taken once
    /// per frame instead of once per pixel, and so the one place it could be
    /// the wrong way round has a test.
    #[must_use]
    pub fn exponent(self) -> f32 {
        100.0 / self.0 as f32
    }

    /// The percentage itself.
    #[must_use]
    pub fn percent(self) -> u32 {
        self.0
    }
}

/// How wide the camera's field of view is, as a percentage of the one the
/// original's own data authored. 100 is the authored value.
///
/// **A percentage of the authored field and not an absolute angle.** The
/// original reason was that `<ExternalCameraFar fov>`'s unit was unrecovered, so
/// a row letting a player type `90` would assert a unit this project had not
/// established. **That reason expired on 2026-08-09**: the unit is vertical
/// degrees at confidence 94 (see `oag_raceplay::Race::projection`).
///
/// The row stays a percentage on a different and weaker argument, recorded so
/// the next reader can overrule it rather than assume it was never revisited:
/// the field the player actually sees is not the authored one. The original
/// widens it with speed, so an absolute row would name a number the game only
/// shows while stationary. A multiplier keeps its meaning at every speed.
/// Switching to degrees is now a presentation decision, not a blocked one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct Fov(u32);

impl Fov {
    /// The field of view the disc's own data asks for.
    pub const AUTHORED: Self = Self(100);

    /// The narrowest and widest this may be.
    ///
    /// The ceiling is not a preference: the tangent scaling below runs to a
    /// half-angle approaching a straight line, and past roughly twice the
    /// authored field the projection is fisheye and the track is unreadable.
    pub const RANGE: std::ops::RangeInclusive<u32> = 50..=200;

    /// The percentages the menus offer.
    pub const OFFERED: [Self; 7] = [
        Self(75),
        Self(90),
        Self(100),
        Self(110),
        Self(125),
        Self(150),
        Self(175),
    ];

    /// `fov_radians` widened or narrowed by this setting.
    ///
    /// **Scales the tangent of the half-angle, not the angle.** The tangent is
    /// what the projection matrix is actually built from, so scaling it is what
    /// makes 150 % mean "half again as much across the screen"; scaling the
    /// angle would make the same percentage mean less and less the wider the
    /// field already was, and would run past half a turn where this cannot.
    ///
    /// At [`Fov::AUTHORED`] the input comes back bit-identical rather than
    /// through `atan(tan(x))`, so leaving the setting alone is not a rounding
    /// difference against every capture taken before it existed.
    #[must_use]
    pub fn apply(self, fov_radians: f32) -> f32 {
        if self == Self::AUTHORED {
            return fov_radians;
        }
        2.0 * ((fov_radians * 0.5).tan() * self.factor()).atan()
    }

    /// The multiplier this percentage means.
    #[must_use]
    pub fn factor(self) -> f32 {
        self.0 as f32 / 100.0
    }

    /// The percentage itself.
    #[must_use]
    pub fn percent(self) -> u32 {
        self.0
    }
}

percentage!(Scale, FULL, "render scale");
percentage!(Brightness, NEUTRAL, "brightness");
percentage!(Gamma, NEUTRAL, "gamma");
percentage!(Fov, AUTHORED, "field of view");

/// How much wider [`Fov`] opens for a moment when a boost pad fires, as a
/// percentage of the tangent of the half-angle - the unit `Fov::apply` itself
/// scales in, so this composes with a player's own field-of-view choice
/// rather than fighting it. 0 turns the effect off.
///
/// **This effect is ours by choice, and it is not being fitted to the
/// original.** It shipped as an authored effect on the reasoning that a boost
/// the player cannot feel reads as nothing happening, and that reasoning is
/// still the whole justification. Every number here is this project's own.
///
/// Two later findings about the original are recorded because they are
/// interesting, and **neither is a specification for this type**:
///
/// - a 2026-08-07 matched-pose comparison against a captured PPSSPP pad
///   crossing on Talon's Junction shows the original's whole scene zoom out
///   and settle back - **but the "90-95 degrees against the authored 60"
///   reading that used to sit here is refuted**, and the effect is not a boost
///   effect at all. The fov's actual ceiling on that circuit is ~71 degrees at
///   146 units/s, reached by *speed*; see
///   `docs/rendering/projection-vs-the-original.md`, "What this retires";
/// - the original's fov chain was recovered on 2026-08-08 and settled on the
///   running game on 2026-08-09
///   (`docs/ghidra/functions/psp-pulse-usa/camera.md`): its fov is
///   `authored + 0.075 * dot(fwd, vel)`, additive **degrees** driven by forward
///   speed, which is a different shape *and* a different driver from this
///   type's boost-gated tangent multiplier. That term **is** ported, as
///   `race::SPEED_FOV_GAIN_DEG`; this type is the separate, invented one.
///
/// **Do not retune the tiers below against either.** They differ from the
/// original by design, not by defect; an earlier version of this doc read as
/// a calibration instruction ("the measured original sits nearest the 32
/// tier") and that reading is withdrawn.
///
/// See `oag_raceplay::Race::projection` for where it is applied and
/// `oag_raceplay::BOOST_FOV_OPEN_RATE`/`BOOST_FOV_CLOSE_RATE` for how it moves.
///
/// **A number, the same idiom as [`Fov`] itself**, rather than a named tier:
/// there is nothing to name that "twice as wide" does not already say. 8 was
/// this project's original on/off toggle's "on", tuned to be felt rather than
/// seen; measured against a player who could not tell it was there at all
/// (see the conversation this type came out of), [`Self::DEFAULT`] ships at
/// 16, one doubling up, with 32 kept as a stronger tier still and 8 kept as
/// the subtler one for a player who wants the old feel back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct BoostFovKick(u32);

impl BoostFovKick {
    /// No kick at all. `oag_raceplay::Race::projection` returns the input
    /// bit-identical here, the same escape hatch [`Fov::AUTHORED`] is.
    pub const OFF: Self = Self(0);

    /// The original toggle's "on", kept as the subtlest non-zero tier.
    pub const SUBTLE: Self = Self(8);

    /// What a fresh install ships with - one doubling past [`Self::SUBTLE`],
    /// which read as nothing happening to a player watching for it.
    pub const DEFAULT: Self = Self(16);

    /// The widest this may be. Past this the kick stops reading as a boost
    /// and starts reading as a camera glitch - the same order-of-magnitude
    /// ceiling [`Fov::RANGE`] enforces for its own effect.
    pub const RANGE: std::ops::RangeInclusive<u32> = 0..=32;

    /// The tiers the menus offer: off, then three doublings.
    pub const OFFERED: [Self; 4] = [Self::OFF, Self::SUBTLE, Self::DEFAULT, Self(32)];

    /// The multiplier `oag_raceplay::Race::projection` widens the tangent by
    /// at full boost.
    #[must_use]
    pub fn gain(self) -> f32 {
        self.0 as f32 / 100.0
    }
}

percentage!(BoostFovKick, DEFAULT, "boost field-of-view kick");

/// Which of the original's three in-race camera perspectives is live.
///
/// **Recovered, including the cycle order.** `Camera_UpdatePlayerView`
/// (`0x0883c0cc`) tests SELECT, consumes the press and rotates one profile
/// setting through exactly three values, selecting a differently named camera rig
/// for each; the order wraps and there is no fourth entry. Confidence **88** for
/// the set and the order - see
/// `docs/ghidra/functions/psp-pulse-usa/camera.md`. [`Self::next`] is that
/// rotation and [`Self::ALL`] is in that order, so the button and the menu row
/// cannot disagree about it.
///
/// # What is not recovered
///
/// - Which one a fresh profile starts on is **recovered**: [`Self::Close`],
///   measured 2026-10-01 on two cold PPSSPP boots (eye 11.644 from the craft, the
///   close block at the 0.75 scale; the setting read `OPT_CLOSE`), and what
///   Wipeout 2048's own `CameraP1 default="OPT_CLOSE"` declares. `Far` was this
///   type's default until then and drew the craft 1.4 times too small. See
///   `camera.md`, "The default view".
/// - **The spelling.** The original persists `OPT_INT`, `OPT_CLOSE` and
///   `OPT_FAR`; [`Self::name`] uses this project's own settings-file vocabulary.
///   Anything that ever reads a real profile save has to know the disc's own
///   three spellings, and `camera.md` records them.
///
/// # It is presentation, never simulation
///
/// This does not enter `oag_gameplay::World` and nothing hashed reads it, so
/// cycling the view cannot move a determinism hash or a replay. That is asserted
/// rather than argued: see
/// `oag_raceplay::tests::cycling_the_camera_changes_no_simulation_state`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CameraView {
    /// The cockpit view, `<InternalCamera>`. The only one that hides the
    /// player's own ship.
    Internal,
    /// The nearer chase view, `<ExternalCameraClose>`. **The default**: what a
    /// fresh profile of the original starts on, see the type's documentation.
    #[default]
    Close,
    /// The further chase view, `<ExternalCameraFar>`.
    Far,
}

impl CameraView {
    /// All three, in the order SELECT cycles them.
    ///
    /// `internal -> close -> far`, wrapping, which is the original's own order
    /// and not alphabetical or nearest-first by accident.
    pub const ALL: [Self; 3] = [Self::Internal, Self::Close, Self::Far];

    /// The spelling used in a settings file and on a menu row.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Internal => "internal",
            Self::Close => "close",
            Self::Far => "far",
        }
    }

    /// The next view SELECT would move to, wrapping.
    ///
    /// The whole cycle in one place, so the button, the menu row and any test
    /// share it.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Internal => Self::Close,
            Self::Close => Self::Far,
            Self::Far => Self::Internal,
        }
    }

    /// Whether the player's own hull is drawn.
    ///
    /// False for the cockpit view alone. The original sets `craft+0x6d` to 1
    /// for `OPT_INT` and 0 for both external views. **A consumer of it is now
    /// read and agrees**: `ShipShield_Update` (`0x0885e254`) branches on it to
    /// draw the shield's hull-shaped shell when clear and its cockpit sphere
    /// when set, hiding the other - only sensible as "the camera is inside
    /// this craft". Confidence **70** -> **82**; see `shield-pickup.md`.
    #[must_use]
    pub fn draws_own_ship(self) -> bool {
        self != Self::Internal
    }
}

impl std::str::FromStr for CameraView {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|view| view.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| format!("{text:?} is not a camera view; try internal, close or far"))
    }
}

impl std::fmt::Display for CameraView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Where a window of `size` goes to sit centred on a monitor whose own
/// rectangle starts at `origin` and is `area` across, all in physical pixels.
///
/// [`Monitor`] picks *which* screen; this is the only reason that picking does
/// anything in windowed mode, where there is no fullscreen request to carry the
/// choice. A window is placed by its top-left corner in one coordinate space
/// spanning every monitor, so "on that screen" is arithmetic rather than a flag.
///
/// A window larger than the monitor is pinned to the origin rather than centred
/// off the top-left edge, where a title bar would be unreachable.
#[must_use]
pub fn centred(origin: (i32, i32), area: (u32, u32), size: (u32, u32)) -> (i32, i32) {
    let offset = |span: u32, window: u32| ((span as i64 - window as i64) / 2).max(0) as i32;
    (
        origin.0.saturating_add(offset(area.0, size.0)),
        origin.1.saturating_add(offset(area.1, size.1)),
    )
}

/// The centred rectangle of `aspect` inside a `target`-sized viewport.
///
/// Returns `(x, y, width, height)` in physical pixels, which is exactly what
/// `wgpu::RenderPass::set_viewport` takes. The leftover is never drawn into and
/// stays whatever the pass cleared to, which is black.
///
/// Both dimensions come back at least one pixel. A zero-area viewport is a
/// validation error in wgpu and a minimised window is the ordinary way to get
/// one, so the degenerate case is handled here rather than at the call site.
#[must_use]
pub fn viewport(target: (u32, u32), aspect: Aspect) -> (f32, f32, f32, f32) {
    let width = target.0.max(1) as f32;
    let height = target.1.max(1) as f32;
    let Some(wanted) = aspect.ratio() else {
        return (0.0, 0.0, width, height);
    };

    let have = width / height;
    if have > wanted {
        // Too wide: bars down the sides.
        let fitted = (height * wanted).max(1.0);
        ((width - fitted) / 2.0, 0.0, fitted, height)
    } else {
        // Too tall, or exact: bars top and bottom, and nothing when exact.
        let fitted = (width / wanted).max(1.0);
        (0.0, (height - fitted) / 2.0, width, fitted)
    }
}

#[cfg(test)]
mod tests;
