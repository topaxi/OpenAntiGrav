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
    /// Builds the menus again where the player stood when what the rows say has changed
    /// (a scan landed, a source appeared): the page's rows are made when the model is.
    pub(crate) fn poll_music_pool(&mut self) {
        let Stage::Menu(stage) = &self.stage else {
            return;
        };
        let pool = self.music_discs.pool();
        let signature = music_pool::signature(pool);
        if signature == self.music_pool_seen && music_pool::is_current(&stage.menu, pool) {
            return;
        }
        self.music_pool_seen = signature;
        let Some(shell) = self.shell.clone() else {
            return;
        };
        let trail = stage.menu.trail();
        let held = oag_game::mods::selection(&stage.menu);
        let skin = stage.skin;
        let mut model = self.menu_model(&shell, &skin);
        if !model.restore_trail(&trail) {
            return;
        }
        if let Some(id) = held {
            oag_game::mods::reseat(&mut model, &id, &shell.mods, &shell.strings);
        }
        model.settle();
        if let Stage::Menu(stage) = &mut self.stage {
            stage.menu = model;
        }
    }

    /// A source's row moved to `on`: puts the source into the playlist or takes it out,
    /// and saves.
    pub(crate) fn switch_music_source(&mut self, id: &str, on: bool) {
        let pool = self.music_discs.pool().clone();
        let known = pool
            .rows()
            .into_iter()
            .find(|row| row.id == id)
            .is_some_and(|row| matches!(row.status, oag_sound::pool::SourceStatus::Ready(_)));
        if !known {
            return;
        }
        info!("music source {id}: turned {} on the MUSIC page", if on { "on" } else { "off" });
        pool.set_enabled(id, on);
        self.settings.audio.music_switches = pool.switches();
        if let Err(e) = oag_game::settings::save(&self.settings) {
            error!("could not save settings: {e:#}");
        }
        self.audio.music_pool_changed();
    }
}
