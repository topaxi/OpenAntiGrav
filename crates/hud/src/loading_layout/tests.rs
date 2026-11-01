use super::*;

const HEADER: &str = "screen = \"loading\"\n[toml-schema]\nlocation = \"https://oag.topaxi.com/mods/schemas/v1/screen-layout.tosd\"\nversion = \"1.0.0\"\n";

fn file(body: &str) -> String {
    format!("{HEADER}{body}")
}

const LAYOUT: &str = "[[layout]]\nname = \"x\"\ngrid = [1920, 1080]\n";

fn shipped() -> LoadingLayouts {
    let text = std::fs::read_to_string(format!(
        "{}/../../assets/fallback/Data/XML/LoadingScreen_HD.toml",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    LoadingLayouts::parse(&text).unwrap()
}

#[test]
fn the_shipped_hd_file_is_the_races_screen_in_the_1920_grid() {
    let hd = shipped();
    let layout = hd.for_race("single_race", 16.0 / 9.0).unwrap();
    assert_eq!(layout.grid, [1920.0, 1080.0]);
    assert_eq!(layout.elements.len(), 25);
    let by_id = |id: &str| layout.elements.iter().find(|e| e.id == id).unwrap();
    // 0x002b6d20 and 0x002b6d78: the two rules, 1600 x 8 from x 160
    assert_eq!(by_id("rule-top").at, [160.0, 110.0]);
    assert_eq!(by_id("rule-bottom").at, [160.0, 975.0]);
    assert_eq!(by_id("rule-top").size, [1600.0, 8.0]);
    // 0x002b70b8: 166 x 30 tiles of 8 from (168, 708), the right edge 1496
    let bar = by_id("bar");
    assert_eq!((bar.columns, bar.rows, bar.tile), (166, 30, 8.0));
    assert_eq!(bar.at[0] + bar.tile * bar.columns as f32, 1496.0);
    assert_eq!(bar.at[1] + bar.tile * bar.rows as f32, 948.0);
    // the paragraph: wrap 780 from x 975 ends at the description panel's right edge, 1755
    let text = by_id("feature-text");
    assert_eq!((text.wrap, text.pitch), (Some(780.0), Some(40.0)));
    assert_eq!(text.at[0] + 780.0, 1755.0);
    assert_eq!(by_id("caption").font.as_deref(), Some("Title"));
}

#[test]
fn a_layout_answers_by_mode_and_by_shape() {
    let layouts = LoadingLayouts::parse(&file(
        "[[layout]]\nname = \"zone\"\ngrid = [10, 10]\nmodes = [\"zone\"]\n[[layout]]\nname = \"wide\"\ngrid = [10, 10]\naspect = [1.5, 3.0]\n[[layout]]\nname = \"any\"\ngrid = [10, 10]\n",
    ))
    .unwrap();
    let name = |mode: &str, aspect: f32| layouts.for_race(mode, aspect).map(|l| l.name.as_str());
    assert_eq!(name("zone", 1.0), Some("zone"));
    assert_eq!(name("single_race", 16.0 / 9.0), Some("wide"));
    assert_eq!(name("single_race", 1.0), Some("any"));
}

#[test]
fn a_refused_file_names_what_is_wrong() {
    let refused = |text: String| LoadingLayouts::parse(&text).unwrap_err().to_string();
    let element = |extra: &str| file(&format!("{LAYOUT}[[layout.element]]\nid = \"e\"\n{extra}"));
    assert!(refused(HEADER.replace("loading", "split-hud")).contains("\"split-hud\""));
    assert!(refused(HEADER.replace("screen = \"loading\"\n", "")).contains("absent"));
    assert!(refused(HEADER.replace("/v1/", "/v9/")).contains("v9"));
    assert!(refused(file("[[layout]]\nname = \"x\"\n")).contains("needs a `grid`"));
    assert!(refused(file("[[layout]]\ngrid = [0, 5]\n")).contains("not positive"));
    assert!(refused(file("[[layout]]\ngrid = [5, 5]\naspect = [2.0, 1.0]\n")).contains("`aspect`"));
    assert!(
        refused(element("draw = \"image\"\nat = [0, 0]\nsize = [1, 1]\n"))
            .contains("needs `asset`")
    );
    assert!(
        refused(element("draw = \"text\"\nat = [0, 0]\n")).contains("one of `source` and `text`")
    );
    assert!(
        refused(element(
            "draw = \"text\"\nat = [0, 0]\nsource = \"caption\"\ntext = \"x\"\n"
        ))
        .contains("one of `source` and `text`")
    );
    assert!(
        refused(element(
            "draw = \"label\"\nat = [0, 0]\nasset = \"square\"\nsize = [1, 1]\ngap = 2\n"
        ))
        .contains("`string_id` and `text`")
    );
    assert!(refused(element("draw = \"corners\"\n")).contains("needs `rect`"));
    assert!(
        refused(element(
            "draw = \"corners\"\nrect = [0, 0, 1, 1]\nasset = \"corner\"\nsize = [1, 1]\n"
        ))
        .contains("needs `back`")
    );
    assert!(
        refused(element(
            "draw = \"label\"\nat = [0, 0]\ntext = \"x\"\nasset = \"square\"\nsize = [1, 1]\n"
        ))
        .contains("needs `gap`")
    );
    assert!(
        refused(element("draw = \"dots\"\nasset = \"dot\"\nat = [0, 0]\n")).contains("`columns`")
    );
    assert!(
        refused(element(
            "draw = \"text\"\nat = [0, 0]\ntext = \"x\"\nscale = 0.0\n"
        ))
        .contains("`scale` 0")
    );
    let twice = format!(
        "{LAYOUT}[[layout.element]]\nid = \"e\"\ndraw = \"corners\"\nrect = [0, 0, 1, 1]\nasset = \"corner\"\nsize = [1, 1]\nback = 1\n[[layout.element]]\nid = \"e\"\ndraw = \"corners\"\nrect = [0, 0, 1, 1]\nasset = \"corner\"\nsize = [1, 1]\nback = 1\n"
    );
    assert!(refused(file(&twice)).contains("used twice"));
}
