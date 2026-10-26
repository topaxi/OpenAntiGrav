//! What launching with the RACE page's VARIANT row's pick means; the row's own
//! choices are `oag_game::livery_row`. Shared with RACE REMIX - see `super::remix`.
//!
//! Split out of `menus.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

/// Combines `team` with whichever variant `stored` names, for launching.
///
/// Two different things a title's own axis can mean, so two different halves
/// of the return value: [`oag_title::RaceDefaults::team_variants_for`]
/// combines into a **new team identity** (the first `String`, unchanged from
/// `team` when the axis is absent or is [`oag_title::race::HullVariant`]'s
/// instead); that axis, when it is the one present, names a hull-file
/// override instead (the `Option<&str>`) with the team's own identity left
/// alone - see [`oag_title::race::HullVariant`]'s own doc comment for why the
/// two cannot share one channel. The third value is a warning to log when
/// `stored` no longer matches one of the offered choices and the first one
/// was raced instead - `Menu::supply` resets what the row *shows* without
/// touching what `self.settings` holds until the player next moves it, so
/// `stored` can be stale the moment `team` changes underneath it, the same
/// way an unrecognised `race.track`/`race.team` already can be.
pub(crate) fn combine_variant(
    title: &'static oag_title::Title,
    team: &str,
    stored: &str,
) -> (String, Option<&'static str>, Option<String>) {
    if let Some(team_variants) = title.race.team_variants_for(team) {
        let (suffix, warning) = match team_variants.variants.iter().find(|v| v.suffix == stored) {
            Some(variant) => (variant.suffix, None),
            None => (
                team_variants
                    .variants
                    .first()
                    .map_or("", |variant| variant.suffix),
                Some(format!(
                    "{team} does not offer variant {stored:?}; racing the first one it does offer"
                )),
            ),
        };
        return (team_variants.join.combine(team, suffix), None, warning);
    }
    let Some(hull_variants) = title.race.hull_variants else {
        return (team.to_string(), None, None);
    };
    let (stem, warning) = match hull_variants.iter().find(|v| v.stem == stored) {
        Some(variant) => (variant.stem, None),
        None => (
            hull_variants.first().map_or("", |variant| variant.stem),
            Some(format!(
                "{team} does not offer variant {stored:?}; racing the first one it does offer"
            )),
        ),
    };
    (team.to_string(), Some(stem), warning)
}

#[cfg(test)]
#[path = "variant/tests.rs"]
mod tests;
