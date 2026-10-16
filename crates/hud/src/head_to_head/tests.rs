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
        .chain(&frame.default_text)
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
        ..HeadToHead::default()
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
            ..HeadToHead::default()
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
            ..HeadToHead::default()
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

fn state(stacked: bool, leads: bool, gap: f32) -> HeadToHead {
    HeadToHead {
        gap,
        player_leads: leads,
        own_team: Some("Qirex".into()),
        other_team: Some("Assegai".into()),
        stacked,
    }
}

/// A split's rows carry the craft's team name, the view's own on the row it is in: leading, it
/// is the first row; trailing, the second (`1st Qirex` / `2nd Assegai` live on PS2).
#[test]
fn the_rows_carry_the_team_names_own_craft_first_only_when_it_leads() {
    let rows = |leads| texts(&frame_for(Some(state(false, leads, 100.0))).0);
    let leading = rows(true);
    assert!(
        leading.iter().any(|r| r.0 == "IG_HUD_1ST Qirex"),
        "{leading:?}"
    );
    assert!(
        leading.iter().any(|r| r.0 == "IG_HUD_2ND Assegai"),
        "{leading:?}"
    );
    let trailing = rows(false);
    assert!(
        trailing.iter().any(|r| r.0 == "IG_HUD_1ST Assegai"),
        "{trailing:?}"
    );
    assert!(
        trailing.iter().any(|r| r.0 == "IG_HUD_2ND Qirex"),
        "{trailing:?}"
    );
}

/// Only the view's own row and the gap are coloured, the other row white, whichever cut.
#[test]
fn only_the_views_own_row_is_coloured() {
    let rows = texts(&frame_for(Some(state(false, false, 100.0))).0);
    let colour = |name: &str| rows.iter().find(|r| r.0 == name).unwrap().2;
    assert_eq!(colour("IG_HUD_1ST Assegai"), [1.0; 4]);
    assert_eq!(colour("IG_HUD_2ND Qirex"), argb_to_rgba(0xFFFF_3030));
}

/// The top and bottom cut clamps the half gap to `[40, 85]` and puts the leader row on the
/// bar's top (the authored place); the other cut keeps `[60, 180]`.
#[test]
fn the_top_and_bottom_cut_clamps_to_40_and_85_with_the_leader_on_the_bar() {
    for (gap, half) in [(0.0, 40.0), (60.0, 40.0), (120.0, 60.0), (900.0, 85.0)] {
        let (frame, _) = frame_for(Some(state(true, true, gap)));
        assert_eq!(fill_rect(&frame).unwrap()[3], half - 30.0, "gap {gap}");
    }
    // Both rows are in the `Default` font, and the leader row stands on the bar's top in either cut.
    let (frame, _) = frame_for(Some(state(true, true, 300.0)));
    assert!(
        frame
            .default_text
            .iter()
            .any(|d| matches!(d, Draw::Text { text, .. } if text.starts_with("IG_HUD_1ST"))),
        "the leader row is Default-font text"
    );
}

/// A layout that is not Pulse's (no `PositionOf`, no bar of its own: 2048's bare `Position`)
/// keeps its place and gains a coloured gap under it, not Pulse's rows.
#[test]
fn a_layout_without_pulses_widgets_keeps_its_place_and_gains_the_gap_under_it() {
    let layout = Layout::from_xml(
        r#"<Screen><Text name="Position"><Values scale="1.0" font="HUD" align="right" vertalign="top" x="945" y="30"/></Text></Screen>"#,
    );
    let strings = strings();
    let mut readout = Readout::blank();
    readout.place = 2;
    readout.ships = 2;
    readout.head_to_head = Some(HeadToHead {
        gap: 42.0,
        player_leads: false,
        ..HeadToHead::default()
    });
    let frame = draw_list(&context(&layout, &strings), &readout);
    let rows = texts(&frame);
    let place = rows.iter().find(|row| row.0 == "2/2").expect("the place");
    let (gap, y, colour) = rows
        .iter()
        .find(|row| row.0 == "+ 42m")
        .expect("the gap label");
    assert!(*y > place.1, "under the place: {rows:?}");
    assert_eq!(*colour, argb_to_rgba(0xFFFF_3030));
    assert_eq!(gap, "+ 42m");
}

const HD_XML: &str = r#"<Screen>
<Text name="PositionTxt"><Values idstring="IG_HUD_POS" font="HUDSmall" align="left" scale="0.8" x="70" y="0"/></Text>
<Text name="Position"><Values scale="1.0" font="HUD" align="centre" vertalign="middle" x="182" y="92"/></Text>
<Text name="DistanceApart"><Values string="" scale="0.6" font="HUD" align="right" vertalign="bottom" x="180" y="104"/></Text>
<Text name="Position Outof"><Values scale="0.6" font="HUD" align="left" vertalign="bottom" x="255" y="80"/></Text>
<Image name="HeadToHeadBar"><Values x="200" y="120" width="20" height="0" Color="0xB40048FF"/></Image>
</Screen>"#;

fn hd_frame(gap: f32, leads: bool) -> Frame {
    let layout = Layout::from_xml(HD_XML);
    let strings = strings();
    let mut readout = Readout::blank();
    readout.place = 2;
    readout.ships = 2;
    readout.head_to_head = Some(HeadToHead {
        gap,
        player_leads: leads,
        ..HeadToHead::default()
    });
    draw_list(&context(&layout, &strings), &readout)
}

/// `Hud_UpdatePositionCounter` (`0x00096088`): the second row sits at `clamp(2 * gap, 240, 720)`,
/// 4x Pulse's law, on HD's `DistanceApart` label, with no bar drawn and the place replaced by
/// the two rows.
#[test]
fn hd_second_row_follows_the_4x_clamp_and_the_gap_uses_distance_apart() {
    let bottoms = |gap| {
        let frame = hd_frame(gap, true);
        let rows = texts(&frame);
        assert!(fill_rect(&frame).is_none(), "HD's bar stays undrawn");
        assert!(rows.iter().all(|r| r.0 != "2/2"), "the place is replaced");
        assert!(rows.iter().any(|r| r.0 == "-" || r.0.starts_with('-')));
        let second = rows.iter().find(|r| r.0 == "IG_HUD_2ND").unwrap().1;
        let label = rows.iter().find(|r| r.0.ends_with('m')).unwrap().1;
        (second, label)
    };
    let (low, _) = bottoms(10.0);
    let (mid, mid_label) = bottoms(200.0);
    let (high, _) = bottoms(900.0);
    // Bottom-aligned at y = D: the top is D less one 0.6-scale line.
    let line = 0.6 * context_line();
    assert_eq!(low, 240.0 - line);
    assert_eq!(mid, 400.0 - line);
    assert_eq!(high, 720.0 - line);
    // The gap label sits at the midpoint of the bar's top (120) and the second row.
    assert_eq!(
        mid_label,
        (120.0 + 400.0) * 0.5 - 0.6 * context_line() / 2.0
    );
}

fn context_line() -> f32 {
    let layout = Layout::from_xml(HD_XML);
    let strings = strings();
    context(&layout, &strings).hud_line_height
}

#[test]
fn hd_colours_the_leader_green_and_the_trailer_red() {
    let leading = texts(&hd_frame(100.0, true));
    let gap = leading.iter().find(|r| r.0.ends_with('m')).unwrap();
    assert_eq!(gap.2, argb_to_rgba(0xFF30_FF30));
    let trailing = texts(&hd_frame(100.0, false));
    let gap = trailing.iter().find(|r| r.0.ends_with('m')).unwrap();
    assert_eq!(gap.2, argb_to_rgba(0xFFFF_3030));
}
