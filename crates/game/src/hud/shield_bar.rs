//! `ShieldBar`'s own colour beyond the plain crop [`super::draw::draw_list`]
//! already applies to every bar: the forced-red override, the shared blink,
//! and the absorb flash layer underneath both.
//!
//! Split out of [`super::draw`] under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change, along the
//! seam [`super::lap_splits`] and [`super::sight_draw`] already used.
//!
//! All three rules are `Hud_UpdateEnergyBar`'s own - see
//! `docs/ghidra/functions/psp-pulse-usa/shield.md#hud_updateenergybar-the-absorb-flash-2026-09-25`
//! for the decompile, and [`crate::hud::Readout::shield_absorbing`],
//! [`crate::hud::Readout::shield_forced_red`] and
//! [`crate::hud::Readout::shield_forced_red`]'s neighbour
//! [`crate::hud::Readout::shield_blinking`] for what each field measures.

use super::{Draw, Readout, Sprite, sprite_draw};

/// Applies `ShieldBar`'s forced-red and blink to `cropped` in place, and
/// returns the absorb flash layer's own draw, if this tick is absorbing.
///
/// **Order matters to the caller**: the flash is meant to paint *underneath*
/// `cropped`, since the blink can zero `cropped`'s own alpha and expose it -
/// see `draw_list`'s own call site, which pushes this return value before
/// `cropped`'s.
///
/// - **The absorb flash.** The original writes a flat `0xff`/`0x00` onto a
///   second field of the same widget (`+0xf4`) in lock-step with
///   [`Readout::shield_absorbing`], with no blink applied to it. *Which*
///   texture or quad that field actually gates was not chased past the
///   write this pass - the widget's own draw method, through its vtable,
///   was not decompiled - so this substitutes a flat white copy of the
///   bar's own asset, not a new texture, painted separately from `cropped`.
///   Confidence 82 for the write's own timing (hard on for the window, hard
///   off outside it); confidence 60 for the geometry and blend standing in
///   for it, which are chosen rather than measured.
/// - **Forced red.** `iVar1` - identified as [`Readout::shield_absorbing`] -
///   suppresses this branch entirely rather than merely being ignored, so a
///   pool at or under the critical floor still reads the bar's own authored
///   colour while absorbing. See [`Readout::shield_forced_red`].
/// - **The blink.** `hud+0x1dc`, scaled by `8.0`, modulates the bar's own
///   alpha on the same three conditions [`Readout::shield_blinking`] reads -
///   low shield, the post-hit flash, and absorbing all included. The
///   accumulator itself ([`Readout::shield_blink_phase`]) is ported, not
///   approximated - see `crate::race::Race::advance_shield_blink`.
pub(super) fn tint(
    cropped: &mut Sprite,
    readout: &Readout,
    sheet: &crate::sprite::Sheet,
) -> Option<Draw> {
    let mut flash = None;
    if readout.shield_absorbing {
        let mut white = cropped.clone();
        white.color = [1.0, 1.0, 1.0, 1.0];
        flash = sprite_draw(&white, sheet);
    }
    if readout.shield_forced_red() {
        cropped.color = [1.0, 0.0, 0.0, cropped.color[3]];
    }
    if readout.shield_blinking() && !readout.shield_blink_phase_on() {
        cropped.color[3] = 0.0;
    }
    flash
}
