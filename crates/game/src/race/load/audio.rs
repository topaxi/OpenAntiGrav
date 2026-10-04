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
    slot_teams: &[String],
    report: &mut Vec<String>,
) -> (
    crate::audio::sfx::Banks,
    crate::audio::sfx::Announcer,
    crate::audio::sfx::ClassAnnouncer,
) {
    // Adding a cue is adding its name and its trigger, never a loader. Zone
    // races read `ship_zone.bnk` instead of `ship.bnk` - a different bank
    // with the same cue names.
    let tick = race
        .zone_announcer
        .map_or(oag_title::SequenceTick::Unknown, |z| z.tick);
    let mut sounds =
        crate::audio::sfx::Banks::load(archives, race.sounds, mode == Mode::Zone, tick);
    // The start-of-race voice, from the speech bank this mode opened rather
    // than the title's: Zone and Eliminator each load their own. See
    // `oag_title::CountdownVoice`.
    if let Some(voice) = race.countdown_voice {
        let entry = match mode {
            Mode::Zone => voice.zone_bank,
            Mode::Eliminator => voice.eliminator_bank,
            _ => race.sounds.speech,
        };
        sounds.load_countdown(archives, entry, tick);
    }
    // HD's engine is a per-team crossfade table, not a cue: loaded only where
    // the ship bank has no `~ENGINE`. See `crate::audio::sfx::XfadeTeam`.
    let ship_bank = if mode == Mode::Zone {
        race.sounds.ship_zone
    } else {
        race.sounds.ship
    };
    sounds.load_xfade(archives, ship_bank, slot_teams, report);
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

/// The circuit's own authored sound emitters, read off the track `.vex` already
/// in hand.
///
/// Separate from [`banks_and_announcers`] because it is keyed by *circuit*
/// rather than by title: one disc's twelve tracks author twelve different sets
/// and a Zone circuit authors none, where the bank table is one per release.
/// See [`crate::audio::sfx::TrackEmitters`].
pub(super) fn track_emitters(
    archives: &mut oag_assets::Archives,
    banks: &oag_title::SoundBanks,
    track: &str,
    blob: &[u8],
    report: &mut Vec<String>,
) -> crate::audio::sfx::TrackEmitters {
    let loaded = crate::audio::sfx::TrackEmitters::load(archives, banks, track, blob);
    report.extend(loaded.report.iter().cloned());
    loaded
}
