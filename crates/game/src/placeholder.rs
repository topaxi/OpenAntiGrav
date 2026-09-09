//! A menu presentation for a title with no front end recovered at all.
//!
//! Wipeout 2048 is the first title in the corpus whose front end is not
//! merely undeclared but **unopened** - see `oag_2048`'s own module docs.
//! `oag_title::FrontEnd` has no state for that: both halves of it, the
//! layout numbers and the boot chain, are read off a disc's own XML, and
//! `oag_title::BootProfile::provenance` only ever says `Measured` or
//! `Declared` - both a claim that *some* front-end data was read. Filling
//! either in for 2048 would be exactly what `CLAUDE.md`'s "never invent what
//! the assets already author" forbids, so `oag_2048::TITLE`'s `front_end`
//! stays `None` and must go on staying `None`.
//!
//! What [`crate::menu`] draws is not disc content, though - see its own module
//! docs: the menu *tree* (`assets/ui/menu.toml`) is this project's own, not
//! the disc's, on every title already. Only its *presentation* - a
//! `MenuSkin`'s numbers - is measured per title. So a title with no front end
//! can still show this project's own menu tree; it just cannot do so in a
//! layout attributed to its own disc, because none was read.
//!
//! [`MENU_SKIN`] is that layout: plain, round numbers chosen for legibility
//! and nothing else, carried by `oag-game`'s composition root rather than by
//! any title package - so it is never mistaken for a measurement the way a
//! `MenuSkin` borrowed from Pulse or Pure would be. See the `[[bin]]` crate's
//! `session::placeholder` for where this is used and what it stands in for.

use oag_title::MenuSkin;

/// A menu layout that belongs to no disc.
///
/// Every optional field is `None`: [`crate::menu::Skin`] already has its own
/// fallback for a title that measured nothing on a given axis, so leaving
/// these unset draws with exactly the numbers this build already uses when a
/// *known* title's `Skin.xml` is silent on one of them, rather than a second,
/// parallel set of fallbacks invented here.
pub const MENU_SKIN: MenuSkin = MenuSkin {
    // The PSP's own grid, and the one the renderer defaults to with no
    // source's own space to set instead - see `oag_display::space::Space`.
    space: (480.0, 272.0),
    menu_x: 40.0,
    menu_scale: 1.0,
    title_x: 40.0,
    title_y: 8.0,
    title_scale: 1.0,
    first_row_y: None,
    row_extra_leading: None,
    menu_font: None,
    text: None,
    title: None,
    background: None,
    selected: None,
    selected_pulse_period_secs: None,
    transition_secs: 0.5,
    strip: None,
};
