//! What the in-race HUD in [`super`] is asserted to do: reading the disc's own
//! layout XML, where each widget lands, the colours it resolves through
//! `FEConst`, the lap-time and bar formatting, and what a readout draws.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of `hud.rs`:
//! well past the 200 lines an inline test module may hold. See
//! `scripts/check-file-size.py`, which is the rule as a gate. The readout and
//! frame half split off again into [`super::readout_tests`] when the file
//! reached 1,000 lines; the fixtures below stay here and are shared by name.

use super::*;
// The draw list moved to `super::draw` when `hud.rs` was split; its decisions
// are `pub(super)`, so this reaches them by name rather than through the
// parent's own imports.
use super::draw::*;
use super::pickup::*;

/// The shape of the real `TimeTrial_HUD.xml`, cut to what this module reads.
/// Every attribute spelling, the `FEConst->` indirection, the `<Item>`
/// offsets and the mixed `centre`/`middle` vertical alignment are as they
/// appear on the disc.
pub(super) const SAMPLE: &str = r#"
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
                    true,
                    layout.label("Position Outof").is_none(),
                    crate::Precision::Tenths,
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

/// A layout with a clock and no place keeps its clock in a timed mode, in a
/// field.
///
/// The yield above has to be a question about the layout and not only about the
/// readout: a layout that authors `TotalTime` and no `Position` has nothing to
/// yield to. (This test's doc used to name `Elimination_HUD.xml` as the layout
/// that keeps its clock in a race with a field. That was an inference and a
/// live Eliminator frame refutes it - see
/// [`the_clock_is_hidden_outside_the_timed_modes`].)
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

/// Pulse's HUD update leaves the clock hidden in every mode but Time Trial and
/// Speed Lap, whatever the layout authors and whatever the place is.
///
/// `PlayerStatus_Update` writes the target field to `-1` each tick and
/// `Hud_UpdateTimeCluster` hides `TotalTime`/`TotalTimeTxt` while it reads
/// that. A PPSSPP frame of an Eliminator race (`elim-hud-01.png`, 2026-09-30)
/// has no `TOTAL` in its top-right corner, and the two widgets' flag words read
/// live with the visible bit clear. The fixture is the real `Arcade_HUD.xml`
/// corner, so a single race with no place yet is covered as well as
/// `Elimination_HUD.xml`'s own shape.
#[test]
fn the_clock_is_hidden_outside_the_timed_modes() {
    use oag_race::Mode;
    let layout = Layout::from_xml(TOP_RIGHT);
    let strings = strings();
    let shown = |art: &oag_title::HudArt, mode: Mode| -> bool {
        let cx = Context {
            art,
            ..context(&layout, &strings)
        };
        let readout = Readout {
            mode,
            ..Readout::blank()
        };
        let frame = draw_list(&cx, &readout);
        !frame.hud_text.is_empty() && !frame.small_text.is_empty()
    };

    for mode in [
        Mode::TimeTrial,
        Mode::SpeedLap,
        Mode::Zone,
        Mode::SingleRace,
        Mode::Eliminator,
        Mode::Tournament,
        Mode::Head2Head,
    ] {
        assert_eq!(
            shown(oag_pulse::hud::ART, mode),
            matches!(mode, Mode::TimeTrial | Mode::SpeedLap),
            "{mode:?}"
        );
    }

    // A title that has not had the rule checked keeps drawing it, as it always
    // did: 2048's own frame shows `TOTAL` beside `POS` in a race with a field.
    let unmeasured = oag_title::HudArt {
        total_time_timed_modes_only: false,
        total_time_hundredths: false,
        kill_column: false,
        message_slots: false,
        ..*oag_pulse::hud::ART
    };
    assert!(shown(&unmeasured, Mode::Eliminator));
    assert!(shown(&unmeasured, Mode::SingleRace));
}

/// A campaign Time Trial cell substitutes the medal caption and the
/// countdown-to-target for `TotalTime`/`TotalTimeTxt`'s own plain elapsed
/// clock, on pace: no red.
#[test]
fn a_campaign_pace_substitutes_the_caption_and_the_countdown() {
    let layout = Layout::from_xml(TOP_RIGHT);
    let strings = strings();
    let readout = Readout {
        time_trial_pace: Some(super::time_trial_pace::TimeTrialPace {
            tier: super::time_trial_pace::PaceTier::Silver,
            remaining_ticks: 179,
            missed: false,
        }),
        ..Readout::blank()
    };
    let frame = draw_list(&context(&layout, &strings), &readout);

    let hud_text: Vec<(&str, [f32; 4])> = frame
        .hud_text
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, color, .. } => Some((text.as_str(), *color)),
            _ => None,
        })
        .collect();
    // `179` ticks is `2.98` at `Precision::Tenths` - `format_lap_time`'s own
    // truncation, not `TimeTrialPace`'s.
    assert_eq!(hud_text, [("0.02.9", [1.0, 1.0, 1.0, 1.0])], "{frame:?}");

    let small_text: Vec<&str> = frame
        .small_text
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    // Substituted, not the layout's own `idstring="IG_HUD_TOTAL"`.
    assert_eq!(small_text, ["IG_HUD_SILVER"], "{frame:?}");
}

/// Past the cell's own bronze target: the caption sticks on `BRONZE`, the
/// countdown reads zero, and `TotalTime` alone reddens -
/// `Hud_UpdateTimeCluster_q`'s literal `0xffff0000`, never applied to the
/// caption.
#[test]
fn a_missed_campaign_pace_reddens_only_the_countdown() {
    let layout = Layout::from_xml(TOP_RIGHT);
    let strings = strings();
    let readout = Readout {
        time_trial_pace: Some(super::time_trial_pace::TimeTrialPace {
            tier: super::time_trial_pace::PaceTier::Bronze,
            remaining_ticks: 0,
            missed: true,
        }),
        ..Readout::blank()
    };
    let frame = draw_list(&context(&layout, &strings), &readout);

    let hud_text: Vec<(&str, [f32; 4])> = frame
        .hud_text
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, color, .. } => Some((text.as_str(), *color)),
            _ => None,
        })
        .collect();
    assert_eq!(hud_text, [("0.00.0", [1.0, 0.0, 0.0, 1.0])], "{frame:?}");

    let small_text: Vec<&str> = frame
        .small_text
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(small_text, ["IG_HUD_BRONZE"], "{frame:?}");
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
    assert_eq!(
        layout.models[0].colour, None,
        "SAMPLE's ReadyGo model authors no colour, the same as every sight \
         model on Pulse and HD/Fury"
    );
}

/// `Data\XML\TimeTrial_HUD.xml`'s own spelling of a weapon icon, cut to one
/// widget: `colour="0xff40ff40"`, lowercase, no `FEConst->` indirection.
/// `crates/pure/src/hud.rs`'s module doc has the other nine.
const PURE_WEAPON_ICON: &str = r#"
<Screen>
<Mode3D>
<Model name="TURBO_icon">
<Values Src="Data\HUD\Weapon_turbo.vex" colour="0xff40ff40" x="240" y="250" z="-10" ztest="0"></Values>
</Model>
</Mode3D>
</Screen>
"#;

#[test]
fn a_models_own_colour_attribute_parses_where_pulse_and_hd_never_author_one() {
    let layout = Layout::from_xml(PURE_WEAPON_ICON);
    assert_eq!(layout.models.len(), 1);
    assert_eq!(
        layout.models[0].colour,
        Some(argb_to_rgba(0xff40_ff40)),
        "Pure's own `colour` spelling, not `Color`, and not routed through \
         the white-when-absent `colour()` helper Image/Text widgets use"
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
    use oag_tables::weapons::Weapon;
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
    use oag_tables::weapons::Weapon;
    let layout = Layout::from_xml(SAMPLE);

    let empty = pickup_sprites(&layout, Weapon::Turbo, oag_pulse::hud::ART, false);
    assert_eq!(empty.len(), 2, "the fixture must author both widgets");

    let names: Vec<String> = pickup_sprites(&layout, Weapon::Turbo, oag_pulse::hud::ART, false)
        .iter()
        .map(|s| s.name.clone())
        .collect();
    assert_eq!(names, ["PickupBackground", "TurboIcon"]);

    let rocket: Vec<String> = pickup_sprites(&layout, Weapon::Rocket, oag_pulse::hud::ART, false)
        .iter()
        .map(|s| s.name.clone())
        .collect();
    assert_eq!(rocket, ["PickupBackground", "RocketIcon"]);

    // A weapon whose icon this layout does not author draws the backdrop and
    // no icon, rather than nothing or a panic: a layout carrying one and not
    // the other is the case that would otherwise crash a race.
    let absent = pickup_sprites(&layout, Weapon::Quake, oag_pulse::hud::ART, false);
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
    use oag_tables::weapons::Weapon;
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

    let drawn = pickup_sprites(&layout, Weapon::Turbo, oag_pulse::hud::ART, false);
    assert_ne!(
        drawn[0].color, authored.color,
        "the backdrop must not keep the authored white"
    );
    // The recovered green word, alpha 0x80 kept: translucent, not opaque.
    assert_eq!(
        drawn[0].color,
        oag_ui::screen::argb_to_rgba(0x801C_BC1C),
        "Turbo's backdrop is the recovered green word"
    );
    assert_eq!(drawn[0].color[3], 0x80 as f32 / 255.0, "alpha is kept");
    assert_eq!(
        drawn[1].color, icon.color,
        "the icon keeps what the disc authored"
    );
    // Everything else about the backdrop is untouched - this is a tint, not
    // a second widget.
    assert_eq!(drawn[0].rect, authored.rect);
    assert_eq!(drawn[0].uv, authored.uv);
}

/// 2048's own `HUD_pickups.xml` shape, cut to what `pickup_sprites` reads:
/// one `PickupIcon` at its authored top-centre rect and one `PickupBgFrame`
/// at the bottom-left slot - values straight off
/// `cargo run -p oag-game --example vita_2048_hud_dump`.
const SAMPLE_2048_PICKUP: &str = r#"
<Screen>
<Screen name="HUD">
<Image name="PickupBgFrame">
<Values x="87" y="400" width="110" height="95" U="74" V="48" TxtrWidth="110" TxtrHeight="95" Color="0xFFFFFFFF" Src="Data\XML\2048_hud\Texture\hud_2048.gxt"></Values>
</Image>
<Image name="PickupIcon">
<Values x="437.5" y="17" width="85" height="86" U="6" V="201" TxtrWidth="85" TxtrHeight="86" Color="0xFFFFFFFF" Src="Data\XML\2048_hud\Texture\hud_2048.gxt"></Values>
</Image>
</Screen>
</Screen>
"#;

/// [`oag_title::HudArt::pickup_icon_uv`]'s own branch of `pickup_sprites`:
/// the backdrop draws at its own authored rect, unmoved, and the icon's UV
/// is rewritten from the table while its destination rect is recentred
/// inside the backdrop - not left at the authored top-centre announcement
/// position, and not left at the table's own `U`/`V` either.
#[test]
fn the_2048_pickup_icon_is_uv_rewritten_and_recentred_in_its_frame() {
    use oag_tables::weapons::Weapon;
    let layout = Layout::from_xml(SAMPLE_2048_PICKUP);

    let drawn = pickup_sprites(&layout, Weapon::Rocket, oag_2048::hud::ART, false);
    assert_eq!(drawn.len(), 2, "the fixture must author both widgets");
    assert_eq!(drawn[0].name, "PickupBgFrame");
    assert_eq!(drawn[1].name, "PickupIcon");

    // The backdrop is untouched - same rect, same uv, same colour.
    let frame = layout.sprite("PickupBgFrame").expect("PickupBgFrame");
    assert_eq!(drawn[0].rect, frame.rect);
    assert_eq!(drawn[0].uv, frame.uv);

    // The icon's uv is the table's Rocket entry, not the authored `(6, 201)`.
    assert_eq!(drawn[1].uv, [203.0, 201.0, 85.0, 86.0]);
    // The icon's rect keeps its own 85x86 size and moves to the frame's own
    // centre: frame is (87, 400, 110, 95), icon is 85x86, so
    // x = 87 + (110-85)/2 = 99.5, y = 400 + (95-86)/2 = 404.5.
    assert_eq!(drawn[1].rect, [99.5, 404.5, 85.0, 86.0]);

    // A weapon this table has no entry for draws the backdrop alone, the
    // same "draw what is held, nothing invented for what is not" rule
    // `the_pickup_widgets_are_drawn_only_for_what_is_held` already covers
    // for Pulse's dialect.
    let unimplemented = pickup_sprites(&layout, Weapon::Repulser, oag_2048::hud::ART, false);
    assert_eq!(unimplemented.len(), 1);
    assert_eq!(unimplemented[0].name, "PickupBgFrame");
}

/// The grant announcement: while it is up the icon sits at its authored top-centre rect with no
/// frame, `PickupText` names the weapon, and after it the held state is what `the_2048_pickup_icon_is_uv_rewritten...`
/// pins. Fails if `Readout::pickup_announcing` stops reaching the sprites or the caption.
#[test]
fn the_2048_grant_announcement_is_the_icon_at_its_authored_rect_with_a_caption() {
    use oag_tables::weapons::Weapon;
    let layout = Layout::from_xml(SAMPLE_2048_PICKUP);
    let announced = pickup_sprites(&layout, Weapon::Rocket, oag_2048::hud::ART, true);
    assert_eq!(announced.len(), 1, "no PickupBgFrame while announcing");
    assert_eq!(announced[0].name, "PickupIcon");
    assert_eq!(announced[0].uv, [203.0, 201.0, 85.0, 86.0]);
    assert_eq!(
        announced[0].rect,
        layout.sprite("PickupIcon").expect("authored").rect
    );

    let caption_layout = Layout::from_xml(
        r#"<Screen><Screen name="HUD"><Text name="PickupText"><Values font="HUD" x="480" y="60"/></Text></Screen></Screen>"#,
    );
    let caption = caption_layout
        .label("PickupText")
        .expect("authored")
        .clone();
    let strings = oag_ui::language::StringTable::default();
    let text = |readout: &Readout| {
        super::draw::text_for(
            &caption,
            readout,
            &strings,
            true,
            false,
            None,
            true,
            false,
            crate::Precision::Tenths,
        )
    };
    let mut readout = Readout::blank();
    readout.pickup = Some(Weapon::Missile);
    assert_eq!(text(&readout), None, "nothing before the announcement");
    readout.pickup_announcing = true;
    assert_eq!(text(&readout).as_deref(), Some("FE_MISSILE"));
}

/// **Every Pulse weapon draws its recovered translucent word, in three groups**,
/// and `Bomb`/`Mine` no longer fall back to `HudBGColour`. Fails if the alpha
/// is dropped, a grouping moves, or a weapon loses its row.
#[test]
fn every_pulse_weapon_draws_its_recovered_translucent_backdrop_word() {
    use oag_tables::weapons::Weapon;
    let layout = Layout::from_xml(SAMPLE);
    let pink = 0x80BE_1E69;
    let green = 0x801C_BC1C;
    let blue = 0x802B_1CBC;
    let expected = |w: Weapon| match w {
        Weapon::Turbo | Weapon::Shield | Weapon::Autopilot => green,
        Weapon::Bomb | Weapon::Mine => blue,
        _ => pink,
    };
    for weapon in Weapon::ALL
        .into_iter()
        .filter(|w| oag_pulse::hud::PICKUP_COLOURS[*w as usize].is_some())
    {
        let drawn = pickup_sprites(&layout, weapon, oag_pulse::hud::ART, false);
        assert_eq!(
            drawn[0].color,
            oag_ui::screen::argb_to_rgba(expected(weapon)),
            "{weapon:?}"
        );
        assert_eq!(drawn[0].color[3], 0x80 as f32 / 255.0, "{weapon:?} alpha");
    }
    let blue_drawn = pickup_sprites(&layout, Weapon::Mine, oag_pulse::hud::ART, false);
    assert_ne!(
        blue_drawn[0].color[3],
        0x40 as f32 / 255.0,
        "Mine is not HudBGColour any more"
    );
    assert_eq!(
        oag_pulse::hud::PICKUP_COLOURS.iter().flatten().count(),
        13,
        "all thirteen Pulse weapons carry a word; Disruptor stays None"
    );
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

/// A missing `U`/`V` pair is the texture's corner and a missing size is the
/// whole texture (`0` x `0`, resolved by [`sprite_draw`]), so a full-texture
/// sprite draws the whole texture instead of nothing.
#[test]
fn a_sprite_with_no_source_rectangle_samples_the_whole_texture() {
    let layout = Layout::from_xml(
        r#"<Screen><Image name="Plain"><Values x="1" y="2" width="8" height="9" Src="a.mip"/></Image></Screen>"#,
    );
    assert_eq!(
        layout.sprite("Plain").expect("Plain").uv,
        [0.0, 0.0, 0.0, 0.0]
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

/// The multiplayer tags are anchored at runtime, so their authored position is
/// an offset and may legitimately be negative. `PosTag0`-`PosTag7` are a fixed
/// column and draw where they are authored.
#[test]
fn only_the_multiplayer_tags_are_not_screen_positioned() {
    assert!(is_screen_positioned("PosTag0"));
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
        quad_extent: None,
        quad_centre: None,
        blend: None,
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
        gradient: None,
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
    let psp = oag_display::space::Space::PSP.size;
    assert!(inside_screen([0.0, 0.0, 480.0, 272.0], psp));
    assert!(!inside_screen([0.0, 0.0, 481.0, 272.0], psp));
    assert!(!inside_screen([-1.0, 0.0, 10.0, 10.0], psp));
}

// A PSP rect at x=480 is off the PSP's own screen and inside the PS2's 640x448
// - the whole reason `inside_screen` takes a `screen` parameter rather than
// assuming `oag_display::space::SCREEN`.
#[test]
fn inside_screen_takes_the_source_grid_it_is_given() {
    let rect = [480.0, 0.0, 10.0, 10.0];
    assert!(!inside_screen(rect, oag_display::space::Space::PSP.size));
    assert!(inside_screen(rect, oag_display::space::Space::PS2.size));
}

pub(super) fn strings() -> oag_ui::language::StringTable {
    oag_ui::language::StringTable::default()
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
            quad_extent: None,
            quad_centre: None,
            blend: None,
        },
    )])
});

pub(super) fn context<'a>(
    layout: &'a Layout,
    strings: &'a oag_ui::language::StringTable,
) -> Context<'a> {
    Context {
        default_border: layout.default_border(),
        layout,
        strings,
        sheet: &SAMPLE_SHEET,
        art: oag_pulse::hud::ART,
        hud_line_height: 25.0,
        small_line_height: 10.0,
        default_line_height: 10.0,
        skin_line_height: 10.0,
    }
}

/// **A widget that authors no `TxtrWidth`/`TxtrHeight` samples its whole
/// texture, scaled to its box**: 2048's `PilotAssist` is a 64x32 box over a
/// 256x128 texture whose alpha spans all of it, and the original draws the icon
/// whole. Cropping a 64x32 patch off the corner drew a solid white bar.
#[test]
fn a_sprite_with_no_texture_size_samples_its_whole_texture() {
    let layout = Layout::from_xml(
        r#"<Screen><Image name="Icon"><Values x="10" y="20" width="64" height="32" U="0" V="0" Src="a.mip"/></Image>
        <Image name="Patch"><Values x="0" y="0" width="64" height="32" U="5" V="6" TxtrWidth="64" TxtrHeight="32" Src="a.mip"/></Image></Screen>"#,
    );
    let placed = crate::sprite::Placed {
        x: 0,
        y: 100,
        width: 256,
        height: 128,
        quad_extent: None,
        quad_centre: None,
        blend: None,
    };
    let sheet = crate::sprite::Sheet::placed_at(&[("a.mip", placed)]);
    let uv_of = |name: &str| {
        let sprite = layout.sprites.iter().find(|s| s.name == name).expect(name);
        let Some(Draw::Sprite { uv, .. }) = sprite_draw(sprite, &sheet) else {
            panic!("expected a sprite draw");
        };
        uv
    };
    assert_eq!(uv_of("Icon"), [0.0, 100.0, 256.0, 128.0]);
    assert_eq!(
        uv_of("Patch"),
        [5.0, 106.0, 64.0, 32.0],
        "an authored size is kept"
    );
}

/// `TotalTime` carries hundredths where the title measured it
/// ([`oag_title::HudArt::total_time_hundredths`]) and tenths elsewhere, while
/// `CurrentTime` stays in tenths on both.
#[test]
fn the_total_time_precision_follows_the_title() {
    let layout = Layout::from_xml(
        r#"<Screen><Text name="TotalTime"><Values font="HUD" x="0" y="0"/></Text>
        <Text name="CurrentTime"><Values font="HUD" x="0" y="0"/></Text></Screen>"#,
    );
    let strings = oag_ui::language::StringTable::default();
    let readout = Readout {
        race_ticks: 178,
        lap_ticks: 178,
        ..Readout::blank()
    };
    let text = |name: &str, precision| {
        let label = layout.labels.iter().find(|l| l.name == name).expect(name);
        text_for(
            label, &readout, &strings, false, false, None, false, false, precision,
        )
    };
    assert_eq!(
        text("TotalTime", Precision::Hundredths).as_deref(),
        Some("0.02.96")
    );
    assert_eq!(
        text("TotalTime", Precision::Tenths).as_deref(),
        Some("0.02.9")
    );
    assert_eq!(
        text("CurrentTime", Precision::Hundredths).as_deref(),
        Some("0.02.9")
    );
    assert!(oag_2048::hud::ART.total_time_hundredths);
    assert!(!oag_pulse::hud::ART.total_time_hundredths);
    assert!(!oag_hd::hud::ART.total_time_hundredths);
}
