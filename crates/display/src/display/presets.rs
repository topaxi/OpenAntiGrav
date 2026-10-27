//! The sizes the WINDOW SIZE and FULLSCREEN RESOLUTION rows offer, derived from the screen's own
//! size so a 19.5:9 phone, a 16:10 handheld and an ultrawide each get choices in their own shape.
//!
//! **Chosen, not measured.** The original ships no such list; the fractions below are ours. The
//! rule is the same on every platform: fractions of the native size, each rounded to even
//! pixels, so the shape is kept to within a pixel and no preset letterboxes on its own screen.

use super::Size;

/// Fractions of native a window is offered at (numerator, denominator). Native itself is
/// left out: a window that large has no room for its frame.
const WINDOW_FRACTIONS: [(u32, u32); 5] = [(1, 3), (1, 2), (2, 3), (3, 4), (7, 8)];

/// Fractions of native the scene can be drawn at in fullscreen. `native` is its own entry.
const FULLSCREEN_FRACTIONS: [(u32, u32); 4] = [(1, 3), (1, 2), (2, 3), (3, 4)];

/// Below this the picture is not worth offering (a 640x360 floor is a third of 1080p).
const MIN_WIDTH: u32 = 640;
const MIN_HEIGHT: u32 = 360;

/// `value` times `num / den`, rounded to the nearest even number.
fn scaled(value: u32, (num, den): (u32, u32)) -> u32 {
    let half = (value as u64 * num as u64 * 2 + den as u64) / (den as u64 * 2);
    ((half as u32).div_ceil(2) * 2).max(2)
}

fn derive(native: Size, fractions: &[(u32, u32)]) -> Vec<Size> {
    let mut out: Vec<Size> = Vec::new();
    for fraction in fractions {
        let size = Size::new(
            scaled(native.width, *fraction),
            scaled(native.height, *fraction),
        );
        if size.width >= MIN_WIDTH
            && size.height >= MIN_HEIGHT
            && size != native
            && !out.contains(&size)
        {
            out.push(size);
        }
    }
    out
}

/// The window sizes for a screen of `native` pixels, smallest first. Empty when `native` is
/// unknown, which leaves the player's own saved size as the only entry (see [`with_saved`]).
#[must_use]
pub fn window_sizes(native: Option<Size>) -> Vec<Size> {
    native.map_or_else(Vec::new, |native| derive(native, &WINDOW_FRACTIONS))
}

/// The fullscreen resolutions for a screen of `native` pixels: `native` first (spelled as the
/// token, so it follows the screen), then smaller shapes of the same aspect.
#[must_use]
pub fn fullscreen_resolutions(native: Option<Size>) -> Vec<String> {
    let mut out = vec![super::FullscreenResolution::NATIVE.to_string()];
    if let Some(native) = native {
        out.extend(
            derive(native, &FULLSCREEN_FRACTIONS)
                .iter()
                .map(ToString::to_string),
        );
    }
    out
}

/// `choices` with the player's saved value kept in it when it is not already there, so a value
/// from another screen or an older list still shows as itself (a custom entry) rather than
/// being snapped to a neighbour and silently rewritten in their file.
#[must_use]
pub fn with_saved(mut choices: Vec<String>, saved: &str) -> Vec<String> {
    if !choices
        .iter()
        .any(|choice| choice.eq_ignore_ascii_case(saved))
    {
        choices.push(saved.to_string());
    }
    choices
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aspect_error(a: Size, b: Size) -> f64 {
        let ra = a.width as f64 / a.height as f64;
        let rb = b.width as f64 / b.height as f64;
        (ra - rb).abs() / rb
    }

    #[test]
    fn each_shape_gets_aspect_true_sizes_and_native() {
        for (w, h) in [(1920, 1080), (2340, 1080), (1280, 800), (3440, 1440)] {
            let native = Size::new(w, h);
            let sizes = window_sizes(Some(native));
            assert!(sizes.len() >= 3, "{native}: {sizes:?}");
            for size in &sizes {
                assert!(aspect_error(*size, native) < 0.006, "{size} vs {native}");
                assert!(size.width < w && size.height < h);
                assert!(size.width % 2 == 0 && size.height % 2 == 0);
            }
            let fullscreen = fullscreen_resolutions(Some(native));
            assert_eq!(fullscreen[0], "native");
            assert!(fullscreen.len() >= 3, "{native}: {fullscreen:?}");
            for text in &fullscreen[1..] {
                let size: Size = text.parse().expect("a size");
                assert!(aspect_error(size, native) < 0.006, "{size} vs {native}");
            }
        }
    }

    #[test]
    fn full_hd_keeps_the_familiar_sixteen_by_nine_steps() {
        let sizes = window_sizes(Some(Size::new(1920, 1080)));
        for wanted in [(960, 540), (1280, 720), (1440, 810)] {
            assert!(
                sizes.contains(&Size::new(wanted.0, wanted.1)),
                "{wanted:?} in {sizes:?}"
            );
        }
    }

    #[test]
    fn a_small_screen_drops_what_is_not_worth_drawing() {
        let sizes = window_sizes(Some(Size::new(1280, 800)));
        assert!(
            sizes
                .iter()
                .all(|s| s.width >= MIN_WIDTH && s.height >= MIN_HEIGHT)
        );
        assert!(sizes.contains(&Size::new(640, 400)));
    }

    #[test]
    fn an_unknown_screen_offers_native_and_the_saved_value_only() {
        assert!(window_sizes(None).is_empty());
        assert_eq!(fullscreen_resolutions(None), ["native"]);
        assert_eq!(
            with_saved(vec!["native".into()], "1440x816"),
            ["native", "1440x816"]
        );
    }

    #[test]
    fn a_saved_value_off_the_list_is_kept_as_itself() {
        let list = fullscreen_resolutions(Some(Size::new(2340, 1080)));
        let kept = with_saved(list.clone(), "1440x816");
        assert_eq!(kept.len(), list.len() + 1);
        assert_eq!(kept.last().map(String::as_str), Some("1440x816"));
        assert_eq!(with_saved(list.clone(), "Native"), list);
    }
}
