use super::*;

const BAR: &str = r#"<Screen><Item OffsetX="445" OffsetY="5">
<Text name="Position"><Values scale="1.0" font="HUD" align="right" x="0" y="30"/></Text>
<Image name="HeadToHeadBar"><Values x="15" y="30" width="5" height="0" Color="0xB40048FF"/></Image>
</Item></Screen>"#;

const PLACE_ONLY: &str = r#"<Screen><Item OffsetX="445" OffsetY="5">
<Text name="Position"><Values scale="1.0" font="HUD" align="right" x="0" y="30"/></Text>
</Item></Screen>"#;

const NEITHER: &str = r#"<Screen><Item OffsetX="445" OffsetY="5">
<Text name="Lap"><Values scale="1.0" font="HUD" align="right" x="0" y="30"/></Text>
</Item></Screen>"#;

#[test]
fn a_pinned_head_to_head_says_when_the_layout_cannot_draw_its_bar() {
    let gap = |xml: &str| head_to_head_pin_gap(Some(&oag_hud::Layout::from_xml(xml)));
    assert_eq!(gap(BAR), None, "the bar is there: nothing to say");
    assert!(gap(PLACE_ONLY).is_some_and(|why| why.contains("stand-in")));
    assert!(gap(NEITHER).is_some_and(|why| why.contains("nothing is drawn")));
    assert!(head_to_head_pin_gap(None).is_some_and(|why| why.contains("no HUD layout")));
}
