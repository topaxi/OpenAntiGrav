use super::super::Screens;

const XML: &str = r#"
<Screen name="Top">
  <Screen name="Cell">
    <Image name="Emblem" OffsetX="750" OffsetY="315">
      <Values x="4" y="2" width="96" height="96"></Values>
      <Bracket>
        <Values corner="true" x="0" y="0" Width="100" Height="96" Colour="0xff969696"></Values>
      </Bracket>
    </Image>
    <Image name="Bar"><Values x="190" y="290" width="1500" height="3"></Values></Image>
    <Image name="Gone"><Values x="0" y="0"></Values></Image>
  </Screen>
</Screen>
"#;

#[test]
fn a_rect_with_no_picture_and_no_colour_is_a_slot_with_its_containers_offsets() {
    let screens = Screens::from_xml(XML);
    let screen = screens.by_name("Cell").expect("the screen");
    let slots: Vec<_> = screen
        .slots
        .iter()
        .map(|slot| {
            (
                slot.name.as_deref(),
                slot.x,
                slot.y,
                slot.width,
                slot.height,
            )
        })
        .collect();
    assert_eq!(
        slots,
        [
            (Some("Emblem"), 754.0, 317.0, Some(96.0), Some(96.0)),
            (Some("Bar"), 190.0, 290.0, Some(1500.0), Some(3.0)),
        ],
        "an image with no size at all is not a slot"
    );
}

#[test]
fn a_bracket_keeps_its_rect_inside_the_image_that_holds_it() {
    let screens = Screens::from_xml(XML);
    let screen = screens.by_name("Cell").expect("the screen");
    assert_eq!(screen.brackets.len(), 1);
    let bracket = &screen.brackets[0];
    assert_eq!(
        (bracket.x, bracket.y, bracket.width, bracket.height),
        (750.0, 315.0, 100.0, 96.0)
    );
    assert_eq!(bracket.color, 0xff96_9696);
    assert!(bracket.corner);
}
