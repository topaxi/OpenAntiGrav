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
    tracks: &[crate::catalogue::Track],
    teams: &[crate::catalogue::Team],
    kill_targets: &[String],
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
        &kill_targets
            .iter()
            .map(oag_ui::menu::Choice::plain)
            .collect::<Vec<_>>(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The RACE page of every title, on a fresh profile: every row that
    /// carries a list shows a value. This is the blank SPEED CLASS a
    /// `--menu-page race` still drew when the capture never supplied it.
    #[test]
    fn every_race_page_row_with_a_list_shows_a_value_on_a_fresh_profile() {
        let strings = oag_ui::language::StringTable::default();
        let kills: Vec<String> = ["5", "10", "15", "20", "25"]
            .iter()
            .map(ToString::to_string)
            .collect();
        for title in [oag_pulse::TITLE, oag_pure::TITLE] {
            let mut definition =
                oag_ui::menu::Definition::parse(oag_ui::menu::BUILT_IN, &strings).unwrap();
            definition.drop_unavailable_race_variant(title);
            definition.drop_rows_picked_on_screen(title);
            let mut model = oag_ui::menu::Menu::new(definition);
            supply(&mut model, title, &[], &[], &kills, &strings);
            model.open("race");
            let class = model
                .page()
                .entries
                .iter()
                .find(|entry| entry.setting() == Some("race.class"))
                .unwrap_or_else(|| panic!("{} has no SPEED CLASS row", title.name));
            let Some(oag_ui::menu::Value::Text(shown)) = class.value() else {
                panic!("{}: SPEED CLASS shows nothing", title.name);
            };
            assert_eq!(shown, "VENOM", "{}: the first rung of the ladder", title.name);
            let kill = model
                .page()
                .entries
                .iter()
                .find(|entry| entry.setting() == Some("race.kill_target"))
                .unwrap_or_else(|| panic!("{} has no KILLS row", title.name));
            assert_eq!(
                kill.value(),
                Some(oag_ui::menu::Value::Text("5".into())),
                "{}",
                title.name
            );
        }
    }
}
