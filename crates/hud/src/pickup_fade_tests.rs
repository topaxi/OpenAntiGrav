//! The pickup fade against the 29 live Missile words, the 24 fade-out words and the size axes
//! recorded in `docs/ghidra/functions/psp-pulse-usa/pickup-colour.md`, plus the state machine.

use oag_tables::weapons::Weapon;

use super::pickup_fade::{Fade, ease, ease_with_bump, scale_word};

const MISSILE: u32 = 0x80BE_1E69;

/// `(T, backdrop word)` of each `Hud_UpdatePickupWidgets` call from the grant: the word at a
/// call's entry was written from that call's `T`. `T` is printed to four decimals.
const FADE_IN: [(f32, u32); 27] = [
    (0.0000, 0x0000_0000),
    (0.0202, 0x0101_0000),
    (0.0401, 0x0406_0103),
    (0.0602, 0x0A0E_0208),
    (0.0801, 0x1119_040E),
    (0.1001, 0x1A27_0615),
    (0.1204, 0x2436_081E),
    (0.1402, 0x3047_0B27),
    (0.1602, 0x3C59_0E31),
    (0.1806, 0x4A6D_113C),
    (0.2002, 0x6190_1650),
    (0.2202, 0x98E2_237D),
    (0.2402, 0xDEFF_34B6),
    (0.2603, 0xE5FF_35BC),
    (0.2802, 0xB0FF_2990),
    (0.3003, 0x8ACC_2071),
    (0.3203, 0x81BF_1E69),
    (0.3403, 0x80BE_1E69),
    (0.3603, 0x80BE_1E69),
    (0.3803, 0x80BE_1E69),
    (0.4011, 0x80BE_1E69),
    (0.4204, 0x80BE_1E69),
    (0.4404, 0x80BE_1E69),
    (0.4607, 0x80BE_1E69),
    (0.4804, 0x80BE_1E69),
    (0.5000, 0x80BE_1E69),
    (0.5000, 0x80BE_1E69),
];

/// The same after use, from the run-on past the hold.
const FADE_OUT: [(f32, u32); 24] = [
    (0.5402, 0x80BE_1E69),
    (0.5602, 0x80BE_1E69),
    (0.5808, 0x80BE_1E69),
    (0.6002, 0x80BE_1E69),
    (0.6204, 0x80BE_1E69),
    (0.6407, 0x80BE_1E69),
    (0.6603, 0x80BE_1E69),
    (0.6803, 0x81BF_1E69),
    (0.7003, 0x8ACD_2071),
    (0.7204, 0xB2FF_2992),
    (0.7404, 0xE6FF_36BD),
    (0.7603, 0xDCFF_33B5),
    (0.7804, 0x96DF_237B),
    (0.8005, 0x608F_164F),
    (0.8204, 0x496C_113C),
    (0.8407, 0x3B58_0D30),
    (0.8612, 0x2F46_0B26),
    (0.8806, 0x2436_081D),
    (0.9006, 0x1A26_0615),
    (0.9209, 0x1019_030D),
    (0.9407, 0x090E_0207),
    (0.9608, 0x0406_0103),
    (0.9807, 0x0101_0000),
    (1.0000, 0x0000_0000),
];

/// Whether some `T` within the table's four-decimal rounding of `printed` makes `word`.
fn reproduces(printed: f32, word: u32, backdrop: impl Fn(f32) -> f32) -> bool {
    (-50..=50).any(|n| scale_word(MISSILE, backdrop(printed + n as f32 * 1.0e-6)) == word)
}

#[test]
fn the_29_live_fade_in_words_come_from_the_eased_overshoot() {
    for (t, word) in FADE_IN {
        assert!(
            reproduces(t, word, |t| ease_with_bump(3.0 * t, 1.0)),
            "T {t}: {word:#010x}, law gives {:#010x}",
            scale_word(MISSILE, ease_with_bump(3.0 * t, 1.0))
        );
    }
}

#[test]
fn the_fade_out_words_replay_the_curve_backwards() {
    for (t, word) in FADE_OUT {
        assert!(
            reproduces(t, word, |t| ease_with_bump((2.0 - 2.0 * t) * 1.5, 1.0)),
            "T {t}: {word:#010x}"
        );
    }
}

#[test]
fn a_linear_clamp_cannot_make_the_overshoot() {
    let (t, word) = FADE_IN[13];
    let clamp = scale_word(MISSILE, (3.0_f32 * t).clamp(0.0, 1.0));
    assert!(
        clamp >> 24 <= 0x80,
        "a clamp never passes the table's alpha"
    );
    assert_ne!(clamp, word);
    assert_eq!(word >> 24, 0xE5, "alpha overshoots 0x80 to 0xE5");
}

/// The size words of the same run: `+0x9c` is `66 / f` and `+0xa0` is `60 f`, with
/// `f = ease(2T * 0.99 + 0.01)`.
#[test]
fn the_size_pulse_is_width_over_f_and_height_times_f() {
    const SIZES: [(f32, f32, f32); 6] = [
        (0.0000, 267_495.16, 0.015),
        (0.1001, 639.15, 6.196),
        (0.2002, 185.88, 21.304),
        (0.3003, 99.82, 39.673),
        (0.4011, 72.66, 54.502),
        (0.5000, 66.0, 60.0),
    ];
    for (t, width, height) in SIZES {
        let f = ease(2.0 * t * 0.99 + 0.01);
        let near = |got: f32, want: f32| (got - want).abs() <= (want * 0.01).max(0.001);
        assert!(near(66.0 / f, width), "T {t}: width {}", 66.0 / f);
        assert!(near(60.0 * f, height), "T {t}: height {}", 60.0 * f);
    }
}

fn run(fade: &mut Fade, held: Option<Weapon>, seconds: f32) {
    let ticks = (seconds * 60.0).round() as u32;
    for _ in 0..ticks {
        fade.advance(held, 1.2, 1.0 / 60.0);
    }
}

#[test]
fn a_grant_grows_holds_after_use_then_fades_out_and_clears() {
    let mut fade = Fade::default();
    assert!(fade.look(None).is_none());
    fade.advance(Some(Weapon::Missile), 1.2, 1.0 / 60.0);
    let first = fade.look(Some(Weapon::Missile)).unwrap();
    assert!(first.backdrop < 0.05 && first.size < 0.01 && first.icon == 0.0);
    run(&mut fade, Some(Weapon::Missile), 0.2);
    let peak = fade.look(Some(Weapon::Missile)).unwrap();
    assert!(peak.backdrop > 1.5, "overshoot {}", peak.backdrop);
    assert_eq!(peak.icon, 0.0, "the icon waits for T >= 0.4");
    run(&mut fade, Some(Weapon::Missile), 0.6);
    let grown = fade.look(Some(Weapon::Missile)).unwrap();
    assert_eq!((grown.backdrop, grown.size, grown.icon), (1.0, 1.0, 1.0));

    run(&mut fade, None, 0.2);
    let held = fade.look(None).unwrap();
    assert_eq!(
        held.weapon,
        Weapon::Missile,
        "the last weapon stays up after use"
    );
    assert_eq!(held.backdrop, 1.0, "the backdrop holds at full strength");
    run(&mut fade, None, 0.3);
    let fading = fade.look(None).unwrap();
    assert!(
        fading.backdrop != 1.0 || fading.size < 1.0,
        "the reverse curve is running"
    );
    run(&mut fade, None, 0.2);
    assert!(fade.look(None).unwrap().backdrop < 0.3);
    run(&mut fade, None, 0.2);
    assert!(fade.look(None).is_none(), "cleared once T has run out");
}

#[test]
fn a_bare_switch_of_weapon_does_not_replay_the_fade_in() {
    let mut fade = Fade::default();
    run(&mut fade, Some(Weapon::Missile), 1.0);
    fade.advance(Some(Weapon::Turbo), 1.2, 1.0 / 60.0);
    let look = fade.look(Some(Weapon::Turbo)).unwrap();
    assert_eq!((look.weapon, look.backdrop), (Weapon::Turbo, 1.0));
}

const LAYOUT: &str = r#"
<Screen>
<Image name="PickupBackground">
<Values x="240" y="35" Centred="true" width="66" height="60" U="3" V="111" TxtrWidth="66" TxtrHeight="60" Color="FEConst->HudColour1" Src="Data\HUD\Textures\PulseHUD.mip"></Values>
</Image>
<Image name="MissileIcon">
<Values x="240" y="35" Centred="true" width="48" height="48" U="70" V="111" TxtrWidth="48" TxtrHeight="48" Color="FEConst->HudColour1" Src="Data\HUD\Textures\PulseHUD.mip"></Values>
</Image>
</Screen>
"#;

/// The wire: the sprites `pickup_sprites` hands the draw carry the fade, not the settled
/// table word. Fails if the look is dropped on the way (the colour stays `0x80BE1E69`, the rect
/// stays 66 x 60, the icon stays white).
#[test]
fn pulse_draws_the_fade_in_the_backdrop_word_the_size_and_the_icon() {
    use super::pickup::pickup_sprites;
    use oag_ui::screen::argb_to_rgba;

    let layout = super::Layout::from_xml(LAYOUT);
    let art = oag_pulse::hud::ART;
    let at = |ticks: u32| {
        let mut fade = Fade::default();
        for _ in 0..ticks {
            fade.advance(Some(Weapon::Missile), 1.2, 1.0 / 60.0);
        }
        let look = fade.look(Some(Weapon::Missile)).unwrap();
        (
            look,
            pickup_sprites(
                &layout,
                Weapon::Missile,
                &super::ext_draws::Outputs::default(),
                art,
                false,
                Some(look),
            ),
        )
    };

    let (look, drawn) = at(13);
    let backdrop = &drawn[0];
    assert_eq!(backdrop.name, "PickupBackground");
    assert_eq!(
        backdrop.color,
        argb_to_rgba(scale_word(MISSILE, look.backdrop))
    );
    assert!(
        backdrop.color[3] > 0.8,
        "the overshoot brightens past the table's 0x80"
    );
    assert!((backdrop.rect[2] - 66.0 / look.size).abs() < 1.0e-3);
    assert!((backdrop.rect[3] - 60.0 * look.size).abs() < 1.0e-3);
    let centre = (
        backdrop.rect[0] + backdrop.rect[2] * 0.5,
        backdrop.rect[1] + backdrop.rect[3] * 0.5,
    );
    assert!((centre.0 - 240.0).abs() < 1.0e-2 && (centre.1 - 35.0).abs() < 1.0e-2);
    assert_eq!(drawn[1].color[3], 0.0, "the icon waits for T >= 0.4");

    let (_, grown) = at(40);
    assert_eq!(grown[0].color, argb_to_rgba(MISSILE));
    assert_eq!(grown[0].rect[2..], [66.0, 60.0]);
    assert_eq!(
        grown[1].color,
        [1.0, 1.0, 1.0, 1.0],
        "the icon is white once grown"
    );
}
