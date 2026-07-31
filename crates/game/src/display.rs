//! The vocabulary the picture is configured in: which screen, what shape, how
//! big, how bright.
//!
//! Every setting here is a separate question, which is why it is a separate
//! type:
//!
//! - [`Monitor`] is *which* screen, when there is more than one.
//! - [`WindowMode`] is what the compositor is asked for.
//! - [`Size`] is how big a *windowed* window is, and means nothing in
//!   borderless, where the display decides.
//! - [`Aspect`] is the shape the game is drawn at **inside** whatever it got,
//!   with the leftover bars left black.
//! - [`Scale`] is how many pixels that shape is actually rendered with.
//! - [`Fov`] is how much of the world fits in it.
//! - [`Brightness`] and [`Gamma`] are what happens to the finished picture on
//!   its way to the surface.
//!
//! **These are the types, not the menu pages.** The menus split them across
//! DISPLAY and GRAPHICS and the settings file across `[display]` and
//! `[graphics]`; this module is one place to define what each value may be and
//! how it is spelled, and that split is [`crate::settings`]'s business.
//!
//! The arithmetic is [`viewport`], a pure function with tests, because none of
//! this is checkable from the gate: it is all window and GPU, and a screenshot
//! per mode is the only empirical check there is. Keeping the fit in a function
//! means the part that can be wrong in a way nobody notices is the part that is
//! tested. The same reasoning puts [`Monitor::choose`] and [`Fov::apply`] here
//! rather than at their call sites.

use serde::{Deserialize, Serialize};

/// The shape the game is drawn at inside its window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Aspect {
    /// 480x272, the PSP's own framebuffer, and the shape every camera value on
    /// the disc was authored for.
    ///
    /// The default, and not merely out of deference: `<ExternalCameraFar>`'s
    /// field of view is only defined at this ratio, so it is the one shape where
    /// what a player sees is what the original framed. See
    /// [`crate::race::AUTHORED_ASPECT`].
    #[default]
    Psp,
    /// 4:3, which is what the PS2 release output.
    Ps2,
    /// Whatever the window is.
    ///
    /// **The field of view does not widen with it.** `Race::projection` caps at
    /// the authored aspect and fits the view inside anything wider, so a wide
    /// window shows the same amount of track rather than more of it. That is a
    /// deliberate reading of "free" as *fill the window* and not as *see more* -
    /// nothing on the disc says what the original would have done with a 21:9
    /// screen, and inventing a wider field of view would be inventing gameplay.
    Free,
}

impl Aspect {
    /// The ratio this shape asks for, or `None` for [`Aspect::Free`], which asks
    /// for whatever it is given.
    #[must_use]
    pub fn ratio(self) -> Option<f32> {
        match self {
            Self::Psp => Some(crate::frontend::SCREEN.0 / crate::frontend::SCREEN.1),
            Self::Ps2 => Some(4.0 / 3.0),
            Self::Free => None,
        }
    }

    /// The spelling used in a settings file and on a menu row.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Psp => "psp",
            Self::Ps2 => "ps2",
            Self::Free => "free",
        }
    }

    /// Every shape, for the menus and for error messages.
    pub const ALL: [Self; 3] = [Self::Psp, Self::Ps2, Self::Free];
}

impl std::str::FromStr for Aspect {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|aspect| aspect.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| format!("{text:?} is not an aspect ratio; try psp, ps2 or free"))
    }
}

impl std::fmt::Display for Aspect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

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
/// reasoning, as [`crate::perf::Vsync`]'s: the canonical rewrite normalises it
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

/// Which resampler carries the offscreen frame onto the surface.
///
/// This is the companion to [`Scale`], and only that pairing makes it mean
/// anything: at 100 % there is nothing to upscale and the choice is between a
/// blit and a sharpen. Below 100 % it is the whole point of the render-scale
/// row - how much of what the lower resolution threw away can be argued back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Upscaler {
    /// One bilinear tap, which is what this always did.
    ///
    /// **The default.** The comparison that would move it has been run once,
    /// at 50 % on one frame of one track, and FSR 1 won it clearly - but one
    /// frame of one track is not the sample `animated_textures` was held to,
    /// and the doubt that motivated the caution is specifically about content
    /// this frame did not contain: the menus and the HUD are 480x272-era
    /// paletted raster and glyphs off a coverage atlas, and a sharpener rings
    /// on those in a way it does not on track geometry. See HANDOVER.
    #[default]
    Bilinear,
    /// AMD FidelityFX Super Resolution 1: EASU, then RCAS.
    ///
    /// Spatial, so it costs two fullscreen passes and needs nothing from the
    /// renderer - no motion vectors, no jitter, no history. See
    /// [`oag_render::post::fsr1`].
    Fsr1,
}

impl Upscaler {
    /// The spelling used in a settings file and on a menu row.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Bilinear => "bilinear",
            Self::Fsr1 => "fsr1",
        }
    }

    /// Every choice, for the menus and for error messages.
    pub const ALL: [Self; 2] = [Self::Bilinear, Self::Fsr1];
}

impl std::str::FromStr for Upscaler {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|mode| mode.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| format!("{text:?} is not an upscaler; try bilinear or fsr1"))
    }
}

impl std::fmt::Display for Upscaler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl TryFrom<String> for Upscaler {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<Upscaler> for String {
    fn from(mode: Upscaler) -> Self {
        mode.to_string()
    }
}

/// How much of the viewport rectangle the game is actually rendered at, as a
/// percentage.
///
/// Below 100 this is the usual internal-resolution knob; above it, it is
/// supersampling. A percentage rather than an absolute resolution because a
/// percentage has no invalid values - see [`crate::upscale`] for the argument.
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
    /// window. [`crate::upscale::target_size`] clamps the actual pixels too,
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
/// blit pass, so both cover every stage; see [`crate::upscale`].
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
/// **A percentage of the authored field and not an absolute angle**, which is
/// the whole point: `<ExternalCameraFar fov>` comes off the disc, its unit is
/// unrecovered (see [`crate::race::Race::projection`]), and a row that let a
/// player type `90` would be quietly asserting a unit this project has not
/// established. A multiplier says what it does - wider or narrower than the
/// game frames it - without claiming to know what the number underneath means.
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
mod tests {
    use super::*;

    /// The aspect of a rectangle `viewport` returned.
    fn ratio(rect: (f32, f32, f32, f32)) -> f32 {
        rect.2 / rect.3
    }

    #[test]
    fn free_fills_whatever_it_is_given() {
        assert_eq!(
            viewport((1920, 1080), Aspect::Free),
            (0.0, 0.0, 1920.0, 1080.0)
        );
        assert_eq!(viewport((100, 900), Aspect::Free), (0.0, 0.0, 100.0, 900.0));
    }

    /// The window this build has always opened at is exactly the PSP's shape,
    /// so the default setting on the default window draws no bars at all.
    #[test]
    fn the_default_window_needs_no_bars_at_the_default_aspect() {
        let size = Size::default();
        let rect = viewport((size.width, size.height), Aspect::Psp);
        assert_eq!(rect.0, 0.0, "{rect:?}");
        assert_eq!(rect.1, 0.0, "{rect:?}");
        assert!(
            (rect.2 - size.width as f32).abs() < 1.0 && (rect.3 - size.height as f32).abs() < 1.0,
            "{rect:?}"
        );
    }

    #[test]
    fn a_wide_window_gets_bars_down_the_sides() {
        let rect = viewport((3440, 1440), Aspect::Psp);
        assert!(rect.0 > 0.0, "no left bar: {rect:?}");
        assert_eq!(rect.1, 0.0, "and no top bar: {rect:?}");
        assert!((rect.3 - 1440.0).abs() < 1e-3, "full height: {rect:?}");
        assert!(
            (ratio(rect) - Aspect::Psp.ratio().expect("psp has a ratio")).abs() < 1e-3,
            "{rect:?}"
        );
        // Centred, so both bars are the same width.
        assert!((rect.0 * 2.0 + rect.2 - 3440.0).abs() < 1e-3, "{rect:?}");
    }

    #[test]
    fn a_tall_window_gets_bars_above_and_below() {
        let rect = viewport((1080, 1920), Aspect::Ps2);
        assert_eq!(rect.0, 0.0, "{rect:?}");
        assert!(rect.1 > 0.0, "{rect:?}");
        assert!((ratio(rect) - 4.0 / 3.0).abs() < 1e-3, "{rect:?}");
        assert!((rect.1 * 2.0 + rect.3 - 1920.0).abs() < 1e-3, "{rect:?}");
    }

    /// The two fixed shapes really are different, or the setting does nothing.
    #[test]
    fn psp_and_ps2_are_not_the_same_shape() {
        let psp = viewport((1920, 1080), Aspect::Psp);
        let ps2 = viewport((1920, 1080), Aspect::Ps2);
        assert!((ratio(psp) - ratio(ps2)).abs() > 0.1, "{psp:?} {ps2:?}");
        assert!(ps2.0 > psp.0, "4:3 is the narrower of the two");
    }

    /// A minimised window reports zero, and a zero-area viewport is a wgpu
    /// validation error rather than a blank frame.
    #[test]
    fn a_degenerate_window_still_gives_a_drawable_rectangle() {
        for target in [(0, 0), (1, 4000), (4000, 1)] {
            for aspect in Aspect::ALL {
                let rect = viewport(target, aspect);
                assert!(
                    rect.2 >= 1.0 && rect.3 >= 1.0,
                    "{target:?} {aspect}: {rect:?}"
                );
                assert!(rect.0.is_finite() && rect.1.is_finite());
            }
        }
    }

    /// The rectangle has to stay inside the surface, or `set_viewport` fails
    /// validation - which is the one way this can be wrong without looking
    /// wrong in a screenshot.
    #[test]
    fn the_rectangle_never_leaves_the_surface() {
        for target in [(1920, 1080), (800, 600), (3440, 1440), (7, 5000)] {
            for aspect in Aspect::ALL {
                let (x, y, w, h) = viewport(target, aspect);
                assert!(x >= 0.0 && y >= 0.0, "{target:?} {aspect}");
                assert!(
                    x + w <= target.0.max(1) as f32 + 1e-3,
                    "{target:?} {aspect}: {x} + {w}"
                );
                assert!(
                    y + h <= target.1.max(1) as f32 + 1e-3,
                    "{target:?} {aspect}: {y} + {h}"
                );
            }
        }
    }

    #[test]
    fn a_size_round_trips_through_its_own_spelling() {
        let size: Size = "1920x1080".parse().expect("parse");
        assert_eq!(size, Size::new(1920, 1080));
        assert_eq!(size.to_string(), "1920x1080");
        assert_eq!("2560X1440".parse::<Size>().expect("parse").width, 2560);
    }

    #[test]
    fn a_malformed_size_says_what_it_wanted() {
        for text in ["1920", "axb", "1920x", "0x0", ""] {
            let error = text.parse::<Size>().expect_err(text);
            assert!(!error.is_empty(), "{text}");
        }
    }

    #[test]
    fn every_offered_size_is_the_psp_shape() {
        let wanted = Aspect::Psp.ratio().expect("psp has a ratio");
        for size in Size::OFFERED {
            let have = size.width as f32 / size.height as f32;
            assert!(
                (have - wanted).abs() < 0.02,
                "{size} is {have}, wanted about {wanted}"
            );
        }
    }

    #[test]
    fn a_scale_round_trips_and_refuses_what_it_cannot_draw() {
        assert_eq!("100".parse::<Scale>(), Ok(Scale::FULL));
        assert_eq!("50%".parse::<Scale>().expect("parse").factor(), 0.5);
        assert_eq!(Scale::FULL.to_string(), "100");
        for bad in ["0", "24", "201", "1000", "half", ""] {
            assert!(bad.parse::<Scale>().is_err(), "{bad}");
        }
    }

    #[test]
    fn every_offered_scale_is_one_this_build_accepts() {
        for scale in Scale::OFFERED {
            assert_eq!(scale.to_string().parse::<Scale>(), Ok(scale));
        }
        assert!(
            Scale::OFFERED.contains(&Scale::FULL),
            "100% must be offered"
        );
    }

    /// The three percentages the macro writes out behave the way the render
    /// scale already did, which is the whole reason they share it.
    #[test]
    fn every_percentage_round_trips_and_refuses_what_is_outside_its_range() {
        assert_eq!("100".parse::<Brightness>(), Ok(Brightness::NEUTRAL));
        assert_eq!("100".parse::<Gamma>(), Ok(Gamma::NEUTRAL));
        assert_eq!("100%".parse::<Fov>(), Ok(Fov::AUTHORED));
        assert_eq!(Brightness::default(), Brightness::NEUTRAL);
        assert_eq!(Gamma::default(), Gamma::NEUTRAL);
        assert_eq!(Fov::default(), Fov::AUTHORED);

        for bad in ["0", "49", "201", "bright", ""] {
            assert!(bad.parse::<Brightness>().is_err(), "{bad}");
            assert!(bad.parse::<Gamma>().is_err(), "{bad}");
            assert!(bad.parse::<Fov>().is_err(), "{bad}");
        }

        for value in Brightness::OFFERED {
            assert_eq!(value.to_string().parse::<Brightness>(), Ok(value));
        }
        for value in Gamma::OFFERED {
            assert_eq!(value.to_string().parse::<Gamma>(), Ok(value));
        }
        for value in Fov::OFFERED {
            assert_eq!(value.to_string().parse::<Fov>(), Ok(value));
        }
    }

    /// Each list has to offer the value that changes nothing, or a player who
    /// moves one of these has no way back to how the game shipped.
    #[test]
    fn every_percentage_offers_its_own_neutral() {
        assert!(Brightness::OFFERED.contains(&Brightness::NEUTRAL));
        assert!(Gamma::OFFERED.contains(&Gamma::NEUTRAL));
        assert!(Fov::OFFERED.contains(&Fov::AUTHORED));
    }

    /// The one place gamma could be the wrong way round: the setting names the
    /// gamma, the shader wants its reciprocal, and a gamma above 1 has to
    /// brighten.
    #[test]
    fn a_higher_gamma_is_a_lower_exponent_and_brightens() {
        assert_eq!(Gamma::NEUTRAL.exponent(), 1.0);
        let up: Gamma = "140".parse().expect("parse");
        let down: Gamma = "60".parse().expect("parse");
        assert!(up.exponent() < 1.0, "{}", up.exponent());
        assert!(down.exponent() > 1.0, "{}", down.exponent());
        // A midtone, raised to each exponent. Above 1.0 gamma it has to come
        // out lighter, below it darker, and black and white must not move.
        let grey = 0.25f32;
        assert!(grey.powf(up.exponent()) > grey);
        assert!(grey.powf(down.exponent()) < grey);
        for exponent in [up.exponent(), down.exponent()] {
            assert_eq!(0.0f32.powf(exponent), 0.0);
            assert_eq!(1.0f32.powf(exponent), 1.0);
        }
    }

    /// The authored field has to come back untouched, bit for bit: every
    /// capture in `data/traces/` was taken before this setting existed.
    #[test]
    fn the_authored_field_of_view_is_returned_unchanged() {
        for degrees in [30.0f32, 45.0, 60.0, 91.5] {
            let fov = degrees.to_radians();
            assert_eq!(Fov::AUTHORED.apply(fov), fov, "{degrees}");
        }
    }

    /// The property the tangent scaling exists for: a percentage means that
    /// much more across the screen, and the result stays a projection.
    #[test]
    fn a_wider_field_of_view_scales_the_tangent_and_stays_below_half_a_turn() {
        let fov = 60.0f32.to_radians();
        let authored = (fov * 0.5).tan();
        for percent in ["50", "75", "125", "150", "200"] {
            let setting: Fov = percent.parse().expect("parse");
            let applied = setting.apply(fov);
            let scaled = (applied * 0.5).tan();
            assert!(
                (scaled - authored * setting.factor()).abs() < 1e-5,
                "{percent}: {scaled} vs {}",
                authored * setting.factor()
            );
            assert!(applied > 0.0 && applied < std::f32::consts::PI, "{percent}");
        }
        // Monotone, or the row would not read as a slider.
        let narrow: Fov = "75".parse().expect("parse");
        let wide: Fov = "150".parse().expect("parse");
        assert!(narrow.apply(fov) < fov && fov < wide.apply(fov));
    }

    /// The second monitor in a left-to-right pair, which is the case the whole
    /// setting exists for: the window has to land inside *its* rectangle, not
    /// at the same offset on the first one.
    #[test]
    fn a_window_is_centred_inside_the_monitor_it_is_given() {
        assert_eq!(centred((0, 0), (1920, 1080), (1440, 816)), (240, 132));
        assert_eq!(centred((1920, 0), (2560, 1440), (1440, 816)), (2480, 312));
        // A monitor above and left of the origin, which is where a compositor
        // puts a second screen arranged that way.
        assert_eq!(
            centred((-1920, -180), (1920, 1080), (1920, 1080)),
            (-1920, -180)
        );
    }

    /// A window bigger than the screen is pinned rather than centred off the
    /// top-left edge, where the title bar cannot be reached.
    #[test]
    fn an_oversized_window_stays_at_the_monitors_own_corner() {
        assert_eq!(centred((1920, 0), (1280, 720), (2560, 1440)), (1920, 0));
        assert_eq!(centred((0, 0), (0, 0), (1440, 816)), (0, 0));
    }

    #[test]
    fn a_monitor_round_trips_through_its_own_spelling() {
        let default: Monitor = "default".parse().expect("infallible");
        assert!(default.is_default());
        assert_eq!(default, Monitor::default());
        assert_eq!(default.to_string(), Monitor::DEFAULT);
        // Whitespace and an empty file value are the default too, not a screen
        // named "".
        for text in ["", "   ", "DEFAULT"] {
            assert!(
                text.parse::<Monitor>().expect("infallible").is_default(),
                "{text:?}"
            );
        }

        let named: Monitor = " DP-2 ".parse().expect("infallible");
        assert_eq!(named.name(), Some("DP-2"));
        assert_eq!(named.to_string(), "DP-2");
        assert_eq!(named.to_string().parse::<Monitor>(), Ok(named));
    }

    /// The offered list is what a player has to be able to get back to the
    /// default from, so the default is on it whatever the machine has.
    #[test]
    fn the_default_is_always_the_first_monitor_offered() {
        assert_eq!(Monitor::offered(&[]), [Monitor::DEFAULT]);
        assert_eq!(
            Monitor::offered(&["eDP-1".to_string(), "DP-2".to_string()]),
            [Monitor::DEFAULT, "eDP-1", "DP-2"]
        );
        // Not repeated, however a compositor spells it.
        assert_eq!(
            Monitor::offered(&["Default".to_string(), "DP-2".to_string()]),
            [Monitor::DEFAULT, "DP-2"]
        );
        // And every entry on it round-trips to a setting that picks something.
        let available = ["eDP-1".to_string(), "DP-2".to_string()];
        for offered in Monitor::offered(&available) {
            let monitor: Monitor = offered.parse().expect("infallible");
            assert!(
                monitor.is_default() || monitor.choose(&available).is_some(),
                "{offered} is offered and selects nothing"
            );
        }
    }

    #[test]
    fn a_monitor_picks_the_screen_it_names_and_nothing_else() {
        let available = ["eDP-1".to_string(), "DP-2".to_string()];
        assert_eq!(
            "DP-2".parse::<Monitor>().unwrap().choose(&available),
            Some(1)
        );
        // Case-insensitive, because a compositor is free to change how it
        // capitalises between releases.
        assert_eq!(
            "dp-2".parse::<Monitor>().unwrap().choose(&available),
            Some(1)
        );
        // The default never names one.
        assert_eq!(Monitor::default_monitor().choose(&available), None);
        // A screen this machine does not have falls back rather than failing,
        // and `is_default` is what tells the two apart.
        let missing: Monitor = "HDMI-A-1".parse().unwrap();
        assert_eq!(missing.choose(&available), None);
        assert!(!missing.is_default());
        // Nothing partial: `DP` must not select `DP-2`.
        assert_eq!("DP".parse::<Monitor>().unwrap().choose(&available), None);
        assert_eq!(missing.choose(&[]), None);
    }

    #[test]
    fn modes_and_aspects_round_trip_through_their_names() {
        for mode in WindowMode::ALL {
            assert_eq!(mode.name().parse::<WindowMode>(), Ok(mode));
        }
        for aspect in Aspect::ALL {
            assert_eq!(aspect.name().parse::<Aspect>(), Ok(aspect));
        }
        assert!("exclusive".parse::<WindowMode>().is_err());
        assert!("16:9".parse::<Aspect>().is_err());
    }
}
