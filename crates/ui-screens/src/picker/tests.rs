use super::*;
use oag_ui::frontend::Draw;

/// The PSP's own grid, which the miniature XML is authored in.
const PSP: [f32; 2] = [480.0, 272.0];
use oag_gameplay::input::Input;
use oag_ui::menu::{Frame, Skin};
use oag_ui::screen::argb_to_rgba;

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
                emblem: None,
                icon: None,
                reversed: false,
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
            loyalty: None,
            models: Vec::new(),
            rating: None,
            variants,
            stats: Vec::new(),
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
    let layout = Layout::read(
        &screens,
        Kind::Track,
        &strings(),
        FaceScales::default(),
        PSP,
    )
    .unwrap();
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

/// Pure names its track screen `Track Selection`, not Pulse's `Track
/// Creation` - `Layout::read` has to find either, since which name a title
/// uses is a fact about the title, not a guess. See
/// `docs/formats/race-setup.md`.
#[test]
fn a_track_screen_named_track_selection_still_reads() {
    let xml = XML.replace("Track Creation", "Track Selection");
    let screens = Screens::from_xml(&xml);
    let layout = Layout::read(
        &screens,
        Kind::Track,
        &strings(),
        FaceScales::default(),
        PSP,
    )
    .unwrap();
    assert_eq!(layout.panel, [290.0, 26.0, 170.0, 200.0]);
}

/// `font="default"` text draws in the `Default` role at its own size when the
/// renderer loaded that face, and as the menu face scaled down when it did not.
#[test]
fn default_role_text_draws_in_its_own_face_when_there_is_one() {
    let screens = Screens::from_xml(XML);
    let draw = |native_default| {
        let layout = Layout::read(
            &screens,
            Kind::Track,
            &strings(),
            FaceScales {
                native_default,
                ..FaceScales::default()
            },
            PSP,
        )
        .unwrap();
        let mut picker = Picker::new(Kind::Track, entries(), Some("18_Track"), None);
        picker.tick(2.0);
        let skin = Skin::new(
            oag_pulse::FRONT_END.menu.unwrap(),
            oag_display::space::Space::PSP,
            22.0,
        );
        draw_list(
            &picker,
            &layout,
            &skin,
            &Frame::default(),
            None,
            false,
            &|_| None,
            &|text| text.len() as f32 * 8.0,
        )
        .body
    };
    let own = draw(true);
    assert!(
        own.iter().any(
            |d| matches!(d, Draw::FacedText { role: "Default", text, scale, .. }
            if text == "5178" && (*scale - 1.0).abs() < f32::EPSILON)
        ),
        "the stat value is in the Default face at native size: {own:?}"
    );
    let scaled = draw(false);
    assert!(
        !scaled.iter().any(|d| matches!(d, Draw::FacedText { .. })),
        "without the face, nothing asks for it"
    );
}

#[test]
fn the_body_names_the_selected_entry_and_counts_the_list() {
    let screens = Screens::from_xml(XML);
    let layout = Layout::read(
        &screens,
        Kind::Track,
        &strings(),
        FaceScales::default(),
        PSP,
    )
    .unwrap();
    let mut picker = Picker::new(Kind::Track, entries(), Some("18_Track"), None);
    // Well past every `LeftLayer`'s own `transition` (0.5s at most here), so
    // this checks the panel's settled content rather than its arrival -
    // that fade has its own tests, below.
    picker.tick(2.0);
    let skin = Skin::new(
        oag_pulse::FRONT_END.menu.unwrap(),
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
fn the_panel_fades_in_over_its_leftlayers_own_transition_and_the_title_bar_does_not() {
    // `Selection_Definition.xml`'s own shape (`XML`, above): the title bar's
    // `LeftLayer` carries `transition="0"`, the panel's carries `0.5` -
    // matching a live PPSSPP capture where the title bar is solid from the
    // first frame and the panel is still arriving at 0.5s
    // (`docs/ui/selection-screens.md`).
    let screens = Screens::from_xml(XML);
    let layout = Layout::read(
        &screens,
        Kind::Track,
        &strings(),
        FaceScales::default(),
        PSP,
    )
    .unwrap();
    let mut picker = Picker::new(Kind::Track, entries(), Some("18_Track"), None);
    let skin = Skin::new(
        oag_pulse::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    );
    let draw_at = |picker: &Picker| {
        draw_list(
            picker,
            &layout,
            &skin,
            &Frame::default(),
            None,
            false,
            &|_| None,
            &|text| text.len() as f32 * 8.0,
        )
        .body
    };
    let distance_alpha = |layers: &[Draw]| {
        layers
            .iter()
            .find_map(|draw| match draw {
                Draw::Text { text, color, .. } if text == "5178" => Some(color[3]),
                _ => None,
            })
            .expect("the distance row draws at every tick checked here")
    };
    // The screen chrome pushes its own title text - `draw_list` always adds
    // one - so check that one's alpha directly rather than through `body`.
    let title_alpha = |layers_full: &Layers| match &layers_full.chrome[..] {
        [Draw::Text { color, .. } | Draw::FacedText { color, .. }, ..] => color[3],
        other => panic!("{other:?}"),
    };
    assert_eq!(distance_alpha(&draw_at(&picker)), 0.0, "not yet enabled");
    let full = draw_list(
        &picker,
        &layout,
        &skin,
        &Frame::default(),
        None,
        false,
        &|_| None,
        &|text| text.len() as f32 * 8.0,
    );
    assert_eq!(title_alpha(&full), 1.0, "transition=\"0\" is instant");

    picker.tick(0.25);
    assert_eq!(
        distance_alpha(&draw_at(&picker)),
        0.5,
        "halfway through 0.5s"
    );

    picker.tick(0.25);
    assert_eq!(distance_alpha(&draw_at(&picker)), 1.0, "settled at 0.5s");

    picker.tick(10.0);
    assert_eq!(distance_alpha(&draw_at(&picker)), 1.0, "stays settled");
}

#[test]
fn a_long_name_wraps_into_the_three_line_block() {
    let screens = Screens::from_xml(XML);
    let layout = Layout::read(
        &screens,
        Kind::Track,
        &strings(),
        FaceScales::default(),
        PSP,
    )
    .unwrap();
    let lines = wrap_name("Talon's Junction White", &layout, &|text| {
        text.len() as f32 * 11.0
    });
    assert_eq!(lines, vec!["Talon's", "Junction", "White"]);
    let lines = wrap_name("Moa Therma White", &layout, &|text| {
        text.len() as f32 * 11.0
    });
    assert_eq!(lines, vec!["Moa Therma", "White"]);
}

#[test]
fn the_hex_grid_tiles_rather_than_reading_past_its_sprite() {
    let screens = Screens::from_xml(
        r#"<Screen name="Top"><Screen type="TrackSelection" name="Track Creation">
<Text idstring="RB_TRACK_SEL" font="Title" x="50" y="0"></Text>
<Text name="honey" String="1/1" x="135" y="213"></Text>
<Image name="Infogradient" x="290" y="26" width="170" height="200" Color1="0x2f000000" Color2="0x2f000000" Color3="0x2f000000" Color4="0x2f000000"></Image>
<Image name="Infohexgrid" x="290" y="26" width="170" height="60" U="0" V="0" TxtrWidth="340" TxtrHeight="120" color="0x7fffffff" src="Data\FE\Images\hex_bg.mip"></Image>
</Screen></Screen>"#,
    );
    let layout = Layout::read(
        &screens,
        Kind::Track,
        &strings(),
        FaceScales::default(),
        PSP,
    )
    .unwrap();
    let picker = Picker::new(Kind::Track, entries(), None, None);
    let skin = Skin::new(
        oag_pulse::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    );
    let tile = Placed {
        x: 0,
        y: 300,
        width: 32,
        height: 16,
        quad_extent: None,
        blend: None,
    };
    let layers = draw_list(
        &picker,
        &layout,
        &skin,
        &Frame::default(),
        None,
        false,
        &|_| Some(tile),
        &|text| text.len() as f32 * 8.0,
    );
    let tiled: Vec<&Draw> = layers
        .body
        .iter()
        .filter(|draw| matches!(draw, Draw::TiledSprite { .. }))
        .collect();
    assert_eq!(
        tiled,
        vec![&Draw::TiledSprite {
            rect: [290.0, 26.0, 170.0, 60.0],
            uv: [0.0, 300.0, 32.0, 16.0],
            repeat: [340.0 / 32.0, 120.0 / 16.0],
            color: argb_to_rgba(0x7fff_ffff),
        }],
        "340x120 of a 32x16 tile is the tile ten and a half times across"
    );
}

#[test]
fn the_ps2s_player_index_is_dropped_off_team_selections_widgets() {
    // The PS2 pressing suffixes every widget on `Team Selection` with the
    // player it belongs to; `Track Creation`'s are bare on both discs.
    let screens = Screens::from_xml(
        r#"<Screen name="Top"><Screen type="TeamSelection" name="Team Selection">
<Text idstring="RC_SHIPSEL" font="Title" x="50" y="0"></Text>
<Text name="honey0" String="1/1" x="180" y="351"></Text>
<Image name="Infogradient0" x="387" y="76" width="227" height="267" Color1="0x2f000000" Color2="0x2f000000" Color3="0x2f000000" Color4="0x2f000000"></Image>
<Image name="Speed Bar0" x="493" y="155" width="77" height="16" U="437" V="1" TxtrWidth="58" TxtrHeight="10" src="Data\FE\Images\pulse_assets.mip"></Image>
<Text name="Speed0" String="10" x="599" y="150"></Text>
<Text name="line bg10" String="x" x="1" y="1"></Text>
</Screen></Screen>"#,
    );
    let layout = Layout::read(
        &screens,
        Kind::Ship,
        &strings(),
        FaceScales::default(),
        [640.0, 448.0],
    )
    .unwrap();
    assert_eq!(
        layout.panel,
        [387.0, 76.0, 227.0, 267.0],
        "found under its bare name"
    );
    let names: Vec<&str> = layout
        .screen
        .texts
        .iter()
        .chain(std::iter::empty())
        .filter_map(|text| text.name.as_deref())
        .collect();
    assert!(names.contains(&"honey"), "{names:?}");
    assert!(names.contains(&"Speed"), "{names:?}");
    assert!(names.contains(&"line bg1"), "{names:?}");
    assert!(
        layout
            .screen
            .images
            .iter()
            .any(|image| image.name.as_deref() == Some("Speed Bar")),
        "{:?}",
        layout.screen.images
    );
    assert!((layout.scale[0] - 4.0 / 3.0).abs() < 1e-5);
    assert!(
        (layout.preview[0] - 10.0 * 4.0 / 3.0).abs() < 1e-3,
        "the PSP's 10 scaled"
    );
}

/// Pure's shape rather than Pulse's: one `<Menu>` widget and no named
/// `<Text>` at all, so the screen lists every entry instead of showing the
/// selected one.
const LISTING_XML: &str = r#"
<Screen name="Top">
<Screen type="TeamSelection" name="Team Selection">
<Text idstring="RB_TRACK_SEL" font="Title" x="21" y="20"></Text>
<Viewport>
<Menu name="Team" x="21" y="45" scale="2" color="0xff11acd0" align="left" font="Default" allocate="16"></Menu>
</Viewport>
</Screen>
</Screen>
"#;

fn listing_layout() -> Layout {
    Layout::read(
        &Screens::from_xml(LISTING_XML),
        Kind::Ship,
        &strings(),
        FaceScales::default(),
        PSP,
    )
    .expect("the listing screen reads")
}

/// One row per entry, stepping by a line of the widget's own font at the
/// widget's own scale - the widget states no pitch of its own.
#[test]
fn a_menu_widget_lists_every_entry_at_its_own_position_and_step() {
    let layout = listing_layout();
    let menu = layout.screen.menu.as_ref().expect("the Menu widget reads");
    assert_eq!((menu.x, menu.y, menu.scale), (21.0, 45.0, 2.0));

    let entries = vec![
        Entry {
            id: "Feisar".into(),
            label: "FEISAR".into(),
            details: Details::Ship {
                loyalty: None,
                models: Vec::new(),
                rating: None,
                variants: Vec::new(),
                stats: Vec::new(),
            },
        },
        Entry {
            id: "Qirex".into(),
            label: "QIREX".into(),
            details: Details::Ship {
                loyalty: None,
                models: Vec::new(),
                rating: None,
                variants: Vec::new(),
                stats: Vec::new(),
            },
        },
        Entry {
            id: "Auricom".into(),
            label: "AURICOM".into(),
            details: Details::Ship {
                loyalty: None,
                models: Vec::new(),
                rating: None,
                variants: Vec::new(),
                stats: Vec::new(),
            },
        },
    ];
    let picker = Picker::new(Kind::Ship, entries, Some("Qirex"), None);
    // A 20-unit line height and `FaceScales::default().default` of 1.0, so
    // the step is the widget's own `scale="2"`: 40.
    let skin = Skin::new(
        oag_pure::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        20.0,
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
    let rows: Vec<(String, f32, f32, f32, [f32; 4])> = layers
        .body
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text {
                text,
                x,
                y,
                scale,
                color,
                ..
            } => Some((text.clone(), *x, *y, *scale, *color)),
            _ => None,
        })
        // `body` also names the selected entry inside the panel, the way
        // Pulse's screens do; the list is what sits at the widget's own `x`.
        .filter(|row| (row.1 - 21.0).abs() < 1e-3)
        .collect();
    let step = 20.0 * FaceScales::default().default * 2.0;
    assert_eq!(rows.len(), 3, "{rows:?}");
    for (index, row) in rows.iter().enumerate() {
        assert_eq!(row.0, ["FEISAR", "QIREX", "AURICOM"][index]);
        assert!((row.1 - 21.0).abs() < 1e-3, "{row:?}");
        assert!(
            (row.2 - (45.0 + index as f32 * step)).abs() < 1e-3,
            "{row:?}"
        );
    }
    // The selected row takes the title's own measured ink, the rest its
    // measured unselected one - **not** the widget's `color` attribute,
    // which on Pure is `TextColor` and is the selected row's colour.
    assert_eq!(rows[1].4, oag_ui::screen::argb_to_rgba(0xFF16_AED1));
    assert_eq!(rows[0].4, oag_ui::screen::argb_to_rgba(0xFF88_D6E8));
    assert_ne!(rows[0].4, rows[1].4, "the selection has to be visible");
}

/// Pulse's own selection screens author no `<Menu>`, so the listing draws
/// nothing there and its single-entry display is untouched.
#[test]
fn pulses_selection_screens_list_nothing() {
    let screens = Screens::from_xml(XML);
    let layout = Layout::read(
        &screens,
        Kind::Track,
        &strings(),
        FaceScales::default(),
        PSP,
    )
    .expect("Track Creation reads");
    assert!(layout.screen.menu.is_none());
}

fn track_layout() -> Layout {
    Layout::read(
        &Screens::from_xml(XML),
        Kind::Track,
        &strings(),
        FaceScales::default(),
        PSP,
    )
    .expect("Track Creation reads")
}

fn pulse_skin() -> Skin {
    Skin::new(
        oag_pulse::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    )
}

fn click_at(at: (f32, f32)) -> oag_ui::pointer::Pointer {
    oag_ui::pointer::Pointer {
        at: Some(at),
        moved: true,
        clicked: true,
        ..Default::default()
    }
}

fn centre(rect: [f32; 4]) -> (f32, f32) {
    (rect[0] + rect[2] * 0.5, rect[1] + rect[3] * 0.5)
}

/// The `up arrow` image is a target where the disc puts it, at the size
/// the disc gives it, and a click on it steps back; the panel confirms;
/// the backdrop is nothing.
#[test]
fn pulses_arrows_step_and_its_panel_confirms() {
    let layout = track_layout();
    let mut picker = Picker::new(Kind::Track, entries(), Some("03_Track"), None);
    let targets = pointer::targets(&picker, &layout, &pulse_skin(), &|_| None);
    let up = targets
        .iter()
        .find(|target| target.what == pointer::What::Previous)
        .expect("the up arrow is a target");
    assert_eq!(up.rect, [117.0, 30.0, 17.0, 14.0]);
    assert_eq!(
        picker.pointer(&click_at(centre(up.rect)), &targets),
        vec![Event::Moved]
    );
    assert_eq!(picker.index(), 0);
    assert_eq!(
        picker.pointer(&click_at(centre(layout.panel)), &targets),
        vec![Event::Confirmed]
    );
    assert!(
        picker.pointer(&click_at((5.0, 260.0)), &targets).is_empty(),
        "a click on the backdrop is nothing"
    );
    let back = oag_ui::pointer::Pointer {
        back: true,
        ..Default::default()
    };
    assert_eq!(picker.pointer(&back, &targets), vec![Event::Back]);
}

/// An arrow with no authored size and no sheet to size it from is no
/// target rather than a zero-sized one.
#[test]
fn an_unsized_arrow_off_the_sheet_is_no_target() {
    let xml = XML.replace(r#" width="17" height="14""#, "");
    let layout = Layout::read(
        &Screens::from_xml(&xml),
        Kind::Track,
        &strings(),
        FaceScales::default(),
        PSP,
    )
    .unwrap();
    let picker = Picker::new(Kind::Track, entries(), None, None);
    let targets = pointer::targets(&picker, &layout, &pulse_skin(), &|_| None);
    assert!(
        targets
            .iter()
            .all(|target| target.what != pointer::What::Previous)
    );
    // With a sheet that places it, the texture's size is the target's.
    let placed = oag_ui::frontend::Placed {
        x: 0,
        y: 0,
        width: 17,
        height: 14,
        quad_extent: None,
        blend: None,
    };
    let targets = pointer::targets(&picker, &layout, &pulse_skin(), &|_| Some(placed));
    let up = targets
        .iter()
        .find(|target| target.what == pointer::What::Previous)
        .unwrap();
    assert_eq!(up.rect, [117.0, 30.0, 17.0, 14.0]);
}

/// Pure's listed rows: hovering selects, a click on the selected row
/// confirms, and a tap on another row selects it without confirming - a
/// tap being a move and a click in one tick.
#[test]
fn pures_listed_rows_select_on_hover_and_confirm_on_a_second_click() {
    let layout = listing_layout();
    let skin = Skin::new(
        oag_pure::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        20.0,
    );
    let entries: Vec<Entry> = ["Feisar", "Qirex", "Auricom"]
        .iter()
        .map(|id| Entry {
            id: (*id).into(),
            label: id.to_uppercase(),
            details: Details::Ship {
                loyalty: None,
                models: Vec::new(),
                rating: None,
                variants: Vec::new(),
                stats: Vec::new(),
            },
        })
        .collect();
    let mut picker = Picker::new(Kind::Ship, entries, Some("Qirex"), None);
    let targets = pointer::targets(&picker, &layout, &skin, &|_| None);
    let row = |index: usize| {
        targets
            .iter()
            .find(|target| target.what == pointer::What::Entry(index))
            .unwrap_or_else(|| panic!("row {index}"))
            .rect
    };
    // The rows step by the same pitch the drawing test measures.
    let step = 20.0 * FaceScales::default().default * 2.0;
    assert_eq!(row(0), [21.0, 45.0, row(0)[2], step]);
    assert!((row(2)[1] - (45.0 + 2.0 * step)).abs() < 1e-3);
    // With the face's ink measured, the same rows sit on the capitals:
    // centred on rows 7 to 16 of the cell at the drawing's scale.
    let mut inked = Skin::new(
        oag_pure::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        20.0,
    );
    inked.set_row_ink(Some(oag_ui::pointer::RowInk {
        top: 7.0,
        bottom: 16.0,
    }));
    let inked = pointer::targets(&picker, &layout, &inked, &|_| None);
    let scale = FaceScales::default().default * 2.0;
    let expect = 45.0 + 11.5 * scale - step * 0.5;
    let first = inked
        .iter()
        .find(|target| target.what == pointer::What::Entry(0))
        .unwrap();
    assert!((first.rect[1] - expect).abs() < 1e-3, "{}", first.rect[1]);

    let hover = oag_ui::pointer::Pointer {
        at: Some(centre(row(2))),
        moved: true,
        ..Default::default()
    };
    assert_eq!(picker.pointer(&hover, &targets), vec![Event::Moved]);
    assert_eq!(picker.index(), 2);
    // Moving within the same row is not another move.
    assert!(picker.pointer(&hover, &targets).is_empty());
    // A tap on a different row selects it and stops there.
    assert_eq!(
        picker.pointer(&click_at(centre(row(0))), &targets),
        vec![Event::Moved]
    );
    assert_eq!(picker.index(), 0);
    // A second tap on the same row confirms.
    assert_eq!(
        picker.pointer(&click_at(centre(row(0))), &targets),
        vec![Event::Confirmed]
    );
}

#[test]
fn the_wheel_steps_the_entry_and_livery_arrows_step_the_livery() {
    let team = |id: &str, variants: Vec<(String, String)>| Entry {
        id: id.to_string(),
        label: id.to_uppercase(),
        details: Details::Ship {
            loyalty: None,
            models: Vec::new(),
            rating: None,
            variants,
            stats: Vec::new(),
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
        None,
    );
    let xml = XML.replace(
        r#"<Text name="honey""#,
        r#"<Image name="skin right arrow" x="439" y="74" width="8" height="10" src="a.mip"></Image><Text name="honey""#,
    );
    let layout = Layout::read(
        &Screens::from_xml(&xml),
        Kind::Track,
        &strings(),
        FaceScales::default(),
        PSP,
    )
    .unwrap();
    let targets = pointer::targets(&picker, &layout, &pulse_skin(), &|_| None);
    let right = targets
        .iter()
        .find(|target| target.what == pointer::What::NextVariant)
        .expect("a two-livery team offers its arrow");
    assert_eq!(
        picker.pointer(&click_at(centre(right.rect)), &targets),
        vec![Event::VariantChanged]
    );
    assert_eq!(picker.variant().map(|(id, _)| id.as_str()), Some("_alt"));

    let wheel = oag_ui::pointer::Pointer {
        scroll: 3,
        ..Default::default()
    };
    assert_eq!(picker.pointer(&wheel, &targets), vec![Event::Moved]);
    assert_eq!(
        picker.index(),
        1,
        "one entry per tick, whatever the detents"
    );
    // And on the single-livery team the arrow is no longer offered.
    let targets = pointer::targets(&picker, &layout, &pulse_skin(), &|_| None);
    assert!(
        targets
            .iter()
            .all(|target| target.what != pointer::What::NextVariant)
    );
}

/// Wipeout HD/Fury's team screen keeps the model row when the team steps -
/// measured on RPCS3, 2026-09-29: `Right` from `concept1` lands on the next
/// team's `concept1`. Every other picker restarts the livery on a new entry.
#[test]
fn a_team_step_keeps_the_model_row_only_on_the_across_layout() {
    let team = |id: &str| Entry {
        id: id.to_string(),
        label: id.to_uppercase(),
        details: Details::Ship {
            loyalty: None,
            models: Vec::new(),
            rating: None,
            variants: vec![
                ("".into(), "HD".into()),
                ("_c1".into(), "Fury Concept".into()),
                ("_n1".into(), "Fury Nitro".into()),
            ],
            stats: Vec::new(),
        },
    };
    let entries = || vec![team("a"), team("b")];
    let mut across =
        Picker::new(Kind::Ship, entries(), Some("a"), Some("_c1")).with_entries_across();
    assert_eq!(across.step_entry(1), Some(Event::Moved));
    assert_eq!(across.variant().map(|(id, _)| id.as_str()), Some("_c1"));

    let mut list = Picker::new(Kind::Ship, entries(), Some("a"), Some("_c1"));
    assert_eq!(list.step_entry(1), Some(Event::Moved));
    assert_eq!(list.variant().map(|(id, _)| id.as_str()), Some(""));
}
