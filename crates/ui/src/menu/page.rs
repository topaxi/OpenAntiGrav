//! One page of the menu tree: its rows and what each row carries beside the row.

use super::Entry;

/// What a row carries beside its [`Entry`]: the stable id a mod names it by, and
/// the top-level row it is attached to.
///
/// Kept parallel to [`Page::entries`], as [`Page::subtitles`] is. Ids and
/// parents are strings, never indices: dropping a row (QUIT on a phone) shifts
/// every index after it, and a link by index would silently point at the next
/// row.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RowMeta {
    /// `menu.toml`'s `id`, or a mod row's. `None` on a row nothing refers to.
    pub id: Option<String>,
    /// The id of the top-level row this one is attached to; `None` on a top-level
    /// row. A flat-list front end draws an attached row inline after its parent;
    /// an HD-style one reveals it below the parent while the parent or one of its
    /// attached rows has focus (`oag_title::Attached`).
    pub parent: Option<String>,
}

/// One page: a title and its rows.
#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    /// The id a `submenu` entry names.
    pub id: String,
    /// Heading drawn above the rows.
    pub title: String,
    /// The rows, in the order they are drawn and moved through.
    pub entries: Vec<Entry>,
    /// One row's second line, shown only while it is selected - `menu.toml`'s
    /// own `subtitle`. Same length and order as [`Self::entries`], `None`
    /// where a row named none. Kept parallel rather than folded into
    /// [`Entry`] itself so adding it did not touch that enum's six variants
    /// or their construction sites elsewhere in this crate and `oag_game`.
    /// See `docs/architecture/menus.md#a-per-row-subtitle`.
    pub subtitles: Vec<Option<String>>,
    /// Each row's id and attachment, parallel to [`Self::entries`].
    pub meta: Vec<RowMeta>,
}

impl Page {
    /// Drops rows from [`Self::entries`] and their [`Self::subtitles`] and
    /// [`Self::meta`] together, so the three never drift out of the lockstep
    /// those fields' own docs promise. Every row-dropping pass of
    /// `definition::Definition` goes through this rather than calling
    /// `entries.retain` directly, which is what a plain `retain` here would
    /// silently get wrong: it would drop a row from one vector and leave its
    /// subtitle attached to whatever row slid into its old index.
    ///
    /// A row left attached to a parent that was dropped is dropped with it: there
    /// is no row for it to appear under.
    pub(super) fn retain_rows(&mut self, mut keep: impl FnMut(&Entry) -> bool) {
        let mut kept: Vec<bool> = self.entries.iter().map(&mut keep).collect();
        let dropped: Vec<&str> = self
            .meta
            .iter()
            .zip(&kept)
            .filter(|(_, kept)| !**kept)
            .filter_map(|(meta, _)| meta.id.as_deref())
            .collect();
        for (flag, meta) in kept.iter_mut().zip(&self.meta) {
            if meta
                .parent
                .as_deref()
                .is_some_and(|parent| dropped.contains(&parent))
            {
                *flag = false;
            }
        }
        let mut flags = kept.iter().copied();
        self.entries.retain(|_| flags.next().unwrap_or(false));
        let mut flags = kept.iter().copied();
        self.subtitles.retain(|_| flags.next().unwrap_or(false));
        let mut flags = kept.iter().copied();
        self.meta.retain(|_| flags.next().unwrap_or(false));
    }

    /// The row with this id.
    #[must_use]
    pub fn row_with_id(&self, id: &str) -> Option<usize> {
        self.meta
            .iter()
            .position(|meta| meta.id.as_deref() == Some(id))
    }

    /// Whether this row is attached to another.
    #[must_use]
    pub fn is_attached(&self, row: usize) -> bool {
        self.meta.get(row).is_some_and(|meta| meta.parent.is_some())
    }

    /// The top-level row `row` belongs to: itself when it is one, its parent when
    /// attached.
    #[must_use]
    pub fn family_of(&self, row: usize) -> usize {
        self.meta
            .get(row)
            .and_then(|meta| meta.parent.as_deref())
            .and_then(|parent| self.row_with_id(parent))
            .unwrap_or(row)
    }

    /// Every row of the family led by top-level row `head`, `head` first.
    pub fn family(&self, head: usize) -> impl Iterator<Item = usize> + '_ {
        (0..self.entries.len()).filter(move |&row| self.family_of(row) == head)
    }

    /// The top-level rows, in order.
    pub fn top_level(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.entries.len()).filter(|&row| !self.is_attached(row))
    }
}
