//! `ShieldBar`'s own colour beyond the plain crop [`super::draw::draw_list`]
//! already applies to every bar: the forced-red override, the shared blink,
//! and the absorb flash layer underneath both.
//!
//! The measured form of the absorb flash, where a title has an [`oag_title::hud::AbsorbGlow`], is
//! the glow mask under the fill (recorded in `super::draw::draw_list`), not a white layer.
//!
//! Split out of [`super::draw`] under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change, along the
//! seam [`super::lap_splits`] and [`super::sight_draw`] already used.
//!
//! All three rules are `Hud_UpdateEnergyBar`'s own - see
//! `docs/ghidra/functions/psp-pulse-usa/shield.md#hud_updateenergybar-the-absorb-flash-2026-09-25`
//! for the decompile, and [`crate::Readout::shield_absorbing`],
//! [`crate::Readout::shield_forced_red`] and
//! [`crate::Readout::shield_forced_red`]'s neighbour
//! [`crate::Readout::shield_blinking`] for what each field measures.

use super::{Draw, Readout, Sprite};

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
///   [`Readout::shield_absorbing`], with no blink applied to it. It is the
///   stencil reference of the fill's draw: the fill stamps the bloom's glow
///   mask, which `draw_list` records where the title's art has an
///   [`oag_title::hud::AbsorbGlow`], and no white layer is drawn. Where it
///   has none (the draw is unmeasured), this substitutes a flat white copy
///   of the bar's own asset painted separately from `cropped`, **chosen, not
///   measured**.
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
    src: &str,
    readout: &Readout,
    art: &oag_title::HudArt,
    sheet: &crate::sprite::Sheet,
) -> Option<Draw> {
    let mut flash = None;
    if readout.shield_absorbing && art.absorb_glow.is_none() {
        let mut white = cropped.clone();
        white.color = [1.0, 1.0, 1.0, 1.0];
        flash = super::draw::sprite_draw_from(src, &white, sheet);
    }
    if readout.shield_forced_red() {
        cropped.color = [1.0, 0.0, 0.0, cropped.color[3]];
    }
    if readout.shield_blinking() && !readout.shield_blink_phase_on() {
        cropped.color[3] = 0.0;
    }
    flash
}
