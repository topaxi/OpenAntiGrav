//! [`super::ranked`]'s rule and the rows' text, pinned against the live frame.

use super::{KillTag, ranked};
use crate::Readout;

/// The live PPSSPP frame (`AG Systems 1`, `Feisar 1`, `Qirex 1`, `AAA 0`,
/// `Triakis 0`, `Goteki 45 0`, `Piranha 0`, `EG-X 0`, top to bottom) is the
/// column this rule produces from a grid array in which the equal-kill craft
/// sit in the order `EG-X`, `Piranha`, `Goteki 45`, `Triakis`, `AAA` and
/// `Qirex`, `Feisar`, `AG Systems`: earlier in the array is lower.
#[test]
fn equal_kills_stack_with_the_later_craft_on_top() {
    // Array order: 0 EG-X, 1 Piranha, 2 Goteki 45, 3 Triakis, 4 AAA,
    // 5 Qirex, 6 Feisar, 7 AG Systems.
    let kills = [0, 0, 0, 0, 0, 1, 1, 1];
    assert_eq!(ranked(&kills), [7, 6, 5, 4, 3, 2, 1, 0]);
}

#[test]
fn more_kills_rank_higher_whatever_the_array_order() {
    // Craft 2 leads on three, craft 0 and 3 tie on one, craft 1 has none.
    assert_eq!(ranked(&[1, 0, 3, 1]), [2, 3, 0, 1]);
}

#[test]
fn a_lone_craft_is_row_zero() {
    assert_eq!(ranked(&[4]), [0]);
    assert!(ranked(&[]).is_empty());
}

fn strings() -> oag_ui::language::StringTable {
    oag_ui::language::StringTable::from_xml(
        r#"<Screen name="Top"><Entry ID="Feisar" String="Feisar"/><Entry ID="AG_Systems" String="AG Systems"/></Screen>"#,
    )
}

/// The player's row has two spaces before the count and full size; an
/// opponent's has one and `0.8` of it; a row past the field has neither.
#[test]
fn the_rows_read_name_then_kills_and_shrink_the_opponents() {
    let readout = Readout {
        kill_tags: vec![
            KillTag {
                team: "AG_Systems".into(),
                kills: 1,
                player: false,
            },
            KillTag {
                team: "Feisar".into(),
                kills: 0,
                player: true,
            },
        ],
        ..Readout::blank()
    };
    let strings = strings();
    assert_eq!(
        super::text(&readout, "PosTag0", &strings).as_deref(),
        Some("AG Systems 1")
    );
    assert_eq!(
        super::text(&readout, "PosTag1", &strings).as_deref(),
        Some("Feisar  0")
    );
    assert_eq!(super::text(&readout, "PosTag2", &strings), None);
    assert_eq!(super::text(&readout, "Position", &strings), None);
    assert_eq!(super::scale(&readout, "PosTag0"), Some(0.8));
    assert_eq!(super::scale(&readout, "PosTag1"), Some(1.0));
    assert_eq!(super::scale(&readout, "PosTag5"), None);
}

const COLUMN: &str = r#"
<Screen>
<Screen name="HUD">
<Variable global="HudBGColour"><Values String="0x40000000"></Values></Variable>
<Item OffsetX="460" OffsetY="5">
<Text name="KillsText">
<Values idstring="IG_HUD_KILLS" font="HUDSmall" align="right" x="0" y="0" CalcBlur="1"/>
</Text>
<Text name="PosTag0">
<Values x="0" y="20" scale="1.0" font="Default" align="right" vertalign="centre" CalcBlur="1"/>
</Text>
<Text name="PosTag1">
<Values x="0" y="40" scale="1.0" font="Default" align="right" vertalign="centre" CalcBlur="1"/>
</Text>
</Item>
</Screen>
</Screen>"#;

fn column_readout() -> Readout {
    Readout {
        kill_target: 5,
        kill_tags: vec![
            KillTag {
                team: "AG_Systems".into(),
                kills: 1,
                player: false,
            },
            KillTag {
                team: "Feisar".into(),
                kills: 0,
                player: true,
            },
        ],
        ..Readout::blank()
    }
}

/// Through the whole draw list: the rows land in the `Default` bucket, the
/// player's at full size and the opponent's at `0.8` of it, under a `KILLS (5)`
/// header - and a title that has not had the column measured draws none of it.
#[test]
fn the_column_draws_in_the_default_bucket_and_only_where_the_title_has_it() {
    use crate::Draw;
    let layout = crate::Layout::from_xml(COLUMN);
    let strings = strings();
    let frame = crate::draw_list(&crate::tests::context(&layout, &strings), &column_readout());
    let scales: Vec<(String, f32)> = frame
        .default_text
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, scale, .. } => Some((text.clone(), *scale)),
            _ => None,
        })
        .collect();
    assert_eq!(
        scales,
        [
            ("AG Systems 1".to_string(), 0.8),
            ("Feisar  0".to_string(), 1.0)
        ]
    );
    let header: Vec<&Draw> = frame.small_text.iter().collect();
    assert!(
        matches!(header.as_slice(), [Draw::Text { text, .. }] if text == "IG_HUD_KILLS (5)"),
        "{header:?}"
    );

    let other = oag_title::HudArt {
        kill_column: false,
        message_slots: false,
        ..*oag_pulse::hud::ART
    };
    let mut cx = crate::tests::context(&layout, &strings);
    cx.art = &other;
    let frame = crate::draw_list(&cx, &column_readout());
    assert!(frame.default_text.is_empty(), "{:?}", frame.default_text);
    let header: Vec<&Draw> = frame.small_text.iter().collect();
    assert!(
        matches!(header.as_slice(), [Draw::Text { text, .. }] if text == "IG_HUD_KILLS"),
        "{header:?}"
    );
}
