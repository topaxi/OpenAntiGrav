//! How a simulation mod's own pickup (`Weapon::Extension`) resolves to what is drawn: the
//! look it declared names an icon and a backdrop colour, each from the title or the mod.
//! Pulse's sprites, HD's and Omega's backdrop words, Pure's models, 2048's UV rewrite.

use oag_tables::pickup_look::{Colour, Icon, PickupLook};
use oag_tables::weapons::Weapon;
use oag_ui::screen::argb_to_rgba;

use super::draw::*;
use super::ext_draws::Outputs;
use super::pickup::*;
use super::*;

const LAYOUT: &str = r#"
<Screen>
<Image name="PickupBackground">
<Values x="240" y="35" Centred="true" width="66" height="60" U="3" V="111" TxtrWidth="66" TxtrHeight="60" Color="FEConst->HudColour1" Src="Data\HUD\Textures\PulseHUD.mip"></Values>
</Image>
<Image name="TurboIcon">
<Values x="240" y="35" Centred="true" width="48" height="48" U="70" V="111" TxtrWidth="48" TxtrHeight="48" Color="FEConst->HudColour1" Src="Data\HUD\Textures\PulseHUD.mip"></Values>
</Image>
<Image name="ShieldIcon">
<Values x="240" y="35" Centred="true" width="48" height="48" U="120" V="111" TxtrWidth="48" TxtrHeight="48" Color="FEConst->HudColour1" Src="Data\HUD\Textures\PulseHUD.mip"></Values>
</Image>
</Screen>
"#;

fn with(icon: Icon, colour: Colour) -> Outputs {
    Outputs {
        pickup_look: Some(PickupLook { icon, colour }),
        ..Outputs::default()
    }
}

fn names(sprites: &[Sprite]) -> Vec<&str> {
    sprites.iter().map(|s| s.name.as_str()).collect()
}

const RED_RGBA: [u8; 4] = [0xE0, 0x20, 0x20, 0x80];
const RED_WORD: u32 = 0x80E0_2020;

#[test]
fn pulse_takes_the_titles_own_row_for_a_title_colour_and_its_own_sprite_for_a_title_icon() {
    let layout = Layout::from_xml(LAYOUT);
    let ext = with(Icon::Title(Weapon::Turbo), Colour::Title(Weapon::Mine));
    let drawn = pickup_sprites(
        &layout,
        Weapon::Extension,
        &ext,
        oag_pulse::hud::ART,
        false,
        None,
    );
    assert_eq!(names(&drawn), ["PickupBackground", "TurboIcon"]);
    assert_eq!(
        drawn[0].color,
        argb_to_rgba(oag_pulse::hud::PICKUP_COLOURS[Weapon::Mine as usize].unwrap()),
        "Mine's row (blue), not Turbo's (green)"
    );
}

#[test]
fn a_mod_rgba_is_the_backdrop_word_as_given_and_the_title_fade_plays_on_it() {
    let layout = Layout::from_xml(LAYOUT);
    let ext = with(Icon::Title(Weapon::Turbo), Colour::Rgba(RED_RGBA));
    let settled = pickup_sprites(
        &layout,
        Weapon::Extension,
        &ext,
        oag_pulse::hud::ART,
        false,
        None,
    );
    assert_eq!(settled[0].color, argb_to_rgba(RED_WORD));
    assert!(settled[0].color[0] > settled[0].color[2], "red, not blue");
    assert!(settled[0].color[3] < 0.6, "translucent");

    let mut fade = pickup_fade::Fade::default();
    for _ in 0..13 {
        fade.advance(Some(Weapon::Extension), 1.2, 1.0 / 60.0);
    }
    let look = fade.look(Some(Weapon::Extension)).unwrap();
    let fading = pickup_sprites(
        &layout,
        Weapon::Extension,
        &ext,
        oag_pulse::hud::ART,
        false,
        Some(look),
    );
    assert_eq!(
        fading[0].color,
        argb_to_rgba(pickup_fade::scale_word(RED_WORD, look.backdrop)),
        "the overshoot scales the mod's colour as it scales a weapon's"
    );
    assert!((fading[0].rect[3] - 60.0 * look.size).abs() < 1.0e-3);
}

#[test]
fn a_mod_pickup_with_no_look_draws_nothing_and_never_borrows_an_icon() {
    let layout = Layout::from_xml(LAYOUT);
    let drawn = pickup_sprites(
        &layout,
        Weapon::Extension,
        &Outputs::default(),
        oag_pulse::hud::ART,
        false,
        None,
    );
    assert!(drawn.is_empty(), "{:?}", names(&drawn));
}

#[test]
fn a_mod_icon_draws_the_backdrop_and_no_title_icon() {
    let layout = Layout::from_xml(LAYOUT);
    let ext = with(Icon::Mod("mod:test/bolt".into()), Colour::Rgba(RED_RGBA));
    let drawn = pickup_sprites(
        &layout,
        Weapon::Extension,
        &ext,
        oag_pulse::hud::ART,
        false,
        None,
    );
    assert_eq!(names(&drawn), ["PickupBackground"]);

    let sheet = crate::sprite::Sheet::placed_at(&[(
        "mod:test/bolt",
        crate::sprite::Placed {
            x: 0,
            y: 0,
            width: 32,
            height: 16,
            quad_extent: None,
            quad_centre: None,
            blend: None,
        },
    )]);
    let strings = oag_ui::language::StringTable::default();
    let cx = Context {
        default_border: layout.default_border(),
        layout: &layout,
        strings: &strings,
        sheet: &sheet,
        art: oag_pulse::hud::ART,
        hud_line_height: 25.0,
        small_line_height: 10.0,
        default_line_height: 10.0,
        skin_line_height: 10.0,
    };
    let shown = shown(Weapon::Extension, &ext);
    assert_eq!(shown.mod_icon, Some("mod:test/bolt"));
    let Some(Draw::Sprite { rect, .. }) = mod_icon_draw(&cx, "mod:test/bolt", None) else {
        panic!("the mod's icon draws as a sprite");
    };
    assert_eq!(rect[2] / rect[3], 2.0, "its own aspect");
    assert!(rect[2] <= 48.0 && rect[3] <= 48.0, "inside the icon slot");
    assert_eq!(
        (rect[0] + rect[2] * 0.5, rect[1] + rect[3] * 0.5),
        (240.0, 35.0),
        "centred where the title's icon sits"
    );
    assert!(mod_icon_draw(&cx, "mod:test/missing", None).is_none());
}

#[test]
fn hd_and_omega_resolve_a_title_colour_through_their_own_tables_and_take_a_mod_rgba() {
    for (art, table) in [
        (oag_hd::hud::ART, oag_hd::hud::PICKUP_COLOURS),
        (oag_omega::hud::ART, oag_omega::hud::PICKUP_COLOURS),
    ] {
        let by_title = with(Icon::Title(Weapon::Turbo), Colour::Title(Weapon::Mine));
        assert_eq!(
            backdrop_word(art, Weapon::Extension, &by_title),
            table[Weapon::Mine as usize]
        );
        assert!(table[Weapon::Mine as usize].is_some());
        let by_mod = with(Icon::Title(Weapon::Turbo), Colour::Rgba(RED_RGBA));
        assert_eq!(
            backdrop_word(art, Weapon::Extension, &by_mod),
            Some(RED_WORD)
        );
        assert_eq!(
            backdrop_word(art, Weapon::Extension, &Outputs::default()),
            None
        );
        assert_eq!(
            backdrop_word(art, Weapon::Mine, &Outputs::default()),
            table[Weapon::Mine as usize],
            "a weapon of the title's is unchanged"
        );
    }
}

#[test]
fn pure_has_no_backdrop_colours_so_both_colour_kinds_draw_nothing_there() {
    for colour in [Colour::Title(Weapon::Mine), Colour::Rgba(RED_RGBA)] {
        let ext = with(Icon::Title(Weapon::Turbo), colour);
        assert_eq!(
            backdrop_word(oag_pure::hud::ART, Weapon::Extension, &ext),
            None
        );
    }
}

#[test]
fn a_mod_pickup_on_2048_never_indexes_past_its_tables() {
    let layout = Layout::from_xml(LAYOUT);
    for ext in [
        Outputs::default(),
        with(
            Icon::Title(Weapon::Extension),
            Colour::Title(Weapon::Extension),
        ),
        with(Icon::Mod("mod:x/y".into()), Colour::Rgba(RED_RGBA)),
    ] {
        for announcing in [false, true] {
            let _ = pickup_sprites(
                &layout,
                Weapon::Extension,
                &ext,
                oag_2048::hud::ART,
                announcing,
                None,
            );
        }
    }
}

#[test]
fn hds_runtime_backdrop_takes_a_mod_pickups_look_through_the_same_fade() {
    let layout = Layout::from_xml(LAYOUT);
    let sheet = crate::sprite::Sheet::default();
    let strings = oag_ui::language::StringTable::default();
    let cx = Context {
        default_border: layout.default_border(),
        layout: &layout,
        strings: &strings,
        sheet: &sheet,
        art: oag_hd::hud::ART,
        hud_line_height: 25.0,
        small_line_height: 10.0,
        default_line_height: 10.0,
        skin_line_height: 10.0,
    };
    let mut fade = pickup_fade::Fade::default();
    for _ in 0..60 {
        fade.advance(Some(Weapon::Extension), 1.5, 1.0 / 60.0);
    }
    let readout = |ext: Outputs| Readout {
        pickup: Some(Weapon::Extension),
        pickup_fade: fade,
        ext,
        ..Readout::default()
    };
    let idle = oag_hd::hud::ART.runtime.unwrap().pickup_idle.unwrap();
    let by_title = readout(with(
        Icon::Title(Weapon::Turbo),
        Colour::Title(Weapon::Mine),
    ));
    assert_eq!(
        super::runtime::colour(&cx, &by_title, idle.backdrop),
        Some(argb_to_rgba(
            oag_hd::hud::PICKUP_COLOURS[Weapon::Mine as usize].unwrap()
        ))
    );
    let by_mod = readout(with(Icon::Title(Weapon::Turbo), Colour::Rgba(RED_RGBA)));
    assert_eq!(
        super::runtime::colour(&cx, &by_mod, idle.backdrop),
        Some(argb_to_rgba(RED_WORD))
    );
}
