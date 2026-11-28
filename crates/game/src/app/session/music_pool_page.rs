//! The MUSIC page's rows: which sources the race playlist draws from.
//!
//! The pool and its scans are `oag_sound::pool`; the rows are `crate::music_pool`. A switch
//! is saved into the open title's `music_switches` and the pool follows at once: what was decoded ahead
//! is dropped ([`oag_sound::Audio::music_pool_changed`]), the song playing goes on.

use log::{error, info};

use crate::music_pool;

use crate::app::stage::Stage;

use super::Session;

impl Session {
    /// Which pressing's soundtrack the open title plays.
    pub(crate) fn music_source(&self) -> oag_sound::MusicSource {
        self.settings.music_source(self.title_raced())
    }

    /// Builds the menus again where the player stood when what the rows say has changed
    /// (a scan landed, a source appeared): the page's rows are made when the model is.
    pub(crate) fn poll_music_pool(&mut self) {
        let Stage::Menu(stage) = self.stack.top() else {
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
        let Some((mut model, true)) = self.model_where_stood(&shell) else {
            return;
        };
        model.settle();
        if let Stage::Menu(stage) = self.stack.top_mut() {
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
        info!(
            "music source {id}: turned {} on the MUSIC page",
            if on { "on" } else { "off" }
        );
        pool.set_enabled(id, on);
        if let Some(title) = self.title_raced() {
            self.settings
                .music_profile_mut(title)
                .music_switches
                .insert(id.to_string(), on);
        }
        if let Err(e) = self.save_settings() {
            error!("could not save settings: {e:#}");
        }
        self.audio.music_pool_changed();
    }
}
