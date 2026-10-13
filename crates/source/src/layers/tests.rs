use super::*;

const TITLE: &Title = oag_pulse::TITLE;

#[test]
fn a_title_directory_is_its_lowercase_hyphenated_name() {
    assert_eq!(slug(TITLE), "wipeout-pulse");
    assert_eq!(slug(oag_pure::TITLE), "wipeout-pure");
}

#[test]
fn no_directory_is_no_override() {
    let none = std::env::temp_dir().join("oag-no-such-overrides-dir");
    assert!(override_layer_in(&[none], TITLE).is_none());
}

#[test]
fn files_under_the_title_directory_are_the_override_and_another_title_s_are_not() {
    let base = std::env::temp_dir().join(format!("oag-overrides-{}", std::process::id()));
    let mine = base.join("wipeout-pulse/Data/HUD");
    std::fs::create_dir_all(&mine).unwrap();
    std::fs::write(mine.join("Foo.mip"), b"mod").unwrap();

    let layer = override_layer_in(std::slice::from_ref(&base), TITLE).expect("an override");
    assert!(layer.contains("Data\\HUD\\foo.mip"));
    assert!(override_layer_in(std::slice::from_ref(&base), oag_pure::TITLE).is_none());
    std::fs::remove_dir_all(&base).unwrap();
}

/// The fallback carries the indicator's layout and art under the entry names the
/// layout itself uses, so a reader asking the archives for them is answered.
#[test]
fn the_fallback_holds_the_assist_indicator_and_the_art_its_layout_names() {
    let layer = fallback_layer();
    let fragment = layer.read("Data\\XML\\HUD_assist_indicator.xml").unwrap();
    let text = String::from_utf8(fragment).unwrap();
    assert!(text.contains("CHOSEN, NOT MEASURED"), "labelled as chosen");
    let art = layer
        .read("Data\\HUD\\Textures\\AssistIndicator.pix")
        .unwrap();
    assert!(layer.contains("/data/hud/textures/assistindicator.pix"));
    assert!(
        art.starts_with(b"OAGMASK 112 144\n"),
        "the atlas the layout's UVs index"
    );
}

/// A title that names a split-HUD file in its `HudLayouts` has it in the fallback under that
/// name, and a split race of it reads a file rather than scaling the whole-screen layout.
/// Dropping either half, or a title's row, fails here.
#[test]
fn every_title_with_a_split_hud_row_has_its_file_in_the_fallback() {
    let layer = fallback_layer();
    for title in [
        oag_pulse::TITLE,
        oag_pure::TITLE,
        oag_hd::TITLE,
        oag_2048::TITLE,
    ] {
        let entry = title
            .hud
            .split_layout
            .unwrap_or_else(|| panic!("{} names no split-HUD file", title.name));
        assert!(layer.contains(entry), "{}: {entry}", title.name);
        let text = String::from_utf8(layer.read(entry).unwrap()).unwrap();
        assert!(
            text.contains("CHOSEN, NOT MEASURED"),
            "{}: labelled",
            title.name
        );
    }
}
