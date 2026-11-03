//! How a simulation mod's own pickup (`Weapon::Extension(ModKind::FIRST)`) resolves to what is drawn: the
//! look it declared names an icon and a backdrop colour, each from the title or the mod.
//! Pulse's sprites, HD's and Omega's backdrop words, Pure's models, 2048's UV rewrite.

use oag_tables::pickup_look::{Colour, Icon, PickupLook};
use oag_tables::weapons::{ModKind, Weapon};
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
        Weapon::Extension(ModKind::FIRST),
        &ext,
        oag_pulse::hud::ART,
        false,
        None,
    );
    assert_eq!(names(&drawn), ["PickupBackground", "TurboIcon"]);
    assert_eq!(
        drawn[0].color,
        argb_to_rgba(oag_pulse::hud::PICKUP_COLOURS[Weapon::Mine.index()].unwrap()),
        "Mine's row (blue), not Turbo's (green)"
    );
}

#[test]
fn a_mod_rgba_is_the_backdrop_word_as_given_and_the_title_fade_plays_on_it() {
    let layout = Layout::from_xml(LAYOUT);
    let ext = with(Icon::Title(Weapon::Turbo), Colour::Rgba(RED_RGBA));
    let settled = pickup_sprites(
        &layout,
        Weapon::Extension(ModKind::FIRST),
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
        fade.advance(Some(Weapon::Extension(ModKind::FIRST)), 1.2, 1.0 / 60.0);
    }
    let look = fade.look(Some(Weapon::Extension(ModKind::FIRST))).unwrap();
    let fading = pickup_sprites(
        &layout,
        Weapon::Extension(ModKind::FIRST),
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
        Weapon::Extension(ModKind::FIRST),
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
        Weapon::Extension(ModKind::FIRST),
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
    let shown = shown(Weapon::Extension(ModKind::FIRST), &ext);
    assert_eq!(shown.mod_icon, Some("mod:test/bolt"));
    let Some(Draw::Sprite { rect, .. }) = mod_icon_draw(&cx, "mod:test/bolt", None) else {
        panic!("the mod's icon draws as a sprite");
    };
    assert_eq!(rect[2] / rect[3], 2.0, "its own aspect");
    assert_eq!((rect[2], rect[3]), (32.0, 16.0), "at its own size");
    assert_eq!(
        (rect[0] + rect[2] * 0.5, rect[1] + rect[3] * 0.5),
        (240.0, 35.0),
        "centred on the backdrop frame"
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
            backdrop_word(art, Weapon::Extension(ModKind::FIRST), &by_title),
            table[Weapon::Mine.index()]
        );
        assert!(table[Weapon::Mine.index()].is_some());
        let by_mod = with(Icon::Title(Weapon::Turbo), Colour::Rgba(RED_RGBA));
        assert_eq!(
            backdrop_word(art, Weapon::Extension(ModKind::FIRST), &by_mod),
            Some(RED_WORD)
        );
        assert_eq!(
            backdrop_word(art, Weapon::Extension(ModKind::FIRST), &Outputs::default()),
            None
        );
        assert_eq!(
            backdrop_word(art, Weapon::Mine, &Outputs::default()),
            table[Weapon::Mine.index()],
            "a weapon of the title's is unchanged"
        );
    }
}

#[test]
fn pure_has_no_backdrop_colours_so_both_colour_kinds_draw_nothing_there() {
    for colour in [Colour::Title(Weapon::Mine), Colour::Rgba(RED_RGBA)] {
        let ext = with(Icon::Title(Weapon::Turbo), colour);
        assert_eq!(
            backdrop_word(oag_pure::hud::ART, Weapon::Extension(ModKind::FIRST), &ext),
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
            Icon::Title(Weapon::Extension(ModKind::FIRST)),
            Colour::Title(Weapon::Extension(ModKind::FIRST)),
        ),
        with(Icon::Mod("mod:x/y".into()), Colour::Rgba(RED_RGBA)),
    ] {
        for announcing in [false, true] {
            let _ = pickup_sprites(
                &layout,
                Weapon::Extension(ModKind::FIRST),
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
        fade.advance(Some(Weapon::Extension(ModKind::FIRST)), 1.5, 1.0 / 60.0);
    }
    let readout = |ext: Outputs| Readout {
        pickup: Some(Weapon::Extension(ModKind::FIRST)),
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
            oag_hd::hud::PICKUP_COLOURS[Weapon::Mine.index()].unwrap()
        ))
    );
    let by_mod = readout(with(Icon::Title(Weapon::Turbo), Colour::Rgba(RED_RGBA)));
    assert_eq!(
        super::runtime::colour(&cx, &by_mod, idle.backdrop),
        Some(argb_to_rgba(RED_WORD))
    );
}

/// The icon's art is what is drawn at native size and centred on the frame, not its canvas:
/// the transparent margin of an icon authored on the title's own texture cell does not shrink
/// the glyph, and an off-centre glyph is moved to the middle of the frame.
#[test]
fn a_padded_icon_draws_its_art_at_native_size_on_the_frame_centre_and_a_big_one_shrinks() {
    let layout = Layout::from_xml(LAYOUT);
    let strings = oag_ui::language::StringTable::default();
    let placed = |width, height, art: [f32; 2], centre: [f32; 2]| {
        crate::sprite::Sheet::placed_at(&[(
            "mod:test/glyph",
            crate::sprite::Placed {
                x: 0,
                y: 0,
                width,
                height,
                quad_extent: Some(art),
                quad_centre: Some(centre),
                blend: None,
            },
        )])
    };
    let draw = |sheet: &crate::sprite::Sheet| {
        let cx = Context {
            default_border: layout.default_border(),
            layout: &layout,
            strings: &strings,
            sheet,
            art: oag_pulse::hud::ART,
            hud_line_height: 25.0,
            small_line_height: 10.0,
            default_line_height: 10.0,
            skin_line_height: 10.0,
        };
        let Some(Draw::Sprite { rect, .. }) = mod_icon_draw(&cx, "mod:test/glyph", None) else {
            panic!("draws");
        };
        rect
    };
    // 66 by 60 canvas, a 22 by 27 glyph 3 pixels left of its middle and 2 above it.
    let rect = draw(&placed(66, 60, [22.0, 27.0], [-3.0, -2.0]));
    assert_eq!(
        (rect[2], rect[3]),
        (66.0, 60.0),
        "the canvas at native size"
    );
    let art_centre = (rect[0] + 33.0 - 3.0, rect[1] + 30.0 - 2.0);
    assert_eq!(
        art_centre,
        (240.0, 35.0),
        "the glyph, not the canvas, is centred"
    );
    // A 132 by 120 glyph is wider than the 66 by 60 frame: halved, aspect kept.
    let rect = draw(&placed(132, 120, [132.0, 120.0], [0.0, 0.0]));
    assert_eq!((rect[2], rect[3]), (66.0, 60.0));
}

/// A mod's icon takes the box the title's own Turbo glyph fills, read off the sheet's pixels,
/// and sits where that glyph sits, however the PNG's canvas is padded.
#[test]
fn a_mod_icon_fills_the_box_the_titles_own_turbo_glyph_fills() {
    let layout = Layout::from_xml(LAYOUT);
    let strings = oag_ui::language::StringTable::default();
    // The Turbo region is (70, 111) 48 by 48 of the HUD texture; its glyph is 20 by 27 texels,
    // opaque, 14 across and 5 down in it.
    let (width, height) = (128_u32, 160_u32);
    let mut hud = vec![0_u8; (width * height * 4) as usize];
    for y in 111 + 5..111 + 5 + 27 {
        for x in 70 + 14..70 + 14 + 20 {
            hud[((y * width + x) * 4 + 3) as usize] = 255;
        }
    }
    let image = |src: &str, width, height, rgba| crate::sprite::DecodedImage {
        src: src.into(),
        width,
        height,
        rgba,
        quad_extent: (src == "mod:test/glyph").then_some([10.0, 10.0]),
        quad_centre: (src == "mod:test/glyph").then_some([0.0, 0.0]),
        blend: None,
    };
    let sheet = crate::sprite::Sheet::build_with(
        &[],
        vec![
            image("Data\\HUD\\Textures\\PulseHUD.mip", width, height, hud),
            image("mod:test/glyph", 16, 16, vec![255; 16 * 16 * 4]),
        ],
        &mut Vec::new(),
    );
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
    let Some(Draw::Sprite { rect, .. }) = mod_icon_draw(&cx, "mod:test/glyph", None) else {
        panic!("draws");
    };
    // 10 by 10 art into 20 by 27: doubled, aspect kept; centred on the glyph (240, 29.5).
    assert_eq!(rect, [224.0, 13.5, 32.0, 32.0]);
}

/// HD and Omega keep the frame always up, and the mod icon sits in its middle.
#[test]
fn on_hd_and_omega_the_mod_icon_is_centred_on_the_always_up_frame() {
    let layout = Layout::from_xml(LAYOUT);
    let strings = oag_ui::language::StringTable::default();
    // The Turbo region is (70, 111) 48 by 48 of the HUD texture; its glyph is 20 by 27 texels,
    // opaque, 14 across and 5 down in it.
    let (width, height) = (128_u32, 160_u32);
    let mut hud = vec![0_u8; (width * height * 4) as usize];
    for y in 111 + 5..111 + 5 + 27 {
        for x in 70 + 14..70 + 14 + 20 {
            hud[((y * width + x) * 4 + 3) as usize] = 255;
        }
    }
    let image = |src: &str, width, height, rgba| crate::sprite::DecodedImage {
        src: src.into(),
        width,
        height,
        rgba,
        quad_extent: (src == "mod:test/glyph").then_some([10.0, 10.0]),
        quad_centre: (src == "mod:test/glyph").then_some([0.0, 0.0]),
        blend: None,
    };
    let sheet = crate::sprite::Sheet::build_with(
        &[],
        vec![
            image("Data\\HUD\\Textures\\PulseHUD.mip", width, height, hud),
            image("mod:test/glyph", 16, 16, vec![255; 16 * 16 * 4]),
        ],
        &mut Vec::new(),
    );
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
    let Some(Draw::Sprite { rect, .. }) = mod_icon_draw(&cx, "mod:test/glyph", None) else {
        panic!("draws");
    };
    // 10 by 10 art into 20 by 27: doubled, aspect kept; centred on the glyph (240, 29.5).
    assert_eq!(
        rect,
        [224.0, 19.0, 32.0, 32.0],
        "size from the glyph box, centre on the frame (240, 35)"
    );
}
