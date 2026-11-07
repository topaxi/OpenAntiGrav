//! Which of photo mode's words a title's own disc already says.
//!
//! Photo mode is one screen for every title (`oag_raceplay::photo`). Where the disc authors a
//! word for one of its rows (Pulse's `ER_PHOTO_CAM`, HD's `MSC_PHOTO_PAN`) the screen plays the
//! disc's own text, in the player's language; where it does not, the project's own text stands
//! (`assets/ui/strings/english.toml`, `OAG_PHOTO_*`). Per-title behaviour is data here, not a
//! comparison on a title's name (ADR-0058). The ids are
//! `docs/ui/photo-mode.md`'s, by binary.

/// One row or line of the photo mode screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    /// The camera row.
    Camera,
    /// The HUD row.
    Hud,
    /// The help toggle.
    Help,
    /// Rotating the camera.
    Rotate,
    /// Panning the camera.
    Pan,
    /// Resetting the camera.
    Reset,
    /// Choosing the craft.
    Ship,
    /// The speed blur row.
    SpeedBlur,
    /// The depth of field row.
    Depth,
}

/// The disc string id, in the title's own string table, that words each [`Row`], or `None` where
/// the disc has no word for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Strings {
    pub camera: Option<&'static str>,
    pub hud: Option<&'static str>,
    pub help: Option<&'static str>,
    pub rotate: Option<&'static str>,
    pub pan: Option<&'static str>,
    pub reset: Option<&'static str>,
    pub ship: Option<&'static str>,
    pub speed_blur: Option<&'static str>,
    pub depth: Option<&'static str>,
}

impl Strings {
    /// A title with no photo mode of its own to take words from: every row is the project's.
    pub const NONE: Self = Self {
        camera: None,
        hud: None,
        help: None,
        rotate: None,
        pan: None,
        reset: None,
        ship: None,
        speed_blur: None,
        depth: None,
    };

    /// Pulse's `Photo Mode Top Level`: its two rows (`docs/ui/photo-mode.md`, "Pulse").
    pub const PULSE: Self = Self {
        camera: Some("ER_PHOTO_CAM"),
        hud: Some("ER_PHOTO_HUD"),
        ..Self::NONE
    };

    /// HD/Fury's `InGame Photo` help lines and HUD row, which Omega's own `InGame Photo`
    /// carries over (`docs/ui/photo-mode.md`, "HD/Fury" and "Omega").
    pub const HD: Self = Self {
        camera: None,
        hud: Some("MSC_PHOTO_HUD"),
        help: Some("MSC_PHOTO_HELP"),
        rotate: Some("MSC_PHOTO_ROT"),
        pan: Some("MSC_PHOTO_PAN"),
        reset: Some("MSC_PHOTO_RESET"),
        ship: Some("MSC_PHOTO_SHIP"),
        speed_blur: Some("MSC_PHOTO_SPEED"),
        depth: Some("MSC_PHOTO_DEPTH"),
    };

    /// The disc id for `row`.
    #[must_use]
    pub const fn disc_id(&self, row: Row) -> Option<&'static str> {
        match row {
            Row::Camera => self.camera,
            Row::Hud => self.hud,
            Row::Help => self.help,
            Row::Rotate => self.rotate,
            Row::Pan => self.pan,
            Row::Reset => self.reset,
            Row::Ship => self.ship,
            Row::SpeedBlur => self.speed_blur,
            Row::Depth => self.depth,
        }
    }
}

/// What a title's photo mode accumulates into one shot: the frames of a depth of field and of a
/// motion blur, read off the executable (`docs/ui/photo-mode.md`, "Accumulated shots").
///
/// `None` on a title whose original has no accumulating shot; the shot is then the one frame the
/// player sees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Accumulation {
    /// Frames a depth of field shot is made of (HD `7 further`, 2048 `7`).
    pub dof_frames: u32,
    /// Frames a motion blur shot is made of (HD `0x78`, 2048 `0x82`).
    pub blur_frames: u32,
    /// What the shutter's seconds are multiplied by for the ship blur: 2048 reads `0.05`
    /// (`docs/ghidra/functions/vita-2048-eu-v104/photo-mode.md`). HD's own ship blur was not
    /// read apart from the track's; it takes 2048's, **chosen, not measured**, because at `1.0`
    /// the craft crosses a held camera in a fraction of the window and leaves no picture.
    pub ship_shutter: f32,
}

impl Accumulation {
    /// Wipeout HD/Fury (`docs/ghidra/functions/ps3-hdfury-eu/photo-mode.md`).
    pub const HD: Self = Self {
        dof_frames: 7,
        blur_frames: 120,
        ship_shutter: 0.05,
    };

    /// Wipeout 2048 (`docs/ghidra/functions/vita-2048-eu-v104/photo-mode.md`).
    pub const WIPEOUT_2048: Self = Self {
        dof_frames: 7,
        blur_frames: 130,
        ship_shutter: 0.05,
    };

    /// The longest a shutter stays open, in seconds: `(100 * 0.01)^2 * 0.25`.
    pub const LONGEST_SHUTTER: f32 = 0.25;

    /// The seconds a shutter row of `value` (`0..=100`) stays open: `(value * 0.01)^2 * 0.25`.
    #[must_use]
    pub fn shutter_seconds(value: i32) -> f32 {
        let v = value.clamp(0, 100) as f32 * 0.01;
        v * v * Self::LONGEST_SHUTTER
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shutter_runs_from_nothing_to_a_quarter_of_a_second() {
        assert_eq!(Accumulation::shutter_seconds(0), 0.0);
        assert_eq!(Accumulation::shutter_seconds(100), 0.25);
        assert!((Accumulation::shutter_seconds(20) - 0.01).abs() < 1e-6);
        assert_eq!(Accumulation::shutter_seconds(900), 0.25);
    }

    #[test]
    fn a_title_with_none_has_no_disc_word_for_any_row() {
        for row in [
            Row::Camera,
            Row::Hud,
            Row::Help,
            Row::Rotate,
            Row::Pan,
            Row::Reset,
            Row::Ship,
            Row::SpeedBlur,
            Row::Depth,
        ] {
            assert_eq!(Strings::NONE.disc_id(row), None);
        }
    }

    #[test]
    fn pulse_words_its_two_rows_and_hd_its_help_lines() {
        assert_eq!(Strings::PULSE.disc_id(Row::Camera), Some("ER_PHOTO_CAM"));
        assert_eq!(Strings::PULSE.disc_id(Row::Pan), None);
        assert_eq!(Strings::HD.disc_id(Row::Pan), Some("MSC_PHOTO_PAN"));
        assert_eq!(Strings::HD.disc_id(Row::Camera), None);
    }
}
