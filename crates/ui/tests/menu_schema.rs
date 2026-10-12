//! `schemas/v1/menu.tosd` and `Definition::parse` must give the same verdict on
//! the shape of a menu file. What the schema cannot say - dangling targets,
//! unknown actions, an unreachable page - stays the loader's, and no example
//! here depends on it.

use oag_ui::language::StringTable;
use oag_ui::menu::Definition;

const HEADER: &str = "toml-schema = { location = \"https://oag.topaxi.com/mods/schemas/v1/menu.tosd\", version = \"1.0.0\" }\n";
const PAGE: &str =
    "root = \"main\"\n[[page]]\nid = \"main\"\n[[page.entry]]\nkind = \"back\"\nlabel = \"X\"\n";

fn with(extra: &str) -> String {
    format!("{HEADER}{PAGE}{extra}")
}

fn cases() -> Vec<(&'static str, String, bool)> {
    vec![
        ("minimal", with(""), true),
        (
            "a choice with a condition and a warning",
            format!(
                "{HEADER}root = \"main\"\n[[page]]\nid = \"main\"\n\
                 [[page.entry]]\nkind = \"choice\"\nlabel = \"A\"\nsetting = \"display.vsync\"\nvalues = [\"on\", \"off\"]\n\
                 [[page.entry]]\nkind = \"choice\"\nlabel = \"B\"\nsetting = \"display.fps\"\nvalues = [\"30\", \"60\"]\n\
                 disabled_by = {{ setting = \"display.vsync\", value = \"on\" }}\n\
                 [[page.entry.warn_when]]\nmessage = \"m\"\nall = [{{ setting = \"display.vsync\", values = [\"off\"] }}, {{ setting = \"display.fps\", value = \"60\" }}]\n\
                 [[page.entry]]\nkind = \"back\"\nlabel = \"X\"\n"
            ),
            true,
        ),
        ("a warning on one setting alone", with("").replace("label = \"X\"", "label = \"X\"\nwarn_when = [{ message = \"m\", all = [{ setting = \"a\", value = 1 }] }]"), false),
        ("no header", PAGE.to_owned(), false),
        ("another major", with("").replace("/v1/", "/v2/"), false),
        (
            "another kind",
            with("").replace("menu.tosd", "pilot.tosd"),
            false,
        ),
        ("unknown top-level key", with("colour = \"red\"\n"), false),
        (
            "old version key",
            with("").replace(HEADER, "version = 1\n"),
            false,
        ),
        ("no root", with("").replace("root = \"main\"\n", ""), false),
        (
            "page without id",
            with("").replace("id = \"main\"\n", ""),
            false,
        ),
        (
            "entry without kind",
            with("").replace("kind = \"back\"\n", ""),
            false,
        ),
        (
            "unknown entry key",
            with("").replace("label = \"X\"", "label = \"X\"\ncolour = 1"),
            false,
        ),
        (
            "values not strings",
            with("").replace("label = \"X\"", "label = \"X\"\nvalues = [1, 2]"),
            false,
        ),
        (
            "label not a string",
            with("").replace("label = \"X\"", "label = 7"),
            false,
        ),
    ]
}

#[test]
fn the_schema_and_the_loader_agree_on_every_example() {
    for (what, text, accepted) in cases() {
        let schema = oag_schema::validate::accepts("menu", 1, &text);
        let loader = Definition::parse(&text, &StringTable::default());
        assert_eq!(schema.is_ok(), accepted, "schema, {what}: {schema:?}");
        assert_eq!(loader.is_ok(), accepted, "loader, {what}: {loader:?}");
    }
}

#[test]
fn the_shipped_menu_is_valid_and_loads() {
    let text = oag_ui::menu::BUILT_IN;
    oag_schema::validate::accepts("menu", 1, text).expect("schema");
    Definition::parse_vanilla(text, &StringTable::default()).expect("loader");
}

const MOD_HEADER: &str = "toml-schema = { location = \"https://oag.topaxi.com/mods/schemas/v1/mod-menu.tosd\", version = \"1.0.0\" }\n";
const ROW: &str = "[[entry]]\nid = \"extra\"\nparent = \"top\"\nkind = \"action\"\nlabel = \"X\"\naction = \"quit\"\n";

/// A mod's `menu.toml`, built onto the shipped tree: the verdict of the schema and
/// of the loader on the shape of the file.
fn mod_cases() -> Vec<(&'static str, String, bool)> {
    let file = |rows: &str| format!("{MOD_HEADER}{rows}");
    vec![
        ("empty", file(""), true),
        ("one row", file(ROW), true),
        ("no header", ROW.to_owned(), false),
        (
            "another kind",
            file(ROW).replace("mod-menu.tosd", "menu.tosd"),
            false,
        ),
        (
            "row without parent",
            file(&ROW.replace("parent = \"top\"\n", "")),
            false,
        ),
        (
            "row without id",
            file(&ROW.replace("id = \"extra\"\n", "")),
            false,
        ),
        (
            "a choice row",
            file(&ROW.replace("action", "choice")),
            false,
        ),
        ("unknown key", file(&format!("{ROW}colour = 1\n")), false),
        (
            "only_if_missing is a boolean",
            file(&format!("{ROW}only_if_missing = \"yes\"\n")),
            false,
        ),
        (
            "after a sibling",
            file(&format!(
                "{ROW}after = \"campaign\"\nonly_if_missing = true\n"
            )),
            true,
        ),
    ]
}

#[test]
fn the_mod_menu_schema_and_the_loader_agree_on_every_example() {
    let strings = StringTable::default();
    let english = oag_ui::strings::project_table(None);
    let extras = oag_mod::extras::builtin().unwrap().menu().unwrap().unwrap();
    for (what, text, accepted) in mod_cases() {
        let schema = oag_schema::validate::accepts("mod-menu", 1, &text);
        let mods = [
            oag_ui::menu::ModMenu {
                id: "oag.extras",
                text: &extras,
            },
            oag_ui::menu::ModMenu {
                id: "t",
                text: &text,
            },
        ];
        let loader = Definition::build(oag_ui::menu::BUILT_IN, &strings, &english, None, &mods);
        assert_eq!(schema.is_ok(), accepted, "schema, {what}: {schema:?}");
        assert_eq!(
            loader.is_ok(),
            accepted,
            "loader, {what}: {:?}",
            loader.err()
        );
    }
}
