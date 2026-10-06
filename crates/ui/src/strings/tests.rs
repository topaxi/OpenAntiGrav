use super::*;

#[test]
fn a_language_with_no_file_overlays_nothing() {
    let mut table = StringTable::from_xml(r#"<StringTable></StringTable>"#);
    let mut report = Vec::new();
    overlay(&mut table, "Klingon", &mut report);
    assert!(table.is_empty());
    assert!(report.is_empty());
}

/// **This is the test that used to assert the file was empty**, with a
/// comment saying it would start failing the moment something looked a
/// project string up by id. That happened: the pilot editor's CRUD rows
/// and the on-screen keyboard it opens are the first screen in this build
/// whose text is looked up rather than written into the widget.
///
/// What it pins now is the property that still matters - every id in the
/// file is one of ours. The `OAG_` prefix is what keeps a project entry
/// from colliding with a disc idstring, and this file shares one id space
/// with the disc's own by design. An id landing here without it would
/// silently override a real front-end string on somebody's disc.
///
/// It deliberately does **not** list the ids: that would be a second
/// place to update for every string added, which is exactly the kind of
/// bookkeeping nobody does.
#[test]
fn every_english_entry_is_one_of_ours_and_reads_back() {
    let mut table = StringTable::default();
    let mut report = Vec::new();
    overlay(&mut table, "English", &mut report);
    // The report is a *count*, not an error list - it used to be empty
    // only because the file was.
    assert_eq!(report.len(), 1, "{report:?}");
    assert!(report[0].contains("english.toml"), "{report:?}");
    assert!(!table.is_empty(), "the file has entries now");
    // One of them, spot-checked, so "not empty" cannot pass on a file
    // that parsed into something unrelated.
    assert_eq!(table.get("OAG_PILOT_RENAME"), Some("RENAME"));

    let text = built_in("English").expect("English ships a file");
    let file: File = toml::from_str(text).expect("the shipped file must parse");
    for id in file.strings.keys() {
        assert!(
            id.starts_with("OAG_"),
            "{id:?} has no OAG_ prefix, so it would override a disc idstring \
             of that name rather than adding one of ours"
        );
    }
}

#[test]
fn a_project_entry_overrides_a_disc_entry_of_the_same_id() {
    let mut table = StringTable::from_xml(
        r#"<StringTable><Entry ID="FE_CONFIRM" String="Confirm"></Entry></StringTable>"#,
    );
    table.merge(HashMap::from([(
        "FE_CONFIRM".to_string(),
        "OK".to_string(),
    )]));
    assert_eq!(table.get("FE_CONFIRM"), Some("OK"));
}

#[test]
fn a_malformed_file_is_an_error_not_a_panic() {
    assert!(toml::from_str::<File>("not = [valid").is_err());
}

#[test]
fn project_table_defaults_to_english_with_no_language_named() {
    // Both empty today, so this only proves they take the same path -
    // see `english_parses_and_is_empty_today` for why that is expected.
    assert_eq!(
        project_table(None).len(),
        project_table(Some("English")).len()
    );
}

#[test]
fn project_table_is_empty_for_a_language_this_build_ships_no_file_for() {
    assert!(project_table(Some("Klingon")).is_empty());
}

/// **The check that catches a shipped file `built_in()` forgot to embed.**
/// `check-strings.py` validates every `assets/ui/strings/*.toml` on disk,
/// but nothing there proves the *binary* ever reads one back - a file
/// this script covers and `built_in()` does not match by name would pass
/// the gate while shipping no translation at all, the same "kept in step
/// by hand" gap that script's own module doc already admits to for
/// `STRING_CONSUMERS`.
#[test]
fn every_file_under_assets_ui_strings_is_reachable_through_built_in() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/ui/strings");
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).expect("assets/ui/strings must exist") {
        let path = entry.expect("a readable dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("a .toml file has a stem");
        assert!(
            built_in(stem).is_some(),
            "{stem}.toml exists under assets/ui/strings/ but built_in() does not embed it"
        );
        checked += 1;
    }
    assert!(
        checked >= 3,
        "expected english, french and portuguesebr here"
    );

    // A title's disc-keyed files too: one with no `built_in_disc` arm validates
    // in `check-strings` and ships nothing.
    let mut disc_checked = 0;
    for namespace in std::fs::read_dir(dir.join("disc")).expect("assets/ui/strings/disc") {
        let namespace = namespace.expect("a readable dir entry").path();
        let name = namespace
            .file_name()
            .and_then(|n| n.to_str())
            .expect("a name")
            .to_string();
        for entry in std::fs::read_dir(&namespace).expect("a namespace directory") {
            let path = entry.expect("a readable dir entry").path();
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .expect("a stem")
                .to_string();
            assert!(
                built_in_disc(&name, &stem).is_some(),
                "disc/{name}/{stem}.toml exists but built_in_disc() does not embed it"
            );
            disc_checked += 1;
        }
    }
    assert!(disc_checked >= 1, "expected disc/pulse/portuguesebr.toml");
}

/// French translates what the maintainer was confident about and defers
/// the rest under `[untranslated]` - see `french.toml`'s own header. A
/// deferred id must not appear in the merged table at all, so a caller's
/// own English fallback (or the disc's own French entry, on the boot
/// path) is what a player actually sees for it.
#[test]
fn french_translates_a_confident_id_and_defers_an_unconfident_one() {
    let mut table = StringTable::default();
    let mut report = Vec::new();
    overlay(&mut table, "French", &mut report);
    assert_eq!(table.get("OAG_MENU_RACEBOX"), Some("COURSE"));
    assert_eq!(table.get("OAG_CONTROLS_SIDESHIFT"), None);
}

/// The loader's `human` field is required: a file that forgets it must not
/// parse into a silent `false`, which would make the flag meaningless.
#[test]
fn an_entry_without_a_human_flag_is_a_parse_error() {
    assert!(toml::from_str::<File>("[strings]\nA = { text = \"x\" }").is_err());
    assert!(toml::from_str::<File>("[strings]\nA = \"x\"").is_err());
    assert!(toml::from_str::<File>("[strings]\nA = { text = \"x\", human = false }").is_ok());
}

/// `overlay` only reports a parse error and drops the whole file, so a shipped
/// file that stopped parsing would pass every other test as an empty overlay.
#[test]
fn every_shipped_file_parses() {
    for language in ["English", "French", "PortugueseBR"] {
        let text = built_in(language).expect("a shipped file");
        toml::from_str::<File>(text).unwrap_or_else(|e| panic!("{language}: {e}"));
    }
    let text = built_in_disc("pulse", "PortugueseBR").expect("a shipped disc file");
    let file: File = toml::from_str(text).expect("the disc file parses");
    assert!(!file.strings.is_empty());
    assert!(
        file.strings.keys().all(|id| !id.starts_with("OAG_")),
        "disc-keyed translations are keyed by the disc's ids; ours live in portuguesebr.toml"
    );
}

#[test]
fn a_disc_namespace_a_title_does_not_name_reads_nothing() {
    let mut table = StringTable::default();
    let mut report = Vec::new();
    overlay_disc(&mut table, "omega", "PortugueseBR", &mut report);
    overlay_disc(&mut table, "pulse", "French", &mut report);
    assert!(table.is_empty());
    assert!(report.is_empty());
}

#[test]
fn our_ids_win_over_the_disc_keyed_file_and_both_over_the_disc() {
    let mut table = StringTable::from_xml(
        r#"<StringTable><Entry ID="FE_MM" String="MAIN MENU"></Entry><Entry ID="FE_ZZ_NOT_TRANSLATED" String="kept"></Entry></StringTable>"#,
    );
    let mut report = Vec::new();
    overlay_disc(&mut table, "pulse", "PortugueseBR", &mut report);
    overlay(&mut table, "PortugueseBR", &mut report);
    assert_ne!(
        table.get("FE_MM"),
        Some("MAIN MENU"),
        "translated: {report:?}"
    );
    assert_eq!(
        table.get("FE_ZZ_NOT_TRANSLATED"),
        Some("kept"),
        "an id we have not reached stays the disc's"
    );
    assert!(table.get("OAG_MENU_RACEBOX").is_some());
}

/// A file may name an id by its hash instead of spelling it, so a Pure string
/// keyed by English text commits no English. The hash is of the exact id.
#[test]
fn a_hashed_key_lands_on_the_id_it_hashes_and_a_plain_id_is_left_alone() {
    assert_eq!(hashed_key("Continue"), "h_ab43d664");
    assert_ne!(hashed_key("Awarded"), hashed_key("awarded"));
    let table = StringTable::from_xml(
        r#"<StringTable><Entry ID="Continue" String="Continue"></Entry><Entry ID="HUD_Lap" String="Lap"></Entry></StringTable>"#,
    );
    let texts = HashMap::from([
        (hashed_key("Continue"), "Continuar".to_string()),
        ("HUD_Lap".to_string(), "Volta".to_string()),
        (hashed_key("Not an id"), "x".to_string()),
    ]);
    let resolved = resolve_hashed_keys(&table, texts);
    assert_eq!(
        resolved.get("Continue").map(String::as_str),
        Some("Continuar")
    );
    assert_eq!(resolved.get("HUD_Lap").map(String::as_str), Some("Volta"));
    assert_eq!(
        resolved.len(),
        2,
        "an unmatched hash is dropped: {resolved:?}"
    );
}
