use super::*;

const TOML: &str = include_str!("../../tests/fixtures/example.rumble.toml");

const XML: &str = "<?xml version=\"1.0\"?><RumbleApp><RumbleEditor><Zoom>1</Zoom></RumbleEditor><RumbleData><FPS>60</FPS><Type LoopStart=\"1\" LoopEnd=\"3\">Looping</Type><Class>UI</Class><Priority>High</Priority><Behaviour>Positional</Behaviour><LargeActuator FallOffInner=\"30\" FallOffOuter=\"20.5\">9 8 7 6 </LargeActuator><SmallActuator>1 0 1 </SmallActuator></RumbleData></RumbleApp>";

fn with_header(body: &str) -> String {
    format!("{}{body}", oag_schema::header(SCHEMA_KIND, SCHEMA_MAJOR))
}

#[test]
fn the_toml_and_the_xml_form_of_a_pattern_read_equal() {
    let toml = parse(TOML).unwrap();
    let xml = oag_tables::rumble::parse(XML.as_bytes()).unwrap();
    assert_eq!(toml, xml);
}

#[test]
fn an_absent_class_priority_loop_and_positional_read_as_the_xml_does() {
    let toml = parse(&with_header(
        "[pattern]\ntype = \"OneShot\"\n[tracks]\nlarge = [0]\nsmall = [0]\n",
    ))
    .unwrap();
    let xml = oag_tables::rumble::parse(
        b"<RumbleApp><RumbleData><FPS>60</FPS><Type>OneShot</Type><Behaviour>Global</Behaviour><LargeActuator>0 </LargeActuator><SmallActuator>0 </SmallActuator></RumbleData></RumbleApp>",
    )
    .unwrap();
    assert_eq!(toml, xml);
    assert_eq!(
        (toml.class, toml.priority, toml.positional),
        (None, None, None)
    );
}

#[test]
fn a_positional_table_without_distances_reads_zero_like_the_xml() {
    let toml = parse(&with_header(
        "[pattern]\ntype = \"OneShot\"\n[pattern.positional]\n",
    ))
    .unwrap();
    assert_eq!(toml.positional, Some(FallOff::default()));
}

#[test]
fn the_toml_reader_refuses_by_name() {
    let body = |s: &str| with_header(&format!("[pattern]\ntype = \"OneShot\"\n{s}"));
    let cases = [
        ("no header", "[pattern]\ntype = \"OneShot\"\n".to_owned()),
        ("another kind", TOML.replace("rumble.tosd", "pilot.tosd")),
        ("another major", TOML.replace("/v1/", "/v2/")),
        (
            "an unknown type",
            with_header("[pattern]\ntype = \"Never\"\n"),
        ),
        ("an unknown key", body("speed = 1\n")),
        ("a byte over 255", body("[tracks]\nlarge = [256]\n")),
        ("a negative byte", body("[tracks]\nsmall = [-1]\n")),
        ("a loop on a one-shot", body("loop = [0, 2]\n")),
        ("no pattern table", with_header("[tracks]\nlarge = [1]\n")),
        ("a lower-case class", body("class = \"ui\"\n")),
    ];
    for (what, text) in cases {
        assert!(parse(&text).is_err(), "{what}");
    }
    assert!(matches!(
        parse(&body("loop = [0, 2]\n")),
        Err(Error::Pattern(oag_tables::rumble::Error::LoopOnOneShot))
    ));
}

#[test]
fn a_mod_shipping_both_forms_is_refused_by_name() {
    assert_eq!(
        from_forms("pad_speed", Some(XML.as_bytes()), Some(TOML)).unwrap_err(),
        Error::BothForms("pad_speed".into())
    );
    assert!(
        from_forms("x", Some(XML.as_bytes()), None)
            .unwrap()
            .is_some()
    );
    assert!(from_forms("x", None, Some(TOML)).unwrap().is_some());
    assert!(from_forms("x", None, None).unwrap().is_none());
}

#[test]
fn the_schema_and_the_loader_agree_on_every_example() {
    let cases = [
        ("every key", TOML.to_owned(), true),
        (
            "the shortest pattern",
            with_header("[pattern]\ntype = \"OneShot\"\n"),
            true,
        ),
        (
            "a oneshot with tracks",
            with_header("[pattern]\ntype = \"OneShot\"\n[tracks]\nlarge = [255]\nsmall = []\n"),
            true,
        ),
        (
            "an unknown type",
            with_header("[pattern]\ntype = \"Never\"\n"),
            false,
        ),
        (
            "an unknown class",
            with_header("[pattern]\ntype = \"OneShot\"\nclass = \"Both\"\n"),
            false,
        ),
        (
            "an unknown pattern key",
            with_header("[pattern]\ntype = \"OneShot\"\nspeed = 1\n"),
            false,
        ),
        (
            "a byte over 255",
            with_header("[pattern]\ntype = \"OneShot\"\n[tracks]\nlarge = [256]\n"),
            false,
        ),
        (
            "a loop of three",
            with_header("[pattern]\ntype = \"Looping\"\nloop = [0, 1, 2]\n"),
            false,
        ),
        ("no pattern", with_header("[tracks]\nlarge = [1]\n"), false),
        (
            "another kind",
            TOML.replace("rumble.tosd", "pilot.tosd"),
            false,
        ),
        ("another major", TOML.replace("/v1/", "/v2/"), false),
    ];
    for (what, text, accepted) in cases {
        let schema = oag_schema::validate::accepts(SCHEMA_KIND, SCHEMA_MAJOR, &text);
        let loader = parse(&text);
        assert_eq!(schema.is_ok(), accepted, "schema, {what}: {schema:?}");
        assert_eq!(loader.is_ok(), accepted, "loader, {what}: {loader:?}");
    }
}
