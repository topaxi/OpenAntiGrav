//! `schemas/v1/pilot.tosd` and `pilots::parse` must give the same verdict on the
//! shape of a pilot file, so neither can drift from the other unseen.
//!
//! What the schema does not carry is the numeric limit of an axis
//! (`pilots::limit`): those are the loader's, and a file past one is refused by
//! the loader alone. Every example below is inside the limits.

use oag_raceplay::pilots;

const HEADER: &str = "toml-schema = { location = \"https://oag.topaxi.com/mods/schemas/v1/pilot.tosd\", version = \"1.0.0\" }\n";

fn cases() -> Vec<(&'static str, String, bool)> {
    let with = |body: &str| format!("{HEADER}{body}");
    vec![
        ("empty file", String::new(), true),
        ("header only", HEADER.to_owned(), true),
        ("headerless axis", "commitment = [0.5, 0.6]\n".to_owned(), true),
        ("axis and lean", with("commitment = [0.5, 0.6]\nlean = \"left\"\n"), true),
        (
            "table form, last",
            "ram = [0.1, 0.2]\n[toml-schema]\nlocation = \"https://oag.topaxi.com/mods/schemas/v1/pilot.tosd\"\nversion = \"1.0.0\"\n"
                .to_owned(),
            true,
        ),
        ("local schema path", with("").replace("https://oag.topaxi.com/mods/schemas", "file:///x/schemas"), true),
        ("unknown key", with("commitmnt = [0.5, 0.6]\n"), false),
        ("three numbers", with("ram = [0.1, 0.2, 0.3]\n"), false),
        ("one number", with("ram = [0.1]\n"), false),
        ("scalar axis", with("ram = 0.1\n"), false),
        ("integers in an axis", with("provocation_ticks = [60, 90]\n"), true),
        ("a string in an axis", with("ram = [\"a\", \"b\"]\n"), false),
        ("lean not a side", with("lean = \"up\"\n"), false),
        ("another major", with("").replace("/v1/", "/v2/"), false),
        ("another kind", with("").replace("pilot.tosd", "menu.tosd"), false),
        ("no version", "toml-schema = { location = \"https://oag.topaxi.com/mods/schemas/v1/pilot.tosd\" }\n".to_owned(), false),
        ("extra header key", with("").replace("version = \"1.0.0\" }", "version = \"1.0.0\", mood = \"x\" }"), false),
    ]
}

#[test]
fn the_schema_and_the_loader_agree_on_every_example() {
    for (what, text, accepted) in cases() {
        let schema = oag_schema::validate::accepts("pilot", 1, &text);
        let loader = pilots::parse("drift", &text);
        assert_eq!(schema.is_ok(), accepted, "schema, {what}: {schema:?}");
        assert_eq!(loader.is_ok(), accepted, "loader, {what}: {loader:?}");
    }
}

#[test]
fn the_shipped_example_pilot_is_valid_and_loads() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/ai/example-pilot.toml"
    ))
    .unwrap();
    oag_schema::validate::accepts("pilot", 1, &text).expect("schema");
    pilots::parse("example-pilot", &text).expect("loader");
}

#[test]
fn what_the_editor_writes_is_valid_with_the_header() {
    let pilot = oag_ai::Pilot::BALANCED;
    let fresh = pilots::template(&pilot);
    let edited = pilots::set_axis(&fresh, "ram", 0.2, 0.4).unwrap();
    // A file from before the header existed gets it on its first save.
    let old = pilots::set_axis("# mine\nram = [0.1, 0.2]\n", "ram", 0.2, 0.4).unwrap();
    for (what, text) in [
        ("template", fresh),
        ("edited", edited),
        ("old file saved", old),
    ] {
        oag_schema::validate::accepts("pilot", 1, &text)
            .unwrap_or_else(|e| panic!("{what}: {e:?}\n{text}"));
        pilots::parse(what, &text).unwrap_or_else(|e| panic!("{what}: {e:#}"));
        assert!(
            text.contains(oag_schema::TABLE),
            "{what} lacks the header:\n{text}"
        );
    }
}

#[test]
fn a_refused_header_names_what_it_found() {
    for (what, text, expect) in [
        ("major", HEADER.replace("/v1/", "/v7/"), "v7"),
        (
            "kind",
            HEADER.replace("pilot.tosd", "track.tosd"),
            "\"track\"",
        ),
    ] {
        let error = format!("{:#}", pilots::parse("drift", &text).unwrap_err());
        assert!(error.contains(expect), "{what}: {error}");
    }
}
