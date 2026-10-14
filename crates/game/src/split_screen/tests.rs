use super::*;
use oag_render::viewport::Cut;
use wgpu::DeviceType;

const ALL: [DeviceType; 5] = [
    DeviceType::IntegratedGpu,
    DeviceType::DiscreteGpu,
    DeviceType::VirtualGpu,
    DeviceType::Cpu,
    DeviceType::Other,
];

fn titles() -> [(&'static oag_title::Title, bool); 5] {
    [
        (oag_pulse::TITLE, true),
        (oag_pure::TITLE, true),
        (oag_hd::TITLE, false),
        (oag_2048::TITLE, false),
        (oag_omega::TITLE, false),
    ]
}

#[test]
fn an_integrated_gpu_offers_a_split_for_pulse_and_pure_only() {
    for (title, on_igpu) in titles() {
        assert_eq!(
            offered(title, DeviceType::IntegratedGpu),
            on_igpu,
            "{}",
            title.name
        );
    }
}

#[test]
fn every_other_adapter_offers_every_title_and_a_software_one_is_not_gated() {
    for (title, _) in titles() {
        for device in ALL
            .into_iter()
            .filter(|device| *device != DeviceType::IntegratedGpu)
        {
            assert!(offered(title, device), "{} on {device:?}", title.name);
        }
    }
}

#[test]
fn players_fall_back_to_one_view_only_where_the_split_is_not_offered() {
    assert_eq!(
        players_offered(oag_hd::TITLE, DeviceType::IntegratedGpu, 4),
        1
    );
    assert_eq!(
        players_offered(oag_hd::TITLE, DeviceType::DiscreteGpu, 4),
        4
    );
    assert_eq!(
        players_offered(oag_pulse::TITLE, DeviceType::IntegratedGpu, 4),
        4
    );
    assert_eq!(
        players_offered(oag_omega::TITLE, DeviceType::IntegratedGpu, 1),
        1
    );
}

/// Whether the main page shows the MULTIPLAYER row.
fn has_multiplayer_row(definition: &oag_ui::menu::Definition) -> bool {
    definition
        .page(&definition.pages[definition.root].id)
        .is_some_and(|main| {
            main.meta
                .iter()
                .any(|meta| meta.id.as_deref() == Some("multiplayer"))
        })
}

fn built(title: &'static oag_title::Title) -> oag_ui::menu::Definition {
    crate::mods::Mods::load()
        .unwrap()
        .build(
            oag_ui::menu::BUILT_IN,
            None,
            Some(oag_ui::menu::For {
                title,
                updated: false,
            }),
        )
        .unwrap()
        .definition
}

/// Every title shows MULTIPLAYER on its main menu, and an integrated GPU takes the row away on the
/// three titles that offer no split on one (the page behind it would hold nothing to do).
#[test]
fn an_integrated_gpu_menu_has_the_multiplayer_row_for_pulse_and_pure_alone() {
    for (title, on_igpu) in titles() {
        for device in ALL {
            let mut definition = built(title);
            assert!(
                has_multiplayer_row(&definition),
                "{} starts with it",
                title.name
            );
            let dropped = trim_menu(&mut definition, title, device);
            assert_eq!(dropped, !offered(title, device));
            assert_eq!(has_multiplayer_row(&definition), !dropped, "{}", title.name);
            if device == DeviceType::IntegratedGpu {
                assert_eq!(has_multiplayer_row(&definition), on_igpu, "{}", title.name);
            }
        }
    }
}

/// The RACE page keeps no NUMBER OF PLAYERS or SPLIT SCREEN row on any title: the head count is
/// the join screen's and the cut the layout screen's.
#[test]
fn no_title_keeps_the_players_or_cut_rows_on_the_race_page() {
    for (title, _) in titles() {
        let mut definition = built(title);
        definition.drop_rows_picked_on_screen(title);
        let settings: Vec<String> = definition
            .pages
            .iter()
            .find(|page| page.id == "race")
            .expect("the RACE page")
            .entries
            .iter()
            .filter_map(|entry| entry.setting().map(str::to_string))
            .collect();
        for key in ["race.players", "race.split_cut"] {
            assert!(
                !settings.iter().any(|s| s == key),
                "{key} on {}",
                title.name
            );
        }
    }
}

fn counts(layout: &[WindowSeats]) -> Vec<usize> {
    layout.iter().map(|window| window.count).collect()
}

/// `--windows N --players P` deals P people across N windows as evenly as it divides, the
/// first windows taking the remainder; a window nobody is dealt shows a holding frame.
#[test]
fn people_are_dealt_across_windows_evenly() {
    let cut = oag_render::viewport::Cut::Horizontal;
    assert_eq!(counts(&deal(2, 2, cut)), [1, 1]);
    assert_eq!(counts(&deal(8, 2, cut)), [4, 4]);
    assert_eq!(counts(&deal(6, 4, cut)), [2, 2, 1, 1]);
    assert_eq!(counts(&deal(3, 2, cut)), [2, 1]);
    assert_eq!(counts(&deal(2, 4, cut)), [1, 1, 0, 0]);
    assert_eq!(
        counts(&deal(12, 2, cut)),
        [4, 4],
        "the grid shows eight at most"
    );
    assert_eq!(counts(&deal(5, 0, cut)), [5], "no windows is one");
}

/// `--window-seats 4,2v`: a count per window, `h`/`v` for how two are cut, the default cut
/// otherwise; everything else is refused by name.
#[test]
fn window_seats_are_a_count_per_window_with_an_optional_cut() {
    use oag_render::viewport::Cut;
    let layout = parse_window_seats("4,2v", Cut::Horizontal).unwrap();
    assert_eq!(counts(&layout), [4, 2]);
    assert_eq!(layout[0].cut, Cut::Horizontal, "the default");
    assert_eq!(layout[1].cut, Cut::Vertical);
    assert_eq!(
        parse_window_seats(" 2H , 2 ", Cut::Vertical).unwrap()[0].cut,
        Cut::Horizontal
    );
    for (bad, why) in [
        ("4", "one window is --players"),
        ("4,x", "not a seat count"),
        ("4,0", "holds 1 to 8"),
        ("4,9", "holds 1 to 8"),
        ("5,4", "at most 8"),
        ("", "not a seat count"),
        ("2q,2", "not a seat count"),
    ] {
        let error = parse_window_seats(bad, Cut::Horizontal).unwrap_err();
        assert!(error.contains(why), "{bad:?}: {error}");
    }
}

/// A launch whose head count is what `--window-seats` spelled keeps the spelling; any other head
/// count (the join screen seated different people) is dealt across the windows that are open.
#[test]
fn a_launch_keeps_the_spelled_layout_only_for_the_head_count_it_spelled() {
    use oag_render::viewport::Cut;
    let wanted = parse_window_seats("4,2v", Cut::Horizontal).unwrap();
    let kept = layout_for(Some(&wanted), 6, 2, Cut::Horizontal);
    assert_eq!(kept, wanted);
    assert_eq!(
        counts(&layout_for(Some(&wanted), 3, 2, Cut::Horizontal)),
        [2, 1]
    );
    assert_eq!(
        counts(&layout_for(Some(&wanted), 4, 1, Cut::Horizontal)),
        [4]
    );
    assert_eq!(
        counts(&layout_for(None, 2, 2, Cut::Horizontal)),
        [1, 1],
        "nothing spelled is dealt"
    );
}

/// What the load is told: two when any window holds exactly two (that window may use the title's
/// authored two-player HUD), else the biggest window; one for a window each.
#[test]
fn the_load_is_told_two_views_when_any_window_holds_two() {
    use oag_render::viewport::Cut;
    let layout = |text: &str| parse_window_seats(text, Cut::Horizontal).unwrap();
    assert_eq!(views_for(&layout("4,2")), 2);
    assert_eq!(views_for(&layout("4,3")), 4);
    assert_eq!(views_for(&layout("1,1")), 1);
}

#[test]
fn a_layout_is_spelled_the_way_the_command_line_reads_it() {
    for text in ["4,2v", "3,1", "2h,2v,1", "1,1,1,1"] {
        let layout = parse_window_seats(text, Cut::Horizontal).unwrap();
        let spelled = spell_window_seats(&layout);
        assert_eq!(
            parse_window_seats(&spelled, Cut::Horizontal).unwrap(),
            layout,
            "{text} spelled {spelled}"
        );
    }
    assert_eq!(
        spell_window_seats(&deal(4, 1, Cut::Horizontal)),
        "",
        "one window is not a layout"
    );
}

#[test]
fn the_preview_cuts_seats_as_a_window_does() {
    for seats in 1..=8 {
        for vertical in [false, true] {
            let cells = seat_cells(seats, vertical);
            assert_eq!(cells.len(), seats);
            for cell in &cells {
                assert!(cell[0] >= 0.0 && cell[1] >= 0.0);
                assert!(cell[0] + cell[2] <= 1.0001 && cell[1] + cell[3] <= 1.0001);
            }
        }
    }
    let two = seat_cells(2, true);
    assert!(
        two[0][1] == two[1][1] && two[0][0] < two[1][0],
        "side by side"
    );
    let two = seat_cells(2, false);
    assert!(
        two[0][0] == two[1][0] && two[0][1] < two[1][1],
        "one above the other"
    );
}
