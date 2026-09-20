//! What the HUD's *readout* becomes: lap-time and bar formatting, which pass a
//! label goes to, and what a whole frame draws for a given [`Readout`].
//!
//! Split off [`super::tests`] under the 1,000-line rule in
//! `scripts/check-file-size.py`, along the seam that file already had - the
//! half above the split reads the layout XML and places widgets, this half
//! feeds a readout through [`super::draw::draw_list`] and asserts the frame.
//! The fixtures stay where they were and are shared by name: a sibling test
//! module reaches them through `super::tests`.

use super::draw::*;
use super::tests::{SAMPLE, context, strings};
use super::*;

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

/// `oag_2048::hud::EnergyBar`'s own rect and uv, off
/// `cargo run -p oag-game --example vita_2048_hud_dump` against the played
/// Arcade layout - `110.0` halves exactly, so `assert_eq!` needs no epsilon
/// the way `crop_horizontally`'s own test does not either.
#[test]
fn a_bar_is_cropped_vertically_from_the_bottom_and_keeps_its_width() {
    let sprite = Sprite {
        name: "EnergyBar".to_string(),
        rect: [27.0, 372.0, 50.0, 110.0],
        uv: [270.0, 16.0, 52.0, 110.0],
        color: [0.188_235_3, 1.0, 0.188_235_3, 0.501_960_8],
        src: oag_2048::hud::skins::played::ARCADE.to_string(),
        rotation: 0.0,
    };

    // Full: the identity, same as `crop_horizontally` at `1.0`.
    let full = crop_vertically(&sprite, 1.0);
    assert_eq!(full.rect, sprite.rect);
    assert_eq!(full.uv, sprite.uv);

    // Half: the bottom 55 px of a 110 px bar stays, so the top edge moves
    // down by the other 55 and the height matches it - width and colour are
    // untouched, the same way `crop_horizontally` leaves height and colour
    // alone.
    let half = crop_vertically(&sprite, 0.5);
    assert_eq!(half.rect, [27.0, 427.0, 50.0, 55.0]);
    assert_eq!(half.uv, [270.0, 71.0, 52.0, 55.0]);
    assert_eq!(half.color, sprite.color);

    // Empty: the whole rect collapses to its own bottom edge rather than a
    // negative-height quad.
    let empty = crop_vertically(&sprite, 0.0);
    assert_eq!(empty.rect, [27.0, 482.0, 50.0, 0.0]);
    assert_eq!(empty.uv, [270.0, 126.0, 52.0, 0.0]);
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

/// HD/Fury drops the `%` - `oag_title::HudArt::shield_percent`'s own doc
/// comment carries the evidence, all three of `talons-matched/{00,01,03}.png`.
/// Pulse's own reading (the test above) keeps it, so this is a title fact
/// rather than a formatting default.
#[test]
fn the_shield_readout_drops_the_percent_on_a_title_that_measures_none() {
    let layout = Layout::from_xml(
        r#"<Screen><Text name="ShieldBarText"><Values font="HUDSmall" x="0" y="0"/></Text></Screen>"#,
    );
    let strings = strings();
    let readout = Readout {
        shield: 98.0,
        shield_max: 100.0,
        ..Readout::blank()
    };
    let label = &layout.labels[0];
    assert_eq!(
        text_for(label, &readout, &strings, false, true, None, false),
        Some("98".to_string())
    );
    assert_eq!(
        text_for(label, &readout, &strings, false, true, None, true),
        Some("98%".to_string())
    );
}

/// **Drawn nothing, not the caption fallback.** `HUD_pickups.xml`'s
/// `PickupDamageTxt`/`PickupAbsorbTxt` (idstring `MSC_DAMAGE`/`MSC_ABSORB`) and
/// the numbers beside them fell through `text_for`'s catch-all before this,
/// which draws any idstring-bearing widget as a static caption - so an
/// undamaged HD race drew `DAMAGE`/`Absorb` with nothing under them, which
/// `talons-matched/00.png` and `01.png` (full shield, no recent hit) do not
/// show at all. `03.png` (after a hit) shows both with real numbers, but
/// nothing in this build tracks a recent hit's damage/absorb total yet - see
/// `docs/formats/hd-hud.md` - so this stays `None` outright rather than a
/// half-wired gate.
#[test]
fn pickup_damage_and_absorb_draw_nothing_without_a_tracked_hit() {
    let strings = strings();
    for name in [
        "PickupDamageTxt",
        "PickupAbsorbTxt",
        "PickupDamage",
        "PickupAbsorb",
    ] {
        let layout = Layout::from_xml(&format!(
            r#"<Screen><Text name="{name}"><Values idstring="MSC_DAMAGE" font="HUDSmall" x="0" y="0"/></Text></Screen>"#
        ));
        let label = &layout.labels[0];
        assert_eq!(
            text_for(label, &Readout::blank(), &strings, false, true, None, false),
            None,
            "{name} should draw nothing without a tracked recent-hit value"
        );
    }
}

/// **A second `POS` never draws.** `HUD_positions.xml` carries two widgets
/// sharing `idstring="IG_HUD_POS"` - `PositionTxt` (this module's own
/// `"PositionTxt"` arm, gated on `place_shown`) and `PositionTxt2`, deep
/// inside the head-to-head `PosTag0`-`7` cluster whose sprites
/// (`VoiceCom0`-`7`) are already excluded from every title's `ALWAYS_ON`. No
/// `talons-matched` frame shows a second `POS` label.
#[test]
fn position_txt2_never_draws_a_second_pos_caption() {
    let layout = Layout::from_xml(
        r#"<Screen><Text name="PositionTxt2"><Values idstring="IG_HUD_POS" font="HUDSmall" x="0" y="0"/></Text></Screen>"#,
    );
    let strings = strings();
    let readout = Readout {
        place: 3,
        ships: 8,
        ..Readout::blank()
    };
    let label = &layout.labels[0];
    assert_eq!(
        text_for(label, &readout, &strings, true, true, None, false),
        None
    );
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
