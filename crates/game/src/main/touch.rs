//! The touchscreen's second job: the on-screen racing controls, and the
//! Android gamepad stick that winit delivers as a touch.
//!
//! Every `WindowEvent::Touch` lands here first ([`Session::touch`]). A touch
//! from a **screen** goes to the menus' pointer as it always did, and also to
//! the overlay while a race is running. A touch from a device that never began
//! one is not a finger at all: winit's Android backend turns every joystick
//! `MotionEvent` into a `Touch::Moved` at `AXIS_X`/`AXIS_Y`, so it is the left
//! stick of a gamepad and goes to [`oag_input::Controls::android_stick`].
//!
//! The overlay hides when a pad is in use and comes back on the next touch,
//! and only exists where there is a touchscreen to need it: Android, or any
//! platform with `OAG_TOUCH_CONTROLS` set, which is how a desktop run shows
//! it. Layout and rules: [`oag_input::touch`]; drawing:
//! [`oag_game::touch_controls`].

use oag_input::Reading;
use oag_input::touch::Touches;
use winit::event::{DeviceId, Touch, TouchPhase};

use crate::session::Session;
use crate::stage::Stage;

/// The overlay's own state: who is touching, and whether it is showing.
#[derive(Debug)]
pub(crate) struct Overlay {
    touches: Touches,
    /// Devices that have started a touch. Anything else that "moves" is a
    /// gamepad stick.
    screens: Vec<DeviceId>,
    /// Whether this platform has a touchscreen to need the overlay.
    enabled: bool,
    /// Hidden while a pad is in use; the next touch shows it again.
    shown: bool,
}

impl Default for Overlay {
    fn default() -> Self {
        Self {
            touches: Touches::default(),
            screens: Vec::new(),
            enabled: oag_game::touch_controls::available(),
            shown: true,
        }
    }
}

/// What a `Touch` event is.
#[derive(Debug, PartialEq)]
pub(crate) enum Routed {
    /// A finger on a screen.
    Finger,
    /// A gamepad's left stick, as `AXIS_X` and `AXIS_Y`.
    Stick(f32, f32),
    /// Nothing the game reads.
    Ignored,
}

impl Overlay {
    /// Classifies `touch` by whether its device ever began one.
    pub(crate) fn route(&mut self, touch: &Touch) -> Routed {
        if touch.phase == TouchPhase::Started {
            if !self.screens.contains(&touch.device_id) {
                self.screens.push(touch.device_id);
            }
            return Routed::Finger;
        }
        if self.screens.contains(&touch.device_id) {
            return Routed::Finger;
        }
        match touch.phase {
            TouchPhase::Moved => Routed::Stick(touch.location.x as f32, touch.location.y as f32),
            _ => Routed::Ignored,
        }
    }

    /// Whether the overlay is on screen and reading fingers.
    pub(crate) fn active(&self) -> bool {
        self.enabled && self.shown
    }

    /// The overlay's draw list for a window of `size` pixels, in a grid
    /// `scale` units per pixel; empty when it is not showing.
    pub(crate) fn draw(
        &self,
        art: &oag_game::touch_controls::art::Art,
        size: (f32, f32),
        scale: f32,
        paused: bool,
        setup: oag_input::touch::Setup,
        opacity: f32,
    ) -> Vec<oag_ui::frontend::Draw> {
        if self.active() {
            oag_game::touch_controls::draw(art, &self.touches, size, scale, paused, setup, opacity)
        } else {
            Vec::new()
        }
    }
}

impl Session {
    /// One `Touch` event, routed. See the module doc.
    pub(crate) fn touch(&mut self, touch: Touch) {
        match self.touch_overlay.route(&touch) {
            Routed::Stick(x, y) => self.controls.android_stick(x, y),
            Routed::Ignored => {}
            Routed::Finger => {
                self.pointer.touch(touch);
                let size = self.window_size();
                let at = (touch.location.x as f32, touch.location.y as f32);
                let racing = self.racing();
                let overlay = &mut self.touch_overlay;
                if touch.phase == TouchPhase::Started && overlay.enabled {
                    overlay.shown = true;
                }
                if !overlay.active() || !racing {
                    return;
                }
                match touch.phase {
                    TouchPhase::Started => overlay.touches.down(touch.id, at, size),
                    TouchPhase::Moved => overlay.touches.moved(touch.id, at, size),
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        overlay.touches.up(touch.id, size);
                    }
                }
            }
        }
    }

    /// Hands this tick's overlay to the devices: held controls as a reading,
    /// presses as taps. Called once per tick, before the snapshots are taken.
    ///
    /// Outside a running race the overlay holds nothing. While paused only
    /// pause itself gets through, which is how it resumes.
    pub(crate) fn feed_touch(&mut self) {
        if self.controls.pad_spoke() {
            self.touch_overlay.shown = false;
        }
        let size = self.window_size();
        let racing = self.racing();
        let overlay = &mut self.touch_overlay;
        if !overlay.active() || !racing {
            overlay.touches.release_all();
            self.controls.set_touch(Reading::default());
            return;
        }
        let paused = self.paused;
        let setup = Self::touch_setup(&self.settings);
        overlay.touches.set_setup(setup, size);
        let taps = overlay.touches.take_taps();
        let mut reading = overlay.touches.reading(size);
        if setup.scheme == oag_input::touch::Scheme::Easy
            && self.scheme == oag_gameplay::ControlScheme::Novice
        {
            oag_input::touch::fold_for_novice_sim(&mut reading);
        }
        self.controls.set_touch(if paused {
            Reading::default()
        } else {
            reading
        });
        for button in taps {
            if !paused || button == oag_gameplay::input::Button::Start {
                self.controls.tap(button);
            }
        }
    }

    /// The overlay's draw list: empty outside a running race, or when hidden.
    pub(crate) fn draw_touch(&self, size: (f32, f32), scale: f32) -> Vec<oag_ui::frontend::Draw> {
        let art = oag_game::touch_controls::art::Art::from_sheet(&self.cursor_sheet);
        if let (true, Some(art)) = (self.racing() && self.race_drawing(), art) {
            self.touch_overlay.draw(
                &art,
                size,
                scale,
                self.paused,
                Self::touch_setup(&self.settings),
                self.settings.controls.touch_alpha(),
            )
        } else {
            Vec::new()
        }
    }

    fn touch_setup(settings: &crate::settings::Settings) -> oag_input::touch::Setup {
        settings.controls.touch_setup()
    }

    /// Whether the race is on the glass: not through the flyby, which draws
    /// no HUD either. The loading screen is a different stage, so a race that
    /// is still loading is not [`Self::racing`] at all.
    fn race_drawing(&self) -> bool {
        matches!(&self.stage, Stage::Race(stage) if stage.race.hud_shown())
    }

    fn racing(&self) -> bool {
        matches!(&self.stage, Stage::Race(stage) if !stage.race.finished())
    }

    fn window_size(&self) -> (f32, f32) {
        let (w, h) = self.gpu.size();
        (w as f32, h as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::dpi::PhysicalPosition;

    fn touch(device: DeviceId, phase: TouchPhase, x: f64, y: f64) -> Touch {
        Touch {
            device_id: device,
            phase,
            location: PhysicalPosition::new(x, y),
            force: None,
            id: 0,
        }
    }

    #[test]
    fn a_device_that_began_a_touch_is_a_screen() {
        let mut overlay = Overlay::default();
        let screen = DeviceId::dummy();
        assert_eq!(overlay.route(&touch(screen, TouchPhase::Started, 5.0, 5.0)), Routed::Finger);
        assert_eq!(overlay.route(&touch(screen, TouchPhase::Moved, 9.0, 9.0)), Routed::Finger);
        assert_eq!(overlay.route(&touch(screen, TouchPhase::Ended, 9.0, 9.0)), Routed::Finger);
    }

    #[test]
    fn a_move_from_a_device_that_never_began_one_is_a_stick() {
        // winit has one dummy id, so the "pad" is a fresh overlay that has not
        // seen any screen yet: the same state a pad's first move arrives in.
        let mut overlay = Overlay::default();
        let pad = DeviceId::dummy();
        assert_eq!(
            overlay.route(&touch(pad, TouchPhase::Moved, 0.5, -1.0)),
            Routed::Stick(0.5, -1.0)
        );
        assert_eq!(overlay.route(&touch(pad, TouchPhase::Ended, 0.0, 0.0)), Routed::Ignored);
    }
}
