//! The in-game benchmark in the windowed session (`docs/tools/in-game-benchmark.md`).
//!
//! A launch is an ordinary race launch (`Session::launch_race`) on a copy of the options with
//! every option the simulation reads pinned to the benchmark's race (`crate::benchmark::plan`),
//! flown by the autopilot, and handed over at `Session::advance_race_build` with a
//! [`crate::benchmark::Live`] on its stage. The frame loop feeds that the same `elapsed` the
//! performance meter gets; the tick on which the player crosses the line freezes the state
//! hash; the stats screen then takes the race stage's place on screen and its BACK returns to
//! where the benchmark was started from. The player's own `race_options` and autopilot flag are
//! put back whichever way the run ends, and nothing a benchmark does is a record, a ghost or a
//! recording.

use log::{debug, info, warn};

use crate::benchmark::recorder::Ending;
use crate::benchmark::screen::{self, Choice};
use crate::benchmark::{self, Done, Live, ModLoaded, Origin, Recorder, Report, Request, plan};
use oag_gameplay::input::Button;
use oag_ui::pointer::Pointer;

use crate::app::stage::Stage;

use super::Session;

/// The BENCHMARK OPTIONS page's id in `assets/ui/menu.toml`.
const OPTIONS_PAGE: &str = "benchmark_options";

/// What the session keeps of the benchmark between ticks.
#[derive(Default)]
pub(crate) struct State {
    /// `--benchmark`: asked for on the command line, launched once the menus are up.
    pub(crate) cli: Option<Request>,
    /// `--ticks` with `--benchmark`: where every benchmark of this run stops, menu launches
    /// included (a short run for a test).
    tick_cap: Option<u32>,
    /// The BENCHMARK OPTIONS page's rows: what START BENCHMARK launches with.
    pub(crate) page: Request,
    /// The launch, from the call to the race's hand-over.
    launching: Option<Launch>,
    /// What the player had, set aside while a benchmark's stand in its place.
    saved: Option<Saved>,
}

struct Launch {
    origin: Origin,
    request: Request,
    tick_cap: u32,
}

/// The launch fields a benchmark clears, as the player (or a dev flag) had them.
struct Pinned {
    autopilot_pilot: Option<oag_ai::Pilot>,
    autopilot_skill: Option<oag_ai::Difficulty>,
    campaign_cell: Option<oag_tables::race_campaign::Cell>,
    campaign_custom: Option<crate::campaign::custom::Launch>,
    campaign_difficulty: Option<oag_tables::race_campaign::Difficulty>,
    campaign_ai_skill_scale: Option<f32>,
}

struct Saved {
    race_options: Option<oag_raceplay::Options>,
    autopilot: bool,
    pinned: Pinned,
    /// The settings as the player has them, put back when the run ends: a run's preset, vsync and
    /// frame limit are applied over them in memory.
    settings: crate::settings::Settings,
    trail: Vec<oag_ui::menu::Spot>,
}

impl State {
    /// Whether a benchmark's settings are in effect, so nothing is written to `settings.toml`
    /// until the player's own are back.
    pub(crate) fn holds_settings(&self) -> bool {
        self.saved.is_some()
    }

    /// Writes `settings` to `settings.toml`, except while a benchmark is in effect: its settings
    /// are in memory only and the player's file is theirs.
    pub(crate) fn save(&self, settings: &crate::settings::Settings) -> anyhow::Result<()> {
        if self.holds_settings() {
            return Ok(());
        }
        crate::settings::save(settings)
    }

    /// Whether the race being launched is a benchmark that loads none of the player's mods.
    pub(crate) fn runs_without_mods(&self) -> bool {
        self.launching
            .as_ref()
            .is_some_and(|launch| !launch.request.mods)
    }

    /// The benchmark the race being built is, if one is being launched: handed to the stage.
    pub(crate) fn take_launch(&mut self) -> Option<Live> {
        let launch = self.launching.take()?;
        Some(Live {
            origin: launch.origin,
            request: launch.request,
            tick_cap: launch.tick_cap,
            recorder: Recorder::begin(crate::benchmark::load_average()),
            done: None,
        })
    }

    /// The state of a run started with `--benchmark`, or of a plain one.
    pub(crate) fn with_cli(cli: Option<Request>, tick_cap: Option<u32>) -> Self {
        Self {
            tick_cap,
            cli,
            ..Self::default()
        }
    }
}

impl Session {
    /// Whether the menus being opened (or open) sit over a race that is kept: a benchmark
    /// replaces the race it is started over, so its rows are not offered then.
    pub(crate) fn race_is_parked_behind_menus(&self) -> bool {
        self.suspended_race.is_some()
            || matches!(&self.stage, Stage::Race(stage)
                if stage.benchmark.is_none() && !stage.race.core.sim.finished())
    }

    /// Whether the stage on screen is a benchmark's: its race or its stats screen.
    pub(crate) fn on_benchmark(&self) -> bool {
        matches!(&self.stage, Stage::Race(stage) if stage.benchmark.is_some())
    }

    /// Whether the race on screen is a benchmark that has not ended.
    pub(crate) fn in_benchmark(&self) -> bool {
        matches!(&self.stage, Stage::Race(stage)
            if stage.benchmark.as_ref().is_some_and(|live| live.done.is_none()))
    }

    /// Whether a benchmark is loading or running: the window and the output stay as they are
    /// until its figures are in.
    pub(crate) fn benchmark_running(&self) -> bool {
        matches!(self.benchmark_phase(), Some("loading" | "running"))
    }

    /// Whether the stats screen is up.
    pub(crate) fn on_benchmark_screen(&self) -> bool {
        matches!(&self.stage, Stage::Race(stage)
            if stage.benchmark.as_ref().is_some_and(|live| live.done.is_some()))
    }

    /// Where the benchmark is, for a script that waits on it: `loading`, `running` or `stats`.
    pub(crate) fn benchmark_phase(&self) -> Option<&'static str> {
        match &self.stage {
            Stage::Race(stage) => stage.benchmark.as_ref().map(|live| {
                if live.done.is_some() {
                    "stats"
                } else {
                    "running"
                }
            }),
            Stage::Loading(_) if self.benchmark.launching.is_some() => Some("loading"),
            _ => None,
        }
    }

    /// A BENCHMARK or RUN BENCHMARK row was fired: opens the BENCHMARK OPTIONS page, its rows
    /// starting from the player's own settings. A menu tree without the page starts the
    /// benchmark with those settings outright.
    pub(crate) fn open_benchmark_options(&mut self) {
        self.benchmark.page = Request {
            preset: None,
            mods: false,
            vsync: Some(self.settings.display.vsync),
            frame_limit: Some(self.settings.display.frame_limit),
        };
        let mods_on_offer = self.benchmark_mods_on_offer();
        let page = self.benchmark.page.clone();
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let menu = &mut stage.menu;
        menu.seed("benchmark.mods", &oag_ui::menu::Value::Text("off".into()));
        menu.seed(
            "benchmark.vsync",
            &oag_ui::menu::Value::Text(page.vsync.unwrap_or_default().to_string()),
        );
        menu.seed(
            "benchmark.frame_limit",
            &oag_ui::menu::Value::Text(page.frame_limit.unwrap_or_default().to_string()),
        );
        menu.seed(
            "benchmark.preset",
            &oag_ui::menu::Value::Text("settings".into()),
        );
        menu.offer_choice("benchmark.mods", mods_on_offer);
        if !menu.push(OPTIONS_PAGE) {
            self.start_benchmark_from_menu();
        }
    }

    /// START BENCHMARK, or the menu without the options page: starts the benchmark the page's
    /// rows ask for, and remembers the page it came from for the stats screen's BACK.
    pub(crate) fn start_benchmark_from_menu(&mut self) {
        let origin = match &self.stage {
            Stage::Menu(stage)
                if stage
                    .menu
                    .trail()
                    .iter()
                    .any(|spot| spot.page == "graphics") =>
            {
                Origin::Graphics
            }
            _ => Origin::Extras,
        };
        let request = std::mem::take(&mut self.benchmark.page);
        self.launch_benchmark(origin, request, None);
    }

    /// Whether any mod of the player's is on, so MODS has something to choose.
    fn benchmark_mods_on_offer(&self) -> bool {
        self.shell
            .as_ref()
            .is_some_and(|shell| shell.mods.iter().any(is_loaded_mod))
    }

    /// The BENCHMARK OPTIONS rows' own keys: held for the run, never saved.
    pub(crate) fn apply_benchmark_option(&mut self, setting: &str, text: &str) -> bool {
        let page = &mut self.benchmark.page;
        match setting {
            "benchmark.mods" => page.mods = text == "on",
            "benchmark.vsync" => match text.parse() {
                Ok(vsync) => page.vsync = Some(vsync),
                Err(why) => warn!("ignoring {setting} = {text:?}: {why}"),
            },
            "benchmark.frame_limit" => match text.parse() {
                Ok(limit) => page.frame_limit = Some(limit),
                Err(why) => warn!("ignoring {setting} = {text:?}: {why}"),
            },
            "benchmark.preset" => page.preset = (text != "settings").then(|| text.to_string()),
            _ => return false,
        }
        true
    }

    /// Starts a benchmark from `origin` with the preset the request names. Returns whether the
    /// loading screen is up.
    pub(crate) fn launch_benchmark(
        &mut self,
        origin: Origin,
        request: Request,
        tick_cap: Option<u32>,
    ) -> bool {
        let Some(mut options) = self.race_options.clone() else {
            warn!("no disc image has been chosen yet: there is nothing to benchmark");
            return false;
        };
        plan::pin(&mut options);
        let preset_settings = match request.preset.as_deref() {
            None => None,
            Some(name) => match self.benchmark_preset(name) {
                Ok(settings) => Some(settings),
                Err(why) => {
                    log::error!("{why}");
                    self.benchmark.cli = None;
                    self.quit |= origin == Origin::Cli;
                    return false;
                }
            },
        };
        // The page BACK returns to is the one the benchmark row was fired from; the options page
        // is reopened by firing the row again.
        let trail = match &self.stage {
            Stage::Menu(stage) => stage
                .menu
                .trail()
                .into_iter()
                .filter(|spot| spot.page != OPTIONS_PAGE)
                .collect(),
            _ => Vec::new(),
        };
        let player = self.race_options.replace(options);
        // The first launch of a run keeps the player's own; a re-run replaces a benchmark's.
        if self.benchmark.saved.is_none() {
            self.benchmark.saved = Some(Saved {
                race_options: player,
                autopilot: self.autopilot,
                pinned: Pinned {
                    autopilot_pilot: self.autopilot_pilot.take(),
                    autopilot_skill: self.autopilot_skill.take(),
                    campaign_cell: self.campaign_cell.take(),
                    campaign_custom: self.campaign_custom.take(),
                    campaign_difficulty: self.campaign_difficulty.take(),
                    campaign_ai_skill_scale: self.campaign_ai_skill_scale.take(),
                },
                settings: self.settings.clone(),
                trail,
            });
        }
        for (key, value) in preset_settings.into_iter().flatten() {
            self.apply_setting(
                &format!("graphics.{key}"),
                &oag_ui::menu::Value::Text(value),
            );
        }
        if let Some(vsync) = request.vsync.filter(|v| *v != self.settings.display.vsync) {
            self.apply_setting(
                "display.vsync",
                &oag_ui::menu::Value::Text(vsync.to_string()),
            );
        }
        if let Some(limit) = request
            .frame_limit
            .filter(|l| *l != self.settings.display.frame_limit)
        {
            self.apply_setting(
                "display.frame_limit",
                &oag_ui::menu::Value::Text(limit.to_string()),
            );
        }
        self.autopilot = true;
        self.autopilot_pilot = None;
        self.autopilot_skill = None;
        self.campaign_cell = None;
        self.campaign_custom = None;
        self.campaign_difficulty = None;
        self.campaign_ai_skill_scale = None;
        self.benchmark.launching = Some(Launch {
            origin,
            request,
            tick_cap: tick_cap
                .or(self.benchmark.tick_cap)
                .unwrap_or(plan::TICK_CAP),
        });
        info!("starting the benchmark");
        match self.launch_race() {
            Ok(()) => true,
            Err(error) => {
                warn!("cannot start the benchmark: {error:#}");
                self.abandon_benchmark_launch();
                false
            }
        }
    }

    /// A benchmark whose race did not build: the player's own options come back and nothing
    /// is left to turn the next ordinary race into a benchmark. A run started on the command
    /// line has nothing left to do and ends.
    pub(crate) fn abandon_benchmark_launch(&mut self) {
        let Some(launch) = self.benchmark.launching.take() else {
            return;
        };
        self.benchmark.cli = None;
        self.restore_benchmark_player();
        self.quit |= launch.origin == Origin::Cli;
    }

    /// The settings the preset `name` sets, or why there is no such preset.
    fn benchmark_preset(&self, name: &str) -> Result<Vec<(String, String)>, String> {
        let presets = self
            .shell
            .as_ref()
            .and_then(|shell| shell.benchmark_presets.as_ref());
        match presets.and_then(|presets| presets.get(name)) {
            Some(preset) => Ok(preset.settings.clone()),
            None => Err(format!(
                "no benchmark preset called {name:?} for this title; there are: {}",
                presets.map_or_else(|| "none".to_string(), |p| p.names().join(", "))
            )),
        }
    }

    /// Writes `settings.toml`, except while a benchmark preset is in effect.
    pub(crate) fn save_settings(&self) -> anyhow::Result<()> {
        self.benchmark.save(&self.settings)
    }

    /// Puts the player's own options, autopilot flag and (after a preset) settings back.
    fn restore_benchmark_player(&mut self) {
        if let Some(saved) = self.benchmark.saved.take() {
            self.race_options = saved.race_options;
            self.autopilot = saved.autopilot;
            let pinned = saved.pinned;
            self.autopilot_pilot = pinned.autopilot_pilot;
            self.autopilot_skill = pinned.autopilot_skill;
            self.campaign_cell = pinned.campaign_cell;
            self.campaign_custom = pinned.campaign_custom;
            self.campaign_difficulty = pinned.campaign_difficulty;
            self.campaign_ai_skill_scale = pinned.campaign_ai_skill_scale;
            self.settings = saved.settings;
            // The one preset key with a runtime field of its own beside the setting.
            self.anisotropy = self.settings.graphics.anisotropy;
            // The surface was configured for the run's vsync.
            self.gpu
                .output
                .set_vsync(&self.gpu.device, self.settings.display.vsync);
            self.display.reset();
            self.stalled = true;
        }
    }

    /// One drawn frame of a benchmark race: `frame_seconds` is what the performance meter was
    /// fed (`None` on a frame that carried a load), `ticks` the simulation steps it took.
    pub(crate) fn record_benchmark_frame(&mut self, frame_seconds: Option<f32>, ticks: u32) {
        let Stage::Race(stage) = &mut self.stage else {
            return;
        };
        let (Some(live), Some(seconds)) = (stage.benchmark.as_mut(), frame_seconds) else {
            return;
        };
        let world = &stage.race.core.sim.world;
        // The flyby has no tick; the countdown's frames are recorded and left out of every
        // figure (`benchmark::report`).
        if world.tick > 0 {
            live.recorder
                .frame(seconds, world.race[0].lap, ticks, world.tick);
        }
    }

    /// One tick of the benchmark, before the stage's own: notices the race's end on the tick it
    /// happened, and reads the stats screen's input. Returns whether the stage changed.
    pub(crate) fn tick_benchmark(
        &mut self,
        pointer: &Pointer,
        rect: (f32, f32, f32, f32),
        back: Button,
    ) -> bool {
        if matches!(&self.stage, Stage::Menu(_))
            && self.benchmark.cli.is_some()
            && self.benchmark.launching.is_none()
            && self.attract_eligible()
        {
            let request = self.benchmark.cli.take().unwrap_or_default();
            return self.launch_benchmark(Origin::Cli, request, None);
        }
        // The run's one input: BACK abandons it.
        if self.in_benchmark() && self.controls.take_presses(back) != 0 {
            self.escape_benchmark();
            return true;
        }
        let Stage::Race(stage) = &mut self.stage else {
            return false;
        };
        let Some(live) = stage.benchmark.as_mut() else {
            return false;
        };
        if live.done.is_some() {
            return self.tick_benchmark_screen(pointer, rect, back);
        }
        if live.recorder.ended() {
            return false;
        }
        let world = &stage.race.core.sim.world;
        let finished = stage.race.core.sim.finished();
        if finished || world.tick >= u64::from(live.tick_cap) {
            live.recorder.end(Ending {
                tick: world.tick,
                state_hash: stage.race.core.sim.state_hash(),
                finished,
                flyby_ticks: stage.race.intro_ticks(),
            });
            self.finish_benchmark();
        }
        false
    }

    /// The stats screen's input: the pad moves the focus and presses it, a click or a touch
    /// presses the button under it, and the back button leaves.
    fn tick_benchmark_screen(
        &mut self,
        pointer: &Pointer,
        rect: (f32, f32, f32, f32),
        back: Button,
    ) -> bool {
        self.age_copy_label();
        let grid = super::pointer::in_grid(*pointer, oag_display::space::Space::PSP, rect, 1.0);
        let buttons = self.controls.buttons_mut();
        let mut chosen = None;
        let mut take = |press: Button| {
            let pressed = buttons.is_pressed(press);
            buttons.consume_press(press);
            pressed
        };
        let forward = take(Button::Right) | take(Button::Down);
        let backward = take(Button::Left) | take(Button::Up);
        let confirmed = take(Button::Cross) | take(Button::Start);
        let left_back = take(back);
        let Stage::Race(stage) = &mut self.stage else {
            return false;
        };
        let Some(done) = stage.benchmark.as_mut().and_then(|live| live.done.as_mut()) else {
            return false;
        };
        let can_copy = done.view.can_copy;
        if forward || backward {
            done.focus = screen::step(done.focus, forward && !backward, can_copy);
        }
        if confirmed {
            chosen = Some(done.focus);
        }
        if left_back {
            chosen = Some(Choice::Back);
        }
        if grid.clicked
            && let Some((x, y)) = grid.at
            && let Some(choice) = screen::hit(x, y, can_copy)
        {
            chosen = Some(choice);
        }
        if let Some(choice) = chosen {
            done.focus = choice;
        }
        match chosen {
            Some(Choice::Details) => {
                done.view.details = !done.view.details;
                false
            }
            Some(Choice::Copy) => {
                self.copy_benchmark_result();
                false
            }
            Some(Choice::Back) => {
                self.leave_benchmark();
                true
            }
            Some(Choice::Again) => {
                let (origin, request, cap) = self.benchmark_run();
                self.launch_benchmark(origin, request, Some(cap))
            }
            None => false,
        }
    }

    /// COPY TO CLIPBOARD: puts the result's text on the clipboard and shows how it went.
    fn copy_benchmark_result(&mut self) {
        let Stage::Race(stage) = &mut self.stage else {
            return;
        };
        let Some(done) = stage.benchmark.as_mut().and_then(|live| live.done.as_mut()) else {
            return;
        };
        let strings = &done.strings;
        let say = |id: &str, english: &str| strings.get_or_fallback(id, english).to_string();
        let text = benchmark::text::copy_text(&done.report, &say);
        debug!("benchmark result: asking for a clipboard write");
        done.copy.start(crate::app::clipboard::write(&text));
        done.view.copied = done.copy.tick();
    }

    /// Puts the write's answer on the button when it arrives, and takes it off in time.
    fn age_copy_label(&mut self) {
        let Stage::Race(stage) = &mut self.stage else {
            return;
        };
        if let Some(done) = stage.benchmark.as_mut().and_then(|live| live.done.as_mut()) {
            done.view.copied = done.copy.tick();
        }
    }

    fn benchmark_origin(&self) -> Origin {
        match &self.stage {
            Stage::Race(stage) => stage
                .benchmark
                .as_ref()
                .map_or(Origin::Extras, |live| live.origin),
            _ => Origin::Extras,
        }
    }

    /// What to launch again: where it started from, what it was asked for and the tick cap.
    fn benchmark_run(&self) -> (Origin, Request, u32) {
        match &self.stage {
            Stage::Race(stage) => stage.benchmark.as_ref().map_or_else(
                || (Origin::Extras, Request::default(), plan::TICK_CAP),
                |live| (live.origin, live.request.clone(), live.tick_cap),
            ),
            _ => (Origin::Extras, Request::default(), plan::TICK_CAP),
        }
    }

    /// Abandons the benchmark (Escape, the pad's back button, the system Back): the player's
    /// settings come back and no result is written. The run's only input.
    pub(crate) fn escape_benchmark(&mut self) {
        debug!("benchmark abandoned");
        self.leave_benchmark();
    }

    /// Back to where the benchmark was started from: the command line quits, a menu row
    /// reopens the menus on the page it left.
    fn leave_benchmark(&mut self) {
        let origin = self.benchmark_origin();
        let trail = self
            .benchmark
            .saved
            .as_ref()
            .map(|saved| saved.trail.clone())
            .unwrap_or_default();
        self.restore_benchmark_player();
        self.audio.pause_race_music();
        self.audio.stop_race_sfx();
        if origin == Origin::Cli {
            self.quit = true;
            return;
        }
        if let Err(error) = self.open_menus() {
            log::error!("cannot return to the menus: {error:#}");
            self.quit = true;
            return;
        }
        if let Stage::Menu(stage) = &mut self.stage
            && !trail.is_empty()
        {
            stage.menu.restore_trail(&trail);
        }
    }

    /// The race has ended: builds the report, writes the result file and raises the screen.
    fn finish_benchmark(&mut self) {
        let Some(report) = self.benchmark_report() else {
            return;
        };
        let saved = write_result(&report);
        if let Some(path) = &saved {
            info!("benchmark result written to {path}");
        }
        if let Stage::Race(stage) = &mut self.stage
            && let Some(live) = stage.benchmark.as_mut()
        {
            live.done = Some(Done {
                report,
                focus: Choice::Back,
                saved,
                strings: oag_ui::strings::project_table(self.settings.language.as_deref()),
                view: screen::View {
                    can_copy: crate::app::clipboard::CAN_WRITE,
                    ..screen::View::default()
                },
                copy: benchmark::CopyLabel::default(),
            });
        }
    }

    /// [`DETAIL_KEYS`] as `(row name, value)` in the words the settings pages show: the row's own
    /// label and the label of the value the row is on, read out of the menu tree the player
    /// browses, so a name edited there is edited here.
    fn benchmark_details(&self) -> Vec<(String, String)> {
        let Some(shell) = self.shell.as_ref() else {
            return Vec::new();
        };
        let rect = self.scene_rect(self.gpu.output.size());
        let seeds = crate::settings::menu_seeds(
            &self.settings,
            self.anisotropy,
            shell.title,
            shell.platform,
        );
        // The rows that follow the screen's refresh rate are labelled with the rate they resolve to.
        let relabels = self.rate_labels();
        DETAIL_KEYS
            .iter()
            .filter_map(|key| {
                let value = if *key == "graphics.render_scale" {
                    self.render_scale_for(rect).to_string()
                } else {
                    seeds
                        .iter()
                        .find(|(seeded, _)| seeded == key)?
                        .1
                        .to_string()
                };
                let entry = shell
                    .definition
                    .pages
                    .iter()
                    .flat_map(|page| page.entries.iter())
                    .find(|entry| entry.setting() == Some(key))?;
                let values = match entry {
                    oag_ui::menu::Entry::Choice { values, .. } => values.as_slice(),
                    _ => &[],
                };
                Some((
                    entry.label().to_string(),
                    shown_value(key, value, values, &relabels),
                ))
            })
            .collect()
    }

    fn benchmark_report(&self) -> Option<Report> {
        let Stage::Race(stage) = &self.stage else {
            return None;
        };
        let live = stage.benchmark.as_ref()?;
        let ending = live.recorder.ending()?;
        let title = self.shell.as_ref()?.title;
        let world = &stage.race.core.sim.world;
        let profile = self.render_profile();
        let config = &self.gpu.output.config;
        let mut settings = vec![
            (
                "size".to_string(),
                format!("{}x{}", config.width, config.height),
            ),
            ("present".to_string(), format!("{:?}", config.present_mode)),
            (
                "render_scale".to_string(),
                plain(&format!("{:?}", profile.render_scale)),
            ),
            (
                "upscaler".to_string(),
                format!("{:?}", profile.reconstruction),
            ),
            ("msaa".to_string(), format!("{:?}", profile.msaa)),
            ("shadows".to_string(), format!("{:?}", profile.shadows)),
            (
                "motion_blur".to_string(),
                format!("{:?}", profile.motion_blur),
            ),
            (
                "motion_blur_resolution".to_string(),
                profile.motion_blur_resolution.name().to_string(),
            ),
            // Under `auto`, the step the run ended at as well.
            (
                "reflection_resolution".to_string(),
                if profile.reflection_resolution.is_auto() {
                    format!("auto, at {}", self.drs.reflection_step())
                } else {
                    profile.reflection_resolution.to_string()
                },
            ),
            (
                "anisotropy".to_string(),
                format!("{:?}", self.settings.graphics.anisotropy),
            ),
            (
                "model_detail".to_string(),
                format!("{:?}", profile.model_detail),
            ),
            (
                "texture_detail".to_string(),
                format!("{:?}", profile.texture_detail),
            ),
            (
                "trail_order".to_string(),
                format!("{:?}", profile.trail_order),
            ),
            (
                "scenery_sort".to_string(),
                format!("{:?}", profile.scenery_sort),
            ),
            (
                "trail_length".to_string(),
                format!("{:?}", profile.trail_length),
            ),
            ("weather".to_string(), format!("{:?}", profile.weather)),
            (
                "frustum_culling".to_string(),
                format!("{:?}", self.settings.graphics.frustum_culling),
            ),
            (
                "pvs_culling".to_string(),
                format!("{:?}", self.settings.graphics.pvs_culling),
            ),
            (
                "camera_view".to_string(),
                format!("{:?}", self.settings.graphics.camera_view),
            ),
            (
                "fov".to_string(),
                plain(&format!("{:?}", self.settings.graphics.fov)),
            ),
            ("autopilot".to_string(), "true".to_string()),
        ];
        settings.extend([
            (
                "vsync".to_string(),
                format!("{:?}", self.settings.display.vsync),
            ),
            (
                "frame_limit".to_string(),
                String::from(self.settings.display.frame_limit),
            ),
            ("target_fps".to_string(), String::from(profile.target_fps)),
        ]);
        settings.retain(|(_, value)| !value.is_empty());
        let info = self.gpu.adapter_info();
        let refresh = self
            .display
            .hz()
            .map_or((60.0, "assumed"), |hz| (f64::from(hz), "display"));
        let mut command = vec!["--benchmark".to_string()];
        if let Some(preset) = &live.request.preset {
            command.push("--preset".to_string());
            command.push(preset.clone());
        }
        command.push("--benchmark-mods".to_string());
        command.push(if live.request.mods { "on" } else { "off" }.to_string());
        let mods = live.request.mods.then(|| {
            let used = world_mods_used(stage);
            self.shell
                .as_ref()
                .map(|shell| shell.mods.as_slice())
                .unwrap_or_default()
                .iter()
                .filter(|listing| is_loaded_mod(listing))
                .map(|listing| ModLoaded {
                    id: listing.id.clone(),
                    version: listing.version.clone(),
                    hash: used
                        .iter()
                        .find(|used| used.id == listing.id)
                        .map(|used| used.sha256.clone())
                        .unwrap_or_default(),
                })
                .collect()
        });
        let context = benchmark::Context {
            title: title.name,
            track: Some(format!("{}: {}", title.name, stage.circuit)),
            track_name: self.shell.as_ref().and_then(|shell| {
                shell
                    .tracks
                    .iter()
                    .find(|(track, _)| track.entry_name() == stage.circuit)
                    .map(|(_, name)| name.clone())
            }),
            details: self.benchmark_details(),
            build_version: oag_net::BUILD_VERSION,
            mode: world.mode().name(),
            laps_target: world.laps_target(),
            adapter: benchmark::report::adapter_json(&info, false),
            adapter_name: info.name.clone(),
            driver: info.driver.clone(),
            device: benchmark::device_label(),
            settings,
            refresh,
            frame_limit_hz: self.frame_period().map(|period| 1.0 / period.as_secs_f64()),
            target_fps_hz: profile
                .target_fps
                .resolve(self.display.hz())
                .hz()
                .map(f64::from),
            preset: live.request.preset.clone(),
            mods,
            vsync: self.settings.display.vsync.to_string(),
            present_mode: format!("{:?}", config.present_mode),
            frame_limit: self.settings.display.frame_limit.to_string(),
            origin: live.origin.name(),
            command,
            load: (
                live.recorder.load_start().to_string(),
                benchmark::load_average(),
            ),
            git_hash: oag_net::GIT_HASH,
            wall_seconds: live.recorder.wall_seconds(),
            people: 1,
            tick_cap: live.tick_cap,
        };
        Some(Report {
            context,
            frames: live.recorder.frames().to_vec(),
            ending,
        })
    }
}

/// The settings that change what a frame costs, by the menu's own setting keys, in the order the
/// stats screen's DETAILS lists them.
const DETAIL_KEYS: [&str; 18] = [
    "graphics.render_scale",
    "graphics.reconstruction",
    "graphics.msaa",
    "graphics.shadows",
    "graphics.motion_blur",
    "graphics.motion_blur_resolution",
    "graphics.anisotropy",
    "graphics.model_detail",
    "graphics.texture_detail",
    "graphics.trail_order",
    "graphics.scenery_sort",
    "graphics.trail_length",
    "graphics.weather",
    "graphics.camera_view",
    "graphics.fov",
    "display.vsync",
    "display.frame_limit",
    "graphics.target_fps",
];

/// The words the settings page shows for `value` of the row `key`: the label of the choice it is,
/// or the rate the value follows (`relabels`, as `Session::rate_labels` has them), or the value.
fn shown_value(
    key: &str,
    value: String,
    values: &[oag_ui::menu::Choice],
    relabels: &[(&str, &str, String)],
) -> String {
    if let Some((_, _, label)) = relabels
        .iter()
        .find(|(setting, token, _)| *setting == key && *token == value)
    {
        return label.clone();
    }
    values
        .iter()
        .find(|choice| choice.value == value)
        .map_or(value, |choice| choice.label.clone())
}

/// Whether `listing` is a mod of the player's that is on and opened: one a run with mods loads.
fn is_loaded_mod(listing: &crate::mods::Listing) -> bool {
    !listing.builtin && listing.error.is_none() && listing.enabled == Some(true)
}

/// The simulation modules the race on `stage` ran.
fn world_mods_used(stage: &crate::app::race_stage::RaceStage) -> Vec<oag_replay::header::ModUse> {
    stage.race.core.sim.mods_used().to_vec()
}

/// A percentage type's debug text (`Scale(100)`) as the number it carries (`100%`); anything
/// else as it is.
fn plain(debug: &str) -> String {
    debug
        .split_once('(')
        .and_then(|(_, rest)| rest.strip_suffix(')'))
        .filter(|inner| inner.chars().all(|c| c.is_ascii_digit()))
        .map_or_else(|| debug.to_string(), |inner| format!("{inner}%"))
}

/// Writes the result under the user data directory, and says where.
fn write_result(report: &Report) -> Option<String> {
    let dir = benchmark::dir()?.join(benchmark::run_id());
    let key = benchmark::title_key(report.context.title);
    let write = || -> std::io::Result<String> {
        std::fs::create_dir_all(&dir)?;
        let json = dir.join(format!("{key}.json"));
        crate::profile::write(&json, report.json())?;
        crate::profile::write(dir.join(format!("{key}.csv")), report.csv())?;
        Ok(json.display().to_string())
    };
    match write() {
        Ok(path) => Some(path),
        Err(error) => {
            warn!("could not write the benchmark result: {error}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn choice(value: &str, label: &str) -> oag_ui::menu::Choice {
        oag_ui::menu::Choice {
            value: value.into(),
            label: label.into(),
            unavailable: None,
        }
    }

    /// DETAILS and the copied text say what the settings page says, never the stored token.
    #[test]
    fn a_detail_reads_as_the_settings_page_draws_it() {
        let values = [choice("off", "OFF"), choice("refresh_rate", "refresh_rate")];
        let relabels = [(
            "display.frame_limit",
            "refresh_rate",
            "REFRESH RATE (144)".to_string(),
        )];
        let shown = |key: &str, value: &str| shown_value(key, value.into(), &values, &relabels);
        assert_eq!(
            shown("display.frame_limit", "refresh_rate"),
            "REFRESH RATE (144)"
        );
        assert_eq!(shown("display.frame_limit", "off"), "OFF");
        assert_eq!(shown("graphics.target_fps", "refresh_rate"), "refresh_rate");
        assert_eq!(shown("display.frame_limit", "240"), "240");
    }
}
