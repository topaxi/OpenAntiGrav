//! The attract demo's flow as plain values: when an untouched main menu starts one, what ends it,
//! and which race it is.
//!
//! The timings and the exit rule are the title's (`oag_title::demo::Demo`); this module holds no
//! number of its own. It knows no window and no stage, so a test drives it a tick at a time, and
//! the windowed session (`session::demo`) and the headless `--demo` run both go through it
//! (`docs/ui/attract-demo.md`).

pub mod chain;

use oag_core::Rng;
use oag_tables::demo_config::DemoConfig;
use oag_title::demo::{AfterEnd, Demo, Selection};

/// The main menu's untouched time.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Attract {
    idle: u32,
}

impl Attract {
    /// One tick. `eligible`: the menu is the title's main menu with nothing over it. `input`: any
    /// press, key, click, pointer move or touch happened this tick. Returns whether the demo
    /// starts now; the count restarts either way.
    pub fn tick(&mut self, idle_ticks: u32, eligible: bool, input: bool) -> bool {
        if input || !eligible {
            self.idle = 0;
            return false;
        }
        self.idle = self.idle.saturating_add(1);
        if self.idle < idle_ticks {
            return false;
        }
        self.idle = 0;
        true
    }

    /// Ticks the menu has been left alone.
    #[must_use]
    pub fn idle_ticks(&self) -> u32 {
        self.idle
    }
}

/// The two durations a demo runs on, in ticks: the title's ([`Demo`]) unless its own
/// `DemoConfig.xml` authors them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timing {
    /// Ticks an untouched main menu waits.
    pub idle: u32,
    /// Ticks a demo runs.
    pub length: u32,
}

impl Timing {
    /// `demo`'s timing with the authored file's seconds laid over it.
    #[must_use]
    pub fn of(demo: &Demo, config: Option<&DemoConfig>) -> Self {
        let ticks = |seconds: Option<f32>, own: u32| {
            seconds
                .filter(|seconds| seconds.is_finite() && *seconds > 0.0)
                .map_or(own, |seconds| (seconds * 60.0).round() as u32)
        };
        Self {
            idle: ticks(
                config.and_then(|config| config.start_seconds),
                demo.idle_ticks(),
            ),
            length: ticks(
                config.and_then(|config| config.end_seconds),
                demo.duration_ticks(),
            ),
        }
    }
}

/// The circuit demo `n` flies when the title authors a cycle: the `n`th named circuit (wrapping)
/// among `circuits`, which are `(id, location)` pairs as the player can pick them. A name matches
/// a circuit's id or the last part of its location, ignoring case. `None` when nothing authored
/// is on offer, and the seeded pick stands.
#[must_use]
pub fn authored_circuit(
    config: Option<&DemoConfig>,
    circuits: &[(&str, &str)],
    n: u32,
) -> Option<usize> {
    let names = &config?.tracks;
    let wanted: Vec<usize> = names
        .iter()
        .filter_map(|name| {
            circuits.iter().position(|(id, location)| {
                let leaf = location.rsplit(['\\', '/']).next().unwrap_or(location);
                id.eq_ignore_ascii_case(name) || leaf.eq_ignore_ascii_case(name)
            })
        })
        .collect();
    wanted.get(n as usize % wanted.len().max(1)).copied()
}

/// Why a demo is over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Leave {
    /// Somebody touched an input; the main menu is next.
    Input,
    /// It ran its length or its race finished.
    Over,
}

/// One demo race, from the tick its race stage is live.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Run {
    ticks: u32,
    wallpaper: bool,
    input: bool,
}

impl Run {
    /// A demo about to start. A `wallpaper` run ignores input.
    #[must_use]
    pub fn new(wallpaper: bool) -> Self {
        Self {
            ticks: 0,
            wallpaper,
            input: false,
        }
    }

    /// Whether input is ignored, and the HUD hidden with it.
    #[must_use]
    pub fn wallpaper(&self) -> bool {
        self.wallpaper
    }

    /// Notes that an input happened. A wallpaper hears nothing. Kept until the next
    /// [`Self::tick`], so a press during the loading screen ends the demo on its first tick.
    pub fn hear(&mut self, input: bool) {
        self.input |= input && !self.wallpaper;
    }

    /// One tick of the live race; `finished` is the race's own end, `length` the ticks a demo
    /// runs ([`Demo::duration_ticks`], unless a headless run shortens it); `stepping` is whether the
    /// race stepped this tick.
    pub fn tick(&mut self, length: u32, finished: bool, stepping: bool) -> Option<Leave> {
        if std::mem::take(&mut self.input) {
            return Some(Leave::Input);
        }
        // A race held still (a window away, a pause) is not running down its length.
        self.ticks = self.ticks.saturating_add(u32::from(stepping));
        (finished || self.ticks >= length).then_some(Leave::Over)
    }

    /// Ticks the race has run.
    #[must_use]
    pub fn ticks(&self) -> u32 {
        self.ticks
    }
}

/// The laps a demo's race is set to, so it never ends on them: with its timer held off the
/// original's demo ran 376 s and never finished, and only the length ends one (measured on Pulse
/// PSP, `docs/ghidra/functions/psp-pulse-usa/attract-demo.md`).
pub const LAPS: u32 = 99;

/// What follows a demo that ran out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Next {
    /// Another demo at once.
    Demo,
    /// The main menu.
    Menu,
}

/// Where a demo that ran out goes: `chain` (a run started with `--demo`) always on to the next
/// demo, else the title's [`Demo::after_end`].
#[must_use]
pub fn after_over(demo: &Demo, chain: bool) -> Next {
    if chain {
        return Next::Demo;
    }
    match demo.after_end.value {
        AfterEnd::NextDemo => Next::Demo,
        AfterEnd::MainMenu => Next::Menu,
    }
}

/// Which circuit and team a demo races, as indices into the lists the caller offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pick {
    /// Index into the circuits.
    pub track: usize,
    /// Index into the teams.
    pub team: usize,
}

/// The title's selection rule over `tracks` circuits and `teams` teams the player can already
/// pick, from `seed`. A different circuit from `previous` when there is more than one. `None`
/// with nothing to pick from.
#[must_use]
pub fn pick(
    demo: &Demo,
    seed: u64,
    tracks: usize,
    teams: usize,
    previous: Option<Pick>,
) -> Option<Pick> {
    if tracks == 0 || teams == 0 {
        return None;
    }
    match demo.selection.value {
        // Both rules draw uniformly; they differ in what the caller offers
        // ([`unlocks_ignored`], [`offered_teams`]) and the measured one never repeats the team.
        Selection::SeededFromPlayable | Selection::AnyCircuitMultiplayerTeam => {
            let mut rng = Rng::new(seed);
            let mut track = rng.below(u32::try_from(tracks).ok()?) as usize;
            let mut team = rng.below(u32::try_from(teams).ok()?) as usize;
            if tracks > 1 && previous.is_some_and(|last| last.track == track) {
                track = (track + 1) % tracks;
            }
            if teams > 1
                && demo.selection.value == Selection::AnyCircuitMultiplayerTeam
                && previous.is_some_and(|last| last.team == team)
            {
                team = (team + 1) % teams;
            }
            Some(Pick { track, team })
        }
    }
}

/// Whether the rule offers circuits the player has not unlocked.
#[must_use]
pub fn unlocks_ignored(demo: &Demo) -> bool {
    demo.selection.value == Selection::AnyCircuitMultiplayerTeam
}

/// The teams the rule draws from, out of `(id, multiplayer)` for every team the title offers:
/// the measured rule takes the ones marked `multiplayer`, or every team on a title that marks
/// none; the chosen one takes every team.
#[must_use]
pub fn offered_teams<'a>(demo: &Demo, teams: &[(&'a str, bool)]) -> Vec<&'a str> {
    let marked = teams.iter().filter(|(_, multiplayer)| *multiplayer).count();
    let only_marked = demo.selection.value == Selection::AnyCircuitMultiplayerTeam && marked > 0;
    teams
        .iter()
        .filter(|(_, multiplayer)| *multiplayer || !only_marked)
        .map(|(id, _)| *id)
        .collect()
}

/// The seed of demo `n` in a chain started from `base`, so one run seed names the whole chain.
#[must_use]
pub fn chain_seed(base: u64, n: u32) -> u64 {
    base ^ u64::from(n).wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

#[cfg(test)]
mod tests;
