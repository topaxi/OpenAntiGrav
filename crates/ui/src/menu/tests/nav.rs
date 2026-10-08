//! The navigation sounds a menu asks for: one per cursor move, page change,
//! value step and refusal, drained with [`Menu::take_nav`].

use super::*;
use crate::menu::nav::Nav;

fn navs(menu: &mut Menu, buttons: &[Button]) -> Vec<Nav> {
    press(menu, buttons);
    menu.take_nav()
}

#[test]
fn a_walk_through_the_menus_asks_for_the_cues_the_original_plays() {
    let mut menu = Menu::new(fixture());
    assert_eq!(navs(&mut menu, &[Button::Down]), [Nav::UpDown]);
    assert_eq!(navs(&mut menu, &[Button::Up]), [Nav::UpDown]);
    assert_eq!(navs(&mut menu, &[Button::Cross]), [Nav::Accept], "OPTIONS");
    assert_eq!(menu.page().id, "options");
    assert_eq!(navs(&mut menu, &[Button::Right]), [Nav::LeftRight]);
    assert_eq!(navs(&mut menu, &[Button::Down]), [Nav::UpDown]);
    assert_eq!(
        navs(&mut menu, &[Button::Cross]),
        [Nav::LeftRight],
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
            "version = 1\nroot = \"main\"\n[[page]]\nid = \"main\"\n\
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
