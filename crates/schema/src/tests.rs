use super::*;

fn h(location: &str, version: &str) -> Header {
    Header {
        location: location.into(),
        version: version.into(),
    }
}

#[test]
fn a_header_this_build_wrote_is_accepted() {
    let header = h(&location("pilot", 1), LANGUAGE_VERSION);
    assert_eq!(check(Some(&header), "pilot", 1), Ok(()));
}

#[test]
fn kind_and_major_come_from_the_last_two_segments() {
    let r = parse_location("file:///tmp/anywhere/v12/track.tosd").unwrap();
    assert_eq!((r.kind.as_str(), r.major), ("track", 12));
}

#[test]
fn the_refusals_name_what_they_found() {
    let ok = LANGUAGE_VERSION;
    let v2 = h(&location("pilot", 2), ok);
    assert_eq!(
        check(Some(&v2), "pilot", 1).unwrap_err().to_string(),
        "pilot schema v2 is not supported; this build reads v1"
    );
    let menu = h(&location("menu", 1), ok);
    assert!(
        check(Some(&menu), "pilot", 1)
            .unwrap_err()
            .to_string()
            .contains("\"menu\"")
    );
    for bad in [
        "",
        "pilot.tosd",
        "https://x/v1/.tosd",
        "https://x/1/pilot.tosd",
        "https://x/v/pilot.tosd",
        "https://x/v1/pilot.toml",
    ] {
        assert!(
            matches!(
                check(Some(&h(bad, ok)), "pilot", 1),
                Err(Error::BadLocation(_))
            ),
            "{bad}"
        );
    }
    assert!(matches!(
        check(Some(&h(&location("pilot", 1), "1.0")), "pilot", 1),
        Err(Error::LanguageVersion { .. })
    ));
    let missing = check(None, "pilot", 1).unwrap_err().to_string();
    assert!(
        missing.contains(
            "toml-schema = { location = \"https://oag.topaxi.com/mods/schemas/v1/pilot.tosd\""
        ),
        "{missing}"
    );
}

#[test]
fn the_inline_and_table_forms_parse_to_the_same_header() {
    let table: toml::Table = header("menu", 1).parse().unwrap();
    let line: toml::Table = inline("menu", 1).parse().unwrap();
    assert_eq!(table, line);
    let parsed: Header = table[TABLE].clone().try_into().unwrap();
    assert_eq!(check(Some(&parsed), "menu", 1), Ok(()));
}
