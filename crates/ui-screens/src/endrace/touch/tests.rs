use super::*;

/// A cut of `EndRace_Definition.xml` with the same shape: a shell holding the
/// panel and the tiles, and a summary whose boxes carry their own offsets and
/// whose code-filled texts carry development placeholders.
const XML: &str = r#"
<Screen name="EndRace">
  <Image name="EndRaceTopBox" OffsetX="180" OffsetY="15">
    <Values x="0" y="0" width="600" height="58" color="0xbeffffff"></Values>
  </Image>
  <Image name="EndRaceBackground" OffsetX="180" OffsetY="77">
    <Values x="0" y="0" width="600" height="347" src="Data\FE\NewImages\endrace_bg.gtf"></Values>
  </Image>
  <Image name="EndRaceButtons">
    <TouchButton name="RestartRaceTouchButton"><Values redirect="Retry" x="483" y="430" width="145" height="96" src="Data/FE/NewImages/Icon_Restart.gtf"></Values></TouchButton>
    <TouchButton name="QuitTouchButton"><Values redirect="X" x="635" y="430" width="145" height="96" src="Data/FE/NewImages/Icon_Exit.gtf"></Values></TouchButton>
    <TouchButton name="QuitTouchButtonPass"><Values redirect="X" x="635" y="430" width="145" height="96" src="Data/FE/NewImages/Icon_Tick.gtf"></Values></TouchButton>
  </Image>
  <Screen name="RaceSummary">
    <Text name="RaceSummaryTitle" OffsetX="180" OffsetY="15">
      <Values idstring="FE_ENDRACE_SUMMARY" x="300" y="25" font="NEOSANS_BOLD" align="centre" vertalign="middle" color="0xff19295d"></Values>
      <Image name="MessageBox" OffsetX="0" OffsetY="62">
        <Values x="0" y="0" width="600" height="64" color="0xff00ff00"></Values>
        <Text name="Message"><Values idstring="ER_CONGRAT" x="300" y="28" align="centre" color="0xff19295d"></Values></Text>
      </Image>
      <Image name="ResultBox" OffsetX="0" OffsetY="176">
        <Values x="0" y="0" width="400" height="122" color="0xffe1e4eb"></Values>
        <Text name="RaceResultText"><Values string="" x="200" y="60" align="centre" color="0xff19295d"></Values></Text>
      </Image>
      <Text name="SpeedResultText"><Values string="" x="300" y="237" align="centre" color="0xff19295d"></Values></Text>
      <Image name="RaceXPBox" OffsetX="0" OffsetY="302">
        <Values x="0" y="0" width="276" height="42" color="0xffe1e4eb"></Values>
        <Text name="RaceXP"><Values string="XP_TEST" x="6" y="20" color="0xff19295d"></Values></Text>
      </Image>
      <Image name="TotalXPBox" OffsetX="0" OffsetY="348">
        <Values x="0" y="0" width="600" height="64" color="0xff00ff00"></Values>
        <Text name="TotalXP"><Values string="TOTAL_4325_XP_TEST" x="262" y="28" align="centre" color="0xff19295d"></Values></Text>
      </Image>
      <Image name="PostRaceMedal" OffsetX="500" OffsetY="237">
        <Values x="0" y="0" centred="true" width="128" height="128" U="0" V="0" txtrwidth="128" txtrheight="128" src="Data\FE\NewImages\Post_Race_Medals.gtf"></Values>
        <Text name="PostRaceMedalText"><Values string="PASS_TEST" x="0" y="60" align="centre" color="0xff19295d"></Values></Text>
      </Image>
    </Text>
  </Screen>
</Screen>"#;

const GLOBALS: &[(&str, &str)] = &[
    ("Blue2048", "0xff19295d"),
    ("Orange2048", "0xffdd580b"),
    ("White2048", "0xffe1e4eb"),
    ("Pass2048", "0xff018400"),
    ("ElitePass2048", "0xfffef502"),
];

fn layouts() -> Layouts {
    let screens = Screens::from_xml_folding_fill_offsets(XML, GLOBALS);
    Layouts::read(
        &screens,
        &StringTable::default(),
        FaceScales::default(),
        AUTHORED_GRID,
    )
    .expect("the fixture carries the shell and the first page")
}

fn model(tone: Tone) -> Summary {
    Summary {
        title: Some("RACE SUMMARY".into()),
        message: Some("CONGRATULATIONS".into()),
        objective: Some("FINISH IN 3RD".into()),
        result: Some("2ND".into()),
        speed_shape: false,
        tone,
        medal_label: Some("PASS".into()),
        rows: Vec::new(),
    }
}

fn sprites(src: &str) -> Option<Placed> {
    let (width, height) = if src.contains("Post_Race_Medals") {
        (512, 128)
    } else {
        (100, 100)
    };
    Some(Placed {
        x: 0,
        y: 0,
        width,
        height,
        quad_extent: None,
        blend: None,
    })
}

fn draws(model: &Summary) -> Vec<Draw> {
    draw_list(
        model,
        Page::Summary,
        Button::Exit,
        &layouts(),
        &sprites,
        &[],
        37.0,
    )
}

fn texts(list: &[Draw]) -> Vec<&str> {
    list.iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

fn fill_of(list: &[Draw], at: [f32; 4]) -> Option<[f32; 4]> {
    list.iter().find_map(|draw| match draw {
        Draw::Fill { rect, color } if *rect == at => Some(*color),
        _ => None,
    })
}

/// The boxes author their own `OffsetY`; read without folding it they would all
/// sit at the container's, 62 and 176 units too high.
#[test]
fn a_boxs_own_offset_places_it() {
    let list = draws(&model(Tone::Pass));
    assert!(
        fill_of(&list, [180.0, 77.0, 600.0, 64.0]).is_some(),
        "MessageBox"
    );
    assert!(
        fill_of(&list, [180.0, 191.0, 400.0, 122.0]).is_some(),
        "ResultBox"
    );
}

/// The verdict paints both coloured bars, and the colours are the disc's.
#[test]
fn a_verdict_paints_the_message_and_total_bars() {
    for (tone, want) in [
        (Tone::Pass, 0xff01_8400),
        (Tone::Elite, 0xfffe_f502),
        (Tone::Fail, 0xffcd_0102),
        (Tone::Unjudged, 0xff52_5e84),
    ] {
        let list = draws(&model(tone));
        let want = argb_to_rgba(want);
        assert_eq!(
            fill_of(&list, [180.0, 77.0, 600.0, 64.0]),
            Some(want),
            "{tone:?}"
        );
        assert_eq!(
            fill_of(&list, [180.0, 363.0, 600.0, 64.0]),
            Some(want),
            "{tone:?}"
        );
    }
}

/// Development text in code-filled widgets never reaches the screen: not the
/// XML's, and not as a stand-in for a number this build does not keep.
#[test]
fn placeholder_text_never_draws() {
    let list = draws(&model(Tone::Pass));
    let shown = texts(&list);
    for leaked in ["TOTAL_4325_XP_TEST", "XP_TEST", "PASS_TEST"] {
        assert!(!shown.contains(&leaked), "{leaked} drew: {shown:?}");
    }
    assert!(shown.contains(&"CONGRATULATIONS"));
    assert!(shown.contains(&"2ND"));
}

/// The medal's cell is the verdict's, and no verdict draws no medal.
#[test]
fn the_medal_cell_follows_the_verdict() {
    let cell = |tone| {
        draws(&model(tone)).iter().find_map(|draw| match draw {
            Draw::Sprite { uv, .. } if uv[2] == 128.0 && uv[3] == 128.0 => Some(uv[0]),
            _ => None,
        })
    };
    assert_eq!(cell(Tone::Pass), Some(128.0));
    assert_eq!(cell(Tone::Elite), Some(256.0));
    assert_eq!(cell(Tone::Fail), Some(0.0));
    assert_eq!(cell(Tone::Unjudged), None);
}

/// A speed lap sits its result bare on the panel: the box and its text go.
#[test]
fn a_speed_lap_has_no_result_box() {
    let mut speed = model(Tone::Unjudged);
    speed.speed_shape = true;
    let list = draws(&speed);
    assert!(fill_of(&list, [180.0, 191.0, 400.0, 122.0]).is_none());
    assert_eq!(texts(&list).iter().filter(|t| **t == "2ND").count(), 1);
}

/// The exit tile is the tick once the event is passed and the door before.
#[test]
fn the_exit_tile_swaps_to_the_tick_on_a_pass() {
    let layouts = layouts();
    let srcs = |tone| {
        layouts
            .tiles(&model(tone))
            .into_iter()
            .map(|(_, _, src)| src.map(str::to_string))
            .collect::<Vec<_>>()
    };
    assert!(
        srcs(Tone::Pass)[1]
            .as_deref()
            .unwrap()
            .ends_with("Icon_Tick.gtf")
    );
    assert!(
        srcs(Tone::Fail)[1]
            .as_deref()
            .unwrap()
            .ends_with("Icon_Exit.gtf")
    );
}

#[test]
fn a_click_on_a_tile_or_the_panel_is_a_hit_and_off_them_is_none() {
    let layouts = layouts();
    let model = model(Tone::Pass);
    let click = |x: f32, y: f32| Pointer {
        at: Some((x, y)),
        clicked: true,
        ..Pointer::default()
    };
    assert_eq!(
        pointer_hit(&layouts, &model, &click(500.0, 450.0)),
        Some(Hit::Tile(Button::Restart))
    );
    assert_eq!(
        pointer_hit(&layouts, &model, &click(700.0, 450.0)),
        Some(Hit::Tile(Button::Exit))
    );
    assert_eq!(
        pointer_hit(&layouts, &model, &click(400.0, 200.0)),
        Some(Hit::Panel)
    );
    assert_eq!(pointer_hit(&layouts, &model, &click(10.0, 10.0)), None);
    assert_eq!(
        pointer_hit(
            &layouts,
            &model,
            &Pointer {
                at: Some((500.0, 450.0)),
                ..Pointer::default()
            }
        ),
        None,
        "a hover is not a tap"
    );
}
