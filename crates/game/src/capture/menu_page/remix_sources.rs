//! The RACE REMIX page's own lists for a `--menu-page remix` capture: what a live
//! session supplies in `Session::open_menus`, for the one title a capture has open.
//!
//! A capture holds a single source, so TRACK TITLE and CRAFT TITLE offer that
//! title (plus CUSTOM when the directories hold tracks), and TRACK is its circuits,
//! or the scan's tracks when the settings name CUSTOM, which is the page a player
//! with Workshop tracks sees.

use oag_ui::menu::{Choice, Menu, ValueSource};

use crate::custom_tracks::{CUSTOM, Scan};

pub(super) fn supply(
    model: &mut Menu,
    title: &'static oag_title::Title,
    tracks: &[oag_raceplay::catalogue::Track],
    teams: &[oag_raceplay::catalogue::Team],
    settings: &crate::settings::Settings,
    strings: &oag_ui::language::StringTable,
) {
    let mut scan = Scan::start(&oag_source::unity::search_path());
    let mut titles = vec![Choice::plain(title.name)];
    if !scan.is_empty() {
        titles.push(Choice::labelled(CUSTOM, scan.title_label(strings)));
    }
    model.supply(ValueSource::Titles, &titles);
    model.supply(ValueSource::CraftTitles, &[Choice::plain(title.name)]);
    let offered = if settings.remix.track_title == CUSTOM {
        scan.wait();
        scan.choices(strings)
    } else {
        tracks
            .iter()
            .map(|track| Choice::labelled(&track.id, strings.get_or_id(&track.id)))
            .collect()
    };
    model.supply(ValueSource::RemixTracks, &offered);
    model.supply(
        ValueSource::RemixTeams,
        &teams
            .iter()
            .map(|team| Choice::labelled(&team.id, strings.get_or_id(&team.id)))
            .collect::<Vec<_>>(),
    );
}
