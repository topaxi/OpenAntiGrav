//! The session's half of the REPLAYS page: the list on the page, KEEP and DELETE, keeping a
//! recorded race when the game ends under it, and (below) opening and watching one.

use std::path::PathBuf;

use crate::replay_view::{self, View};
use crate::replays::{self, page};
use anyhow::Context;
use log::{debug, error, info, warn};
use oag_gameplay::input::Button;
use oag_raceplay::timeline::OpenError;
use oag_raceplay::viewer::{Moved, SKIP, State, Viewer};
use oag_replay::Replay;
use oag_ui::hints::Entry;
use oag_ui::language::StringTable;
use oag_ui::menu;
use oag_ui::pointer::Pointer;

use crate::app::stage::Stage;

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
        // Photo mode's rewind has taken the recording out of the race, and only leaving the mode
        // puts it back: without that the race has nothing to write.
        if self
            .photo
            .as_ref()
            .is_some_and(|open| open.origin() != super::photo::Origin::Viewer)
        {
            self.leave_photo();
        }
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
                let key = format!(
                    "{}/{}",
                    "saved",
                    to.file_name()
                        .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
                );
                self.resupply_replays(Some(&key));
            }
            Err(e) => error!("replays: could not keep {}: {e:#}", entry.path.display()),
        }
    }

    /// The EXPORT row: gives the file the REPLAY row is on to the phone's Downloads folder, and
    /// says whether it went.
    pub(crate) fn export_selected_replay(&mut self) {
        use oag_mobile::export::{Error, Kind, export};
        let Some(entry) = self.selected_replay() else {
            return;
        };
        let word = match export(&entry.path, Kind::Replay) {
            Ok(uri) => {
                info!("replays: exported {} as {uri}", entry.path.display());
                "OAG_REPLAYS_EXPORTED"
            }
            Err(Error::Unsupported(why)) => {
                debug!("replays: not exported: {why}");
                "OAG_REPLAYS_EXPORT_FAILED"
            }
            Err(e) => {
                warn!("replays: could not export {}: {e}", entry.path.display());
                "OAG_REPLAYS_EXPORT_FAILED"
            }
        };
        self.notice = Some(super::shot_open::Notice::new(self.project_word(word)));
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
                stage
                    .menu
                    .seed(page::SELECTED, &menu::Value::Text(key.to_string()));
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
    /// The setups and modules the circuit's load produced, for the viewer's own race.
    pub(crate) setup: Option<(
        oag_raceplay::SimSetup,
        oag_raceplay::ViewSetup,
        oag_raceplay::extensions::Prepared,
    )>,
    /// The tick and state hash of the screenshot this recording was cut from: the viewer opens
    /// paused on that tick (`session::shot_open`).
    pub(crate) shot: Option<(u64, u64)>,
    /// Whether the file was dropped on the window or named on the command line rather than
    /// picked from the REPLAYS list: leaving the viewer returns to the main menu.
    pub(crate) dropped: bool,
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
    /// Whether the race's HUD is drawn: SQUARE turns it off for a clean picture, as it does in
    /// photo mode.
    pub(crate) hud: bool,
    /// The playhead was moved by a trigger or the right stick on the last tick, so the sound is
    /// rebuilt when it stops rather than on every frame of the scrub.
    scrubbing: bool,
    /// The tick and state hash of the screenshot the viewer was opened from, until the first
    /// time the race stands on that tick and the hash has been compared.
    pub(crate) shot: Option<(u64, u64)>,
    /// Whether the viewer was opened from a screenshot, which leaving it returns to the main
    /// menu from rather than to the REPLAYS page.
    pub(crate) from_shot: bool,
    /// The touch controls were hidden by a tap on the picture.
    chrome_hidden: bool,
    /// Ticks since a finger last touched the screen.
    idle: u32,
    /// A finger is scrubbing the bar: where it is in the grid, and the ticks it has held.
    bar_drag: Option<(f32, u32)>,
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
            // A shot's own last tick is where the viewer opened, not the end of anything: the
            // banner would cover the picture the player came for.
            at_end: matches!(state, State::Ended | State::Stopped | State::PoseEnded)
                && !(self.from_shot && !viewer.playing() && viewer.start() == Some(viewer.at())),
            pose_view: matches!(state, State::PoseView | State::PoseEnded),
            track_short: viewer.pose_track_is_short(),
        }
    }

    /// Whether the touch controls are showing.
    fn chrome_shown(&self) -> bool {
        replay_view::touch_layout::chrome_shown(
            self.chrome_hidden,
            self.viewer.playing(),
            self.idle,
        )
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
    stage: &mut crate::app::race_stage::RaceStage,
    watching: Watching,
) -> anyhow::Result<Active> {
    let Watching {
        replay,
        path,
        setup,
        shot,
        dropped,
    } = watching;
    let (setup, view, modules) = setup.context("the race's setup was not kept")?;
    // Mods switched since the recording: whatever the rebuild then says, the mod is named.
    let mods_differ =
        oag_replay::ActiveMod::differences(&replay.header.active_mods, &view.active_mods);
    let viewer = Viewer::open(setup, view, modules, *replay).map_err(|e| {
        let why = match (&e, &mods_differ) {
            (_, Some(differ)) => format!("{differ}; {e}"),
            (OpenError::NoPoseTrack(_), None) => page::NO_POSE_TRACK.to_string(),
            (OpenError::Start(start), None) => format!("{start:#}"),
        };
        // A screenshot is no row of the list: its refusal is told to the player at once.
        if shot.is_none() {
            refused.insert(path.clone(), why);
        }
        let differ = mods_differ.as_deref().map_or_else(String::new, |d| format!("{d}; "));
        anyhow::anyhow!("{} cannot be rebuilt here: {differ}{e}", path.display())
    })?;
    let viewer = match shot {
        Some((tick, _)) => viewer.starting_at(tick),
        None => viewer,
    };
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
        hud: true,
        scrubbing: false,
        shot,
        from_shot: shot.is_some() || dropped,
        chrome_hidden: false,
        idle: 0,
        bar_drag: None,
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
        let options = self.replay_options(&replay)?;
        self.return_to_replays = true;
        self.start_watching(
            options,
            Watching {
                replay: Box::new(replay),
                path: path.to_path_buf(),
                setup: None,
                shot: None,
                dropped: false,
            },
        )
    }

    /// The options that load the race `replay` was recorded in, over the disc and packs the
    /// session has open and the mods that are on.
    ///
    /// # Errors
    ///
    /// Why it cannot be rebuilt here: no disc is open, or the header does not name a race.
    pub(crate) fn replay_options(&self, replay: &Replay) -> Result<oag_raceplay::Options, String> {
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
        options.trail_hits =
            crate::launcher::trail_hits_from(self.settings.graphics.trail_hits, &self.titles);
        options.wingtip_contrails = crate::launcher::wingtip_contrails_from(
            self.settings.graphics.wingtip_contrails,
            &self.titles,
        );
        options.own_shield = self.settings.graphics.own_shield;
        options.own_rockets = self.settings.graphics.own_rockets;
        // The packs this machine has mounted, as the race box mounts them: the header's
        // `opponent_teams` may name a team only a pack ships, and a rebuild without the pack
        // fields another hull there. Whether the packs are the recording's is the tick-zero
        // hash's to say.
        options.dlc = dlc;
        // The mods that are on, for the load to pick the recording's own modules from
        // (`Options::recorded_mods`): a recording that ran a module is refused by name when
        // this machine lacks it or has another build.
        #[cfg(feature = "extensions")]
        crate::mod_extensions::attach(
            &mut options,
            &self.settings.mods,
            &self.settings.mod_settings,
            false,
        );
        Ok(options)
    }

    /// Parks nothing and starts loading the race `options` describes for the viewer to watch
    /// `watching` over.
    ///
    /// # Errors
    ///
    /// Why the load could not start.
    pub(crate) fn start_watching(
        &mut self,
        options: oag_raceplay::Options,
        watching: Watching,
    ) -> Result<(), String> {
        if let Some(parked) = self.suspended_race.take() {
            crate::app::race_build::drop_off_thread(parked);
        }
        self.pending_event = None;
        self.pending_card = None;
        let mode = options.mode;
        self.watching = Some(Box::new(watching));
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

    /// The viewer's followed craft (a grid slot) and HUD (`on`, `off`) for the drive's state line.
    pub(crate) fn replay_view_names(&self) -> (Option<String>, Option<String>) {
        let (Some(active), Stage::Race(stage)) = (self.viewer.as_ref(), &self.stage) else {
            return (None, None);
        };
        (
            Some(stage.race.core.player_slot().to_string()),
            Some(if active.hud { "on" } else { "off" }.to_string()),
        )
    }

    /// Whether the race on screen steps between ticks, for the draw's blend alpha.
    pub(crate) fn viewer_stepping(&self) -> bool {
        self.viewer.as_ref().is_none_or(|active| active.stepping)
    }

    /// The lines of what the keys do under the viewer's chips: each entry that names one control
    /// is a click target (`replay_view::legend_hints`).
    fn viewer_keys(&self, active: &Active) -> Vec<Vec<Entry>> {
        let name = |button: Button, fallback: &str| {
            self.controls
                .bound_keys(button)
                .first()
                .map_or_else(|| fallback.to_string(), ToString::to_string)
        };
        let word = |id: &str| active.word(id);
        let (cross, square) = (name(Button::Cross, "CROSS"), name(Button::Square, "SQUARE"));
        vec![
            vec![
                Entry::button(
                    Button::Cross,
                    format!("{cross} {}", word(replay_view::strings::PLAY)),
                ),
                Entry::new(
                    None,
                    format!("LEFT RIGHT {}", word(replay_view::strings::SKIP)),
                )
                .after("  "),
                Entry::button(
                    Button::Up,
                    format!("UP {}", word(replay_view::strings::SPEED)),
                )
                .after("  "),
            ],
            vec![
                Entry::new(
                    None,
                    format!(
                        "{} {} {}",
                        name(Button::L, "L1"),
                        name(Button::R, "R1"),
                        word(replay_view::strings::CRAFT)
                    ),
                ),
                Entry::button(
                    Button::Square,
                    format!("{square} {}", word(replay_view::strings::HUD)),
                )
                .after("  "),
                Entry::new(None, format!("L2 R2 {}", word(replay_view::strings::SCRUB)))
                    .after("  "),
                Entry::new(None, format!("R-STICK {}", word(replay_view::strings::JOG)))
                    .after("  "),
            ],
        ]
    }

    /// The viewer's screen as an overlay draw list in `grid`; empty when none is open.
    pub(crate) fn viewer_screen(&mut self, grid: (f32, f32)) -> Vec<oag_ui::frontend::Draw> {
        // Photo mode's screen takes its place while it is open over the viewer.
        let Some(active) = self.viewer.as_ref().filter(|_| self.photo.is_none()) else {
            return Vec::new();
        };
        let keys = self.viewer_keys(active);
        let Some(active) = self.viewer.as_mut() else {
            return Vec::new();
        };
        let view = active.view();
        if self.pointer.touch_spoke_last() {
            let chrome = active.chrome_shown();
            let say = |id: &str| active.word(id);
            return replay_view::touch_layout::draw_list(&view, grid, &say, chrome);
        }
        let say = |id: &str| active.word(id);
        replay_view::draw_list(&view, grid, &say, &keys)
    }

    /// The hints of the viewer's key lines on the overlay grid `grid`; none when it is not up.
    pub(crate) fn viewer_hints(&mut self, grid: (f32, f32)) -> Vec<oag_ui::hints::Hint> {
        let Some(active) = self
            .viewer
            .as_ref()
            .filter(|_| self.photo.is_none() && !self.pointer.touch_spoke_last())
        else {
            return Vec::new();
        };
        let keys = self.viewer_keys(active);
        let Some(active) = self.viewer.as_mut() else {
            return Vec::new();
        };
        replay_view::legend_hints(&active.view(), grid, &keys)
    }

    /// One tick of the viewer: the devices into the playhead, and the race stepped or sought.
    pub(crate) fn tick_viewer(&mut self, pointer: &Pointer) {
        use replay_view::Control;
        let surface = self.gpu.output.size();
        let space = oag_present::perf::grid(surface);
        let rect = (0.0, 0.0, surface.0 as f32, surface.1 as f32);
        let grid = super::pointer::in_grid(*pointer, space, rect, 1.0);
        let roles = self.roles();
        // Fingers as gestures; while one spoke last the pointer's click and drag are the same
        // touch twice, so the gestures decide (`Self::tick_photo` does the same).
        let size = self.window_size();
        let frame = self.gestures.take(size);
        let finger = self.pointer.touch_spoke_last();
        let touched = frame.fingers > 0 || frame.tap.is_some() || frame.double_tap.is_some();
        let shown = self.viewer.as_mut().is_none_or(|active| {
            active.idle = if touched {
                0
            } else {
                active.idle.saturating_add(1)
            };
            active.chrome_shown()
        });
        let controls = &mut self.controls;
        let view = controls.view();
        // The playhead's speed: a trigger scrubs (R2 on, L2 back) and the right stick's
        // horizontal axis jogs, each faster the further it is pulled. **Chosen, not measured.**
        let scrub =
            scrub_ticks(view.r2 - view.l2, SCRUB_MOST) + scrub_ticks(view.right.0, JOG_MOST);
        let mut asked = Asked {
            toggle: controls.view_take(Button::Cross) | controls.view_take(Button::Start),
            back: controls.view_take(Button::Left),
            forward: controls.view_take(Button::Right),
            faster: controls.view_take(Button::Up) | controls.view_take(Button::Down),
            leave: controls.view_take(roles.back) | pointer.back,
            photo: controls.view_take(Button::Select),
            previous: controls.view_take(Button::L),
            next: controls.view_take(Button::R),
            hud: controls.view_take(Button::Square),
            scrub,
            seek: None,
            bar: None,
            camera: false,
            bar_touch: None,
            free_camera: false,
        };
        let clicked = pointer.clicked && (!finger || frame.tap.is_some());
        if finger {
            use replay_view::touch_layout as touch;
            if let Some(active) = self.viewer.as_mut() {
                if let Some(at) = grid.press_at
                    && shown
                    && touch::control_at(space.size, at) == Some(Control::Bar)
                {
                    active.bar_drag = Some((at.0, 0));
                }
                if let Some((x, held)) = active.bar_drag.as_mut() {
                    *x += grid.drag.0;
                    *held += 1;
                    if *held % BAR_SEEK_EVERY == 0 || pointer.released {
                        asked.bar_touch = Some(*x);
                    }
                    if pointer.released {
                        active.bar_drag = None;
                    }
                } else if frame.fingers > 0
                    && !active.pose_view
                    && (frame.drag != (0.0, 0.0)
                        || frame.pan != (0.0, 0.0)
                        || frame.pinch != 1.0
                        || frame.twist != 0.0)
                {
                    asked.free_camera = true;
                }
                if let Some(at) = frame.double_tap {
                    let x = at.0 * space.size.0 / size.0.max(1.0);
                    match touch::zone_at(space.size, x) {
                        touch::Zone::Back => asked.back = true,
                        touch::Zone::Toggle => asked.toggle = true,
                        touch::Zone::Forward => asked.forward = true,
                    }
                }
            }
        }
        if clicked
            && finger
            && let Some(at) = grid.at
        {
            let hit = shown
                .then(|| replay_view::touch_layout::control_at(space.size, at))
                .flatten();
            match hit {
                Some(Control::Play) => asked.toggle = true,
                Some(Control::Back) => asked.back = true,
                Some(Control::Forward) => asked.forward = true,
                Some(Control::Speed) => asked.faster = true,
                Some(Control::Photo) => asked.photo = true,
                Some(Control::Exit) => asked.leave = true,
                Some(Control::Bar) => asked.bar_touch = Some(at.0),
                Some(Control::Hud) => asked.hud = true,
                Some(Control::Camera) => asked.camera = true,
                Some(Control::Previous) => asked.previous = true,
                Some(Control::Next) => asked.next = true,
                None => {
                    if let Some(active) = self.viewer.as_mut() {
                        active.chrome_hidden = shown;
                        active.idle = 0;
                    }
                }
            }
        } else if clicked && let Some(at) = grid.at {
            match replay_view::control_at(space.size, at) {
                Some(Control::Play) => asked.toggle = true,
                Some(Control::Back) => asked.back = true,
                Some(Control::Forward) => asked.forward = true,
                Some(Control::Speed) => asked.faster = true,
                Some(Control::Photo) => asked.photo = true,
                Some(Control::Exit) => asked.leave = true,
                Some(Control::Bar) => asked.bar = Some(at.0),
                _ => {}
            }
        }
        if asked.leave {
            self.leave_viewer();
            return;
        }
        if asked.photo || asked.free_camera {
            self.open_photo_from_viewer_with(asked.free_camera);
            return;
        }
        if asked.camera {
            self.cycle_camera_view();
        }
        let (Some(active), Stage::Race(stage)) = (self.viewer.as_mut(), &mut self.stage) else {
            return;
        };
        if asked.hud {
            active.hud = !active.hud;
        }
        let mut followed = false;
        if asked.previous {
            followed |= stage.race.step_followed(false);
        }
        if asked.next {
            followed |= stage.race.step_followed(true);
        }
        let viewer = &mut active.viewer;
        if let Some(x) = asked.bar {
            let tick = replay_view::tick_at(space.size, x, viewer.ticks());
            asked.seek = Some(tick);
        }
        if let Some(x) = asked.bar_touch {
            asked.seek = Some(replay_view::touch_layout::tick_at(
                space.size,
                x,
                viewer.ticks(),
            ));
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
        if asked.scrub != 0 {
            moved = viewer.skip(&mut stage.race, asked.scrub);
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
        // A screenshot's viewer standing on the shot's tick for the first time: the race on
        // screen is compared with the hash the shot was taken at.
        let reached = match active.shot {
            Some((tick, hash)) if moved == Moved::Seeked && at == tick => {
                active.shot = None;
                Some((stage.race.core.sim.state_hash(), hash))
            }
            _ => None,
        };
        match moved {
            // The race at its own speed sounds as a race does; at any other speed the effects
            // are muted, and back at 1X they are rebuilt (`crate::sound::viewer_step`).
            Moved::Stepped(ticks) => {
                // A pose track has no cues: its picture is silent.
                let normal = normal && ticks == 1 && !active.pose_view;
                crate::sound::viewer_step(
                    &mut self.audio,
                    &mut stage.race,
                    normal,
                    &mut active.muted,
                    at,
                );
            }
            // A scrub is silent and its sound is rebuilt where it stops.
            Moved::Seeked if asked.scrub != 0 => active.scrubbing = true,
            Moved::Seeked => {
                active.muted = false;
                active.scrubbing = false;
                crate::sound::rebuild(&mut self.audio, &mut stage.race, at);
            }
            Moved::Still if active.scrubbing && asked.scrub == 0 => {
                active.scrubbing = false;
                active.muted = false;
                crate::sound::rebuild(&mut self.audio, &mut stage.race, at);
            }
            Moved::Still => {}
        }
        // Another craft's engine and cues are the sound now.
        if followed {
            active.muted = false;
            crate::sound::rebuild(&mut self.audio, &mut stage.race, at);
        }
        if let Some((found, shot)) = reached {
            self.shot_reached(found, shot);
        }
    }

    /// Leaves the viewer for the REPLAYS page. Nothing was recorded, so nothing is written; the
    /// race is dropped, never parked.
    pub(crate) fn leave_viewer(&mut self) {
        let Some(closed) = self.viewer.take() else {
            return;
        };
        info!("replays: the viewer closed");
        self.return_to_replays = !closed.from_shot;
        self.audio.pause_race_music();
        self.save_race_music();
        self.audio.stop_race_sfx();
        if let Err(error) = self.open_menus() {
            error!("cannot return to the menus: {error:#}");
            self.quit = true;
        }
    }
}

/// The most ticks a full trigger pull moves the playhead in a frame (half a second), and a full
/// right-stick deflection (a sixth of a second).
const SCRUB_MOST: u32 = super::photo_rewind::SCRUB_MOST;
const JOG_MOST: u32 = 10;
/// Ticks between seeks while a finger holds the bar: a seek rebuilds the race.
const BAR_SEEK_EVERY: u32 = 4;

/// Signed ticks for a signed pull.
fn scrub_ticks(pull: f32, most: u32) -> i64 {
    let ticks = oag_raceplay::photo::effects::scrub_ticks(pull.abs(), most);
    if pull < 0.0 { -ticks } else { ticks }
}

/// What the player asked of the viewer this tick.
struct Asked {
    toggle: bool,
    back: bool,
    forward: bool,
    faster: bool,
    leave: bool,
    photo: bool,
    previous: bool,
    next: bool,
    hud: bool,
    /// Ticks to move the playhead, negative back.
    scrub: i64,
    seek: Option<u64>,
    bar: Option<f32>,
    /// The next camera view (touch).
    camera: bool,
    /// Where a finger holds the bar, in grid units (touch).
    bar_touch: Option<f32>,
    /// A finger asked for the camera on the picture: photo mode opens over the viewer, with
    /// playback going on.
    free_camera: bool,
}

impl Session {
    /// Whether SQUARE turned the viewer's HUD off.
    pub(crate) fn viewer_hud_hidden(&self) -> bool {
        self.viewer.as_ref().is_some_and(|active| !active.hud)
    }

    /// Whether the race on screen is POSE VIEW, whose HUD is not drawn.
    pub(crate) fn pose_view_shown(&self) -> bool {
        self.viewer.as_ref().is_some_and(|active| active.pose_view)
    }
}
