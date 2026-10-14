//! Opens and drives the window layout screen, between the join screen and the race box's own
//! Track and Ship Select.
//!
//! **Where it sits.** The join screen's START opens it when two or more people are seated and
//! the run can open windows (a desktop build on an adapter the title offers a split on,
//! [`oag_game::split_screen::offered`]); its START goes on to the race box exactly as the join
//! screen's did, and its BACK returns to the join screen with everybody still seated. On the
//! browser, Android and iOS (where a program gets the one surface the platform makes, and the
//! opener is compiled out) the screen is **skipped**: the join screen's START goes straight
//! on. On an integrated GPU the HD, 2048 and Omega splits are not offered at all, so neither
//! screen is reachable there; Pulse and Pure, which the rule exempts, get the screen with
//! every window count. **Chosen, not measured**: the screen, the rows, the persistence.
//!
//! **What START writes** is what `--window-seats` writes: [`Session::window_layout`] (one
//! `WindowSeats` per window), `windows_wanted` for the event loop to open the windows on its
//! next pass (`Session::screens_to_open`), the monitor of each further window, and the cut of
//! window 1. Window 1's own mode and monitor are the DISPLAY page's rows
//! (`display.window_mode`, `display.monitor`), which the screen's rows set through the same
//! `apply_setting`, so the two pages never disagree. The layout is remembered in
//! `[race] window_seats` and `window_monitors` the way the RACE page's choices are, with no
//! save slot, and re-validated when the screen next opens: a monitor that is gone is the
//! first monitor, and another head count is dealt anew from the same windows.

use oag_ui::menu::nav::Nav;
use oag_ui_screens::window_layout::{Event, Labels, Picker, Window};

use crate::join_stage::JoinStage;
use crate::stage::Stage;
use crate::window_layout_stage::WindowLayoutStage;

use super::Session;

impl Session {
    /// The join screen's START with `join` holding the people: opens the layout screen over
    /// the menus when it applies, else hands `join` back for the race box to start as it
    /// did.
    pub(crate) fn open_window_layout(&mut self, join: JoinStage) -> Option<JoinStage> {
        #[cfg(any(target_arch = "wasm32", target_os = "android", target_os = "ios"))]
        {
            Some(join)
        }
        #[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
        {
            let offered = self.shell.as_ref().is_some_and(|shell| {
                oag_game::split_screen::offered(shell.title, self.gpu.device_type)
            });
            if !offered || join.lobby().count() < 2 || !matches!(self.stage, Stage::Menu(_)) {
                return Some(join);
            }
            let Some(shell) = self.shell.as_ref() else {
                return Some(join);
            };
            let labels = Labels::read(&shell.strings);
            let monitors = self.monitor_names();
            let picker = Picker::new(
                labels,
                join.lobby().count(),
                self.starting_windows(join.lobby().count(), &monitors),
                monitors,
                self.starting_fullscreen(),
            );
            if let Stage::Menu(stage) = &mut self.stage {
                stage.layout = Some(WindowLayoutStage::new(join, picker));
            }
            None
        }
    }

    /// One tick of the open screen: pad and keyboard, then the pointer.
    pub(crate) fn tick_window_layout(&mut self, pointer: &oag_ui::pointer::Pointer) {
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let Some(open) = stage.layout.as_mut() else {
            return;
        };
        let mut events = open.screen_mut().update(self.controls.buttons_mut());
        events.extend(open.screen_mut().pointer(pointer, &stage.skin));
        for event in events {
            if !matches!(self.stage, Stage::Menu(ref stage) if stage.layout.is_some()) {
                break;
            }
            self.handle_window_layout(event);
        }
    }

    /// Escape on the open screen: back, as circle is.
    pub(crate) fn escape_window_layout(&mut self) {
        self.handle_window_layout(Event::Back);
    }

    /// What one event of the open screen does: a sound, and for START and BACK a change of
    /// screen.
    pub(crate) fn handle_window_layout(&mut self, event: Event) {
        match event {
            Event::Moved | Event::Changed => self.play_navs(Some(Nav::Moved(None))),
            Event::Back => {
                if let Stage::Menu(stage) = &mut self.stage
                    && let Some(open) = stage.layout.take()
                {
                    stage.join = Some(open.into_join());
                }
                self.play_navs(Some(Nav::Decline));
            }
            Event::Start => self.finish_window_layout(),
        }
    }

    /// START: the layout is applied, remembered and handed to the race box, which goes on
    /// to Track and Ship Select with the lobby the join screen made.
    fn finish_window_layout(&mut self) {
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let Some(open) = stage.layout.take() else {
            return;
        };
        self.apply_window_layout(open.screen());
        self.play_navs(Some(Nav::Accept));
        if open.join().lobby().count() > 1 {
            self.split_lobby = Some(open.join().lobby().clone());
        }
        self.start_race_box();
    }
}

#[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
impl Session {
    /// What each monitor is called, in the platform's order.
    fn monitor_names(&self) -> Vec<String> {
        crate::window::monitor_names(
            &self
                .gpu
                .output
                .window
                .available_monitors()
                .collect::<Vec<_>>(),
        )
    }

    /// Whether the windows start the screen fullscreen: window 1's mode, which is the DISPLAY
    /// page's, or `--windows-fullscreen`.
    fn starting_fullscreen(&self) -> bool {
        self.windows_fullscreen
            || self.settings.display.window_mode == oag_display::display::WindowMode::Borderless
    }

    /// The windows the screen opens on for `people` seated people: the layout this run already
    /// has (`--windows`, or the last one the screen applied), else the one remembered in the
    /// profile, else one window holding everybody. Monitors are the names' indices; a name
    /// that is gone is the first monitor. The screen deals again whatever does not add up.
    fn starting_windows(&self, people: usize, monitors: &[String]) -> Vec<Window> {
        let remembered;
        let layout = if self.window_layout.len() >= 2 {
            &self.window_layout
        } else {
            remembered = oag_game::split_screen::parse_window_seats(
                &self.settings.race.window_seats,
                self.split_cut,
            )
            .unwrap_or_default();
            &remembered
        };
        let index_of = |name: Option<&String>| {
            name.and_then(|name| monitors.iter().position(|candidate| candidate == name))
                .unwrap_or(0)
        };
        let first = self
            .settings
            .display
            .monitor
            .choose(monitors)
            .unwrap_or(0);
        if layout.len() < 2 {
            return vec![Window {
                seats: people,
                vertical: self.split_cut == oag_render::viewport::Cut::Vertical,
                monitor: first,
            }];
        }
        let named = if self.window_layout.len() >= 2 {
            &self.window_monitors
        } else {
            &self.settings.race.window_monitors
        };
        layout
            .iter()
            .enumerate()
            .map(|(index, window)| Window {
                seats: window.count,
                vertical: window.cut == oag_render::viewport::Cut::Vertical,
                monitor: if index == 0 {
                    first
                } else if named.is_empty() {
                    index % monitors.len().max(1)
                } else {
                    index_of(named.get(index - 1))
                },
            })
            .collect()
    }

    /// Writes the screen's choice: the layout the race box deals the people by, the windows
    /// the event loop opens or closes, each further window's monitor, window 1's cut, mode
    /// and monitor (the DISPLAY page's rows, only when they changed: setting them moves and
    /// resizes the window), and the profile's memory of it all.
    fn apply_window_layout(&mut self, screen: &Picker) {
        use oag_display::display::{Monitor, WindowMode};
        let monitors = self.monitor_names();
        let name_of = |monitor: usize| monitors.get(monitor).cloned().unwrap_or_default();
        let layout: Vec<oag_game::split_screen::WindowSeats> = screen
            .windows()
            .iter()
            .map(|window| oag_game::split_screen::WindowSeats {
                count: window.seats,
                cut: if window.vertical {
                    oag_render::viewport::Cut::Vertical
                } else {
                    oag_render::viewport::Cut::Horizontal
                },
            })
            .collect();
        self.split_cut = layout[0].cut;
        self.settings.race.window_seats = oag_game::split_screen::spell_window_seats(&layout);
        if layout.len() >= 2 {
            self.window_monitors = screen.windows()[1..]
                .iter()
                .map(|window| name_of(window.monitor))
                .collect();
            self.windows_fullscreen = screen.fullscreen();
            self.windows_wanted = u8::try_from(layout.len()).unwrap_or(u8::MAX);
            self.window_layout = layout;
        } else {
            self.window_monitors.clear();
            self.windows_wanted = 0;
            self.window_layout.clear();
        }
        self.settings.race.window_monitors = self.window_monitors.clone();
        self.drop_screens_beyond(self.window_layout.len().max(1));
        for index in 0..self.screens.len() {
            let title = self.window_title(index + 1);
            self.screens[index].output.window.set_title(&title);
        }

        let mode = if screen.fullscreen() {
            WindowMode::Borderless
        } else {
            WindowMode::Windowed
        };
        if mode != self.settings.display.window_mode {
            self.apply_setting(
                "display.window_mode",
                &oag_ui::menu::Value::Text(mode.name().to_string()),
            );
            self.seed_display_row("display.window_mode", mode.name().to_string());
        }
        if monitors.len() > 1 {
            let wanted = Monitor::from(name_of(screen.windows()[0].monitor));
            if wanted != self.settings.display.monitor {
                let text = wanted.to_string();
                self.apply_setting("display.monitor", &oag_ui::menu::Value::Text(text.clone()));
                self.seed_display_row("display.monitor", text);
            }
        }
        if let Err(e) = crate::settings::save(&self.settings) {
            log::error!("could not save settings: {e:#}");
        }
    }

    /// Moves a DISPLAY page row to `value`, for a setting the layout screen wrote, so the page
    /// does not show what it held before.
    fn seed_display_row(&mut self, setting: &str, value: String) {
        if let Stage::Menu(stage) = &mut self.stage {
            stage
                .menu
                .seed(setting, &oag_ui::menu::Value::Text(value));
        }
    }

    /// Closes the windows after the first `count`, with no seat handed to anyone: nothing
    /// has been dealt yet, so they only had holding frames.
    fn drop_screens_beyond(&mut self, count: usize) {
        while self.screens.len() + 1 > count {
            if let Some(screen) = self.screens.pop() {
                self.focus.forget(screen.output.window.id());
            }
        }
    }
}
