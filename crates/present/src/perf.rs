//! Frame pacing: how many frames arrive, how evenly, and how many are allowed
//! to.
//!
//! [`Meter`] and [`draw_list`] measure and report; [`FrameLimit`] is the one
//! knob that changes what is measured. They live together because a limiter
//! whose effect cannot be seen is a setting nobody can tell is working.
//!
//! # The overlay
//!
//! Two numbers, because they answer different questions. **Frames per second**
//! says whether there is headroom. **Frame pacing** says whether the frames are
//! spaced evenly, and a game can hold a perfect 60 and still stutter visibly if
//! every fourth frame takes twice as long as the three before it. An average
//! cannot show that - it is exactly what an average erases - so the graph draws
//! every frame in the window and the text reports the 99th percentile next to
//! the mean.
//!
//! # Why this is not a determinism problem
//!
//! `docs/architecture/determinism.md` forbids the simulation reading a wall
//! clock, and this module is built on nothing else. The rule holds because
//! **nothing here is read by the simulation**: [`Meter`] is fed durations by
//! the composition root's frame loop and produces [`Draw`]s, and no path leads
//! from a [`Stats`] back into a tick. Keeping [`Meter::record`] taking a
//! duration rather than reading the clock itself is what makes that checkable -
//! the whole module is a pure function of a fed sequence, and its tests feed
//! one.
//!
//! [`FrameLimit`] is the same argument from the other side: it changes how
//! often the *loop* runs, never how far a tick advances. The timestep is fixed
//! at 60 Hz per ADR-0007 whatever the frame rate is, so limiting to 30 gives
//! two ticks a frame rather than a slow-motion race.
//!
//! # Where it is drawn
//!
//! Onto the **surface**, after `upscale::Framebuffer::composite` has already
//! put the frame there - so the overlay is rasterised at presentation
//! resolution whatever the render scale is, per
//! [ADR-0036](../../../docs/architecture/adr/0036-ui-composites-at-presentation-resolution.md).
//! It still lays out in the same 480x272 space as the front end and the menus;
//! that grid is mapped onto the aspect rectangle of the surface rather than of
//! the offscreen target, which is the same rectangle on screen.
//!
//! **This used to be the other way round**, and the argument for it was that
//! "the overlay should cost what the game costs, or it is measuring a frame
//! that does not exist". That holds for a render scale a player sets once. It
//! does not hold for one that moves: this overlay is what somebody reads to
//! judge what a resolution controller is doing, and an overlay resampling along
//! with the scene cannot be read while it moves. Its cost also has to leave the
//! scene's budget before a GPU-timestamp signal can measure the part that
//! scales.
//!
//! Two consequences worth knowing. The overlay is **not graded** - brightness
//! and gamma ride in the composite it comes after - which is deliberate for an
//! instrument and is the one standing exception to ADR-0036: the HUD, which
//! moved to presentation resolution in the same way, *is* graded, because it
//! draws into the presentation target before that pass rather than onto the
//! surface after it. And the overlay is no longer touched by FXAA, SMAA or
//! FSR 1, so its glyphs stop being edge-detected and sharpened as though they
//! were scene content.
//!
//! It is **window-only**. `--screenshot` runs the sequence as fast as it can
//! with no presentation at all, so a frame time from it would be a real
//! measurement of something nobody is asking about.

use serde::{Deserialize, Serialize};

use oag_ui::frontend::{Align, Draw};

pub mod cost;
pub mod memory;
pub mod scene;
pub mod transition;

pub use cost::{CpuCost, GpuCost};
pub use scene::SceneStats;

/// How many frames the meter remembers: two seconds at 60 Hz.
///
/// Long enough that a single hitch is visible for long enough to read, short
/// enough that the numbers still follow what is happening now. It is also the
/// graph's width in pixels, one column per frame, so no sample is dropped or
/// doubled on the way to the screen.
pub const WINDOW: usize = 120;

/// The longest frame the meter will believe, in seconds.
///
/// A load - a track, a ship, a set of pipelines - lands in exactly one frame's
/// duration and is not a frame time in any useful sense. The composition root
/// calls [`Meter::clear`] whenever it knows it has stalled; this is the
/// backstop for the stalls it does not know about, and it clamps rather than
/// discards because a two-second hitch the player *saw* should not vanish from
/// the graph that exists to show hitches.
const LONGEST: f32 = 1.0;

/// What the overlay shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Overlay {
    /// Nothing.
    #[default]
    Off,
    /// One line: the frame rate and the mean frame time.
    Fps,
    /// The line, a second line of pacing figures, and a graph of every frame in
    /// the window.
    Pacing,
    /// Everything [`Self::Pacing`] shows, plus a line of scene counts (draw
    /// calls submitted against the total, and triangles submitted - see
    /// [`SceneStats`]), the size the scene was drawn at (see
    /// [`RenderSize`]), a line of resident memory (see [`memory::Probe`]), and
    /// a line naming the video decoder, each present only when the caller
    /// actually has one to give it.
    ///
    /// Its own tier rather than folded into `Pacing`, since the extra lines
    /// name renderer- and process-internal things (`--lod`, frustum
    /// culling, resident memory) most players reaching for `pacing` are not
    /// asking about.
    Dev,
}

impl Overlay {
    /// The spelling used in a settings file and on a menu row.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Fps => "fps",
            Self::Pacing => "pacing",
            Self::Dev => "dev",
        }
    }

    /// Whether anything is drawn at all.
    #[must_use]
    pub fn is_on(self) -> bool {
        self != Self::Off
    }

    /// Every mode, for the menus and for error messages.
    pub const ALL: [Self; 4] = [Self::Off, Self::Fps, Self::Pacing, Self::Dev];
}

impl std::str::FromStr for Overlay {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|mode| mode.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| {
                format!("{text:?} is not a performance overlay; try off, fps, pacing or dev")
            })
    }
}

impl std::fmt::Display for Overlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// How the finished frame reaches the display.
///
/// **"Double buffering or triple buffering" is not a setting any more**, and
/// this is what it became. On Vulkan - which is what wgpu is - the buffer count
/// is not the knob; the *present mode* is, and it decides both whether the
/// picture can tear and whether the loop is allowed to run ahead of the
/// refresh. The three below are the three answers that differ from a player's
/// side, not the six the API has.
///
/// The middle one is the one with the cliff. Classic vsync blocks until the
/// next refresh, so missing it by a microsecond costs a whole one and 144
/// becomes 72 - which is the exact problem triple buffering was invented for,
/// and why [`Vsync::Smooth`] is worth a row of its own rather than being the
/// same thing as "on".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "VsyncRepr", into = "String")]
pub enum Vsync {
    /// No waiting at all: Vulkan's `Immediate`. Tears, and has the lowest
    /// latency there is.
    ///
    /// The default, because this is a racing game: vsync's cost is a frame of
    /// latency, and that is the one thing a player steering at 60 Hz feels
    /// directly. [`FrameLimit`] is what stops it running at four figures.
    #[default]
    Off,
    /// Waits for the refresh: Vulkan's `Fifo`, the only mode every device is
    /// required to have.
    ///
    /// Never tears, and has the half-rate cliff described above. This is the
    /// one mode where the display decides how often a frame is presented, so it
    /// is the one that makes [`FrameLimit`] meaningless.
    ///
    /// Measured on a 60 Hz panel: 59 frames a second at a 16.8 ms mean - and a
    /// **34.1 ms maximum**, which is the cliff caught in the act. One frame in
    /// the window missed its refresh and waited for the next, and that single
    /// doubled frame is exactly what [`Vsync::Smooth`] exists to avoid.
    On,
    /// Presents the newest finished frame at the refresh and throws the rest
    /// away: Vulkan's `Mailbox`, which is triple buffering done properly.
    ///
    /// Spelled `smooth` rather than `triple` because the buffer count is not
    /// what a player is choosing - what they get is "never tears, never
    /// halves", and how many images the swapchain holds to do it is the
    /// driver's business.
    ///
    /// Never tears *and* never halves - the loop is not blocked, so a frame
    /// that misses one refresh simply catches the next. What it costs is the
    /// frames it discards, which is why a limiter is still worth having here
    /// and why this mode does **not** grey the limit row out.
    ///
    /// **It does not pin to the refresh, and it does not free-run either.**
    /// Measured on the same 60 Hz panel with no limit: [`Vsync::Off`] gave
    /// 1356 frames a second and this gave 240, because acquiring an image
    /// blocks once they are all in flight. Four times the refresh is not a rate
    /// anybody asked for, which is the practical argument for keeping the limit
    /// row live here rather than the theoretical one.
    Smooth,
}

impl Vsync {
    /// The spelling used in a settings file and on a menu row.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::On => "on",
            Self::Smooth => "smooth",
        }
    }

    /// Whether the display decides the rate, which is what makes a frame limit
    /// pointless.
    ///
    /// True for [`Vsync::On`] alone. `Smooth` looks like vsync and is not, in
    /// the way that matters here: it caps *presentation* at the refresh and
    /// leaves *rendering* unbounded, so without a limit it will happily render
    /// a thousand frames a second and discard nine hundred and forty of them.
    #[must_use]
    pub fn paces_itself(self) -> bool {
        self == Self::On
    }

    /// Every mode, for the menus and for error messages.
    pub const ALL: [Self; 3] = [Self::Off, Self::On, Self::Smooth];
}

impl std::str::FromStr for Vsync {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|mode| mode.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| format!("{text:?} is not a vsync mode; try off, on or smooth"))
    }
}

impl std::fmt::Display for Vsync {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// What a settings file is allowed to say, which includes what an older one
/// already says.
///
/// This was a `bool` for exactly one commit before the smooth mode made it
/// three-valued, and a file written in between would otherwise fail to parse
/// with "invalid type: boolean". Accepting both costs twenty lines and the
/// canonical rewrite normalises it to a name on the next run, so the
/// compatibility does not accumulate.
#[derive(Deserialize)]
#[serde(untagged)]
enum VsyncRepr {
    Flag(bool),
    Name(String),
}

impl TryFrom<VsyncRepr> for Vsync {
    type Error = String;

    fn try_from(repr: VsyncRepr) -> Result<Self, Self::Error> {
        match repr {
            VsyncRepr::Flag(true) => Ok(Self::On),
            VsyncRepr::Flag(false) => Ok(Self::Off),
            VsyncRepr::Name(name) => name.parse(),
        }
    }
}

impl From<Vsync> for String {
    fn from(mode: Vsync) -> Self {
        mode.to_string()
    }
}

/// How many frames a second the loop may produce, or unlimited.
///
/// **Not consulted under [`Vsync::On`].** There the display decides, and
/// a second limiter underneath it would either do nothing or beat against the
/// refresh - which is worse than no limiter at all, because it turns an even
/// 60 into an uneven one. The graphics page says so by greying the row out; see
/// `docs/architecture/menus.md`.
///
/// Zero is unlimited rather than a rate of zero, which is why the field is
/// private and [`FrameLimit::hz`] returns an `Option`: nothing outside this type
/// has to know the sentinel, and nothing can divide by it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct FrameLimit(u32);

impl FrameLimit {
    /// As fast as the loop can go.
    pub const UNLIMITED: Self = Self(0);

    /// The fastest limit that can be asked for.
    ///
    /// Not a preference: past about here the limiter is measuring intervals
    /// shorter than the scheduler's own resolution, so what it delivers stops
    /// being what it was asked for. A player who wants more than this wants
    /// [`FrameLimit::UNLIMITED`], and that is a different row value rather than
    /// a larger number.
    pub const MAX: u32 = 1000;

    /// What a fresh install is limited to, with vsync off by default.
    ///
    /// **240**, which is the top of the tier displays are actually built at
    /// (144, 165, 240) rather than a number picked for roundness: it does not
    /// cap any common panel, and it still stops a menu page running the GPU at
    /// four figures for no picture anybody can see. Unlimited is a row away for
    /// a player who wants it, and the point of a default is that the run nobody
    /// configured is already sane.
    pub const DEFAULT: Self = Self(240);

    /// The limits the menus offer: unlimited, then the refresh rates displays
    /// are actually built at, then the ceiling.
    pub const OFFERED: [Self; 15] = [
        Self::UNLIMITED,
        Self(30),
        Self(60),
        Self(70),
        Self(72),
        Self(75),
        Self(90),
        Self(100),
        Self(120),
        Self(144),
        Self(165),
        Self(240),
        Self(360),
        Self(480),
        Self(Self::MAX),
    ];

    /// The rate asked for, or `None` for unlimited.
    #[must_use]
    pub fn hz(self) -> Option<u32> {
        (self.0 > 0).then_some(self.0)
    }

    /// The frame limit the front end runs under, derived from this one
    /// (maintainer's rule, 2026-10-07 - **chosen, not measured**: a menu has no
    /// motion that needs more than 120 and a player who capped the race low
    /// wants the menus quiet too).
    ///
    /// - unlimited, or 120 and above: **120**;
    /// - 60 up to but not including 120: **60**;
    /// - below 60: this limit unchanged.
    ///
    /// Exactly 120 reads as "above or at" and takes the 120 arm. A race keeps
    /// the limit as configured; only the front end calls this.
    #[must_use]
    pub fn front_end(self) -> Self {
        match self.hz() {
            None => Self(Self::FRONT_END_FAST),
            Some(hz) if hz >= Self::FRONT_END_FAST => Self(Self::FRONT_END_FAST),
            Some(hz) if hz >= Self::FRONT_END_STEADY => Self(Self::FRONT_END_STEADY),
            Some(_) => self,
        }
    }

    const FRONT_END_FAST: u32 = 120;
    const FRONT_END_STEADY: u32 = 60;

    /// How long one frame is allowed to take at the least, or `None` for
    /// unlimited.
    #[must_use]
    pub fn period(self) -> Option<std::time::Duration> {
        self.hz()
            .map(|hz| std::time::Duration::from_nanos(1_000_000_000 / u64::from(hz)))
    }
}

impl Default for FrameLimit {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl std::str::FromStr for FrameLimit {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        if text.eq_ignore_ascii_case("unlimited") {
            return Ok(Self::UNLIMITED);
        }
        let hz: u32 = text
            .parse()
            .map_err(|_| format!("{text:?} is not a frame limit; write a rate or \"unlimited\""))?;
        // Zero is rejected rather than accepted as a second spelling of
        // unlimited: two spellings for one value is how a settings file and a
        // menu row start disagreeing about what is selected.
        if hz == 0 || hz > Self::MAX {
            return Err(format!(
                "a frame limit is 1 to {} frames a second, or \"unlimited\"",
                Self::MAX
            ));
        }
        Ok(Self(hz))
    }
}

impl std::fmt::Display for FrameLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.hz() {
            Some(hz) => write!(f, "{hz}"),
            None => f.write_str("unlimited"),
        }
    }
}

impl TryFrom<String> for FrameLimit {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<FrameLimit> for String {
    fn from(limit: FrameLimit) -> Self {
        limit.to_string()
    }
}

/// A ring of the most recent frame durations, in seconds.
///
/// Fixed size and no allocation: this is fed once per frame forever, and a
/// growing buffer measuring frame times would eventually be the thing making
/// them worse.
#[derive(Debug, Clone)]
pub struct Meter {
    samples: [f32; WINDOW],
    /// How many of `samples` are real. Saturates at [`WINDOW`].
    len: usize,
    /// Where the next sample goes, which is also the oldest one once full.
    next: usize,
}

impl Default for Meter {
    fn default() -> Self {
        Self::new()
    }
}

impl Meter {
    /// An empty meter, which reports nothing until it has been fed.
    #[must_use]
    pub fn new() -> Self {
        Self {
            samples: [0.0; WINDOW],
            len: 0,
            next: 0,
        }
    }

    /// Records one frame, given how long it took in seconds.
    ///
    /// **Takes the duration rather than reading a clock**, which is what makes
    /// every number below testable against a fed sequence instead of against
    /// whatever the machine happened to do.
    ///
    /// A duration that is not a finite positive number is dropped: a clock that
    /// went backwards is not a fast frame, and a zero would make the frame rate
    /// infinite. Anything longer than a second is clamped - see [`LONGEST`].
    pub fn record(&mut self, seconds: f32) {
        if !seconds.is_finite() || seconds <= 0.0 {
            return;
        }
        self.samples[self.next] = seconds.min(LONGEST);
        self.next = (self.next + 1) % WINDOW;
        self.len = (self.len + 1).min(WINDOW);
    }

    /// Forgets everything recorded so far.
    ///
    /// Called by the frame loop whenever it knows it has just stalled - opening
    /// the menus, loading a race - because that stall is a load and not a frame
    /// time, and one 1000 ms column would otherwise dominate the graph for the
    /// two seconds a player is most likely to be looking at it.
    pub fn clear(&mut self) {
        self.len = 0;
        self.next = 0;
    }

    /// Whether anything has been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Every remembered frame, oldest first.
    fn frames(&self) -> impl Iterator<Item = f32> + '_ {
        // Full, the oldest sample is the one `next` is about to overwrite;
        // partly filled, nothing has wrapped yet and the oldest is index 0.
        let start = if self.len == WINDOW { self.next } else { 0 };
        (0..self.len).map(move |i| self.samples[(start + i) % WINDOW])
    }

    /// What the window says, or `None` if nothing usable is in it.
    #[must_use]
    pub fn stats(&self) -> Option<Stats> {
        if self.len == 0 {
            return None;
        }
        let mut sorted = [0.0f32; WINDOW];
        let mut total = 0.0;
        for (slot, frame) in sorted.iter_mut().zip(self.frames()) {
            *slot = frame;
            total += frame;
        }
        let window = &mut sorted[..self.len];
        // `total_cmp` rather than `partial_cmp`: `record` already rejected the
        // NaN that would make a partial ordering panic, and asking for a total
        // one means this cannot start depending on that having happened.
        window.sort_unstable_by(f32::total_cmp);

        if total <= 0.0 {
            return None;
        }
        let count = self.len as f32;
        // The 99th percentile by nearest rank, which at a full window is the
        // second-worst frame. Reported instead of the worst because one outlier
        // - the frame a compositor decided to reconfigure the surface on - is
        // not what stutter feels like; it is the shape of the tail that is.
        let rank = (((self.len - 1) as f32) * 0.99).round() as usize;

        Some(Stats {
            fps: count / total,
            mean_ms: (total / count) * 1000.0,
            p99_ms: window[rank] * 1000.0,
            worst_ms: window[self.len - 1] * 1000.0,
        })
    }
}

/// What a window of frames adds up to. All times in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stats {
    /// Frames divided by the time they took, which is not `1000 / mean_ms`
    /// rounded - it is the same number, and it is computed from the total so a
    /// short window is still honest.
    pub fps: f32,
    /// Mean frame time.
    pub mean_ms: f32,
    /// 99th percentile frame time: what the slow frames cost.
    pub p99_ms: f32,
    /// The longest frame in the window.
    pub worst_ms: f32,
}

/// The overlay's grid height. Its width follows the window's own aspect (see
/// [`grid`]), so the panels sit on the window's real edges whatever its shape.
pub const GRID_H: f32 = 272.0;

/// The grid the overlay lays out in for a window of `size` pixels: [`GRID_H`]
/// tall and as wide as the window's aspect makes it, shown as itself.
///
/// **The window, not the game's aspect rectangle.** The rest of the frame is
/// fitted into the aspect the player chose, with bars beside it; the overlay
/// is an instrument, and an instrument belongs on the edge of the glass, not
/// floating at the edge of a 480x272 box that moves with the resolution.
#[must_use]
pub fn grid(size: (u32, u32)) -> oag_display::space::Space {
    let aspect = size.0.max(1) as f32 / size.1.max(1) as f32;
    oag_display::space::Space {
        size: (GRID_H * aspect, GRID_H),
        display_aspect: aspect,
    }
}

/// How far the panels sit from the window's edges, in grid units.
///
/// Top right for the frame-time panel and top left for the cost panel, both
/// flush to the window's corner. Nothing here is recovered from anything; it
/// is chosen, to stay legible at the smallest window the game opens.
const EDGE: f32 = 0.0;
const TOP: f32 = EDGE;
const PAD: f32 = 4.0;
const LINE: f32 = 10.0;
/// One column per remembered frame, so the graph is [`WINDOW`] pixels wide.
const GRAPH_W: f32 = WINDOW as f32;
const GRAPH_H: f32 = 30.0;
const PANEL_W: f32 = GRAPH_W + PAD * 2.0;

/// Where the GPU-cost panel sits: top left, mirroring the frame-time panel off
/// the opposite edge.
///
/// **Its own panel rather than folded into the frame-time one**, which is
/// what it used to be: `GpuCost::rows` puts one reading a row rather than
/// several crammed onto one line, and a reader comparing "does GPU add up to
/// the frame time" wants the two numbers in two places their eye can jump
/// between, not five values sharing a line with a p99 and a triangle count.
/// Left rather than another right-side panel because the front end and the
/// menus both start at the left margin too - but neither of those routes ever
/// has a `GpuCost` to draw, so there is nothing there for this to collide
/// with in practice.
const GPU_LEFT: f32 = EDGE;
/// Wide enough for the longest single row, `FSR3 REN 100.00 MS` at three
/// digits - picked to stay legible, the same way [`PANEL_W`] was. It was
/// `SCENE 100.00 MS` and 90 until the FSR 3.1 chain became two rows rather
/// than one (ADR-0045).
///
/// The *height* is not a constant: the panel is sized to however many rows
/// have readings, which is up to nine now that [`CpuCost`]'s three wall-clock
/// rows sit under the five timed ones and `OTHER`.
const GPU_PANEL_W: f32 = 110.0;

/// The panel, the text, and the three bands a frame time can fall in.
const PANEL: [f32; 4] = [0.0, 0.0, 0.0, 0.55];
const TEXT: [f32; 4] = [0.92, 0.95, 1.0, 1.0];
const GOOD: [f32; 4] = [0.35, 0.86, 0.45, 0.9];
const WARN: [f32; 4] = [0.96, 0.78, 0.25, 0.9];
const BAD: [f32; 4] = [0.94, 0.32, 0.3, 0.95];
/// The line across the graph at one target frame time.
const RULE: [f32; 4] = [1.0, 1.0, 1.0, 0.35];

/// How much over the target still counts as on time.
///
/// A little over 5 %, so a 60 Hz target does not paint every frame amber for
/// arriving at 16.8 ms instead of 16.67. The graph exists to show stutter, and
/// a graph that is always amber shows nothing.
const SLACK: f32 = 1.05;

/// The height of the graph, in frame times: two target frames, so the target
/// itself sits exactly halfway up and the rule is easy to read against.
const GRAPH_FRAMES: f32 = 2.0;

/// What the scene was actually drawn at this frame.
///
/// Two sizes rather than one, per
/// [ADR-0037](../../../docs/architecture/adr/0037-dynamic-resolution-varies-a-viewport-not-an-allocation.md):
/// the **allocation** is the texture, sized off the `[graphics] render_scale`
/// ceiling, and the **extent** is the sub-rectangle drawn into it this frame.
/// They are equal on every frame the game draws today, because nothing moves
/// the extent yet - and the whole point of putting this on screen is that when
/// something does, a player can watch it move.
///
/// **This is the row that tells dynamic resolution from a stutter**, which is
/// the argument for a fourth `Dev` line on an already dense readout. A frame
/// rate that recovers because the controller dropped the resolution and one
/// that recovers because the load passed look identical without it. It is also
/// readable *while* the scale moves, which is what
/// [ADR-0036](../../../docs/architecture/adr/0036-ui-composites-at-presentation-resolution.md)
/// bought by taking this overlay out of the scaled target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderSize {
    /// The rectangle drawn into - `upscale::Framebuffer::extent`.
    pub extent: (u32, u32),
    /// The texture behind it - `upscale::Framebuffer::allocation`.
    pub allocation: (u32, u32),
}

impl RenderSize {
    /// One line, and it says less when there is less to say.
    ///
    /// With the two equal - every frame today - it is the render size alone,
    /// because "1440x816 of 1440x816, 100 %" is three ways of saying one
    /// number. Below the ceiling it names both and the percentage between
    /// them, which is the number a controller is moving.
    ///
    /// The percentage is off the **width**. Both axes are scaled by the same
    /// factor, so either would do, and quoting one avoids a row that reads
    /// `84% / 85%` because the two rounded differently.
    #[must_use]
    pub fn line(self) -> String {
        let (w, h) = self.extent;
        let (aw, ah) = self.allocation;
        if (w, h) == (aw, ah) {
            return format!("RENDER {w}x{h}");
        }
        let percent = f64::from(w) / f64::from(aw.max(1)) * 100.0;
        format!("RENDER {w}x{h} OF {aw}x{ah}  {percent:.0}%")
    }
}

/// The overlay, as plain data.
///
/// The same [`Draw`] vocabulary the front end and the menus emit, so the
/// composition root rasterises it with the renderer it already has.
///
/// `target_hz` is what a frame is being measured against: **the rate the loop
/// is actually trying to present at**, which is [`FrameLimit`] when one is in
/// force and the simulation's rate otherwise. It only sets the colours and the
/// graph's scale; the numbers are measured, not scaled - but get it wrong and
/// the graph says nothing, because against a target four times too slow every
/// column is a green sliver at the floor.
///
/// `memory` is the process's resident set size in bytes, from
/// [`memory::Probe::sample`] - `None` when the platform has no reader or the
/// probe has not sampled yet, in which case the line is left out the same way
/// an absent `scene` or `video` already is.
///
/// `render` is what the scene was drawn at, and is `None` on a stage that has
/// no scene - the launcher, the loading screen, the front end and the menus
/// all draw straight into the presentation target and never touch the scaled
/// one ([ADR-0038](../../../docs/architecture/adr/0038-a-stage-with-no-scene-draws-at-presentation-resolution.md)),
/// so a size there would name a texture nothing on screen came from. Same
/// discipline as `scene`: absent rather than stale.
///
/// `gpu` and `cpu` are the two clocks a frame can be asked about, and they
/// share one panel because they are read as one answer - see [`cost`] for why
/// only the second of them adds up to the frame time.
#[must_use]
#[expect(clippy::too_many_arguments, reason = "one row's worth of GPU cost")]
pub fn draw_list(
    meter: &Meter,
    mode: Overlay,
    target_hz: u32,
    scene: Option<SceneStats>,
    video: Option<&str>,
    memory: Option<u64>,
    render: Option<RenderSize>,
    gpu: GpuCost,
    cpu: CpuCost,
    width: f32,
) -> Vec<Draw> {
    if !mode.is_on() {
        return Vec::new();
    }
    let Some(stats) = meter.stats() else {
        return Vec::new();
    };
    let target_ms = 1000.0 / target_hz.max(1) as f32;
    // The window's right edge in the grid [`grid`] built, less the margin.
    let right = width - EDGE;

    let graph = mode == Overlay::Pacing || mode == Overlay::Dev;
    let mut lines = vec![format!("{:.0} FPS  {:.1} MS", stats.fps, stats.mean_ms)];
    if graph {
        lines.push(format!(
            "P99 {:.1}  MAX {:.1}",
            stats.p99_ms, stats.worst_ms
        ));
    }
    // The scene's own numbers, so `--lod` and frustum culling have
    // something on screen to show they did anything at all - a stage with no
    // scene (the front end, the menus) passes `None` rather than a stats
    // value of all zeroes, which would read as "nothing is ever drawn". Only
    // under `Dev`: a player choosing `pacing` wants frame timing, not a
    // renderer-internal count.
    if mode == Overlay::Dev
        && let Some(scene) = scene
    {
        let total = scene.draws_submitted + scene.draws_culled;
        lines.push(format!(
            "DRAWS {}/{total}  TRIS {}",
            scene.draws_submitted, scene.triangles
        ));
    }
    // Directly under the draw counts, because it is the denominator for them:
    // a frame that got cheaper because the resolution fell and one that got
    // cheaper because half the track was culled are the same two numbers
    // otherwise. See [`RenderSize`].
    if mode == Overlay::Dev
        && let Some(render) = render
    {
        lines.push(render.line());
    }
    // Resident memory, not virtual: what the process is actually holding,
    // rather than address space it has merely reserved (a wgpu process's
    // virtual size is a number nobody watching for a leak wants). `None`
    // means the platform has no reader or the probe has not sampled yet -
    // left out rather than shown as zero, which would read as "no memory in
    // use" instead of "not measured".
    if mode == Overlay::Dev
        && let Some(bytes) = memory
    {
        lines.push(format!("MEM {:.1} MB", bytes as f32 / (1024.0 * 1024.0)));
    }
    // Which decoder is actually serving the movie on screen right now - the
    // front end and the menu backdrop are the only stages with one, and a
    // stage with no movie playing (a menu page with the backdrop hidden, a
    // race) passes `None` rather than a stale label. See
    // [`crate::movie::VideoDecoder::label`].
    if mode == Overlay::Dev
        && let Some(video) = video
    {
        lines.push(format!("VIDEO {video}"));
    }

    let text_height = lines.len() as f32 * LINE;
    let panel_h = PAD * 2.0 + text_height + if graph { GRAPH_H + PAD } else { 0.0 };

    let mut out = vec![Draw::Fill {
        rect: [right - PANEL_W, TOP, PANEL_W, panel_h],
        color: PANEL,
    }];

    for (row, text) in lines.into_iter().enumerate() {
        out.push(Draw::Text {
            x: right - PAD,
            y: TOP + PAD + row as f32 * LINE,
            scale: 1.0,
            color: TEXT,
            border: None,
            align: Align::Right,
            text,
            wrap_width: None,
        });
    }

    if graph {
        let left = right - PAD - GRAPH_W;
        let top = TOP + PAD + text_height + PAD;
        let bottom = top + GRAPH_H;
        let ceiling = target_ms * GRAPH_FRAMES;

        // The target, drawn under the columns so a column that reaches it is
        // not cut in half by its own reference line.
        out.push(Draw::Fill {
            rect: [left, bottom - GRAPH_H / GRAPH_FRAMES, GRAPH_W, 1.0],
            color: RULE,
        });

        // Oldest at the left, so the graph reads the way a chart does and the
        // newest frame is the one nearest the numbers above it.
        for (column, frame) in meter.frames().enumerate() {
            let ms = frame * 1000.0;
            let height = (ms / ceiling).min(1.0) * GRAPH_H;
            if height <= 0.0 {
                continue;
            }
            out.push(Draw::Fill {
                rect: [left + column as f32, bottom - height, 1.0, height],
                color: band(ms, target_ms),
            });
        }
    }

    // **Its own panel, top left** - see [`GPU_LEFT`] for why it is not a
    // second block inside the one above. `Dev`-only, the same tier every
    // other GPU-cost row here already was.
    if mode == Overlay::Dev {
        // The GPU's rows first and the wall clock's under them, in one panel
        // rather than two: they are two axes (see [`cost`]) but they are read
        // together, and a reader asking "where did the frame go" should not
        // have to find a third rectangle to finish the sentence.
        let mut gpu_rows = gpu.rows(stats.mean_ms);
        gpu_rows.extend(cpu.rows(stats.mean_ms));
        if !gpu_rows.is_empty() {
            let gpu_h = PAD * 2.0 + gpu_rows.len() as f32 * LINE;
            out.push(Draw::Fill {
                rect: [GPU_LEFT, TOP, GPU_PANEL_W, gpu_h],
                color: PANEL,
            });
            for (row, text) in gpu_rows.into_iter().enumerate() {
                out.push(Draw::Text {
                    x: GPU_LEFT + PAD,
                    y: TOP + PAD + row as f32 * LINE,
                    scale: 1.0,
                    color: TEXT,
                    border: None,
                    align: Align::Left,
                    text,
                    wrap_width: None,
                });
            }
        }
    }

    out
}

/// Which band a frame time falls in.
fn band(ms: f32, target_ms: f32) -> [f32; 4] {
    if ms <= target_ms * SLACK {
        GOOD
    } else if ms <= target_ms * GRAPH_FRAMES {
        WARN
    } else {
        BAD
    }
}

#[cfg(test)]
mod tests;
