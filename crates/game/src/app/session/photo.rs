//! The session's half of photo mode: opening it, reading the player's devices into the
//! camera, drawing its screen and saving a shot. The rules and the screen are
//! [`crate::photo`], the camera is `oag_raceplay::photo`.
//!
//! The mode is the pause plus a camera. While it is open the frame loop does not tick the race
//! (`crate::photo::ticks`), so no state hash, replay or ghost can tell it happened, and the
//! race resumes where it stood. **Except through a rewind**, which moves the race on screen
//! through its past and restores its simulation on leaving (`photo_rewind`). Audio stops the way
//! it does under the pause.

use crate::photo::{self, Chip, Words, strings};
use log::{debug, info, warn};
use oag_gameplay::input::{Button, InputSnapshot};
use oag_raceplay::photo::{Action, Pad};
use oag_ui::pointer::Pointer;

use crate::app::stage::Stage;

use oag_ui::language::StringTable;

/// How long a result stays on screen, in ticks.
const MESSAGE_TICKS: u32 = 180;

/// Where the mode was opened from, which is where leaving it goes back to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Origin {
    /// The pause menu's PHOTO MODE row: the race resumes.
    Menu,
    /// SELECT on `Race End Photo`: the end screens carry on.
    RaceEnd,
    /// The replay viewer's PHOTO chip or SELECT: the viewer carries on from where the mode left
    /// the playhead (ADR-0088).
    Viewer,
}

/// A shot that waits for the frame carrying its progress bar to be drawn.
#[derive(Debug)]
pub(super) struct Announced {
    pub(super) path: std::path::PathBuf,
    pub(super) pending: Option<super::photo_meta::Pending>,
    /// That frame has been drawn.
    pub(super) shown: bool,
}

/// Photo mode while it is open.
#[derive(Debug)]
pub(crate) struct Open {
    origin: Origin,
    /// The last shot's result and the ticks it stays up.
    message: Option<(String, u32)>,
    /// A shot asked for that is drawn at the output's size (`RenderProfile::for_a_photo`) and
    /// has not begun: the frame that shows its progress bar is drawn first, because the redraw
    /// stalls the frame it is in.
    pub(super) announced: Option<Announced>,
    /// When the shot began drawing at the output's size, for the log line that says how long
    /// the redraw stalled.
    redraw_began: Option<web_time::Instant>,
    /// The frame before was drawn at the profile a photo is drawn with.
    was_native: bool,
    /// A shot asked for this tick, read back by `Session::draw` once the frame is drawn.
    pub(super) shot: Option<(std::path::PathBuf, Option<super::photo_meta::Pending>)>,
    /// The project's words in the player's language and in English, read once when the mode
    /// opens: the screen is drawn every frame.
    own: StringTable,
    english: StringTable,
    /// The race on screen moved through its past, once the rewind row or a motion blur shot
    /// first needs it (`photo_rewind`).
    pub(super) rewind: Option<oag_raceplay::photo::rewind::Rewind>,
    /// A shot of several frames under way.
    pub(super) job: Option<super::photo_rewind::Job>,
    /// A finished shot of several frames waiting for the HUD to be drawn over it
    /// (`photo_hud`).
    pub(super) averaged: Option<super::photo_hud::Averaged>,
    /// The tick the rewind row last asked for.
    pub(super) asked: u64,
    /// Whether the viewer this mode was opened from was playing when it was, so leaving puts
    /// it back to playing from the tick the mode stands at.
    pub(super) resume_viewer: bool,
    /// R2 is held at its full pull, so one pull is one press of PLAY.
    pub(super) play_latched: bool,
}

impl Open {
    /// Where the mode was opened from.
    pub(super) fn origin(&self) -> Origin {
        self.origin
    }

    /// A shot of several frames times itself.
    pub(super) fn clear_redraw_clock(&mut self) {
        self.redraw_began = None;
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

/// Every button the mode reads, spent on leaving so none of them reaches the race as an edge.
const SPENT: [Button; 10] = [
    Button::Cross,
    Button::Circle,
    Button::Triangle,
    Button::Square,
    Button::L,
    Button::R,
    Button::Start,
    Button::Select,
    Button::Up,
    Button::Down,
];

/// Whether the mode can frame this race: one view, on one window.
fn fits(stage: &crate::app::race_stage::RaceStage) -> bool {
    !stage.race.windowed() && photo::offered(stage.race.view_state().viewport_count())
}

impl super::Session {
    /// Whether PHOTO MODE is on the pause menu now: a race with one view that has not finished
    /// is on screen or parked behind the menus, and this build can write a shot.
    pub(crate) fn photo_available(&self) -> bool {
        let race = match self.stack.top() {
            // A benchmark's race is never held, so it offers no PHOTO MODE.
            Stage::Race(stage) if !stage.race.core.sim.finished() && stage.benchmark.is_none() => {
                Some(&**stage)
            }
            _ => self.stack.paused_race(),
        };
        self.net.is_none() && race.is_some_and(fits)
    }

    /// The PHOTO MODE row: back to the parked race, with the mode open on it.
    pub(crate) fn open_photo_from_menu(&mut self) {
        if !self.photo_available() || self.stack.paused_race().is_none() {
            return;
        }
        self.resume_race();
        self.begin_photo(Origin::Menu);
    }

    /// The viewer's PHOTO chip: photo mode over the viewer's race at the tick on screen. The
    /// viewer stands still while it is open and carries on from where the mode's rewind row
    /// leaves it ([`Self::leave_photo`]). Playback goes on in the mode when `keep_playing` and the viewer was playing: what a finger
    /// on the picture asks for.
    pub(crate) fn open_photo_from_viewer_with(&mut self, keep_playing: bool) {
        let Some(active) = self.viewer.as_mut() else {
            return;
        };
        if self.photo.is_some()
            || !matches!(self.stack.top(), Stage::Race(stage) if fits(stage))
            || active.pose_view
        {
            return;
        }
        let playing = active.viewer.playing();
        active.viewer.set_playing(false);
        active.stepping = false;
        self.begin_photo(Origin::Viewer);
        match self.photo.as_mut() {
            Some(open) => {
                open.resume_viewer = playing;
                if keep_playing
                    && playing
                    && let Stage::Race(stage) = self.stack.top_mut()
                {
                    stage
                        .race
                        .photo_effects(|effects| effects.set_playing(true));
                }
            }
            None => {
                if let Some(active) = self.viewer.as_mut() {
                    active.viewer.set_playing(playing);
                }
            }
        }
    }

    /// SELECT on `Race End Photo` (once it takes input), with the mode open over the finish.
    pub(crate) fn open_photo_at_the_finish(&mut self) {
        if self.net.is_none() && matches!(self.stack.top(), Stage::Race(stage) if fits(stage)) {
            self.begin_photo(Origin::RaceEnd);
        }
    }

    fn begin_photo(&mut self, origin: Origin) {
        let held = self.controls.buttons();
        let (select, square) = (held.is_held(Button::Select), held.is_held(Button::Square));
        let title_shot = self.shell.as_ref().and_then(|shell| shell.title.photo_shot);
        let watched = self
            .viewer
            .as_ref()
            .map(|active| (active.viewer.at(), active.viewer.ticks()));
        let Stage::Race(stage) = self.stack.top_mut() else {
            return;
        };
        if !stage.race.photo_begin(
            select,
            square,
            Some(oag_source::title::borrowed_zoom_ring()),
        ) {
            return;
        }
        info!("photo mode opened ({origin:?})");
        // What the title's original accumulates, and whether the race can be moved through its
        // past (a menu visit to a race with keyframes; the finish keeps its board).
        // The viewer has no keyframes of the race it moves (the blur rows would do nothing), so
        // over it the shot is the one frame, with the single-frame depth of field.
        stage.race.photo_offer(
            title_shot.filter(|_| origin != Origin::Viewer),
            match origin {
                Origin::Menu => stage.keeper.is_some(),
                Origin::RaceEnd => false,
                Origin::Viewer => watched.is_some(),
            },
        );
        let mut asked = stage.race.core.sim.world.tick;
        // A watched race is moved through the whole recording by the viewer: the row runs to its
        // end and stands where the playhead does.
        if let (Origin::Viewer, Some((at, ticks))) = (origin, watched) {
            stage.race.photo_effects(|effects| {
                effects.set_live(ticks);
                effects.set_target_tick(at);
            });
            asked = at;
        }
        self.controls.silence_motors();
        if origin != Origin::Viewer {
            self.gestures.release_all();
        }
        let language = self.settings.language.as_deref();
        self.photo = Some(Box::new(Open {
            origin,
            message: None,
            announced: None,
            redraw_began: None,
            was_native: false,
            shot: None,
            own: oag_ui::strings::project_table(language),
            english: oag_ui::strings::project_table(None),
            rewind: None,
            job: None,
            averaged: None,
            asked,
            resume_viewer: false,
            play_latched: false,
        }));
    }

    /// Whether the race is stepping between ticks: not frozen by the pause and not held by the
    /// mode. What the draw's blend alpha asks.
    pub(crate) fn race_stepping(&self) -> bool {
        !self.pause.frozen() && photo::ticks(self.photo.is_some()) && self.viewer_stepping()
    }

    /// One tick of the mode: the player's devices into the camera, and what came of it.
    pub(crate) fn tick_photo(&mut self, pointer: &Pointer, input: &InputSnapshot) {
        // The mode belongs to the race on screen: with none, it is over, and the race that was
        // moved is put back wherever it is now (parked behind the menus, say).
        if !matches!(self.stack.top(), Stage::Race(_)) {
            self.leave_photo();
            return;
        }
        let Some(open) = self.photo.as_mut() else {
            return;
        };
        if let Some((_, left)) = open.message.as_mut() {
            *left = left.saturating_sub(1);
        }
        if open.message.as_ref().is_some_and(|(_, left)| *left == 0) {
            open.message = None;
        }
        let surface = self.gpu.output.size();
        let space = oag_present::perf::grid(surface);
        let rect = (0.0, 0.0, surface.0 as f32, surface.1 as f32);
        let grid = super::pointer::in_grid(*pointer, space, rect, 1.0);
        let roles = self.roles();
        // Fingers as gestures. A finger is also the pointer, so while one spoke last the
        // pointer's own drag and click are the same touch twice: the gestures decide.
        let size = self.window_size();
        let frame = self.gestures.take(size);
        let finger = self.pointer.touch_spoke_last();
        let controls = &mut self.controls;
        let view = controls.view();
        let mut pad = Pad {
            stick: (input.stick_x, input.stick_y),
            right: view.right,
            select: controls.view_held(Button::Select),
            square: controls.view_held(Button::Square),
            reset: controls.view_take(roles.help),
            previous: controls.view_take(Button::L),
            next: controls.view_take(Button::R),
            help: controls.view_take(Button::Start),
            shot: controls.view_take(Button::Cross),
            leave: controls.view_take(roles.back),
            drag: if finger { (0.0, 0.0) } else { pointer.drag },
            scroll: pointer.scroll as f32,
            touch: oag_raceplay::photo::touch::Touch {
                drag: frame.drag,
                pan: frame.pan,
                pinch: frame.pinch,
                twist: frame.twist,
            },
        };
        if frame.double_tap.is_some() {
            pad.reset = true;
        }
        let clicked = pointer.clicked && (!finger || frame.tap.is_some());
        // A shot of several frames under way owns the mode: any input abandons it, and the
        // controls stand still while it runs.
        // The frame that puts the HUD over a finished shot is part of it: the race stands where
        // it was taken until that frame is drawn, and no input moves it.
        if self.photo_wants_frames() {
            let abandons = Self::photo_input_abandons(&pad, pointer, input, &view);
            let announced = self
                .photo
                .as_ref()
                .is_some_and(|open| open.announced.is_some());
            if self.photo_job_running() && abandons {
                self.photo_job_cancel();
            } else if announced && abandons {
                if let Some(open) = self.photo.as_mut() {
                    open.announced = None;
                }
                self.photo_message(strings::CANCELLED);
            } else if !abandons {
                self.photo_begin_announced();
            }
            return;
        }
        let Stage::Race(stage) = self.stack.top_mut() else {
            return;
        };
        if pointer.back {
            pad.leave = true;
        }
        if clicked {
            let grid_size = space.size;
            let hit = grid.at.and_then(|at| {
                stage.race.photo().filter(|p| p.help()).and_then(|p| {
                    let free = p.lens() == oag_raceplay::photo::Lens::Free;
                    photo::chip_at_with(grid_size, at, free, finger)
                })
            });
            let row = grid.at.and_then(|at| {
                stage
                    .race
                    .photo()
                    .and_then(|p| photo::row_at_with(p, grid_size, at, finger))
            });
            if let Some(row) = row {
                stage.race.photo_effects(|effects| {
                    effects.select(row.row);
                    if row.step != 0 {
                        effects.adjust_row(row.row, row.step);
                    }
                });
            }
            match hit {
                Some(Chip::Effects) => stage.race.photo_toggle_panel(),
                Some(Chip::Camera) => stage.race.photo_cycle_lens(),
                Some(Chip::Hud) => stage.race.photo_toggle_hud(),
                Some(Chip::Drag) => stage.race.photo_toggle_drag(),
                Some(Chip::Take) => pad.shot = true,
                Some(Chip::Back) => pad.leave = true,
                Some(Chip::Reset) => pad.reset = true,
                Some(Chip::Previous) => pad.previous = true,
                Some(Chip::Next) => pad.next = true,
                None if row.is_none() && finger => stage.race.photo_toggle_chrome(),
                None if row.is_none() && !stage.race.photo().is_some_and(|p| p.help()) => {
                    pad.help = true;
                }
                None => {}
            }
        }
        // A held mouse button reports its travel as a drag, as a finger does, while the free
        // lens is up; on the others a click is a click.
        let free = stage
            .race
            .photo()
            .is_some_and(|p| p.lens() == oag_raceplay::photo::Lens::Free);
        self.pointer.set_mouse_pans(free);
        let Stage::Race(stage) = self.stack.top_mut() else {
            return;
        };
        let action = stage.race.photo_step(pad);
        self.photo_triggers(&view);
        match action {
            Action::None => {
                self.photo_rewind_sync();
                self.photo_play_tick();
            }
            Action::Leave => self.leave_photo(),
            Action::Shot => self.ask_for_a_shot(),
        }
    }

    /// Names the file the next drawn frame is saved to.
    fn ask_for_a_shot(&mut self) {
        let title = self.shell.as_ref().map_or("game", |shell| shell.title.name);
        let Some(dir) = dirs::data_dir().map(|dir| dir.join("oag").join(photo::DIRECTORY)) else {
            self.photo_message(strings::FAILED);
            warn!("photo mode: there is no data directory to save into");
            return;
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());
        if let Err(why) = std::fs::create_dir_all(&dir) {
            warn!("photo mode: cannot make {}: {why}", dir.display());
            self.photo_message(strings::FAILED);
            return;
        }
        let path = photo::shot_path(&dir, title, now);
        // The description is taken now: an accumulated shot moves the race and puts it back
        // before it writes.
        let pending = self.shot_pending("photo mode");
        // A shot drawn at the output's size is a redraw that stalls its frame: the frame with the
        // progress bar goes first.
        if self.photo_redraws() {
            if let Some(open) = self.photo.as_mut() {
                open.announced = Some(Announced {
                    path,
                    pending,
                    shown: false,
                });
            }
            return;
        }
        self.photo_begin(path, pending);
    }

    /// Begins the shot, once the frame that announces it has been drawn.
    fn photo_begin_announced(&mut self) {
        let Some(open) = self.photo.as_mut() else {
            return;
        };
        if open.announced.as_ref().is_none_or(|waiting| !waiting.shown) {
            return;
        }
        if let Some(Announced { path, pending, .. }) = open.announced.take() {
            open.redraw_began = Some(web_time::Instant::now());
            self.photo_begin(path, pending);
        }
    }

    /// Begins the shot: several frames when the title's shot accumulates and the controls ask for
    /// it, else the frame the next draw leaves on screen.
    fn photo_begin(
        &mut self,
        path: std::path::PathBuf,
        pending: Option<super::photo_meta::Pending>,
    ) {
        let Err(pending) = self.start_photo_job(&path, pending) else {
            return;
        };
        if let Some(open) = self.photo.as_mut() {
            open.shot = Some((path, pending));
        }
    }

    /// Whether a shot is being drawn now (or waits for the frame that shows its bar): the frame
    /// is drawn with the profile a photo is made with (`RenderProfile::for_a_photo`).
    pub(super) fn photo_draws_natively(&self) -> bool {
        self.photo.as_ref().is_some_and(|open| {
            open.shot.is_some() || open.job.is_some() || open.averaged.is_some()
        })
    }

    /// Whether the player's profile draws a different picture than a photo's does.
    pub(super) fn photo_redraws(&self) -> bool {
        let rect = self.scene_rect(self.gpu.output.size());
        let gamescope = crate::app::window::under_gamescope_cached(
            self.gpu.output.window.current_monitor().as_ref(),
        );
        let profile = self.player_render_profile();
        let glass_step = match self.stack.top() {
            Stage::Race(stage) if stage.scene.reflection_target_size().is_some() => {
                Some(self.reflection_step(&profile))
            }
            _ => None,
        };
        profile.photo_differs((rect.2, rect.3), gamescope, glass_step)
    }

    /// How far a shot has got, for its bar: a shot of several frames by its frames, and a shot
    /// that waits for the frame that shows its bar as not started.
    pub(super) fn photo_bar(&self) -> Option<(u32, u32)> {
        self.photo_progress().or_else(|| {
            self.photo
                .as_ref()
                .filter(|open| open.announced.is_some())
                .map(|_| (0, 1))
        })
    }

    pub(super) fn photo_message(&mut self, id: &str) {
        if let Some(open) = self.photo.as_mut() {
            open.message = Some((open.word(id), MESSAGE_TICKS));
        }
    }

    /// Writes the picture the frame just drawn, if one was asked for: the presentation target
    /// as it stood before the grade and before anything was drawn over the surface, so neither
    /// the screen nor (unless its row is on) the HUD is in the file. Called once the frame is
    /// submitted.
    pub(super) fn finish_photo_shot(&mut self) {
        // A frame at the profile a photo is drawn with, and the one that puts the preview's
        // back, each reallocate the targets: neither is a frame time.
        let native = self.photo_draws_natively();
        if let Some(open) = self.photo.as_mut() {
            let stalls = native || open.was_native;
            open.was_native = native;
            self.stalled |= stalls;
        }
        if let Some(waiting) = self.photo.as_mut().and_then(|open| open.announced.as_mut()) {
            waiting.shown = true;
        }
        // The average a job set aside on an earlier frame is written first: the frame that set
        // it aside has not drawn the HUD over it yet.
        self.photo_finish_averaged();
        self.photo_job_collect();
        let Some((path, pending)) = self.photo.as_mut().and_then(|open| open.shot.take()) else {
            return;
        };
        let result = self
            .read_photo_output()
            .and_then(|(pixels, width, height)| match pending {
                Some(pending) => pending.write(&path, width, height, &pixels),
                None => crate::capture::write_png(&path, width, height, &pixels),
            });
        let redrawn = self
            .photo
            .as_mut()
            .and_then(|open| open.redraw_began.take())
            .map(|began| began.elapsed());
        match result {
            Ok(()) => {
                debug!(
                    "photo mode: saved {}{}",
                    path.display(),
                    redrawn.map_or_else(String::new, |took| format!(
                        " (drawn at the output's size, {took:?} from the first frame to the file)"
                    ))
                );
                self.photo_message(photo::saved_word(&path));
            }
            Err(why) => {
                warn!("photo mode: cannot save {}: {why:#}", path.display());
                self.photo_message(strings::FAILED);
            }
        }
    }

    /// The presentation target as the frame just submitted left it, as RGBA bytes with its
    /// size.
    pub(super) fn read_photo_output(&mut self) -> anyhow::Result<(Vec<u8>, u32, u32)> {
        let (width, height) = self.framebuffer.output_size();
        let texture = self.framebuffer.output_texture();
        let device = &self.gpu.device;
        let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("photo shot"),
        });
        let mut pixels =
            crate::capture::read_back(device, &self.gpu.queue, encoder, texture, width, height)?;
        if matches!(
            texture.format(),
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        ) {
            for pixel in pixels.as_chunks_mut::<4>().0 {
                pixel.swap(0, 2);
            }
        }
        Ok((pixels, width, height))
    }

    /// Closes the mode: the camera as it was, the race stepping again, every button the mode
    /// read spent so none reaches the race or the end screens as a press.
    pub(crate) fn leave_photo(&mut self) {
        self.pointer.set_mouse_pans(false);
        self.gestures.release_all();
        self.photo_job_cancel();
        let Some(mut open) = self.photo.take() else {
            return;
        };
        let stage = if matches!(self.stack.top(), Stage::Race(_)) {
            match self.stack.top_mut() {
                Stage::Race(stage) => Some(&mut **stage),
                _ => None,
            }
        } else {
            self.stack.paused_race_mut()
        };
        let mut play_on = false;
        if let Some(stage) = stage {
            play_on = stage.race.photo().is_some_and(|p| p.effects().playing());
            // The craft the mode ended on is the one the viewer goes on following.
            if open.origin == Origin::Viewer
                && let Some(subject) = stage.race.photo().map(oag_raceplay::photo::Photo::subject)
            {
                stage.race.follow(subject);
            }
            // The live race, put back to the bit, before the camera is.
            if let (Some(rewind), Some(keeper)) = (open.rewind.take(), stage.keeper.as_mut()) {
                rewind.end(keeper, &mut stage.race);
            }
            stage.race.photo_end();
        }
        // A viewer carries on from the tick the mode stands at: playing if it was playing when
        // the mode opened or the PLAY row is on, silent voices rebuilt from that tick.
        if open.origin == Origin::Viewer
            && let (Some(active), Stage::Race(stage)) = (self.viewer.as_mut(), self.stack.top_mut())
        {
            active.viewer.set_playing(open.resume_viewer || play_on);
            active.muted = false;
            crate::sound::rebuild(&mut self.audio, &mut stage.race, active.viewer.at());
        }
        let buttons = self.controls.buttons_mut();
        for button in SPENT {
            buttons.consume_press(button);
        }
        self.controls.silence_motors();
        info!("photo mode closed ({:?})", open.origin);
    }

    /// Runs `f` on the mode's photo, the words its screen is worded with and the state of the
    /// mode; `None` when it is not open.
    fn with_photo_words<R>(
        &self,
        f: impl FnOnce(&oag_raceplay::photo::Photo, &Words, &Open) -> R,
    ) -> Option<R> {
        let (Some(open), Stage::Race(stage)) = (self.photo.as_ref(), self.stack.top()) else {
            return None;
        };
        let photo = stage.race.photo()?;
        // The disc's own word for a row where the title's disc has one, else the project's.
        let say_id = |id: &str| {
            self.shell
                .as_ref()
                .and_then(|shell| photo::disc_word(id, &shell.title.photo, &shell.strings))
                .unwrap_or_else(|| open.word(id))
        };
        let button = |button: Button| {
            let name = button.to_string().to_uppercase();
            match self.controls.bound_keys(button).first() {
                Some(key) => format!("{name} ({key})"),
                None => name,
            }
        };
        let words = Words {
            say: &say_id,
            button: &button,
            roles: self.roles(),
            touch: self.pointer.touch_spoke_last(),
        };
        Some(f(photo, &words, open))
    }

    /// The mode's screen as an overlay draw list in the grid `grid`: empty when it is not open.
    /// It goes over the surface, after the picture a shot reads.
    pub(crate) fn photo_screen(&self, grid: (f32, f32)) -> Vec<oag_ui::frontend::Draw> {
        self.with_photo_words(|photo, words, open| {
            photo::draw_list(
                photo,
                grid,
                words,
                open.message.as_ref().map(|(text, _)| text.as_str()),
                self.photo_bar(),
            )
        })
        .unwrap_or_default()
    }

    /// The hints of the mode's key legend in the grid `grid`: none when it is not open.
    pub(crate) fn photo_hints(&self, grid: (f32, f32)) -> Vec<oag_ui::hints::Hint> {
        self.with_photo_words(|photo, words, _| {
            photo::legend_hints(photo, grid, words, self.photo_bar().is_some())
        })
        .unwrap_or_default()
    }

    /// The drive's reading of the picture controls while the mode is open.
    pub(crate) fn photo_fx_name(&self) -> Option<String> {
        use oag_raceplay::photo::effects::Row;
        let Stage::Race(stage) = self.stack.top() else {
            return None;
        };
        let photo = stage.race.photo()?;
        let effects = photo.effects();
        let (yaw, pitch) = photo.orbit();
        let (pan, fov, tilt) = (photo.pan(), photo.fov(), photo.tilt());
        Some(format!(
            "exposure={} saturation={} speed_blur={} dof={} aperture={} focus={} blur={} shutter={} rewind={} play={} orbit={yaw:.2},{pitch:.2} pan={:.2},{:.2} fov={fov:.1} tilt={tilt:.2} subject={} hud={} free={}",
            effects.value(Row::Exposure),
            effects.value(Row::Saturation),
            effects.value(Row::SpeedBlur),
            if effects.value(Row::DepthOfField) == 1 {
                "on"
            } else {
                "off"
            },
            effects.value(Row::Aperture),
            effects.value(Row::Focus),
            effects.value(Row::MotionBlur),
            effects.value(Row::Shutter),
            effects.target_tick(),
            u8::from(effects.playing()),
            pan.0,
            pan.1,
            photo.subject(),
            u8::from(photo.hud()),
            photo.free_pose().map_or_else(
                || "none".to_string(),
                |(eye, yaw, pitch, roll)| format!(
                    "{:.1},{:.1},{:.1}/{yaw:.2},{pitch:.2},{roll:.2}",
                    eye.x, eye.y, eye.z
                ),
            ),
        ))
    }

    /// The drive's reading of the shot under way: `frames drawn / frames`, `hud` while its
    /// average waits for the HUD to be drawn over it, or `none`.
    pub(crate) fn photo_job_name(&self) -> String {
        if self
            .photo
            .as_ref()
            .is_some_and(|open| open.averaged.is_some())
        {
            return "hud".to_string();
        }
        self.photo_progress()
            .map_or_else(|| "none".to_string(), |(done, all)| format!("{done}/{all}"))
    }

    /// The drive's name for the mode: the lens, or `none`.
    pub(crate) fn photo_lens_name(&self) -> &'static str {
        use oag_raceplay::photo::Lens;
        match self.stack.top() {
            Stage::Race(stage) => match stage.race.photo().map(oag_raceplay::photo::Photo::lens) {
                Some(Lens::Internal) => "internal",
                Some(Lens::Close) => "close",
                Some(Lens::Track) => "track",
                Some(Lens::Free) => "free",
                None => "none",
            },
            _ => "none",
        }
    }
}
