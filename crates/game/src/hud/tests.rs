//! What the in-race HUD in [`super`] is asserted to do: reading the disc's own
//! layout XML, where each widget lands, the colours it resolves through
//! `FEConst`, the lap-time and bar formatting, and what a readout draws.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `hud.rs`: the tests are 862 lines, past the 200 an inline test
//! module may hold. See `scripts/check-file-size.py`, which is the rule as a
//! gate.

use super::*;
// The draw list moved to `super::draw` when `hud.rs` was split; its decisions
// are `pub(super)`, so this reaches them by name rather than through the
// parent's own imports.
use super::draw::*;

/// The shape of the real `TimeTrial_HUD.xml`, cut to what this module reads.
/// Every attribute spelling, the `FEConst->` indirection, the `<Item>`
/// offsets and the mixed `centre`/`middle` vertical alignment are as they
/// appear on the disc.
const SAMPLE: &str = r#"
<Screen>
<Screen name="HUD">
<Variable global="HudColour1"><Values String="0xFFFFFFFF"></Values></Variable>
<Variable global="HudColour2"><Values String="0xFF7DEFC0"></Values></Variable>
<Variable global="HudColour3A"><Values String="0x60B5D7C8"></Values></Variable>
<Variable global="HudBGColour"><Values String="0x40000000"></Values></Variable>

<Mode3D>
<Values FirstPass="yes" OriginX="0.0" OriginY="35.0"></Values>
<Model name="ReadyGo" Enabletransition="0" Delay="0">
<Values Src="Data\HUD\Pulse_Ready_Go.vex" x="0.0" y="0.0" z="-70.0" ztest="0"></Values>
</Model>
</Mode3D>

<Item OffsetX="300" OffsetY="210">
<Image name="SpeedBarBg">
<Values x="6" y="10" width="168" height="26" U="6" V="0" TxtrWidth="168" TxtrHeight="26" CalcBlur="1" Color="FEConst->HudColour3A" Src="Data\HUD\Textures\PulseHUD.mip"></Values>
</Image>
<Text name="SpeedBarText">
<Values string="0 kmh" font="HUDSmall" align="right" scale="1.0" x="0" y="0" CalcBlur="1" Color="FEConst->HudColour1" BorderColor="FEConst->HudBGColour"></Values>
</Text>
</Item>

<Item OffsetX="5" OffsetY="5">
<Text name="LapTxt">
<Values idstring="IG_HUD_LAP" font="HUDSmall" scale="1.0" x="4" y="0" CalcBlur="1" Color="FEConst->HudColour2"></Values>
</Text>
<Text name="Lap">
<Values scale="1.0" font="HUD" align="left" vertalign="bottom" x="3" y="32" CalcBlur="1"/>
</Text>
</Item>

<Image name="PickupBackground">
<Values x="240" y="35" Centred="true" width="66" height="60" U="3" V="111" TxtrWidth="66" TxtrHeight="60" CalcBlur="1" Color="FEConst->HudColour1" Src="Data\HUD\Textures\PulseHUD.mip"></Values>
</Image>
<Image name="TurboIcon">
<Values x="240" y="35" Centred="true" width="48" height="48" U="70" V="111" TxtrWidth="48" TxtrHeight="48" Color="FEConst->HudColour1" Src="Data\HUD\Textures\PulseHUD.mip"></Values>
</Image>
<Image name="RocketIcon">
<Values x="240" y="35" Centred="true" width="48" height="48" U="120" V="111" TxtrWidth="48" TxtrHeight="48" Color="FEConst->HudColour1" Src="Data\HUD\Textures\PulseHUD.mip"></Values>
</Image>
<Text name="TimeDiffText">
<Values font="HUD" scale="0.6" align="centre" vertalign="middle" x="0" y="5"/>
</Text>
</Screen>
</Screen>
"#;

/// The top-right corner of the real `Arcade_HUD.xml`, verbatim.
///
/// Six widgets in one `<Item>`, and the point of the fixture is the two
/// **coincident** anchors it contains - `TotalTime` and `Position` share `x`,
/// `y`, font, scale and both alignments. Copied off the USA PSP disc with
/// `just wad cat --expand`, attribute order included, so a reader can check it
/// against the file rather than against this comment. See
/// [`place_owns_the_anchor`].
const TOP_RIGHT: &str = r#"
<Screen>
<Screen name="HUD">
<Variable global="HudColour2"><Values String="0xFF7DEFC0"></Values></Variable>
<Variable global="HudBGColour"><Values String="0x40000000"></Values></Variable>
<Item OffsetX="445" OffsetY="5">
<Text name="TotalTimeTxt">
<Values idstring="IG_HUD_TOTAL" font="HUDSmall" align="right" scale="1.0" x="0" y="0" CalcBlur="1" Color="FEConst->HudColour2"></Values>
</Text>
<Text name="TotalTime">
<Values scale="1.0" font="HUD" align="right" vertalign="bottom" x="0" y="30" CalcBlur="1"/>
</Text>
<Text name="PositionTxt">
<Values idstring="IG_HUD_POS" font="HUDSmall" align="right" scale="1.0" x="15" y="0" CalcBlur="1" Color="FEConst->HudColour2" BorderColor="FEConst->HudBGColour"></Values>
</Text>
<Text name="Position">
<Values scale="1.0" font="HUD" align="right" vertalign="bottom" x="0" y="30" CalcBlur="1" BorderColor="FEConst->HudBGColour"/>
</Text><Text name="PositionOf">
<Values string="/" scale="0.6" font="HUD" align="left" vertalign="bottom" x="3" y="31" CalcBlur="1" Color="FEConst->HudColour2" BorderColor="FEConst->HudBGColour"/>
</Text>
<Text name="Position Outof">
<Values scale="0.6" font="HUD" align="left" vertalign="bottom" x="16" y="34" CalcBlur="1" BorderColor="FEConst->HudBGColour"/>
</Text>
</Item>
</Screen>
</Screen>
"#;

/// The readout a racer in a field has: third of eight.
fn placed() -> Readout {
    Readout {
        place: 3,
        ships: 8,
        ..Readout::blank()
    }
}

/// The whole point of the widget group: a place the player can read.
#[test]
fn the_place_and_the_field_size_are_drawn_with_their_separator() {
    let layout = Layout::from_xml(TOP_RIGHT);
    let strings = strings();
    let frame = draw_list(&context(&layout, &strings), &placed());

    let texts: Vec<&str> = frame
        .hud_text
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(texts, ["3", "/", "8"], "{frame:?}");
}

/// The field size is never drawn on its own.
///
/// **This was live, not hypothetical**: `Position Outof` gated on `ships > 0`
/// alone, and the field size is known from the moment a race is set up, so a
/// single race drew the `POS` caption and a bare `8` with no number and no
/// separator.
#[test]
fn the_field_size_is_not_drawn_without_a_place() {
    let layout = Layout::from_xml(TOP_RIGHT);
    let strings = strings();
    let readout = Readout {
        place: 0,
        ships: 8,
        ..Readout::blank()
    };
    let frame = draw_list(&context(&layout, &strings), &readout);

    for draw in frame.hud_text.iter().chain(frame.small_text.iter()) {
        let Draw::Text { text, .. } = draw else {
            continue;
        };
        assert_ne!(
            text, "8",
            "the field size is drawn with no place: {frame:?}"
        );
        assert_ne!(text, "/", "the separator is drawn with no place: {frame:?}");
    }
}

/// A caption with nothing under it is a rendering fault, so the `POS` caption
/// goes with its number.
///
/// The reachable case: a single race on a track with no authored `Start
/// Position` grids one craft, which has no place, and the arcade layout still
/// carries the widgets.
#[test]
fn the_place_caption_is_not_drawn_on_its_own() {
    let layout = Layout::from_xml(TOP_RIGHT);
    let strings = strings();
    let readout = Readout {
        place: 0,
        ships: 1,
        ..Readout::blank()
    };
    let frame = draw_list(&context(&layout, &strings), &readout);

    let captions: Vec<&str> = frame
        .small_text
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    // `strings()` is empty, so `get_or_id` falls back to the key itself, which
    // is what makes the two captions distinguishable here.
    assert_eq!(captions, ["IG_HUD_TOTAL"], "{frame:?}");
}

/// The two coincident widgets are never both live. See
/// [`place_owns_the_anchor`].
#[test]
fn the_total_time_yields_its_anchor_to_the_place() {
    let layout = Layout::from_xml(TOP_RIGHT);
    let strings = strings();

    // The anchor the two share, which is what makes this exclusive rather than
    // a preference: read off the fixture rather than asserted from memory.
    let total = layout.label("TotalTime").expect("the fixture has one");
    let position = layout.label("Position").expect("the fixture has one");
    assert_eq!((total.x, total.y), (position.x, position.y));
    assert_eq!(total.scale, position.scale);
    assert_eq!(total.align, position.align);
    assert_eq!(total.vertalign, position.vertalign);
    assert_eq!(total.font, position.font);

    let names = |readout: &Readout| -> Vec<String> {
        layout
            .labels
            .iter()
            .filter(|label| {
                text_for(
                    label,
                    readout,
                    &strings,
                    place_owns_the_anchor(&layout, readout),
                    true,
                    None,
                )
                .is_some()
            })
            .map(|label| label.name.clone())
            .collect()
    };

    // In a field: the place and its caption, and no clock.
    let racing = names(&placed());
    assert!(racing.contains(&"Position".to_string()), "{racing:?}");
    assert!(racing.contains(&"PositionTxt".to_string()), "{racing:?}");
    assert!(!racing.contains(&"TotalTime".to_string()), "{racing:?}");
    assert!(!racing.contains(&"TotalTimeTxt".to_string()), "{racing:?}");

    // Alone: the clock and its caption, and no place.
    let solo = names(&Readout::blank());
    assert!(solo.contains(&"TotalTime".to_string()), "{solo:?}");
    assert!(solo.contains(&"TotalTimeTxt".to_string()), "{solo:?}");
    assert!(!solo.contains(&"Position".to_string()), "{solo:?}");
}

/// A layout with a clock and no place keeps its clock in a race with a field.
///
/// `Elimination_HUD.xml` is that layout: it carries `TotalTime` and no
/// `Position` at all, so the yield above has to be a question about the layout
/// and not only about the readout - otherwise an Eliminator race loses its
/// clock to a widget that is not there.
#[test]
fn a_layout_with_no_place_widget_keeps_its_total_time() {
    let layout = Layout::from_xml(
        r#"<Screen><Item OffsetX="445" OffsetY="5">
            <Text name="TotalTime"><Values scale="1.0" font="HUD" align="right" vertalign="bottom" x="0" y="30"/></Text>
            </Item></Screen>"#,
    );
    let strings = strings();
    let frame = draw_list(&context(&layout, &strings), &placed());
    assert_eq!(frame.hud_text.len(), 1, "{frame:?}");
}

#[test]
fn the_sample_yields_every_widget_it_declares() {
    let layout = Layout::from_xml(SAMPLE);
    assert_eq!(layout.sprites.len(), 4);
    assert_eq!(layout.labels.len(), 4);
    assert_eq!(layout.models.len(), 1);
    assert_eq!(layout.widget_count(), 8);
    assert!(
        layout.skipped.is_empty(),
        "nothing should be skipped: {:?}",
        layout.skipped
    );
}

/// The single most important thing this parser does. A child of an `<Item>`
/// is positioned relative to the group, and reading `x`/`y` as absolute puts
/// the entire HUD in the top-left corner - a failure that looks like a
/// layout bug rather than a parsing one.
#[test]
fn an_item_offsets_the_widgets_inside_it() {
    let layout = Layout::from_xml(SAMPLE);
    let bar = layout.sprite("SpeedBarBg").expect("SpeedBarBg");
    // 300 + 6, 210 + 10, and the size untouched.
    assert_eq!(bar.rect, [306.0, 220.0, 168.0, 26.0]);

    let lap = layout.label("Lap").expect("Lap");
    assert_eq!((lap.x, lap.y), (8.0, 37.0));
}

/// A widget outside any `<Item>` keeps its own coordinates.
#[test]
fn a_widget_outside_an_item_is_not_offset() {
    let layout = Layout::from_xml(SAMPLE);
    let diff = layout.label("TimeDiffText").expect("TimeDiffText");
    assert_eq!((diff.x, diff.y), (0.0, 5.0));
}

#[test]
fn centred_makes_the_position_the_middle_of_the_rectangle() {
    let layout = Layout::from_xml(SAMPLE);
    let box_ = layout.sprite("PickupBackground").expect("PickupBackground");
    // x=240 is the screen's centre line, so a 66-wide box starts at 207.
    assert_eq!(box_.rect, [207.0, 5.0, 66.0, 60.0]);
    // The centring must not disturb the source rectangle.
    assert_eq!(box_.uv, [3.0, 111.0, 66.0, 60.0]);
}

/// The name rule, against the layout's own widget names. See
/// [`pickup_icon_name`] - `docs/ui/hud.md` had these down as numeric ids
/// with no known mapping, and they are simply named after the weapons.
#[test]
fn a_weapons_icon_widget_is_named_after_the_weapon() {
    use oag_formats::weapons::Weapon;
    assert_eq!(pickup_icon_name(Weapon::Turbo), "TurboIcon");
    // The two the game misspells. Whatever the file says is what the layout
    // says, which is the whole reason this can be a rule at all.
    assert_eq!(pickup_icon_name(Weapon::LeachBeam), "LeachBeamIcon");
    assert_eq!(pickup_icon_name(Weapon::Repulser), "RepulserIcon");
}

/// Holding nothing draws neither the backdrop nor an icon, and holding
/// something draws exactly its own - not every icon the layout carries.
#[test]
fn the_pickup_widgets_are_drawn_only_for_what_is_held() {
    use oag_formats::weapons::Weapon;
    let layout = Layout::from_xml(SAMPLE);

    let empty = pickup_sprites(&layout, Weapon::Turbo, oag_pulse::hud::ART);
    assert_eq!(empty.len(), 2, "the fixture must author both widgets");

    let names: Vec<String> = pickup_sprites(&layout, Weapon::Turbo, oag_pulse::hud::ART)
        .iter()
        .map(|s| s.name.clone())
        .collect();
    assert_eq!(names, ["PickupBackground", "TurboIcon"]);

    let rocket: Vec<String> = pickup_sprites(&layout, Weapon::Rocket, oag_pulse::hud::ART)
        .iter()
        .map(|s| s.name.clone())
        .collect();
    assert_eq!(rocket, ["PickupBackground", "RocketIcon"]);

    // A weapon whose icon this layout does not author draws the backdrop and
    // no icon, rather than nothing or a panic: a layout carrying one and not
    // the other is the case that would otherwise crash a race.
    let absent = pickup_sprites(&layout, Weapon::Quake, oag_pulse::hud::ART);
    assert_eq!(absent.len(), 1);
    assert_eq!(absent[0].name, "PickupBackground");
}

/// **The backdrop is retinted and the icon is not**, which is the whole
/// reason a held pickup is legible - see
/// [`oag_pulse::hud::PICKUP_COLOURS`]. Both widgets are authored
/// `HudColour1`, which is opaque white, and the art behind them is a solid
/// white hexagon and a white glyph, so drawing them as authored is an opaque
/// hexagon with nothing visible on it.
#[test]
fn the_pickup_backdrop_takes_its_weapons_own_colour_and_the_icon_keeps_its_authored_one() {
    use oag_formats::weapons::Weapon;
    let layout = Layout::from_xml(SAMPLE);

    // The premise, asserted rather than assumed: if a future layout stopped
    // authoring both in the same colour, the substitution would need
    // revisiting and this is what would say so.
    let authored = layout.sprite("PickupBackground").expect("PickupBackground");
    let icon = layout.sprite("TurboIcon").expect("TurboIcon");
    assert_eq!(
        authored.color, icon.color,
        "the disc authors both in one colour; that is what makes the pair unreadable"
    );
    assert_eq!(
        authored.color,
        [1.0, 1.0, 1.0, 1.0],
        "and that colour is white"
    );

    let drawn = pickup_sprites(&layout, Weapon::Turbo, oag_pulse::hud::ART);
    assert_ne!(
        drawn[0].color, authored.color,
        "the backdrop must not keep the authored white"
    );
    // `PICKUP_COLOURS`' green, opaque - Turbo is one of the three weapons
    // that category covers.
    assert_eq!(
        drawn[0].color,
        crate::screen::argb_to_rgba(
            oag_pulse::hud::PICKUP_COLOURS[Weapon::Turbo as usize].unwrap()
        )
    );
    assert_eq!(
        drawn[1].color, icon.color,
        "the icon keeps what the disc authored"
    );
    // Everything else about the backdrop is untouched - this is a tint, not
    // a second widget.
    assert_eq!(drawn[0].rect, authored.rect);
    assert_eq!(drawn[0].uv, authored.uv);
}

/// **A weapon `PICKUP_COLOURS` has not measured falls back to
/// `pickup_backdrop_colour`**, the single quarter-alpha substitute this
/// build drew for every weapon before 2026-09-04 - see
/// [`oag_pulse::hud::PICKUP_BACKDROP_COLOUR`]. `Bomb` and `Mine` are that
/// case today: no weapon pad placement taken came up either, so this table
/// has no frame to draw them in.
#[test]
fn a_weapon_with_no_measured_colour_falls_back_to_the_single_substitute() {
    use oag_formats::weapons::Weapon;
    let layout = Layout::from_xml(SAMPLE);

    assert_eq!(
        oag_pulse::hud::PICKUP_COLOURS[Weapon::Bomb as usize],
        None,
        "this test is asserting the fallback path, which only fires when this is None"
    );

    let drawn = pickup_sprites(&layout, Weapon::Bomb, oag_pulse::hud::ART);
    // `HudBGColour` is `0x40000000` - a quarter-alpha black.
    assert_eq!(drawn[0].color[3], 0x40 as f32 / 255.0);
    assert_eq!(drawn[0].color[0..3], [0.0, 0.0, 0.0]);
}

#[test]
fn feconst_colours_resolve_through_the_variable_table() {
    let layout = Layout::from_xml(SAMPLE);
    assert_eq!(
        layout.constants.get("HudColour2").map(String::as_str),
        Some("0xFF7DEFC0")
    );

    let bar = layout.sprite("SpeedBarBg").expect("SpeedBarBg");
    // 0x60B5D7C8 -> alpha 0x60, and the channels in RGBA order.
    let [r, g, b, a] = bar.color;
    assert!((a - 96.0 / 255.0).abs() < 1e-6, "alpha was {a}");
    assert!((r - 181.0 / 255.0).abs() < 1e-6, "red was {r}");
    assert!((g - 215.0 / 255.0).abs() < 1e-6, "green was {g}");
    assert!((b - 200.0 / 255.0).abs() < 1e-6, "blue was {b}");
}

/// An unresolvable colour must not silently become black: a black HUD on a
/// dark track is invisible, and invisible reads as "not implemented".
#[test]
fn an_unknown_constant_leaves_the_colour_white() {
    let layout = Layout::from_xml(
        r#"<Screen><Image name="X"><Values x="0" y="0" width="4" height="4"
               Color="FEConst->NoSuchThing" Src="a.mip"/></Image></Screen>"#,
    );
    assert_eq!(layout.sprite("X").expect("X").color, [1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn both_spellings_of_vertical_centring_mean_the_same_thing() {
    assert_eq!(VertAlign::parse("middle"), VertAlign::Middle);
    assert_eq!(VertAlign::parse("centre"), VertAlign::Middle);
    assert_eq!(VertAlign::parse("center"), VertAlign::Middle);
    assert_eq!(VertAlign::parse("bottom"), VertAlign::Bottom);
    // Anything else is the renderer's own native behaviour.
    assert_eq!(VertAlign::parse(""), VertAlign::Top);
}

#[test]
fn the_three_font_roles_are_distinguished() {
    assert_eq!(Font::parse("HUD"), Font::Hud);
    assert_eq!(Font::parse("HUDSmall"), Font::Small);
    assert_eq!(Font::parse("hudsmall"), Font::Small);
    assert_eq!(Font::parse("Default"), Font::Default);
}

/// `TxtrWidth`/`TxtrHeight` are the *source* size and `width`/`height` the
/// *destination* size, and they really do differ in the shipped data.
#[test]
fn a_sprite_can_sample_larger_than_it_draws() {
    let layout = Layout::from_xml(
        r#"<Screen><Image name="TimeDiffIcon"><Values x="-35" y="0" width="14" height="12"
               U="52" V="86" TxtrWidth="28" TxtrHeight="23" Src="a.mip"/></Image></Screen>"#,
    );
    let icon = layout.sprite("TimeDiffIcon").expect("TimeDiffIcon");
    assert_eq!(icon.rect, [-35.0, 0.0, 14.0, 12.0]);
    assert_eq!(icon.uv, [52.0, 86.0, 28.0, 23.0]);
}

/// A missing `U`/`V` pair falls back to the destination size rather than to
/// zero, so a full-texture sprite draws the whole texture instead of nothing.
#[test]
fn a_sprite_with_no_source_rectangle_samples_its_own_size() {
    let layout = Layout::from_xml(
        r#"<Screen><Image name="Plain"><Values x="1" y="2" width="8" height="9" Src="a.mip"/></Image></Screen>"#,
    );
    assert_eq!(
        layout.sprite("Plain").expect("Plain").uv,
        [0.0, 0.0, 8.0, 9.0]
    );
}

/// `HeadToHeadBar`, the one widget of this kind on the disc, verbatim.
///
/// It has a `Color` and no `Src`, so it is a solid rectangle - and reading it
/// as a broken sprite loses it silently, which is what the parser did first.
#[test]
fn an_image_with_no_src_is_a_solid_fill_rather_than_a_broken_sprite() {
    let layout = Layout::from_xml(
        r#"<Screen>
            <Variable global="HudColour2"><Values String="0xFF7DEFC0"/></Variable>
            <Image name="HeadToHeadBar">
            <Values x="15" y="30" width="5" height="0" Color="FEConst->HudColour2" CalcBlur="1"></Values>
            </Image></Screen>"#,
    );
    assert!(layout.sprites.is_empty());
    assert!(
        layout.skipped.is_empty(),
        "a colour-only Image is not an error: {:?}",
        layout.skipped
    );
    let bar = layout.fill("HeadToHeadBar").expect("HeadToHeadBar");
    // The authored height really is zero - the bar's length is a runtime
    // quantity, so this must survive parsing rather than being defaulted.
    assert_eq!(bar.rect, [15.0, 30.0, 5.0, 0.0]);
    assert_eq!(bar.color, argb_to_rgba(0xFF7D_EFC0));
    assert_eq!(layout.widget_count(), 1);
}

/// The floating opponent tags are anchored at runtime, so their authored
/// position is an offset and may legitimately be negative.
#[test]
fn the_opponent_tags_are_not_screen_positioned() {
    assert!(!is_screen_positioned("PosTag0"));
    assert!(!is_screen_positioned("PlrTag7"));
    assert!(is_screen_positioned("SpeedBar"));
    assert!(is_screen_positioned("Lap"));
}

/// This asserted the opposite until 2026-08-17 - that a nameless widget was
/// dropped and counted in `skipped`. It is drawn instead; see [`anonymous_or`].
#[test]
fn a_nameless_widget_keeps_its_geometry_and_gets_no_name() {
    let layout = Layout::from_xml(
        r#"<Screen><Image><Values x="0" y="0" width="1" height="1" Src="a.mip"/></Image></Screen>"#,
    );
    assert_eq!(layout.sprites.len(), 1);
    assert_eq!(layout.sprites[0].name, "");
    assert_eq!(layout.sprites[0].rect, [0.0, 0.0, 1.0, 1.0]);
    assert!(layout.skipped.is_empty());
}

/// **A sprite is offset by its own texture's placement, not by the frame's.**
///
/// The bug this pins is what an HD race drew until 2026-08-25: a layout naming
/// six textures had one origin for all of them, so a sprite naming the second
/// sampled the first at coordinates meant for the second - the right rectangle
/// out of the wrong picture, which reads as art rather than as an error.
#[test]
fn a_sprite_is_offset_by_its_own_texture() {
    let placed = |y| crate::sprite::Placed {
        x: 0,
        y,
        width: 64,
        height: 64,
    };
    let sheet =
        crate::sprite::Sheet::placed_at(&[("first.mip", placed(0)), ("second.mip", placed(200))]);
    let sprite = |src: &str| Sprite {
        name: "X".to_string(),
        rect: [1.0, 2.0, 3.0, 4.0],
        uv: [10.0, 20.0, 30.0, 40.0],
        color: [1.0; 4],
        src: src.to_string(),
        rotation: 0.0,
    };

    let Some(Draw::Sprite { rect, uv, .. }) = sprite_draw(&sprite("second.mip"), &sheet) else {
        panic!("expected a sprite draw");
    };
    assert_eq!(rect, [1.0, 2.0, 3.0, 4.0], "the destination must not move");
    assert_eq!(uv, [10.0, 220.0, 30.0, 40.0]);

    // The first texture is at the sheet's own origin, so its sprites are
    // unmoved - which is every Pulse and Pure sprite there is.
    let Some(Draw::Sprite { uv, .. }) = sprite_draw(&sprite("first.mip"), &sheet) else {
        panic!("expected a sprite draw");
    };
    assert_eq!(uv, [10.0, 20.0, 30.0, 40.0]);

    // A texture the sheet does not hold draws nothing at all, rather than
    // sampling whichever one the packer put first.
    assert!(sprite_draw(&sprite("absent.mip"), &sheet).is_none());
}

#[test]
fn inside_screen_rejects_what_falls_off_the_edge() {
    assert!(inside_screen([0.0, 0.0, 480.0, 272.0]));
    assert!(!inside_screen([0.0, 0.0, 481.0, 272.0]));
    assert!(!inside_screen([-1.0, 0.0, 10.0, 10.0]));
}

fn strings() -> crate::language::StringTable {
    crate::language::StringTable::default()
}

/// The sheet [`SAMPLE`] would be drawn against: Pulse's atlas at the sheet's
/// own origin, which is where a one-texture sheet always puts it.
///
/// A `'static` because [`context`] hands out a borrow of it, and a real
/// placement rather than an empty sheet because [`sprite_draw`] draws nothing
/// for a texture the sheet does not hold - which is the point of it, and would
/// otherwise silently empty every sprite assertion below.
static SAMPLE_SHEET: std::sync::LazyLock<crate::sprite::Sheet> = std::sync::LazyLock::new(|| {
    crate::sprite::Sheet::placed_at(&[(
        oag_pulse::hud::ATLAS,
        crate::sprite::Placed {
            x: 0,
            y: 0,
            width: 256,
            height: 256,
        },
    )])
});

fn context<'a>(layout: &'a Layout, strings: &'a crate::language::StringTable) -> Context<'a> {
    Context {
        default_border: layout.default_border(),
        layout,
        strings,
        sheet: &SAMPLE_SHEET,
        art: oag_pulse::hud::ART,
        hud_line_height: 25.0,
        small_line_height: 10.0,
    }
}

/// Every reading of this format taken off the running original.
#[test]
fn a_lap_time_is_formatted_the_way_the_original_writes_it() {
    // `best 1.11.08`, from a lap boundary - see docs/tools/oag-trace.md.
    let ticks = (71.08 * TICKS_PER_SECOND).round() as u64;
    assert_eq!(format_lap_time(ticks, Precision::Hundredths), "1.11.08");

    // `0.50.25` - under a minute, so the minute field is a bare zero.
    let ticks = (50.25 * TICKS_PER_SECOND).round() as u64;
    assert_eq!(format_lap_time(ticks, Precision::Hundredths), "0.50.25");

    // `current 1.27.9` and `record 0.29.0`, both off the 2026-07-30 reference
    // frame: the running clocks carry **one** fractional digit, not two.
    let ticks = (87.9 * TICKS_PER_SECOND).round() as u64;
    assert_eq!(format_lap_time(ticks, Precision::Tenths), "1.27.9");
    let ticks = (29.0 * TICKS_PER_SECOND).round() as u64;
    assert_eq!(format_lap_time(ticks, Precision::Tenths), "0.29.0");
}

#[test]
fn a_lap_time_truncates_rather_than_rounding_up() {
    // 59/60 of a second is 0.983s: it has not reached 0.99, and must not be
    // shown as a whole second either.
    assert_eq!(format_lap_time(59, Precision::Hundredths), "0.00.98");
    assert_eq!(format_lap_time(60, Precision::Hundredths), "0.01.00");
    assert_eq!(format_lap_time(0, Precision::Hundredths), "0.00.00");
    // The same at one digit: 0.983s shows as .9, not 1.0.
    assert_eq!(format_lap_time(59, Precision::Tenths), "0.00.9");
}

/// A clock that rolls a minute over correctly is worth one assertion.
#[test]
fn the_minute_field_is_not_capped() {
    let ten_minutes = (600.0 * TICKS_PER_SECOND) as u64;
    assert_eq!(
        format_lap_time(ten_minutes, Precision::Hundredths),
        "10.00.00"
    );
}

#[test]
fn a_bar_fraction_never_leaves_zero_to_one() {
    let mut readout = Readout::blank();
    readout.speed_full_kmh = 600.0;

    readout.speed_kmh = 0.0;
    assert!((readout.speed_fraction() - 0.0).abs() < 1e-6);
    readout.speed_kmh = 300.0;
    assert!((readout.speed_fraction() - 0.5).abs() < 1e-6);
    // Over the maximum clamps rather than overflowing the bar.
    readout.speed_kmh = 900.0;
    assert!((readout.speed_fraction() - 1.0).abs() < 1e-6);
    // Reverse reads as empty, not as a negative-width quad.
    readout.speed_kmh = -50.0;
    assert!((readout.speed_fraction() - 0.0).abs() < 1e-6);
}

/// A zero pool must give zero rather than `NaN`. A `NaN` width draws nothing
/// at all, which looks exactly like the HUD not being wired up.
#[test]
fn an_empty_pool_gives_zero_rather_than_a_nan() {
    let readout = Readout {
        shield: 0.0,
        shield_max: 0.0,
        ..Readout::blank()
    };
    let f = readout.shield_fraction();
    assert!(f.is_finite(), "shield fraction was {f}");
    assert!((f - 0.0).abs() < 1e-6);
}

#[test]
fn a_bar_is_cropped_horizontally_and_keeps_its_height() {
    let sprite = Sprite {
        name: "SpeedBar".to_string(),
        rect: [306.0, 220.0, 168.0, 26.0],
        uv: [6.0, 0.0, 168.0, 26.0],
        color: [1.0; 4],
        src: oag_pulse::hud::ATLAS.to_string(),
        rotation: 0.0,
    };
    let half = crop_horizontally(&sprite, 0.5);
    assert_eq!(half.rect, [306.0, 220.0, 84.0, 26.0]);
    // The source has to be cropped by the same factor, or the art is squashed
    // into the shorter bar rather than clipped by it.
    assert_eq!(half.uv, [6.0, 0.0, 84.0, 26.0]);
}

#[test]
fn a_bottom_anchored_label_is_lifted_by_its_own_line_height() {
    let label = Label {
        name: "Lap".to_string(),
        x: 8.0,
        y: 37.0,
        font: Font::Hud,
        scale: 1.0,
        color: [1.0; 4],
        border: None,
        align: Align::Left,
        vertalign: VertAlign::Bottom,
        idstring: None,
        string: None,
    };
    assert!((top_edge(&label, 25.0) - 12.0).abs() < 1e-6);

    // Scale multiplies the lift, because the drawn line is that much taller.
    let scaled = Label {
        scale: 0.6,
        ..label.clone()
    };
    assert!((top_edge(&scaled, 25.0) - 22.0).abs() < 1e-6);

    let middle = Label {
        vertalign: VertAlign::Middle,
        ..label.clone()
    };
    assert!((top_edge(&middle, 25.0) - 24.5).abs() < 1e-6);

    let top = Label {
        vertalign: VertAlign::Top,
        ..label
    };
    assert!((top_edge(&top, 25.0) - 37.0).abs() < 1e-6);
}

#[test]
fn the_two_fonts_go_to_separate_passes() {
    let layout = Layout::from_xml(SAMPLE);
    let strings = strings();
    let readout = Readout {
        speed_kmh: 300.0,
        lap: 2,
        ..Readout::blank()
    };
    let frame = draw_list(&context(&layout, &strings), &readout);

    // `SpeedBarText` and `LapTxt` are HUDSmall; `Lap` is HUD.
    let hud: Vec<&str> = frame
        .hud_text
        .iter()
        .filter_map(|d| match d {
            Draw::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(hud, ["2"], "the lap value belongs in the HUD font");

    assert_eq!(frame.small_text.len(), 2, "{:?}", frame.small_text);
    assert!(!frame.is_empty());
}

/// A blank readout must still produce a valid frame rather than panicking or
/// emitting a degenerate quad: this is what the first frame of a race is.
#[test]
fn a_blank_readout_draws_the_furniture_and_no_bars() {
    let layout = Layout::from_xml(SAMPLE);
    let strings = strings();
    let frame = draw_list(&context(&layout, &strings), &Readout::blank());

    for draw in &frame.sprites {
        let Draw::Sprite { rect, .. } = draw else {
            panic!("expected sprites only");
        };
        assert!(
            rect[2] > 0.0 && rect[3] > 0.0,
            "a zero-area quad reached the draw list: {rect:?}"
        );
    }
    // The background is furniture and is always there; the fill is not.
    assert!(
        !frame.sprites.is_empty(),
        "the bar background should still draw"
    );
    // Lap is unknown at 0, so the value is omitted rather than shown as "0".
    assert!(
        frame.hud_text.is_empty(),
        "nothing should claim a value it does not have: {:?}",
        frame.hud_text
    );
}

/// With no best lap set the original shows **zeros**, not a dash placeholder:
/// `best 0.00.00` on the reference frame. An earlier revision here invented
/// `-.--.--`, which is the kind of thing that looks deliberate forever.
#[test]
fn a_missing_best_lap_shows_zeros_at_hundredths() {
    let layout = Layout::from_xml(
        r#"<Screen><Text name="BestTime"><Values font="HUD" x="0" y="0"/></Text></Screen>"#,
    );
    let strings = strings();
    let frame = draw_list(&context(&layout, &strings), &Readout::blank());
    let Some(Draw::Text { text, .. }) = frame.hud_text.first() else {
        panic!("expected the best-time widget");
    };
    assert_eq!(text, "0.00.00");
}

/// The outline is what makes a pre-outlined glyph read as a glyph, and 57 of
/// the 84 HUD-font widgets name no colour for it, so the default is doing real
/// work rather than covering a corner case.
#[test]
fn a_widget_with_no_border_colour_still_gets_the_layouts_own() {
    let layout = Layout::from_xml(
        r#"<Screen>
            <Variable global="HudBGColour"><Values String="0x40000000"/></Variable>
            <Text name="CurrentTime"><Values font="HUD" x="0" y="0"/></Text>
            </Screen>"#,
    );
    assert_eq!(layout.default_border(), argb_to_rgba(0x4000_0000));
    // The widget itself declares none...
    assert_eq!(
        layout.label("CurrentTime").expect("CurrentTime").border,
        None
    );

    // ...and the draw list supplies one anyway.
    let strings = strings();
    let frame = draw_list(&context(&layout, &strings), &Readout::blank());
    let Some(Draw::Text { border, .. }) = frame.hud_text.first() else {
        panic!("expected the current-time widget");
    };
    assert_eq!(*border, Some(argb_to_rgba(0x4000_0000)));
}

/// A layout with no `HudBGColour` must still outline rather than fall through
/// to "no border", which is what drew filled boxes.
#[test]
fn a_layout_with_no_background_constant_falls_back_to_black() {
    let layout = Layout::from_xml(
        r#"<Screen><Text name="CurrentTime"><Values font="HUD" x="0" y="0"/></Text></Screen>"#,
    );
    assert_eq!(layout.default_border(), FALLBACK_BORDER);
}

/// The shield readout is a percentage, not the raw `<Misc shield/>` pool -
/// `100%` on the reference frame against a pool of several hundred units.
#[test]
fn the_shield_readout_is_a_percentage() {
    let layout = Layout::from_xml(
        r#"<Screen><Text name="ShieldBarText"><Values font="HUDSmall" x="0" y="0"/></Text></Screen>"#,
    );
    let strings = strings();
    let readout = Readout {
        shield: 300.0,
        shield_max: 300.0,
        ..Readout::blank()
    };
    let frame = draw_list(&context(&layout, &strings), &readout);
    let Some(Draw::Text { text, .. }) = frame.small_text.first() else {
        panic!("expected the shield readout");
    };
    assert_eq!(text, "100%");

    let readout = Readout {
        shield: 150.0,
        shield_max: 300.0,
        ..Readout::blank()
    };
    let frame = draw_list(&context(&layout, &strings), &readout);
    let Some(Draw::Text { text, .. }) = frame.small_text.first() else {
        panic!("expected the shield readout");
    };
    assert_eq!(text, "50%");
}

#[test]
fn the_wrong_way_warning_only_appears_when_it_applies() {
    let layout = Layout::from_xml(
        r#"<Screen><Text name="WrongWay"><Values font="HUD" x="240" y="70"/></Text></Screen>"#,
    );
    let strings = strings();

    let quiet = draw_list(&context(&layout, &strings), &Readout::blank());
    assert!(quiet.is_empty(), "{quiet:?}");

    let readout = Readout {
        wrong_way: true,
        ..Readout::blank()
    };
    let warned = draw_list(&context(&layout, &strings), &readout);
    assert_eq!(warned.hud_text.len(), 1);
}

/// The opponent tags are runtime-anchored, so drawing them at their authored
/// position would put text at negative coordinates.
#[test]
fn the_opponent_tags_are_never_drawn() {
    let layout = Layout::from_xml(
        r#"<Screen><Item OffsetX="-40" OffsetY="0">
            <Text name="PosTag0"><Values idstring="X" font="Default" x="0" y="20"/></Text>
            </Item></Screen>"#,
    );
    let strings = strings();
    let frame = draw_list(&context(&layout, &strings), &Readout::blank());
    assert!(frame.is_empty(), "{frame:?}");
}

/// Splitting text into two passes gives up strict document paint order between
/// fonts, which is only safe if no two live labels overlap.
///
/// Here on the sample, and on the shipped files by
/// `hud_layout_ground_truth::no_two_live_widgets_share_an_anchor_on_any_shipped_layout`,
/// which is where the property is really tested and which **found a collision**
/// the first time it ran: see [`place_owns_the_anchor`]. This comment claimed
/// the ground-truth check existed before it did.
#[test]
fn no_two_live_labels_overlap() {
    let layout = Layout::from_xml(SAMPLE);
    let strings = strings();
    let readout = Readout {
        speed_kmh: 300.0,
        lap: 2,
        laps: 3,
        ..Readout::blank()
    };
    let frame = draw_list(&context(&layout, &strings), &readout);

    let mut anchors: Vec<(f32, f32)> = Vec::new();
    for draw in frame.hud_text.iter().chain(frame.small_text.iter()) {
        let Draw::Text { x, y, .. } = draw else {
            continue;
        };
        assert!(
            !anchors.contains(&(*x, *y)),
            "two labels share the anchor ({x}, {y}), so paint order between the \
             two font passes would decide which is visible"
        );
        anchors.push((*x, *y));
    }
}
