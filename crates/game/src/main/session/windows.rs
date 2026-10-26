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

/// A further window's pointer for one tick, in that window's pixels, with the aspect rectangle
/// its frame is drawn into.
pub(super) type SeatPointer = (oag_ui::pointer::Pointer, (f32, f32, f32, f32));

impl Session {
    /// How many windows to open now, or `None`: `--windows` or `--window-seats` asked for them, none are open
    /// yet, and the title is known.
    #[cfg_attr(
        any(target_arch = "wasm32", target_os = "android", target_os = "ios"),
        allow(dead_code)
    )]
    pub(crate) fn screens_to_open(&mut self) -> Option<u8> {
        if self.windows_wanted < 2 {
            return None;
        }
        if self.shell.is_none() && self.race_title.is_none() {
            return None;
        }
        let wanted = std::mem::take(&mut self.windows_wanted);
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
        let scale = crate::window::effective_scale(
            self.gpu.output.window.scale_factor(),
            crate::window::under_gamescope(self.gpu.output.window.current_monitor().as_ref()),
        );
        let mode = self.seat_mode();
        // `--windows-fullscreen` (the profile's own WINDOW MODE already did this for window 1):
        // window 1 goes borderless on the monitor the profile names, like the others.
        if self.windows_fullscreen
            && self.settings.display.window_mode != display::WindowMode::Borderless
            && !crate::window::under_gamescope(self.gpu.output.window.current_monitor().as_ref())
        {
            let names = crate::window::monitor_names(&monitors);
            let first = self.settings.display.monitor.choose(&names).unwrap_or(0);
            self.gpu.output.window.set_resizable(true);
            self.gpu.output.window.set_fullscreen(Some(
                winit::window::Fullscreen::Borderless(monitors.get(first).cloned()),
            ));
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
                    crate::window::Look {
                        title: &title,
                        size,
                        mode,
                        monitor,
                        position,
                    },
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
                        rings: super::timing::Rings::new(&self.gpu.device, &self.gpu.queue),
                        pointer: crate::pointer::Window::default(),
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

    /// How a further window opens: windowed or borderless, the same as window 1 - the profile's
    /// WINDOW MODE, or `--windows-fullscreen` / the layout screen's FULLSCREEN for the run.
    #[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
    fn seat_mode(&self) -> display::WindowMode {
        let gamescope =
            crate::window::under_gamescope(self.gpu.output.window.current_monitor().as_ref());
        if self.windows_fullscreen
            || crate::window::effective_mode(self.settings.display.window_mode, gamescope)
                == display::WindowMode::Borderless
        {
            display::WindowMode::Borderless
        } else {
            display::WindowMode::Windowed
        }
    }

    /// Where window `window` (from 1 for the first further window) goes: the monitor it is
    /// asked to open on and, windowed, the position it is asked for.
    ///
    /// The monitor is the layout screen's choice for it (`window_monitors`, by name; a name this
    /// machine no longer has is the first monitor), else the windows are spread over the monitors
    /// in winit's order starting from the one window 1 is on (the profile's `display.monitor`,
    /// the first by default) and wrapping. Borderless covers that monitor. A windowed window for
    /// the monitor window 1 is on goes beside window 1; one for another monitor is centred
    /// there, stepped aside for each window already on it. **Windowed positions are a request:**
    /// X11 and Windows honour `with_position`, Wayland has no protocol for one and the
    /// compositor places the window (a tiling one, such as niri, floats it because every
    /// windowed window is fixed size, `window::attributes`). **Chosen, not measured.**
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
        let count = monitors.len().max(1);
        let first = self.settings.display.monitor.choose(&names).unwrap_or(0);
        let index_of = |window: usize| {
            self.window_monitors
                .get(window - 1)
                .map_or((first + window) % count, |name| {
                    names
                        .iter()
                        .position(|candidate| candidate == name)
                        .unwrap_or(0)
                })
        };
        let index = index_of(window);
        let Some(monitor) = monitors.get(index) else {
            return (None, None);
        };
        if self.seat_mode() == display::WindowMode::Borderless {
            return (Some(monitor.clone()), None);
        }
        if index != first {
            let aside = (1..window).filter(|&other| index_of(other) == index).count();
            let mut at = crate::window::centred_on(monitor, size);
            at.x += 48 * aside as i32;
            at.y += 32 * aside as i32;
            return (Some(monitor.clone()), Some(at));
        }
        // Beside window 1 where the platform lets a program say so (X11); the compositor
        // places it otherwise (Wayland), which is why this is a request and not a layout.
        let position = origin.map(|origin| {
            let step = f64::from(size.width) * scale;
            winit::dpi::PhysicalPosition::new(
                origin.x + (step * window as f64) as i32 + 16 * window as i32,
                origin.y,
            )
        });
        (Some(monitor.clone()), position)
    }

    /// What window `window` (0 is window 1) is called: the game's name and the seats it is
    /// planned to hold (`PLAYER 5 - 6`).
    #[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
    pub(super) fn window_title(&self, window: usize) -> String {
        let table = strings::project_table(self.settings.language.as_deref());
        let word = table.get("OAG_JOIN_PLAYER").unwrap_or("PLAYER");
        format!(
            "{} - {}",
            crate::hints::title(&table),
            windows::label(word, &self.planned_seats(window))
        )
    }

    /// How many windows a race launched now is laid out across: the windows open, or the ones
    /// asked for and not yet opened. The event loop opens a window on its next pass
    /// ([`Self::screens_to_open`]), so a launch in the same frame as the layout screen's START
    /// (a title with neither Track nor Ship Select) must still count them.
    pub(crate) fn open_windows(&self) -> usize {
        (1 + self.screens.len()).max(usize::from(self.windows_wanted))
    }

    /// The slots window `window` (0 is window 1) is planned to hold before a race has dealt
    /// them, in pane order: the counts of the layout the command line or the layout screen asked
    /// for (one person per window when it spelled none), the slots the seat assignment gave
    /// their panes ([`oag_game::split_screen::window_slots`]). What a holding frame and a
    /// window's title say.
    pub(crate) fn planned_seats(&self, window: usize) -> Vec<usize> {
        let windows = self.window_layout.len().max(window + 1);
        let layout: Vec<oag_game::split_screen::WindowSeats> = (0..windows)
            .map(|index| oag_game::split_screen::WindowSeats {
                count: self.window_layout.get(index).map_or(1, |seats| seats.count),
                cut: oag_render::viewport::Cut::default(),
            })
            .collect();
        oag_game::split_screen::window_slots(&layout, &self.pane_people)
            .swap_remove(window)
    }

    /// How many people the open windows are planned to seat: the layout's counts over window 1
    /// and the further windows still open.
    fn planned_people(&self) -> usize {
        (0..=self.screens.len())
            .map(|window| self.planned_seats(window).len())
            .sum()
    }

    /// Seats everyone the windows are planned for a race launched from the menus, when nobody
    /// was seated otherwise: a layout the layout screen chose (that was for its split race), the
    /// join screen's lobby and a restart's kept seats (both of which
    /// say how many people race) win, and so does a campaign race, which is one person's and
    /// keeps its records. Otherwise the keyboard and the first pad take slot 0 and each
    /// further pad the next slot, as `--players N` seats them, and the grid fields opponents
    /// for the others to race. **Chosen, not measured.**
    pub(crate) fn seat_the_windows(&mut self, options: &mut oag_raceplay::Options) {
        let people = self.planned_people();
        if self.screens.is_empty()
            || people < 2
            || self.window_layout_chosen
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

    /// F11 in window `id`: that window alone toggles borderless fullscreen on its own monitor.
    #[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
    pub(crate) fn toggle_window_fullscreen(&mut self, id: winit::window::WindowId) {
        let window = if self.gpu.output.window.id() == id {
            &self.gpu.output.window
        } else if let Some(screen) = self.screens.iter().find(|s| s.output.window.id() == id) {
            &screen.output.window
        } else {
            return;
        };
        let at = self.windowed_sizes.iter().position(|(w, _)| *w == id);
        let mut restore = at.map(|i| self.windowed_sizes.remove(i).1);
        crate::window::toggle_fullscreen(window, &mut restore);
        if let Some(size) = restore {
            self.windowed_sizes.push((id, size));
        }
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
            .map(|screen| {
                self.target_for(crate::window::is_fullscreen(&screen.output.window), screen.output.size())
            })
            .collect()
    }

    /// The shape window 1 is drawn at ([`crate::window::scene_aspect`]), by its actual
    /// fullscreen state.
    pub(super) fn aspect(&self) -> display::Aspect {
        crate::window::scene_aspect(&self.settings.display, self.fullscreen())
    }

    /// The aspect rectangle the scene's resolution is measured against
    /// ([`crate::window::scene_rect_of`]) for window 1. Only the size is used, the scene is
    /// stretched into the surface's own rectangle afterwards.
    pub(super) fn scene_rect(&self, surface: (u32, u32)) -> (f32, f32, f32, f32) {
        crate::window::scene_rect_of(&self.settings.display, self.fullscreen(), surface)
    }

    /// Whether window 1 is fullscreen now (gamescope, F11 and the saved mode all count). A
    /// further window asks [`crate::window::is_fullscreen`] of its own window.
    pub(super) fn fullscreen(&self) -> bool {
        crate::window::is_fullscreen(&self.gpu.output.window)
    }

    /// The render target a window of `surface` pixels, `fullscreen` or not, wants at the
    /// profile's render scale.
    pub(super) fn target_for(&self, fullscreen: bool, surface: (u32, u32)) -> (u32, u32) {
        let rect = crate::window::scene_rect_of(&self.settings.display, fullscreen, surface);
        oag_game::settings::web::render_target(
            rect,
            self.render_scale_for(rect),
            self.gpu.device.limits().max_texture_dimension_2d,
        )
    }

    /// The render scale for the aspect rectangle `rect`: the profile's, or under gamescope the
    /// default that keeps an oversized output within a pixel budget when the player chose none
    /// ([`settings::RenderProfile::scale_for`]).
    pub(super) fn render_scale_for(
        &self,
        rect: (f32, f32, f32, f32),
    ) -> oag_display::display::Scale {
        let gamescope =
            crate::window::under_gamescope_cached(self.gpu.output.window.current_monitor().as_ref());
        self.render_profile().scale_for((rect.2, rect.3), gamescope)
    }

    /// Every window's seats, window 1 first, for a race dealt across windows (on screen or
    /// parked behind the pause menu), or none.
    fn window_seats(&self) -> Vec<Vec<usize>> {
        let race = match &self.stage {
            Stage::Race(stage) => Some(&stage.race),
            Stage::Menu(_) => self.suspended_race.as_ref().map(|stage| &stage.race),
            _ => None,
        };
        match race {
            Some(race) if self.screens_opened && race.windowed() => race
                .windows()
                .iter()
                .map(|window| window.seats.clone())
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Whether the race on screen reads its keys per window: dealt across windows and not
    /// parked behind a menu, which window 1's keyboard (the first seat's) drives.
    pub(crate) fn keys_by_window(&self) -> bool {
        self.screens_opened && matches!(&self.stage, Stage::Race(stage) if stage.race.windowed())
    }

    /// The index of window `id` among the run's windows (window 1 is 0), if it is one.
    pub(crate) fn window_index(&self, id: winit::window::WindowId) -> Option<usize> {
        let ids = self.screen_ids();
        let primary = self.gpu.output.window.id();
        windows::which(id, primary, &ids).map(|which| match which {
            windows::Which::Primary => 0,
            windows::Which::Screen(index) => index + 1,
        })
    }

    /// A key reported on window `id`, in a race dealt across windows: **a keyboard seat per
    /// window, chosen, not measured.** It goes to the keyboard seat of the window it arrived on;
    /// a press in a window that holds none claims the window's free seat, and a window with no
    /// keyboard seat and nothing to claim ignores it (`Controls::window_key`). Dropped once a
    /// second physical keyboard has spoken, because the raw events carry the keys then
    /// ([`Self::device_key`]) and these are the same presses again.
    pub(crate) fn window_key(&mut self, id: winit::window::WindowId, key: &winit::keyboard::Key, pressed: bool) {
        if self.controls.per_device_keys() {
            return;
        }
        let Some(window) = self.window_index(id) else {
            return;
        };
        let seats = self.window_seats().get(window).cloned().unwrap_or_default();
        self.controls.window_key(&seats, key, pressed);
    }

    /// A raw key event from physical keyboard `device`, which names no window: read against the
    /// window with the focus. Only a platform that reports the keyboard sends any (X11, Windows).
    pub(crate) fn device_key(
        &mut self,
        device: winit::event::DeviceId,
        physical: winit::keyboard::PhysicalKey,
        pressed: bool,
    ) {
        // The keyboards are told apart for as long as the LOCAL MULTIPLAYER flow is on the menus:
        // the join screen, the layout, the RACE page and the seats' Ship Select all read a seat's
        // device (`Controls::lobby_key`). Never in a race, which routes them by seat.
        let joining = matches!(&self.stage, Stage::Menu(stage)
            if stage.join.is_some()
                || stage.layout.is_some()
                || stage.assign.is_some()
                || self.multiplayer);
        if !joining && !self.keys_by_window() && !self.controls.per_device_keys() {
            return;
        }
        let Some(key) = oag_input::keys::key_at(physical) else {
            return;
        };
        // The join screen listens to the keyboards one by one so that a second keyboard can
        // take a seat of its own (`Controls::lobby_key`); nothing else of the menus reads them.
        if joining {
            let id = self.controls.keyboard_id(device);
            self.controls.lobby_key(id, &key, pressed);
            return;
        }
        // One window with seated keyboards (a split screen joined with two): no window has the
        // focus to ask, and the keys are already bound to their seats.
        let focused = self
            .focus
            .current()
            .and_then(|id| self.window_index(id))
            .or_else(|| self.screens.is_empty().then_some(0));
        let Some(focused) = focused else {
            return;
        };
        let all = self.window_seats();
        let window = all.get(focused).cloned().unwrap_or_default();
        let race: Vec<usize> = all.iter().flatten().copied().collect();
        let id = self.controls.keyboard_id(device);
        self.controls.device_key(id, &window, &race, &key, pressed);
    }

    /// A window lost the focus: the keys its keyboard seat held are let go, and stay released
    /// until seen to come up (`Controls::let_go_window`).
    pub(crate) fn window_lost_focus(&mut self, id: winit::window::WindowId) {
        let Some(window) = self.window_index(id) else {
            return;
        };
        let seats = self.window_seats().get(window).cloned().unwrap_or_default();
        self.controls.let_go_window(&seats);
    }

    /// Every window's first seat counts as driven (`Assignment::set_keyboard_reach`): a window
    /// is shown to a person, so a seat nobody has pressed a key at yet idles rather than being
    /// handed to the AI. **Chosen, not measured**; kept from the keyboard-follows-focus rule it
    /// replaces. Once a frame, for a race that starts or a window that closes.
    pub(crate) fn sync_keyboard_reach(&mut self) {
        let seats = self.window_seats();
        let views: Vec<&[usize]> = seats.iter().map(Vec::as_slice).collect();
        self.controls
            .assignment_mut()
            .set_keyboard_reach(&windows::first_seats(views.iter().copied()));
    }

    /// A left click landed on further window `window` (1-based index of the run's windows,
    /// 1 being the first further one). Kept for the pause only.
    pub(crate) fn seat_clicked(&mut self, window: usize) {
        if let Ok(bit) = u32::try_from(window)
            && bit < u8::BITS
        {
            self.seat_clicks |= 1 << bit;
        }
    }

    /// A mouse or touch event on further window `index` (0 is window 2), latched on that
    /// window's own pointer for [`Self::seat_pointers`].
    pub(crate) fn seat_pointer(
        &mut self,
        index: usize,
        event: &winit::event::WindowEvent,
    ) {
        use winit::event::WindowEvent;
        let Some(screen) = self.screens.get_mut(index) else {
            return;
        };
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                screen.pointer.cursor_moved(position.x, position.y);
            }
            WindowEvent::CursorLeft { .. } => screen.pointer.cursor_left(),
            WindowEvent::MouseInput { state, button, .. } => screen.pointer.button(*button, *state),
            WindowEvent::MouseWheel { delta, .. } => screen.pointer.wheel(*delta),
            WindowEvent::Touch(touch) => screen.pointer.touch(*touch),
            _ => {}
        }
    }

    /// One tick of every further window's pointer, each in that window's pixels with the aspect
    /// rectangle its frame is drawn into: taken every tick so a click made mid-race is spent
    /// rather than waiting for the end screens. What the end screens read
    /// ([`Self::end_screen_pointer`]).
    pub(crate) fn seat_pointers(&mut self) -> Vec<SeatPointer> {
        let settings = &self.settings.display;
        self.screens
            .iter_mut()
            .map(|screen| {
                let aspect = crate::window::scene_aspect(
                    settings,
                    crate::window::is_fullscreen(&screen.output.window),
                );
                let rect = display::viewport(screen.output.size(), aspect);
                (screen.pointer.take(), rect)
            })
            .collect()
    }

    /// The slots whose Start a click on a further window is, as the bitmask the pause reads
    /// (`oag_input::Controls::take_presses`): each clicked window's first seat. Taken: clicks
    /// are spent whether or not the race is paused, so a click while it ran cannot pause later.
    pub(crate) fn take_seat_clicks(&mut self) -> u8 {
        let clicks = std::mem::take(&mut self.seat_clicks);
        let Stage::Race(stage) = &self.stage else {
            return 0;
        };
        let seats: Vec<&[usize]> = stage
            .race
            .windows()
            .iter()
            .map(|window| window.seats.as_slice())
            .collect();
        windows::click_slots(clicks, &seats)
    }

    /// Whether the race on screen (or parked behind the pause menu) is dealt across windows.
    pub(crate) fn windowed_race(&self) -> bool {
        match &self.stage {
            Stage::Race(stage) => stage.race.windowed(),
            Stage::Menu(_) => self
                .suspended_race
                .as_ref()
                .is_some_and(|stage| stage.race.windowed()),
            _ => false,
        }
    }

    /// A frame begins: the race on screen (or parked behind the menus) has had no window draw
    /// yet this frame, so the first window to draw does the camera-independent work and the
    /// others read it ([`RaceStage::begin_frame`]).
    ///
    /// [`RaceStage::begin_frame`]: crate::race_stage::RaceStage::begin_frame
    pub(crate) fn begin_scene_frame(&mut self) {
        if let Stage::Race(stage) = &mut self.stage {
            stage.begin_frame();
        }
        if let Some(stage) = self.suspended_race.as_mut() {
            stage.begin_frame();
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
    /// resolve, HUD, bloom, grade. **A window of one seat has its own camera history**
    /// (`Scene::render_window`), so motion blur, camera jitter and the temporal upscaler (FSR
    /// 3.1, which keeps its history in this window's framebuffer) run per window; **a window of
    /// several seats is a split**, whose views are N cameras in one frame: each view blurs,
    /// jitters and is resolved by FSR 3.1 on its own seat's history (a chain per seat in this
    /// window's framebuffer). Chosen, not measured.
    fn encode_screen_race(
        &mut self,
        window: usize,
        screen: &mut Screen,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        let profile = self.governed_profile();
        let surface = screen.output.size();
        let fullscreen = crate::window::is_fullscreen(&screen.output.window);
        let rect = display::viewport(surface, crate::window::scene_aspect(&self.settings.display, fullscreen));
        let wanted = self.target_for(fullscreen, surface);
        let device = &self.gpu.device;
        screen.framebuffer.resize_output(device, surface);
        // Window 1's allocation, the scene's own; this window's extent inside it.
        screen.framebuffer.resize(device, self.framebuffer.allocation());
        // The one controller's scale on this window's own ceiling (the target it wants),
        // while every window holds one seat; the ceiling itself otherwise (`Session::drs_runs`).
        let (wanted, blur_resolution) = if self.drs_runs() {
            let limits = self.drs_limits_for(fullscreen, surface, wanted);
            (self.drs.extent(limits), self.drs.blur_resolution(limits))
        } else {
            (wanted, profile.motion_blur_resolution)
        };
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
        // This window's own blur and camera jitter, on its own history in the scene
        // (`Scene::render_window`): a window of one seat draws them, one of several is a split
        // and does not. The jitter's length is a property of the upscaler resolving this
        // window's frames, as for window 1.
        stage.scene.set_blur_resolution(blur_resolution);
        let effects = oag_raceplay::WindowEffects {
            motion_blur: profile.motion_blur,
            camera_jitter: upscale::jitter_phases(
                self.camera_jitter,
                profile.reconstruction,
                self.gpu.temporal,
                extent,
                (rect.2 as u32, rect.3 as u32),
            ),
            ..Default::default()
        };
        // This window's own timers, bracketing its own passes: its cost joins window 1's in the
        // controller's frame (`Session::feed_windows`).
        let frame = self.frame_index;
        screen.rings.claim(frame);
        let stats = stage.render_window(
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
            screen.rings.effects(effects),
        );
        screen
            .rings
            .give_back(frame, &stats, stage.race.window_drawn(window).len() == 1);
        let presentation = upscale::Presentation {
            reconstruction: profile.reconstruction,
            sharpness: profile.upscale_sharpness.stops(),
            brightness: self.settings.display.brightness,
            gamma: self.settings.display.gamma,
        };
        // This window's own temporal record (`None` for a window of several seats, and where
        // the adapter cannot run the upscaler): FSR 3.1 keeps its history in `screen.framebuffer`.
        let temporal = if self.gpu.temporal {
            stage.temporal(window)
        } else {
            None
        };
        // Claimed on the three conditions `resolve_scene` decides with, as window 1's is
        // (`Session::draw`): a slot nothing writes into does not read back as zero.
        let upscaling = temporal.is_some()
            && presentation.reconstruction == display::Reconstruction::Fsr3
            && screen.framebuffer.temporal_upscaler_viable();
        if upscaling {
            screen.rings.claim_upscale(frame);
        }
        let encoded = screen.framebuffer.resolve_scene(
            &self.gpu.device,
            &self.gpu.queue,
            encoder,
            rect,
            &presentation,
            temporal,
            screen.rings.upscale_timestamps(),
        );
        if upscaling && !encoded {
            screen.rings.abandon_upscale();
        }
        stage
            .scene
            .composite_bloom_under_hud(encoder, screen.framebuffer.output(), rect);
        // The results and the end screens are the one flow every window draws at its own size
        // (`RaceStage::draw_end_screens`); a race still running gets this window's panel and HUD.
        if !stage.draw_end_screens(
            &self.gpu,
            encoder,
            screen.framebuffer.output(),
            rect,
            screen.framebuffer.output_size(),
        ) {
            stage.draw_window_track_panel(
                &self.gpu,
                encoder,
                screen.framebuffer.output(),
                rect,
                window,
            );
            stage.draw_window_hud(
                &self.gpu,
                encoder,
                screen.framebuffer.output(),
                rect,
                screen.framebuffer.output_size(),
                window,
            );
        }
        stage
            .scene
            .composite_bloom(encoder, screen.framebuffer.output(), rect);
        screen.rings.resolve(encoder);
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
        // The pause's line, as window 1 draws it: over the graded frame, on the window's own
        // whole-surface grid. Every window that holds a seat says who paused and when it is
        // anyone's; nothing while the race runs or the resume counts down.
        let space = perf::grid(surface);
        let mut line = Vec::new();
        // Escape has parked the race behind window 1's front-end menu: the other windows keep
        // showing the frozen race under the menus' own pause tint (`PAUSE_OVERLAY`, the alpha
        // window 1's menu is drawn over the parked race with), a stopgap for the menu itself,
        // which only window 1 has. **Chosen, not measured.**
        if matches!(self.stage, Stage::Menu(_)) && self.suspended_race.is_some() {
            line.push(Draw::Fill {
                rect: [0.0, 0.0, space.size.0, space.size.1],
                color: crate::menu_stage::PAUSE_OVERLAY,
            });
        }
        line.extend(self.pause_line(space.size));
        if !line.is_empty() {
            self.overlay.set_space(space);
            self.overlay.overlay(
                &self.gpu.device,
                &self.gpu.queue,
                encoder,
                view,
                &line,
                (0.0, 0.0, surface.0 as f32, surface.1 as f32),
            );
        }
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
        let seats = self.planned_seats(window);
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
            text: windows::label(word, &seats),
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
