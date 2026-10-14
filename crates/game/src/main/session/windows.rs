//! A `--windows` / `--window-seats` run's further windows: opening them, closing them, and
//! drawing them.
//!
//! The design is `docs/rendering/multi-window.md`. Window 1 is drawn by
//! [`Session::draw`] as ever; [`Session::draw_screens`] runs after it and draws each further
//! window. A further window of a race that is windowed draws the same chain as window 1 -
//! scene, resolve, HUD, bloom, grade - on a framebuffer of its own, **recorded into an
//! encoder of its own and submitted before the next window**, because a queue write lands
//! before the submission that follows it (`Scene::render_viewports` has the long form): two
//! windows in one submission would draw with the last window's uniforms. What a window draws
//! is the seats the race gives it (`Race::windows`): one whole-screen view, or several cut up
//! like a split screen. Anything else gets a holding frame.

use anyhow::Result;
use log::{info, warn};

use oag_display::display;
use oag_present::perf;
use oag_present::upscale;
use oag_ui::frontend::{Align, Draw};
use oag_ui::strings;

use crate::stage::Stage;
use crate::windows::{self, Screen};

use super::Session;

/// A holding frame's text colour and scale: the performance overlay's own, large.
const HOLD_TEXT: [f32; 4] = [0.92, 0.95, 1.0, 1.0];
const HOLD_SCALE: f32 = 3.0;

impl Session {
    /// How many windows to open now, or `None`: `--windows` or `--window-seats` asked for them, none are open
    /// yet, the title is known, and the title offers a split on this adapter - the same
    /// rule `--players` follows ([`oag_game::split_screen::offered`]). A refusal is said
    /// once and ends the request.
    #[cfg_attr(
        any(target_arch = "wasm32", target_os = "android", target_os = "ios"),
        allow(dead_code)
    )]
    pub(crate) fn screens_to_open(&mut self) -> Option<u8> {
        if self.windows_wanted < 2 {
            return None;
        }
        let title = self
            .shell
            .as_ref()
            .map(|shell| shell.title)
            .or(self.race_title)?;
        let wanted = std::mem::take(&mut self.windows_wanted);
        if !oag_game::split_screen::offered(title, self.gpu.device_type) {
            warn!(
                "--windows {wanted}: split screen is not offered for {} on an integrated GPU, \
                 so one window",
                title.name
            );
            return None;
        }
        // Windows are already open for as many (a layout the picker chose after a race on
        // another one): nothing to add.
        (usize::from(wanted) > 1 + self.screens.len()).then_some(wanted)
    }

    /// Opens the windows after the ones already open, up to `count` in all, on the one
    /// device, each with a framebuffer of its own at window 1's allocation. Window `k` shows the seats the layout gives it. A window
    /// that cannot open is said and skipped (and leaves the layout, so the windows after it
    /// keep their seats): the run goes on with the windows it has.
    #[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
    pub(crate) fn open_screens(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, count: u8) {
        use anyhow::Context;

        self.screens_opened = true;
        let monitors: Vec<_> = event_loop.available_monitors().collect();
        let size = self.settings.display.window_size;
        let origin = self.gpu.output.window.outer_position().ok();
        let scale = self.gpu.output.window.scale_factor();
        if self.windows_fullscreen
            && self.window_monitors.is_empty()
            && let Some(monitor) = monitors.first()
        {
            self.gpu
                .output
                .window
                .set_fullscreen(Some(winit::window::Fullscreen::Borderless(Some(
                    monitor.clone(),
                ))));
        }
        for _ in 1 + self.screens.len()..usize::from(count) {
            // The place in the layout: window 1 and the windows already open come before it.
            let window = 1 + self.screens.len();
            let (monitor, position) = self.seat_placement(window, &monitors, origin, scale);
            let title = self.window_title(window);
            let opened = self
                .gpu
                .open_output(
                    event_loop,
                    &title,
                    winit::dpi::LogicalSize::new(size.width, size.height),
                    monitor,
                    self.windows_fullscreen,
                    position,
                    crate::window::seat_present_modes(),
                )
                .and_then(|output| {
                    let framebuffer = upscale::Framebuffer::new(
                        &self.gpu.device,
                        output.config.format,
                        self.framebuffer.allocation(),
                    )
                    .context("building a further window's upscale pipeline")?;
                    Ok(Screen {
                        output,
                        framebuffer,
                        acquire: perf::Meter::new(),
                        present: perf::Meter::new(),
                    })
                });
            match opened {
                Ok(screen) => {
                    info!(
                        "window {}: {title} ({}x{})",
                        window + 1,
                        screen.output.config.width,
                        screen.output.config.height,
                    );
                    self.screens.push(screen);
                }
                Err(e) => {
                    warn!("window {}: {e:#}", window + 1);
                    // Its seats go to the windows that did open, dealt anew at the launch.
                    if window < self.window_layout.len() {
                        self.window_layout.remove(window);
                    }
                }
            }
        }
    }

    /// Where window `window` (from 1 for the first further window) goes: the monitor it is
    /// asked to sit on and the position it is asked for.
    ///
    /// A layout the picker chose names a monitor per window (`window_monitors`): a name this
    /// machine no longer has is the first monitor. A windowed window is centred on its
    /// monitor and stepped aside for each window already there; fullscreen needs no
    /// position. With no choice (`--windows`) a window sits beside window 1, or on the
    /// platform's `window`-th monitor when `--windows-fullscreen`. **Chosen, not measured**,
    /// and a request the compositor may refuse (Wayland places a plain window itself).
    #[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
    fn seat_placement(
        &self,
        window: usize,
        monitors: &[winit::monitor::MonitorHandle],
        origin: Option<winit::dpi::PhysicalPosition<i32>>,
        scale: f64,
    ) -> (
        Option<winit::monitor::MonitorHandle>,
        Option<winit::dpi::PhysicalPosition<i32>>,
    ) {
        let size = self.settings.display.window_size;
        let names = crate::window::monitor_names(monitors);
        let chosen = |index: usize| {
            self.window_monitors.get(index).map(|name| {
                names
                    .iter()
                    .position(|candidate| candidate == name)
                    .unwrap_or(0)
            })
        };
        if let Some(index) = chosen(window - 1) {
            let Some(monitor) = monitors.get(index) else {
                return (None, None);
            };
            if self.windows_fullscreen {
                return (Some(monitor.clone()), None);
            }
            // Window 1 is on the monitor the DISPLAY page names, the first one by default.
            // A window for that same monitor goes beside window 1 as a `--windows` one does,
            // rather than on top of it; one for another monitor is centred there and stepped
            // aside for each window already on it.
            let first = self.settings.display.monitor.choose(&names).unwrap_or(0);
            if first != index {
                let aside = (0..window - 1)
                    .filter(|&other| chosen(other) == Some(index))
                    .count();
                let mut at = crate::window::centred_on(monitor, size);
                at.x += 48 * aside as i32;
                at.y += 32 * aside as i32;
                return (Some(monitor.clone()), Some(at));
            }
        }
        let monitor = self
            .windows_fullscreen
            .then(|| monitors.get(window % monitors.len().max(1)).cloned())
            .flatten();
        // Beside window 1 where the platform lets a program say so (X11); the compositor
        // places it otherwise (Wayland), which is why this is a request and not a layout.
        let position = origin.map(|origin| {
            let step = f64::from(size.width) * scale;
            winit::dpi::PhysicalPosition::new(
                origin.x + (step * window as f64) as i32 + 16 * window as i32,
                origin.y,
            )
        });
        (monitor, position)
    }

    /// What window `window` (0 is window 1) is called: the game's name and the seats it is
    /// planned to hold (`PLAYER 5 - 6`).
    #[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
    pub(super) fn window_title(&self, window: usize) -> String {
        let table = strings::project_table(self.settings.language.as_deref());
        let word = table.get("OAG_JOIN_PLAYER").unwrap_or("PLAYER");
        let (first, held) = self.planned_seats(window);
        format!("{} - {}", crate::hints::title(&table), windows::label(word, first, held))
    }

    /// The seats window `window` (0 is window 1) is planned to hold before a race has dealt
    /// them: the first slot and how many, from the layout the command line asked for (one
    /// person per window when it spelled none). What a holding frame and a window's title say.
    pub(crate) fn planned_seats(&self, window: usize) -> (usize, usize) {
        let counts = |index: usize| self.window_layout.get(index).map_or(1, |seats| seats.count);
        ((0..window).map(counts).sum(), counts(window))
    }

    /// How many people the open windows are planned to seat: the layout's counts over window 1
    /// and the further windows still open.
    fn planned_people(&self) -> usize {
        (0..=self.screens.len())
            .map(|window| self.planned_seats(window).1)
            .sum()
    }

    /// Seats everyone the windows are planned for a race launched from the menus, when nobody
    /// was seated otherwise: the join screen's lobby and a restart's kept seats (both of which
    /// say how many people race) win, and so does a campaign race, which is one person's and
    /// keeps its records. Otherwise the keyboard and the first pad take slot 0 and each
    /// further pad the next slot, as `--players N` seats them, and the grid fields opponents
    /// for the others to race. **Chosen, not measured.**
    pub(crate) fn seat_the_windows(&mut self, options: &mut oag_raceplay::Options) {
        let people = self.planned_people();
        if self.screens.is_empty()
            || people < 2
            || self.split_lobby.is_some()
            || self.controls.assignment().humans().is_some()
            || self.campaign_cell.is_some()
            || self.campaign_difficulty.is_some()
        {
            return;
        }
        self.controls.seat_default(people);
        options.opponents = true;
    }

    /// The ids of the further windows, in layout order (window 2 first).
    pub(crate) fn screen_ids(&self) -> Vec<winit::window::WindowId> {
        self.screens
            .iter()
            .map(|screen| screen.output.window.id())
            .collect()
    }

    /// Closes further window `index` (window `index + 2`): the window goes, and so does every
    /// person at it - their slots are flown by the AI from the next tick
    /// (`split_screen::follow_devices` reads `Assignment::drives`). The window leaves the race's
    /// layout and the planned one, so the windows after it keep their seats.
    pub(crate) fn close_screen(&mut self, index: usize) {
        if index >= self.screens.len() {
            return;
        }
        let window = index + 1;
        let screen = self.screens.remove(index);
        self.focus.forget(screen.output.window.id());
        let race = match &mut self.stage {
            Stage::Race(stage) => Some(&mut stage.race),
            _ => self.suspended_race.as_mut().map(|stage| &mut stage.race),
        };
        let mut slots = Vec::new();
        if let Some(race) = race {
            slots = race
                .windows()
                .get(window)
                .map(|views| views.seats.clone())
                .unwrap_or_default();
            race.remove_window(window);
        }
        for slot in &slots {
            self.controls.assignment_mut().unseat(*slot);
        }
        if window < self.window_layout.len() {
            self.window_layout.remove(window);
        }
        info!("window {} closed: slot(s) {slots:?} flown by the AI", window + 1);
    }

    /// Further window `index` was resized to `size`: its surface follows; its framebuffer
    /// follows on the next draw. A window with no area (minimised) is left alone, and
    /// **never pauses the race**: only window 1 does that.
    pub(crate) fn resize_screen(&mut self, index: usize, size: winit::dpi::PhysicalSize<u32>) {
        let Some(screen) = self.screens.get_mut(index) else {
            return;
        };
        if size.width == 0 || size.height == 0 {
            return;
        }
        screen.output.config.width = size.width;
        screen.output.config.height = size.height;
        screen.output.configure(&self.gpu.device);
    }

    /// The render target the further windows of their sizes want, for the allocation they
    /// share with window 1 ([`windows::shared_allocation`]).
    pub(super) fn screen_targets(&self) -> Vec<(u32, u32)> {
        self.screens
            .iter()
            .map(|screen| self.target_for(screen.output.size()))
            .collect()
    }

    /// The render target a window of `surface` pixels wants at the profile's render scale.
    pub(super) fn target_for(&self, surface: (u32, u32)) -> (u32, u32) {
        let rect = display::viewport(surface, self.settings.display.aspect);
        oag_game::settings::web::render_target(
            rect,
            self.render_profile().render_scale,
            self.gpu.device.limits().max_texture_dimension_2d,
        )
    }

    /// Whether the race on screen (or parked behind the pause menu) is dealt across windows.
    pub(super) fn windowed_race(&self) -> bool {
        match &self.stage {
            Stage::Race(stage) => stage.race.windowed(),
            Stage::Menu(_) => self
                .suspended_race
                .as_ref()
                .is_some_and(|stage| stage.race.windowed()),
            _ => false,
        }
    }

    /// Draws every further window once, after window 1's frame. A window that fails to draw is
    /// said once per failure and does not end the run.
    pub(crate) fn draw_screens(&mut self) {
        if self.screens.is_empty() {
            return;
        }
        // Out of `self` for the loop: drawing a window borrows the stage, the devices and the
        // settings beside it, none of which are the windows.
        let mut screens = std::mem::take(&mut self.screens);
        for (index, screen) in screens.iter_mut().enumerate() {
            if let Err(e) = self.draw_screen(index + 1, screen) {
                warn!("window {}: {e:#}", index + 2);
            }
        }
        self.screens = screens;
        if self.frame_index.is_multiple_of(windows::PACING_LOG_FRAMES) {
            self.log_pacing();
        }
    }

    /// What each window's acquire and present cost, over the meters' window of frames: the
    /// pacing measurement of `docs/rendering/multi-window.md`.
    fn log_pacing(&self) {
        let ms = |meter: &perf::Meter| meter.stats().map_or(f32::NAN, |stats| stats.mean_ms);
        let mut line = format!(
            "windows: frame {:.2} ms, window 1 acquire+present {:.2} ms",
            self.meter.stats().map_or(f32::NAN, |stats| stats.mean_ms),
            ms(&self.present_cost),
        );
        for (index, screen) in self.screens.iter().enumerate() {
            line += &format!(
                ", window {} acquire {:.2} ms present {:.2} ms ({:?})",
                index + 2,
                ms(&screen.acquire),
                ms(&screen.present),
                screen.output.config.present_mode,
            );
        }
        info!("{line}");
    }

    /// Window `window`'s frame (window 1 is 0, so this is 1 or more): the race's views of the
    /// seats the race gives it when the race is windowed and the window has some, a holding
    /// frame otherwise.
    fn draw_screen(&mut self, window: usize, screen: &mut Screen) -> Result<()> {
        let waited = web_time::Instant::now();
        if !screen.output.acquire(&self.gpu.device) {
            return Ok(());
        }
        screen.acquire.record(waited.elapsed().as_secs_f32());
        let Some(frame) = screen.output.take_acquired() else {
            return Ok(());
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("window frame"),
            });
        if self.window_has_seats(window) {
            self.encode_screen_race(window, screen, &mut encoder, &view);
        } else {
            self.encode_holding(window, screen, &mut encoder, &view);
        }
        self.gpu.queue.submit(Some(encoder.finish()));
        let presenting = web_time::Instant::now();
        self.gpu.queue.present(frame);
        screen.present.record(presenting.elapsed().as_secs_f32());
        Ok(())
    }

    /// Whether the race on screen (or parked behind the pause menu) gives window `window`
    /// something to draw.
    fn window_has_seats(&self, window: usize) -> bool {
        let race = match &self.stage {
            Stage::Race(stage) => Some(&stage.race),
            Stage::Menu(_) => self.suspended_race.as_ref().map(|stage| &stage.race),
            _ => None,
        };
        race.is_some_and(|race| race.windowed() && !race.window_drawn(window).is_empty())
    }

    /// Window `window`'s views of the race through the whole of window 1's chain: scene,
    /// resolve, HUD, bloom, grade. No upscaler history, blur or jitter (**chosen, not
    /// measured**: the windows alternate cameras every frame, and each history holds one),
    /// so FSR 3.1 takes the rung down, as it does on a menu frame.
    fn encode_screen_race(
        &mut self,
        window: usize,
        screen: &mut Screen,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        let profile = self.render_profile();
        let surface = screen.output.size();
        let rect = display::viewport(surface, self.settings.display.aspect);
        let wanted = self.target_for(surface);
        let device = &self.gpu.device;
        screen.framebuffer.resize_output(device, surface);
        // Window 1's allocation, the scene's own; this window's extent inside it.
        screen.framebuffer.resize(device, self.framebuffer.allocation());
        screen.framebuffer.set_extent(wanted);
        let extent = screen.framebuffer.extent();
        let inside = (0.0, 0.0, extent.0 as f32, extent.1 as f32);
        // The tick exactly while the race is held for any reason - paused and counting a
        // resume down alike (`pause::State::frozen`, the gate `Session::draw` reads for window
        // 1) - and between the last two ticks while it runs.
        let alpha = match &self.stage {
            Stage::Race(_) => {
                oag_raceplay::between_ticks::frame_alpha(&self.clock, !self.pause.frozen())
            }
            _ => 1.0,
        };
        let pvs_culling = self.pvs_culling();
        let preset = self.screen_filters.get(&profile.screen_filter).cloned();
        let native = self.native_size();
        let Some(stage) = (match &mut self.stage {
            Stage::Race(stage) => Some(stage),
            _ => self.suspended_race.as_mut(),
        }) else {
            return;
        };
        stage.race.set_draw_alpha(alpha);
        stage.render_window(
            &self.gpu,
            encoder,
            screen.framebuffer.view(),
            window,
            inside,
            self.settings.graphics.fov,
            self.settings.graphics.frustum_culling,
            pvs_culling,
            self.anim_seconds,
            profile.shadows,
            self.zone_hold.levels(),
        );
        let presentation = upscale::Presentation {
            reconstruction: profile.reconstruction,
            sharpness: profile.upscale_sharpness.stops(),
            brightness: self.settings.display.brightness,
            gamma: self.settings.display.gamma,
        };
        let _ = screen.framebuffer.resolve_scene(
            &self.gpu.device,
            &self.gpu.queue,
            encoder,
            rect,
            &presentation,
            None,
            None,
        );
        stage.draw_window_hud(
            &self.gpu,
            encoder,
            screen.framebuffer.output(),
            rect,
            screen.framebuffer.output_size(),
            window,
        );
        stage
            .scene
            .composite_bloom(encoder, screen.framebuffer.output(), rect);
        stage.race.set_draw_alpha(1.0);
        screen
            .framebuffer
            .set_screen_filter(&self.gpu.device, preset.as_ref());
        screen.framebuffer.composite(
            &self.gpu.device,
            &self.gpu.queue,
            encoder,
            view,
            upscale::Composite {
                brightness: self.settings.display.brightness,
                gamma: self.settings.display.gamma,
                screen: upscale::ScreenFrame {
                    native,
                    strength: profile.screen_filter_strength,
                },
            },
        );
    }

    /// The frame a window shows when the race is not drawing for it - the front end, a race of
    /// one person, a race in the middle of loading: black, and the join screen's own word for
    /// a person and the seats it is planned to hold. **Chosen, not measured**: the original has
    /// no second window.
    fn encode_holding(
        &mut self,
        window: usize,
        screen: &Screen,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        let table = strings::project_table(self.settings.language.as_deref());
        let word = table.get("OAG_JOIN_PLAYER").unwrap_or("PLAYER");
        let (first, held) = self.planned_seats(window);
        let surface = screen.output.size();
        let space = perf::grid(surface);
        self.overlay.set_space(space);
        let list = [Draw::Text {
            x: space.size.0 * 0.5,
            y: space.size.1 * 0.5 - 12.0,
            scale: HOLD_SCALE,
            color: HOLD_TEXT,
            border: None,
            align: Align::Centre,
            text: windows::label(word, first, held),
            wrap_width: None,
        }];
        self.overlay.render(
            &self.gpu.device,
            &self.gpu.queue,
            encoder,
            view,
            &list,
            (0.0, 0.0, surface.0 as f32, surface.1 as f32),
            None,
        );
    }
}
