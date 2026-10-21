//! The developer switches of `oag_game::devrun` on the live session: `--preseat` landing on
//! the WINDOW LAYOUT screen, `--script` tapping buttons on a timer, `--frame-log`'s line.
//!
//! All off by default. The clock here is the wall clock on purpose - the script and the log
//! describe the window, not the simulation.

use log::info;
use web_time::{Duration, Instant};

use oag_game::devrun::{self, Reading, Step};
use oag_gameplay::input::Button;
use oag_input::lobby::Outcome;

use crate::stage::Stage;

use super::Session;

/// How often the frame log speaks.
const LOG_EVERY: Duration = Duration::from_secs(1);

/// Wall-clock state of the switches.
#[derive(Debug, Default)]
pub(crate) struct Clock {
    started: Option<Instant>,
    /// When `--preseat` landed, which `+` script entries count from.
    anchor: Option<Instant>,
    last_log: Option<Instant>,
    /// Buttons tapped on the previous frame, released on this one.
    down: Vec<Button>,
    /// The sizes last logged, so a change is said when it happens.
    sizes: Option<((u32, u32), (u32, u32))>,
}

impl Session {
    /// Start of a frame: release last frame's taps, then run what is due.
    pub(crate) fn dev_frame(&mut self) {
        if self.dev.script.is_empty() && self.dev.preseat.is_none() {
            return;
        }
        let now = Instant::now();
        let started = *self.dev_clock.started.get_or_insert(now);
        for button in std::mem::take(&mut self.dev_clock.down) {
            self.controls.press_button(button, false);
        }
        if self.dev_preseat() {
            self.dev_clock.anchor = Some(now);
        }
        let anchored = self.dev_clock.anchor.map(|at| (now - at).as_secs_f32());
        for step in self.dev.script.due((now - started).as_secs_f32(), anchored) {
            match step {
                Step::Tap(button) => {
                    info!("script: tap {button:?} on {}", self.dev_stage_name());
                    self.controls.press_button(button, true);
                    self.dev_clock.down.push(button);
                }
                Step::SeatTap(seat, button) => {
                    info!("script: tap {button:?} for seat {seat}");
                    self.controls.press_keyboard_device(
                        oag_input::pad::KeyboardId(devrun::PLACEHOLDER_BASE + seat as u32),
                        button,
                    );
                }
                Step::Quit => {
                    info!("script: quit");
                    self.quit = true;
                }
            }
        }
    }

    /// `--preseat`: once the menus are up, seat the people on placeholder devices and press
    /// START on the join screen, which lands on WINDOW LAYOUT (or, where that screen does not
    /// apply, on the next step of the flow). The same two calls a pad's LOCAL MULTIPLAYER and
    /// START make, so nothing about the screens differs.
    fn dev_preseat(&mut self) -> bool {
        let Some(people) = self.dev.preseat else {
            return false;
        };
        let ready = matches!(&self.stage, Stage::Menu(stage)
            if stage.join.is_none() && stage.layout.is_none() && stage.picker.is_none())
            && self.shell.is_some();
        if !ready {
            return false;
        }
        self.dev.preseat = None;
        self.mp_devices = devrun::placeholder_devices(people);
        if let Stage::Menu(stage) = &mut self.stage {
            stage.menu.push("multiplayer");
        }
        info!("preseat: {people} people on placeholder devices");
        self.open_join();
        self.handle_join(Outcome::Start);
        true
    }

    /// The stage, and for the menus the screen over them, as the log names it.
    fn dev_stage_name(&self) -> String {
        match &self.stage {
            Stage::Launcher(_) => "launcher".into(),
            Stage::Loading(_) => "loading".into(),
            Stage::Frontend(_) => "frontend".into(),
            Stage::Race(_) if self.paused() => "race-paused".into(),
            Stage::Race(_) => "race".into(),
            Stage::Menu(stage) => {
                let screen = if let Some(open) = &stage.layout {
                    let windows = open.screen().windows();
                    let cuts: String = windows
                        .iter()
                        .map(|w| if w.vertical { 'v' } else { 'h' })
                        .collect();
                    format!("window-layout(windows={},cut={cuts})", windows.len())
                } else if stage.assign.is_some() {
                    "seat-assign".into()
                } else if stage.join.is_some() {
                    "join".into()
                } else if stage.picker.is_some() {
                    "picker".into()
                } else {
                    stage.menu.page().id.to_string()
                };
                format!("menu:{screen}")
            }
        }
    }

    /// End of a draw: once a second, the frame-time line, and a line the moment the window or
    /// surface size changes. `render` is the scene's extent when one was drawn.
    pub(crate) fn dev_log(&mut self, render: Option<(u32, u32)>) {
        if !self.dev.frame_log {
            return;
        }
        let now = Instant::now();
        let window = self.gpu.output.window.inner_size();
        let window = (window.width, window.height);
        let surface = self.gpu.output.size();
        let sizes = Some((window, surface));
        if self.dev_clock.sizes != sizes {
            info!(
                "frame-log size change on {}: window {}x{} surface {}x{}",
                self.dev_stage_name(),
                window.0,
                window.1,
                surface.0,
                surface.1
            );
            self.dev_clock.sizes = sizes;
        }
        if self
            .dev_clock
            .last_log
            .is_some_and(|last| now - last < LOG_EVERY)
        {
            return;
        }
        self.dev_clock.last_log = Some(now);
        let phases = self.phases.take_line();
        let Some(stats) = self.meter.stats() else {
            return;
        };
        if let Some(phases) = phases {
            info!("{phases}");
        }
        let ms = |s: Option<oag_present::perf::Stats>| s.map(|s| s.mean_ms);
        let stage = self.dev_stage_name();
        info!(
            "{}",
            devrun::format_line(&Reading {
                stage: &stage,
                fps: stats.fps,
                frame_ms: stats.mean_ms,
                p99_ms: stats.p99_ms,
                cpu_ms: ms(self.cpu_cost.stats()),
                gpu_ms: ms(self.scene_cost.stats()),
                present_ms: ms(self.present_cost.stats()),
                target_hz: self.presentation_hz(),
                window,
                surface,
                render,
            })
        );
    }
}
