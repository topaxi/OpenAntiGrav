//! Every entry point's pending race reaches the same Zone hull screen.

use oag_race::Mode;

use super::PendingRace;
use crate::{livery_row, preview, settings};

const TITLES: [&oag_title::Title; 5] = [
    oag_pulse::TITLE,
    oag_pure::TITLE,
    oag_hd::TITLE,
    oag_omega::TITLE,
    oag_2048::TITLE,
];

fn page(mode: Mode) -> settings::Race {
    settings::Race {
        mode: mode.name().to_string(),
        ..settings::Race::default()
    }
}

fn cell(mode: Mode) -> oag_raceplay::Options {
    oag_raceplay::Options {
        mode,
        class: "phantom".to_string(),
        ..oag_raceplay::Options::default()
    }
}

/// What Ship Select draws for `pending` on `title`: the fixed Zone label, and the Zone hull file
/// the preview loads (`None` on a title whose Zone hull is the team's own).
fn ship_screen(
    title: &'static oag_title::Title,
    pending: &PendingRace,
) -> (bool, Option<String>, usize) {
    let livery = livery_row::ship_livery(
        title,
        "Feisar",
        None,
        &oag_ui::language::StringTable::default(),
        &crate::records::Store::default(),
        pending.mode,
    );
    (
        livery_row::fixed_row(title, pending.mode),
        preview::zone_hull_file(title, pending.mode),
        livery.variants.len(),
    )
}

/// **The reported bug.** A campaign Zone tile leaves the RACE page's settings on the last mode, so
/// a picker that read them previewed the ordinary hull. The racebox and the cell must resolve to
/// the same screen, on every title.
#[test]
fn a_racebox_zone_race_and_a_campaign_zone_tile_show_the_same_zone_screen() {
    for title in TITLES {
        let racebox = PendingRace::resolve(&page(Mode::Zone), None);
        let tile = PendingRace::resolve(&page(Mode::TimeTrial), Some(&cell(Mode::Zone)));
        assert_eq!(racebox.mode, Mode::Zone, "{}", title.name);
        assert_eq!(tile.mode, Mode::Zone, "{}", title.name);
        let (fixed, hull, variants) = ship_screen(title, &tile);
        assert!(fixed, "{}: the Zone row is a fixed label", title.name);
        assert_eq!((fixed, hull.clone(), variants), {
            let (f, h, v) = ship_screen(title, &racebox);
            (f, h, v)
        });
        assert_eq!(
            variants, 1,
            "{}: no hull-variant choice in Zone",
            title.name
        );
    }
}

/// A non-Zone tile keeps the ordinary hull and the team's variants even when the RACE page was
/// left on Zone, and carries the cell's own class.
#[test]
fn a_non_zone_campaign_tile_keeps_the_ordinary_hull() {
    for title in TITLES {
        let tile = PendingRace::resolve(&page(Mode::Zone), Some(&cell(Mode::TimeTrial)));
        assert_eq!(tile.mode, Mode::TimeTrial, "{}", title.name);
        assert_eq!(tile.class, "phantom");
        let (fixed, hull, _) = ship_screen(title, &tile);
        assert!(!fixed, "{}", title.name);
        assert_eq!(hull, None, "{}", title.name);
    }
}

/// With no campaign cell the RACE page's settings are the race.
#[test]
fn the_race_page_settings_are_the_race_without_a_cell() {
    let pending = PendingRace::resolve(&page(Mode::Eliminator), None);
    assert_eq!(pending.mode, Mode::Eliminator);
    assert_eq!(pending.class, "venom");
}
