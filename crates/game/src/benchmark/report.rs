//! What a finished benchmark says: the numbers the stats screen shows, and the result file in
//! `scripts/bench-render.py`'s JSON format.
//!
//! The summaries and the JSON primitives are [`super::stats`]'s, the ones the offscreen
//! whole-race bench writes with, so `just bench-compare` reads both. A window's frame is the
//! interval between two presents, so only `frame_us` is timed: the CPU and GPU columns of the
//! offscreen bench stay absent (the comparison says "not timed in one of the runs"), and
//! `run.kind` names the difference so a result drawn through a window is never mistaken for an
//! offscreen one.

use std::fmt::Write as _;

use super::plan::COUNTDOWN_TICKS;
use super::recorder::{Ending, Frame};
use super::stats::{Summary, json_string, low_fps, summarise};

/// A frame is over the frame budget ([`Context::budget`]) when it took more than this many times
/// the budget. A loop held by vsync, the frame limit or dynamic resolution lands within a few
/// percent of the period on every frame, so the literal period would count half of them.
pub const OVER_BUDGET_SLACK: f64 = 1.05;

/// What set the frame budget a run is judged against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetFrom {
    /// TARGET FPS.
    TargetFps,
    /// FRAME LIMIT.
    FrameLimit,
    /// The screen's refresh rate, reported, measured or assumed.
    Refresh,
}

impl BudgetFrom {
    /// The name the result file uses.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::TargetFps => "target_fps",
            Self::FrameLimit => "frame_limit",
            Self::Refresh => "refresh",
        }
    }
}

/// The frame rate a run is judged against.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Budget {
    /// Frames a second.
    pub hz: f64,
    /// Which setting, or the screen, set it.
    pub from: BudgetFrom,
}

/// The adapter, as the offscreen bench's result file names it.
#[must_use]
pub fn adapter_json(adapter: &wgpu::AdapterInfo, gpu_timestamps: bool) -> String {
    format!(
        "{{\"name\": {}, \"type\": {}, \"backend\": {}, \"driver\": {}, \"driver_info\": {}, \"gpu_timestamps\": {gpu_timestamps}}}",
        json_string(&adapter.name),
        json_string(&format!("{:?}", adapter.device_type)),
        json_string(&format!("{:?}", adapter.backend)),
        json_string(&adapter.driver),
        json_string(&adapter.driver_info),
    )
}

/// The `mods` field of the result: `"off"`, or the loaded mods as `[{id, version, hash}]`.
#[must_use]
pub fn mods_json(mods: Option<&[super::ModLoaded]>) -> String {
    let Some(mods) = mods else {
        return json_string("off");
    };
    let list = mods
        .iter()
        .map(|m| {
            format!(
                "{{\"id\": {}, \"version\": {}, \"hash\": {}}}",
                json_string(&m.id),
                json_string(&m.version),
                json_string(&m.hash)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{list}]")
}

/// How this binary was built, as the result files say it.
#[must_use]
pub fn build_label() -> String {
    format!(
        "{}, perf-probe {}",
        if cfg!(debug_assertions) {
            "debug assertions on"
        } else {
            "release"
        },
        cfg!(feature = "perf-probe")
    )
}

/// Everything about the run that is not a frame time.
#[derive(Debug, Clone)]
pub struct Context {
    /// The title's name.
    pub title: &'static str,
    /// The circuit the loader chose.
    pub track: Option<String>,
    /// The mode's name.
    pub mode: &'static str,
    /// Laps the race ran, where it has a count.
    pub laps_target: Option<u32>,
    /// The adapter's description, already JSON (see [`adapter_json`]).
    pub adapter: String,
    /// The adapter's name, for the screen.
    pub adapter_name: String,
    /// The driver, for the screen.
    pub driver: String,
    /// The machine: operating system, architecture and processor.
    pub device: String,
    /// The settings that change what a frame costs, in the order the screen lists them.
    pub settings: Vec<(String, String)>,
    /// The same settings with the menu's own row names and values, for the screen and the copied
    /// text.
    pub details: Vec<(String, String)>,
    /// The circuit as the track select names it; `None` where the run knows no such name.
    pub track_name: Option<String>,
    /// The build's version string.
    pub build_version: &'static str,
    /// The screen's refresh rate, and what it came from.
    pub refresh: (f64, &'static str),
    /// The rate the frame limiter holds the loop to, where one is in force.
    pub frame_limit_hz: Option<f64>,
    /// The rate TARGET FPS names: `None` where it is off, or `original`, which names none of its
    /// own and paces at the limit or the screen.
    pub target_fps_hz: Option<f64>,
    /// The preset the run drew with; `None` is the player's own settings.
    pub preset: Option<String>,
    /// The mods that loaded with the run; `None` is a run without any.
    pub mods: Option<Vec<super::ModLoaded>>,
    /// The vsync mode the run asked for (`off`, `on`, `smooth`).
    pub vsync: String,
    /// The present mode the surface was configured with.
    pub present_mode: String,
    /// The frame limit the run was configured with (`unlimited`, `240`, `refresh_rate`).
    pub frame_limit: String,
    /// Where the benchmark was started from: `cli`, `extras` or `graphics`.
    pub origin: &'static str,
    /// The flags that start the same benchmark again.
    pub command: Vec<String>,
    /// The machine's load average at the start and the end of the run.
    pub load: (String, String),
    /// `GIT_HASH` of the build.
    pub git_hash: &'static str,
    /// Seconds the run took.
    pub wall_seconds: f64,
    /// The run's people count.
    pub people: u8,
    /// The tick the race would have been stopped at.
    pub tick_cap: u32,
}

impl Context {
    /// The frame budget the player asked for: the longest period (lowest rate) of TARGET FPS,
    /// the frame limit and the screen's refresh. A setting that is off or unlimited names no
    /// period, so the screen alone judges a run with neither. A screen whose rate is only
    /// assumed still counts: judging a browser held to a 60 Hz screen against a higher target
    /// would call every frame late. Equal rates are named in that order.
    #[must_use]
    pub fn budget(&self) -> Budget {
        let screen = Budget {
            hz: self.refresh.0,
            from: BudgetFrom::Refresh,
        };
        [
            (self.target_fps_hz, BudgetFrom::TargetFps),
            (self.frame_limit_hz, BudgetFrom::FrameLimit),
        ]
        .into_iter()
        .filter_map(|(hz, from)| hz.map(|hz| Budget { hz, from }))
        .chain([screen])
        // The first of equal rates.
        .min_by(|a, b| a.hz.total_cmp(&b.hz))
        .unwrap_or(screen)
    }
}

/// The numbers of one run.
#[derive(Debug, Clone)]
pub struct Report {
    /// What is not a frame time.
    pub context: Context,
    /// Every frame, the countdown's included.
    pub frames: Vec<Frame>,
    /// Where the race ended.
    pub ending: Ending,
}

/// The figures the screen shows, over the frames after the start-line countdown.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Figures {
    /// Frames counted.
    pub counted: usize,
    /// Frames per second over the whole counted run.
    pub average_fps: f64,
    /// The rate of the slowest 1 % of frames.
    pub low_1: f64,
    /// The rate of the slowest 0.1 % of frames; `None` under [`LOW_01_FRAMES`] counted frames,
    /// where it would be the one worst frame.
    pub low_01: Option<f64>,
    /// Median, 95th and 99th percentile and worst frame time, in milliseconds.
    pub median_ms: f64,
    /// 95th percentile, milliseconds.
    pub p95_ms: f64,
    /// 99th percentile, milliseconds.
    pub p99_ms: f64,
    /// The slowest frame, milliseconds.
    pub worst_ms: f64,
    /// Frames over the frame budget (see [`OVER_BUDGET_SLACK`]).
    pub over_budget: usize,
    /// The frame budget, milliseconds (see [`Context::budget`]).
    pub budget_ms: f64,
    /// The frame limit, when the average sits on it: the figure is then the limiter's, not
    /// the machine's.
    pub limited_by: Option<f64>,
}

/// How a run reads to a player, from the share of counted frames that missed the frame budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// At most [`SMOOTH_SHARE`] of the frames missed it.
    Smooth,
    /// At most [`MOSTLY_SMOOTH_SHARE`] did.
    MostlySmooth,
    /// More than that.
    Choppy,
}

/// The largest share of late frames a SMOOTH run has: 1 %, at 60 FPS one late frame every 1.7
/// seconds.
pub const SMOOTH_SHARE: f64 = 0.01;

/// The largest share a MOSTLY SMOOTH run has: 5 %, at 60 FPS three a second, past which a
/// player sees a stutter as the rule rather than the exception.
pub const MOSTLY_SMOOTH_SHARE: f64 = 0.05;

/// Counted frames below which the 0.1 % low is only the worst frame.
pub const LOW_01_FRAMES: usize = 1000;

/// An average this close under the frame limit (or over it) is the limiter's.
const LIMITER_BINDS: f64 = 0.9;

impl Figures {
    /// The share of counted frames that missed the frame budget.
    #[must_use]
    pub fn stutter_share(&self) -> f64 {
        self.over_budget as f64 / self.counted.max(1) as f64
    }

    /// How the run reads: see [`Verdict`].
    #[must_use]
    pub fn verdict(&self) -> Verdict {
        let share = self.stutter_share();
        if share <= SMOOTH_SHARE {
            Verdict::Smooth
        } else if share <= MOSTLY_SMOOTH_SHARE {
            Verdict::MostlySmooth
        } else {
            Verdict::Choppy
        }
    }
}

impl Report {
    /// The frames after the countdown: the craft is released and the race is what is drawn.
    fn raced(&self) -> impl Iterator<Item = &Frame> {
        self.frames
            .iter()
            .filter(|frame| frame.tick >= COUNTDOWN_TICKS)
    }

    fn counted(&self) -> Vec<f64> {
        self.raced().map(|frame| frame.frame_us).collect()
    }

    /// The summary of the counted frame times, in microseconds.
    #[must_use]
    pub fn summary(&self) -> Option<Summary> {
        summarise(&self.counted())
    }

    /// The screen's figures; `None` when no frame was counted (a run shorter than the warm-up).
    #[must_use]
    pub fn figures(&self) -> Option<Figures> {
        let counted = self.counted();
        let summary = summarise(&counted)?;
        let budget_us = 1e6 / self.context.budget().hz;
        let average_fps = 1e6 / summary.mean;
        Some(Figures {
            counted: summary.count,
            average_fps,
            low_1: low_fps(&counted, 1.0)?,
            low_01: if summary.count >= LOW_01_FRAMES {
                low_fps(&counted, 0.1)
            } else {
                None
            },
            median_ms: summary.p50 / 1000.0,
            p95_ms: summary.p95 / 1000.0,
            p99_ms: summary.p99 / 1000.0,
            worst_ms: summary.max / 1000.0,
            over_budget: counted
                .iter()
                .filter(|us| **us > budget_us * OVER_BUDGET_SLACK)
                .count(),
            budget_ms: budget_us / 1000.0,
            limited_by: self
                .context
                .frame_limit_hz
                .filter(|limit| average_fps >= limit * LIMITER_BINDS),
        })
    }

    /// `frame,lap,ticks,frame_us` for every frame.
    #[must_use]
    pub fn csv(&self) -> String {
        let mut out = String::from("frame,lap,ticks,frame_us\n");
        for (index, frame) in self.frames.iter().enumerate() {
            let _ = writeln!(
                out,
                "{index},{},{},{:.1}",
                frame.lap, frame.ticks, frame.frame_us
            );
        }
        out
    }

    /// The result file, in the format `scripts/bench-render.py compare` reads.
    #[must_use]
    pub fn json(&self) -> String {
        let c = &self.context;
        let mut out = String::from("{\n");
        let mut put = |key: &str, value: String| {
            let _ = writeln!(out, "  {}: {value},", json_string(key));
        };
        put("format", "1".to_string());
        put("title", json_string(c.title));
        put("git_hash", json_string(c.git_hash));
        put("build", json_string(&build_label()));
        put("adapter", c.adapter.clone());
        put(
            "settings",
            format!(
                "{{{}}}",
                c.settings
                    .iter()
                    .map(|(k, v)| format!("{}: {}", json_string(k), json_string(v)))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
        put("mode", json_string(c.mode));
        put(
            "laps_target",
            c.laps_target
                .map_or_else(|| "null".to_string(), |n| n.to_string()),
        );
        put("people", c.people.to_string());
        put("tick_cap", c.tick_cap.to_string());
        put("finished", self.ending.finished.to_string());
        put("final_tick", self.ending.tick.to_string());
        put(
            "final_state_hash",
            json_string(&format!("{:016x}", self.ending.state_hash)),
        );
        put("flyby_ticks", self.ending.flyby_ticks.to_string());
        put("frames", self.frames.len().to_string());
        put(
            "warmup_frames_excluded",
            (self.frames.len() - self.raced().count()).to_string(),
        );
        put("countdown_ticks_excluded", COUNTDOWN_TICKS.to_string());
        put("wall_seconds", format!("{:.3}", c.wall_seconds));
        put("load_average_start", json_string(&c.load.0));
        put("load_average_end", json_string(&c.load.1));
        put(
            "run",
            format!(
                "{{\"kind\": \"in_game\", \"title_key\": {}, \"track\": {}, \"case\": \"1\", \"preset\": {}, \"origin\": {}, \"mods\": {}, \"vsync_requested\": {}, \"present_mode\": {}, \"frame_limit\": {}, \"command\": [{}]}}",
                json_string(c.title),
                c.track
                    .as_deref()
                    .map_or_else(|| "null".to_string(), json_string),
                c.preset
                    .as_deref()
                    .map_or_else(|| "null".to_string(), json_string),
                json_string(c.origin),
                mods_json(c.mods.as_deref()),
                json_string(&c.vsync),
                json_string(&c.present_mode),
                json_string(&c.frame_limit),
                c.command
                    .iter()
                    .map(|flag| json_string(flag))
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
        );
        if let Some(f) = self.figures() {
            put(
                "in_game",
                format!(
                    "{{\"average_fps\": {:.2}, \"low_1pct_fps\": {:.2}, \"low_0_1pct_fps\": {}, \"frames_over_budget\": {}, \"budget_ms\": {:.3}, \"budget_from\": {}, \"refresh_hz\": {:.2}, \"refresh_source\": {}, \"frame_limit_hz\": {}, \"limiter_binds\": {}, \"device\": {}, \"driver\": {}}}",
                    f.average_fps,
                    f.low_1,
                    f.low_01
                        .map_or_else(|| "null".to_string(), |low| format!("{low:.2}")),
                    f.over_budget,
                    f.budget_ms,
                    json_string(c.budget().from.name()),
                    c.refresh.0,
                    json_string(c.refresh.1),
                    c.frame_limit_hz
                        .map_or_else(|| "null".to_string(), |hz| format!("{hz:.0}")),
                    f.limited_by.is_some(),
                    json_string(&c.device),
                    json_string(&c.driver),
                ),
            );
        }
        let metric = self
            .summary()
            .as_ref()
            .map_or_else(|| "null".to_string(), Summary::json);
        put("metrics", format!("{{\n    \"frame_us\": {metric}\n  }}"));
        let last_lap = self.frames.iter().map(|f| f.lap).max().unwrap_or(0);
        let laps = (1..=last_lap)
            .map(|lap| {
                let of: Vec<f64> = self
                    .raced()
                    .filter(|f| f.lap == lap)
                    .map(|f| f.frame_us)
                    .collect();
                let line = summarise(&of)
                    .as_ref()
                    .map_or_else(|| "null".to_string(), Summary::json);
                format!("    {{\"lap\": {lap}, \"frame_us\": {line}}}")
            })
            .collect::<Vec<_>>()
            .join(",\n");
        let _ = write!(out, "  \"per_lap\": [\n{laps}\n  ]\n}}\n");
        out
    }
}

#[cfg(test)]
pub(crate) mod tests;
