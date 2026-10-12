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

    /// The `menu.toml` ids this title shows as its own, in the disc's order.
    pub fn built(&self, updated: bool) -> impl Iterator<Item = &'static str> {
        self.rows_for(updated).iter().filter_map(|row| row.builds)
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
