//! Photo mode's picture controls: exposure, saturation, speed blur and depth of field.
//!
//! The values are the mode's own and die with it: [`super::Photo`] holds them, so leaving the
//! mode puts the race back as it was. They are render-side only; nothing here reaches the
//! simulation.
//!
//! The sliders are HD/Fury's (`docs/ghidra/functions/ps3-hdfury-eu/photo-mode.md`, "`PhotoMode_Update`"),
//! used on every title:
//!
//! | Row | Range, step | Opens on | Law |
//! | --- | --- | --- | --- |
//! | Exposure | 0 to 100, 1 | `33` | HD opens at `(int)(33.3 - 33.3 g)`; no title here models the race grade `g`, so `g = 0` |
//! | Saturation | 0 to 100, 1 | `20` | `v * 0.05`, so `20` is the colour as it is |
//! | Speed blur | 0 to 100, 5 | `0` | `v * 0.01` into the zoom ring's boost and damage pulses (84) |
//! | Depth of field | off, on | off | HD's list |
//! | Aperture | 0 to 100, 1 | `40` | `v * 0.01` |
//! | Focus | 0 to 100, 1 | `20` | `v * 0.01` into HD's focus curve ([`oag_post::photo_fx::Edges`]) |
//!
//! On a title whose photo mode accumulates (`oag_title::photo::Accumulation`), three rows more:
//! the motion blur (off, track, ship), its shutter (`(v * 0.01)^2 * 0.25` seconds, default `20`)
//! and, where the race can be moved through its past, the rewind (`docs/ui/photo-mode.md`,
//! "Accumulated shots and rewind").
//!
//! **Chosen, not measured:** the direction of the exposure slider (up is brighter; HD's own value
//! falls as the slider rises and what its shader does with it was never read), the exposure as
//! stops of linear light, and the blur and depth of field being independent rows rather than
//! HD's one list.

use oag_post::photo_fx::{Dof, Grade, Params};

/// A row of the effects panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    /// Brightness of the whole picture.
    Exposure,
    /// Colour against grey.
    Saturation,
    /// HD's zoom streak at the picture's edge.
    SpeedBlur,
    /// Whether the far and near fields blur.
    DepthOfField,
    /// How wide the depth of field's blur gets.
    Aperture,
    /// Where the depth of field is sharp.
    Focus,
    /// The race's own moment the picture shows: the scrub back to its start.
    Rewind,
    /// Whether the shot smears the scenery (track) or the craft (ship), or neither.
    MotionBlur,
    /// How long the shutter stays open.
    Shutter,
}

/// What a motion blur shot smears.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blur {
    /// The scenery: the camera rides the craft, so the craft stays sharp.
    Track,
    /// The craft: the camera holds where the shot is taken, so the scenery stays sharp.
    Ship,
}

/// The slider values, as the integers the original shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Effects {
    exposure: i32,
    saturation: i32,
    speed_blur: i32,
    depth_of_field: bool,
    aperture: i32,
    focus: i32,
    selected: Row,
    /// Whether the title's photo mode accumulates frames, so the blur rows are on offer.
    accumulates: bool,
    /// 0 off, 1 track, 2 ship.
    motion_blur: i32,
    shutter: i32,
    /// Whether the race can be moved through its past, so the rewind row is on offer.
    rewinds: bool,
    /// How many ticks before the live tick the picture is, and the live tick.
    back: u64,
    live: u64,
}

impl Default for Effects {
    fn default() -> Self {
        Self::new()
    }
}

impl Effects {
    /// Where exposure opens: HD's `(int)(33.3 - 33.3 g)` at `g = 0`.
    pub const EXPOSURE_OPENS: i32 = 33;
    /// Where saturation opens: the colour as it is.
    pub const SATURATION_OPENS: i32 = 20;
    /// Where the aperture opens.
    pub const APERTURE_OPENS: i32 = 40;
    /// Where the focus opens.
    pub const FOCUS_OPENS: i32 = 20;
    /// The speed blur's step.
    pub const SPEED_BLUR_STEP: i32 = 5;
    /// Where the shutter opens.
    pub const SHUTTER_OPENS: i32 = 20;
    /// Ticks a push of the rewind row moves the picture: half a second.
    pub const REWIND_STEP: u64 = 30;

    /// Every value where the mode opens it: no effect.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            exposure: Self::EXPOSURE_OPENS,
            saturation: Self::SATURATION_OPENS,
            speed_blur: 0,
            depth_of_field: false,
            aperture: Self::APERTURE_OPENS,
            focus: Self::FOCUS_OPENS,
            selected: Row::Exposure,
            accumulates: false,
            motion_blur: 0,
            shutter: Self::SHUTTER_OPENS,
            rewinds: false,
            back: 0,
            live: 0,
        }
    }

    /// The rows for a title whose photo mode accumulates frames, and a race that can be moved
    /// through its past.
    #[must_use]
    pub const fn offering(mut self, accumulates: bool, rewinds: bool) -> Self {
        self.accumulates = accumulates;
        self.rewinds = rewinds;
        self
    }

    /// The rows on offer, top to bottom: aperture and focus only while depth of field is on,
    /// the shutter only while a motion blur is chosen.
    #[must_use]
    pub fn rows(&self) -> Vec<Row> {
        let mut rows = vec![
            Row::Exposure,
            Row::Saturation,
            Row::SpeedBlur,
            Row::DepthOfField,
        ];
        if self.depth_of_field {
            rows.extend([Row::Aperture, Row::Focus]);
        }
        if self.accumulates {
            rows.push(Row::MotionBlur);
            if self.motion_blur != 0 {
                rows.push(Row::Shutter);
            }
        }
        if self.rewinds {
            rows.push(Row::Rewind);
        }
        rows
    }

    /// The row the panel's cursor is on.
    #[must_use]
    pub fn selected(&self) -> Row {
        self.selected
    }

    /// Puts the cursor on `row`, if it is on offer.
    pub fn select(&mut self, row: Row) {
        if self.rows().contains(&row) {
            self.selected = row;
        }
    }

    /// Moves the cursor `by` rows, stopping at the ends.
    pub fn step_row(&mut self, by: i32) {
        let rows = self.rows();
        let at = rows.iter().position(|&r| r == self.selected).unwrap_or(0) as i32;
        let next = (at + by).clamp(0, rows.len() as i32 - 1);
        self.selected = rows[next as usize];
    }

    /// The value of `row` as the panel shows it: a number on `0..=100`, or `0`/`1` for depth of
    /// field.
    #[must_use]
    pub fn value(&self, row: Row) -> i32 {
        match row {
            Row::Exposure => self.exposure,
            Row::Saturation => self.saturation,
            Row::SpeedBlur => self.speed_blur,
            Row::DepthOfField => i32::from(self.depth_of_field),
            Row::Aperture => self.aperture,
            Row::Focus => self.focus,
            Row::Rewind => i32::try_from(self.back).unwrap_or(i32::MAX),
            Row::MotionBlur => self.motion_blur,
            Row::Shutter => self.shutter,
        }
    }

    /// What a motion blur shot smears, or `None` with the row off.
    #[must_use]
    pub fn blur(&self) -> Option<Blur> {
        match self.motion_blur {
            1 => Some(Blur::Track),
            2 => Some(Blur::Ship),
            _ => None,
        }
    }

    /// The shutter row, `0..=100`.
    #[must_use]
    pub fn shutter(&self) -> i32 {
        self.shutter
    }

    /// Whether depth of field is on, and its sliders as fractions `(aperture, focus)`.
    #[must_use]
    pub fn depth_of_field(&self) -> Option<(f32, f32)> {
        self.depth_of_field
            .then_some((self.aperture as f32 * 0.01, self.focus as f32 * 0.01))
    }

    /// Tells the rewind row where the live race stands: the furthest forward it goes and the
    /// furthest back is tick 0.
    pub fn set_live(&mut self, tick: u64) {
        self.live = tick;
        self.back = self.back.min(tick);
    }

    /// The live tick: the rewind row's far end.
    #[must_use]
    pub fn live(&self) -> u64 {
        self.live
    }

    /// The tick the rewind row asks the picture to show.
    #[must_use]
    pub fn target_tick(&self) -> u64 {
        self.live - self.back.min(self.live)
    }

    /// Puts the rewind row where it shows `tick` (clamped to the race so far).
    pub fn set_target_tick(&mut self, tick: u64) {
        self.back = self.live - tick.min(self.live);
    }

    /// Moves the selected row's value `by` steps (negative is down), stopping at its ends.
    pub fn adjust(&mut self, by: i32) {
        self.adjust_row(self.selected, by);
    }

    /// Moves `row`'s value `by` steps.
    pub fn adjust_row(&mut self, row: Row, by: i32) {
        let slide = |value: i32, step: i32| (value + by * step).clamp(0, 100);
        match row {
            Row::Exposure => self.exposure = slide(self.exposure, 1),
            Row::Saturation => self.saturation = slide(self.saturation, 1),
            Row::SpeedBlur => {
                self.speed_blur = slide(self.speed_blur, Self::SPEED_BLUR_STEP);
            }
            Row::DepthOfField => {
                self.depth_of_field = by > 0;
                if !self.depth_of_field {
                    self.selected = Row::DepthOfField;
                }
            }
            Row::Aperture => self.aperture = slide(self.aperture, 1),
            Row::Focus => self.focus = slide(self.focus, 1),
            // Left goes back in the race, right forward to the live tick.
            Row::Rewind => {
                let step = Self::REWIND_STEP.saturating_mul(by.unsigned_abs().into());
                self.back = if by < 0 {
                    self.back.saturating_add(step).min(self.live)
                } else {
                    self.back.saturating_sub(step)
                };
            }
            Row::MotionBlur => self.motion_blur = (self.motion_blur + by).clamp(0, 2),
            Row::Shutter => self.shutter = slide(self.shutter, 1),
        }
    }

    /// Whether the picture is as the camera drew it.
    #[must_use]
    pub fn is_neutral(&self) -> bool {
        self.exposure == Self::EXPOSURE_OPENS
            && self.saturation == Self::SATURATION_OPENS
            && self.speed_blur == 0
            && !self.depth_of_field
    }

    /// The speed blur as the zoom ring's pulse, `0..=1`.
    #[must_use]
    pub fn speed_blur(&self) -> f32 {
        self.speed_blur as f32 * 0.01
    }

    /// The colour and depth of field for the pass, with the projection's planes.
    #[must_use]
    pub fn params(&self, near: f32, far: f32) -> Params {
        let grade = Grade {
            exposure: (self.exposure - Self::EXPOSURE_OPENS) as f32 * Grade::EV_PER_STEP,
            saturation: 1.0 + (self.saturation - Self::SATURATION_OPENS) as f32 * 0.05,
        };
        Params {
            grade: Some(grade),
            dof: self.depth_of_field.then_some(Dof {
                focus: self.focus as f32 * 0.01,
                aperture: self.aperture as f32 * 0.01,
            }),
            near,
            far,
        }
    }
}
