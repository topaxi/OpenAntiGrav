//! A screenshot of ours opened in the replay viewer: dropped on the main menu or the attract
//! demo, or named on the command line (`--replay-png`). `docs/formats/shot-png.md`, "Opening a
//! shot in the viewer".
//!
//! The PNG's `oaGs` chunk holds the run cut at the shot's tick, which is a replay file; the
//! viewer is the one the REPLAYS page opens (`session::replays`), started on that tick and
//! paused. **Every refusal is decided here, before anything leaves the screen** (the file, the
//! title, the modules), so a refused drop leaves the menu or the demo where it was and says why
//! in a [`Notice`] over it; a failure that only the load can find (the rebuild is not the
//! recorded race) comes back through [`Session::shot_failed`] to the main menu with the same
//! notice.

use crate::{replay_view, shot_meta};
use log::{debug, info};
use oag_gameplay::input::Button;
use oag_replay::Replay;
use oag_ui::frontend::Draw;
use oag_ui::pointer::Pointer;

use crate::app::stage::Stage;

use super::Session;
use super::replays::Watching;

/// How long a notice stays up on its own: eight seconds at 60 Hz.
const NOTICE_TICKS: u32 = 480;

/// The ids of the words, in `assets/ui/strings/english.toml`.
mod words {
    pub(super) const NOT_OURS: &str = "OAG_SHOT_NOT_OURS";
    pub(super) const NO_STREAM: &str = "OAG_SHOT_NO_STREAM";
    pub(super) const NO_REASON: &str = "OAG_SHOT_NO_REASON";
    pub(super) const OTHER_HASH: &str = "OAG_SHOT_OTHER_HASH";
    pub(super) const OTHER_TITLE: &str = "OAG_REPLAYS_OTHER_TITLE";
    pub(super) const NO_SETUP: &str = "OAG_REPLAYS_NO_SETUP";
    pub(super) const REFUSED: &str = "OAG_REPLAYS_REFUSED";
}

/// A line over whatever is on screen, until it times out or the player acts: a mouse click or
/// tap, a button or a key, which it spends so the press does not also act on the screen under
/// it.
#[derive(Debug)]
pub(crate) struct Notice {
    text: String,
    ticks: u32,
}

impl Notice {
    /// The line.
    pub(crate) fn text(&self) -> &str {
        &self.text
    }
}

/// Everything that starts a shot's viewer, decided and not yet started.
struct Prepared {
    options: oag_raceplay::Options,
    watching: Watching,
}

impl Session {
    /// A file dropped on the window while the main menu or an attract demo is up: opens it in
    /// the viewer if it is a screenshot of ours with a stream, and says why not if it is not.
    /// Anywhere else (a race, a page below the main menu, a prompt, the loading screen) the drop
    /// is ignored, as the original ignores every drop.
    pub(crate) fn shot_dropped(&mut self, path: &std::path::Path) {
        if !self.accepts_shot_drop() {
            debug!(
                "shot: ignoring {} dropped outside the main menu and the demo",
                path.display()
            );
            return;
        }
        let prepared = match self.prepare_shot(path) {
            Ok(prepared) => prepared,
            Err(why) => {
                info!("shot: {} not opened: {why}", path.display());
                self.notice = Some(Notice::new(why));
                return;
            }
        };
        if self.in_demo() {
            self.leave_demo_for_input();
        }
        self.begin_shot(prepared);
    }

    /// Whether a dropped screenshot is taken now: on the main menu with nothing over it, or in an
    /// attract demo that is not a wallpaper (which ignores every input).
    fn accepts_shot_drop(&self) -> bool {
        match self.stack.top() {
            Stage::Menu(_) => self.attract_eligible(),
            Stage::Race(stage) => stage.demo.is_some() && !self.demo_is_wallpaper(),
            _ => false,
        }
    }

    /// `--replay-png`: opens the file named on the command line as soon as the main menu is up.
    /// Returns whether the viewer's load began.
    pub(crate) fn tick_shot_cli(&mut self) -> bool {
        if self.shot_cli.is_none()
            || !matches!(self.stack.top(), Stage::Menu(_))
            || !self.attract_eligible()
        {
            return false;
        }
        let Some(path) = self.shot_cli.take() else {
            return false;
        };
        match self.prepare_shot(&path) {
            Ok(prepared) => {
                self.begin_shot(prepared);
                true
            }
            Err(why) => {
                info!("shot: {} not opened: {why}", path.display());
                self.notice = Some(Notice::new(why));
                false
            }
        }
    }

    /// What `path` holds to watch: a replay file (`OAGR`), played from the start, or a
    /// screenshot's stream with the tick and hash it was cut at.
    fn read_watchable(
        &self,
        path: &std::path::Path,
    ) -> Result<(Replay, Option<(u64, u64)>), String> {
        let bytes = std::fs::read(path).map_err(|_| self.project_word(words::NOT_OURS))?;
        if bytes.starts_with(oag_replay::file::MAGIC) {
            let replay = Replay::from_bytes(&bytes).map_err(|why| {
                self.project_word(words::REFUSED)
                    .replace("%s", &why.to_string())
            })?;
            return Ok((replay, None));
        }
        let found = shot_meta::read(&bytes).map_err(|_| self.project_word(words::NOT_OURS))?;
        let Some(record) = found.record else {
            let because = found
                .meta
                .map(|meta| meta.shot.no_stream_because)
                .filter(|why| !why.is_empty());
            return Err(match (found.text, because) {
                (None, _) => self.project_word(words::NOT_OURS),
                (Some(_), Some(why)) => self
                    .project_word(words::NO_STREAM)
                    .replace("%s", &why.to_uppercase()),
                (Some(_), None) => self
                    .project_word(words::NO_STREAM)
                    .replace("%s", &self.project_word(words::NO_REASON)),
            });
        };
        let mark = (record.tick, record.hash);
        Ok((record.replay, Some(mark)))
    }

    /// Reads `path` and decides whether the viewer can open it here.
    fn prepare_shot(&self, path: &std::path::Path) -> Result<Prepared, String> {
        let (replay, shot) = self.read_watchable(path)?;
        let header = &replay.header;
        let title = self.shell.as_ref().map_or("", |shell| shell.title.name);
        if !header.title.eq_ignore_ascii_case(title) {
            return Err(self
                .project_word(words::OTHER_TITLE)
                .replace("%s", &header.title.to_uppercase()));
        }
        if header.setup.is_none() {
            return Err(self.project_word(words::NO_SETUP));
        }
        let options = self.replay_options(&replay)?;
        oag_raceplay::extensions::preflight_recorded(&options)
            .map_err(|why| self.project_word(words::REFUSED).replace("%s", &why))?;
        Ok(Prepared {
            watching: Watching {
                replay: Box::new(replay),
                path: path.to_path_buf(),
                setup: None,
                shot,
                dropped: true,
            },
            options,
        })
    }

    /// Starts loading the race a prepared shot is in. The viewer opens when the load hands over.
    fn begin_shot(&mut self, prepared: Prepared) {
        let Prepared { options, watching } = prepared;
        let path = watching.path.clone();
        self.shot_pending = true;
        match self.start_watching(options, watching) {
            Ok(()) => info!("shot: opening {} in the viewer", path.display()),
            Err(why) => self.shot_failed(&why),
        }
    }

    /// A shot's viewer that could not open after it was accepted: the main menu with the reason
    /// over it.
    pub(crate) fn shot_failed(&mut self, why: &str) {
        self.shot_pending = false;
        self.notice = Some(Notice::new(
            self.project_word(words::REFUSED).replace("%s", why),
        ));
    }

    /// The first time the race on screen stands on the shot's tick: a hash that is not the
    /// shot's says so, since the picture is then this build's and not the shot's.
    pub(crate) fn shot_reached(&mut self, hash: u64, shot: u64) {
        if hash != shot {
            debug!(
                "shot: the viewer's race hashes {hash:#018x} where the shot was taken at {shot:#018x}"
            );
            // Other mods explain another hash, and name what to switch.
            let differ = (self.viewer.as_ref()).and_then(|active| active.mods_differ.as_deref());
            self.notice = Some(match differ {
                Some(differ) => Notice::mods(differ),
                None => Notice::new(self.project_word(words::OTHER_HASH)),
            });
        }
    }

    /// The project's text for `id` in the language, its English where that is untranslated.
    pub(crate) fn project_word(&self, id: &str) -> String {
        let own = oag_ui::strings::project_table(self.settings.language.as_deref());
        let english = oag_ui::strings::project_table(None);
        own.get(id)
            .or_else(|| english.get(id))
            .unwrap_or(id)
            .to_string()
    }

    /// One tick of the notice: counts it down, and ends it on a click, tap, button or key, which
    /// is spent here (`pointer` is cleared, the presses and the demo's key latch are consumed).
    pub(crate) fn tick_notice(&mut self, pointer: &mut Pointer) {
        let Some(notice) = self.notice.as_mut() else {
            return;
        };
        notice.ticks = notice.ticks.saturating_sub(1);
        let acted = pointer.clicked
            || pointer.back
            || self.demo.key
            || self.controls.buttons().pressed_mask() != 0;
        if !acted && notice.ticks > 0 {
            return;
        }
        self.notice = None;
        if acted {
            *pointer = Pointer::default();
            self.demo.key = false;
            let buttons = self.controls.buttons_mut();
            for button in SPENT {
                buttons.consume_press(button);
            }
            // The viewer and photo mode read their buttons apart from the menus', so a press
            // that dismissed a notice over the viewer is spent there too.
            for button in SPENT {
                self.controls.view_take(button);
            }
        }
    }

    /// The notice as draws over the overlay grid `grid`; empty when there is none.
    pub(crate) fn notice_screen(&self, grid: (f32, f32)) -> Vec<Draw> {
        match &self.notice {
            Some(notice) => replay_view::notice_draw(grid, notice.text()),
            None => Vec::new(),
        }
    }
}

impl Notice {
    /// The mods differ from the recording's, `differ` naming each.
    pub(crate) fn mods(differ: &str) -> Self {
        Self::new(oag_raceplay::ModsDiffer(differ.to_string()).to_string())
    }

    pub(crate) fn new(text: String) -> Self {
        Self {
            text,
            ticks: NOTICE_TICKS,
        }
    }
}

/// Every button a notice's dismissal spends.
const SPENT: [Button; 13] = [
    Button::Up,
    Button::Down,
    Button::Left,
    Button::Right,
    Button::Circle,
    Button::Cross,
    Button::Triangle,
    Button::Square,
    Button::L,
    Button::R,
    Button::Start,
    Button::Select,
    Button::Any,
];
