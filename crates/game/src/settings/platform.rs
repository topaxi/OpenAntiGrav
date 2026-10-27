//! What a platform with a fixed surface does to the DISPLAY page: a phone's window is the whole
//! panel and cannot be resized, moved or put on another monitor, so its rows are left out and the
//! game is fullscreen at the panel's native size unless the player picks a smaller render size.
//!
//! Every function takes the platform as an argument (callers pass [`oag_mobile::MOBILE`]) so the
//! rule is tested on the host.

use oag_display::display::{Size, WindowMode, presets};

/// The rows with nothing to choose on a phone. `display.fullscreen_resolution` stays: it is the
/// one size setting that means something there (the render size, stretched to the panel).
pub const MOBILE_HIDDEN_ROWS: [&str; 3] = [
    "display.monitor",
    "display.window_mode",
    "display.window_size",
];

/// The rows this platform leaves out of the DISPLAY page, other than the web build's
/// ([`super::web::HIDDEN_ROWS`]).
#[must_use]
pub fn hidden_rows(mobile: bool) -> &'static [&'static str] {
    if mobile { &MOBILE_HIDDEN_ROWS } else { &[] }
}

/// The window mode a fresh profile starts in: fullscreen on a phone.
#[must_use]
pub fn default_window_mode(mobile: bool) -> WindowMode {
    if mobile {
        WindowMode::Borderless
    } else {
        WindowMode::Windowed
    }
}

/// Whether the window system's fullscreen mode is unavailable to the game and the window is
/// fullscreen regardless: a phone, or gamescope (one window per output, no decoration).
#[must_use]
pub fn surface_is_fixed(mobile: bool, gamescope: bool) -> bool {
    mobile || gamescope
}

/// The WINDOW SIZE and FULLSCREEN RESOLUTION rows' choices for a screen of `native` pixels, each
/// keeping the player's saved value when it is not among the derived ones.
#[must_use]
pub fn size_choices(native: Option<Size>, display: &super::Display) -> (Vec<String>, Vec<String>) {
    let windows = presets::window_sizes(native)
        .iter()
        .map(ToString::to_string)
        .collect();
    let fullscreen = presets::fullscreen_resolutions(native);
    (
        presets::with_saved(windows, &display.window_size.to_string()),
        presets::with_saved(fullscreen, &display.fullscreen_resolution.to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_display::display;

    #[test]
    fn a_phone_hides_the_window_rows_and_starts_fullscreen_at_native() {
        assert_eq!(default_window_mode(true), WindowMode::Borderless);
        assert_eq!(default_window_mode(false), WindowMode::Windowed);
        assert!(hidden_rows(false).is_empty());
        for row in [
            "display.window_mode",
            "display.window_size",
            "display.monitor",
        ] {
            assert!(hidden_rows(true).contains(&row), "{row}");
        }
        assert!(!hidden_rows(true).contains(&"display.fullscreen_resolution"));
        assert!(surface_is_fixed(true, false) && surface_is_fixed(false, true));
        assert!(!surface_is_fixed(false, false));
        assert_eq!(
            display::FullscreenResolution::default(),
            display::FullscreenResolution::Native
        );
    }

    #[test]
    fn the_size_rows_follow_the_screen_and_keep_a_saved_custom_value() {
        let mut display = super::super::Display::default();
        let (windows, fullscreen) = size_choices(Some(Size::new(2340, 1080)), &display);
        assert!(windows.contains(&"1170x540".to_string()), "{windows:?}");
        assert_eq!(fullscreen[0], "native");
        assert!(
            windows.contains(&"1440x816".to_string()),
            "saved default kept: {windows:?}"
        );
        display.fullscreen_resolution = "1600x900".parse().expect("size");
        let (_, fullscreen) = size_choices(Some(Size::new(2340, 1080)), &display);
        assert_eq!(fullscreen.last().map(String::as_str), Some("1600x900"));
    }
}
