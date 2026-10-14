use super::*;
use oag_render::viewport::Cut;

fn titles() -> [&'static oag_title::Title; 5] {
    [
        oag_pulse::TITLE,
        oag_pure::TITLE,
        oag_hd::TITLE,
        oag_2048::TITLE,
        oag_omega::TITLE,
    ]
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

/// The RACE page keeps no NUMBER OF PLAYERS or SPLIT SCREEN row on any title: the head count is
/// the join screen's and the cut the layout screen's.
#[test]
fn no_title_keeps_the_players_or_cut_rows_on_the_race_page() {
    for title in titles() {
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

/// The seat assignment screen opens for any layout with two views or more in all, one window
/// holding a split included, and is skipped for a single view.
#[test]
fn the_seat_assignment_opens_for_any_layout_with_more_than_one_view() {
    assert!(has_panes_to_choose([2]), "one window holding a split");
    assert!(has_panes_to_choose([4]));
    assert!(has_panes_to_choose([1, 1]));
    assert!(has_panes_to_choose([2, 1]));
    assert!(!has_panes_to_choose([1]), "a single view");
    assert!(!has_panes_to_choose([]));
}

/// A race deals its views by pane when several windows are open, or one is open and the seat
/// assignment ran; `--players N` (no assignment, one window) keeps the plain split.
#[test]
fn a_one_window_race_deals_only_after_a_seat_assignment() {
    assert!(deals_the_panes(2, &[]));
    assert!(deals_the_panes(1, &[1, 0]));
    assert!(!deals_the_panes(1, &[]));
}

fn slots(layout: &[WindowSeats], order: &[usize]) -> Vec<Vec<usize>> {
    window_slots(layout, order)
}

/// Player 3 can sit in window 1: the windows take the slots the assignment gives their panes,
/// and the grid order (the slots) is not touched.
#[test]
fn an_assignment_puts_a_person_in_the_window_they_chose() {
    let cut = oag_render::viewport::Cut::Horizontal;
    let layout = deal(3, 2, cut);
    assert_eq!(
        slots(&layout, &[]),
        [vec![0, 1], vec![2]],
        "no assignment: in order"
    );
    assert_eq!(slots(&layout, &[0, 2, 1]), [vec![0, 2], vec![1]]);
    assert_eq!(slots(&layout, &[2, 0, 1]), [vec![2, 0], vec![1]]);
}

/// An order that is not every slot once is no assignment: the windows deal in order rather
/// than seat a slot twice or not at all.
#[test]
fn an_order_that_is_not_a_permutation_is_ignored() {
    let layout = deal(3, 2, oag_render::viewport::Cut::Horizontal);
    for bad in [
        &[0, 0, 1][..],
        &[0, 1][..],
        &[0, 1, 3][..],
        &[0, 1, 2, 3][..],
    ] {
        assert_eq!(slots(&layout, bad), [vec![0, 1], vec![2]], "{bad:?}");
    }
}

fn keys(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_string()).collect()
}

/// A device sits where it sat; the rest fill the panes left, in join order.
#[test]
fn the_remembered_panes_put_each_device_back_where_it_sat() {
    let now = keys(&["keyboard", "pad", "pad 2"]);
    assert_eq!(
        remembered_panes(&now, &keys(&["pad 2", "keyboard", "pad"])),
        [2, 0, 1]
    );
    assert_eq!(
        remembered_panes(&now, &[]),
        [0, 1, 2],
        "nothing remembered: join order"
    );
    // `pad 2` was unplugged and `other` is new: `keyboard` keeps pane 1, the rest fill in order.
    assert_eq!(
        remembered_panes(
            &keys(&["keyboard", "pad", "other"]),
            &keys(&["pad 2", "keyboard", "pad"])
        ),
        [2, 0, 1]
    );
    for saved in [
        keys(&["x", "y", "z", "w"]),
        keys(&["keyboard", "keyboard", "keyboard"]),
    ] {
        let mut order = remembered_panes(&now, &saved);
        order.sort_unstable();
        assert_eq!(
            order,
            [0, 1, 2],
            "{saved:?}: a permutation whatever was remembered"
        );
    }
}

#[test]
fn devices_are_told_apart_by_kind_name_and_number_among_their_name() {
    use DeviceName::{Keyboard, Pad};
    let devices = [
        Keyboard(0),
        Pad(Some("Xbox".into())),
        Pad(Some("Xbox".into())),
        Pad(None),
        Keyboard(1),
    ];
    let named: Vec<String> = (0..devices.len())
        .map(|seat| device_key(&devices, seat))
        .collect();
    assert_eq!(
        named,
        ["keyboard", "xbox", "xbox 2", "gamepad", "keyboard 2"]
    );
}
