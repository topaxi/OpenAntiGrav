//! Offering, fetching and cancelling the pinned `ffmpeg` (`docs/architecture/ffmpeg.md`).
//!
//! The offer is a yes/no prompt over the menus, made once per run and only when the player has
//! no `ffmpeg` at all (`oag_ffmpeg::wants_download`): a system tool on `PATH` is never offered
//! a replacement. A yes is stored as the standing answer and the download runs on its own
//! thread while a status line sits over the menus; a no is stored too and ends the asking until
//! the AUDIO page's DOWNLOAD FFMPEG row asks again.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use log::{debug, info, warn};
use oag_ffmpeg::{Pin, Progress};
use oag_ui_screens::prompt::Confirm;

use crate::ffmpeg_prompt::{prompt_labels, status_text};

use crate::app::overlay::{Prompt, Purpose};
use crate::app::stage::Stage;
use crate::settings::ffmpeg::Download;

use super::Session;

/// One download in flight.
#[derive(Debug)]
pub(crate) struct Fetch {
    task: oag_thread::Task<anyhow::Result<PathBuf>>,
    progress: Arc<Progress>,
}

/// What the session remembers about `ffmpeg` between ticks.
#[derive(Debug, Default)]
pub(crate) struct State {
    /// The offer has been made (or ruled out) this run.
    asked: bool,
    /// The download, while it runs.
    fetch: Option<Fetch>,
}

impl State {
    /// Whether a download is running, which freezes the menus behind its status line.
    pub(crate) fn fetching(&self) -> bool {
        self.fetch.is_some()
    }
}

impl Session {
    /// Every menu tick: finishes a download that ended, cancels one the player backed out of,
    /// and makes the offer once.
    pub(crate) fn sync_ffmpeg(&mut self, pointer: &oag_ui::pointer::Pointer) {
        let back = self.roles().back;
        if let Some(fetch) = &mut self.ffmpeg.fetch {
            let cancel = self.controls.buttons_mut().take(back) || pointer.back;
            if cancel {
                fetch.progress.cancel.store(true, Ordering::Relaxed);
            }
            if fetch.task.is_finished() {
                let result = fetch.task.join();
                self.ffmpeg.fetch = None;
                match result {
                    Some(Ok(Ok(dir))) => info!("ffmpeg installed in {}", dir.display()),
                    Some(Ok(Err(why))) if why.downcast_ref::<oag_ffmpeg::Cancelled>().is_some() => {
                        info!("the ffmpeg download was cancelled");
                    }
                    Some(Ok(Err(why))) => warn!(
                        "ffmpeg could not be installed ({why:#}); music and movies stay off \
                         until it is"
                    ),
                    _ => warn!("the ffmpeg download stopped unexpectedly"),
                }
            }
            return;
        }
        if self.ffmpeg.asked || !self.interactive_launch || self.demo.cli == Some(true) {
            return;
        }
        let Stage::Menu(stage) = self.stack.top() else {
            return;
        };
        if stage.prompt.is_some() || self.shell.is_none() {
            return;
        }
        self.ffmpeg.asked = true;
        if !oag_ffmpeg::wants_download() {
            return;
        }
        match self.settings.ffmpeg.download {
            Download::Never => {
                debug!("ffmpeg is missing and the download is turned off in the settings");
            }
            Download::Yes => self.start_ffmpeg_fetch(),
            Download::Ask => self.open_ffmpeg_prompt(),
        }
    }

    /// The AUDIO page's DOWNLOAD FFMPEG row: asks again whatever was answered before.
    pub(crate) fn offer_ffmpeg(&mut self) {
        if self.ffmpeg.fetching() || Pin::current().is_none() {
            return;
        }
        self.open_ffmpeg_prompt();
    }

    fn open_ffmpeg_prompt(&mut self) {
        let Some(pin) = Pin::current() else {
            return;
        };
        let labels = prompt_labels(pin.megabytes(), |id, english| self.say(id, english));
        if let Stage::Menu(stage) = self.stack.top_mut() {
            stage.prompt = Some(Prompt::asking(Purpose::FetchFfmpeg, Confirm::new(labels)));
        }
    }

    /// The prompt was answered: a yes starts the download, either answer is stored.
    pub(crate) fn ffmpeg_answered(&mut self, yes: bool) {
        self.settings.ffmpeg.download = if yes { Download::Yes } else { Download::Never };
        if let Err(e) = self.save_settings() {
            warn!("could not save the ffmpeg answer: {e:#}");
        }
        if yes {
            self.start_ffmpeg_fetch();
        } else {
            info!("ffmpeg will not be downloaded; the AUDIO page can ask again");
        }
    }

    fn start_ffmpeg_fetch(&mut self) {
        let Some(pin) = Pin::current() else {
            return;
        };
        let root = oag_source::cache::default_ffmpeg_dir();
        info!(
            "downloading ffmpeg {} ({} MB) from {}",
            pin.version,
            pin.megabytes(),
            pin.url
        );
        let progress = Arc::new(Progress::default());
        progress.total.store(pin.size, Ordering::Relaxed);
        let shared = Arc::clone(&progress);
        let task = oag_thread::Task::spawn("ffmpeg-download", move || {
            oag_ffmpeg::install(pin, &root, &shared)
        });
        self.ffmpeg.fetch = Some(Fetch { task, progress });
    }

    /// The status line over the menus while a download runs.
    pub(crate) fn ffmpeg_status(&self) -> Option<String> {
        let fetch = self.ffmpeg.fetch.as_ref()?;
        Some(status_text(
            fetch.progress.done.load(Ordering::Relaxed),
            fetch.progress.total.load(Ordering::Relaxed),
            |id, english| self.say(id, english),
        ))
    }
}
