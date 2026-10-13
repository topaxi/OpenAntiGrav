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
    // Pulse and Pure draw an attached row inline: the order is the list. Both show the
    // PS2 pressing's two split rows right after the campaign (borrowed on the PSP and on
    // Pure), so the extras' generic SPLIT SCREEN row is skipped.
    let flat = [
        "campaign",
        "split_vertical",
        "split_horizontal",
        "racebox",
        "racebox>remix",
        "options",
        "options>records",
        "options>mods",
        "options>quit",
    ];
    assert_eq!(main_rows(Some(oag_pulse::TITLE), HEADER).unwrap(), flat);
    assert_eq!(main_rows(Some(oag_pure::TITLE), HEADER).unwrap(), flat);
    // HD and Omega open on RECORDS themselves, so it is no one's child.
    let hd = [
        "campaign",
        "racebox",
        "racebox>remix",
        "racebox>split_screen",
        "options",
        "options>mods",
        "options>quit",
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
            "racebox>remix",
            "racebox>split_screen",
            "options",
            "options>records",
            "options>mods",
            "options>quit"
        ]
    );
}

#[test]
fn no_title_keeps_every_vanilla_row_and_the_extras_around_them() {
    assert_eq!(
        main_rows(None, HEADER).unwrap(),
        [
            "campaign",
            "split_vertical",
            "split_horizontal",
            "racebox",
            "racebox>remix",
            "racebox>split_screen",
            "options",
            "options>mods",
            "options>quit",
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
    assert_eq!(
        rows[1..5],
        [
            "racebox",
            "racebox>remix",
            "racebox>split_screen",
            "racebox>a"
        ]
    );

    let rows = main_rows(
        Some(oag_hd::TITLE),
        &row("id = \"b\"\nparent = \"racebox\"\nafter = \"racebox\"\n"),
    )
    .unwrap();
    assert_eq!(rows[1..4], ["racebox", "racebox>b", "racebox>remix"]);

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
        borrowed: &[],
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

/// The integrated-GPU rule drops the one row and nothing around it: REMIX stays under
/// RACEBOX, and a definition it is not applied to still has the row.
#[test]
fn dropping_the_split_screen_row_leaves_every_other_row() {
    for title in [oag_hd::TITLE, oag_omega::TITLE, oag_2048::TITLE] {
        let mut definition = built_for(title);
        let before = main_row_ids(&definition);
        assert!(
            before.iter().any(|row| row == "racebox>split_screen"),
            "{}: {before:?}",
            title.name
        );
        definition.drop_split_screen();
        let after = main_row_ids(&definition);
        let expected: Vec<String> = before
            .into_iter()
            .filter(|row| row != "racebox>split_screen")
            .collect();
        assert_eq!(after, expected, "{}", title.name);
        assert!(after.iter().any(|row| row == "racebox>remix"));
    }
}

/// Pulse and Pure show the PS2 pressing's two split rows after the campaign and skip the
/// extras' generic one; HD, Omega and 2048 show only the generic one. The iGPU trim takes
/// every split entry and nothing else.
#[test]
fn the_split_rows_are_pulses_and_pures_and_the_generic_row_is_everyone_elses() {
    for title in [oag_pulse::TITLE, oag_pure::TITLE] {
        let mut definition = built_for(title);
        let rows = main_row_ids(&definition);
        assert!(title.main_menu.has_split_rows(), "{}", title.name);
        assert!(
            !rows.iter().any(|row| row.ends_with("split_screen")),
            "{}: {rows:?}",
            title.name
        );
        definition.drop_split_screen();
        let trimmed = main_row_ids(&definition);
        let expected: Vec<String> = rows
            .into_iter()
            .filter(|row| row != "split_vertical" && row != "split_horizontal")
            .collect();
        assert_eq!(trimmed, expected, "{}", title.name);
    }
    for title in [oag_hd::TITLE, oag_omega::TITLE, oag_2048::TITLE] {
        let rows = main_row_ids(&built_for(title));
        assert!(!title.main_menu.has_split_rows(), "{}", title.name);
        assert!(
            rows.iter()
                .all(|row| !row.contains("split_vertical") && !row.contains("split_horizontal")),
            "{}: {rows:?}",
            title.name
        );
    }
}

/// A borrowed row follows a row the title's list has: an anchor that names nothing would
/// drop the row without a word, and the title's main menu would quietly lose its split
/// rows.
#[test]
fn every_borrowed_row_follows_a_row_its_title_lists() {
    for title in [
        oag_pulse::TITLE,
        oag_pure::TITLE,
        oag_hd::TITLE,
        oag_omega::TITLE,
        oag_2048::TITLE,
    ] {
        let menu = title.main_menu;
        for borrowed in menu.borrowed {
            assert!(
                menu.rows
                    .iter()
                    .any(|row| row.disc_id == borrowed.after && row.builds.is_some()),
                "{}: {} follows {:?}, which is no row it builds",
                title.name,
                borrowed.disc_id,
                borrowed.after
            );
        }
    }
}

/// Both pressings of Pulse and Pure borrow the same two rows in the same order, the PS2's
/// (`FE_RACE_CAM_SS` vertical, `FE_RACE_CAM_SSH` horizontal).
#[test]
fn pulse_and_pure_borrow_the_ps2s_two_rows_in_its_order() {
    for title in [oag_pulse::TITLE, oag_pure::TITLE] {
        let rows: Vec<(&str, &str)> = title
            .main_menu
            .borrowed
            .iter()
            .map(|row| (row.disc_id, row.builds))
            .collect();
        assert_eq!(
            rows,
            [
                ("FE_RACE_CAM_SS", "split_vertical"),
                ("FE_RACE_CAM_SSH", "split_horizontal")
            ],
            "{}",
            title.name
        );
    }
}
