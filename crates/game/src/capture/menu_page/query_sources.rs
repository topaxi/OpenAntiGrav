//! The title rows and table-form queries a `--menu-page` capture answers: what a live
//! session supplies in `Session::open_menus` and `Session::resolve_menu_queries`, for the one
//! title a capture has open.
//!
//! A capture holds a single source, so a `titles_available` or `craft_titles` row offers that
//! title (plus CUSTOM when the directories hold tracks), `speed_classes_union` is its ladder,
//! and `tracks_of`, `teams_of` and `variants_of` answer for it, or the scan's tracks under
//! CUSTOM, which is the page a player with Workshop tracks sees.

use oag_ui::menu::{Choice, Menu, Query, ValueSource};

use crate::custom_tracks::{CUSTOM, Scan};

pub(super) fn supply(
    model: &mut Menu,
    title: &'static oag_title::Title,
    tracks: &[oag_raceplay::catalogue::Track],
    teams: &[oag_raceplay::catalogue::Team],
    settings: &crate::settings::Settings,
    strings: &oag_ui::language::StringTable,
) {
    let mut scan = Scan::discover();
    let mut titles = vec![Choice::plain(title.name)];
    if !scan.is_empty() {
        titles.push(Choice::labelled(CUSTOM, scan.title_label(strings)));
    }
    model.supply(ValueSource::Titles, &titles);
    model.supply(ValueSource::CraftTitles, &[Choice::plain(title.name)]);
    let ladder = oag_title::SpeedClasses::union(title.race.speed_classes);
    model.supply(
        ValueSource::RemixSpeedClasses,
        &ladder
            .into_iter()
            .filter(|name| oag_title::SpeedClasses::is_selectable(name))
            .map(|name| Choice::labelled(name.to_lowercase(), name))
            .collect::<Vec<_>>(),
    );
    // The rows a query reads from are seeded first, so the query answers for the saved pick.
    for (key, value) in crate::settings::mod_settings::seeds(&settings.mod_settings) {
        model.seed(&key, &value);
    }
    let team_choices: Vec<Choice> = teams
        .iter()
        .map(|team| Choice::labelled(&team.id, strings.get_or_id(&team.id)))
        .collect();
    crate::remix::resolve_queries(model, None, |query, args| match query {
        Query::TracksOf if args.title == CUSTOM => {
            scan.wait();
            Some(scan.choices(strings))
        }
        _ if args.title != title.name => None,
        Query::TracksOf => Some(
            tracks
                .iter()
                .map(|track| Choice::labelled(&track.id, strings.get_or_id(&track.id)))
                .collect(),
        ),
        Query::TeamsOf => Some(team_choices.clone()),
        Query::VariantsOf => {
            let team = args
                .companion
                .or_else(|| teams.first().map(|team| team.id.as_str()));
            Some(crate::livery_row::variant_choices(
                title,
                team.unwrap_or(""),
            ))
        }
        _ => None,
    });
}
