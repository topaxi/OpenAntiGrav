//! The RACE page's own lists for a `--menu-page` capture: what a live session
//! supplies in `Session::open_menus`, so a captured row draws its value.
//!
//! A row whose list is never supplied draws its label and nothing on the right,
//! which reads as a blank value rather than as a missing list. SPEED CLASS did
//! exactly that in a capture until this module named it - the live menus
//! supplied it, this path did not.

/// Supplies every list the RACE page's rows read, off the same sources the live
/// menus use.
pub(super) fn supply(
    model: &mut oag_ui::menu::Menu,
    title: &'static oag_title::Title,
    tracks: &[oag_raceplay::catalogue::Track],
    teams: &[oag_raceplay::catalogue::Team],
    race_setup: &crate::boot::RaceSetup,
    strings: &oag_ui::language::StringTable,
) {
    model.supply(
        oag_ui::menu::ValueSource::Tracks,
        &tracks
            .iter()
            .map(|track| oag_ui::menu::Choice::labelled(&track.id, strings.get_or_id(&track.id)))
            .collect::<Vec<_>>(),
    );
    model.supply(
        oag_ui::menu::ValueSource::Teams,
        &teams
            .iter()
            .map(|team| oag_ui::menu::Choice::labelled(&team.id, strings.get_or_id(&team.id)))
            .collect::<Vec<_>>(),
    );
    model.supply(
        oag_ui::menu::ValueSource::RaceModes,
        &oag_ui::menu::mode_choices(strings),
    );
    model.supply(
        oag_ui::menu::ValueSource::SpeedClasses,
        &crate::scoreboard::speed_class_choices(title),
    );
    model.supply(
        oag_ui::menu::ValueSource::KillTargets,
        &race_setup
            .kill_targets
            .iter()
            .map(oag_ui::menu::Choice::plain)
            .collect::<Vec<_>>(),
    );
    model.supply(
        oag_ui::menu::ValueSource::Weapons,
        &race_setup.weapon_choices(strings),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Pulse RACE page the way a capture or a live session builds it: the
    /// title's own trims, the disc's own lists (Pulse's authored `Eliminations`
    /// and `Weapons`), a strings table with `FE_ON`/`FE_OFF`.
    fn pulse_page() -> oag_ui::menu::Menu {
        let mut strings = oag_ui::language::StringTable::default();
        strings.merge(
            [("FE_ON", "ON"), ("FE_OFF", "OFF")]
                .map(|(id, text)| (id.to_string(), text.to_string()))
                .into(),
        );
        let title = oag_pulse::TITLE;
        let mut definition =
            oag_ui::menu::Definition::parse(oag_ui::menu::BUILT_IN, &strings).unwrap();
        definition.drop_unavailable_race_variant(title);
        definition.drop_rows_picked_on_screen(title);
        let setup = crate::boot::RaceSetup {
            kill_targets: ["5", "10", "15", "20", "25"].map(String::from).to_vec(),
            weapons: vec![
                ("FE_ON".into(), "On".into()),
                ("FE_OFF".into(), "Off".into()),
            ],
        };
        let mut model = oag_ui::menu::Menu::new(definition);
        supply(&mut model, title, &[], &[], &setup, &strings);
        model.open("race");
        model
    }

    fn shown(model: &oag_ui::menu::Menu, setting: &str) -> String {
        let entry = model
            .page()
            .entries
            .iter()
            .find(|entry| entry.setting() == Some(setting))
            .unwrap_or_else(|| panic!("no {setting} row"));
        match model.shown(entry) {
            Some(oag_ui::menu::Value::Text(text)) => text,
            other => panic!("{setting} shows {other:?}"),
        }
    }

    /// The RACE page on a fresh profile shows a value on every row that has a
    /// list. SPEED CLASS drew blank in a `--menu-page race` still when the
    /// capture never supplied it.
    #[test]
    fn every_race_page_row_with_a_list_shows_a_value_on_a_fresh_profile() {
        let model = pulse_page();
        assert_eq!(shown(&model, "race.class"), "VENOM");
        assert_eq!(shown(&model, "race.kill_target"), "5");
        // Time Trial is the fresh profile's mode, and it pins WEAPONS to OFF.
        assert_eq!(shown(&model, "race.weapons"), "OFF");
        assert_eq!(shown(&model, "ai.difficulty"), "novice");
    }

    /// The recurrence guard for the blank SPEED CLASS: every row on the RACE
    /// page that draws its list from a source has something in it after
    /// `supply`, so a new `values_from` row the capture forgets fails here
    /// instead of drawing blank in a still.
    #[test]
    fn every_race_page_row_that_reads_a_source_is_supplied() {
        let model = pulse_page();
        for entry in &model.page().entries {
            if let oag_ui::menu::Entry::Choice {
                source: Some(source),
                values,
                label,
                ..
            } = entry
            {
                assert!(
                    !values.is_empty(),
                    "{label} reads {source:?} and the capture never supplied it"
                );
            }
        }
    }

    /// WEAPONS is greyed and shows the mode's own answer in every mode but a
    /// single race, which `Mode::weapons_enabled` says too - the pin in
    /// `menu.toml` and the simulation cannot drift apart without this failing.
    #[test]
    fn weapons_row_shows_what_the_mode_will_actually_do() {
        let mut model = pulse_page();
        for mode in oag_race::Mode::ALL {
            model.seed("race.mode", &oag_ui::menu::Value::Text(mode.name().into()));
            let entry = model
                .page()
                .entries
                .iter()
                .find(|entry| entry.setting() == Some("race.weapons"))
                .unwrap();
            let single_race = mode == oag_race::Mode::SingleRace;
            assert_eq!(model.is_disabled(entry), !single_race, "{mode:?} greyed");
            if !single_race {
                let want = if mode.weapons_enabled() { "ON" } else { "OFF" };
                assert_eq!(shown(&model, "race.weapons"), want, "{mode:?}");
            }
        }
    }

    /// A pick made in a single race survives a visit to a pinned mode.
    #[test]
    fn the_players_pick_returns_when_the_pinned_mode_is_left() {
        let mut model = pulse_page();
        model.seed(
            "race.mode",
            &oag_ui::menu::Value::Text("single_race".into()),
        );
        model.seed("race.weapons", &oag_ui::menu::Value::Text("Off".into()));
        assert_eq!(shown(&model, "race.weapons"), "OFF");
        model.seed("race.mode", &oag_ui::menu::Value::Text("eliminator".into()));
        assert_eq!(shown(&model, "race.weapons"), "ON");
        model.seed(
            "race.mode",
            &oag_ui::menu::Value::Text("single_race".into()),
        );
        assert_eq!(shown(&model, "race.weapons"), "OFF");
    }
}
