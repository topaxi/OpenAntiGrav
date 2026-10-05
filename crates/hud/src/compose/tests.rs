use std::collections::HashMap;

use super::*;

/// A reader over a handful of in-memory files, spelled as the disc spells them.
fn files(pairs: &[(&str, &str)]) -> impl FnMut(&str) -> Option<Vec<u8>> + use<> {
    let table: HashMap<String, Vec<u8>> = pairs
        .iter()
        .map(|(k, v)| (k.to_ascii_lowercase(), (*v).as_bytes().to_vec()))
        .collect();
    move |path: &str| table.get(&path.to_ascii_lowercase()).cloned()
}

const SHELL: &str = r#"<Screen>
  <Image name="ForwardHUD">
    <LoadXML><Values SrcRel="colours.xml" DirectEmbed="true"></Values></LoadXML>
    <LoadXML><Values SrcRel="bars.xml" DirectEmbed="true"></Values></LoadXML>
  </Image>
</Screen>"#;

const COLOURS: &str = r#"<Variable global="HudColour1"><Values String="0xFF00FF00"/></Variable>"#;

const BARS: &str = r#"<Text name="BarParent" OffsetX="100" OffsetY="200">
  <Image name="SpeedBar">
    <Values x="10" y="20" width="30" height="8" Color="FEConst->HudColour1"
            U="1" V="2" TxtrWidth="30" TxtrHeight="8" src="a.gtf"/>
  </Image>
</Text>"#;

#[test]
fn a_fragment_s_widgets_land_in_the_composed_layout() {
    let composed = compose(
        "/data/xml/hud.xml",
        files(&[
            ("/data/xml/hud.xml", SHELL),
            ("/data/xml/colours.xml", COLOURS),
            ("/data/xml/bars.xml", BARS),
        ]),
    )
    .expect("root reads");

    assert!(composed.missing.is_empty(), "{:?}", composed.missing);
    assert_eq!(
        composed.files,
        [
            "/data/xml/hud.xml",
            "/data/xml/colours.xml",
            "/data/xml/bars.xml"
        ]
    );
    assert_eq!(composed.layout.sprites.len(), 1);
}

#[test]
fn a_fragment_s_constants_resolve_against_another_fragment_s_widget() {
    let composed = compose(
        "/data/xml/hud.xml",
        files(&[
            ("/data/xml/hud.xml", SHELL),
            ("/data/xml/colours.xml", COLOURS),
            ("/data/xml/bars.xml", BARS),
        ]),
    )
    .expect("root reads");

    // `HudColour1` is declared in one file and referenced in another; splicing
    // before parsing is what makes that reachable.
    let bar = &composed.layout.sprites[0];
    assert_eq!(bar.color, [0.0, 1.0, 0.0, 1.0]);
}

#[test]
fn offsets_compose_through_the_splice() {
    let composed = compose(
        "/data/xml/hud.xml",
        files(&[
            ("/data/xml/hud.xml", SHELL),
            ("/data/xml/colours.xml", COLOURS),
            ("/data/xml/bars.xml", BARS),
        ]),
    )
    .expect("root reads");

    // 100 + 10 and 200 + 20: the fragment's own `<Text OffsetX>` group, applied
    // to a widget that came out of a different file.
    assert_eq!(composed.layout.sprites[0].rect, [110.0, 220.0, 30.0, 8.0]);
}

#[test]
fn srcrel_resolves_against_the_including_file_s_directory() {
    // The same basename in two directories, which is exactly how the three HUD
    // skins are told apart on the disc.
    let composed = compose(
        "/data/xml/wo3_hud/hud.xml",
        files(&[
            ("/data/xml/wo3_hud/hud.xml", SHELL),
            ("/data/xml/wo3_hud/colours.xml", COLOURS),
            ("/data/xml/wo3_hud/bars.xml", BARS),
            ("/data/xml/colours.xml", "<Screen/>"),
            ("/data/xml/bars.xml", "<Screen/>"),
        ]),
    )
    .expect("root reads");

    assert_eq!(composed.files[1], "/data/xml/wo3_hud/colours.xml");
    assert_eq!(composed.layout.sprites.len(), 1);
}

#[test]
fn a_missing_fragment_is_reported_and_the_rest_still_loads() {
    let composed = compose(
        "/data/xml/hud.xml",
        files(&[("/data/xml/hud.xml", SHELL), ("/data/xml/bars.xml", BARS)]),
    )
    .expect("root reads");

    assert_eq!(composed.missing, ["/data/xml/colours.xml"]);
    assert_eq!(composed.layout.sprites.len(), 1);
    // The colour it could not resolve falls back to white rather than dropping
    // the widget.
    assert_eq!(composed.layout.sprites[0].color, [1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn an_include_cycle_is_broken_rather_than_followed() {
    let composed = compose(
        "/data/xml/a.xml",
        files(&[
            (
                "/data/xml/a.xml",
                r#"<Screen><LoadXML><Values SrcRel="b.xml"/></LoadXML></Screen>"#,
            ),
            (
                "/data/xml/b.xml",
                r#"<Screen><LoadXML><Values SrcRel="a.xml"/></LoadXML></Screen>"#,
            ),
        ]),
    )
    .expect("root reads");

    assert_eq!(composed.files, ["/data/xml/a.xml", "/data/xml/b.xml"]);
    assert_eq!(composed.missing.len(), 1);
    assert!(composed.missing[0].contains("would close a cycle"));
}

#[test]
fn a_full_src_path_is_taken_as_it_is_written() {
    let composed = compose(
        "/data/xml/hud.xml",
        files(&[
            (
                "/data/xml/hud.xml",
                r#"<Screen><LoadXML><Values Src="Data\XML\wo3_HUD\bars.xml"/></LoadXML></Screen>"#,
            ),
            (r"Data\XML\wo3_HUD\bars.xml", BARS),
        ]),
    )
    .expect("root reads");

    assert!(composed.missing.is_empty(), "{:?}", composed.missing);
    assert_eq!(composed.layout.sprites.len(), 1);
}

#[test]
fn an_unreadable_root_is_none_rather_than_an_empty_layout() {
    assert!(compose("/data/xml/hud.xml", files(&[])).is_none());
}
