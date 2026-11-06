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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
