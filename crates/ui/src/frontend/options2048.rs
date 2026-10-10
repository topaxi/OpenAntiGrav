//! `Options_Definition.xml`: the settings hub `Home`'s own `FE_OPT_PLUS`
//! tile jumps straight into, and its four camera/audio/controls/pilot
//! sub-panels.
//!
//! # Hub versus panel
//!
//! `optionsshell->options` (`w2048::OPTIONS`) is an ordinary touch grid - four
//! `<TouchButton>`s (`Camera Button`/`Audio Button`/`Controls Button`/`Pilot
//! Button`) at `x` 95-345, each `redirect`-ing to one of the four screens
//! below, plus the tick to `save_2048_options`. That is real `<TouchButton>`
//! data, so it draws and answers to a tap through the same generic path
//! `Home`'s own tiles already use - nothing in this module touches it.
//!
//! The four sub-panels (`OptionsCamera`/`OptionsAudio`/`OptionsControls`/
//! `OptionsPilot`) are **nested inside** `options` in the XML, not siblings
//! of it - their own `Touchlist`/`TouchSlider` widgets sit at `x` 430-940,
//! the hub's own buttons stay at `x` 95-345, and neither overlaps the other.
//! None of the four carries a header or a back button of its own (contrast
//! `OptionsControlsConfig`, a true sibling page that does carry both) - so
//! this module draws the **hub's** own widgets first, for the header and the
//! four entry buttons still visibly present, then the panel's own control on
//! top, and returns to the hub on a Circle press this build supplies:
//! `NEWGUI` authors no button for it anywhere on these four screens, the same
//! standing `newFEshell`'s own `<TouchHomeButton>` targets are on.
//! **`Home`'s own `FE_OPT_PLUS` tile jumps straight to `OptionsCamera`, never
//! to `options` itself** - read directly off `Definition.xml`, not a guess.
//!
//! # What is wired, and what is drawn inert
//!
//! **`OptionsCamera`'s `CameraP1` list** cycles
//! [`oag_display::display::CameraView`]'s three values in the file's own
//! order (`OPT_CLOSE`/`OPT_FAR`/`OPT_INT`) and **`OptionsAudio`'s two
//! `TouchSlider`s** step [`oag_sound::Volume`] - wait, this crate is
//! `oag-ui` and does not know that type; see [`Frontend::camera_choice`]/
//! [`Frontend::music_choice`]/[`Frontend::sfx_choice`], which carry a plain
//! index/percentage for the composition root to apply, the same "`None`
//! until touched" contract [`Frontend::chosen`] already sets for the
//! language picker; [`Frontend::pilot_choice`] is `OptionsPilot`'s `Pilot
//! Assist` list on the same terms. **`OptionsControls`'s `Motion Sensor` list
//! draws and cycles and reaches nothing** - this build has no motion-sensor axis.
//!
//! **The starting index is `CameraP1`'s own authored `default="OPT_CLOSE"`
//! (index 0), which is also what [`oag_display::display::CameraView::default`]
//! now is** - a fresh Pulse profile starts on `OPT_CLOSE` too (measured
//! 2026-10-01, `camera.md`). Until then this picker started on `OPT_FAR`
//! because the type behind it did. Threading the live `Settings` into
//! `Frontend::booting` so the picker could seed itself from the session's
//! actual value needs `boot::assemble`'s own signature widened, which touches
//! every title's boot path for a value only this one reads; left as a
//! follow-up.

use oag_2048::frontend::states as w2048;

use crate::pointer::{Pointer, contains};
use crate::screen::{TouchList, TouchSlider};

use super::*;

/// `CameraP1`'s own three entries, in file order - `OptionsCamera`.
const CAMERA_ENTRIES: [&str; 3] = ["OPT_CLOSE", "OPT_FAR", "OPT_INT"];

/// `CameraP1`'s authored `default="OPT_CLOSE"`: index 0 of [`CAMERA_ENTRIES`].
const CAMERA_DEFAULT_INDEX: u8 = 0;
/// `Pilot Assist`'s own three entries - `OptionsPilot`. See [`Frontend::pilot_choice`].
const PILOT_ENTRIES: [&str; 3] = ["FE_OFF", "FE_NORMAL", "FE_SUPER"];
/// `Pilot Assist`'s authored `default="FE_NORMAL"`: index 1 of [`PILOT_ENTRIES`].
const PILOT_DEFAULT_INDEX: u8 = 1;
/// `Motion Sensor`'s own three entries - `OptionsControls`. Drawn, not wired.
const CONTROLS_ENTRIES: [&str; 3] = ["FE_CTRL_WIPEOUT", "FE_CTRL_RACER", "FE_CTRL_MOTION"];

const VOLUME_STEP: i32 = 10;

impl Frontend {
    /// `OptionsCamera`'s own choice, once the player has moved it - `0`
    /// close, `1` far, `2` internal, `CameraP1`'s own entry order. `None`
    /// until touched.
    #[must_use]
    pub fn camera_choice(&self) -> Option<u8> {
        self.touch.camera_choice
    }

    /// `OptionsPilot`'s own choice once moved - `0` off, `1` normal, `2` super,
    /// the list's own order. `None` until touched.
    #[must_use]
    pub fn pilot_choice(&self) -> Option<u8> {
        self.touch.pilot_choice
    }

    /// `OptionsAudio`'s `Music Volume` slider, `0..=100`. `None` until moved.
    #[must_use]
    pub fn music_choice(&self) -> Option<u32> {
        self.touch.music_choice
    }

    /// `OptionsAudio`'s `SFX Volume` slider, `0..=100`. `None` until moved.
    #[must_use]
    pub fn sfx_choice(&self) -> Option<u32> {
        self.touch.sfx_choice
    }

    pub(super) fn update_options_panel(&mut self, input: &mut Input, current: &str) {
        let step = if input.is_pressed(Button::Right) {
            input.consume_press(Button::Right);
            1i32
        } else if input.is_pressed(Button::Left) {
            input.consume_press(Button::Left);
            -1i32
        } else {
            0
        };
        if step != 0 {
            self.step_panel(current, step);
        }
        if current == w2048::OPTIONS_AUDIO {
            if input.is_pressed(Button::Down) {
                input.consume_press(Button::Down);
                self.touch.audio_slider = 1;
            } else if input.is_pressed(Button::Up) {
                input.consume_press(Button::Up);
                self.touch.audio_slider = 0;
            }
        }
        if input.is_pressed(Button::Circle) {
            input.consume_press(Button::Circle);
            self.notes.push(format!(
                "{current}: Circle pressed, returning to {} - chosen, no widget names this button",
                w2048::OPTIONS
            ));
            self.machine.fire(w2048::OPTIONS);
        }
    }

    /// One step (`+1`/`-1`) on whichever control `current` owns - the shared
    /// core of the pad's Left/Right and a pointer click on the widget's own
    /// rect.
    fn step_panel(&mut self, current: &str, step: i32) {
        match current {
            w2048::OPTIONS_CAMERA => {
                let len = CAMERA_ENTRIES.len() as i32;
                let index = self.touch.camera_choice.unwrap_or(CAMERA_DEFAULT_INDEX) as i32;
                let index = (index + step).rem_euclid(len);
                self.touch.camera_choice = Some(index as u8);
                self.notes
                    .push(format!("OptionsCamera: {}", CAMERA_ENTRIES[index as usize]));
            }
            w2048::OPTIONS_CONTROLS => {
                let len = CONTROLS_ENTRIES.len() as i32;
                self.touch.controls_index =
                    (self.touch.controls_index as i32 + step).rem_euclid(len) as usize;
            }
            w2048::OPTIONS_PILOT => {
                let len = PILOT_ENTRIES.len() as i32;
                let index = self.touch.pilot_choice.unwrap_or(PILOT_DEFAULT_INDEX) as i32;
                self.touch.pilot_choice = Some((index + step).rem_euclid(len) as u8);
            }
            w2048::OPTIONS_AUDIO => {
                let current_value = if self.touch.audio_slider == 0 {
                    self.touch.music_choice.unwrap_or(100)
                } else {
                    self.touch.sfx_choice.unwrap_or(100)
                };
                let next = (current_value as i32 + step * VOLUME_STEP).clamp(0, 100) as u32;
                if self.touch.audio_slider == 0 {
                    self.touch.music_choice = Some(next);
                    self.notes
                        .push(format!("OptionsAudio: Music Volume {next}"));
                } else {
                    self.touch.sfx_choice = Some(next);
                    self.notes.push(format!("OptionsAudio: SFX Volume {next}"));
                }
            }
            _ => {}
        }
    }

    /// The pointer on a settings panel: a click inside the widget's own rect
    /// steps it forward by one, the way a d-pad Right already does, since a
    /// `Touchlist`'s per-entry rects are chosen (see `draw_touch_list`'s own
    /// doc) and a generic in-rect click is honest about not claiming to hit
    /// one entry rather than another.
    pub(super) fn options_panel_pointer(&mut self, pointer: &Pointer) -> bool {
        let Some(current) = self.machine.current().map(str::to_string) else {
            return false;
        };
        if !pointer.clicked {
            return true;
        }
        let Some(at) = pointer.at else {
            return true;
        };
        let widget_rect = self.screens.by_name(&current).and_then(|screen| {
            screen
                .touch_lists
                .first()
                .map(|list| [list.x, list.y, list.width, list.height])
                .or_else(|| {
                    screen
                        .touch_sliders
                        .first()
                        .map(|slider| [slider.x, slider.y, slider.width, slider.height])
                })
        });
        if widget_rect.is_some_and(|rect| contains(rect, at)) {
            self.step_panel(&current, 1);
        }
        true
    }

    /// `options`'s own widgets, then the sub-panel's `Touchlist`/`TouchSlider`
    /// on top - see the module docs for why both draw.
    pub(super) fn draw_options_panel(&self, current: &str, out: &mut Vec<Draw>) {
        out.extend(self.draw_screen_at(w2048::OPTIONS, self.on_screen_for));
        let Some(screen) = self.screens.by_name(current) else {
            return;
        };
        let cursor = self.global_colour("Orange2048");
        match current {
            w2048::OPTIONS_CAMERA => {
                if let Some(list) = screen.touch_lists.first() {
                    self.draw_touch_list(
                        list,
                        &CAMERA_ENTRIES,
                        self.touch.camera_choice.unwrap_or(CAMERA_DEFAULT_INDEX) as usize,
                        cursor,
                        out,
                    );
                }
            }
            w2048::OPTIONS_CONTROLS => {
                if let Some(list) = screen.touch_lists.first() {
                    self.draw_touch_list(
                        list,
                        &CONTROLS_ENTRIES,
                        self.touch.controls_index,
                        cursor,
                        out,
                    );
                }
            }
            w2048::OPTIONS_PILOT => {
                if let Some(list) = screen.touch_lists.first() {
                    self.draw_touch_list(
                        list,
                        &PILOT_ENTRIES,
                        self.touch.pilot_choice.unwrap_or(PILOT_DEFAULT_INDEX) as usize,
                        cursor,
                        out,
                    );
                }
            }
            w2048::OPTIONS_AUDIO => {
                for (i, slider) in screen.touch_sliders.iter().enumerate() {
                    let value = if i == 0 {
                        self.touch.music_choice.unwrap_or(100)
                    } else {
                        self.touch.sfx_choice.unwrap_or(100)
                    };
                    self.draw_touch_slider(
                        slider,
                        value,
                        i == self.touch.audio_slider,
                        cursor,
                        out,
                    );
                }
            }
            _ => {}
        }
    }

    /// One `Touchlist`, its `len` entries spaced evenly across its own
    /// authored rect - **chosen, not measured**, the same standing every
    /// per-entry layout in this crate's touch screens carries: the widget
    /// authors one rect for the whole strip and no per-entry position.
    fn draw_touch_list(
        &self,
        list: &TouchList,
        entries: &[&str],
        selected: usize,
        cursor: [f32; 4],
        out: &mut Vec<Draw>,
    ) {
        let tile = self.global_colour("Blue2048");
        let count = entries.len().max(1) as f32;
        let cell_w = list.width / count;
        for (i, label) in entries.iter().enumerate() {
            let rect = if list.vertical {
                [
                    list.x,
                    list.y + (list.height / count) * i as f32,
                    list.width,
                    list.height / count,
                ]
            } else {
                [list.x + cell_w * i as f32, list.y, cell_w, list.height]
            };
            out.push(Draw::Fill { rect, color: tile });
            if i == selected {
                self.draw_cursor_ring(rect, cursor, out);
            }
            out.push(Draw::Text {
                x: rect[0] + rect[2] * 0.5,
                y: rect[1] + rect[3] * 0.5 - 8.0,
                scale: 0.5,
                color: [1.0, 1.0, 1.0, 1.0],
                border: None,
                align: Align::Centre,
                text: self.strings.get_or_id(label).to_string(),
                wrap_width: Some(rect[2] - 4.0),
            });
        }
    }

    fn draw_touch_slider(
        &self,
        slider: &TouchSlider,
        value: u32,
        focused: bool,
        cursor: [f32; 4],
        out: &mut Vec<Draw>,
    ) {
        let tile = self.global_colour("Blue2048");
        let rect = [slider.x, slider.y, slider.width, slider.height];
        out.push(Draw::Fill { rect, color: tile });
        let filled_width = if slider.max > slider.min {
            slider.width * (value as f32 - slider.min) / (slider.max - slider.min)
        } else {
            0.0
        };
        out.push(Draw::Fill {
            rect: [
                slider.x,
                slider.y,
                filled_width.clamp(0.0, slider.width),
                slider.height,
            ],
            color: cursor,
        });
        if focused {
            self.draw_cursor_ring(rect, cursor, out);
        }
        let label = slider
            .idstring
            .as_deref()
            .map(|id| self.strings.get_or_id(id).to_string())
            .unwrap_or_default();
        out.push(Draw::Text {
            x: rect[0] + rect[2] * 0.5,
            y: rect[1] + rect[3] * 0.5 - 8.0,
            scale: 0.5,
            color: [1.0, 1.0, 1.0, 1.0],
            border: None,
            align: Align::Centre,
            text: format!("{label} {value}%"),
            wrap_width: Some(rect[2] - 4.0),
        });
    }
}
