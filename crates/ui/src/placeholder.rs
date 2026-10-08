//! A menu presentation for a title with no `MenuSkin`-shaped menu to draw.
//!
//! Two distinct titles can land here, and [ADR-0054] is what tells them
//! apart. A title whose front end is unopened entirely -
//! `oag_title::Title::front_end` itself `None` - has no state to read a
//! layout from at all. Wipeout 2048, since this front end was wired, is the
//! other case: `oag_title::Title::front_end` is `Some`, its boot chain and
//! language plugins are real and read, but `oag_title::FrontEnd::menu` stays
//! `None` because 2048 authors no `FEGlobals`/`<Menu>`/`<HorizMenu>`
//! vocabulary anywhere - a touch-icon grid instead
//! (`oag_title::FrontEnd::touch`, `docs/formats/2048-frontend.md`). Filling
//! `menu` in for either case with another title's numbers, or with
//! placeholder numbers dressed as measurements, would be exactly what
//! `CLAUDE.md`'s "never invent what the assets already author" forbids -
//! so both stay `None`. Since 2026-09-21 `oag_game::boot::load_shell` takes
//! [`MENU_SKIN`] for a title whose `menu` is `None`, so 2048's boot walks
//! its own chain and draws its own grids (`crate::frontend::touch`) with
//! this layout standing in only for the shell's own menus, which that boot
//! never opens; the composition root's `session::placeholder` route is what
//! a title with *no* front end still falls into.
//!
//! What [`crate::menu`] draws is not disc content, though - see its own module
//! docs: the menu *tree* (`assets/ui/menu.toml`) is this project's own, not
//! the disc's, on every title already. Only its *presentation* - a
//! `MenuSkin`'s numbers - is measured per title. So a title with no
//! `MenuSkin`-shaped menu can still show this project's own menu tree; it
//! just cannot do so in a layout attributed to its own disc, because none was
//! read (or, for 2048, because none exists in that vocabulary to read).
//!
//! [ADR-0054]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0054-a-touch-front-end-is-a-second-axis-not-a-menuskin-variant.md
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
    title_font: None,
    body_font: None,
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
    blocks: None,
    list: None,
    settings: None,
    help_text: None,
};
