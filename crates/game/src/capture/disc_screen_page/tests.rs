use super::*;

/// Each Pulse PSP pressing present, with its English string table.
fn discs() -> Vec<(String, StringTable)> {
    ["pulse-psp-eu.chd", "pulse-psp-usa.chd"]
        .into_iter()
        .filter_map(|name| oag_testdata::image(&format!("data/images/{name}")))
        .map(|path| {
            let source = path.display().to_string();
            let mut archives = oag_pulse::open(&source).expect("opening the disc");
            // English is `PI000` on the EU pressing and `PI012` on the USA one.
            let blob = ["PI000", "PI012"]
                .iter()
                .find_map(|plugin| {
                    archives
                        .read_name(&oag_pulse::names::language_entries(plugin))
                        .ok()
                })
                .expect("an English plugin on the PSP disc");
            let table = StringTable::from_xml(&oag_tables::fexml::text(&blob).expect("text"));
            (source, table)
        })
        .collect()
}

fn page_of(name: &str, source: &str, strings: &StringTable) -> Page {
    page(
        oag_pulse::TITLE,
        spec(oag_pulse::TITLE, name).expect("a disc screen"),
        source,
        strings,
        0,
    )
    .expect("the screen reads")
}

/// The four pages are what the disc authors: the rows `Title::main_menu` lists, in its
/// order, the Racebox's three, the settings lists, and each page's header.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_four_pages_are_the_discs_own_rows() {
    let discs = discs();
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none() || !discs.is_empty(),
        "OAG_REQUIRE_GAME_DATA is set but neither Pulse PSP image is present"
    );
    for (source, strings) in discs {
        let labels = |page: &Page| -> Vec<String> {
            page.entries
                .iter()
                .map(|entry| entry.label().to_string())
                .collect()
        };
        let main = page_of("pulse-main", &source, &strings);
        let wanted: Vec<String> = oag_pulse::TITLE
            .main_menu
            .rows
            .iter()
            .map(|row| strings.get_or_id(row.disc_id).to_string())
            .collect();
        assert_eq!(labels(&main), wanted, "{source}: Main Menu's rows");
        assert_eq!(main.title, strings.get_or_id("FE_MM"));
        assert_eq!(
            main.subtitles.first(),
            Some(&Some(strings.get_or_id("FE_HELP_RC").to_string())),
            "{source}: helptext0 under the first row"
        );
        assert!(main.subtitles.iter().all(Option::is_some));

        let racebox = page_of("pulse-racebox", &source, &strings);
        assert_eq!(
            labels(&racebox),
            ["RB_SP", "RB_LOAD_GRID", "RB_EDIT_GRID"].map(|id| strings.get_or_id(id).to_string())
        );
        assert_eq!(racebox.title, strings.get_or_id("FE_RACEBOX"));

        let options = page_of("pulse-options", &source, &strings);
        assert_eq!(
            labels(&options),
            ["OPT_GAME_OPT", "OPT_CTRL", "OPT_MUSIC_PL"]
                .map(|id| strings.get_or_id(id).to_string())
        );
        assert_eq!(options.title, strings.get_or_id("FE_OPT_PLUS"));

        let single = page_of("pulse-single", &source, &strings);
        assert_eq!(
            labels(&single),
            ["RC_RT", "RC_SC", "RB_WEAP", "RB_AI_DIF", "IG_HUD_KILLS"]
                .map(|id| strings.get_or_id(id).to_string())
        );
        let [
            Entry::Choice { values, .. },
            ..,
            Entry::Choice { disabled_by, .. },
        ] = single.entries.as_slice()
        else {
            panic!("{source}: the settings rows are choices");
        };
        assert_eq!(values.len(), 7, "{source}: seven race types");
        let held = Condition::matches;
        let kills = disabled_by.as_ref().expect("KILLS is conditional");
        assert!(held(kills, Some(&Value::Text("Arcade".into()))));
        assert!(!held(kills, Some(&Value::Text("Elimination".into()))));
    }
}
