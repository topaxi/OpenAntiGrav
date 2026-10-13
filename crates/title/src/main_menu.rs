//! A title's own main menu, as its disc authors it: which rows, in what order,
//! which of them this build has something behind, and how a row attached to one
//! of them is drawn. Per-title behaviour as data ([ADR-0058]) and the vanilla
//! half of the menu/mod model ([ADR-0066]).
//!
//! The rows are **read off the disc** (`MainMenu_Definition.xml`'s `<Menu>` or
//! `<HorizMenu>`, `Skin.xml`'s `Main Menu` on Pure, `NEWGUI/Definition.xml`'s
//! `GameModeChoice` on 2048), in the order the file lists them, by the
//! `idstring` the file spells. Each ground-truth test that asserts a list here
//! reads the same file back and compares the whole list, so a row dropped or
//! reordered fails by name.
//!
//! [`OriginalRow::builds`] is the one thing that is ours: which row of
//! `assets/ui/menu.toml`'s `main` page that original row opens. `None` means
//! this build has nothing behind it (Multiplayer, Profile, an online mode, a
//! website) and the row is not drawn; it stays in the list so the list is the
//! disc's, and the report can say what is missing.
//!
//! [ADR-0058]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0058-per-title-behaviour-is-title-data-with-provenance.md
//! [ADR-0066]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0066-the-menu-is-the-disc-plus-mods.md

use crate::effects::Origin;

/// One row of the original's main menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OriginalRow {
    /// The `idstring` the disc spells.
    pub disc_id: &'static str,
    /// The `id` of the `menu.toml` main-page row this opens, or `None` where
    /// this build has nothing behind it.
    pub builds: Option<&'static str>,
}

/// The `menu.toml` main-page id of the vertical split-screen row.
pub const SPLIT_VERTICAL: &str = "split_vertical";

/// The `menu.toml` main-page id of the horizontal split-screen row.
pub const SPLIT_HORIZONTAL: &str = "split_horizontal";

/// A row this build shows that the pressing the title's rows were read from does not author,
/// placed right after one of its rows.
///
/// The title's [`MainMenu::rows`] are the disc's own and a ground-truth test compares them
/// whole, so a row some other pressing authors cannot go in that list. Wipeout Pulse's PS2
/// pressing authors `FE_RACE_CAM_SS` and `FE_RACE_CAM_SSH` after `FE_RACE_CAM`; the PSP
/// pressing authors neither, and Pure has no split screen at all. **The PS2's design, borrowed**
/// (the maintainer's rule, 2026-10-13: anything for Pulse is the same on both pressings, and a
/// title with no split screen mirrors its closest relative). A pressing that authors the row
/// itself (`pulses_ps2_main_menu_puts_the_borrowed_rows_where_the_disc_does`) is the proof of
/// the placement, so it carries no confidence score on the pressings that borrow it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Borrowed {
    /// The `disc_id` of the [`OriginalRow`] this one follows.
    pub after: &'static str,
    /// The `idstring` the pressing that authors the row spells.
    pub disc_id: &'static str,
    /// The `id` of the `menu.toml` main-page row this opens.
    pub builds: &'static str,
}

/// How a row a mod attaches to another is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attached {
    /// Inline in the list, right after its parent. A flat-list front end: Pulse,
    /// Pure, 2048.
    Inline,
    /// Below its parent, shown only while the parent or one of its attached rows
    /// has focus. A horizontal top level with no room for more tabs: HD, Omega.
    /// **Chosen, not measured**: the discs author no second level on the main
    /// menu.
    Revealed,
}

/// A title's main menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainMenu {
    /// The rows, in the disc's order.
    pub rows: &'static [OriginalRow],
    /// Rows another pressing authors and this build shows here too, each placed after one
    /// of `rows` ([`Borrowed`]). Empty where the title has none.
    pub borrowed: &'static [Borrowed],
    /// The list when an update the title ships is mounted, where it differs (2048's
    /// v1.04 patch authors two more `GameModeChoice` tiles). `None` where it does
    /// not change.
    pub updated_rows: Option<&'static [OriginalRow]>,
    /// How attached rows are drawn. **Chosen** for the title; the layout it follows
    /// (a strip or a column) is measured in [`crate::MenuSkin`].
    pub attached: Attached,
    /// Where `rows` was read.
    pub origin: Origin,
}

impl MainMenu {
    /// The rows to use, given whether the title's update is mounted.
    #[must_use]
    pub fn rows_for(&self, updated: bool) -> &'static [OriginalRow] {
        match (updated, self.updated_rows) {
            (true, Some(rows)) => rows,
            _ => self.rows,
        }
    }

    /// The `menu.toml` ids this title shows as its own, in the disc's order, each
    /// [`Borrowed`] row right after the row it follows.
    pub fn built(&self, updated: bool) -> impl Iterator<Item = &'static str> {
        self.rows_for(updated).iter().flat_map(|row| {
            let own = row.builds;
            let borrowed = self
                .borrowed
                .iter()
                .filter(|borrowed| borrowed.after == row.disc_id)
                .map(|borrowed| borrowed.builds);
            own.into_iter().chain(borrowed)
        })
    }

    /// Whether this title shows the two split-screen rows in its own main menu, so the
    /// generic SPLIT SCREEN row of the built-in extras has no place beside them.
    #[must_use]
    pub fn has_split_rows(&self) -> bool {
        self.borrowed
            .iter()
            .any(|row| row.builds == SPLIT_VERTICAL || row.builds == SPLIT_HORIZONTAL)
    }

    /// The disc rows this build has nothing behind.
    pub fn not_built(&self, updated: bool) -> impl Iterator<Item = &'static str> {
        self.rows_for(updated)
            .iter()
            .filter(|row| row.builds.is_none())
            .map(|row| row.disc_id)
    }
}

/// An [`OriginalRow`], for a title package's table.
#[must_use]
pub const fn original(disc_id: &'static str, builds: Option<&'static str>) -> OriginalRow {
    OriginalRow { disc_id, builds }
}
