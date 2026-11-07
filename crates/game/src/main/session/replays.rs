//! The session's half of the REPLAYS page: the list on the page, KEEP and DELETE, keeping a
//! recorded race when the game ends under it, and (below) opening and watching one.

use log::{error, info};
use oag_game::replays::{self, page};
use oag_ui::menu;

use crate::stage::Stage;

use super::{Session, Shell};

/// [`Session::replay_context`] from its parts, for a caller holding another field of the session
/// mutably.
pub(crate) fn context<'a>(
    shell: &'a Shell,
    refused: &'a std::collections::BTreeMap<std::path::PathBuf, String>,
    recording: bool,
) -> page::Context<'a> {
    page::Context {
        title: shell.title.name,
        strings: &shell.strings,
        modes: &shell.modes,
        circuit: Box::new(|mode, entry| {
            shell
                .tracks_for(mode)
                .iter()
                .find(|(track, _)| track.entry_name() == entry)
                .map(|(_, name)| name.clone())
        }),
        refused,
        recording,
    }
}

impl Session {
    /// Writes the recordings of the race on screen and of the one parked behind the menus: the
    /// process is ending, and a stage that is never dropped writes nothing else.
    pub(crate) fn keep_replays(&mut self) {
        if let Stage::Race(stage) = &mut self.stage {
            stage.write_auto_replay();
        }
        if let Some(stage) = self.suspended_race.as_mut() {
            stage.write_auto_replay();
        }
    }

    /// Reads the files again and returns the REPLAY row's choices for them.
    pub(crate) fn replay_choices(&mut self, shell: &Shell) -> Vec<menu::Choice> {
        self.replay_list = replays::list();
        let cx = self.replay_context(shell);
        page::choices(&self.replay_list, &cx)
    }

    /// What the page says about the files, in the booted title's words.
    pub(crate) fn replay_context<'a>(&'a self, shell: &'a Shell) -> page::Context<'a> {
        context(shell, &self.replay_refused, self.settings.replay.record)
    }

    /// The file the REPLAY row is on.
    fn selected_replay(&self) -> Option<replays::Entry> {
        let Stage::Menu(stage) = &self.stage else {
            return None;
        };
        page::selected(&stage.menu, &self.replay_list).cloned()
    }

    /// The KEEP row: copies the automatic file the REPLAY row is on into `saved/`, and puts the
    /// row on the copy.
    pub(crate) fn keep_selected_replay(&mut self) {
        let (Some(entry), Some(root)) = (self.selected_replay(), replays::dir()) else {
            return;
        };
        match replays::keep_in(&root, &entry) {
            Ok(to) => {
                info!("replays: kept {} as {}", entry.path.display(), to.display());
                let key = format!("{}/{}", "saved", to.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned()));
                self.resupply_replays(Some(&key));
            }
            Err(e) => error!("replays: could not keep {}: {e:#}", entry.path.display()),
        }
    }

    /// The DELETE row: removes the file the REPLAY row is on.
    pub(crate) fn delete_selected_replay(&mut self) {
        let Some(entry) = self.selected_replay() else {
            return;
        };
        match replays::delete(&entry) {
            Ok(()) => {
                info!("replays: deleted {}", entry.path.display());
                self.replay_refused.remove(&entry.path);
                self.resupply_replays(None);
            }
            Err(e) => error!("replays: could not delete {}: {e:#}", entry.path.display()),
        }
    }

    /// The page's rows again after the files changed under it.
    fn resupply_replays(&mut self, select: Option<&str>) {
        let Some(shell) = self.shell.clone() else {
            return;
        };
        let choices = self.replay_choices(&shell);
        if let Stage::Menu(stage) = &mut self.stage {
            stage.menu.supply(menu::ValueSource::Replays, &choices);
            if let Some(key) = select {
                stage.menu.seed(page::SELECTED, &menu::Value::Text(key.to_string()));
            }
        }
    }
}

impl Session {
    /// The WATCH row: opens the replay the REPLAY row is on.
    pub(crate) fn watch_selected_replay(&mut self) {
        let Some(entry) = self.selected_replay() else {
            return;
        };
        info!("replays: WATCH {}", entry.path.display());
    }
}
