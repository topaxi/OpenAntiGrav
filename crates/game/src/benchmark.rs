//! The in-game benchmark: one fixed race per title, played in the window with the player's own
//! settings (or a named preset), ending on a stats screen and a result file
//! `scripts/bench-render.py compare` reads (`docs/tools/in-game-benchmark.md`).
//!
//! The race is [`plan`]'s and nothing else's; the numbers are [`stats`]'s, shared with the
//! offscreen whole-race bench (`crate::race_capture`); [`Recorder`] collects the window's frame
//! times, [`report::Report`] turns them into what the screen and the file say, and [`screen`]
//! draws it. None of it reads or writes simulation state: the benchmark is a race flown by the
//! autopilot with a clock beside it.

pub mod plan;
pub mod presets;
pub mod recorder;
pub mod report;
pub mod screen;
pub mod stats;
pub mod text;

pub use recorder::Recorder;
pub use report::{Context, Report};
use screen::Choice;

use std::path::PathBuf;

/// The directory under the user's data directory that results are kept in.
pub const DIRECTORY: &str = "bench";

/// Where results are written: `<data dir>/oag/bench`. `None` where there is no data directory.
#[must_use]
pub fn dir() -> Option<PathBuf> {
    dirs::data_dir().map(|dir| dir.join("oag").join(DIRECTORY))
}

/// The machine's load average (`/proc/loadavg`'s three figures), as the result files record it.
#[must_use]
pub fn load_average() -> String {
    std::fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|text| {
            let mut fields = text.split_whitespace();
            Some(format!(
                "{} {} {}",
                fields.next()?,
                fields.next()?,
                fields.next()?
            ))
        })
        .unwrap_or_else(|| "unavailable".to_string())
}

/// The machine, for the stats screen and the result file: operating system, architecture and
/// processor model (where the OS names one).
#[must_use]
pub fn device_label() -> String {
    let cpu = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|text| {
            text.lines()
                .find(|line| line.starts_with("model name") || line.starts_with("Hardware"))
                .and_then(|line| line.split_once(':'))
                .map(|(_, name)| name.trim().to_string())
        });
    let machine = os_arch(std::env::consts::OS, std::env::consts::ARCH);
    match cpu {
        Some(cpu) => format!("{machine}, {cpu}"),
        None => machine,
    }
}

fn os_arch(os: &str, arch: &str) -> String {
    format!("{os} {arch}").trim_start().to_string()
}

/// A directory name for this run: seconds since the epoch, which sorts and never repeats within
/// a player's sittings.
#[must_use]
pub fn run_id() -> String {
    let seconds = web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    format!("run-{seconds}")
}

/// The short names of the titles, as `title_key` gives them.
pub const TITLE_KEYS: [&str; 5] = ["pulse", "pure", "hd", "2048", "omega"];

/// The short name `scripts/bench-render.py` files a title's result under (`pulse`, `pure`, `hd`,
/// `2048`, `omega`), so two runs' directories line up by file name.
#[must_use]
pub fn title_key(title_name: &str) -> String {
    let lower = title_name.to_ascii_lowercase();
    let rest = lower.strip_prefix("wipeout").unwrap_or(&lower);
    rest.split(|c: char| !c.is_ascii_alphanumeric())
        .find(|word| !word.is_empty())
        .unwrap_or("title")
        .to_string()
}

/// What one benchmark run is asked for: the command line's flags, or the BENCHMARK OPTIONS page's
/// rows. Never saved: a run applies it in memory and puts the player's settings back.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Request {
    /// The preset to draw with; `None` is the player's own settings.
    pub preset: Option<String>,
    /// Whether the player's mods load with the run. Off by default, so two machines' results
    /// are one workload.
    pub mods: bool,
    /// The vsync mode for the run; `None` keeps the player's.
    pub vsync: Option<oag_present::perf::Vsync>,
    /// The frame limit for the run; `None` keeps the player's.
    pub frame_limit: Option<oag_present::perf::FrameLimit>,
}

/// The name the track select gives the circuit a race loaded, out of its rows (`tracks`):
/// `None` where no row addresses `circuit`.
#[must_use]
pub fn circuit_name(
    tracks: &[(oag_raceplay::catalogue::Track, String)],
    circuit: &str,
) -> Option<String> {
    tracks
        .iter()
        .find(|(track, _)| track.addresses(circuit))
        .map(|(_, name)| name.clone())
}

/// A mod that loaded with a run, as the result file names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModLoaded {
    /// The mod's id.
    pub id: String,
    /// The mod's release.
    pub version: String,
    /// SHA-256 of its simulation module; empty for a mod that has none.
    pub hash: String,
}

/// Where the benchmark was started from, and so where the stats screen's BACK leaves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// The command line: BACK quits.
    Cli,
    /// The menus' BENCHMARK row: BACK reopens the menus.
    Extras,
    /// The graphics page's RUN BENCHMARK row: BACK reopens that page.
    Graphics,
}

impl Origin {
    /// The spelling the result file records.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Cli => "cli",
            Self::Extras => "extras",
            Self::Graphics => "graphics",
        }
    }
}

/// A finished run, on its stats screen.
#[derive(Debug)]
pub struct Done {
    /// The numbers.
    pub report: Report,
    /// The button the pad's confirm presses.
    pub focus: Choice,
    /// Where the result file went.
    pub saved: Option<String>,
    /// The project's strings in the player's language, which the screen draws its words from.
    pub strings: oag_ui::language::StringTable,
    /// What the screen shows besides the report.
    pub view: screen::View,
    /// What the COPY button says.
    pub(crate) copy: CopyLabel,
}

/// Ticks a copy's answer stays on the button: two seconds.
pub const COPIED_TICKS: u32 = 120;

/// The COPY button's label: its own name until the platform says how a write went, then that
/// answer for [`COPIED_TICKS`].
#[derive(Debug, Default)]
pub(crate) struct CopyLabel {
    waiting: Option<crate::app::clipboard::Outcome>,
    shown: Option<bool>,
    ticks: u32,
}

impl CopyLabel {
    /// A write was asked for: the last answer comes off the button.
    pub(crate) fn start(&mut self, outcome: crate::app::clipboard::Outcome) {
        *self = Self {
            waiting: Some(outcome),
            ..Self::default()
        };
    }

    /// One tick, and what the button says now: `Some(true)` copied, `Some(false)` refused.
    pub(crate) fn tick(&mut self) -> Option<bool> {
        if let Some(took) = self.waiting.as_ref().and_then(|outcome| outcome.get()) {
            self.waiting = None;
            self.shown = Some(took);
            self.ticks = COPIED_TICKS;
        } else if self.ticks > 0 {
            self.ticks -= 1;
            if self.ticks == 0 {
                self.shown = None;
            }
        }
        self.shown
    }
}

/// A benchmark race, from its first frame to its stats screen. Lives on the race stage, so it
/// ends with the race.
#[derive(Debug)]
pub struct Live {
    /// Where it was started from.
    pub origin: Origin,
    /// What it was asked for: the preset, the mods, the vsync and the frame limit.
    pub request: Request,
    /// The tick the race is stopped at when nobody has crossed the line.
    pub tick_cap: u32,
    /// The frames so far.
    pub recorder: Recorder,
    /// The stats screen, once the race has ended.
    pub done: Option<Done>,
}

#[cfg(test)]
mod tests {
    use super::{COPIED_TICKS, CopyLabel, os_arch, title_key};
    use crate::app::clipboard::Outcome;

    /// `wasm32` has no operating system name, so the device line starts at the architecture.
    #[test]
    fn a_device_with_no_operating_system_name_has_no_leading_space() {
        assert_eq!(os_arch("", "wasm32"), "wasm32");
        assert_eq!(os_arch("linux", "x86_64"), "linux x86_64");
    }

    /// The browser answers a write later: until then the button claims nothing, and a refusal
    /// reads as one.
    #[test]
    fn the_copy_button_claims_nothing_until_the_write_is_answered() {
        for took in [false, true] {
            let outcome = Outcome::default();
            let mut label = CopyLabel::default();
            label.start(outcome.clone());
            for _ in 0..2 * COPIED_TICKS {
                assert_eq!(label.tick(), None);
            }
            outcome.settle(took);
            assert_eq!(label.tick(), Some(took));
            for _ in 1..COPIED_TICKS {
                assert_eq!(label.tick(), Some(took));
            }
            assert_eq!(label.tick(), None);
        }
    }

    #[test]
    fn a_second_press_takes_the_last_answer_off_the_button() {
        let mut label = CopyLabel::default();
        label.start(Outcome::known(true));
        assert_eq!(label.tick(), Some(true));
        label.start(Outcome::default());
        assert_eq!(label.tick(), None);
    }

    #[test]
    fn a_title_is_filed_under_the_name_the_offscreen_bench_uses() {
        for (name, key) in [
            ("Wipeout Pulse", "pulse"),
            ("Wipeout Pure", "pure"),
            ("Wipeout HD", "hd"),
            ("Wipeout 2048", "2048"),
            ("Wipeout: Omega Collection", "omega"),
        ] {
            assert_eq!(title_key(name), key);
        }
    }
}
