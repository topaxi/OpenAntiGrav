//! `schemas/v1/split-hud.tosd` and `SplitHud::parse` must give the same verdict on the
//! shape of a split-HUD file, so neither can drift from the other unseen.
//!
//! What the schema does not carry is the loader's own range checks (`views` in 1 to 8,
//! `scale` in (0, 8], an `offset` inside one view, an increasing `aspect`): a file past
//! one is refused by the loader alone, and every example below is inside them.

use oag_hud::split_hud::SplitHud;

const HEADER: &str = "toml-schema = { location = \"https://oag.topaxi.com/mods/schemas/v1/split-hud.tosd\", version = \"1.0.0\" }\n";

const GROUP: &str = "[[layout]]\n[[layout.group]]\nwidgets = [\"Lap*\"]\nanchor = \"top-left\"\n";

fn cases() -> Vec<(&'static str, String, bool)> {
    let with = |body: &str| format!("{HEADER}{body}");
    let group_with = |extra: &str| with(&format!("{GROUP}{extra}"));
    vec![
        ("header only", HEADER.to_owned(), true),
        ("empty file", String::new(), true),
        ("one group", with(GROUP), true),
        (
            "every key",
            with("[[layout]]\nname = \"x\"\nviews = [2, 3]\naspect = [1.0, 2.5]\nmodes = [\"zone\"]\n[[layout.group]]\nwidgets = [\"a\", \"b*\"]\nanchor = \"bottom-right\"\noffset = [-0.1, 0.05]\nscale = 1.5\n"),
            true,
        ),
        ("integers in a pair", group_with("offset = [0, 0]\nscale = 1\n"), true),
        ("aspect in integers", with("[[layout]]\naspect = [1, 3]\n"), true),
        ("local schema path", with(GROUP).replace("https://oag.topaxi.com/mods/schemas", "file:///x/schemas"), true),
        ("unknown top key", with("groups = 1\n"), false),
        ("unknown group key", group_with("colour = 1\n"), false),
        ("unknown layout key", with("[[layout]]\nzoom = 2\n"), false),
        ("anchor not a point", with(&GROUP.replace("top-left", "middle")), false),
        ("no anchor", with("[[layout]]\n[[layout.group]]\nwidgets = [\"a\"]\n"), false),
        ("no widgets", with("[[layout]]\n[[layout.group]]\nanchor = \"top\"\n"), false),
        ("empty widgets", with("[[layout]]\n[[layout.group]]\nwidgets = []\nanchor = \"top\"\n"), false),
        ("offset of three", group_with("offset = [0.0, 0.0, 0.0]\n"), false),
        ("scale a string", group_with("scale = \"big\"\n"), false),
        ("views as strings", with("[[layout]]\nviews = [\"2\"]\n"), false),
        ("another major", with(GROUP).replace("/v1/", "/v2/"), false),
        ("another kind", with(GROUP).replace("split-hud.tosd", "pilot.tosd"), false),
        ("no version", "toml-schema = { location = \"https://oag.topaxi.com/mods/schemas/v1/split-hud.tosd\" }\n".to_owned(), false),
    ]
}

#[test]
fn the_schema_and_the_loader_agree_on_every_example() {
    for (what, text, accepted) in cases() {
        let schema = oag_schema::validate::accepts("split-hud", 1, &text);
        let loader = SplitHud::parse(&text);
        assert_eq!(schema.is_ok(), accepted, "schema, {what}: {schema:?}");
        assert_eq!(loader.is_ok(), accepted, "loader, {what}: {loader:?}");
    }
}

#[test]
fn every_shipped_file_is_valid_and_loads() {
    for title in ["Pulse", "Pure", "HD"] {
        let text = std::fs::read_to_string(format!(
            "{}/../../assets/fallback/Data/XML/SplitHud_{title}.toml",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        oag_schema::validate::accepts("split-hud", 1, &text)
            .unwrap_or_else(|e| panic!("{title}: {e:?}"));
        let file = SplitHud::parse(&text).unwrap_or_else(|e| panic!("{title}: {e}"));
        assert!(!file.rules.is_empty(), "{title}");
    }
}

#[test]
fn a_refused_file_names_what_it_found() {
    for (what, text, expect) in [
        ("major", with_header("/v7/"), "v7"),
        ("kind", HEADER.replace("split-hud", "track"), "\"track\""),
    ] {
        let error = SplitHud::parse(&text).unwrap_err().to_string();
        assert!(error.contains(expect), "{what}: {error}");
    }
}

fn with_header(version: &str) -> String {
    HEADER.replace("/v1/", version)
}
