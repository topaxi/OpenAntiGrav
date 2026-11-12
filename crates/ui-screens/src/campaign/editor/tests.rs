use super::*;
use oag_ui::screen::Screens;

const XML: &str = r#"
<Screen name="Top">
<Screen type="CellSelection" name="Cell Selection">
<LeftLayer transition="0.5">
<GridController name="Grid" OffsetX="35" OffsetY="40" focus="true">
<Item OffsetX="0" OffsetY="0">
<Image name="Outline_0_0" x="0" y="0" src="Data\FE\Images\hex_outline.mip"></Image>
<Image name="Outline_0_1" x="0" y="38" src="Data\FE\Images\hex_outline.mip"></Image>
</Item>
<Item OffsetX="30" OffsetY="19">
<Image name="Outline_1_0" x="0" y="0" src="Data\FE\Images\hex_outline.mip"></Image>
</Item>
<Image name="Selector" x="30" y="95" width="42" height="43" src="Data\FE\Images\pulse_assets.mip"></Image>
</GridController>
</LeftLayer>
<LeftLayer transition="0.5">
<Text name="Title" String="x" font="Menu" x="270" y="38" color="0xffffffff"></Text>
<Item OffsetX="260" OffsetY="60">
<Text name="Track Line" font="default" x="10" y="0" color="0xffffffff"></Text>
</Item>
<Item OffsetX="260" OffsetY="82">
<Text name="Line1 Title" String="l1 title" font="default" x="10" y="0" color="0xff34ACC2"></Text>
<Text name="Line1" String="l1" font="default" x="110" y="0" color="0xffffffff"></Text>
</Item>
<Item OffsetX="260" OffsetY="180">
<Image name="Target0 Image" x="10" y="1" width="14" height="14" src="Data\FE\Images\hex_filled.mip"></Image>
</Item>
<Item OffsetX="260" OffsetY="97">
<Text name="Line2 Title" String="l2 title" font="default" x="10" y="0" color="0xff34ACC2"></Text>
</Item>
</LeftLayer>
</Screen>
</Screen>
"#;

fn layout() -> Layout {
    Layout::read(
        &Screens::from_xml(XML),
        "Cell Selection",
        &oag_ui::language::StringTable::default(),
        crate::picker::FaceScales::default(),
        [480.0, 272.0],
    )
    .unwrap()
}

fn sprites(_: &str) -> Option<Placed> {
    Some(Placed {
        x: 0,
        y: 0,
        width: 32,
        height: 32,
        quad_extent: None,
        quad_centre: None,
        blend: None,
        scale: [1.0, 1.0],
    })
}

fn row(label: &str, value: &str) -> Row {
    Row {
        label: label.to_string(),
        value: value.to_string(),
    }
}

fn view<'a>(
    pulse: &'a SelectorPulse,
    cursor: (u32, u32),
    placed: &'a [(u32, u32)],
    fields: &'a [Row],
    actions: &'a [Row],
) -> View<'a> {
    View {
        title: "EDITOR",
        name: "MINE",
        heading: "0, 0",
        cursor,
        placed,
        fields,
        actions,
        selected_row: None,
        status: "",
        legend: ["Clear", "Name"],
        hint: "",
        pulse,
    }
}

fn centre(rect: [f32; 4]) -> (f32, f32) {
    (rect[0] + rect[2] * 0.5, rect[1] + rect[3] * 0.5)
}

fn pointer_at(at: (f32, f32), clicked: bool) -> Pointer {
    Pointer {
        at: Some(at),
        moved: true,
        clicked,
        ..Pointer::default()
    }
}

#[test]
fn every_slot_the_layout_draws_is_a_target_placed_or_not() {
    let layout = layout();
    let pulse = SelectorPulse::default();
    let actions = [row("NAME", "MINE"), row("SAVE", "")];
    let view = view(&pulse, (0, 0), &[(0, 0)], &[], &actions);
    let targets = targets(&view, &layout, &sprites, &|_| None);
    let hexes: Vec<(u32, u32)> = targets
        .iter()
        .filter_map(|t| match t.what {
            What::Hex(at) => Some(at),
            What::Row(_) => None,
        })
        .collect();
    assert_eq!(hexes, [(0, 0), (0, 1), (1, 0)]);
    assert_eq!(
        targets
            .iter()
            .filter(|t| matches!(t.what, What::Row(_)))
            .count(),
        2
    );
}

#[test]
fn a_click_on_an_empty_hex_selects_it_and_a_second_click_confirms() {
    let layout = layout();
    let pulse = SelectorPulse::default();
    let actions = [row("NAME", "MINE"), row("SAVE", "")];
    let at = |cursor| view(&pulse, cursor, &[], &[], &actions);
    let ts = targets(&at((0, 0)), &layout, &sprites, &|_| None);
    let empty = ts
        .iter()
        .find(|t| t.what == What::Hex((1, 0)))
        .map(|t| centre(t.rect))
        .unwrap();
    assert!(
        pointer(&at((0, 0)), &pointer_at(empty, false), &ts).is_empty(),
        "hovering a hex does not move the cursor, so crossing hexes on the way to the panel keeps the event"
    );
    assert_eq!(
        pointer(&at((0, 0)), &pointer_at(empty, true), &ts),
        [Event::Select((1, 0))]
    );
    assert_eq!(
        pointer(&at((1, 0)), &pointer_at(empty, true), &ts),
        [Event::Confirm]
    );
}

#[test]
fn a_click_on_a_row_names_it_and_the_secondary_button_backs_out() {
    let layout = layout();
    let pulse = SelectorPulse::default();
    let fields = [row("MODE", "RACE"), row("CIRCUIT", "X")];
    let actions = [row("NAME", "MINE"), row("SAVE", "")];
    let v = view(&pulse, (0, 0), &[(0, 0)], &fields, &actions);
    let ts = targets(&v, &layout, &sprites, &|_| None);
    let rows = |n: usize| {
        ts.iter()
            .find(|t| t.what == What::Row(n))
            .map(|t| centre(t.rect))
            .unwrap()
    };
    assert_eq!(
        pointer(&v, &pointer_at(rows(1), true), &ts),
        [Event::Field(1), Event::Row(1)]
    );
    assert_eq!(
        pointer(&v, &pointer_at(rows(3), true), &ts),
        [Event::Row(3)],
        "SAVE is the last row"
    );
    let back = Pointer {
        back: true,
        ..Pointer::default()
    };
    assert_eq!(pointer(&v, &back, &ts), [Event::Back]);
    assert!(pointer(&v, &Pointer::default(), &ts).is_empty());
}

#[test]
fn the_actions_stand_in_the_same_place_on_an_empty_slot_and_a_placed_one() {
    let layout = layout();
    let pulse = SelectorPulse::default();
    let fields: Vec<Row> = (0..FIELD_ROWS).map(|n| row("F", &n.to_string())).collect();
    let actions = [row("NAME", "MINE"), row("SAVE", "")];
    let placed = view(&pulse, (0, 0), &[(0, 0)], &fields, &actions);
    let empty = view(&pulse, (0, 0), &[], &[], &actions);
    let name = |v: &View| {
        let ts = targets(v, &layout, &sprites, &|_| None);
        let index = v.fields.len();
        ts.iter().find(|t| t.what == What::Row(index)).unwrap().rect
    };
    assert_eq!(name(&placed), name(&empty));
}

#[test]
fn a_placed_event_draws_a_fill_under_its_outline_and_an_empty_one_does_not() {
    let layout = layout();
    let pulse = SelectorPulse::default();
    let skin = Skin::new(
        oag_pulse::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    );
    let frame = Frame::default();
    let sprites_for = |src: &str| sprites(src);
    let count = |placed: &[(u32, u32)]| {
        let v = view(&pulse, (0, 0), placed, &[], &[]);
        draw_list(&v, &layout, &skin, &frame, None, false, &sprites_for, &[])
            .body
            .iter()
            .filter(
                |d| matches!(d, Draw::Sprite { color, .. } if *color == argb_to_rgba(PLACED_HEX)),
            )
            .count()
    };
    assert_eq!(count(&[]), 0);
    assert_eq!(count(&[(0, 0)]), 1, "one fill, under the one placed hex");
    assert_eq!(count(&[(0, 0), (0, 1)]), 2);
}
