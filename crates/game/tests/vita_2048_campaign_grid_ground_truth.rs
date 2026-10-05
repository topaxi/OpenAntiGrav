//! `M_PGRIDSHIPMODELDATA` wired into the AI grid: `"2048 - Event 6"`'s own
//! seven authored opponents reach `race::load_event`'s output roster, in
//! place of `oag_livery::teams_for_slots`'s own roster draw (Pulse's
//! law, inherited) - see `oag_2048::campaign::craft::grid_craft`'s own doc comment
//! for the census this pins one row of, and `docs/formats/2048-campaign.md`'s
//! "Craft choice" section for the full write-up.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship - `data/extracted/vita/PCSF00007/base`, the
//! decrypted EU package. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(vita_2048_campaign_grid_ground_truth)'
//! ```

use std::path::{Path, PathBuf};

fn source() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007");
    if path.join("base/PSP2/data.psarc").exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

fn sp_xml_document() -> Option<oag_tables::mjolnir::Document> {
    let source = source()?;
    let mut archives = oag_2048::open(&source.display().to_string()).expect("mounting 2048");
    let bytes = archives
        .read_name(oag_2048::campaign::SP_XML)
        .unwrap_or_else(|e| panic!("reading {}: {e}", oag_2048::campaign::SP_XML));
    let text = String::from_utf8(bytes).expect("SP.xml is not valid UTF-8");
    Some(oag_tables::mjolnir::parse(&text))
}

/// `"2048 - Event 6"`'s own `M_PGRIDSHIPMODELDATA`, measured directly
/// against the real EU v1.04 file: all 7 slots authored, every one
/// resolving - three `Feisar2048` (`combat`), two `Auricom2048` (`combat`),
/// two `AG_Systems2048` (`combat`). Read at the data layer alone, with no
/// `race::load_event`/loader involved, so a failure here says "the parse
/// drifted" rather than "the wiring did".
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn event_6_authors_all_seven_grid_slots_and_they_resolve() {
    let Some(doc) = sp_xml_document() else {
        return;
    };
    let instance = doc
        .instance_named("2048 - Event 6")
        .expect("\"2048 - Event 6\" is not in SP.xml");

    let grid = oag_2048::campaign::craft::grid_craft(&doc, instance);
    assert_eq!(
        grid,
        vec![
            Some(r"Feisar2048\1".to_string()),
            Some(r"Feisar2048\1".to_string()),
            Some(r"Feisar2048\1".to_string()),
            Some(r"Auricom2048\1".to_string()),
            Some(r"Auricom2048\1".to_string()),
            Some(r"AG_Systems2048\1".to_string()),
            Some(r"AG_Systems2048\1".to_string()),
        ],
        "grid_craft drifted from the real file's own M_PGRIDSHIPMODELDATA"
    );
}

/// The same event through the actual launch path: `race::load_event` places
/// the same seven teams onto the AI grid slots, and the player's own choice
/// (forced to a fixed team here so the assertion does not depend on this
/// title's own default) still lands in slot 0 untouched.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn load_event_places_event_6s_authored_roster_on_the_ai_grid() {
    let Some(source) = source() else { return };
    let options = oag_game::race::Options {
        source: source.display().to_string(),
        team: Some(r"Qirex2048\2".to_string()),
        ..oag_game::race::Options::default()
    };
    let loaded = oag_game::race::load_event(&options, "2048 - Event 6").expect("the event loads");

    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.contains("grid: 7 of 7 AI slot(s) authored")),
        "report never mentions the grid override: {:?}",
        loaded.report
    );
    let expected_roster = [
        r"Qirex2048\2", // slot 0, the player's own forced pick above
        r"Feisar2048\1",
        r"Feisar2048\1",
        r"Feisar2048\1",
        r"Auricom2048\1",
        r"Auricom2048\1",
        r"AG_Systems2048\1",
        r"AG_Systems2048\1",
    ]
    .join(", ");
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.contains(&format!("grid liveries: {expected_roster}"))),
        "the resolved slot roster does not match \"2048 - Event 6\"'s own authored grid: {:?}",
        loaded.report
    );
}
