//! `<Animation><Key>`'s own timeline and interpolation - split out of
//! `screen.rs` under the 1,000-line rule (`scripts/check-file-size.py`), the
//! same shape `widgets.rs`/`tests.rs` already split off for their own seams.
//! A move, not a behaviour change.

/// One `<Animation><Key>` on a wrapped [`super::Image`]/[`super::Fill`]: a
/// texture-space width delta at a point in time.
///
/// Pure's `Title Screen` wraps its frame lines, corner brackets and small
/// textured patches (the barcode, the double arrows, the Japanese
/// characters, the tiny text) each in their own `<Animation>`, and every one
/// of the thirteen brackets `TextureWidth` between `-<width>` (the wrapped
/// widget's own authored width, negated - fully hidden) and `0` (fully
/// shown) over the animation's own key times. Read off a live PPSSPP capture
/// rather than guessed: `Animation_Update`/`Animation_InterpolateKeys`
/// (`docs/ghidra/functions/psp-pure-eu/title-screen.md`) linearly interpolate
/// between the two keys bracketing the current time, and
/// `Animation_ComputeRect` adds the result straight into the wrapped
/// widget's own computed width - confirmed live via a `08ab90f8`/`+0x24`
/// vtable dispatch and, separately, by a non-halting write watch on a real
/// `TitleAnim` object during a scripted boot into `Title Screen`.
///
/// The wrapped widget's own `X`/`Y`/`TextureHeight`/`ScaleX`/`ScaleY` are
/// never authored on this screen (`Time`/`TextureWidth` only), so only those
/// two fields are modelled - see that page's own struct table for the other
/// four, left unread because nothing on any title's disc measured so far
/// authors them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RevealKey {
    /// Seconds since the wrapping `<Animation>` started.
    pub time: f32,
    /// Added to the wrapped widget's own authored width once interpolated -
    /// authored negative, ramping to zero.
    pub texture_width: f32,
}

/// The interpolated `TextureWidth` at `elapsed` seconds, added to the
/// wrapped widget's own authored width by the caller - `0.0` for an empty
/// `keys` (no wrapping `<Animation>`, today's behaviour unchanged).
///
/// Mirrors `Animation_InterpolateKeys`
/// (`docs/ghidra/functions/psp-pure-eu/title-screen.md`): walk to the last
/// key at or before `elapsed`, then linearly interpolate to the next one.
/// Before the first key or after the last, the boundary key's own value
/// holds rather than extrapolating - `FixedFrames`/`Loop` are not modelled
/// since nothing measured on any title's disc authors either as anything but
/// their defaults (`false`) on a widget this reads.
#[must_use]
pub fn interpolate_reveal(keys: &[RevealKey], elapsed: f32) -> f32 {
    let Some(first) = keys.first() else {
        return 0.0;
    };
    if elapsed <= first.time {
        return first.texture_width;
    }
    for pair in keys.windows(2) {
        let [before, after] = pair else {
            unreachable!()
        };
        if elapsed <= after.time {
            let t = (elapsed - before.time) / (after.time - before.time);
            return before.texture_width + t * (after.texture_width - before.texture_width);
        }
    }
    keys[keys.len() - 1].texture_width
}
