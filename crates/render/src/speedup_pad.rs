//! A `Speedup Pad`'s illumination.
//!
//! **Not recovered, and that is the whole of what this module is.**
//! `docs/ghidra/functions/psp-pulse-usa/pads.md` reads `Pad_UpdateRefreshTimer`
//! (`0x089265f0`, `Speedup Pad`'s own class-table `update` slot) as
//! `pad->0x1a0 = max(0.0, pad->0x1a0 - dt)` and nothing else - inert, because
//! nothing anywhere in the executable ever writes a `Speedup Pad`'s `+0x1a0`
//! non-zero. There is no ready/cooling state to represent and no colour cycle
//! driving one, unlike [`crate::weapon_pad`], whose `Weapon Pad` sibling class
//! does both.
//!
//! So a flat blue tint is a **deliberate gameplay-clarity choice with no disc
//! evidence behind it**, applied unconditionally to every speed pad - the same
//! shape of override `docs/gameplay/pickups.md` records for the pickup HUD
//! background (`HudBGColour` substituted for an authored colour pair that
//! renders invisibly as shipped): every value that *can* come off the disc
//! still does, and this is the one constant that cannot.
//!
//! `Drawable::tint_speedup_pads` in `oag_game::race::drawable` is what applies
//! it, the same *replacement*-not-multiply shape `Drawable::tint_weapon_pads`
//! uses for [`crate::weapon_pad::ready_colour`] - the difference being that
//! function's colour is recovered and cycling, and this one is neither.

/// The flat colour every speed pad draws, always - there is no cooldown state
/// on this class to switch between, see this module's own doc comment.
pub const COLOUR: [f32; 3] = [51.0 / 255.0, 128.0 / 255.0, 1.0];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_channel_stays_in_the_unit_range() {
        for c in COLOUR {
            assert!((0.0..=1.0).contains(&c), "COLOUR produced {c}");
        }
    }
}
