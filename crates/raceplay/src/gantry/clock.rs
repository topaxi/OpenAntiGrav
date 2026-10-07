//! Which tick the gantry's timeline starts on, and what holds it on `GO`.
//!
//! **Pulse's rule, applied to a title's own asset.** Pulse is the one title
//! whose countdown was captured: the board turns green and shows `GO` on race
//! tick 273, one tick after the thrust gate opens
//! ([`oag_race::COUNTDOWN_TICKS`]), and keeps showing it. The timeline's `GO`
//! edge is asset frame 181 there, so frame 0 is tick 92 ([`Clock::PULSE`]).
//!
//! # What Pulse does after the release: [`PULSE_PRE_LAP_WINDOW`]
//!
//! **Read from `BOOT.BIN`, confidence 85**
//! (`docs/ghidra/functions/psp-pulse-usa/gantry-clock.md`). The same law HD
//! runs, with Pulse's own numbers: the intro's last substate sets the gantry
//! mesh's time to 0 as it enters the countdown (`0x08829e6c`, beside `ready`),
//! `RaceMode_UpdateCountdown` sets it to 3.0 on the release (`0x088274b4`),
//! and from then `RaceManager_Update` (`0x08829778`) keeps it in `[3.2, 5.5)`
//! until the player's first line crossing. So the board turns green one tick
//! after the release (frame 192 is past the 181 step) and `GO` strobes over
//! frames 192..330 for as long as the craft has not crossed, which is what the
//! 21 s stationary capture showed. It replaces the chosen 216..349 loop.
//!
//! The maintainer's rule (2026-10-03) is that a title with no measured rule of
//! its own runs Pulse's, labelled **inherited from Pulse, unmeasured on
//! <title>**. Inheriting means the *rule*, not the literal 92: the title's own
//! `GO` edge lands on the same tick and the clock is held on `GO` from there.
//! [`go_edge`] finds that edge in the title's own asset and [`Clock::inherited`]
//! turns it into a start tick. Where no edge can be found the clock runs from
//! the race start, as it always did, and the loader says so.
//!
//! # What `GO` is on Wipeout HD
//!
//! HD authors no `TEXOFFSET` step. Its board is one material's Edge Animation
//! curve (`docs/formats/edge-animation.md`) walking `uvOffset` over a texture
//! whose staircase is Pulse's at four times the size. The backdrop's red and
//! its green are texels of that texture, so the edge is read the way Pulse's
//! was measured: the first frame at which a drawn vertex samples the authored
//! green. That is the frame the `u` offset finishes stepping across to the
//! green marker column - the same "green step" Pulse's tick 273 was pinned on.
//! HD's own countdown was captured on RPCS3 on 2026-10-04 (confidence 75): the
//! green step lands on the release within one 30 fps video frame. Since the
//! release jump below, that bounds the start tick from below rather than
//! measuring it: any start that keeps the free run short of the edge at the
//! release turns the board green on the release.
//!
//! # What HD does after the release: [`ReleaseWindow`]
//!
//! HD's clock is not held on a loop of this project's choosing. **Read from
//! the EBOOT** (`docs/ghidra/functions/ps3-hdfury-eu/gantry-clock.md`): the
//! billboard's curve time is the gantry node's own animation time, and the
//! race manager's per-frame update (`0x0005e948`) keeps that time inside a
//! window for the race phase it is in, resetting it to the window's start when
//! it is outside. Before the craft first crosses the line the window is
//! `[3.83, 5.25)` s, so on the release the clock jumps from frame ~203 to frame
//! 229.8 (`GO` already lit) and then loops 86 ticks. [`HD_PRE_LAP_WINDOW`].

use oag_mesh::mesh::{DrawCall, Model};
use oag_mesh::mesh_render::TexAnims;

use oag_race::COUNTDOWN_TICKS;

use super::{CLOCK_START_TICK, HD_LAP_WINDOWS, LapWindows, PULSE_LAP_WINDOWS};

/// How far into the asset's timeline [`go_edge`] looks, in frames: the 6.000 s
/// at which both Pulse's and HD's countdown panels leave and their texture
/// loops close.
pub const SEARCH_FRAMES: u32 = 360;

/// A node counts as having moved once its translation is this far from where
/// it sat at the `GO` edge, in world units. The panels' own exits are about
/// ten.
const MOVED: f32 = 1.0;

/// The gantry's timeline clock: where it starts and what holds it on `GO`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Clock {
    /// The race tick the timeline starts on.
    pub start_tick: u64,
    /// The span, in seconds, the clock loops once `GO` is up, or `None` to run
    /// the authored timeline straight on.
    pub hold: Option<(f32, f32)>,
    /// A window the title's own race code forces the clock into from a tick
    /// on, taking over from `hold`: Pulse's [`PULSE_PRE_LAP_WINDOW`], HD's
    /// [`HD_PRE_LAP_WINDOW`]. `None` on every inherited clock.
    pub window: Option<ReleaseWindow>,
    /// The windows the race manager picks by lap after the first crossing.
    /// `None` on every inherited clock.
    pub laps: Option<LapWindows>,
}

/// A race-phase window the original's race manager keeps the gantry's time
/// in, resetting it to `from` whenever it reads outside `[from, to)`.
///
/// One tick of the window is: advance the time by one 60 Hz tick, then reset it
/// to `from` if it is outside. The two halves run in different functions in
/// the original (the window check in the race manager, the advance in the
/// node's own update), and which runs first in a frame was not read, so the
/// visible phase carries one tick of uncertainty.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReleaseWindow {
    /// The time the clock is reset to, in seconds.
    pub from: f32,
    /// The first time outside the window, in seconds.
    pub to: f32,
    /// The race tick the window first applies on.
    pub from_tick: u64,
}

/// HD's window before the first line crossing: `[3.83, 5.25)` s, from the
/// release on.
///
/// **Read from `EBOOT.BIN`, confidence 85.** `0x0005e948` (the race manager's
/// per-frame update) compares the gantry node's time against the TOC floats at
/// `0x008a6a74` (`0x40751eb8`, 3.83) and `0x008a6a90` (`0x40a80000`, 5.25) while
/// the player's lap counter (`ship+0x7810`) is 0, and calls `SetTime(3.83)`
/// (`0x002c1b30` -> `0x002be978`) when it is outside. The window code runs in
/// race phases 2 to 4 (`RaceManager+0x1970`); that phase 2 begins on the
/// release is the 2026-10-04 RPCS3 capture (the green step on the race clock's
/// zero, confidence 75), so `from_tick` is the tick after the thrust gate, the
/// tick the inherited rule already lands the green on. The same capture's `GO`
/// dark centres, +23, +61 and +109 ticks from the step, are this window's
/// frames 252.8 and 290.8 and the second loop's 252.8: 86 ticks apart.
pub const HD_PRE_LAP_WINDOW: ReleaseWindow = ReleaseWindow {
    from: f32::from_bits(0x4075_1eb8),
    to: f32::from_bits(0x40a8_0000),
    from_tick: COUNTDOWN_TICKS + 1,
};

/// Pulse's window before the first line crossing: `[3.2, 5.5)` s, from the
/// release on.
///
/// **Read from `BOOT.BIN`, confidence 85.** `RaceManager_Update`
/// (`0x08829778`), in race states 2 and later (`manager+0x7c8`), reads the
/// gantry mesh's time (`0x089128ac`, `mesh+0x40` of the `Mesh` under
/// `billboards[8]+0x3c`) and, while the player craft's crossing count
/// (`craft+0xac8`) is 0, compares it against the floats at `0x08a7a49c`
/// (`0x404ccccd`, 3.2) and `0x08a7a4a0` (`0x40b00000`, 5.5), calling
/// `SetTime(3.2)` (`0x08912890`) when it is outside. The release sets it to
/// 3.0 first (`RaceMode_UpdateCountdown`, `0x08a7a498`), which is outside, so
/// the tick after the release reads 3.2. Which of the race manager's check
/// and the mesh's own advance runs first in a frame was not read: one tick of
/// phase uncertainty, as on HD.
pub const PULSE_PRE_LAP_WINDOW: ReleaseWindow = ReleaseWindow {
    from: f32::from_bits(0x404c_cccd),
    to: f32::from_bits(0x40b0_0000),
    from_tick: COUNTDOWN_TICKS + 1,
};

impl ReleaseWindow {
    /// Whether `seconds` is inside the window.
    fn holds(&self, seconds: f32) -> bool {
        seconds >= self.from && seconds < self.to
    }

    /// Ticks from `from` until the next reset: the loop's period.
    #[must_use]
    pub fn period(&self) -> u64 {
        self.ticks_inside(self.from)
    }

    /// How many ticks the clock stays inside from `start`, `start` included.
    fn ticks_inside(&self, start: f32) -> u64 {
        let mut n = 0u64;
        while self.holds(start + n as f32 / 60.0) {
            n += 1;
        }
        n
    }

    /// The clock `ticks` after [`ReleaseWindow::from_tick`], given the time
    /// the free-running clock read on that tick.
    #[must_use]
    pub fn seconds(&self, at_from_tick: f32, ticks: u64) -> f32 {
        let start = if self.holds(at_from_tick) {
            at_from_tick
        } else {
            self.from
        };
        let first = self.ticks_inside(start);
        if ticks < first {
            start + ticks as f32 / 60.0
        } else {
            self.from + ((ticks - first) % self.period()) as f32 / 60.0
        }
    }
}

impl Clock {
    /// The timeline off the race start, never held: how every title ran
    /// before Pulse's countdown was measured, and what a title whose `GO` edge
    /// cannot be found still does - **chosen, not measured**.
    pub const FROM_RACE_START: Self = Self {
        start_tick: 0,
        hold: None,
        window: None,
        laps: None,
    };

    /// Pulse's: frame 0 on tick 92, **measured** (confidence 85), then from
    /// the release the race manager's windows, **read** (confidence 85):
    /// [`PULSE_PRE_LAP_WINDOW`] and [`PULSE_LAP_WINDOWS`].
    pub const PULSE: Self = Self {
        start_tick: CLOCK_START_TICK,
        hold: None,
        window: Some(PULSE_PRE_LAP_WINDOW),
        laps: Some(PULSE_LAP_WINDOWS),
    };

    /// Pulse's rule on a title's own asset: `edge`'s `GO` frame lands on the
    /// tick after the thrust gate opens, as Pulse's frame 181 lands on 273, and
    /// the clock is held over the frames from the digits' last fade to the
    /// asset's own exit.
    ///
    /// `None` when the edge is later than the release, which would need a
    /// negative start tick.
    #[must_use]
    pub fn inherited(edge: GoEdge) -> Option<Self> {
        let start_tick = (COUNTDOWN_TICKS + 1).checked_sub(u64::from(edge.frame))?;
        Some(Self {
            start_tick,
            hold: Some((
                edge.settled_frame as f32 / 60.0,
                edge.last_frame_before_exit as f32 / 60.0,
            )),
            window: None,
            laps: None,
        })
    }

    /// HD's: the timeline free-runs from the inherited start tick until the
    /// release and is then kept in [`HD_PRE_LAP_WINDOW`] by the rule HD's own
    /// race manager runs.
    ///
    /// `None` when the edge is later than the release, as for
    /// [`Clock::inherited`].
    #[must_use]
    pub fn hd(edge: GoEdge) -> Option<Self> {
        Some(Self {
            hold: None,
            window: Some(HD_PRE_LAP_WINDOW),
            laps: Some(HD_LAP_WINDOWS),
            ..Self::inherited(edge)?
        })
    }

    /// The gantry's clock at `tick` with the board in a later race-manager
    /// window, `entry`, from the line crossing that entered it.
    ///
    /// The time on entry is what [`Self::seconds`] gives there, and is never
    /// inside a later window (the pre-lap clock stays below 5.25 s and the
    /// windows ascend), so every entry resets to the window's start, as
    /// `0x0005f110` (HD) and `0x08829a44` (Pulse) do. `entry` is ignored on
    /// a clock with no [`Self::laps`] - every inherited one - and before its
    /// own tick.
    #[must_use]
    pub fn seconds_in(&self, tick: u64, entry: Option<super::WindowEntry>) -> f32 {
        if let Some(laps) = &self.laps
            && let Some(entry) = entry
            && tick >= entry.since
            && let Some(window) = entry.release_window(laps)
        {
            return window.seconds(self.seconds(entry.since), tick - entry.since);
        }
        self.seconds(tick)
    }

    /// The gantry's clock in seconds at race tick `tick`.
    #[must_use]
    pub fn seconds(&self, tick: u64) -> f32 {
        if let Some(window) = self.window
            && tick >= window.from_tick
        {
            let at_release = super::clock_seconds(self.start_tick, window.from_tick);
            return window.seconds(at_release, tick - window.from_tick);
        }
        let seconds = super::clock_seconds(self.start_tick, tick);
        match self.hold {
            Some(span) => super::held_within(seconds, span),
            None => seconds,
        }
    }
}

/// Where a title's own asset shows `GO`, in frames of its own timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoEdge {
    /// The first frame a drawn vertex samples the authored green.
    pub frame: u32,
    /// The first frame from `frame` on at which every vertex that sampled lit
    /// white before the edge - the digits - samples no alpha at all: the
    /// earliest the clock can loop back to without replaying a digit. `frame`
    /// itself where they never clear.
    pub settled_frame: u32,
    /// The last frame before a drawn node first leaves its place after the
    /// edge: where the asset stops showing `GO` and starts to exit.
    pub last_frame_before_exit: u32,
}

/// The vertices an animated texture track reaches, once each: `(track, uv,
/// texture slot)`.
fn animated_samples(model: &Model) -> Vec<(usize, [f32; 2], usize)> {
    let mut seen = Vec::new();
    let lists: [&[DrawCall]; 3] = [
        &model.draws,
        &model.alpha_tested_draws,
        &model.transparent_draws,
    ];
    for draw in lists.into_iter().flatten() {
        let Some(texture) = draw.texture else {
            continue;
        };
        for index in &model.indices[draw.range.start as usize..draw.range.end as usize] {
            let vertex = &model.vertices[*index as usize];
            if vertex.anim == 0 {
                continue;
            }
            let sample = (vertex.anim as usize, vertex.texcoord, texture);
            if !seen.contains(&sample) {
                seen.push(sample);
            }
        }
    }
    seen
}

/// The animated node slots a drawn vertex rides.
fn moving_slots(model: &Model) -> Vec<usize> {
    let mut slots = Vec::new();
    let lists: [&[DrawCall]; 3] = [
        &model.draws,
        &model.alpha_tested_draws,
        &model.transparent_draws,
    ];
    for draw in lists.into_iter().flatten() {
        for index in &model.indices[draw.range.start as usize..draw.range.end as usize] {
            let slot = model.vertices[*index as usize].xform as usize;
            if slot >= 1 && !slots.contains(&slot) {
                slots.push(slot);
            }
        }
    }
    slots
}

/// Whether a texel is the authored green: green clearly above red and blue and
/// not transparent. HD's marker is `(0, 170, 25)` at half alpha.
fn is_green([r, g, b, a]: [u8; 4]) -> bool {
    a > 0 && g >= 128 && i32::from(g) > i32::from(r) + 64 && i32::from(g) > i32::from(b) + 64
}

/// Whether a texel is opaque white: a lit digit.
fn is_lit_white([r, g, b, a]: [u8; 4]) -> bool {
    a >= 200 && r >= 200 && g >= 200 && b >= 200
}

/// Finds `model`'s `GO` edge, or says why it cannot.
///
/// # Errors
///
/// A line for the loader report: the model has no animated texture, its
/// texels are not on the CPU, or no drawn vertex ever samples green within
/// [`SEARCH_FRAMES`].
pub fn go_edge(model: &Model) -> Result<GoEdge, String> {
    let samples = animated_samples(model);
    if samples.is_empty() {
        return Err("no drawn vertex rides an animated texture".to_string());
    }
    let mut textures = Vec::with_capacity(samples.len());
    for (_, _, slot) in &samples {
        let rgba = model
            .textures
            .get(*slot)
            .and_then(Option::as_ref)
            .and_then(|t| {
                t.to_rgba()
                    .map(|texels| (t.width as usize, t.height as usize, texels.into_owned()))
            });
        textures.push(rgba);
    }
    if textures.iter().all(Option::is_none) {
        return Err("the board's texels are not readable on the CPU".to_string());
    }

    // One table per frame, not per frame and vertex: sampling it was a third
    // of an HD race load (`docs/architecture/load-time.md`).
    let tables: Vec<TexAnims> = (0..=SEARCH_FRAMES)
        .map(|frame| TexAnims::sample(model, frame as f32 / 60.0))
        .collect();
    let texel =
        |frame: u32, (track, uv, _): &(usize, [f32; 2], usize), at: usize| -> Option<[u8; 4]> {
            let (width, height, rgba) = textures[at].as_ref()?;
            let [su, sv, ou, ov] = tables[frame as usize].transform[*track];
            let u = (uv[0] * su + ou).rem_euclid(1.0);
            let v = (uv[1] * sv + ov).rem_euclid(1.0);
            let x = ((u * *width as f32) as usize).min(width - 1);
            let y = ((v * *height as f32) as usize).min(height - 1);
            let at = (y * width + x) * 4;
            rgba.get(at..at + 4).map(|p| [p[0], p[1], p[2], p[3]])
        };

    let frame = (0..=SEARCH_FRAMES)
        .find(|&frame| {
            samples
                .iter()
                .enumerate()
                .any(|(at, s)| texel(frame, s, at).is_some_and(is_green))
        })
        .ok_or_else(|| {
            format!(
                "no drawn vertex samples the authored green in the first {SEARCH_FRAMES} frames"
            )
        })?;

    // The digits: what sampled lit white before the edge. Once all of them
    // are clear the board shows no digit, and a loop that starts there never
    // replays one - Pulse's loop starts 35 frames after its edge for this.
    let digits: Vec<usize> = (0..samples.len())
        .filter(|&at| (0..frame).any(|f| texel(f, &samples[at], at).is_some_and(is_lit_white)))
        .collect();
    let settled_frame = (frame..=SEARCH_FRAMES)
        .find(|&f| {
            digits
                .iter()
                .all(|&at| texel(f, &samples[at], at).is_none_or(|p| p[3] == 0))
        })
        // An asset whose digits and `GO` share texels never clears (Pulse's own
        // board does not): the loop then starts at the edge, which can replay
        // a digit, and the loader line names the span so that is visible.
        .unwrap_or(frame);

    let slots = moving_slots(model);
    let at_edge = model.sample_anim_nodes(frame as f32 / 60.0);
    let exit = (frame + 1..=SEARCH_FRAMES).find(|&later| {
        let now = model.sample_anim_nodes(later as f32 / 60.0);
        slots.iter().any(|slot| {
            let (Some(a), Some(b)) = (at_edge.get(slot - 1), now.get(slot - 1)) else {
                return false;
            };
            (12..15).any(|i| (a[i] - b[i]).abs() > MOVED)
        })
    });
    Ok(GoEdge {
        frame,
        settled_frame,
        last_frame_before_exit: exit.map_or(SEARCH_FRAMES, |f| f - 1),
    })
}

#[cfg(test)]
mod tests;
