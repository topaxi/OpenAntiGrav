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
    assert_eq!(medals.hard.unwrap().gold, Some(80.0));
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
    let files: &'static [(&'static str, &'static [u8])] = Box::leak(Box::new([
        ("oag-mod.toml", manifest.as_bytes()),
        ("campaigns/night-run.toml", campaign.as_bytes()),
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
    for (what, text, accepted) in cases {
        let schema = oag_schema::validate::accepts(SCHEMA_KIND, SCHEMA_MAJOR, &text);
        let reader = CampaignDef::parse(&text);
        assert_eq!(schema.is_ok(), accepted, "schema, {what}: {schema:?}");
        assert_eq!(reader.is_ok(), accepted, "reader, {what}: {reader:?}");
    }
}
