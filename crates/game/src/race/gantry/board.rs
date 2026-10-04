//! The gantry after the first line crossing: which race-manager window the
//! board is in, when it entered it, and which draws stand on the panel there.
//!
//! **Both titles that were read run the same law with their own numbers**
//! ([`LapWindows`]): HD's from its EBOOT, Pulse's from its `BOOT.BIN`
//! (`RaceManager_Update`, `0x08829778`,
//! `docs/ghidra/functions/psp-pulse-usa/gantry-clock.md`, confidence 85).
//!
//! **HD, read from `EBOOT.BIN`** (`RaceManager_Update`, `0x0005e948`,
//! `docs/ghidra/functions/ps3-hdfury-eu/gantry-clock.md`). Every frame the
//! race manager picks a window by the player's lap counter `ship+0x7810` and
//! the race's lap count, and resets the gantry's time to the window's start
//! whenever the time reads outside it. The windows ascend, so every switch
//! lands below the new window and is an immediate jump to its start.
//!
//! **What each window shows was measured on the original** (RPCS3, Talon's
//! Junction, 2026-10-04, each window's bounds written over the pre-lap
//! window's from the grid): the `FX-350` sponsor art animating onto the teal
//! board between laps, a strobing `FINAL LAP` on the lap before the last, and
//! a scrolling chequered flag on the last. All three are the asset's own
//! frames at those times; nothing here authors a state.

use oag_race::Standing;
use oag_render::gantry::panel;
use oag_render::mesh::Model;

use super::clock::ReleaseWindow;

/// The window between laps, `[6.017, 9.3)` s: TOC floats `0x008a6a8c` and
/// `0x008a6a88`. Shows the `FX-350` art.
pub const HD_BETWEEN_LAPS_WINDOW: (f32, f32) =
    (f32::from_bits(0x40c0_8b44), f32::from_bits(0x4114_cccd));

/// The window on the lap before the last, `[9.5, 9.9)` s: TOC floats
/// `0x008a6a94` and `0x008a6a78`. Shows `FINAL LAP`, strobing.
pub const HD_FINAL_LAP_AHEAD_WINDOW: (f32, f32) =
    (f32::from_bits(0x4118_0000), f32::from_bits(0x411e_6666));

/// The window on the last lap and after the finish, `[12.35, 13.3)` s: TOC
/// floats `0x008a6a7c` and `0x008a6a80`. Shows the chequered flag; the
/// branch also queues the `FINAL_LAP` announcer cue (`0x0005f0e0`).
pub const HD_CHEQUERED_WINDOW: (f32, f32) =
    (f32::from_bits(0x4145_999a), f32::from_bits(0x4154_cccd));

/// A title's race-manager windows after the first line crossing, and whether
/// a finished craft selects the last one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LapWindows {
    /// Any lap but the last two.
    pub between_laps: (f32, f32),
    /// The lap before the last.
    pub final_lap_ahead: (f32, f32),
    /// The last lap.
    pub chequered: (f32, f32),
    /// HD's selection also takes the finish (`ship+0x7814 - 2 >= total`) to
    /// the chequered window; Pulse's does not, so a Pulse craft past the
    /// line on its last lap is back between laps.
    pub finish_is_chequered: bool,
}

/// HD's: [`HD_BETWEEN_LAPS_WINDOW`], [`HD_FINAL_LAP_AHEAD_WINDOW`],
/// [`HD_CHEQUERED_WINDOW`], with the finish taken to the last.
pub const HD_LAP_WINDOWS: LapWindows = LapWindows {
    between_laps: HD_BETWEEN_LAPS_WINDOW,
    final_lap_ahead: HD_FINAL_LAP_AHEAD_WINDOW,
    chequered: HD_CHEQUERED_WINDOW,
    finish_is_chequered: true,
};

/// Pulse's, **read from `BOOT.BIN`, confidence 85**: `RaceManager_Update`
/// (`0x08829778`) compares the gantry mesh's time against the floats at
/// `0x08a7a4a4`..`0x08a7a4b8` by the player craft's crossing count
/// (`craft+0xac8`): `[6.0, 9.0)` on any lap but the last two, `[9.5, 12.0)`
/// on `crossings == laps - 1`, `[12.4, 13.3)` on `crossings == laps` (where it
/// also plays `FINAL_LAP` once). Its own asset's beats agree: the `Board`
/// arrives at 5.85-6.0 s, `Final_Lap` at 9.333 s, the chequered state at
/// 12.333 s. Nothing selects on the finish: past it the count is `laps + 1`
/// and the else-branch, between laps, holds the board.
pub const PULSE_LAP_WINDOWS: LapWindows = LapWindows {
    between_laps: (f32::from_bits(0x40c0_0000), f32::from_bits(0x4110_0000)),
    final_lap_ahead: (f32::from_bits(0x4118_0000), f32::from_bits(0x4140_0000)),
    chequered: (f32::from_bits(0x4146_6666), f32::from_bits(0x4154_cccd)),
    finish_is_chequered: false,
};

/// Which of the race manager's windows holds the gantry's time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardWindow {
    /// Before the first line crossing: `GO`, [`super::Clock::window`].
    PreLap,
    /// Any lap but the last two: [`LapWindows::between_laps`].
    BetweenLaps,
    /// The lap before the last: [`LapWindows::final_lap_ahead`].
    FinalLapAhead,
    /// The last lap, and on HD the race once finished:
    /// [`LapWindows::chequered`].
    Chequered,
}

impl BoardWindow {
    /// The window for HD's lap counter `lap` (0 before the first crossing,
    /// then the lap being driven from 1), the race's `total` laps (0 for a
    /// race with no lap count) and whether the craft has finished.
    ///
    /// In the order `0x0005ef7c`-`0x0005f068` tests them: `lap == 0`, then
    /// `lap == total - 1` (a signed compare, so never true for `total == 0`),
    /// then `lap == total` or the crossing count `ship+0x7814 - 2` reaching
    /// `total` (which this build reads as the finish), either of them only
    /// with `total != 0`; anything else is between laps.
    #[must_use]
    pub fn select(lap: u32, total: u32, finished: bool) -> Self {
        if lap == 0 {
            Self::PreLap
        } else if total.checked_sub(1) == Some(lap) {
            Self::FinalLapAhead
        } else if total != 0 && (lap == total || finished) {
            Self::Chequered
        } else {
            Self::BetweenLaps
        }
    }

    /// The window for the player's `standing` in a race of `laps_target`,
    /// under `windows`' selection.
    ///
    /// HD's counter is 0 on the grid and 1 from the first line crossing:
    /// **read live** on RPCS3 (2026-10-04: `ship+0x7810 = 0`, `+0x7814 = 2`
    /// with the craft parked behind the line after the release) and in the
    /// ship constructor (`0x000ddd58` stores both). This build's
    /// [`Standing::lap`] is already 1 there, and the first crossing is the one
    /// that starts its lap clock, so the counter is `lap` once
    /// [`Standing::lap_start_tick`] is set and 0 before. Pulse's crossing count
    /// `craft+0xac8` is the same number: `wraps + past_line`, 0 on the grid
    /// whichever side of the line the craft sits (`race-progress.md`'s seed),
    /// 1 from the first crossing, and still rising past the finish, as
    /// [`Standing::lap`] does.
    #[must_use]
    pub fn of(standing: &Standing, laps_target: Option<u32>, windows: &LapWindows) -> Self {
        let lap = if standing.lap_start_tick.is_some() {
            standing.lap
        } else {
            0
        };
        Self::select(
            lap,
            laps_target.unwrap_or(0),
            windows.finish_is_chequered && standing.finish_tick.is_some(),
        )
    }

    /// The window's `[from, to)` in seconds out of `windows`, or `None` for
    /// [`Self::PreLap`], which [`super::Clock::window`] carries.
    #[must_use]
    pub fn bounds(self, windows: &LapWindows) -> Option<(f32, f32)> {
        match self {
            Self::PreLap => None,
            Self::BetweenLaps => Some(windows.between_laps),
            Self::FinalLapAhead => Some(windows.final_lap_ahead),
            Self::Chequered => Some(windows.chequered),
        }
    }
}

/// A window the board is in and the tick it entered it on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowEntry {
    /// The window.
    pub window: BoardWindow,
    /// The tick the window was entered: the line crossing that changed it.
    pub since: u64,
}

impl WindowEntry {
    /// The window as a [`ReleaseWindow`] from `since` out of `windows`, or
    /// `None` before the first crossing.
    #[must_use]
    pub fn release_window(self, windows: &LapWindows) -> Option<ReleaseWindow> {
        let (from, to) = self.window.bounds(windows)?;
        Some(ReleaseWindow {
            from,
            to,
            from_tick: self.since,
        })
    }
}

/// Remembers when the board entered its current window.
///
/// The original keeps no such tick: it keeps the time itself and checks it
/// every frame. Two laps between the same window's bounds do not reset it
/// (the time is already inside), so the phase is carried from the crossing
/// that *entered* the window, which a lap's own start tick does not give
/// once a second lap shares it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BoardTracker {
    entry: Option<WindowEntry>,
}

impl BoardTracker {
    /// The entry for `window` on `tick`, given the tick of the player's
    /// latest line crossing.
    ///
    /// A new window is entered on `crossed` when that is not later than
    /// `tick`, else on `tick`. A tick before the remembered entry (a rewind)
    /// re-enters the window there.
    pub fn observe(
        &mut self,
        tick: u64,
        window: BoardWindow,
        crossed: Option<u64>,
    ) -> Option<WindowEntry> {
        if window == BoardWindow::PreLap {
            self.entry = None;
            return None;
        }
        match self.entry {
            Some(entry) if entry.window == window && entry.since <= tick => {}
            _ => {
                self.entry = Some(WindowEntry {
                    window,
                    since: crossed.filter(|&at| at <= tick).unwrap_or(tick),
                });
            }
        }
        self.entry
    }
}

/// The draws to leave out at each frame of the board's timeline.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PanelCull {
    /// Hidden before [`Self::from`]: the countdown and the pre-lap window,
    /// the set the loader always removed there (draws beside the panel at
    /// 0 s, and on HD the `FX-350` art).
    pub countdown: Vec<u32>,
    /// The time the per-frame table takes over: the title's
    /// [`LapWindows::between_laps`] start.
    pub from: f32,
    /// The first frame [`Self::frames`] covers.
    pub first_frame: u32,
    /// Hidden at each frame from [`Self::first_frame`]: the draws whose
    /// midpoint is off the panel ([`panel::off_panel`]).
    pub frames: Vec<Vec<u32>>,
}

impl PanelCull {
    /// Builds the table over `windows`' span of `model`'s timeline, for a
    /// panel `2 half_width` wide and `2 half_height` high.
    #[must_use]
    pub fn build(
        model: &Model,
        countdown: Vec<u32>,
        windows: &LapWindows,
        half_width: f32,
        half_height: f32,
    ) -> Self {
        let first_frame = (windows.between_laps.0 * 60.0).floor() as u32;
        let last_frame = (windows.chequered.1 * 60.0).ceil() as u32;
        let frames = (first_frame..=last_frame)
            .map(|frame| panel::off_panel(model, half_width, half_height, frame as f32 / 60.0))
            .collect();
        Self {
            countdown,
            from: windows.between_laps.0,
            first_frame,
            frames,
        }
    }

    /// The draws to leave out with the board's clock at `seconds`.
    #[must_use]
    pub fn hidden(&self, seconds: f32) -> &[u32] {
        if seconds < self.from || self.frames.is_empty() {
            return &self.countdown;
        }
        let frame = (seconds * 60.0).round() as u32;
        let at = frame.saturating_sub(self.first_frame) as usize;
        &self.frames[at.min(self.frames.len() - 1)]
    }
}

#[cfg(test)]
mod tests;
