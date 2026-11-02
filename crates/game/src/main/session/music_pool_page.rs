//! The MUSIC page's rows: which sources the race playlist draws from.
//!
//! The pool and its scans are `oag_sound::pool`; the rows are `oag_game::music_pool`. A switch
//! is saved into `[audio] music_switches` and the pool follows at once: what was decoded ahead
//! is dropped ([`oag_sound::Audio::music_pool_changed`]), the song playing goes on.

use log::{error, info};

use oag_game::music_pool;

use crate::stage::Stage;

use super::Session;

impl Session {
    /// Supplies the SOURCE and ENABLED rows from the pool as it stands.
    pub(crate) fn resupply_music_pool(&mut self) {
        let Some(shell) = self.shell.as_ref() else {
            return;
        };
        if let Stage::Menu(stage) = &mut self.stage {
            music_pool::supply(&mut stage.menu, self.music_discs.pool(), &shell.strings);
        }
    }

    /// Keeps the rows current while the page is open: a folder's scan answers on its own
    /// thread, so a source can turn from SCANNING to usable under the player's cursor.
    pub(crate) fn poll_music_pool(&mut self) {
        let on_page = matches!(&self.stage, Stage::Menu(stage) if stage.menu.page().id == music_pool::PAGE);
        if on_page {
            self.resupply_music_pool();
        }
    }

    /// The ENABLED row moved to `on`: puts the source the SOURCE row is on into the playlist
    /// or takes it out, and saves.
    pub(crate) fn switch_selected_music(&mut self, on: bool) {
        let (Some(_), Stage::Menu(stage)) = (self.shell.as_ref(), &self.stage) else {
            return;
        };
        let pool = self.music_discs.pool().clone();
        let rows = pool.rows();
        let Some(row) = music_pool::selected(&stage.menu, &rows) else {
            return;
        };
        if row.enabled == on || !matches!(row.status, oag_sound::pool::SourceStatus::Ready(_)) {
            self.resupply_music_pool();
            return;
        }
        let id = row.id.clone();
        info!("music source {id}: turned {} on the MUSIC page", if on { "on" } else { "off" });
        pool.set_enabled(&id, on);
        self.settings.audio.music_switches = pool.switches();
        if let Err(e) = oag_game::settings::save(&self.settings) {
            error!("could not save settings: {e:#}");
        }
        self.audio.music_pool_changed();
        self.resupply_music_pool();
    }
}
