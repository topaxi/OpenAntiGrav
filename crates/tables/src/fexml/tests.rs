//! What the front-end XML reader in [`super`] is asserted to do: the `<code>`
//! name mapping, the tree it builds, and the documents it refuses. Its own file
//! because the tests exceed the 200-line inline limit
//! (`scripts/check-file-size.py`).

use super::*;

const SAMPLE: &str = concat!(
    r#"<code as="Values" bs="Screen" ds="name" es="Model" ls="Src"></code>"#,
    "\n",
    r#"<b d="Top"><e d="Ship"><a l="Data\Ships\Feisar\ship_FE.vex" x="1.0"></a></e></b>"#
);

#[test]
fn reads_the_dictionary() {
    let map = dictionary(SAMPLE).unwrap();
    assert_eq!(map["a"], "Values");
    assert_eq!(map["b"], "Screen");
    assert_eq!(map["l"], "Src");
    assert_eq!(map.len(), 5);
}

#[test]
fn expands_elements_and_attributes() {
    let out = expand(SAMPLE.as_bytes()).unwrap();
    assert!(out.contains(r#"<Screen name="Top">"#), "{out}");
    assert!(out.contains(r#"<Model name="Ship">"#), "{out}");
    assert!(
        out.contains(r#"Src="Data\Ships\Feisar\ship_FE.vex""#),
        "{out}"
    );
    assert!(out.contains("</Screen>"), "closing tags expand too: {out}");
    assert!(!out.contains("<code"), "the dictionary itself is dropped");
}

#[test]
fn leaves_unmapped_names_alone() {
    // `x` is not in the dictionary: it must survive untouched.
    let out = expand(SAMPLE.as_bytes()).unwrap();
    assert!(out.contains(r#"x="1.0""#), "{out}");
}

#[test]
fn the_dictionary_is_per_file() {
    // The same short name means different things per file, so a shared table would corrupt.
    let other = r#"<code as="Class" bs="Difficulty"></code><a b="Hard"></a>"#;
    let out = expand(other.as_bytes()).unwrap();
    assert!(out.contains(r#"<Class Difficulty="Hard">"#), "{out}");
}

#[test]
fn extracts_asset_paths() {
    let expanded = expand(SAMPLE.as_bytes()).unwrap();
    assert_eq!(
        asset_paths(&expanded),
        vec![r"Data\Ships\Feisar\ship_FE.vex"]
    );
}

#[test]
fn does_not_mistake_plain_values_for_paths() {
    assert!(!looks_like_path("1.0"));
    assert!(!looks_like_path("Top"));
    assert!(!looks_like_path("true"));
    assert!(looks_like_path(r"Data\Ships\Feisar\ship_FE.vex"));
    assert!(looks_like_path("data/plugins/grids/grid_00.xml"));
}

#[test]
fn handles_self_closing_tags() {
    let src = r#"<code as="Values"></code><a x="1"/>"#;
    let out = expand(src.as_bytes()).unwrap();
    assert!(out.contains(r#"<Values x="1"/>"#), "{out}");
}

#[test]
fn keeps_attributes_after_an_angle_bracket_in_a_value() {
    // `FEGlobals->TitleColor` is how real front-end XML references a global;
    // ending the tag at its `>` drops `color` and spills the rest into text.
    let src = concat!(
        r#"<code as="Values" bs="Text" xs="x" cs="color"></code>"#,
        r#"<b><a x="FEGlobals->TitleXOffset" c="FEGlobals->TitleColor"></a></b>"#
    );
    let out = expand(src.as_bytes()).unwrap();
    assert!(out.contains(r#"x="FEGlobals->TitleXOffset""#), "{out}");
    assert!(out.contains(r#"color="FEGlobals->TitleColor""#), "{out}");
    assert!(!out.contains("TitleXOffset\">"), "no truncated tag: {out}");
}

#[test]
fn finds_asset_paths_after_an_angle_bracket_in_a_value() {
    let expanded = r#"<Image y="FEGlobals->Top" src="Data\FE\Images\pulse_logo.mip"></Image>"#;
    assert_eq!(
        asset_paths(expanded),
        vec![r"Data\FE\Images\pulse_logo.mip"]
    );
}

#[test]
fn rejects_a_blob_with_no_dictionary() {
    assert_eq!(expand(b"<Screen/>"), Err(Error::NoDictionary));
}

/// The PS2's `Data\XML\*_HUD.xml` shape: plain `<?xml`, no dictionary, and
/// nothing to expand. `expand` refuses it and `text` must not.
#[test]
fn plain_xml_passes_through_text_untouched() {
    let plain = b"<?xml version=\"1.0\" ?>\n<Screen name=\"HUD\"></Screen>";
    assert_eq!(expand(plain), Err(Error::NoDictionary));
    assert_eq!(
        text(plain).as_deref(),
        Ok("<?xml version=\"1.0\" ?>\n<Screen name=\"HUD\"></Screen>")
    );
}

#[test]
fn text_still_expands_a_shortened_blob() {
    let short = br#"<code as="Values" bs="Screen"></code><b><a d="1"></a></b>"#;
    assert_eq!(text(short), expand(short));
    assert!(text(short).expect("expands").contains("<Screen>"));
}

#[test]
fn text_refuses_bytes_that_are_not_utf8() {
    assert_eq!(text(&[0xff, 0xfe, 0x00]), Err(Error::NotText));
}

#[test]
fn identifies_candidates_by_their_leading_bytes() {
    assert!(is_fexml(b"<code as=\"Values\">"));
    assert!(!is_fexml(b"<?xml version=\"1.0\"?>"));
}

#[test]
fn builds_a_tree_from_expanded_text() {
    let root = parse(&expand(SAMPLE.as_bytes()).unwrap());
    assert_eq!(root.name, "#document");
    let screen = &root.children[0];
    assert_eq!(screen.name, "Screen");
    assert_eq!(screen.attr("name"), Some("Top"));
    assert_eq!(screen.children[0].name, "Model");
}

#[test]
fn an_angle_bracket_in_a_value_does_not_end_the_tag() {
    let node = parse(r#"<Text x="FEGlobals->X" y="2"></Text>"#);
    let text = &node.children[0];
    assert_eq!(text.attr("x"), Some("FEGlobals->X"));
    assert_eq!(text.attr("y"), Some("2"));
}

#[test]
fn attributes_match_case_insensitively() {
    let node = parse(r#"<Text Color="0xFF00FF00"></Text>"#);
    assert_eq!(node.children[0].attr("color"), Some("0xFF00FF00"));
}

#[test]
fn unterminated_markup_yields_what_it_opened() {
    let root = parse("<Screen name=\"Top\"><Text");
    assert_eq!(root.children.len(), 1);
    assert_eq!(root.children[0].attr("name"), Some("Top"));
}

#[test]
fn a_stray_close_tag_is_ignored() {
    let root = parse("</Screen><Screen name=\"Top\"></Screen>");
    assert_eq!(root.children.len(), 1);
}

#[test]
fn self_closing_elements_close_themselves() {
    let root = parse(r#"<Screen name="Top"><Values x="1"/></Screen>"#);
    assert_eq!(root.children.len(), 1);
    assert_eq!(root.children[0].children.len(), 1);
    assert_eq!(root.children[0].value("x"), Some("1"));
}

#[test]
fn an_elements_own_attribute_beats_its_values_carrier() {
    let root = parse(r#"<Movie src="own"><Values src="carrier" sound="true"/></Movie>"#);
    let movie = &root.children[0];
    assert_eq!(movie.value("src"), Some("own"));
    assert_eq!(movie.flag("sound"), Some(true));
    assert_eq!(movie.flag("absent"), None);
}

/// The PS2 ship files really ship this declaration; its unquoted `encoding`
/// leaves an odd number of quotes, putting quote tracking out of phase so every
/// element after it disappears.
#[test]
fn a_malformed_declaration_does_not_swallow_the_document() {
    let root = parse(concat!(
        r#" <?xml version="1.0" encoding=utf-81"?>"#,
        "\r\n<Handling>\r\n  <Stats team=\"Testers\"></Stats>\r\n</Handling>",
    ));
    assert_eq!(root.children.len(), 1, "{root:?}");
    assert_eq!(root.children[0].name, "Handling");
    assert_eq!(root.children[0].children[0].attr("team"), Some("Testers"));
}

#[test]
fn a_comment_ends_at_its_own_terminator() {
    let root = parse(r#"<!-- a "quoted > thing --><Screen name="Top"></Screen>"#);
    assert_eq!(root.children.len(), 1);
    assert_eq!(root.children[0].attr("name"), Some("Top"));
}

#[test]
fn children_are_selected_by_name_case_insensitively() {
    let root = parse("<Stats><Class/><class/><Misc/></Stats>");
    assert_eq!(root.children[0].children_named("CLASS").count(), 2);
}

/// HD/Fury's `grid_04.xml` ships a `<Values>` start tag with no `>` before the
/// real `</Values>`. Synthetic here (made-up names), same shape:
/// `<Foo attr="x"</Foo>` then the sibling. Untreated, every following sibling
/// becomes `Foo`'s child; see [`tag_end`].
#[test]
fn an_unquoted_lt_ends_a_tag_one_character_short_of_it() {
    let root = parse(r#"<Outer><Foo attr="x"</Foo><Bar/></Outer>"#);
    let outer = &root.children[0];
    assert_eq!(
        outer.children.len(),
        2,
        "Foo and Bar both children of Outer, not Bar nested inside Foo: {outer:?}"
    );
    assert_eq!(outer.children[0].name, "Foo");
    // The truncated value still reads whole (end-of-input closes an unterminated value).
    assert_eq!(outer.children[0].attr("attr"), Some("x"));
    assert_eq!(outer.children[1].name, "Bar");
}

/// A stray `<<`, not on any disc: `tag_end`'s `close = index - 1` would be `0`
/// and callers slice `rest[1..close]`, which panics. The `index > 1` guard
/// exists for this input; see [`tag_end`].
#[test]
fn a_stray_double_lt_does_not_panic() {
    let root = parse("<Outer><<Foo/></Outer>");
    // Only that it returns rather than panicking.
    let _ = root;
}

/// [`tag_end`]'s recovery is reached through [`expand`] too, and `expand` runs it
/// *before* the dictionary substitutes names, so this is the only test of the
/// recovery on a shortened blob. A cross-title census (`cargo run -p oag-game
/// --example fexml_recovery_noop_census`) found the same `attr="x"</Tag>` shape
/// on Pulse's shortened "stats holder" screen (`Data.wad`, PSP EU/USA).
#[test]
fn expand_recovers_a_start_tag_missing_its_closing_bracket() {
    let src = concat!(
        r#"<code as="Values" bs="ImageModel"></code>"#,
        r#"<b><a Src="x.vex" RotY="-0.5"</a></b>"#,
    );
    let out = expand(src.as_bytes()).unwrap();
    assert!(
        out.contains(r#"<Values Src="x.vex" RotY="-0.5">"#),
        "the opening tag gets its own `>`: {out}"
    );
    assert!(out.contains("</Values>"), "{out}");
    assert!(out.contains("</ImageModel>"), "{out}");
}
