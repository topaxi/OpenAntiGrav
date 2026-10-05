//! Which team, variant and livery a launched race flies: the team the player
//! last picked (`settings.race`), resolved against what this source offers.
//!
//! Shared by the RACE page's own launch ([`super::Session::launch_from_settings`])
//! and a campaign cell's launch (`Session::launch_campaign_race`), because both
//! end on the same `Team Selection` screen, and that screen writes its pick
//! into `settings.race` and nowhere else. Before this was shared, the campaign
//! path skipped it entirely, so a campaign race flew whatever `race_options.team`
//! already held: `None` on a session's first race (no loyalty row, no loyalty
//! banked, and `race::load`'s own default team), or the previous RACE-page
//! team after one. See `docs/ui/endrace-screens.md`.

use oag_game::settings;
use oag_raceplay as race;
use oag_ui::menu;

use super::combine_variant;

/// Writes `race`'s team, its combined variant, its hull variant and its skin
/// into `options`, when `teams` offers that team.
///
/// Returns warnings to log. A team this source does not offer (a stored
/// team only a missing pack carried) leaves `options` exactly as it was and
/// says so, the same way the RACE page always has - `Menu::supply` resets
/// the row silently, so without the warning the menu would show one team and
/// the race would attempt another.
pub(crate) fn apply_race_team(
    title: &'static oag_title::Title,
    teams: &[menu::Choice],
    race: &settings::Race,
    options: &mut race::Options,
) -> Vec<String> {
    let mut warnings = Vec::new();
    let Some(team) = teams
        .iter()
        .find(|choice| choice.value == race.team)
        .map(|choice| choice.value.as_str())
    else {
        warnings.push(format!(
            "this source does not offer team {:?}, racing as {} instead",
            race.team,
            options
                .team
                .as_deref()
                .unwrap_or("this source's own default"),
        ));
        return warnings;
    };
    let (combined, hull_variant, warning) = combine_variant(title, team, &race.variant);
    warnings.extend(warning);
    options.team = Some(combined);
    options.hull_variant = hull_variant.map(str::to_string);
    // The livery Ship Select picked, by the skin's declared name - what
    // `--skin` takes, resolved against the team's own `PI_ModelSkin`s at
    // load. Empty is the baseline paint.
    options.skin = Some(race.skin.clone()).filter(|skin| !skin.trim().is_empty());
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn teams(ids: &[&str]) -> Vec<menu::Choice> {
        ids.iter()
            .map(|id| menu::Choice::labelled(*id, *id))
            .collect()
    }

    fn picked(team: &str) -> settings::Race {
        settings::Race {
            team: team.to_string(),
            variant: String::new(),
            skin: String::new(),
            ..settings::Race::default()
        }
    }

    /// **The campaign bug's first shape.** A session's first race carries
    /// `race_options.team = None` (no `--team`); the team Team Selection
    /// picked must land, or the EndRace loyalty row is omitted.
    #[test]
    fn an_unset_team_takes_the_picked_one() {
        let mut options = race::Options {
            team: None,
            ..race::Options::default()
        };
        apply_race_team(
            oag_pulse::TITLE,
            &teams(&["Feisar", "Goteki45"]),
            &picked("Goteki45"),
            &mut options,
        );
        assert_eq!(options.team.as_deref(), Some("Goteki45"));
    }

    /// **Its second shape.** After a RACE-page race, `race_options.team`
    /// still names that race's team; a campaign race must fly the one
    /// picked since, not the stale one.
    #[test]
    fn a_stale_team_is_replaced_by_the_picked_one() {
        let mut options = race::Options {
            team: Some("Feisar".to_string()),
            ..race::Options::default()
        };
        apply_race_team(
            oag_pulse::TITLE,
            &teams(&["Feisar", "Goteki45"]),
            &picked("Goteki45"),
            &mut options,
        );
        assert_eq!(options.team.as_deref(), Some("Goteki45"));
    }

    /// The livery rides with the team: an empty skin is the baseline paint
    /// (`None`), a named one is carried by name.
    #[test]
    fn the_picked_skin_rides_with_the_team() {
        let mut options = race::Options::default();
        let mut race = picked("Feisar");
        race.skin = "Gold".to_string();
        apply_race_team(oag_pulse::TITLE, &teams(&["Feisar"]), &race, &mut options);
        assert_eq!(options.skin.as_deref(), Some("Gold"));

        race.skin = "  ".to_string();
        apply_race_team(oag_pulse::TITLE, &teams(&["Feisar"]), &race, &mut options);
        assert_eq!(options.skin, None);
    }

    /// A team this source does not offer changes nothing and says why.
    #[test]
    fn an_unoffered_team_leaves_the_options_alone_and_warns() {
        let mut options = race::Options {
            team: Some("Feisar".to_string()),
            ..race::Options::default()
        };
        let warnings = apply_race_team(
            oag_pulse::TITLE,
            &teams(&["Feisar"]),
            &picked("Mantis"),
            &mut options,
        );
        assert_eq!(options.team.as_deref(), Some("Feisar"));
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Mantis"), "{warnings:?}");
    }
}
