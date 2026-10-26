//! What the menu system in [`super`] is asserted to do.
//!
//! Its own directory rather than a `#[cfg(test)]` block at the end of
//! `menu.rs`: the tests are 1,930 lines and the menu itself is 2,160, so the
//! module that had to be read to change one row was nearly half something
//! else. See `scripts/check-file-size.py`, which is the rule as a gate.
//!
//! This file holds only the fixtures the themes below share; each theme is a
//! file of its own, split along the seams the tests already had.

mod blocks;
mod drawing;
mod frame;
mod hide;
mod nav;
mod navigation;
mod pointer;
mod reveal;
mod rows;
mod settings;
mod strip;

use super::*;
use crate::frontend::Align;

/// [`draw_list`] flattened, which is what every test below wants: the
/// layer split exists for the transition, and none of these animate.
fn list(
    menu: &Menu,
    bindings: &dyn Fn(Button) -> Vec<&'static str>,
    backdrop: Option<Backdrop>,
) -> Vec<Draw> {
    draw_list(
        menu,
        &skin(),
        bindings,
        &measure,
        backdrop.map(Picture::from),
        &Frame::default(),
        false,
    )
    .flatten()
}

/// A stand-in for a real face's widths: every glyph six pixels wide.
///
/// The tests that need this are the strip's, and none of them assert a number
/// that came off a font - they assert that entry *n+1* starts a measured width
/// and one gap past entry *n*, which a flat advance states as clearly as a real
/// atlas would and without pinning this build's layout to a glyph table. The
/// same stand-in `marquee`'s own tests use, for the same reason.
fn measure(text: &str) -> f32 {
    text.chars().count() as f32 * 6.0
}

/// The skin every test below draws with: Pulse's own table against the
/// 22-pixel face its menus name. Using the shipped table rather than a
/// fixture means a change to the recovered numbers shows up here, which is
/// the point - these tests are where the layout is pinned.
fn skin() -> Skin {
    Skin::new(
        oag_pulse::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        PULSE_MENU_LINE_HEIGHT,
    )
}

/// `Pulse_20.fnt`'s line height, per `docs/formats/fnt.md` and confirmed by
/// reading it back out of `Atlas::from_font`.
const PULSE_MENU_LINE_HEIGHT: f32 = 22.0;

/// How many rows that skin fits with nothing reserved under them, which is
/// what the window logic uses on every page but AI PILOTS.
fn visible() -> usize {
    visible_rows(&skin(), &Frame::default(), false)
}

/// The definition this build actually ships. Every test that can use it
/// does, so the file is exercised rather than a fixture standing in for it.
fn built_in() -> Definition {
    Definition::built_in(&crate::language::StringTable::default())
        .expect("the built-in menu must parse")
}

/// HD's main menu the way the game builds it: its own four rows and the extras around them,
/// without the split rows Pulse and Pure show. The strip and reveal tests model HD's front
/// end, so they read this and not the title-less tree, which keeps every vanilla row.
fn built_in_hd() -> Definition {
    let extras = oag_mod::extras::builtin().unwrap();
    let text = extras.menu().unwrap().unwrap_or_default();
    let english = crate::strings::project_table(None);
    Definition::build(
        crate::menu::BUILT_IN,
        &english,
        &english,
        Some(For {
            title: oag_hd::TITLE,
            updated: false,
        }),
        &[ModMenu {
            id: oag_mod::extras::ID,
            text: &text,
        }],
    )
    .expect("HD's menu must build")
    .0
}

/// One tick with `buttons` newly down, which is what the edges above read.
fn press(menu: &mut Menu, buttons: &[Button]) -> Vec<MenuEvent> {
    let mut input = Input::new();
    let mask = buttons.iter().fold(0u32, |mask, &b| mask | 1 << b.index());
    input.begin_frame(mask);
    menu.update(&mut input)
}

fn fixture() -> Definition {
    Definition::parse(
        r#"
        toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/menu.tosd", version = "1.0.0" }
        root = "main"
        [[page]]
        id = "main"
        title = "MAIN"
        [[page.entry]]
        kind = "submenu"
        label = "OPTIONS"
        target = "options"
        [[page.entry]]
        kind = "action"
        label = "QUIT"
        action = "quit"
        [[page]]
        id = "options"
        title = "OPTIONS"
        [[page.entry]]
        kind = "choice"
        label = "FILTERING"
        setting = "graphics.anisotropy"
        values = ["off", "4x", "16x"]
        [[page.entry]]
        kind = "toggle"
        label = "SWITCH"
        setting = "graphics.switch"
        [[page.entry]]
        kind = "back"
        label = "BACK"
        "#,
        &crate::language::StringTable::default(),
    )
    .expect("the fixture must parse")
}

/// No keys bound to anything, for the layout tests: what a binding row
/// shows is `oag-input`'s business and is asserted there.
fn no_bindings(_: Button) -> Vec<&'static str> {
    Vec::new()
}
