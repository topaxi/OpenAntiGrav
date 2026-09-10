use super::*;
use crate::frontend::Draw;
use crate::menu::{Frame, Skin};
use oag_gameplay::input::Input;

/// A `Selection_Definition.xml` in miniature: the shape the disc authors,
/// with the same container nesting and the same widget names, and none of
/// its strings.
const XML: &str = r#"
<Screen name="Top">
<Screen type="TrackSelection" name="Track Creation">
<LeftLayer transition="0">
<Text idstring="RB_TRACK_SEL" font="Title" x="50" y="0"></Text>
</LeftLayer>
<LeftLayer transition="0.5">
<Image name="up arrow" x="117" y="30" U="242" V="71" width="17" height="14" TxtrWidth="17" TxtrHeight="14" color="0xffffffff" src="Data\FE\Images\pulse_assets.mip"></Image>
<Text name="honey" String="1/1" x="135" y="213" color="0xffffffff"></Text>
</LeftLayer>
<LeftLayer transition="0.5" OffsetX="290" OffsetY="25">
<Image width="85" height="1" Color1="0x00ffffff" Color2="0x00ffffff" Color3="0xffffffff" Color4="0xffffffff"></Image>
<Image name="Infogradient" y="1" width="170" height="200" Color1="0x2f000000" Color2="0x2f000000" Color3="0x2f000000" Color4="0x2f000000"></Image>
<Item name="line bg1" OffsetY="142">
<Image name="linebg1l" width="85" height="14" Color1="0x00ffffff" Color2="0x00ffffff" Color3="0x3fffffff" Color4="0x3fffffff"></Image>
</Item>
<Item OffsetX="14">
<Text name="Info Track 1.1" String="Moa Therma" font="Menu" y="18" color="0xff34acc2"></Text>
<Text name="Info Track 2.1" String="Moa Therma" font="Menu" y="10" color="0xff34acc2"></Text>
<Text name="Info Track 2.2" String="White" font="Menu" y="25" color="0xff34acc2"></Text>
<Item OffsetY="141">
<Text name="Info1" String="2500" font="default" x="95" color="0xffffffff"></Text>
<Text name="Info1 Title" idstring="IG_HUD_DISTANCE" font="default" color="0xff34ACC2"></Text>
</Item>
</Item>
</LeftLayer>
</Screen>
</Screen>
"#;

fn strings() -> StringTable {
    StringTable::from_xml(
        r#"<Strings><Entry ID="RB_TRACK_SEL" String="TRACK SELECT"></Entry><Entry ID="IG_HUD_DISTANCE" String="Distance(m)"></Entry></Strings>"#,
    )
}

fn entries() -> Vec<Entry> {
    ["16_Track", "03_Track", "18_Track"]
        .iter()
        .map(|id| Entry {
            id: (*id).to_string(),
            label: format!("Circuit {id}"),
            details: Details::Track {
                info: ["5178".into(), "-.--.--".into(), "-.--.--".into()],
            },
        })
        .collect()
}

/// One press: a frame with the button up, then one with it down, so the
/// edge fires even when the same button was pressed the tick before.
fn press(input: &mut Input, button: Button) {
    input.begin_frame(0);
    input.begin_frame(1 << button as u32);
}

#[test]
fn the_list_wraps_both_ways_and_confirms() {
    let mut picker = Picker::new(Kind::Track, entries(), Some("03_Track"), None);
    assert_eq!(picker.index(), 1);
    let mut input = Input::new();
    press(&mut input, Button::Down);
    assert_eq!(picker.update(&mut input), vec![Event::Moved]);
    press(&mut input, Button::Down);
    picker.update(&mut input);
    assert_eq!(
        picker.index(),
        0,
        "down off the last entry wraps to the first"
    );
    press(&mut input, Button::Up);
    picker.update(&mut input);
    assert_eq!(picker.index(), 2, "up off the first wraps to the last");
    press(&mut input, Button::Cross);
    assert_eq!(picker.update(&mut input), vec![Event::Confirmed]);
    press(&mut input, Button::Circle);
    assert_eq!(picker.update(&mut input), vec![Event::Back]);
}

#[test]
fn left_and_right_move_the_livery_only_when_there_is_one_to_move_to() {
    let team = |id: &str, variants: Vec<(String, String)>| Entry {
        id: id.to_string(),
        label: id.to_uppercase(),
        details: Details::Ship {
            rating: None,
            variants,
        },
    };
    let mut picker = Picker::new(
        Kind::Ship,
        vec![
            team(
                "Assegai",
                vec![
                    ("".into(), "Classic".into()),
                    ("_alt".into(), "Alternative".into()),
                ],
            ),
            team("Qirex", Vec::new()),
        ],
        None,
        Some("_alt"),
    );
    assert_eq!(picker.variant().map(|(id, _)| id.as_str()), Some("_alt"));
    let mut input = Input::new();
    press(&mut input, Button::Right);
    assert_eq!(picker.update(&mut input), vec![Event::VariantChanged]);
    assert_eq!(picker.variant().map(|(id, _)| id.as_str()), Some(""));
    press(&mut input, Button::Down);
    picker.update(&mut input);
    press(&mut input, Button::Right);
    assert!(
        picker.update(&mut input).is_empty(),
        "a team with no liveries ignores right"
    );
    assert!(picker.variant().is_none());
}

#[test]
fn the_layout_sums_offsets_and_resolves_strings() {
    let screens = Screens::from_xml(XML);
    let layout = Layout::read(&screens, Kind::Track, &strings(), FaceScales::default()).unwrap();
    assert_eq!(layout.title, "TRACK SELECT");
    assert_eq!(layout.panel, [290.0, 26.0, 170.0, 200.0]);
    let distance = layout
        .screen
        .texts
        .iter()
        .find(|text| text.name.as_deref() == Some("Info1 Title"))
        .unwrap();
    assert_eq!(
        (distance.x, distance.y),
        (304.0, 166.0),
        "290+14 across, 25+141 down"
    );
    assert_eq!(distance.string.as_deref(), Some("Distance(m)"));
    let rule = layout
        .screen
        .fills
        .iter()
        .find(|fill| fill.gradient.is_some() && fill.height == Some(14.0))
        .unwrap();
    assert_eq!((rule.x, rule.y), (290.0, 167.0));
    assert!(
        layout.preview[1] > layout.panel[1]
            && layout.preview[1] + layout.preview[3] <= rule.y + 0.5
    );
}

#[test]
fn the_body_names_the_selected_entry_and_counts_the_list() {
    let screens = Screens::from_xml(XML);
    let layout = Layout::read(&screens, Kind::Track, &strings(), FaceScales::default()).unwrap();
    let picker = Picker::new(Kind::Track, entries(), Some("18_Track"), None);
    let skin = Skin::new(
        oag_pulse::FRONT_END.menu,
        oag_display::space::Space::PSP,
        22.0,
    );
    let layers = draw_list(
        &picker,
        &layout,
        &skin,
        &Frame::default(),
        None,
        false,
        &|_| None,
        &|text| text.len() as f32 * 8.0,
    );
    let texts: Vec<(String, f32, f32)> = layers
        .body
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, x, y, .. } => Some((text.clone(), *x, *y)),
            _ => None,
        })
        .collect();
    assert!(
        texts.contains(&("3 / 3".to_string(), 135.0, 213.0)),
        "{texts:?}"
    );
    assert!(
        texts.contains(&("Circuit 18_Track".to_string(), 304.0, 43.0)),
        "one line, so the 1.1 block: {texts:?}"
    );
    assert!(
        !texts.iter().any(|(text, _, _)| text == "White"),
        "the 2.x block stays unused: {texts:?}"
    );
    assert!(
        texts.contains(&("5178".to_string(), 399.0, 166.0)),
        "{texts:?}"
    );
    assert!(
        layers
            .body
            .iter()
            .any(|draw| matches!(draw, Draw::GradientFill { rect, left, right } if rect[1] == 167.0 && left[3] == 0.0 && right[3] > 0.0)),
        "the rule fades in from the left"
    );
    assert!(
        !layers
            .body
            .iter()
            .any(|draw| matches!(draw, Draw::Sprite { .. })),
        "no sheet, no arrow: an image with no placement is left out, not boxed"
    );
}

#[test]
fn a_long_name_wraps_into_the_three_line_block() {
    let screens = Screens::from_xml(XML);
    let layout = Layout::read(&screens, Kind::Track, &strings(), FaceScales::default()).unwrap();
    let lines = wrap_name("Talon's Junction White", &layout, &|text| {
        text.len() as f32 * 11.0
    });
    assert_eq!(lines, vec!["Talon's", "Junction", "White"]);
    let lines = wrap_name("Moa Therma White", &layout, &|text| {
        text.len() as f32 * 11.0
    });
    assert_eq!(lines, vec!["Moa Therma", "White"]);
}
