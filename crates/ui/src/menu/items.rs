//! A list page: one row per item, the item's on/off value on the right
//! ([`Page::item_page`](super::Page::item_page)). MODS is one.
//!
//! On such a page a `choice` row is an item, not a setting to step:
//!
//! - **accept opens** the item ([`MenuEvent::OpenItem`], its row id; the owner opens the page
//!   `item_page` names),
//! - **Square steps** its value in place, with the footer's Square prompt reading the words
//!   [`Menu::set_item_toggle_label`] gave (left and right step it too, as on any value row),
//! - **rows holding `on` are listed first**, each group in the order the rows were built in,
//!   and that is decided when the page is *entered*: a toggle never moves a row under the
//!   cursor. [`Menu::item_orders`] carries the order on screen over a rebuilt tree.

use super::{Entry, Menu, MenuEvent, Value};

/// The row ids of every list page's items, in the order they are on screen, by page id.
pub type ItemOrders = Vec<(String, Vec<String>)>;

/// Whether `entry` holds the `on` value, which sorts first.
fn is_on(entry: &Entry) -> bool {
    matches!(entry.chosen(), Some(Value::Text(value)) if value == "on")
}

impl Menu {
    /// The words of the footer's Square prompt on a list page's item that can be toggled.
    pub fn set_item_toggle_label(&mut self, label: &str) {
        label.clone_into(&mut self.item_toggle_label);
    }

    /// Whether `row` of the page on screen is an item of a list page.
    #[must_use]
    pub fn is_item(&self, row: usize) -> bool {
        self.page().item_page.is_some()
            && matches!(self.page().entries.get(row), Some(Entry::Choice { .. }))
    }

    /// Whether `row` is an item with a value to step to.
    #[must_use]
    pub fn can_toggle(&self, row: usize) -> bool {
        self.is_item(row)
            && matches!(self.page().entries.get(row),
                Some(entry @ Entry::Choice { values, .. })
                    if values.len() > 1 && !self.is_disabled(entry))
    }

    /// The Square prompt's words while the selected row can be toggled.
    pub(super) fn item_toggle_offered(&self) -> Option<&str> {
        (!self.item_toggle_label.is_empty() && self.can_toggle(self.selected()))
            .then_some(self.item_toggle_label.as_str())
    }

    /// Accept on an item: the event naming it, `None` on any other row.
    pub(super) fn open_item(&self) -> Option<MenuEvent> {
        let row = self.selected();
        if !self.is_item(row) {
            return None;
        }
        let id = self.page().meta.get(row)?.id.clone()?;
        Some(MenuEvent::OpenItem(id))
    }

    /// Lists the items holding `on` first, each group in the order the rows were built in
    /// ([`Self::item_base`]), the cursor staying on its item. Rows that are not items (BACK)
    /// keep to the end.
    pub(super) fn sort_items(&mut self, at: usize) {
        if self.definition.pages[at].item_page.is_none() {
            return;
        }
        let page = &self.definition.pages[at];
        let built = self.item_base.iter().find(|(id, _)| *id == page.id);
        let mut order: Vec<usize> = (0..page.entries.len()).collect();
        order.sort_by_key(|&row| match &page.entries[row] {
            entry @ Entry::Choice { .. } => {
                let place = built.and_then(|(_, ids)| {
                    let id = page.meta[row].id.as_ref()?;
                    ids.iter().position(|held| held == id)
                });
                (u8::from(!is_on(entry)), place.unwrap_or(usize::MAX))
            }
            _ => (2, usize::MAX),
        });
        self.reorder(at, &order);
    }

    /// Rows `order[0]`, `order[1]`, ... of page `at`, in that order.
    fn reorder(&mut self, at: usize, order: &[usize]) {
        let held = self.cursor[at];
        let page = &mut self.definition.pages[at];
        page.entries = order.iter().map(|&row| page.entries[row].clone()).collect();
        page.subtitles = order
            .iter()
            .map(|&row| page.subtitles[row].clone())
            .collect();
        page.meta = order.iter().map(|&row| page.meta[row].clone()).collect();
        self.cursor[at] = order.iter().position(|&row| row == held).unwrap_or(0);
        self.focus[at] = (0..order.len())
            .map(|row| self.focus_target(at, row))
            .collect();
        self.pointed[at] = None;
    }

    /// The order the items of every list page are in.
    #[must_use]
    pub fn item_orders(&self) -> ItemOrders {
        self.definition
            .pages
            .iter()
            .filter(|page| page.item_page.is_some())
            .map(|page| {
                let ids = page
                    .meta
                    .iter()
                    .filter_map(|meta| meta.id.clone())
                    .collect();
                (page.id.clone(), ids)
            })
            .collect()
    }

    /// Puts each list page's items back in the order [`Self::item_orders`] recorded. An item
    /// the record lacks keeps its place after the recorded ones.
    pub fn set_item_orders(&mut self, orders: &ItemOrders) {
        for (id, ids) in orders {
            let Some(at) = self.definition.pages.iter().position(|page| &page.id == id) else {
                continue;
            };
            let page = &self.definition.pages[at];
            let mut order: Vec<usize> = (0..page.entries.len()).collect();
            order.sort_by_key(|&row| match &page.entries[row] {
                Entry::Choice { .. } => page.meta[row]
                    .id
                    .as_ref()
                    .and_then(|id| ids.iter().position(|held| held == id))
                    .unwrap_or(ids.len()),
                _ => usize::MAX,
            });
            self.reorder(at, &order);
        }
    }

    /// Puts the cursor of page `id`, on screen or not, on the row `row_id`.
    pub fn point_item(&mut self, id: &str, row_id: &str) {
        let Some(at) = self.definition.pages.iter().position(|page| page.id == id) else {
            return;
        };
        let Some(row) = self.definition.pages[at].row_with_id(row_id) else {
            return;
        };
        if self.cursor[at] != row {
            self.cursor[at] = row;
            self.pointed[at] = None;
        }
    }
}
