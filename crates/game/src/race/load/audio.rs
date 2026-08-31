//! The race's decoded sound cues and its two Zone-mode voice ladders.
//!
//! Split out of `load.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;

/// [`crate::audio::sfx::Banks`], [`crate::audio::sfx::Announcer`] and
/// [`crate::audio::sfx::ClassAnnouncer`], each loaded on the same "never
/// fails, an empty result costs nothing to carry" terms.
pub(super) fn banks_and_announcers(
    archives: &mut oag_assets::Archives,
    race: &'static oag_title::RaceDefaults,
    mode: Mode,
    report: &mut Vec<String>,
) -> (
    crate::audio::sfx::Banks,
    crate::audio::sfx::Announcer,
    crate::audio::sfx::ClassAnnouncer,
) {
    // Adding a cue is adding its name and its trigger, never a loader. Zone
    // races read `ship_zone.bnk` instead of `ship.bnk` - a different bank
    // with the same cue names.
    let sounds = crate::audio::sfx::Banks::load(archives, race.sounds, mode == Mode::Zone);
    report.extend(sounds.report.iter().cloned());

    // Loaded regardless of mode, on the same terms `sounds` is: an announcer
    // with nothing to say (not Zone, or a title with no recovered ladder) is
    // an empty map.
    let announcer = crate::audio::sfx::Announcer::load(archives, race.zone_announcer);
    report.extend(announcer.report.iter().cloned());

    // `announcer`'s sibling, keyed by stage instead of milestone.
    let class_announcer =
        crate::audio::sfx::ClassAnnouncer::load(archives, race.zone_class_announcer);
    report.extend(class_announcer.report.iter().cloned());

    (sounds, announcer, class_announcer)
}
