use super::*;

const HEAD: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\r\n<RumbleApp><RumbleEditor><Zoom>2.0</Zoom><Offset>10.0</Offset></RumbleEditor><RumbleData><FPS>60</FPS>";
const TAIL: &str = "</RumbleData></RumbleApp>";

fn file(body: &str) -> Vec<u8> {
    format!("{HEAD}{body}{TAIL}").into_bytes()
}

const LOOPING: &str = "<Type LoopStart=\"1\" LoopEnd=\"3\">Looping</Type><Class>UI</Class><Priority>High</Priority><Behaviour>Positional</Behaviour><LargeActuator FallOffInner=\"30\" FallOffOuter=\"20\">9 8 7 6 </LargeActuator><SmallActuator>1 0 1 </SmallActuator>";

#[test]
fn a_looping_positional_pattern_reads_every_field() {
    let p = parse(&file(LOOPING)).unwrap();
    assert_eq!(p.kind, Kind::Looping);
    assert_eq!((p.loop_start, p.loop_end), (1, 3));
    assert_eq!(p.class, Some(Class::Ui));
    assert_eq!(p.priority, Some(Priority::High));
    assert_eq!(
        p.positional,
        Some(FallOff {
            inner: 30.0,
            outer: 20.0
        })
    );
    assert_eq!(p.large, [9, 8, 7, 6]);
    assert_eq!(p.small, [1, 0, 1]);
    assert_eq!(p.frames(), 4);
}

#[test]
fn a_track_with_and_without_its_trailing_space_reads_the_same() {
    let with = parse(&file(
        "<Type>OneShot</Type><LargeActuator>5 6 7 </LargeActuator><SmallActuator>1 </SmallActuator>",
    ))
    .unwrap();
    let without = parse(&file(
        "<Type>OneShot</Type><LargeActuator>5 6 7</LargeActuator><SmallActuator>1</SmallActuator>",
    ))
    .unwrap();
    assert_eq!(with, without);
    assert_eq!(without.large, [5, 6, 7]);
}

#[test]
fn unequal_tracks_keep_their_own_lengths() {
    let p = parse(&file(
        "<Type>OneShot</Type><LargeActuator>1 2 3 4 5 </LargeActuator><SmallActuator>1 </SmallActuator>",
    ))
    .unwrap();
    assert_eq!((p.large.len(), p.small.len(), p.frames()), (5, 1, 5));
}

#[test]
fn an_absent_class_and_priority_read_as_the_originals_defaults() {
    let p = parse(&file(
        "<Type>OneShot</Type><Behaviour>Global</Behaviour><LargeActuator>0 </LargeActuator><SmallActuator>0 </SmallActuator>",
    ))
    .unwrap();
    assert_eq!((p.class, p.priority), (None, None));
    assert_eq!(p.effective_class(), Class::Game);
    assert_eq!(p.effective_priority(), Priority::Low);
    assert_eq!(p.positional, None);
}

#[test]
fn a_looping_type_without_attributes_loops_from_zero() {
    let p = parse(&file(
        "<Type>Looping</Type><LargeActuator>1 </LargeActuator>",
    ))
    .unwrap();
    assert_eq!((p.kind, p.loop_start, p.loop_end), (Kind::Looping, 0, 0));
    assert!(p.small.is_empty());
}

#[test]
fn the_editor_block_and_line_endings_do_not_matter() {
    let plain = parse(&file(LOOPING)).unwrap();
    let editor = format!(
        "\u{feff}<?xml version=\"1.0\"?>\n<RumbleApp><RumbleEditor><Zoom>1</Zoom><Colours><Colour0>0 0 0</Colour0></Colours></RumbleEditor>\n<RumbleData>\n<FPS>60</FPS>\n{LOOPING}\n</RumbleData></RumbleApp>"
    );
    assert_eq!(parse(editor.as_bytes()).unwrap(), plain);
}

#[test]
fn a_reader_refuses_what_it_does_not_understand_by_name() {
    let cases: [(&str, Error); 10] = [
        ("<Type>Sometimes</Type>", unknown("Type", "Sometimes")),
        (
            "<Type>OneShot</Type><Priority>Urgent</Priority>",
            unknown("Priority", "Urgent"),
        ),
        (
            "<Type>OneShot</Type><Class>Both</Class>",
            unknown("Class", "Both"),
        ),
        (
            "<Type>OneShot</Type><Behaviour>Local</Behaviour>",
            unknown("Behaviour", "Local"),
        ),
        (
            "<Type>OneShot</Type><Extra>1</Extra>",
            Error::UnknownElement("Extra".into()),
        ),
        (
            "<Type>OneShot</Type><LargeActuator>1 256 </LargeActuator>",
            Error::TrackValue {
                track: "LargeActuator",
                token: "256".into(),
            },
        ),
        (
            "<Type>OneShot</Type><SmallActuator>1 x </SmallActuator>",
            Error::TrackValue {
                track: "SmallActuator",
                token: "x".into(),
            },
        ),
        (
            "<Type>OneShot</Type><Type>Looping</Type>",
            Error::Duplicate("Type"),
        ),
        (
            "<Type>OneShot</Type><LargeActuator FallOffInner=\"1\" FallOffOuter=\"2\">1 </LargeActuator>",
            Error::FallOffOnGlobal,
        ),
        (
            "<Type LoopStart=\"1\" LoopEnd=\"2\">OneShot</Type>",
            Error::LoopOnOneShot,
        ),
    ];
    for (body, want) in cases {
        assert_eq!(parse(&file(body)).unwrap_err(), want, "{body}");
    }
    let other_fps =
        "<?xml?><RumbleApp><RumbleData><FPS>30</FPS><Type>OneShot</Type></RumbleData></RumbleApp>";
    assert_eq!(
        parse(other_fps.as_bytes()).unwrap_err(),
        Error::Fps("30".into())
    );
    assert_eq!(parse(b"<Other/>").unwrap_err(), Error::NoRoot);
    assert_eq!(
        parse(b"<RumbleApp></RumbleApp>").unwrap_err(),
        Error::NoData
    );
    assert_eq!(parse(&[0xff, 0xfe]).unwrap_err(), Error::NotText);
}

#[test]
fn a_library_overrides_by_name_without_regard_to_case() {
    let a = parse(&file(
        "<Type>OneShot</Type><LargeActuator>1 </LargeActuator>",
    ))
    .unwrap();
    let b = parse(&file(
        "<Type>OneShot</Type><LargeActuator>2 </LargeActuator>",
    ))
    .unwrap();
    let mut lib = Library::new();
    assert!(lib.insert("Landing_Heavy", a.clone()).is_none());
    assert_eq!(lib.insert("landing_heavy", b.clone()), Some(a));
    assert_eq!((lib.len(), lib.get("LANDING_HEAVY")), (1, Some(&b)));
    assert_eq!(lib.iter().next().unwrap().0, "landing_heavy");
}

#[test]
fn an_archive_path_names_its_pattern() {
    assert_eq!(
        pattern_name("Data/xml/rumble/landing_heavy.xml"),
        Some(("landing_heavy", "xml"))
    );
    assert_eq!(
        pattern_name("data/xml/Rumble/Pad_Speed.TOML"),
        Some(("Pad_Speed", "TOML"))
    );
    assert_eq!(pattern_name("Data/xml/rumble/notes.txt"), None);
    assert_eq!(pattern_name("Data/xml/handling/x.xml"), None);
    assert_eq!(pattern_name("rumble.xml"), None);
}
