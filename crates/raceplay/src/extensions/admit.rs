//! Which modules a race asks at all: before anything of a module is loaded for the race, the
//! module that exports `oag_wants_race` is told what the race is and says whether it runs
//! (`docs/architecture/wasm-extensions/hooks.md`, "Choosing its races").
//!
//! One choke point: the load calls [`admit`] right after finding the sources, so a module that
//! declines is gone before its shaders, effects, sounds, icons, pickup odds and pickup kind are
//! read, before [`prepare`](super::prepare) compiles it for the tick, and before the race is
//! marked modded.

#[cfg(feature = "extensions")]
use oag_ext_abi::{RaceInfo, RaceMode, channel_key};

use super::Source;

#[cfg(feature = "extensions")]
/// The ABI's name for `mode`. No `_` arm: a new mode does not compile until it has a code.
#[must_use]
pub(crate) fn race_mode(mode: oag_race::Mode) -> RaceMode {
    match mode {
        oag_race::Mode::TimeTrial => RaceMode::TimeTrial,
        oag_race::Mode::SpeedLap => RaceMode::SpeedLap,
        oag_race::Mode::Zone => RaceMode::Zone,
        oag_race::Mode::SingleRace => RaceMode::SingleRace,
        oag_race::Mode::Eliminator => RaceMode::Eliminator,
        oag_race::Mode::Tournament => RaceMode::Tournament,
        oag_race::Mode::Head2Head => RaceMode::Head2Head,
        oag_race::Mode::Detonator => RaceMode::Detonator,
    }
}

#[cfg(feature = "extensions")]
/// The race `options` will run, as a module is told it: `title` is the title's slug. The
/// module's own options are left zero; [`admit`] fills them per module.
#[must_use]
pub(crate) fn describe(options: &crate::Options, title: &str) -> RaceInfo {
    let people = options.people_flying();
    let field = crate::spawn::field_opponents(
        options.mode,
        people,
        options.no_driver_for_class,
        options.opponents,
    );
    RaceInfo {
        title: channel_key(&title.to_ascii_lowercase()),
        class: if options.class.is_empty() {
            0
        } else {
            channel_key(&options.class.to_ascii_lowercase())
        },
        mode: race_mode(options.mode),
        weapons: options.weapons_on(),
        campaign: options.campaign,
        people,
        crafts: field + 1,
        options: [0; oag_ext_abi::OPTIONS_LEN],
    }
}

/// The sources the race `options` runs: [`gather`](super::gather)ed, then [`admit`]ted for
/// `title` (its slug). The one call the load makes, so nothing of a module that declines is
/// read for the race.
#[must_use]
pub fn sources_for_race(
    options: &crate::Options,
    title: &str,
    report: &mut Vec<String>,
) -> Vec<Source> {
    let found = super::gather(
        &options.extensions,
        options.find_extensions.as_ref(),
        report,
    );
    admit(
        found,
        title,
        options.foreign_track.as_deref(),
        options,
        report,
    )
}

/// The `sources` that run in the race `options` describes: those for `title` (and
/// `foreign_track`, for a track module) whose module does not decline it. A module that
/// exports no `oag_wants_race` runs everywhere. One that declines is dropped with a debug
/// line in `report`; one whose check traps, runs out of fuel or answers neither run nor skip is
/// dropped too, by name. A module that does not compile stays: [`prepare`](super::prepare)
/// names why it is left out.
///
/// Without the `extensions` feature nothing is compiled, so nothing can decline.
#[must_use]
pub fn admit(
    sources: Vec<Source>,
    title: &str,
    foreign_track: Option<&std::path::Path>,
    options: &crate::Options,
    report: &mut Vec<String>,
) -> Vec<Source> {
    #[cfg(not(feature = "extensions"))]
    {
        let _ = (title, foreign_track, options, report);
        sources
    }
    #[cfg(feature = "extensions")]
    {
        let race = describe(options, title);
        sources
            .into_iter()
            .filter(|s| {
                s.titles.is_empty() || s.titles.iter().any(|t| t.eq_ignore_ascii_case(title))
            })
            .filter(|s| s.scope.runs_on(foreign_track))
            .filter(|source| runs(source, &race, report))
            .collect()
    }
}

#[cfg(feature = "extensions")]
fn runs(source: &Source, race: &RaceInfo, report: &mut Vec<String>) -> bool {
    let label = format!("mod {}: {}", source.mod_id, source.module);
    let limits = oag_modhost::Limits {
        memory_pages: source.memory_pages,
        state_bytes: source.state_bytes,
        fuel: oag_modhost::FUEL_PER_TICK,
    };
    let Ok(module) = (source.read)()
        .map_err(|e| e.to_string())
        .and_then(|wasm| oag_modhost::Module::compile(&wasm, limits).map_err(|e| e.to_string()))
    else {
        return true;
    };
    let race = RaceInfo {
        options: source.options.sim,
        ..*race
    };
    match module.wants_race(&race) {
        Ok(true) => true,
        Ok(false) => {
            let line = format!(
                "{label}: not run in this race ({:?}, weapons {}): its own check declined",
                race.mode, race.weapons
            );
            log::debug!("{line}");
            report.push(line);
            false
        }
        Err(fault) => {
            let line = format!("{label} left out: its race check {fault}");
            log::warn!("{line}");
            report.push(line);
            false
        }
    }
}
