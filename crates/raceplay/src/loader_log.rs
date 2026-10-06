//! Where a loader's report reaches the logger.
//!
//! The loaders (`race::load`, `boot`, the sound banks, the preview cards)
//! describe what they did as a `Vec<String>` of sentences, because the same
//! report is also read by tests and by `--screenshot` runs. Those sentences carry
//! no level, and they mix two kinds of thing: counts and sizes a reader only
//! wants when something looks wrong, and the places the loader drew or played
//! **nothing** because an asset was absent or would not decode.
//!
//! The first kind is `debug` and the second is `warn`. Hiding the second at
//! `debug` would break the project's rule that a missing asset is an honest,
//! visible absence ("Never invent what the assets already author" in
//! `CLAUDE.md`), so every report goes through here, and the one list of phrases
//! that mark an absence lives here too. See `docs/architecture/logging.md`.
//!
//! A phrase list is a heuristic, and it errs towards `warn`: a sentence that
//! should have been `warn` and was missed would hide an absence, where one that
//! was escalated by mistake costs a line. A loader that adds a new way of
//! saying "this draws nothing" adds the phrase to [`ABSENCE`] and a case to the
//! test below.

use log::Level;

/// Phrases a report line uses when the thing it describes is not there, did not
/// load, or is replaced by something the engine made up. Compared lowercase.
const ABSENCE: &[&str] = &[
    "draws nothing",
    "draw nothing",
    "drawn nothing",
    "nothing drawn",
    "nothing is drawn",
    "loaded, not drawn",
    "decoded but not drawn",
    "(0 triangle(s))",
    "will not be drawn",
    "no flyer is drawn",
    "play nothing",
    "plays nothing",
    ": absent",
    "did not decode",
    "not in the archive set",
    "shows no stills",
    "did not fit the sprite sheet",
];

/// Whether a report line says an asset is missing or was not used.
#[must_use]
pub fn is_absence(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    ABSENCE.iter().any(|phrase| lower.contains(phrase))
}

/// The level one report line is logged at: `quiet` for an ordinary line,
/// [`Level::Warn`] for an absence.
#[must_use]
pub fn level_of(line: &str, quiet: Level) -> Level {
    if is_absence(line) { Level::Warn } else { quiet }
}

/// How many absences one report may give a `warn` line of their own.
///
/// A source can lack dozens of assets of one kind (2048 ships none of the 30-odd
/// particle effects), and thirty lines saying nearly the same thing bury the
/// three that differ. Past the cap each absence's full line is logged at
/// `debug`, and one closing `warn` names every one of them, so nothing is
/// unstated at the default filter: it is named, not recited.
const WARN_CAP: usize = 8;

/// Logs a loader's report at `debug`, with each absence at `warn`.
pub fn lines<S: AsRef<str>>(report: impl IntoIterator<Item = S>) {
    lines_at(Level::Debug, report);
}

/// [`lines`] with the level an ordinary line gets chosen by the caller: `trace`
/// for a report with one line per item (every sound cue, every ship), where
/// even `debug` would print hundreds of lines.
pub fn lines_at<S: AsRef<str>>(quiet: Level, report: impl IntoIterator<Item = S>) {
    let report: Vec<S> = report.into_iter().collect();
    let (levels, held_back) = plan(report.iter().map(AsRef::as_ref), quiet);
    for (line, level) in report.iter().zip(levels) {
        let line = line.as_ref();
        log::log!(level, "{line}");
    }
    if !held_back.is_empty() {
        log::warn!(
            "{} more absence(s), full lines at debug: {}",
            held_back.len(),
            held_back.join(", ")
        );
    }
}

/// The level each line is logged at, and the name of every absence [`WARN_CAP`]
/// moved down to `quiet`. Pure, so the cap is testable without a logger.
///
/// The name is the text before the line's first `": "` - the asset, in every
/// loader that writes `"<asset>: absent ..."` - or the whole line when it has
/// none, so a held-back absence is still *named* at `warn`.
fn plan<'a>(report: impl Iterator<Item = &'a str>, quiet: Level) -> (Vec<Level>, Vec<&'a str>) {
    let mut warned = 0;
    let mut held_back = Vec::new();
    let levels = report
        .map(|line| match level_of(line, quiet) {
            Level::Warn if warned < WARN_CAP => {
                warned += 1;
                Level::Warn
            }
            Level::Warn => {
                held_back.push(line.split_once(": ").map_or(line, |(name, _)| name));
                quiet
            }
            level => level,
        })
        .collect();
    (levels, held_back)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sentences lifted from the loaders' own reports on a real disc.
    const ABSENT: &[&str] = &[
        r"Data\Weapons\Textures\Cannon_bolt.mip: absent - the cannon bolt streak draws nothing",
        r"Data\Weapons\Pulse_Bomb.vex: absent - a laid bomb falls back to a billboard",
        r"Data\Environments\x\cloud.mip: not found - 12 cloud sprite(s) decoded but not drawn",
        r"8 billboard-slot placeholder draw(s) suppressed: drawn nothing rather than the stub",
        r"track audio talonsj~SETREG_02: 2 node(s) play nothing: binds no waveform",
        r"Data\Weapons\Textures\x.mip: not in the archive set (no entry) - the LeachBeam draws without it",
        r"HUD atlas a.gtf did not decode; its sprites draw nothing",
        r"Data\Weapons\x.vex: 0 of 2 mesh node(s) drawn from the .rcsmodel (0 triangle(s)); 2 addressed no chunk",
        r"Data\Tex\staticglow.mip: absent - the ghost ship's static draws nothing",
        r"Data\Mixed\Case.mip: ABSENT - Draws Nothing",
        r"WO_X: sprite of a, b did not fit the sprite sheet - drawn as the procedural profile",
    ];

    /// Sentences that report what loaded, which must stay out of the default log.
    const PRESENT: &[&str] = &[
        r"Data\Ships\Assegai\Ship.vex: 1482 triangle(s), model centre [0.0, 0.0, 0.0], radius 6.97",
        r"sfx: ABSORB -> 1 waveform(s) from Data\Sound\weapons.bnk",
        r"Data\Ships\Qirex\Ship.vex: 12 of 12 mesh node(s) drawn from the .rcsmodel (29643 triangle(s))",
        r"slot 0: Data\Ships\assegai\Ship.vex authors no Dynamic Shadow Occluder - this craft casts no `original` shadow",
        r"Data\Ships\Qirex\Ship.vex: 12 of 12 mesh node(s) drawn from the .rcsmodel (29643 triangle(s)), 1 second texture(s) loaded but not drawn (role unread)",
        "dlc: 16 archive(s) mounted behind this source",
        "racing on Wipeout Pulse",
        r"slot 0: no Data\Ships\Assegai\textures\ambient_shadow.gtf - the blob shadow falls back to a generated falloff",
        "start gantry x.vex on node Some(74): 6 draw(s) parked outside the panel are not drawn: the FINAL LAP states",
    ];

    #[test]
    fn an_absence_is_a_warning_and_a_load_is_not() {
        for line in ABSENT {
            assert_eq!(level_of(line, Level::Debug), Level::Warn, "{line}");
        }
        for line in PRESENT {
            assert_eq!(level_of(line, Level::Debug), Level::Debug, "{line}");
        }
    }

    #[test]
    fn the_quiet_level_is_the_callers_to_choose() {
        assert_eq!(level_of("racing on Pulse", Level::Trace), Level::Trace);
        assert_eq!(
            level_of("x: absent - draws nothing", Level::Trace),
            Level::Warn
        );
    }

    #[test]
    fn a_report_of_many_absences_warns_for_the_first_few_and_names_the_rest() {
        let absent = "x.POB: not in the archive set - x will not be drawn";
        let report: Vec<&str> = std::iter::repeat_n(absent, WARN_CAP + 5)
            .chain(["racing on Pulse"])
            .collect();
        let (levels, held_back) = plan(report.iter().copied(), Level::Debug);
        assert_eq!(
            levels.iter().filter(|l| **l == Level::Warn).count(),
            WARN_CAP
        );
        assert_eq!(held_back, vec!["x.POB"; 5]);
        assert_eq!(levels.last(), Some(&Level::Debug));
    }

    #[test]
    fn a_report_within_the_cap_loses_nothing() {
        let (levels, held_back) = plan(ABSENT[..WARN_CAP].iter().copied(), Level::Debug);
        assert!(levels.iter().all(|l| *l == Level::Warn));
        assert!(held_back.is_empty());
    }
}
