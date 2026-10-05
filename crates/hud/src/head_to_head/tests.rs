use super::*;
use crate::draw::draw_list;
use crate::tests::{context, strings};
use crate::{Draw, Layout};

/// The five widgets, verbatim from `Arcade_HUD.xml`'s `<Item OffsetX="445"
/// OffsetY="5">`.
const XML: &str = r#"<Screen>
<Variable global="HudColour2"><Values String="0xFF7DEFC0"/></Variable>
<Item OffsetX="445" OffsetY="5">
<Text name="PositionTxt"><Values idstring="IG_HUD_POS" font="HUDSmall" align="right" scale="1.0" x="15" y="0"/></Text>
<Text name="Position"><Values scale="1.0" font="HUD" align="right" vertalign="bottom" x="0" y="30"/></Text>
<Text name="PositionOf"><Values string="/" scale="0.6" font="HUD" align="left" vertalign="bottom" x="3" y="31"/></Text>
<Text name="Position Outof"><Values scale="0.6" font="HUD" align="left" vertalign="bottom" x="16" y="34"/></Text>
<Image name="HeadToHeadBar"><Values x="15" y="30" width="5" height="0" Color="FEConst->HudColour2"/></Image>
</Item></Screen>"#;

fn frame_for(state: Option<HeadToHead>) -> (Frame, Layout) {
    let layout = Layout::from_xml(XML);
    let strings = strings();
    let mut readout = Readout::blank();
    readout.place = 2;
    readout.ships = 2;
    readout.head_to_head = state;
    let frame = draw_list(&context(&layout, &strings), &readout);
    (frame, layout)
}

fn fill_rect(frame: &Frame) -> Option<[f32; 4]> {
    frame.sprites.iter().find_map(|draw| match draw {
        Draw::Fill { rect, .. } => Some(*rect),
        _ => None,
    })
}

fn texts(frame: &Frame) -> Vec<(String, f32, [f32; 4])> {
    frame
        .hud_text
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, y, color, .. } => Some((text.clone(), *y, *color)),
            _ => None,
        })
        .collect()
}

#[test]
fn the_bar_runs_from_its_authored_y_to_the_second_row() {
    let (frame, _) = frame_for(Some(HeadToHead {
        gap: 100.0,
        player_leads: true,
    }));
    // half = 60: the bar's local top is 30, so 30 px tall, from screen y 35.
    assert_eq!(fill_rect(&frame), Some([460.0, 35.0, 5.0, 30.0]));
    let rows = texts(&frame);
    let second = rows.iter().find(|row| row.0 == "IG_HUD_2ND").unwrap();
    assert_eq!(second.1, 5.0 + 60.0);
}

#[test]
fn the_gap_is_halved_and_clamped_to_60_and_180() {
    for (gap, half) in [(0.0, 60.0), (119.0, 60.0), (200.0, 100.0), (900.0, 180.0)] {
        let (frame, _) = frame_for(Some(HeadToHead {
            gap,
            player_leads: false,
        }));
        assert_eq!(fill_rect(&frame).unwrap()[3], half - 30.0, "gap {gap}");
    }
}

#[test]
fn leading_reads_minus_green_and_trailing_plus_red() {
    let label = |leads| {
        let (frame, _) = frame_for(Some(HeadToHead {
            gap: 42.0,
            player_leads: leads,
        }));
        texts(&frame)
            .into_iter()
            .find(|row| row.0.ends_with("m"))
            .unwrap()
    };
    let (text, _, colour) = label(true);
    assert_eq!(text, "- 42m");
    assert_eq!(colour, argb_to_rgba(0xFF30_FF30));
    let (text, _, colour) = label(false);
    assert_eq!(text, "+ 42m");
    assert_eq!(colour, argb_to_rgba(0xFFFF_3030));
}

#[test]
fn outside_head2head_nothing_here_draws_and_the_ordinary_place_stands() {
    let (frame, _) = frame_for(None);
    assert_eq!(fill_rect(&frame), None);
    let texts = texts(&frame);
    assert!(texts.iter().all(|row| !row.0.contains("IG_HUD_2ND")));
    assert!(texts.iter().any(|row| row.0 == "2"), "{texts:?}");
}
