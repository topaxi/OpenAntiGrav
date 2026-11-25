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
    let cpu = std::fs::read_to_string("/proc/cpuinfo").ok().and_then(|text| {
        text.lines()
            .find(|line| line.starts_with("model name") || line.starts_with("Hardware"))
            .and_then(|line| line.split_once(':'))
            .map(|(_, name)| name.trim().to_string())
    });
    match cpu {
        Some(cpu) => format!("{} {}, {cpu}", std::env::consts::OS, std::env::consts::ARCH),
        None => format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
    }
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

/// What a command line asks the benchmark for.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Request {
    /// The preset to draw with; `None` is the player's own settings.
    pub preset: Option<String>,
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
}

/// A benchmark race, from its first frame to its stats screen. Lives on the race stage, so it
/// ends with the race.
#[derive(Debug)]
pub struct Live {
    /// Where it was started from.
    pub origin: Origin,
    /// The preset it draws with.
    pub preset: Option<String>,
    /// The tick the race is stopped at when nobody has crossed the line.
    pub tick_cap: u32,
    /// The frames so far.
    pub recorder: Recorder,
    /// The stats screen, once the race has ended.
    pub done: Option<Done>,
}

#[cfg(test)]
mod tests {
    use super::title_key;

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
