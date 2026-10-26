//! What a craft's livery row offers: one rule for the live Ship Select and its `--menu-page`
//! still, and for the RACE page's VARIANT row.
//!
//! Three axes meet here. A team that declares `PI_ModelSkin`s moves on the **skin** axis
//! (`Classic` plus the skins its loyalty has earned); a team that declares none moves on the
//! title's **variant** table (HD's `_c1`/`_n1`, Pulse's Normal/Concept hull files). Which axis a
//! team is on follows what it **declares**, never what is left once the locked ones are cut: a
//! fresh profile has every skin locked, and deciding on the cut list switched the row to the
//! variant table and offered a locked Concept hull until the first skin unlocked.
//!
//! An Eliminator race forces the `Eliminator` skin on every craft with no unlock consulted
//! (`oag_title::RaceDefaults::eliminator_skin`), so the row holds that skin alone.

use oag_raceplay::catalogue::{self, Team};
use oag_ui::menu;

use crate::records::Store;
use crate::unlock::{gates_variants, loyalty_unlocked};

/// A team's livery row.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Livery {
    /// Whether the row is on the skin axis: the team declares at least one `PI_ModelSkin`.
    pub skinned: bool,
    /// The declared skins the row offers, `(name, archive entry)`.
    pub skins: Vec<(String, String)>,
    /// The row's values, `(id, label)`, the first being what a bare Confirm picks.
    pub variants: Vec<(String, String)>,
}

/// `title`'s own [`oag_title::RaceDefaults::team_variants_for`], scoped to `team`, or - when that
/// names no such axis - [`oag_title::RaceDefaults::hull_variants`], which is not team-scoped at
/// all. Empty when neither source has anything to offer.
#[must_use]
pub fn variant_choices(title: &'static oag_title::Title, team: &str) -> Vec<menu::Choice> {
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

/// [`variant_choices`] with a hull variant cut to the ones `team`'s own `<Unlock>` rows let
/// through (`Definition_IsUnlocked`, via [`loyalty_unlocked`]): a locked hull is absent from the
/// row. A team `details` does not hold, a title that does not gate ([`gates_variants`]) and an
/// axis that is a [`oag_title::TeamVariants`] one are returned whole.
#[must_use]
pub fn gated_variant_choices(
    title: &'static oag_title::Title,
    team: &str,
    details: &[Team],
    records: &Store,
) -> Vec<menu::Choice> {
    let mut choices = variant_choices(title, team);
    if !gates_variants(title) || title.race.team_variants_for(team).is_some() {
        return choices;
    }
    let Some(declared) = details.iter().find(|candidate| candidate.id == team) else {
        return choices;
    };
    choices.retain(|choice| {
        declared
            .hull_unlocks
            .iter()
            .filter(|(stem, _)| stem.eq_ignore_ascii_case(&choice.value))
            .all(|(_, rows)| loyalty_unlocked(rows, records, title))
    });
    choices
}

/// The livery row of `team` (`details` is its catalogue entry, when the source has one) in `mode`.
///
/// `records` is read only on a title that gates ([`gates_variants`]).
#[must_use]
pub fn ship_livery(
    title: &'static oag_title::Title,
    team: &str,
    details: Option<&Team>,
    strings: &oag_ui::language::StringTable,
    records: &Store,
    mode: oag_race::Mode,
) -> Livery {
    let gates = gates_variants(title);
    // A Zone race flies one hull and carries no skin (`oag_title::ZoneCraft`): the row holds
    // that hull alone, so there is nothing to pick.
    if mode == oag_race::Mode::Zone {
        return Livery {
            skinned: details.is_some_and(|team| !team.skins.is_empty()),
            skins: Vec::new(),
            variants: vec![(String::new(), strings.get_or_id("Zone").to_string())],
        };
    }
    let declared = details.map_or(&[][..], |team| team.skins.as_slice());
    if !declared.is_empty() {
        if let Some(name) = title
            .race
            .eliminator_skin
            .filter(|_| mode == oag_race::Mode::Eliminator)
            && let Some(skin) = details.and_then(|team| team.skin(name))
        {
            return Livery {
                skinned: true,
                skins: vec![(skin.name.clone(), skin.location.clone())],
                variants: vec![(skin.name.clone(), strings.get_or_id(&skin.name).to_string())],
            };
        }
        let skins: Vec<(String, String)> = declared
            .iter()
            .filter(|skin| !gates || loyalty_unlocked(&skin.unlock, records, title))
            .map(|skin| (skin.name.clone(), skin.location.clone()))
            .collect();
        let variants = std::iter::once((
            String::new(),
            strings.get_or_id(catalogue::BASELINE_SKIN).to_string(),
        ))
        .chain(
            skins
                .iter()
                .map(|(name, _)| (name.clone(), strings.get_or_id(name).to_string())),
        )
        .collect();
        return Livery {
            skinned: true,
            skins,
            variants,
        };
    }
    let details_slice: &[Team] = details.map_or(&[], std::slice::from_ref);
    let mut variants: Vec<(String, String)> =
        gated_variant_choices(title, team, details_slice, records)
            .into_iter()
            .map(|choice| (choice.value, choice.label))
            .collect();
    if variants.is_empty()
        && let Some(own) = details.and_then(Team::own_directory_variant)
    {
        variants.push(own);
    }
    Livery {
        skinned: false,
        skins: Vec::new(),
        variants,
    }
}

#[cfg(test)]
mod tests;
