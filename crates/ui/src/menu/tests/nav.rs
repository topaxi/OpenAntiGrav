//! The navigation sounds a menu asks for: one per cursor move, page change,
//! value step and refusal, drained with [`Menu::take_nav`].

use super::*;
use crate::menu::nav::{Dir, Nav};
use crate::menu::pointer;

fn navs(menu: &mut Menu, buttons: &[Button]) -> Vec<Nav> {
    press(menu, buttons);
    menu.take_nav()
}

#[test]
fn a_walk_through_the_menus_asks_for_the_cues_the_original_plays() {
    let mut menu = Menu::new(fixture());
    assert_eq!(
        navs(&mut menu, &[Button::Down]),
        [Nav::Moved(Some(Dir::Down))]
    );
    assert_eq!(navs(&mut menu, &[Button::Up]), [Nav::Moved(Some(Dir::Up))]);
    assert_eq!(navs(&mut menu, &[Button::Cross]), [Nav::Accept], "OPTIONS");
    assert_eq!(menu.page().id, "options");
    assert_eq!(
        navs(&mut menu, &[Button::Right]),
        [Nav::Stepped(Some(Dir::Right))]
    );
    assert_eq!(
        navs(&mut menu, &[Button::Down]),
        [Nav::Moved(Some(Dir::Down))]
    );
    assert_eq!(
        navs(&mut menu, &[Button::Cross]),
        [Nav::Stepped(Some(Dir::Right))],
        "confirming a toggle flips it"
    );
    navs(&mut menu, &[Button::Down]);
    assert_eq!(navs(&mut menu, &[Button::Cross]), [Nav::Decline], "BACK");
    assert_eq!(menu.page().id, "main");
    assert_eq!(navs(&mut menu, &[Button::Circle]), [Nav::Decline]);
}

#[test]
fn a_press_that_changes_nothing_makes_no_sound() {
    let mut menu = Menu::new(
        Definition::parse(
            "toml-schema = { location = \"https://oag.topaxi.com/mods/schemas/v1/menu.tosd\", version = \"1.0.0\" }\nroot = \"main\"\n[[page]]\nid = \"main\"\n\
         [[page.entry]]\nkind = \"back\"\nlabel = \"ONLY\"\n",
            &crate::language::StringTable::default(),
        )
        .expect("the fixture must parse"),
    );
    assert!(
        navs(&mut menu, &[Button::Down]).is_empty(),
        "a one-row page has nowhere to move"
    );
    assert!(navs(&mut menu, &[Button::Left]).is_empty());
}

fn two_rows() -> Vec<pointer::Region> {
    [0, 1]
        .map(|row| pointer::Region {
            rect: [0.0, row as f32 * 20.0, 100.0, 18.0],
            row,
            part: pointer::Part::Row,
        })
        .to_vec()
}

fn pointer_nav(menu: &mut Menu, pointer: &crate::pointer::Pointer) -> Vec<Nav> {
    menu.pointer(pointer, &two_rows());
    menu.take_nav()
}

#[test]
fn hovering_sounds_once_per_row_and_a_click_on_a_submenu_accepts() {
    let mut menu = Menu::new(fixture());
    let over = |row: f32| crate::pointer::Pointer {
        at: Some((10.0, row * 20.0 + 5.0)),
        moved: true,
        ..Default::default()
    };
    assert_eq!(
        pointer_nav(&mut menu, &over(1.0)),
        [Nav::Moved(Some(Dir::Down))]
    );
    assert!(
        pointer_nav(&mut menu, &over(1.0)).is_empty(),
        "the same row again is not another move"
    );
    assert_eq!(
        pointer_nav(&mut menu, &over(0.0)),
        [Nav::Moved(Some(Dir::Up))]
    );
    let click = crate::pointer::Pointer {
        clicked: true,
        ..over(0.0)
    };
    assert_eq!(pointer_nav(&mut menu, &click), [Nav::Accept], "OPTIONS");
    assert_eq!(menu.page().id, "options");
}

/// The MULTIPLAYER page offers LOCAL MULTIPLAYER, and LAN and ONLINE as rows for later: dimmed,
/// focusable so the line under them can say "Coming later", and refusing the confirm with the
/// decline cue (a pointer click goes through the same gate, `pointer`'s own test).
#[test]
fn the_multiplayer_page_has_local_lan_and_online_rows() {
    let mut menu = Menu::new(built_in());
    menu.open("multiplayer");
    let actions: Vec<Option<Action>> = menu
        .page()
        .entries
        .iter()
        .map(|entry| match entry {
            Entry::Run { action, .. } => Some(*action),
            _ => None,
        })
        .collect();
    assert_eq!(
        actions,
        [
            Some(Action::OpenMultiplayerLocal),
            None,
            Some(Action::OpenOnline),
            None
        ]
    );
    let _ = menu.take_nav();
    assert_eq!(
        press(&mut menu, &[Button::Cross]),
        [MenuEvent::Fired(Action::OpenMultiplayerLocal)]
    );
    // LAN opens its own page, whose rows are the two LAN actions.
    press(&mut menu, &[Button::Down]);
    assert_eq!(menu.selected(), 1);
    assert_eq!(menu.page().entries[1].label(), "LAN");
    assert!(
        !menu.is_disabled(&menu.page().entries[1]),
        "LAN is real now"
    );
    assert!(
        press(&mut menu, &[Button::Cross]).is_empty(),
        "it opens a page"
    );
    assert_eq!(menu.page().id, "lan");
    let lan: Vec<Option<Action>> = menu
        .page()
        .entries
        .iter()
        .map(|entry| match entry {
            Entry::Run { action, .. } => Some(*action),
            _ => None,
        })
        .collect();
    assert_eq!(
        lan,
        [Some(Action::HostLan), Some(Action::JoinLan), None],
        "HOST GAME and JOIN GAME"
    );
    press(&mut menu, &[Button::Circle]);
    assert_eq!(menu.page().id, "multiplayer");
    // ONLINE is real: it fires its action, which opens the lobby connection.
    press(&mut menu, &[Button::Down]);
    assert_eq!(menu.selected(), 2);
    let entry = &menu.page().entries[2];
    assert_eq!(entry.label(), "ONLINE");
    assert!(!menu.is_disabled(entry), "ONLINE is not dimmed");
    assert_eq!(
        press(&mut menu, &[Button::Cross]),
        [MenuEvent::Fired(Action::OpenOnline)]
    );
}

#[test]
fn the_online_page_has_create_and_join() {
    let mut menu = Menu::new(built_in());
    menu.open("online");
    let actions: Vec<Option<Action>> = menu
        .page()
        .entries
        .iter()
        .map(|entry| match entry {
            Entry::Run { action, .. } => Some(*action),
            _ => None,
        })
        .collect();
    assert_eq!(
        actions,
        [Some(Action::OnlineCreate), Some(Action::OnlineJoin), None]
    );
}
