//! The DOWNLOAD FFMPEG row, which is on the AUDIO page only while there is something to fetch.

use super::{Action, Definition, Entry};

impl Definition {
    /// Drops the DOWNLOAD FFMPEG row unless `offered` (no `ffmpeg` anywhere and a build to fetch
    /// for this OS). Decided when the tree is built, so a row that was on the page stays until
    /// the menus are next built; it is not parked, because a parked row returns to an index
    /// that a later page edit (the MUSIC row leaving) has made stale.
    pub fn drop_unavailable_fetch_ffmpeg(&mut self, offered: bool) {
        if offered {
            return;
        }
        for page in &mut self.pages {
            page.retain_rows(|entry| {
                !matches!(
                    entry,
                    Entry::Run {
                        action: Action::FetchFfmpeg,
                        ..
                    }
                )
            });
        }
    }
}
