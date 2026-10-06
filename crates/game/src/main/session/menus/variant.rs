//! The RACE page's own VARIANT row: what it offers, and what launching with
//! it means. Shared with RACE REMIX - see `super::remix`.
//!
//! Split out of `menus.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use oag_ui::menu;

/// `title`'s own [`oag_title::RaceDefaults::team_variants_for`], scoped to
/// `team`, or - when that names no such axis - [`oag_title::RaceDefaults::hull_variants`],
/// which is not team-scoped at all. The VARIANT row's supply, on both the
/// RACE page and RACE REMIX. Empty when neither source has anything to offer:
/// `team` names no [`oag_title::TeamVariants`]/[`oag_title::GuestRoster`] axis
/// and the title authors no [`oag_title::race::HullVariant`] either - the
/// same idiom [`menu::ValueSource::MusicSources`] uses for a choice that
/// cannot be made.
pub(crate) fn variant_choices(title: &'static oag_title::Title, team: &str) -> Vec<menu::Choice> {
    if let Some(team_variants) = title.race.team_variants_for(team) {
        return team_variants
            .variants
            .iter()
            .map(|variant| menu::Choice::labelled(variant.suffix, variant.label))
            .collect();
    }
    title
        .race
        .hull_variants
        .into_iter()
        .flatten()
        .map(|variant| menu::Choice::labelled(variant.stem, variant.label))
        .collect()
}

/// [`variant_choices`] with Pulse's hull variants cut to the ones `team`'s own
/// `<Unlock>` rows let through (`Definition_IsUnlocked`, via
/// [`oag_game::unlock::loyalty_unlocked`]): a locked craft is absent from the
/// row, as it is from Team Selection's. `details` is the booted title's own
/// team catalogue; a team it does not hold, a title that does not gate
/// ([`oag_game::unlock::gates_variants`]) and an axis that is a
/// [`oag_title::TeamVariants`] one are returned whole.
pub(crate) fn gated_variant_choices(
    title: &'static oag_title::Title,
    team: &str,
    details: &[oag_raceplay::catalogue::Team],
) -> Vec<menu::Choice> {
    let mut choices = variant_choices(title, team);
    if !oag_game::unlock::gates_variants(title) || title.race.team_variants_for(team).is_some() {
        return choices;
    }
    let Some(declared) = details.iter().find(|candidate| candidate.id == team) else {
        return choices;
    };
    let records = oag_game::records::load();
    choices.retain(|choice| {
        declared
            .hull_unlocks
            .iter()
            .filter(|(stem, _)| stem.eq_ignore_ascii_case(&choice.value))
            .all(|(_, rows)| oag_game::unlock::loyalty_unlocked(rows, &records, title.name))
    });
    choices
}

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
