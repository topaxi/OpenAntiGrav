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
//! Onto the **surface**, after `upscale::Framebuffer::resolve` has already put
//! the frame there - so the overlay is rasterised at presentation resolution
//! whatever the render scale is, per
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
//! and gamma ride in the resolve it now comes after - which is deliberate for
//! an instrument and is the one standing exception to ADR-0036. And it is no
//! longer touched by FXAA, SMAA or FSR 1, so its glyphs stop being edge-detected
//! and sharpened as though they were scene content.
//!
//! It is **window-only**. `--screenshot` runs the sequence as fast as it can
//! with no presentation at all, so a frame time from it would be a real
//! measurement of something nobody is asking about.

use serde::{Deserialize, Serialize};

use crate::frontend::{Align, Draw};

pub mod memory;

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
    /// [`crate::race::SceneStats`]), a line of resident memory (see
    /// [`memory::Probe`]), and a line naming the video decoder, each present
    /// only when the caller actually has one to give it.
    ///
    /// Its own tier rather than folded into `Pacing`, since the extra lines
    /// name renderer- and process-internal things (`[graphics] lod`, frustum
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

/// Where the panel sits in the 480x272 space, and how big it is.
///
/// Top right, because the front end and the menus both start at the left margin
/// and a race puts its ship in the middle. Nothing here is recovered from
/// anything; it is picked to stay legible at the smallest window the game opens.
const RIGHT: f32 = 474.0;
const TOP: f32 = 4.0;
const PAD: f32 = 4.0;
const LINE: f32 = 10.0;
/// One column per remembered frame, so the graph is [`WINDOW`] pixels wide.
const GRAPH_W: f32 = WINDOW as f32;
const GRAPH_H: f32 = 30.0;
const PANEL_W: f32 = GRAPH_W + PAD * 2.0;

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
#[must_use]
pub fn draw_list(
    meter: &Meter,
    mode: Overlay,
    target_hz: u32,
    scene: Option<crate::race::SceneStats>,
    video: Option<&str>,
    memory: Option<u64>,
) -> Vec<Draw> {
    if !mode.is_on() {
        return Vec::new();
    }
    let Some(stats) = meter.stats() else {
        return Vec::new();
    };
    let target_ms = 1000.0 / target_hz.max(1) as f32;

    let graph = mode == Overlay::Pacing || mode == Overlay::Dev;
    let mut lines = vec![format!("{:.0} FPS  {:.1} MS", stats.fps, stats.mean_ms)];
    if graph {
        lines.push(format!(
            "P99 {:.1}  MAX {:.1}",
            stats.p99_ms, stats.worst_ms
        ));
    }
    // The scene's own numbers, so `[graphics] lod` and frustum culling have
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
        rect: [RIGHT - PANEL_W, TOP, PANEL_W, panel_h],
        color: PANEL,
    }];

    for (row, text) in lines.into_iter().enumerate() {
        out.push(Draw::Text {
            x: RIGHT - PAD,
            y: TOP + PAD + row as f32 * LINE,
            scale: 1.0,
            color: TEXT,
            border: None,
            align: Align::Right,
            text,
            wrap_width: None,
        });
    }

    if !graph {
        return out;
    }

    let left = RIGHT - PAD - GRAPH_W;
    let top = TOP + PAD + text_height + PAD;
    let bottom = top + GRAPH_H;
    let ceiling = target_ms * GRAPH_FRAMES;

    // The target, drawn under the columns so a column that reaches it is not
    // cut in half by its own reference line.
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
