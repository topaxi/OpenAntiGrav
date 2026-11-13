use super::*;

const HEADER: &str = r#"toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/campaign.tosd", version = "1.0.0" }
"#;

fn with_header(body: &str) -> String {
    format!("{HEADER}{body}")
}

const SMALL: &str = r#"
[campaign]
id = "small"
name = "Small"

[[grid]]
id = "g0"

[[grid.event]]
slot = [1, 2]
mode = "race"
track = "01_Track"
class = "venom"
unlock = { open = true }

[[grid.event]]
slot = [2, 2]
mode = "time_trial"
track = "01_Track"
class = "venom"
unlock = { requires = [{ event = "g0/1-2", medal = "gold" }] }
medals = { gold = 90.0, hard = { gold = 80.0 } }
"#;

/// A file of the example mods' submodule (github.com/OpenAntiGrav/mod-examples),
/// or `None`, said on stderr, when it is not checked out.
fn example_file(path: &str) -> Option<&'static str> {
    let full = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/mods/examples")
        .join(path);
    match std::fs::read_to_string(&full) {
        Ok(text) => Some(Box::leak(text.into_boxed_str())),
        Err(_) => {
            eprintln!("skipped: {} is not checked out", full.display());
            None
        }
    }
}

#[test]
fn the_example_mod_s_campaign_parses() {
    let Some(text) = example_file("example-campaign/campaigns/night-run.toml") else {
        return;
    };
    let def = CampaignDef::parse(text).unwrap();
    assert_eq!(def.campaign.id, "night-run");
    assert_eq!(def.campaign.reveal, Reveal::Hidden);
    assert_eq!(def.grids.len(), 2);
    assert_eq!(def.grids[0].reveal, Some(Reveal::Locked));
    assert_eq!(def.grids[1].reveal, None);
    assert_eq!(def.grids[0].events.len(), 6);
    assert_eq!(def.grids[1].events.len(), 5);
    let duel = &def.grids[0].events[3];
    let field = duel.field.as_ref().unwrap();
    assert_eq!(field.ai.as_deref(), Some("ours"));
    assert_eq!(field.pilot, Some(PilotDef::Named("rival".to_string())));
    let last = &def.grids[1].events[1];
    assert!(matches!(
        last.field.as_ref().unwrap().pilot,
        Some(PilotDef::Inline(_))
    ));
    let tour = &def.grids[1].events[0];
    assert_eq!(tour.tracks.len(), 3);
    assert_eq!(
        tour.field.as_ref().unwrap().pilot,
        Some(PilotDef::Named("aggressive".to_string()))
    );
}

#[test]
fn a_small_campaign_reads_with_its_defaults() {
    let def = CampaignDef::parse(&with_header(SMALL)).unwrap();
    assert_eq!(def.campaign.reveal, Reveal::Locked);
    let grid = &def.grids[0];
    assert_eq!(grid.required_points, 0);
    assert_eq!(grid.events[0].reference(grid), "g0/1-2");
    let medals = grid.events[1].medals.as_ref().unwrap();
    assert_eq!(medals.all().gold, Some(90.0));
    assert_eq!(medals.hard.as_ref().unwrap().gold, Some(80.0));
    let requires = &grid.events[1].unlock.as_ref().unwrap().requires;
    assert_eq!(requires[0].medal, "gold");
}

#[test]
fn a_file_without_the_header_is_refused() {
    assert!(matches!(CampaignDef::parse(SMALL), Err(Error::Schema(_))));
}

#[test]
fn an_unknown_key_is_refused() {
    let text = with_header(&SMALL.replace("mode = \"race\"", "mode = \"race\"\nspeed = 3"));
    assert!(matches!(CampaignDef::parse(&text), Err(Error::Toml(_))));
}

#[test]
fn two_events_in_one_slot_are_refused() {
    let text = with_header(&SMALL.replace("slot = [2, 2]", "slot = [1, 2]"));
    let error = CampaignDef::parse(&text).unwrap_err().to_string();
    assert!(error.contains("two events in one slot"), "{error}");
}

#[test]
fn a_slot_past_nine_is_refused() {
    let text = with_header(&SMALL.replace("slot = [2, 2]", "slot = [10, 2]"));
    let error = CampaignDef::parse(&text).unwrap_err().to_string();
    assert!(error.contains("0..=9"), "{error}");
}

#[test]
fn a_requirement_naming_no_event_is_refused() {
    let text = with_header(&SMALL.replace("event = \"g0/1-2\"", "event = \"g0/5-5\""));
    let error = CampaignDef::parse(&text).unwrap_err().to_string();
    assert!(error.contains("no such event"), "{error}");
}

#[test]
fn an_unknown_medal_word_is_refused() {
    let text = with_header(&SMALL.replace("medal = \"gold\"", "medal = \"platinum\""));
    let error = CampaignDef::parse(&text).unwrap_err().to_string();
    assert!(error.contains("platinum"), "{error}");
}

#[test]
fn an_unlock_grid_naming_no_grid_is_refused() {
    let text = with_header(&SMALL.replace("id = \"g0\"", "id = \"g0\"\nunlock_grid = \"g9\""));
    let error = CampaignDef::parse(&text).unwrap_err().to_string();
    assert!(error.contains("no grid \"g9\""), "{error}");
}

#[test]
fn a_bad_campaign_id_is_refused() {
    let text = with_header(&SMALL.replace("id = \"small\"", "id = \"Small Run\""));
    let error = CampaignDef::parse(&text).unwrap_err().to_string();
    assert!(error.contains("[campaign] id"), "{error}");
}

#[test]
fn a_mod_lists_its_campaigns_and_reports_a_broken_one() {
    let (Some(manifest), Some(campaign)) = (
        example_file("example-campaign/oag-mod.toml"),
        example_file("example-campaign/campaigns/night-run.toml"),
    ) else {
        return;
    };
    let medals: &'static Vec<u8> = Box::leak(Box::new(
        std::fs::read(
            example_dir("example-campaign/ui")
                .expect("the manifest's icon is there")
                .join("medals.png"),
        )
        .expect("the example's medal atlas reads"),
    ));
    let files: &'static [(&'static str, &'static [u8])] = Box::leak(Box::new([
        ("oag-mod.toml", manifest.as_bytes()),
        ("campaigns/night-run.toml", campaign.as_bytes()),
        ("ui/medals.png", medals.as_slice()),
        ("campaigns/broken.toml", b"[campaign]\nid = 3\n"),
        ("campaigns/notes.txt", b"not a campaign"),
        ("campaigns/deeper/skip.toml", b""),
    ]));
    let module = crate::Mod::from_container(crate::Container::memory("test", files)).unwrap();
    let found = module.campaigns();
    let names: Vec<&str> = found.iter().map(|f| f.file.as_str()).collect();
    assert_eq!(names, ["campaigns/broken.toml", "campaigns/night-run.toml"]);
    assert!(found[0].def.is_err());
    assert!(found[1].def.is_ok());
    assert!(module.is_for("wipeout-pulse"));
    assert!(!module.is_for("wipeout-hd"));
}

/// `schemas/v1/campaign.tosd` and [`CampaignDef::parse`] give the same verdict on
/// a file's shape. What only the loader checks (unique ids, slots, references)
/// is in the schema's own header comment and not exercised here.
#[test]
fn the_schema_and_the_reader_agree_on_every_example() {
    let mut cases: Vec<(&str, String, bool)> = vec![
        ("small", with_header(SMALL), true),
        ("no header", SMALL.to_string(), false),
        (
            "unknown key",
            with_header(&SMALL.replace("laps", "lapz"))
                .replace("mode = \"race\"", "mode = \"race\"\nspeed = 3"),
            false,
        ),
        (
            "unknown reveal",
            with_header(&SMALL.replace("name = \"Small\"", "name = \"Small\"\nreveal = \"faded\"")),
            false,
        ),
        (
            "a ramp from a base",
            with_header(&SMALL.replace(
                "name = \"Small\"",
                "name = \"Small\"\ndifficulty = \"hard\"\nprogression = \"ramp\"",
            )),
            true,
        ),
        (
            "our driver's base",
            with_header(&SMALL.replace(
                "name = \"Small\"",
                "name = \"Small\"\ndifficulty = \"skilled\"",
            )),
            true,
        ),
        (
            "an unknown progression",
            with_header(&SMALL.replace(
                "name = \"Small\"",
                "name = \"Small\"\nprogression = \"spiral\"",
            )),
            false,
        ),
        (
            "three slot numbers",
            with_header(&SMALL.replace("[1, 2]", "[1, 2, 3]")),
            false,
        ),
        (
            "a string slot",
            with_header(&SMALL.replace("slot = [2, 2]", "slot = \"2-2\"")),
            false,
        ),
        (
            "unknown medal key",
            with_header(&SMALL.replace("gold = 90.0", "platinum = 90.0")),
            false,
        ),
        (
            "integer targets",
            with_header(&SMALL.replace("90.0", "90")),
            true,
        ),
        (
            "a named pilot",
            with_header(&format!(
                "{SMALL}field = {{ ai = \"ours\", pilot = \"shy\" }}\n"
            )),
            true,
        ),
        (
            "an inline pilot",
            with_header(&format!(
                "{SMALL}field = {{ ai = \"ours\", pilot = {{ ram = [0.1, 0.2] }} }}\n"
            )),
            true,
        ),
        (
            "no grid",
            format!("{HEADER}[campaign]\nid = \"x\"\nname = \"X\"\n"),
            false,
        ),
        (
            "another major",
            with_header(SMALL).replace("/v1/", "/v2/"),
            false,
        ),
    ];
    if let Some(example) = example_file("example-campaign/campaigns/night-run.toml") {
        cases.push(("the example", example.to_string(), true));
    }
    // An inline pilot's keys are the pilot reader's (`oag_raceplay::pilots`),
    // which this crate cannot reach: the schema refuses a typo, the reader
    // keeps the table, and the game refuses it when the campaign compiles.
    let typo = with_header(&format!(
        "{SMALL}field = {{ ai = \"ours\", pilot = {{ rma = [0.1, 0.2] }} }}\n"
    ));
    assert!(oag_schema::validate::accepts(SCHEMA_KIND, SCHEMA_MAJOR, &typo).is_err());
    assert!(CampaignDef::parse(&typo).is_ok());
    // A difficulty word is a name the game checks when the campaign compiles.
    let brutal = with_header(&SMALL.replace(
        "name = \"Small\"",
        "name = \"Small\"\ndifficulty = \"brutal\"",
    ));
    assert!(oag_schema::validate::accepts(SCHEMA_KIND, SCHEMA_MAJOR, &brutal).is_err());
    assert!(CampaignDef::parse(&brutal).is_ok());
    for (what, text, accepted) in cases {
        let schema = oag_schema::validate::accepts(SCHEMA_KIND, SCHEMA_MAJOR, &text);
        let reader = CampaignDef::parse(&text);
        assert_eq!(schema.is_ok(), accepted, "schema, {what}: {schema:?}");
        assert_eq!(reader.is_ok(), accepted, "reader, {what}: {reader:?}");
    }
}

fn built() -> CampaignDef {
    let mut def = CampaignDef::parse(&with_header(SMALL)).unwrap();
    def.grids[0].events[0].craft = Some(CraftDef {
        team: "Feisar".to_string(),
    });
    def.grids[0].events[0].field = Some(FieldDef {
        ai: Some("original".to_string()),
        difficulty: Some("hard".to_string()),
        skill: Some(1.75),
        pilot: None,
        teams: vec!["Qirex".to_string(), "Piranha".to_string()],
    });
    def.grids[0].events[1].weapons = Some(false);
    def.grids[0].events[1].laps = Some(4);
    def
}

#[test]
fn a_written_campaign_reads_back_as_the_same_definition() {
    for def in [CampaignDef::parse(&with_header(SMALL)).unwrap(), built()] {
        let text = def.write().unwrap();
        assert_eq!(CampaignDef::parse(&text).unwrap(), def, "{text}");
    }
}

#[test]
fn a_difficulty_setting_survives_a_write_and_validates_against_the_schema() {
    let mut def = built();
    def.campaign.difficulty = Some("medium".to_string());
    def.campaign.progression = Progression::Ramp;
    let text = def.write().unwrap();
    assert!(text.contains("difficulty = \"medium\""), "{text}");
    assert!(text.contains("progression = \"ramp\""), "{text}");
    assert_eq!(CampaignDef::parse(&text).unwrap(), def);
    oag_schema::validate::accepts(SCHEMA_KIND, SCHEMA_MAJOR, &text)
        .unwrap_or_else(|e| panic!("{e:?}\n{text}"));
    let flat = built().write().unwrap();
    assert!(
        !flat.contains("progression") && !flat.contains("[campaign]\ndifficulty"),
        "{flat}"
    );
}

#[test]
fn a_definition_built_in_code_writes_with_the_header() {
    let info = Info {
        id: "mine".to_string(),
        name: "Mine".to_string(),
        description: String::new(),
        titles: Vec::new(),
        reveal: Reveal::Locked,
        difficulty: None,
        progression: Progression::Flat,
    };
    let event = EventDef {
        slot: [3, 2],
        id: None,
        name: String::new(),
        mode: "race".to_string(),
        track: Some("01_Track".to_string()),
        tracks: Vec::new(),
        file: None,
        class: Some("venom".to_string()),
        laps: None,
        weapons: None,
        craft: None,
        field: None,
        medals: None,
        unlock: None,
    };
    let grid = GridDef {
        id: "grid0".to_string(),
        name: String::new(),
        required_points: 0,
        unlock_grid: None,
        reveal: None,
        flyer: None,
        events: vec![event],
    };
    let def = CampaignDef::new(info, vec![grid]);
    let text = def.write().unwrap();
    assert!(text.starts_with("[toml-schema]"), "{text}");
    assert_eq!(CampaignDef::parse(&text).unwrap(), def);
}

#[test]
fn every_written_campaign_validates_against_the_schema() {
    let mut texts = vec![built().write().unwrap()];
    if let Some(base) = example_dir("example-campaign/campaigns") {
        for entry in std::fs::read_dir(base).unwrap().flatten() {
            let def = CampaignDef::parse(&std::fs::read_to_string(entry.path()).unwrap()).unwrap();
            texts.push(def.write().unwrap());
        }
    }
    for text in texts {
        oag_schema::validate::accepts(SCHEMA_KIND, SCHEMA_MAJOR, &text)
            .unwrap_or_else(|e| panic!("{e:?}\n{text}"));
    }
}

#[test]
fn a_definition_that_would_not_read_back_is_not_written() {
    let mut def = built();
    def.grids[0].events[1].slot = def.grids[0].events[0].slot;
    assert!(def.write().is_err());
}

#[test]
fn every_committed_example_campaign_survives_a_write() {
    let Some(base) = example_dir("example-campaign/campaigns") else {
        return;
    };
    let mut seen = 0;
    for entry in std::fs::read_dir(base).unwrap().flatten() {
        let text = std::fs::read_to_string(entry.path()).unwrap();
        let def = CampaignDef::parse(&text).unwrap();
        let again = def.write().unwrap();
        assert_eq!(
            CampaignDef::parse(&again).unwrap(),
            def,
            "{}",
            entry.path().display()
        );
        seen += 1;
    }
    assert!(seen > 0);
}

fn example_dir(path: &str) -> Option<std::path::PathBuf> {
    let full = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/mods/examples")
        .join(path);
    if full.is_dir() {
        Some(full)
    } else {
        eprintln!("skipped: {} is not checked out", full.display());
        None
    }
}

#[test]
fn a_written_mod_is_found_opened_and_lists_its_campaign() {
    let root = std::env::temp_dir().join(format!("oag-campaign-write-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let def = built();
    let first = write_mod(&root, &def, &["wipeout-pulse"]).unwrap();
    let second = write_mod(&root, &def, &["wipeout-pulse"]).unwrap();
    assert_ne!(first, second, "a saved campaign never overwrites another");
    let found = crate::search(std::slice::from_ref(&root));
    assert_eq!(found.len(), 2);
    let opened = crate::Mod::open(&first).unwrap();
    assert_eq!(opened.manifest.id, "player.small");
    assert!(opened.is_for("wipeout-pulse") && !opened.is_for("wipeout-hd"));
    let campaigns = opened.campaigns();
    assert_eq!(campaigns.len(), 1);
    assert_eq!(campaigns[0].def.as_ref().unwrap(), &def);
    let again = crate::Mod::open(&second).unwrap();
    assert_eq!(again.manifest.id, "player.small-2");
    oag_schema::validate::accepts(
        crate::SCHEMA_KIND,
        crate::SCHEMA_MAJOR,
        &std::fs::read_to_string(first.join(crate::MANIFEST)).unwrap(),
    )
    .unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

const WITH_PLATINUM: &str = r##"
[[medal-tier]]
name = "platinum"
points = 4
colour = "#e5e4e2"
string_id = "NR_PLATINUM"

[[medal-tier]]
name = "gold"
points = 3
colour = "#faeb38"

[[medal-tier]]
name = "silver"
points = 2
colour = "#dae3e4"

[[medal-tier]]
name = "bronze"
points = 1
colour = "#df942f"
"##;

fn platinum_campaign() -> String {
    let body = SMALL.replace(
        "medals = { gold = 90.0, hard = { gold = 80.0 } }",
        "medals = { gold = 90.0, extra = [{ tier = \"platinum\", target = 80.0 }], hard = { gold = 80.0, extra = [{ tier = \"platinum\", target = 70.0 }] } }",
    );
    with_header(&format!("{WITH_PLATINUM}{body}"))
}

#[test]
fn a_campaign_declares_its_medal_tiers_best_first() {
    let text = platinum_campaign();
    let def = CampaignDef::parse(&text).expect("parses");
    let names: Vec<_> = def.medal_tiers.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["platinum", "gold", "silver", "bronze"]);
    assert_eq!(def.medal_tiers[0].argb(), Some(0xffe5_e4e2));
    assert_eq!(def.medal_tiers[0].string_id.as_deref(), Some("NR_PLATINUM"));
    oag_schema::validate::accepts(SCHEMA_KIND, SCHEMA_MAJOR, &text)
        .unwrap_or_else(|e| panic!("{e:?}\n{text}"));
    let written = def.write().unwrap();
    assert_eq!(CampaignDef::parse(&written).unwrap(), def);
}

#[test]
fn a_requirement_may_name_a_declared_tier_and_no_other() {
    let text = platinum_campaign().replace("medal = \"gold\"", "medal = \"platinum\"");
    assert!(CampaignDef::parse(&text).is_ok());
    let text = platinum_campaign().replace("medal = \"gold\"", "medal = \"diamond\"");
    let error = CampaignDef::parse(&text).unwrap_err().to_string();
    assert!(
        error.contains("diamond") && error.contains("platinum"),
        "{error}"
    );
}

#[test]
fn a_target_for_an_undeclared_tier_is_refused_by_name() {
    let text = platinum_campaign().replace(
        "tier = \"platinum\", target = 80.0",
        "tier = \"diamond\", target = 80.0",
    );
    let error = CampaignDef::parse(&text).unwrap_err().to_string();
    assert!(error.contains("diamond"), "{error}");
    let plain = with_header(&SMALL.replace(
        "gold = 90.0",
        "gold = 90.0, extra = [{ tier = \"platinum\", target = 80.0 }]",
    ));
    assert!(
        CampaignDef::parse(&plain)
            .unwrap_err()
            .to_string()
            .contains("platinum")
    );
}

#[test]
fn a_bad_tier_list_is_refused() {
    let twice = platinum_campaign().replace("name = \"silver\"", "name = \"gold\"");
    assert!(
        CampaignDef::parse(&twice)
            .unwrap_err()
            .to_string()
            .contains("twice")
    );
    let colour = platinum_campaign().replace("#e5e4e2", "silver");
    assert!(
        CampaignDef::parse(&colour)
            .unwrap_err()
            .to_string()
            .contains("#RRGGBB")
    );
}

#[test]
fn a_file_without_medal_tiers_reads_as_before() {
    let def = CampaignDef::parse(&with_header(SMALL)).expect("parses");
    assert!(def.medal_tiers.is_empty());
    assert!(!def.write().unwrap().contains("medal-tier"));
}
