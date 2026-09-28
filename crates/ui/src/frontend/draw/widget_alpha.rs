//! A widget's own alpha multiplier at a given `elapsed` time - `pulse_alpha`
//! for a `pulse`/`delay` `Text`, `fade_alpha` for a `transition`-bearing
//! `Image`, and `reveal_delta` for an `<Animation>`-wrapped widget's own
//! width wipe - split out of `draw.rs` under the 1,000-line rule
//! (`scripts/check-file-size.py`), the same shape `storage_warning.rs`/
//! `transform.rs` already split their own seams into. A move, not a
//! behaviour change.

use crate::screen::Text;

/// The throb's period, in seconds, for a [`Text`] widget whose `pulse` is set.
///
/// Measured off a real capture of `BOOT_PRESS_START` on `Show Logo`
/// (`pulse-psp-eu.chd` under PPSSPPSDL, two independent 20s captures,
/// confidence 75 - a screen-pixel luminance proxy, not a live memory read).
/// See `docs/architecture/frontend-boot.md`'s `BOOT_PRESS_START does not
/// pulse` section. No other `pulse="true"` widget has been captured, so this
/// is applied to any future one too rather than left unimplemented, on the
/// same "measure one, extrapolate rather than invent a second value" basis
/// the rest of this crate uses for shared constants.
pub(in crate::frontend) const PULSE_PERIOD: f32 = 1.10;

/// The throb's dim floor, as a fraction of the widget's own authored alpha.
///
/// The same capture put the dim phase at roughly 42% of the bright phase's
/// luminance - never fully faded to black - and the bright phase at roughly
/// the widget's own static (currently: only) rendered alpha, so 1.0 is the
/// ceiling this multiplies against rather than a second measured number.
pub(in crate::frontend) const PULSE_FLOOR: f32 = 0.42;

/// A `pulse`/`delay` widget's alpha multiplier, `elapsed` seconds after its
/// screen appeared. `1.0` for a non-pulsing widget or an infinite `elapsed`
/// (`draw_screen`'s "settled" default) - both read as "just use the authored
/// colour". Otherwise: `0.0` before `delay` has elapsed (on hardware the
/// widget is not drawn at all yet), then a sine throb between [`PULSE_FLOOR`]
/// and `1.0` with period [`PULSE_PERIOD`], itself ramped in linearly over its
/// first cycle so the widget's first appearance is a fade rather than a pop
/// at the dim floor.
///
/// **The ramp's own shape and duration are chosen, not measured.** The
/// capture only covers the repeating throb, not the fade-in `delay="1"`
/// implies; using one throb period as the fade-in's length reuses the one
/// timescale that *is* measured rather than inventing an unrelated second
/// one, and a plain sine stands in for the throb's own shape, which the
/// capture shows is not quite sinusoidal (it holds near each extreme rather
/// than smoothly reversing - a detail not modelled here). See
/// `docs/architecture/frontend-boot.md`'s `BOOT_PRESS_START now pulses`
/// section for the capture this is built on.
pub(in crate::frontend) fn pulse_alpha(text: &Text, elapsed: f64) -> f32 {
    if !text.pulse || !elapsed.is_finite() {
        return 1.0;
    }
    let since_delay = elapsed - f64::from(text.delay);
    if since_delay < 0.0 {
        return 0.0;
    }
    let cycles = (since_delay / f64::from(PULSE_PERIOD)) as f32;
    let wave = (cycles.fract() * std::f32::consts::TAU).sin();
    let settled = PULSE_FLOOR + (1.0 - PULSE_FLOOR) * (wave + 1.0) / 2.0;
    settled * cycles.min(1.0)
}

/// [`crate::screen::interpolate_reveal`] at `elapsed`, `0.0` for a widget no
/// `<Animation>` wraps (`keys` empty) - the same "no clock, no motion" shape
/// [`pulse_alpha`] gives an unpulsed `Text`.
pub(in crate::frontend) fn reveal_delta(keys: &[crate::screen::RevealKey], elapsed: f64) -> f32 {
    crate::screen::interpolate_reveal(keys, elapsed as f32)
}

/// An `Image`'s own alpha multiplier, `elapsed` seconds after its screen
/// appeared - `1.0` (fully visible, "just use the authored colour") for a
/// non-finite `elapsed` or a `transition` of zero or less, otherwise a
/// linear ramp from `0.0` to `1.0` over `transition` seconds and clamped
/// there. Mirrors `Element_UpdateFade`'s own two relevant branches
/// (`docs/ghidra/functions/psp-pure-eu/title-screen.md`): the `fVar2==0.0`
/// case snaps straight to shown, and the entering case is
/// `elapsed / duration` clamped - matching Pulse's own confirmed-linear
/// `Widget_UpdateTransitionFraction`
/// (`docs/ghidra/functions/psp-pulse-usa/race-box-screens.md`), not a
/// separately chosen curve. See [`crate::screen::resolve_fade_in`] for
/// where `transition` itself comes from.
pub(in crate::frontend) fn fade_alpha(transition: f32, elapsed: f64) -> f32 {
    if !elapsed.is_finite() || transition <= 0.0 {
        return 1.0;
    }
    ((elapsed / f64::from(transition)) as f32).clamp(0.0, 1.0)
}
