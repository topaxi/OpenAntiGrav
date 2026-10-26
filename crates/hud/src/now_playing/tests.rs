use super::*;

/// Shaped like the item in Pulse's `InGame_Definition.xml`. The numbers are the authored ones
/// (a layout's numbers are not shipped text); the bar and text names are the title's.
const LAYOUT: &str = r#"
<Screen name="InGame Screens">
<Viewport>
<Values x="130" height="272" width="150"></Values>
<item name="InGameMusicDisplay" OffsetX="130" OffsetY="247">
<Image name="InGameMusicBG">
<Values width="150" height="20" color="0x50000000"></Values>
</Image>
<Text name="InGameMusicText">
<Values idstring="IG_HUD_ARTIST" font="HUDSmall" scale="1.0" align="left" x="150" y="5" color="0xFFFFFFFF"></Values>
</Text>
</item>
</Viewport>
</Screen>
"#;

fn authored() -> NowPlaying {
    oag_pulse::NOW_PLAYING
}

fn display() -> Display {
    let authored = authored();
    Display::new(geometry(LAYOUT, &authored).expect("the item"), &authored)
}

#[test]
fn the_geometry_is_the_layouts_own() {
    let geometry = geometry(LAYOUT, &authored()).expect("the item");
    assert_eq!(geometry.bar, [130.0, 247.0, 150.0, 20.0]);
    assert_eq!(geometry.bar_argb, 0x5000_0000);
    assert_eq!(geometry.origin_x, 280.0);
    assert_eq!(geometry.text_y, 252.0);
    assert_eq!(geometry.clip, [130.0, 0.0, 150.0, 272.0]);
}

#[test]
fn a_layout_without_the_item_gives_no_geometry() {
    assert!(geometry("<Screen name=\"x\"></Screen>", &authored()).is_none());
}

/// `RaceManager_UpdateNowPlaying`: the text fades by 5 a step to 255 (52 steps to pass it), the
/// bar by `int(80 / 256 * 5)` = 1 a step, the text moves 1 a step, and it leaves once it is
/// `width` past the item's left edge.
#[test]
fn the_text_fades_in_scrolls_and_fades_out_by_the_executables_steps() {
    let mut display = display();
    assert!(display.marquee().is_none(), "nothing before a song");
    display.show("A - B".to_string(), 100.0);
    assert_eq!(display.marquee().expect("up").color[3], 0.0);

    for _ in 0..51 {
        display.step();
    }
    let full = display.marquee().expect("up");
    assert!((full.color[3] - 1.0).abs() < 1e-6, "{}", full.color[3]);
    assert!(
        (full.bar_color[3] - 51.0 / 255.0).abs() < 1e-6,
        "{}",
        full.bar_color[3]
    );
    assert_eq!(full.x, 130.0 + 150.0 - 51.0);

    // `x <= -width` is 250 steps from the start: 150 + 100.
    let mut steps = 51;
    while display.marquee().expect("up").color[3] >= 1.0 {
        display.step();
        steps += 1;
    }
    assert_eq!(steps, 250 + 1, "the first step of the fade-out");
    for _ in 0..60 {
        display.step();
    }
    assert!(display.marquee().is_none(), "gone after the fade");
}

#[test]
fn the_bar_and_the_text_reach_the_frame_and_the_text_is_clipped() {
    let mut display = display();
    display.show("Artist - Song".to_string(), 80.0);
    display.step();
    let marquee = display.marquee().expect("up");
    let mut frame = Frame::default();
    append(
        &mut frame,
        &marquee,
        [0.0, 0.0, 0.0, 1.0],
        &crate::sprite::Sheet::default(),
    );
    assert_eq!(frame.sprites.len(), 1);
    assert_eq!(frame.small_text.len(), 1);
    assert!(
        matches!(&frame.small_text[0], Draw::Text { text, .. } if text == "Artist - Song"),
        "{:?}",
        frame.small_text
    );
    assert_eq!(frame.spans.len(), 2, "a whole-screen span, then the clip");
    assert_eq!(frame.spans[1].clip, marquee.clip);
    assert_eq!(
        frame.spans[1].start[2], 0,
        "the text starts the clipped span"
    );
}

#[test]
fn placing_moves_the_bar_and_keeps_the_text_that_runs_off_the_screen() {
    let mut display = display();
    display.show("x".to_string(), 10.0);
    let marquee = display.marquee().expect("up");
    let at = crate::Placement {
        scale: 0.5,
        offset: [10.0, 20.0],
        clip: [-1.0e9, -1.0e9, 2.0e9, 2.0e9],
    };
    let placed = marquee.placed(&at);
    assert_eq!(placed.bar, [75.0, 143.5, 75.0, 10.0]);
    assert_eq!(placed.scale, 0.5);
}

/// Shaped like HD's: a textured bar, the viewport a child of the item.
const HD_LAYOUT: &str = r#"
<Screen name="InGame">
<Item name="InGameMusicDisplay" OffsetX="310" OffsetY="52" SDOffsetY="80">
<Image name="InGameMusicBG">
<Values width="230" height="48" U="145" V="653" TxtrWidth="230" TxtrHeight="48" src="Data\HUD\Textures\HUD_Components.gtf"></Values>
</Image>
<Viewport>
<Values x="19" height="1080" width="192"></Values>
<Text name="InGameMusicText">
<Values idstring="IG_HUD_ARTIST" font="HUDSmall" scale="0.8" align="left" x="230" y="10" color="0xFFFFFFFF"></Values>
</Text>
</Viewport>
</Item>
</Screen>
"#;

#[test]
fn a_textured_bar_with_a_viewport_inside_the_item_reads_too() {
    let geometry = geometry(HD_LAYOUT, &authored()).expect("the item");
    assert_eq!(geometry.bar, [310.0, 52.0, 230.0, 48.0]);
    assert_eq!(
        geometry.bar_argb, 0xFFFF_FFFF,
        "a textured bar authors no colour"
    );
    assert_eq!(
        geometry.bar_art,
        Some((
            r"Data\HUD\Textures\HUD_Components.gtf".to_string(),
            [145.0, 653.0, 230.0, 48.0]
        ))
    );
    assert_eq!(geometry.origin_x, 540.0, "just past the bar's right edge");
    assert_eq!(geometry.text_y, 62.0);
    assert_eq!(geometry.scale, 0.8);
    assert_eq!(
        geometry.clip,
        [329.0, 52.0, 192.0, 1080.0],
        "inside the bar"
    );
}

/// The bar of 255 climbs `int(255 / 256 * 5)` = 4 a step, so it is at 204 when the text is full.
#[test]
fn a_textured_bar_climbs_by_the_integer_step_of_its_own_alpha() {
    let authored = authored();
    let mut display = Display::new(geometry(HD_LAYOUT, &authored).expect("the item"), &authored);
    display.show("A - B".to_string(), 50.0);
    for _ in 0..51 {
        display.step();
    }
    let up = display.marquee().expect("up");
    assert!(
        (up.bar_color[3] - 204.0 / 255.0).abs() < 1e-6,
        "{}",
        up.bar_color[3]
    );
    assert_eq!(
        up.bar_color[..3],
        [0.0, 0.0, 0.0],
        "alpha << 24 is a black tint"
    );
}

#[test]
fn a_bar_texture_the_sheet_lacks_is_left_out_and_the_text_stays() {
    let authored = authored();
    let mut display = Display::new(geometry(HD_LAYOUT, &authored).expect("the item"), &authored);
    display.show("A - B".to_string(), 50.0);
    display.step();
    let mut frame = Frame::default();
    append(
        &mut frame,
        &display.marquee().expect("up"),
        [0.0, 0.0, 0.0, 1.0],
        &crate::sprite::Sheet::default(),
    );
    assert!(frame.sprites.is_empty(), "no stand-in bar");
    assert_eq!(frame.small_text.len(), 1);
}
