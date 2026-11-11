use super::*;
use crate::menu::Entry;

const HEADER: &str = "toml-schema = { location = \"https://oag.topaxi.com/mods/schemas/v1/mod-menu.tosd\", version = \"1.0.0\" }\n";

fn extras() -> String {
    oag_mod::extras::builtin().unwrap().menu().unwrap().unwrap()
}

fn english() -> StringTable {
    crate::strings::project_table(None)
}

/// The main page's row ids in drawn order, with the attached ones marked.
fn main_rows(title: Option<&oag_title::Title>, mod_text: &str) -> Result<Vec<String>, Error> {
    let extras = extras();
    let mods = [
        ModMenu {
            id: "oag.extras",
            text: &extras,
        },
        ModMenu {
            id: "test.mod",
            text: mod_text,
        },
    ];
    let english = english();
    let (tree, _) = Definition::build(
        crate::menu::BUILT_IN,
        &english,
        &english,
        title.map(|title| For {
            title,
            updated: false,
        }),
        &mods,
    )?;
    let page = &tree.pages[tree.root];
    Ok(page
        .meta
        .iter()
        .map(|meta| {
            let id = meta.id.clone().unwrap_or_default();
            match &meta.parent {
                Some(parent) => format!("{parent}>{id}"),
                None => id,
            }
        })
        .collect())
}

fn row(rest: &str) -> String {
    format!("{HEADER}[[entry]]\nkind = \"action\"\nlabel = \"X\"\naction = \"quit\"\n{rest}")
}

#[test]
fn every_title_shows_its_own_top_level_and_the_extras_attach_where_they_say() {
    // Pulse and Pure draw an attached row inline: the order is the list. Each shows MULTIPLAYER
    // where its own disc has its multiplayer entry (Pulse's `FE_MP` after the race box, Pure's
    // `Multiplayer` after `Single Player`, with RACEBOX from the extras in between: the same list).
    let pulse = [
        "campaign",
        "racebox",
        "racebox>replays",
        "multiplayer",
        "options",
        "options>records",
        "options>mods",
        "options>quit",
        "photo_mode",
    ];
    assert_eq!(main_rows(Some(oag_pulse::TITLE), HEADER).unwrap(), pulse);
    assert_eq!(main_rows(Some(oag_pure::TITLE), HEADER).unwrap(), pulse);
    // HD and Omega open on RECORDS themselves, so it is no one's child.
    let hd = [
        "campaign",
        "racebox",
        "racebox>replays",
        "multiplayer",
        "options",
        "options>mods",
        "options>quit",
        "photo_mode",
        "records",
    ];
    assert_eq!(main_rows(Some(oag_hd::TITLE), HEADER).unwrap(), hd);
    assert_eq!(main_rows(Some(oag_omega::TITLE), HEADER).unwrap(), hd);
    // 2048's grid is the disc's own touch front end; it builds no row, so
    // every one on its tree is ours.
    assert_eq!(
        main_rows(Some(oag_2048::TITLE), HEADER).unwrap(),
        [
            "racebox",
            "racebox>replays",
            "multiplayer",
            "options",
            "options>records",
            "options>mods",
            "options>quit",
            "photo_mode"
        ]
    );
}

#[test]
fn no_title_keeps_every_vanilla_row_and_the_extras_around_them() {
    assert_eq!(
        main_rows(None, HEADER).unwrap(),
        [
            "campaign",
            "multiplayer",
            "racebox",
            "racebox>replays",
            "options",
            "options>mods",
            "options>quit",
            "photo_mode",
            "records"
        ]
    );
}

#[test]
fn a_mod_row_goes_after_a_sibling_or_at_the_end_of_its_parents_rows() {
    let rows = main_rows(
        Some(oag_hd::TITLE),
        &row("id = \"a\"\nparent = \"racebox\"\n"),
    )
    .unwrap();
    // REPLAYS hangs off RACEBOX from the extras, so a mod's row lands after it.
    assert_eq!(rows[1..4], ["racebox", "racebox>replays", "racebox>a"]);

    let two = format!(
        "{}[[entry]]\nid = \"b\"\nparent = \"racebox\"\nafter = \"racebox\"\nkind = \"action\"\nlabel = \"X\"\naction = \"quit\"\n",
        row("id = \"a\"\nparent = \"racebox\"\n")
    );
    let rows = main_rows(Some(oag_hd::TITLE), &two).unwrap();
    assert_eq!(
        rows[1..5],
        ["racebox", "racebox>b", "racebox>replays", "racebox>a"]
    );

    let rows = main_rows(
        Some(oag_hd::TITLE),
        &row("id = \"c\"\nparent = \"top\"\nafter = \"campaign\"\n"),
    )
    .unwrap();
    assert_eq!(rows[..2], ["campaign", "c"]);
}

#[test]
fn a_parent_the_title_lacks_but_the_tree_knows_places_the_row_at_the_top() {
    // `records` is HD's own row and attached to OPTIONS elsewhere; a mod naming
    // `campaign` on 2048 (which has no such row) falls to the top level.
    let rows = main_rows(
        Some(oag_2048::TITLE),
        &row("id = \"d\"\nparent = \"campaign\"\n"),
    )
    .unwrap();
    assert_eq!(rows.last().unwrap(), "d");
}

#[test]
fn a_mod_that_does_not_fit_is_refused_naming_the_mod_and_the_key() {
    let problems = [
        (row("id = \"x\"\nparent = \"nowhere\"\n"), "parent"),
        (
            row("id = \"x\"\nparent = \"top\"\nafter = \"nowhere\"\n"),
            "after",
        ),
        (row("id = \"x\"\nparent = \"remix\"\n"), "parent"),
        (row("id = \"racebox\"\nparent = \"top\"\n"), "id"),
        (
            row("id = \"x\"\nparent = \"top\"\nstring_id = \"NO_SUCH_STRING\"\n"),
            "string_id",
        ),
        (
            row("id = \"x\"\nparent = \"top\"\n").replace("\"quit\"", "\"launch_nothing\""),
            "action",
        ),
        (
            row("id = \"x\"\nparent = \"top\"\n")
                .replace("action", "submenu")
                .replace("= \"quit\"", "= \"no_page\""),
            "",
        ),
    ];
    for (text, key) in problems {
        let error = main_rows(Some(oag_pulse::TITLE), &text).unwrap_err();
        let Error::Mod {
            mod_id, key: found, ..
        } = &error
        else {
            panic!("{text}: {error}");
        };
        assert_eq!(mod_id, "test.mod", "{error}");
        assert!(found.contains(key), "{key}: {error}");
    }
}

#[test]
fn a_title_that_names_a_row_the_tree_lacks_is_a_build_defect() {
    static BAD: oag_title::MainMenu = oag_title::MainMenu {
        rows: &[oag_title::main_menu::original("FE_X", Some("not_a_row"))],
        multiplayer: None,
        updated_rows: None,
        attached: oag_title::Attached::Inline,
        origin: oag_title::Origin::Chosen,
    };
    let title = oag_title::Title {
        main_menu: &BAD,
        ..*oag_pulse::TITLE
    };
    let error = main_rows(Some(&title), HEADER).unwrap_err();
    assert!(error.to_string().contains("not_a_row"), "{error}");
}

#[test]
fn the_extras_that_duplicate_a_vanilla_row_are_that_row() {
    let vanilla = Definition::parse_vanilla(crate::menu::BUILT_IN, &english()).unwrap();
    let main = &vanilla.pages[vanilla.root];
    let file: ModFile = toml::from_str(&extras()).unwrap();
    for id in ["racebox", "options", "records"] {
        let at = main.row_with_id(id).unwrap();
        let twin = file.entries.iter().find(|e| e.id == id).unwrap();
        assert!(twin.only_if_missing, "{id} in the extras");
        assert_eq!(twin.label, main.entries[at].label(), "{id}: label");
        assert_eq!(
            twin.subtitle.as_deref(),
            main.subtitles[at].as_deref(),
            "{id}: subtitle"
        );
        let Entry::Submenu { target, .. } = &main.entries[at] else {
            panic!("{id} is a submenu");
        };
        assert_eq!(
            twin.target.as_deref(),
            Some(vanilla.pages[*target].id.as_str()),
            "{id}: target"
        );
    }
}

fn built_for(title: &oag_title::Title) -> Definition {
    let extras = extras();
    let english = english();
    Definition::build(
        crate::menu::BUILT_IN,
        &english,
        &english,
        Some(For {
            title,
            updated: false,
        }),
        &[ModMenu {
            id: "oag.extras",
            text: &extras,
        }],
    )
    .unwrap()
    .0
}

fn main_row_ids(definition: &Definition) -> Vec<String> {
    definition.pages[definition.root]
        .meta
        .iter()
        .map(|meta| match (&meta.parent, &meta.id) {
            (Some(parent), Some(id)) => format!("{parent}>{id}"),
            (_, id) => id.clone().unwrap_or_default(),
        })
        .collect()
}

/// Every title shows exactly one MULTIPLAYER row and none of the old split-screen ones: Pulse
/// and Pure from their own disc's multiplayer entry, the other three from the built-in extras.
#[test]
fn every_title_shows_one_multiplayer_row_and_no_split_rows() {
    for title in [
        oag_pulse::TITLE,
        oag_pure::TITLE,
        oag_hd::TITLE,
        oag_omega::TITLE,
        oag_2048::TITLE,
    ] {
        let rows = main_row_ids(&built_for(title));
        assert_eq!(
            rows.iter().filter(|row| *row == "multiplayer").count(),
            1,
            "{}: {rows:?}",
            title.name
        );
        assert!(
            rows.iter().all(|row| !row.contains("split")),
            "{}: {rows:?}",
            title.name
        );
    }
    let own = |title: &oag_title::Title| {
        title
            .main_menu
            .rows
            .iter()
            .any(|row| row.builds == Some(oag_title::main_menu::MULTIPLAYER))
    };
    assert!(own(oag_pulse::TITLE) && own(oag_pure::TITLE));
    assert!(!own(oag_hd::TITLE) && !own(oag_omega::TITLE) && !own(oag_2048::TITLE));
}

/// Pulse words the row as its disc does (`FE_MP`) and the others keep the project's.
#[test]
fn pulse_words_the_multiplayer_row_as_its_disc_does() {
    let english = english();
    let label_of = |title: &oag_title::Title| {
        let mut definition = built_for(title);
        definition.rename_multiplayer_row(title, &english);
        let main = &definition.pages[definition.root];
        let at = main.row_with_id("multiplayer").unwrap();
        (
            main.entries[at].label().to_string(),
            main.subtitles[at].clone().unwrap_or_default(),
        )
    };
    assert_eq!(
        label_of(oag_pulse::TITLE),
        (
            "MULTIPLAYER & SHARING".to_string(),
            "Race online or over Ad Hoc, and share data".to_string()
        )
    );
    assert_eq!(label_of(oag_pure::TITLE).0, "MULTIPLAYER");
    assert_eq!(label_of(oag_hd::TITLE).0, "MULTIPLAYER");
}

const PAGE_MOD: &str = "
[[page]]
id = \"picker\"
title = \"PICKER\"

[[page.entry]]
kind = \"toggle\"
label = \"SPARKS\"
setting = \"mod.test.mod.bloom\"

[[page.entry]]
kind = \"submenu\"
label = \"DEEPER\"
target = \"deeper\"

[[page]]
id = \"deeper\"
title = \"DEEPER\"

[[page.entry]]
kind = \"action\"
label = \"BACK\"
action = \"quit\"

[[entry]]
id = \"picker_row\"
parent = \"racebox\"
kind = \"submenu\"
label = \"PICKER\"
target = \"picker\"
";

fn build_with(mod_text: &str) -> Result<(Definition, Vec<Added>), Error> {
    let extras = extras();
    let mods = [
        ModMenu {
            id: "oag.extras",
            text: &extras,
        },
        ModMenu {
            id: "test.mod",
            text: mod_text,
        },
    ];
    let english = english();
    Definition::build(crate::menu::BUILT_IN, &english, &english, None, &mods)
}

fn with_header(body: &str) -> String {
    format!("{HEADER}{body}")
}

fn mod_error(mod_text: &str) -> (String, String) {
    match build_with(&with_header(mod_text)) {
        Err(Error::Mod { mod_id, key, .. }) => (mod_id, key),
        other => panic!("expected a mod error, got {:?}", other.map(|(_, a)| a)),
    }
}

#[test]
fn a_mod_page_merges_under_its_namespaced_id_and_its_row_reaches_it() {
    let (tree, report) = build_with(&with_header(PAGE_MOD)).unwrap();
    let picker = tree.page("test.mod/picker").expect("the mod's page");
    assert_eq!(picker.entries.len(), 2);
    assert!(tree.page("test.mod/deeper").is_some());
    assert!(
        tree.page("picker").is_none(),
        "the short id is not a tree id"
    );
    let added = report.iter().find(|a| a.mod_id == "test.mod").unwrap();
    assert_eq!(added.pages, ["test.mod/picker", "test.mod/deeper"]);
    let at = tree
        .pages
        .iter()
        .position(|p| p.id == "test.mod/picker")
        .unwrap();
    let racebox = tree
        .pages
        .iter()
        .flat_map(|page| page.entries.iter())
        .any(|entry| matches!(entry, Entry::Submenu { target, .. } if *target == at));
    assert!(racebox, "the mod's row opens its page");
}

#[test]
fn the_shipped_vanilla_file_still_parses_without_a_mod() {
    Definition::parse_vanilla(crate::menu::BUILT_IN, &english()).unwrap();
}

#[test]
fn a_mod_page_no_entry_reaches_names_the_mod_and_the_page() {
    let orphan = "[[page]]\nid = \"lost\"\ntitle = \"LOST\"\n";
    assert_eq!(
        mod_error(orphan),
        ("test.mod".to_string(), "page \"lost\"".to_string())
    );
}

#[test]
fn a_mod_page_id_must_be_plain_and_unique() {
    let slash = "[[page]]\nid = \"a/b\"\ntitle = \"X\"\n";
    assert_eq!(mod_error(slash).1, "page \"a/b\" id");
    let twice = "[[page]]\nid = \"a\"\ntitle = \"X\"\n[[page]]\nid = \"a\"\ntitle = \"Y\"\n";
    assert_eq!(mod_error(twice).1, "page \"a\" id");
}

#[test]
fn a_mod_page_row_naming_a_missing_page_or_string_names_the_row() {
    let target = "[[page]]\nid = \"a\"\ntitle = \"X\"\n[[page.entry]]\nkind = \"submenu\"\nlabel = \"Y\"\ntarget = \"nowhere\"\n[[entry]]\nid = \"r\"\nparent = \"top\"\nkind = \"submenu\"\nlabel = \"A\"\ntarget = \"a\"\n";
    assert_eq!(
        mod_error(target),
        (
            "test.mod".to_string(),
            "page \"a\" entry 0 target".to_string()
        )
    );
    let string = "[[page]]\nid = \"a\"\ntitle = \"X\"\ntitle_string_id = \"NO_SUCH_STRING\"\n";
    assert_eq!(mod_error(string).1, "page \"a\" title_string_id");
}

#[test]
fn a_mod_cannot_name_another_mods_page_by_its_short_id() {
    let extras = extras();
    let owner = with_header(PAGE_MOD);
    let thief = with_header(
        "[[entry]]\nid = \"t\"\nparent = \"top\"\nkind = \"submenu\"\nlabel = \"T\"\ntarget = \"picker\"\n",
    );
    let mods = [
        ModMenu {
            id: "oag.extras",
            text: &extras,
        },
        ModMenu {
            id: "test.mod",
            text: &owner,
        },
        ModMenu {
            id: "thief.mod",
            text: &thief,
        },
    ];
    let english = english();
    match Definition::build(crate::menu::BUILT_IN, &english, &english, None, &mods) {
        Err(Error::Mod { mod_id, key, .. }) => {
            assert_eq!(
                (mod_id.as_str(), key.as_str()),
                ("thief.mod", "entry \"t\" target")
            );
        }
        other => panic!(
            "expected the thief to be refused, got {:?}",
            other.map(|(_, a)| a)
        ),
    }
}

#[test]
fn a_template_choice_row_on_a_mod_page_expands_like_one_on_an_engine_page() {
    use crate::menu::{Choice, Menu, RowItem, ValueSource};
    let text = "[[page]]\nid = \"pool\"\ntitle = \"POOL\"\n[[page.entry]]\nkind = \"choice\"\nlabel = \"SOURCE\"\nsetting = \"mod.test.mod.pool\"\nvalues_from = \"music_pool\"\n[[entry]]\nid = \"pool_row\"\nparent = \"top\"\nkind = \"submenu\"\nlabel = \"POOL\"\ntarget = \"pool\"\n";
    let (mut tree, _) = build_with(&with_header(text)).unwrap();
    let items: Vec<RowItem> = ["a", "b"]
        .iter()
        .map(|id| RowItem {
            id: (*id).to_string(),
            label: id.to_uppercase(),
            detail: None,
            values: vec![Choice::labelled("on", "ON"), Choice::labelled("off", "OFF")],
            current: "off".to_string(),
        })
        .collect();
    // The engine's own MUSIC template expands in the same call.
    assert!(tree.expand_rows(ValueSource::MusicPool, &items) >= 2);
    let page = tree.page("test.mod/pool").expect("the mod's page");
    assert_eq!(page.entries.len(), 2, "the template is replaced, not kept");
    assert_eq!(page.entries.len(), page.meta.len());
    assert_eq!(page.entries[1].setting(), Some("mod.test.mod.pool.b"));
    let menu = Menu::new(tree);
    assert_eq!(
        menu.expanded_settings("mod.test.mod.pool"),
        ["mod.test.mod.pool.a", "mod.test.mod.pool.b"]
    );
}
#[test]
fn a_mod_page_query_row_checks_its_of_against_the_mods_own_settings() {
    let page = |of: &str| {
        format!(
            "[[page]]\nid = \"a\"\ntitle = \"X\"\n[[page.entry]]\nkind = \"choice\"\nlabel = \"T\"\nsetting = \"mod.test.mod.title\"\nvalues_from = \"titles\"\n[[page.entry]]\nkind = \"choice\"\nlabel = \"R\"\nsetting = \"mod.test.mod.track\"\nvalues_from = {{ query = \"tracks_of\", of = \"{of}\" }}\n[[entry]]\nid = \"r\"\nparent = \"top\"\nkind = \"submenu\"\nlabel = \"A\"\ntarget = \"a\"\n"
        )
    };
    let (tree, _) = build_with(&with_header(&page("mod.test.mod.title"))).unwrap();
    let Entry::Choice { query, .. } = &tree.page("test.mod/a").unwrap().entries[1] else {
        panic!("a choice row");
    };
    assert_eq!(query.as_ref().unwrap().of, "mod.test.mod.title");
    assert_eq!(
        mod_error(&page("mod.test.mod.nothing")),
        (
            "test.mod".to_string(),
            "page \"a\" entry 1 values_from".to_string()
        )
    );
}

const SETTINGS_ONLY: &str = "settings = \"picker\"
[[page]]
id = \"picker\"
title = \"PICKER\"

[[page.entry]]
kind = \"toggle\"
label = \"SPARKS\"
setting = \"mod.test.mod.bloom\"
";

#[test]
fn a_settings_page_needs_no_row_on_the_main_menu() {
    let (tree, added) = build_with(&with_header(SETTINGS_ONLY)).unwrap();
    assert_eq!(
        tree.mod_settings.get("test.mod").map(String::as_str),
        Some("test.mod/picker")
    );
    let test = added.iter().find(|a| a.mod_id == "test.mod").unwrap();
    assert!(test.rows.is_empty(), "no row: {:?}", test.rows);
    assert_eq!(test.pages, ["test.mod/picker"]);
}

#[test]
fn a_settings_key_naming_no_page_of_the_mod_is_refused() {
    let text = SETTINGS_ONLY.replace("settings = \"picker\"", "settings = \"options\"");
    assert_eq!(
        mod_error(&text),
        ("test.mod".to_string(), "settings".to_string())
    );
}
