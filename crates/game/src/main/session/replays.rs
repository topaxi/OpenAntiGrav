//! The session's half of the REPLAYS page: the list on the page, KEEP and DELETE, keeping a
//! recorded race when the game ends under it, and (below) opening and watching one.

use std::path::PathBuf;

use anyhow::Context;
use log::{error, info};
use oag_game::replay_view::{self, View};
use oag_game::replays::{self, page};
use oag_gameplay::input::Button;
use oag_raceplay::timeline::OpenError;
use oag_raceplay::viewer::{Moved, SKIP, State, Viewer};
use oag_replay::Replay;
use oag_ui::language::StringTable;
use oag_ui::menu;
use oag_ui::pointer::Pointer;

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

/// A recording WATCH asked to open, until the race it is played in is handed over.
#[derive(Debug)]
pub(crate) struct Watching {
    /// The recording, read whole.
    pub(crate) replay: Box<Replay>,
    /// The file it came from, which the list dims if the rebuild refuses it.
    pub(crate) path: PathBuf,
    /// The setup the circuit's load produced, for the viewer's own race.
    pub(crate) setup: Option<oag_raceplay::Setup>,
}

/// The viewer while a recording is being watched.
#[derive(Debug)]
pub(crate) struct Active {
    pub(crate) viewer: Box<Viewer>,
    /// Whether the race is stepping between ticks, for the draw's blend alpha: playing, and not
    /// held at the opening, the end or the last tick that agrees.
    pub(crate) stepping: bool,
    /// Whether the effects were stopped for a speed other than 1X: back at 1X they are rebuilt
    /// from the state, not started cold.
    pub(crate) muted: bool,
    /// Whether the picture is the recorded poses: the race's HUD is not drawn (a pose track has
    /// no readouts to feed it).
    pub(crate) pose_view: bool,
    own: StringTable,
    english: StringTable,
}

impl Active {
    /// What the screen shows of the viewer now.
    fn view(&mut self) -> View {
        let viewer = &mut self.viewer;
        let state = viewer.state();
        let built = viewer.built();
        View {
            at: viewer.at(),
            ticks: viewer.ticks(),
            built,
            playing: viewer.playing(),
            speed: viewer.speed(),
            // The banner names the last second that agreed, the end of what can be played.
            mismatch: viewer.stops_matching().then_some(built).flatten(),
            at_end: matches!(state, State::Ended | State::Stopped | State::PoseEnded),
            pose_view: matches!(state, State::PoseView | State::PoseEnded),
            track_short: viewer.pose_track_is_short(),
        }
    }

    /// The project's text for `id`, its English where the language leaves it untranslated.
    fn word(&self, id: &str) -> String {
        self.own
            .get(id)
            .or_else(|| self.english.get(id))
            .unwrap_or(id)
            .to_string()
    }
}

/// Puts the viewer over the race the loader built: its timeline starts a race of its own from the
/// same setup, and a rebuild that is not the recorded race (the tick-zero hash) is refused here,
/// by name, and the file is listed dimmed with why (`refused`).
pub(crate) fn open_viewer(
    refused: &mut std::collections::BTreeMap<PathBuf, String>,
    language: Option<&str>,
    stage: &mut crate::race_stage::RaceStage,
    watching: Watching,
) -> anyhow::Result<Active> {
    let Watching { replay, path, setup } = watching;
    let setup = setup.context("the race's setup was not kept")?;
    let viewer = Viewer::open(setup, *replay).map_err(|e| {
        let why = match &e {
            OpenError::Desync(_) => page::TICK_ZERO.to_string(),
            OpenError::Start(start) => format!("{start:#}"),
        };
        refused.insert(path.clone(), why);
        anyhow::anyhow!("{} cannot be rebuilt here: {e}", path.display())
    })?;
    viewer.begin(&mut stage.race)?;
    // Nothing the race does while it is watched is a result: the finish and the exit are not
    // captured, and it is never parked behind the menus.
    stage.result_saved = true;
    stage.watching = true;
    info!("replays: watching {}", path.display());
    Ok(Active {
        viewer: Box::new(viewer),
        stepping: false,
        muted: false,
        pose_view: false,
        own: oag_ui::strings::project_table(language),
        english: oag_ui::strings::project_table(None),
    })
}

impl Session {
    /// The WATCH row: reads the recording the REPLAY row is on and loads the race it was recorded
    /// in. A file that will not read, or whose header cannot be rebuilt here, is listed dimmed
    /// with why and nothing is loaded.
    pub(crate) fn watch_selected_replay(&mut self) {
        let Some(entry) = self.selected_replay() else {
            return;
        };
        info!("replays: WATCH {}", entry.path.display());
        if let Err(why) = self.watch_path(&entry.path) {
            error!("replays: {}: {why}", entry.path.display());
            self.replay_refused.insert(entry.path.clone(), why);
            self.resupply_replays(Some(&entry.key()));
        }
    }

    /// The end-of-race menu's WATCH REPLAY row: the race just driven is written to `auto/` now
    /// (it would be on leaving) and opened in the viewer.
    pub(crate) fn watch_last_race(&mut self) {
        let Stage::Race(stage) = &mut self.stage else {
            return;
        };
        let Some(path) = stage.write_auto_replay() else {
            error!("replays: this race left no recording to watch");
            return;
        };
        info!("replays: WATCH REPLAY {}", path.display());
        if let Err(why) = self.watch_path(&path) {
            error!("replays: {}: {why}", path.display());
            self.replay_refused.insert(path, why);
        }
    }

    /// Reads the recording at `path` and starts loading the race it was made in.
    ///
    /// # Errors
    ///
    /// Why it cannot be opened: the file does not read, no disc is open, or the header cannot be
    /// rebuilt here.
    fn watch_path(&mut self, path: &std::path::Path) -> Result<(), String> {
        let replay = std::fs::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| Replay::from_bytes(&bytes).map_err(|e| e.to_string()))?;
        let home = self
            .race_options
            .as_ref()
            .ok_or_else(|| "no disc image has been chosen yet".to_string())?;
        let (source, dlc) = (home.source.clone(), home.dlc.clone());
        let mut options = oag_raceplay::Options::for_replay(&replay.header, &source)
            .map_err(|e| format!("{e:#}"))?;
        options.language = self.settings.language.clone();
        options.hud_style = self.settings.display.hud_style_choice();
        options.boost_zoom = self
            .settings
            .graphics
            .boost_zoom
            .then(oag_source::title::borrowed_zoom_ring);
        // The packs this machine has mounted, as the race box mounts them: the header's
        // `opponent_teams` may name a team only a pack ships, and a rebuild without the pack
        // fields another hull there. Whether the packs are the recording's is the tick-zero
        // hash's to say.
        options.dlc = dlc;
        if let Some(parked) = self.suspended_race.take() {
            crate::race_build::drop_off_thread(parked);
        }
        self.pending_event = None;
        let mode = options.mode;
        self.watching = Some(Watching {
            replay: Box::new(replay),
            path: path.to_path_buf(),
            setup: None,
        });
        self.return_to_replays = true;
        self.start_race_loading(options, mode, None, false)
            .map_err(|e| {
                self.watching = None;
                format!("{e:#}")
            })
    }

    /// The viewer's state for the drive's state line: the word and `at/length/built`.
    pub(crate) fn replay_names(&mut self) -> (Option<String>, Option<String>) {
        let Some(active) = self.viewer.as_mut() else {
            return (None, None);
        };
        let word = match active.viewer.state() {
            State::Opening => "opening",
            State::Ended => "ended",
            State::Stopped => "stopped",
            State::PoseView => "pose_view",
            State::PoseEnded => "pose_ended",
            State::Ready if active.viewer.playing() => "playing",
            State::Ready => "paused",
        };
        let built = active
            .viewer
            .built()
            .map_or_else(|| "-".to_string(), |built| built.to_string());
        let at = format!("{}/{}/{built}", active.viewer.at(), active.viewer.ticks());
        (Some(word.to_string()), Some(at))
    }

    /// Whether the race on screen steps between ticks, for the draw's blend alpha.
    pub(crate) fn viewer_stepping(&self) -> bool {
        self.viewer.as_ref().is_none_or(|active| active.stepping)
    }

    /// The viewer's screen as an overlay draw list in `grid`; empty when none is open.
    pub(crate) fn viewer_screen(&mut self, grid: (f32, f32)) -> Vec<oag_ui::frontend::Draw> {
        // Photo mode's screen takes its place while it is open over the viewer.
        let Some(active) = self.viewer.as_mut().filter(|_| self.photo.is_none()) else {
            return Vec::new();
        };
        let view = active.view();
        let keys = format!(
            "{} {}  {} {}  {} {}",
            self.controls.bound_keys(Button::Cross).first().map_or_else(|| "CROSS".to_string(), ToString::to_string),
            active.word(replay_view::strings::PLAY),
            "LEFT RIGHT",
            active.word(replay_view::strings::SKIP),
            "UP",
            active.word(replay_view::strings::SPEED),
        );
        let say = |id: &str| active.word(id);
        replay_view::draw_list(&view, grid, &say, &keys)
    }

    /// One tick of the viewer: the devices into the playhead, and the race stepped or sought.
    pub(crate) fn tick_viewer(&mut self, pointer: &Pointer) {
        use replay_view::Control;
        let surface = self.gpu.output.size();
        let space = oag_present::perf::grid(surface);
        let rect = (0.0, 0.0, surface.0 as f32, surface.1 as f32);
        let grid = super::pointer::in_grid(*pointer, space, rect, 1.0);
        let buttons = self.controls.buttons_mut();
        let mut asked = Asked {
            toggle: buttons.take(Button::Cross) | buttons.take(Button::Start),
            back: buttons.take(Button::Left),
            forward: buttons.take(Button::Right),
            faster: buttons.take(Button::Up) | buttons.take(Button::Down),
            leave: buttons.take(Button::Circle) | pointer.back,
            photo: buttons.take(Button::Select),
            seek: None,
            bar: None,
        };
        if pointer.clicked
            && let Some(at) = grid.at
        {
            match replay_view::control_at(space.size, at) {
                Some(Control::Play) => asked.toggle = true,
                Some(Control::Back) => asked.back = true,
                Some(Control::Forward) => asked.forward = true,
                Some(Control::Speed) => asked.faster = true,
                Some(Control::Photo) => asked.photo = true,
                Some(Control::Exit) => asked.leave = true,
                Some(Control::Bar) => asked.bar = Some(at.0),
                None => {}
            }
        }
        if asked.leave {
            self.leave_viewer();
            return;
        }
        if asked.photo {
            self.open_photo_from_viewer();
            return;
        }
        let (Some(active), Stage::Race(stage)) = (self.viewer.as_mut(), &mut self.stage) else {
            return;
        };
        let viewer = &mut active.viewer;
        if let Some(x) = asked.bar {
            let tick = replay_view::tick_at(space.size, x, viewer.ticks());
            asked.seek = Some(tick);
        }
        let mut moved = Moved::Still;
        if asked.toggle {
            moved = viewer.toggle(&mut stage.race);
        }
        if asked.faster {
            viewer.faster();
        }
        if asked.back {
            moved = viewer.skip(&mut stage.race, -(SKIP as i64));
        }
        if asked.forward {
            moved = viewer.skip(&mut stage.race, SKIP as i64);
        }
        if let Some(tick) = asked.seek {
            moved = viewer.seek(&mut stage.race, tick);
        }
        if moved == Moved::Still {
            moved = viewer.update(&mut stage.race);
        }
        let state = viewer.state();
        active.stepping = viewer.playing() && state == State::Ready;
        active.pose_view = matches!(state, State::PoseView | State::PoseEnded);
        let at = viewer.at();
        let normal = viewer.speed() == 16;
        match moved {
            // The race at its own speed sounds as a race does; at any other speed the effects
            // are muted, and back at 1X they are rebuilt (`oag_game::sound::viewer_step`).
            Moved::Stepped(ticks) => {
                // A pose track has no cues: its picture is silent.
                let normal = normal && ticks == 1 && !active.pose_view;
                oag_game::sound::viewer_step(
                    &mut self.audio,
                    &mut stage.race,
                    normal,
                    &mut active.muted,
                    at,
                );
            }
            Moved::Seeked => {
                active.muted = false;
                oag_game::sound::rebuild(&mut self.audio, &mut stage.race, at);
            }
            Moved::Still => {}
        }
    }

    /// Leaves the viewer for the REPLAYS page. Nothing was recorded, so nothing is written; the
    /// race is dropped, never parked.
    pub(crate) fn leave_viewer(&mut self) {
        if self.viewer.take().is_none() {
            return;
        }
        info!("replays: the viewer closed");
        self.return_to_replays = true;
        self.audio.pause_race_music();
        self.save_race_music();
        self.audio.stop_race_sfx();
        if let Err(error) = self.open_menus() {
            error!("cannot return to the menus: {error:#}");
            self.quit = true;
        }
    }
}

/// What the player asked of the viewer this tick.
struct Asked {
    toggle: bool,
    back: bool,
    forward: bool,
    faster: bool,
    leave: bool,
    photo: bool,
    seek: Option<u64>,
    bar: Option<f32>,
}

impl Session {
    /// Whether the race on screen is POSE VIEW, whose HUD is not drawn.
    pub(crate) fn pose_view_shown(&self) -> bool {
        self.viewer.as_ref().is_some_and(|active| active.pose_view)
    }
}
