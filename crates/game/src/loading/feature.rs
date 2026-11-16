//! Which feature a race's loading screen shows, and the two words over it.
//!
//! Its own file because `loading.rs` is near the size ceiling, and because the
//! pick is a law read off Wipeout HD's constructor while the drawing is this
//! build's layout.

use super::Assets;

/// Which of `assets.features` this screen shows, as a position in that list.
///
/// **A draw from the mode's own deck.** The original reduces `rand()` modulo a
/// range its mode picks, and the set that range covers is the title's
/// [`oag_title::loading::Deck`]; `draw` is a seed rather than a clock read, so
/// a capture of this screen is reproducible. A title with no deck, or a deck
/// none of whose features resolved, draws from everything it has - a screen
/// with no feature would be a worse answer than a wider draw.
///
/// `mode` is the source executable's own mode id, `None` where no race is being
/// covered.
pub(super) fn pick(assets: &Assets, mode: Option<u32>, draw: u64) -> Option<usize> {
    let wanted = assets.deck.map(|deck| deck.indices(mode));
    let mut candidates: Vec<usize> = assets
        .features
        .iter()
        .enumerate()
        .filter(|(_, feature)| wanted.is_none_or(|deck| deck.contains(&feature.slot)))
        .map(|(at, _)| at)
        .collect();
    if candidates.is_empty() {
        candidates = (0..assets.features.len()).collect();
    }
    let draw = oag_core::rng::Rng::new(draw).next_u32() as usize;
    (!candidates.is_empty()).then(|| candidates[draw % candidates.len()])
}

/// The source executable's own mode id for a mode this build races, where it
/// is named.
///
/// Only the ones `docs/ghidra/functions/ps3-hdfury-eu/mode-manager.md` names:
/// `3` is `SPArcade` (a single race), `4` `SPTournament`, `5` `SPTimeTrial` and
/// `8` `SPElimination`, and `0xe` Detonator (settled on the same page). **Speed Lap,
/// Zone and Head2Head are `None`**, because the ids that would be theirs are among the
/// unnamed ones - `6` and `0xd` carry decks of their own and which of them is Zone is not
/// established. They draw from the default deck, which is chosen, not measured.
#[must_use]
pub fn executable_mode(mode: oag_race::Mode) -> Option<u32> {
    use oag_race::Mode;
    match mode {
        Mode::SingleRace => Some(3),
        Mode::Tournament => Some(4),
        Mode::TimeTrial => Some(5),
        Mode::Eliminator => Some(8),
        Mode::Detonator => Some(0xe),
        Mode::SpeedLap | Mode::Zone | Mode::Head2Head => None,
    }
}

/// The `oag_race::Mode::name` of the race the source executable's mode id `mode` covers: the
/// inverse of [`executable_mode`], which is how a layout file's `modes` list is matched.
#[must_use]
pub(super) fn mode_name(mode: u32) -> Option<&'static str> {
    use oag_race::Mode;
    [
        Mode::SingleRace,
        Mode::Tournament,
        Mode::TimeTrial,
        Mode::Eliminator,
        Mode::Detonator,
    ]
    .into_iter()
    .find(|candidate| executable_mode(*candidate) == Some(mode))
    .map(Mode::name)
}
