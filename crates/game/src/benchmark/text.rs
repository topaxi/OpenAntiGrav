//! What the stats screen and the copied text say, in plain words: one place, so the two cannot
//! disagree and every line goes through a string id (`assets/ui/strings/english.toml`).
//!
//! A run reads as a verdict, a few plain lines and a DETAILS block. The verdict comes from the
//! share of counted frames that missed the frame budget the player asked for
//! ([`super::report::Context::budget`]): up to [`super::report::SMOOTH_SHARE`] is SMOOTH, up to
//! [`super::report::MOSTLY_SMOOTH_SHARE`] MOSTLY SMOOTH, more CHOPPY
//! (`docs/tools/in-game-benchmark.md`, "Reading the screen").

use super::report::{Figures, LOW_01_FRAMES, Report, Verdict};

/// How a string id is looked up: its id and the English it falls back to.
pub type Say<'a> = &'a dyn Fn(&str, &str) -> String;

/// A string id and its English text, which `english.toml` holds too (a test keeps them equal).
pub type Word = (&'static str, &'static str);

/// Every id this module and the stats screen draw.
pub mod words {
    use super::Word;

    pub const COMPLETE: Word = ("OAG_BENCH_COMPLETE", "BENCHMARK COMPLETE");
    pub const PRESET_NAMED: Word = ("OAG_BENCH_PRESET_NAMED", "PRESET %s");
    pub const SETTINGS: Word = ("OAG_BENCH_PRESET_SETTINGS", "YOUR SETTINGS");
    pub const SMOOTH: Word = ("OAG_BENCH_VERDICT_SMOOTH", "SMOOTH");
    pub const MOSTLY_SMOOTH: Word = ("OAG_BENCH_VERDICT_MOSTLY_SMOOTH", "MOSTLY SMOOTH");
    pub const CHOPPY: Word = ("OAG_BENCH_VERDICT_CHOPPY", "CHOPPY");
    pub const VERDICT: Word = (
        "OAG_BENCH_VERDICT",
        "%s: %s FPS ON AVERAGE, JUDGED AGAINST %s FPS",
    );
    pub const SLOWEST: Word = (
        "OAG_BENCH_SLOWEST",
        "Slowest moments: %s FPS (worst 1 in 100 frames)",
    );
    pub const STUTTERS: Word = (
        "OAG_BENCH_STUTTERS",
        "Stutters: %s of %s frames missed the %s ms frame budget",
    );
    pub const CAPPED: Word = (
        "OAG_BENCH_CAPPED",
        "Your frame limit (%s FPS) capped this result. Turn FRAME LIMIT off in BENCHMARK OPTIONS to see how fast your hardware can go.",
    );
    pub const MODS_NONE: Word = ("OAG_BENCH_MODS_NONE", "Mods: none");
    pub const MODS_LOADED: Word = ("OAG_BENCH_MODS_LOADED", "Mods: %s loaded");
    pub const TOO_SHORT: Word = (
        "OAG_BENCH_TOO_SHORT",
        "The race ended before enough frames were drawn",
    );
    pub const DETAILS: Word = ("OAG_BENCH_DETAILS", "DETAILS");
    pub const SUMMARY: Word = ("OAG_BENCH_SUMMARY", "SUMMARY");
    pub const COPY: Word = ("OAG_BENCH_COPY", "COPY TO CLIPBOARD");
    pub const COPIED: Word = ("OAG_BENCH_COPIED", "COPIED");
    pub const COPY_FAILED: Word = ("OAG_BENCH_COPY_FAILED", "COULD NOT COPY");
    pub const AGAIN: Word = ("OAG_BENCH_AGAIN", "RUN AGAIN");
    pub const BACK: Word = ("OAG_MENU_BACK", "BACK");
    pub const SAVED: Word = ("OAG_BENCH_SAVED", "SAVED %s");
    pub const AVERAGE: Word = ("OAG_BENCH_D_AVERAGE", "AVERAGE FPS");
    pub const LOW_1: Word = ("OAG_BENCH_D_LOW_1", "1% LOW FPS");
    pub const LOW_01: Word = ("OAG_BENCH_D_LOW_01", "0.1% LOW FPS");
    pub const LOW_01_NA: Word = ("OAG_BENCH_D_LOW_01_NA", "N/A UNDER %s FRAMES");
    pub const MEDIAN: Word = ("OAG_BENCH_D_MEDIAN", "FRAME TIME MEDIAN");
    pub const P95: Word = ("OAG_BENCH_D_P95", "FRAME TIME P95");
    pub const P99: Word = ("OAG_BENCH_D_P99", "FRAME TIME P99");
    pub const WORST: Word = ("OAG_BENCH_D_WORST", "FRAME TIME WORST");
    pub const OVER: Word = ("OAG_BENCH_D_OVER", "FRAMES OVER %s MS");
    pub const OVER_OF: Word = ("OAG_BENCH_D_OVER_OF", "%s OF %s");
    pub const REFRESH: Word = ("OAG_BENCH_D_REFRESH", "SCREEN REFRESH");
    pub const REFRESH_DISPLAY: Word = ("OAG_BENCH_D_REFRESH_DISPLAY", "%s HZ");
    pub const REFRESH_ASSUMED: Word = ("OAG_BENCH_D_REFRESH_ASSUMED", "%s HZ (ASSUMED)");
    pub const MODS: Word = ("OAG_BENCH_D_MODS", "MODS");
    pub const MODS_OFF: Word = ("OAG_BENCH_D_MODS_OFF", "OFF");
    pub const DEVICE: Word = ("OAG_BENCH_D_DEVICE", "DEVICE");
    pub const GPU: Word = ("OAG_BENCH_D_GPU", "GPU");
    pub const BUILD: Word = ("OAG_BENCH_D_BUILD", "BUILD");
    pub const TITLE: Word = ("OAG_BENCH_D_TITLE", "GAME");
    pub const CIRCUIT: Word = ("OAG_BENCH_D_CIRCUIT", "CIRCUIT");
    pub const PRESET: Word = ("OAG_BENCH_PRESET", "PRESET");

    /// Every word, for the test that holds them to `english.toml`.
    pub const ALL: &[Word] = &[
        COMPLETE,
        PRESET_NAMED,
        SETTINGS,
        SMOOTH,
        MOSTLY_SMOOTH,
        CHOPPY,
        VERDICT,
        SLOWEST,
        STUTTERS,
        CAPPED,
        MODS_NONE,
        MODS_LOADED,
        TOO_SHORT,
        DETAILS,
        SUMMARY,
        COPY,
        COPIED,
        COPY_FAILED,
        AGAIN,
        BACK,
        SAVED,
        AVERAGE,
        LOW_1,
        LOW_01,
        LOW_01_NA,
        MEDIAN,
        P95,
        P99,
        WORST,
        OVER,
        OVER_OF,
        REFRESH,
        REFRESH_DISPLAY,
        REFRESH_ASSUMED,
        MODS,
        MODS_OFF,
        DEVICE,
        GPU,
        BUILD,
        TITLE,
        CIRCUIT,
        PRESET,
    ];
}

/// `word` in the player's language, each `%s` filled by the next of `args`.
#[must_use]
pub fn say(say: Say<'_>, word: Word, args: &[&str]) -> String {
    let mut out = say(word.0, word.1);
    for arg in args {
        match out.find("%s") {
            Some(at) => out.replace_range(at..at + 2, arg),
            None => break,
        }
    }
    out
}

/// The circuit's name as the player reads it, or nothing: never the archive's.
fn circuit(report: &Report) -> Option<&str> {
    report.context.track_name.as_deref()
}

/// The preset the run drew with, or YOUR SETTINGS.
fn preset(report: &Report, s: Say<'_>) -> String {
    report.context.preset.as_deref().map_or_else(
        || say(s, words::SETTINGS, &[]),
        |name| say(s, words::PRESET_NAMED, &[&name.to_uppercase()]),
    )
}

/// The line under the heading: the game, the circuit and the preset.
#[must_use]
pub fn head(report: &Report, s: Say<'_>) -> String {
    let mut parts = vec![report.context.title.to_uppercase()];
    parts.extend(circuit(report).map(str::to_uppercase));
    parts.push(preset(report, s));
    parts.join("  ")
}

/// The word for a verdict.
#[must_use]
pub fn verdict_word(verdict: Verdict, s: Say<'_>) -> String {
    say(
        s,
        match verdict {
            Verdict::Smooth => words::SMOOTH,
            Verdict::MostlySmooth => words::MOSTLY_SMOOTH,
            Verdict::Choppy => words::CHOPPY,
        },
        &[],
    )
}

/// Whole frames per second, as a player reads them.
fn fps(value: f64) -> String {
    format!("{value:.0}")
}

/// A refresh rate without a trailing `.0`.
fn hz(value: f64) -> String {
    if (value - value.round()).abs() < 0.05 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

/// The frame budget in milliseconds, to a tenth: the stutter line and the FRAMES OVER row.
fn tenth_ms(value: f64) -> String {
    format!("{value:.1}")
}

/// The mods line of the summary.
fn mods_line(report: &Report, s: Say<'_>) -> String {
    match &report.context.mods {
        None => say(s, words::MODS_NONE, &[]),
        Some(mods) => say(s, words::MODS_LOADED, &[&mods.len().to_string()]),
    }
}

/// The verdict first, then the plain lines. The first is the verdict; the rest are in order the
/// slowest moments, the stutters, the frame-limit line when the limit binds, the mods line.
#[must_use]
pub fn summary(report: &Report, s: Say<'_>) -> Vec<String> {
    let Some(figures) = report.figures() else {
        return vec![say(s, words::TOO_SHORT, &[])];
    };
    let mut lines = vec![say(
        s,
        words::VERDICT,
        &[
            &verdict_word(figures.verdict(), s),
            &fps(figures.average_fps),
            &hz(report.context.budget().hz),
        ],
    )];
    lines.push(say(s, words::SLOWEST, &[&fps(figures.low_1)]));
    lines.push(say(
        s,
        words::STUTTERS,
        &[
            &figures.over_budget.to_string(),
            &figures.counted.to_string(),
            &tenth_ms(figures.budget_ms),
        ],
    ));
    if let Some(limit) = figures.limited_by {
        lines.push(say(s, words::CAPPED, &[&fps(limit)]));
    }
    lines.push(mods_line(report, s));
    lines
}

/// The DETAILS block as `(label, value)` rows: the numbers and the settings, which are short, and
/// the mods and the machine, whose values run long.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Details {
    /// The numbers, then the settings under the menu's own row names.
    pub rows: Vec<(String, String)>,
    /// The mods, the device, the GPU with its driver and the build.
    pub machine: Vec<(String, String)>,
}

impl Details {
    /// Every row, in the order the copied text lists them.
    pub fn all(&self) -> impl Iterator<Item = &(String, String)> {
        self.rows.iter().chain(&self.machine)
    }
}

/// The numbers, the settings with the menu's own names, and the machine.
#[must_use]
pub fn details(report: &Report, s: Say<'_>) -> Details {
    let c = &report.context;
    let mut rows = Vec::new();
    if let Some(Figures {
        counted,
        average_fps,
        low_1,
        low_01,
        median_ms,
        p95_ms,
        p99_ms,
        worst_ms,
        over_budget,
        budget_ms,
        ..
    }) = report.figures()
    {
        let ms = |value: f64| format!("{value:.2} MS");
        rows.push((say(s, words::AVERAGE, &[]), format!("{average_fps:.1}")));
        rows.push((say(s, words::LOW_1, &[]), format!("{low_1:.1}")));
        rows.push((
            say(s, words::LOW_01, &[]),
            low_01.map_or_else(
                || say(s, words::LOW_01_NA, &[&LOW_01_FRAMES.to_string()]),
                |low| format!("{low:.1}"),
            ),
        ));
        rows.push((say(s, words::MEDIAN, &[]), ms(median_ms)));
        rows.push((say(s, words::P95, &[]), ms(p95_ms)));
        rows.push((say(s, words::P99, &[]), ms(p99_ms)));
        rows.push((say(s, words::WORST, &[]), ms(worst_ms)));
        rows.push((
            say(s, words::OVER, &[&tenth_ms(budget_ms)]),
            say(
                s,
                words::OVER_OF,
                &[&over_budget.to_string(), &counted.to_string()],
            ),
        ));
    }
    rows.push((
        say(s, words::REFRESH, &[]),
        say(
            s,
            if c.refresh.1 == "display" {
                words::REFRESH_DISPLAY
            } else {
                words::REFRESH_ASSUMED
            },
            &[&hz(c.refresh.0)],
        ),
    ));
    rows.extend(c.details.iter().cloned());
    let mut machine = vec![(
        say(s, words::MODS, &[]),
        match &c.mods {
            None => say(s, words::MODS_OFF, &[]),
            Some(mods) => mods
                .iter()
                .map(|m| format!("{} {}", m.id, m.version))
                .collect::<Vec<_>>()
                .join(", "),
        },
    )];
    machine.push((say(s, words::DEVICE, &[]), c.device.clone()));
    if let Some(gpu) = gpu_line(&c.adapter_name, &c.driver) {
        machine.push((say(s, words::GPU, &[]), gpu));
    }
    machine.push((say(s, words::BUILD, &[]), c.build_version.to_string()));
    Details { rows, machine }
}

/// The adapter with its driver in brackets, as far as the platform names them: a browser names
/// neither, and some drivers only repeat the adapter's name.
fn gpu_line(adapter: &str, driver: &str) -> Option<String> {
    match (adapter, driver) {
        ("", "") => None,
        ("", only) | (only, "") => Some(only.to_string()),
        (adapter, driver) if adapter == driver => Some(adapter.to_string()),
        (adapter, driver) => Some(format!("{adapter} ({driver})")),
    }
}

/// The text COPY TO CLIPBOARD puts on the clipboard: what a player pastes into a forum post or an
/// issue. The same order as the screen: the plain lines first, then a DETAILS block.
#[must_use]
pub fn copy_text(report: &Report, s: Say<'_>) -> String {
    let mut out = String::new();
    let mut put = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };
    let mut head = vec![report.context.title.to_string()];
    head.extend(circuit(report).map(str::to_string));
    head.push(preset(report, s));
    put(head.join(", "));
    for line in summary(report, s) {
        put(line);
    }
    put(String::new());
    put(say(s, words::DETAILS, &[]));
    for (label, value) in details(report, s).all() {
        put(format!("{label}: {value}"));
    }
    out
}

#[cfg(test)]
mod tests;
