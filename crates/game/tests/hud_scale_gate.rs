//! Which titles obey `[graphics] hud_scale`: the ones whose HUD is raster art.

use oag_display::display::HudScale;
use oag_game::hud_overlay::stretch_for;

#[test]
fn pulse_and_pure_take_the_players_stretch() {
    for (name, art) in [("pulse", oag_pulse::hud::ART), ("pure", oag_pure::hud::ART)] {
        for mode in HudScale::ALL {
            assert_eq!(stretch_for(art, mode), mode, "{name} {mode}");
        }
    }
}

#[test]
fn hd_2048_and_omega_draw_linear_whatever_is_asked() {
    for (name, art) in [
        ("hd", oag_hd::hud::ART),
        ("2048", oag_2048::hud::ART),
        ("omega", oag_omega::hud::ART),
    ] {
        for mode in HudScale::ALL {
            assert_eq!(stretch_for(art, mode), HudScale::Linear, "{name} {mode}");
        }
    }
}
