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
use crate::benchmark::{self, Done, Live, Origin, Recorder, Report, Request, plan};
use oag_gameplay::input::Button;
use oag_ui::pointer::Pointer;

use crate::app::stage::Stage;

use super::Session;

/// What the session keeps of the benchmark between ticks.
#[derive(Default)]
pub(crate) struct State {
    /// `--benchmark`: asked for on the command line, launched once the menus are up.
    pub(crate) cli: Option<Request>,
    /// `--ticks` with `--benchmark`: where every benchmark of this run stops, menu launches
    /// included (a short run for a test).
    tick_cap: Option<u32>,
    /// The launch, from the call to the race's hand-over.
    launching: Option<Launch>,
    /// What the player had, set aside while a benchmark's stand in its place.
    saved: Option<Saved>,
}

struct Launch {
    origin: Origin,
    preset: Option<String>,
    tick_cap: u32,
}

struct Saved {
    race_options: Option<oag_raceplay::Options>,
    autopilot: bool,
    /// The settings as the player has them, put back when a preset was applied over them.
    settings: Option<crate::settings::Settings>,
    trail: Vec<oag_ui::menu::Spot>,
}

impl State {
    /// Whether a preset's settings are in effect, so nothing is written to `settings.toml` until
    /// the player's own are back.
    pub(crate) fn holds_settings(&self) -> bool {
        self.saved
            .as_ref()
            .is_some_and(|saved| saved.settings.is_some())
    }

    /// The benchmark the race being built is, if one is being launched: handed to the stage.
    pub(crate) fn take_launch(&mut self) -> Option<Live> {
        let launch = self.launching.take()?;
        Some(Live {
            origin: launch.origin,
            preset: launch.preset,
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
    /// Whether the race on screen is a benchmark that has not ended.
    pub(crate) fn in_benchmark(&self) -> bool {
        matches!(&self.stage, Stage::Race(stage)
            if stage.benchmark.as_ref().is_some_and(|live| live.done.is_none()))
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

    /// A BENCHMARK or RUN BENCHMARK row was fired: starts the benchmark with the player's own
    /// settings, and remembers the page it came from for the stats screen's BACK.
    pub(crate) fn start_benchmark_from_menu(&mut self) {
        let origin = match &self.stage {
            Stage::Menu(stage) if stage.menu.page().id == "graphics" => Origin::Graphics,
            _ => Origin::Extras,
        };
        self.launch_benchmark(origin, None, None);
    }

    /// Starts a benchmark from `origin` with the preset the request names. Returns whether the
    /// loading screen is up.
    pub(crate) fn launch_benchmark(
        &mut self,
        origin: Origin,
        preset: Option<String>,
        tick_cap: Option<u32>,
    ) -> bool {
        let Some(mut options) = self.race_options.clone() else {
            warn!("no disc image has been chosen yet: there is nothing to benchmark");
            return false;
        };
        plan::pin(&mut options);
        let preset_settings = match preset.as_deref() {
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
        let trail = match &self.stage {
            Stage::Menu(stage) => stage.menu.trail(),
            _ => Vec::new(),
        };
        let player = self.race_options.replace(options);
        // The first launch of a run keeps the player's own; a re-run replaces a benchmark's.
        if self.benchmark.saved.is_none() {
            self.benchmark.saved = Some(Saved {
                race_options: player,
                autopilot: self.autopilot,
                settings: preset_settings.is_some().then(|| self.settings.clone()),
                trail,
            });
        }
        for (key, value) in preset_settings.into_iter().flatten() {
            self.apply_setting(
                &format!("graphics.{key}"),
                &oag_ui::menu::Value::Text(value),
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
            preset,
            tick_cap: tick_cap
                .or(self.benchmark.tick_cap)
                .unwrap_or(plan::TICK_CAP),
        });
        info!("starting the benchmark");
        match self.launch_race() {
            Ok(()) => true,
            Err(error) => {
                warn!("cannot start the benchmark: {error:#}");
                self.benchmark.launching = None;
                self.benchmark.cli = None;
                self.restore_benchmark_player();
                false
            }
        }
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

    /// Puts the player's own options, autopilot flag and (after a preset) settings back.
    fn restore_benchmark_player(&mut self) {
        if let Some(saved) = self.benchmark.saved.take() {
            self.race_options = saved.race_options;
            self.autopilot = saved.autopilot;
            if let Some(settings) = saved.settings {
                self.settings = settings;
            }
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
        // The flyby and the grid before tick zero are not the race: the offscreen bench starts
        // on its first tick too.
        if world.tick > 0 {
            live.recorder.frame(seconds, world.race[0].lap, ticks);
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
            return self.launch_benchmark(Origin::Cli, request.preset, None);
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
        let grid = super::pointer::in_grid(*pointer, oag_display::space::Space::PSP, rect, 1.0);
        let buttons = self.controls.buttons_mut();
        let mut chosen = None;
        let mut take = |press: Button| {
            let pressed = buttons.is_pressed(press);
            buttons.consume_press(press);
            pressed
        };
        let moved = take(Button::Left) | take(Button::Right);
        let confirmed = take(Button::Cross) | take(Button::Start);
        let left_back = take(back);
        let Stage::Race(stage) = &mut self.stage else {
            return false;
        };
        let Some(done) = stage.benchmark.as_mut().and_then(|live| live.done.as_mut()) else {
            return false;
        };
        if moved {
            done.focus = match done.focus {
                Choice::Back => Choice::Again,
                Choice::Again => Choice::Back,
            };
        }
        if confirmed {
            chosen = Some(done.focus);
        }
        if left_back {
            chosen = Some(Choice::Back);
        }
        if grid.clicked
            && let Some((x, y)) = grid.at
            && let Some(choice) = screen::hit(x, y)
        {
            chosen = Some(choice);
        }
        match chosen {
            Some(Choice::Back) => {
                self.leave_benchmark();
                true
            }
            Some(Choice::Again) => {
                let (origin, preset, cap) = self.benchmark_run();
                self.launch_benchmark(origin, preset, Some(cap))
            }
            None => false,
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

    /// What to launch again: where it started from, the preset and the tick cap.
    fn benchmark_run(&self) -> (Origin, Option<String>, u32) {
        match &self.stage {
            Stage::Race(stage) => stage
                .benchmark
                .as_ref()
                .map_or((Origin::Extras, None, plan::TICK_CAP), |live| {
                    (live.origin, live.preset.clone(), live.tick_cap)
                }),
            _ => (Origin::Extras, None, plan::TICK_CAP),
        }
    }

    /// Escape in a benchmark race: it is abandoned, with no result.
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
            });
        }
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
        settings.retain(|(_, value)| !value.is_empty());
        let info = self.gpu.adapter_info();
        let refresh = self
            .display
            .hz()
            .map_or((60.0, "assumed"), |hz| (f64::from(hz), "display"));
        let mut command = vec!["--benchmark".to_string()];
        if let Some(preset) = &live.preset {
            command.push("--preset".to_string());
            command.push(preset.clone());
        }
        let context = benchmark::Context {
            title: title.name,
            track: Some(format!("{}: {}", title.name, stage.circuit)),
            mode: world.mode().name(),
            laps_target: world.laps_target(),
            adapter: benchmark::report::adapter_json(&info, false),
            adapter_name: info.name.clone(),
            driver: info.driver.clone(),
            device: benchmark::device_label(),
            settings,
            refresh,
            preset: live.preset.clone(),
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
        crate::profile::write(&dir.join(format!("{key}.csv")), report.csv())?;
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
