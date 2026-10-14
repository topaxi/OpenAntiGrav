//! A `--windows N` run's further windows: opening them, closing them, and drawing them.
//!
//! The design is `docs/rendering/multi-window.md`. Window 1 is drawn by
//! [`Session::draw`] as ever; [`Session::draw_seats`] runs after it and draws each seat.
//! A seat of a race that is windowed draws the same chain as window 1 - scene, resolve,
//! HUD, bloom, grade - on a framebuffer of its own, **recorded into an encoder of its own
//! and submitted before the next window**, because a queue write lands before the
//! submission that follows it (`Scene::render_viewports` has the long form): two windows
//! in one submission would draw with the last window's uniforms. Anything else gets a
//! holding frame.

use anyhow::{Context, Result};
use log::{info, warn};

use oag_display::display;
use oag_present::perf;
use oag_present::upscale;
use oag_ui::frontend::{Align, Draw};
use oag_ui::strings;

use crate::hints;
use crate::stage::Stage;
use crate::windows::{self, Seat};

use super::Session;

/// A holding frame's text colour and scale: the performance overlay's own, large.
const HOLD_TEXT: [f32; 4] = [0.92, 0.95, 1.0, 1.0];
const HOLD_SCALE: f32 = 3.0;

impl Session {
    /// How many windows to open now, or `None`: `--windows` asked for them, none are open
    /// yet, the title is known, and the title offers a split on this adapter - the same
    /// rule `--players` follows ([`oag_game::split_screen::offered`]). A refusal is said
    /// once and ends the request.
    pub(crate) fn seats_to_open(&mut self) -> Option<u8> {
        if self.windows_wanted < 2 || !self.seats.is_empty() {
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
        Some(wanted)
    }

    /// Opens windows 2 to `count` on the one device, each with a framebuffer of its own
    /// at window 1's allocation. Window `k` shows slot `k`. A window that cannot open is
    /// said and skipped: the run goes on with the windows it has.
    #[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
    pub(crate) fn open_seats(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, count: u8) {
        let table = strings::project_table(self.settings.language.as_deref());
        let app = hints::title(&table);
        let word = table.get("OAG_JOIN_PLAYER").unwrap_or("PLAYER").to_string();
        let monitors: Vec<_> = event_loop.available_monitors().collect();
        let size = self.settings.display.window_size;
        let origin = self.gpu.output.window.outer_position().ok();
        let scale = self.gpu.output.window.scale_factor();
        if self.windows_fullscreen
            && let Some(monitor) = monitors.first()
        {
            self.gpu
                .output
                .window
                .set_fullscreen(Some(winit::window::Fullscreen::Borderless(Some(
                    monitor.clone(),
                ))));
        }
        for slot in 1..usize::from(count) {
            let monitor = self
                .windows_fullscreen
                .then(|| monitors.get(slot % monitors.len().max(1)).cloned())
                .flatten();
            let title = windows::title(&app, &word, slot + 1);
            // Beside window 1 where the platform lets a program say so (X11); the
            // compositor places it otherwise (Wayland), which is why this is a request
            // and not a layout.
            let position = origin.map(|origin| {
                let step = f64::from(size.width) * scale;
                winit::dpi::PhysicalPosition::new(
                    origin.x + (step * slot as f64) as i32 + 16 * slot as i32,
                    origin.y,
                )
            });
            let opened = self
                .gpu
                .open_output(
                    event_loop,
                    &title,
                    winit::dpi::LogicalSize::new(size.width, size.height),
                    monitor,
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
                    Ok(Seat {
                        output,
                        framebuffer,
                        slot,
                        acquire: perf::Meter::new(),
                        present: perf::Meter::new(),
                    })
                });
            match opened {
                Ok(seat) => {
                    info!(
                        "window {}: {title} on slot {slot} ({}x{})",
                        slot + 1,
                        seat.output.config.width,
                        seat.output.config.height,
                    );
                    self.seats.push(seat);
                }
                Err(e) => warn!("window {}: {e:#}", slot + 1),
            }
        }
    }

    /// How many people a `--windows` race seats: window 1's and one for each seat window open
    /// from slot 1 without a gap. A window closed in the middle ends the run of windows,
    /// so slots past it are not seated (nothing would show them).
    fn people_in_windows(&self) -> usize {
        1 + (1..)
            .take_while(|slot| self.seats.iter().any(|seat| seat.slot == *slot))
            .count()
    }

    /// Seats everyone who has a window for a race launched from the menus, when nobody was
    /// seated otherwise: the join screen's lobby and a restart's kept seats (both of which
    /// say how many people race) win, and so does a campaign race, which is one person's
    /// and keeps its records. Otherwise the keyboard and the first pad take slot 0 and
    /// each further pad the next slot, as `--players N` seats them, and the grid fields
    /// opponents for the others to race. **Chosen, not measured.**
    pub(crate) fn seat_the_windows(&mut self, options: &mut oag_raceplay::Options) {
        let people = self.people_in_windows();
        if people < 2
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

    /// The ids of the seat windows, in seat order.
    pub(crate) fn seat_ids(&self) -> Vec<winit::window::WindowId> {
        self.seats.iter().map(|seat| seat.output.window.id()).collect()
    }

    /// Closes seat `index`: the window goes, and so does the person at it - that slot is
    /// flown by the AI from the next tick (`split_screen::follow_devices` reads
    /// `Assignment::drives`).
    pub(crate) fn close_seat(&mut self, index: usize) {
        if index >= self.seats.len() {
            return;
        }
        let seat = self.seats.remove(index);
        self.focus.forget(seat.output.window.id());
        self.controls.assignment_mut().unseat(seat.slot);
        info!(
            "window {} closed: slot {} is flown by the AI",
            seat.slot + 1,
            seat.slot
        );
    }

    /// Seat `index` was resized to `size`: its surface follows; its framebuffer follows on
    /// the next draw. A window with no area (minimised) is left alone, and **never pauses
    /// the race**: only window 1 does that.
    pub(crate) fn resize_seat(&mut self, index: usize, size: winit::dpi::PhysicalSize<u32>) {
        let Some(seat) = self.seats.get_mut(index) else {
            return;
        };
        if size.width == 0 || size.height == 0 {
            return;
        }
        seat.output.config.width = size.width;
        seat.output.config.height = size.height;
        seat.output.configure(&self.gpu.device);
    }

    /// Whether a race of `people` is shown as windows. See [`windows::covers`].
    pub(crate) fn seats_cover(&self, people: usize) -> bool {
        windows::covers(&self.open_slots(), people)
    }

    /// The slots the open seat windows show.
    pub(crate) fn open_slots(&self) -> Vec<usize> {
        self.seats.iter().map(|seat| seat.slot).collect()
    }

    /// The render target seat windows of the given sizes want, for the allocation they
    /// share with window 1 ([`windows::shared_allocation`]).
    pub(super) fn seat_targets(&self) -> Vec<(u32, u32)> {
        self.seats
            .iter()
            .map(|seat| self.target_for(seat.output.size()))
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

    /// Whether the race on screen (or parked behind the pause menu) is shown as windows.
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

    /// Draws every seat once, after window 1's frame. Each seat that fails to draw is said
    /// once per failure and does not end the run.
    pub(crate) fn draw_seats(&mut self) {
        if self.seats.is_empty() {
            return;
        }
        // Out of `self` for the loop: drawing a seat borrows the stage, the devices and the
        // settings beside it, none of which are the seats.
        let mut seats = std::mem::take(&mut self.seats);
        for seat in &mut seats {
            if let Err(e) = self.draw_seat(seat) {
                warn!("window {}: {e:#}", seat.slot + 1);
            }
        }
        self.seats = seats;
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
        for seat in &self.seats {
            line += &format!(
                ", window {} acquire {:.2} ms present {:.2} ms ({:?})",
                seat.slot + 1,
                ms(&seat.acquire),
                ms(&seat.present),
                seat.output.config.present_mode,
            );
        }
        info!("{line}");
    }

    /// One seat's frame: the race from its slot's camera when the race is windowed, a
    /// holding frame otherwise.
    fn draw_seat(&mut self, seat: &mut Seat) -> Result<()> {
        let waited = web_time::Instant::now();
        if !seat.output.acquire(&self.gpu.device) {
            return Ok(());
        }
        seat.acquire.record(waited.elapsed().as_secs_f32());
        let Some(frame) = seat.output.take_acquired() else {
            return Ok(());
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("seat frame"),
            });
        if self.windowed_race() {
            self.encode_seat_race(seat, &mut encoder, &view);
        } else {
            self.encode_holding(seat, &mut encoder, &view);
        }
        self.gpu.queue.submit(Some(encoder.finish()));
        let presenting = web_time::Instant::now();
        self.gpu.queue.present(frame);
        seat.present.record(presenting.elapsed().as_secs_f32());
        Ok(())
    }

    /// Slot `seat.slot`'s view of the race through the whole of window 1's chain: scene,
    /// resolve, HUD, bloom, grade. No upscaler history, blur or jitter (**chosen, not
    /// measured**: the windows alternate cameras every frame, and each history holds one),
    /// so FSR 3.1 takes the rung down, as it does on a menu frame.
    fn encode_seat_race(
        &mut self,
        seat: &mut Seat,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        let profile = self.render_profile();
        let surface = seat.output.size();
        let rect = display::viewport(surface, self.settings.display.aspect);
        let wanted = self.target_for(surface);
        let device = &self.gpu.device;
        seat.framebuffer.resize_output(device, surface);
        // Window 1's allocation, the scene's own; this window's extent inside it.
        seat.framebuffer.resize(device, self.framebuffer.allocation());
        seat.framebuffer.set_extent(wanted);
        let extent = seat.framebuffer.extent();
        let inside = (0.0, 0.0, extent.0 as f32, extent.1 as f32);
        let alpha = match &self.stage {
            Stage::Race(_) => oag_raceplay::between_ticks::frame_alpha(&self.clock, !self.paused),
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
        stage.render_seat(
            &self.gpu,
            encoder,
            seat.framebuffer.view(),
            seat.slot,
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
        let _ = seat.framebuffer.resolve_scene(
            &self.gpu.device,
            &self.gpu.queue,
            encoder,
            rect,
            &presentation,
            None,
            None,
        );
        stage.draw_slot_hud(
            &self.gpu,
            encoder,
            seat.framebuffer.output(),
            rect,
            seat.framebuffer.output_size(),
            seat.slot,
        );
        stage
            .scene
            .composite_bloom(encoder, seat.framebuffer.output(), rect);
        stage.race.set_draw_alpha(1.0);
        seat.framebuffer
            .set_screen_filter(&self.gpu.device, preset.as_ref());
        seat.framebuffer.composite(
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

    /// The frame a seat shows when the race is not drawing for it - the front end, a race of
    /// one person, a race in the middle of loading: black, and the join screen's own word
    /// for a person and the seat's number. **Chosen, not measured**: the original has no
    /// second window.
    fn encode_holding(
        &mut self,
        seat: &Seat,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        let table = strings::project_table(self.settings.language.as_deref());
        let word = table.get("OAG_JOIN_PLAYER").unwrap_or("PLAYER");
        let surface = seat.output.size();
        let space = perf::grid(surface);
        self.overlay.set_space(space);
        let list = [Draw::Text {
            x: space.size.0 * 0.5,
            y: space.size.1 * 0.5 - 12.0,
            scale: HOLD_SCALE,
            color: HOLD_TEXT,
            border: None,
            align: Align::Centre,
            text: format!("{word} {}", seat.slot + 1),
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
