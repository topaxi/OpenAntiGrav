use super::*;
use oag_mod::campaign::Reveal;

/// The example mods: the `mod-examples` submodule, absent from a checkout that
/// never fetched it (`git submodule update --init`), where these tests return
/// early and say so. CI checks it out.
fn examples() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/mods/examples")
}

/// Returns from the test when the example mods are not checked out.
macro_rules! needs_examples {
    () => {
        if !examples().join("example-campaign/oag-mod.toml").is_file() {
            eprintln!("skipped: the example mods submodule is not checked out");
            return;
        }
    };
}

fn read(file: &str) -> &'static str {
    let path = examples().join(file);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    Box::leak(text.into_boxed_str())
}

fn example_text() -> &'static str {
    static TEXT: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();
    TEXT.get_or_init(|| read("example-campaign/campaigns/night-run.toml"))
}

fn pilots() -> Pilots {
    Pilots::from([(
        "rival".to_string(),
        oag_raceplay::pilots::parse("rival", read("example-campaign/pilots/rival.toml")).unwrap(),
    )])
}

/// The two example mods as installed: the campaign, and the track-only Hillside
/// it races out of.
fn example_mods() -> [oag_mod::Mod; 2] {
    ["example-campaign", "hillside"].map(|name| oag_mod::Mod::open(&examples().join(name)).unwrap())
}

/// A track file the example names: its own, or `oag.hillside:` Hillside's.
fn files(name: &str) -> Result<TrackFile, String> {
    let [campaign, hillside] = example_mods();
    track_file_in(
        &[&campaign, &hillside],
        &[],
        &campaign,
        "wipeout-pulse",
        name,
    )
}

fn example() -> Custom {
    let def = CampaignDef::parse(example_text()).unwrap();
    compile(
        &def,
        "oag.example-campaign",
        oag_pulse::TITLE,
        &pilots(),
        &files,
    )
    .unwrap()
}

fn edited(from: &str, to: &str) -> Result<Custom, String> {
    let text = example_text().replacen(from, to, 1);
    assert_ne!(text, example_text(), "{from:?} is not in the example");
    let def = CampaignDef::parse(&text).map_err(|e| e.to_string())?;
    compile(&def, "m", oag_pulse::TITLE, &pilots(), &files)
}

#[test]
fn slots_become_the_titles_own_cell_names_and_grids_chain_by_name() {
    needs_examples!();
    let custom = example();
    let names: Vec<&str> = custom.grids[0]
        .cells
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "grid0_2_1",
            "grid0_2_2",
            "grid0_3_1",
            "grid0_3_2",
            "grid0_1_1",
            "grid0_4_1"
        ]
    );
    assert_eq!(custom.grids[1].cells.len(), 5);
    assert_eq!(custom.grids[0].cells[1].grid_coords(), Some((2, 2)));
    assert!(!custom.grids[0].locked);
    assert!(custom.grids[1].locked);
    assert_eq!(custom.grids[1].unlock_grid.as_deref(), Some("grid0"));
    assert_eq!(custom.grids[1].required_points, 6);
    // Tier 1 shows its locks, tier 2 hides what is not open (the campaign's default).
    assert_eq!(custom.reveal(0), Reveal::Locked);
    assert_eq!(custom.reveal(1), Reveal::Hidden);
}

#[test]
fn progress_is_kept_under_a_namespaced_title() {
    needs_examples!();
    assert_eq!(
        example().records_title,
        "wipeout-pulse/mod:oag.example-campaign/night-run"
    );
}

#[test]
fn times_are_seconds_in_the_file_and_centiseconds_in_the_cell() {
    needs_examples!();
    let cell = &example().grids[0].cells[0];
    assert_eq!(
        (cell.gold, cell.silver, cell.bronze),
        (15_000, 16_500, 18_000)
    );
    assert_eq!(cell.evaluate_medal(15_999), Some(Medal::Silver));
    assert_eq!(cell.locked, Some(false));
}

#[test]
fn per_rung_targets_fill_difficulty_targets() {
    needs_examples!();
    let cell = &example().grids[0].cells[1];
    let rungs = cell.difficulty_targets.unwrap();
    assert_eq!(rungs.easy.bronze, 5);
    assert_eq!(rungs.hard.silver, 1);
    use oag_tables::race_campaign::Difficulty;
    assert_eq!(
        cell.evaluate_medal_for_difficulty(4, Difficulty::Easy),
        Some(Medal::Bronze)
    );
    assert_eq!(
        cell.evaluate_medal_for_difficulty(4, Difficulty::Hard),
        None
    );
}

#[test]
fn an_absent_tier_is_never_awarded_on_either_direction() {
    needs_examples!();
    // The duel names gold and bronze but no silver.
    let duel = &example().grids[0].cells[3];
    assert_eq!(duel.silver, 0);
    assert_eq!(duel.evaluate_medal(1), Some(Medal::Gold));
    assert_eq!(duel.evaluate_medal(2), Some(Medal::Bronze));
    // A counting mode with no silver: no kill count reaches it.
    let custom = edited("gold = 10, silver = 6, bronze = 3", "gold = 10, bronze = 3").unwrap();
    let elimination = &custom.grids[1].cells[1];
    assert_eq!(elimination.silver, i64::MAX);
    assert_eq!(elimination.evaluate_medal(7), Some(Medal::Bronze));
    assert_eq!(elimination.evaluate_medal(12), Some(Medal::Gold));
    assert!(elimination.nitro_elimination_targets.is_none());
}

#[test]
fn an_event_with_no_medals_awards_none() {
    needs_examples!();
    let custom = edited("medals = { gold = 10, silver = 6, bronze = 3 }\n", "").unwrap();
    let cell = &custom.grids[1].cells[1];
    assert_eq!(cell.evaluate_medal(99), None);
}

#[test]
fn a_preset_craft_says_no_ship_choice() {
    needs_examples!();
    let custom = example();
    let gate = &custom.grids[0].cells[1];
    assert_eq!(gate.ship.as_deref(), Some("Feisar"));
    assert_eq!(gate.ship_choice, Some(false));
    assert_eq!(
        custom.event("grid0_2_2").unwrap().team.as_deref(),
        Some("Feisar")
    );
    assert_eq!(custom.grids[0].cells[0].ship_choice, Some(true));
}

#[test]
fn the_field_carries_ai_difficulty_pilot_and_teams() {
    needs_examples!();
    let custom = example();
    let gate = custom.event("grid0_2_2").unwrap();
    assert_eq!(gate.ai, Ai::Original);
    assert_eq!(gate.teams, ["Qirex", "Goteki", "Piranha"]);
    assert_eq!(custom.grids[0].cells[1].skill, Some(1.75));
    let clean = custom.event("grid0_4_1").unwrap();
    assert_eq!((clean.level, clean.weapons), (Some(2), Some(false)));
    let duel = custom.event("grid0_3_2").unwrap();
    assert_eq!(duel.ai, Ai::Ours);
    assert_eq!(duel.difficulty, Some(oag_ai::Difficulty::Ace));
    let Some(Pilot::Built(entry)) = &duel.pilot else {
        panic!("the duel flies the mod's own pilot");
    };
    assert_eq!(entry.name, "rival");
    assert_eq!(entry.pilot.commitment.low, 0.8);
    let Some(Pilot::Built(inline)) = &custom.event("grid1_3_1").unwrap().pilot else {
        panic!("the last stand writes its pilot out");
    };
    assert_eq!(inline.pilot.ram.low, 0.6);
    assert_eq!(custom.grids[0].cells[3].ai_count, Some(1));
    assert_eq!(duel.teams, ["Qirex"]);
    let tour = custom.event("grid1_2_1").unwrap();
    assert_eq!(tour.pilot, Some(Pilot::Named("aggressive".to_string())));
}

#[test]
fn requirements_name_cells_and_hold_on_the_least_medal() {
    needs_examples!();
    let custom = example();
    let none = |_: &str| None;
    assert_eq!(custom.fixed_lock("grid0_2_1", &none), Some(false));
    assert_eq!(custom.fixed_lock("grid0_2_2", &none), Some(true));
    let bronze_first = |name: &str| (name == "grid0_2_1").then_some(Medal::Bronze);
    assert_eq!(custom.fixed_lock("grid0_2_2", &bronze_first), Some(false));
    // The duel wants silver on the gate.
    let bronze_gate = |name: &str| (name == "grid0_2_2").then_some(Medal::Bronze);
    assert_eq!(custom.fixed_lock("grid0_3_2", &bronze_gate), Some(true));
    let gold_gate = |name: &str| (name == "grid0_2_2").then_some(Medal::Gold);
    assert_eq!(custom.fixed_lock("grid0_3_2", &gold_gate), Some(false));
    // No unlock table: the title's neighbour rule decides.
    assert_eq!(custom.fixed_lock("grid0_3_1", &none), None);
    // Two requirements: both must hold.
    let sprint_silver = |name: &str| (name == "grid0_1_1").then_some(Medal::Silver);
    assert_eq!(custom.fixed_lock("grid1_4_1", &sprint_silver), Some(true));
    let both = |name: &str| match name {
        "grid0_1_1" => Some(Medal::Silver),
        "grid1_3_2" => Some(Medal::Bronze),
        _ => None,
    };
    assert_eq!(custom.fixed_lock("grid1_4_1", &both), Some(false));
}

#[test]
fn a_tournament_reads_its_legs() {
    needs_examples!();
    let tour = &example().grids[1].cells[0];
    assert_eq!(tour.mode, Mode::Tournament);
    assert!(tour.track.is_none());
    assert_eq!(tour.tournament_tracks, ["01_Track", "02_Track", "03_Track"]);
}

#[test]
fn a_cross_disc_track_names_its_title() {
    needs_examples!();
    let custom = edited("track = \"03_Track\"", "track = \"wipeout-hd:03_Track\"").unwrap();
    assert_eq!(custom.grids[0].cells[3].track.as_deref(), Some("03_Track"));
    assert_eq!(
        custom.event("grid0_3_2").unwrap().track_title.as_deref(),
        Some("wipeout-hd")
    );
}

#[test]
fn what_the_engine_cannot_run_is_refused_by_name() {
    needs_examples!();
    for (from, to, says) in [
        ("mode = \"race\"", "mode = \"drag\"", "mode \"drag\""),
        (
            "class = \"flash\"\nunlock",
            "class = \"warp\"\nunlock",
            "class \"warp\"",
        ),
        (
            "difficulty = \"ace\"",
            "difficulty = \"hard\"",
            "novice, skilled",
        ),
        ("ai = \"original\"", "ai = \"theirs\"", "ai \"theirs\""),
        ("ram = [0.6, 0.9]", "ram = [0.6, 9.0]", "ram"),
        ("ram = [0.6, 0.9]", "rma = [0.6, 0.9]", "rma"),
        (
            "ai = \"ours\"\ndifficulty = \"ace\"",
            "difficulty = \"hard\"",
            "set ai = \"ours\"",
        ),
        (
            "tracks = [\"01_Track\", \"02_Track\", \"03_Track\"]",
            "tracks = [\"01_Track\", \"wipeout-hd:02_Track\"]",
            "one disc",
        ),
    ] {
        let error = edited(from, to).unwrap_err();
        assert!(error.contains(says), "{to:?}: {error}");
        assert!(
            error.starts_with("mod m campaign night-run grid"),
            "{error}"
        );
    }
}

#[test]
fn a_grid_with_nothing_open_opens_its_first_event_without_requirements() {
    needs_examples!();
    let custom = edited(
        "unlock = { open = true }\nmedals = { gold = 1",
        "medals = { gold = 1",
    )
    .unwrap();
    assert!(custom.event("grid1_2_1").unwrap().open);
    assert_eq!(custom.grids[1].cells[0].locked, Some(false));
    // Tier 1 keeps the Hillside sprint open, so its first event, its opening
    // taken away, stays shut.
    let custom = edited("unlock = { open = true }\n# Seconds", "# Seconds").unwrap();
    assert!(!custom.event("grid0_2_1").unwrap().open);
    assert!(custom.event("grid0_1_1").unwrap().open);
}

#[test]
fn a_track_file_is_the_mods_own_or_another_installed_mods() {
    needs_examples!();
    let custom = example();
    // Practice Octagon, bundled in the campaign's own mod.
    let own = custom
        .event("grid1_2_2")
        .unwrap()
        .track_file
        .as_ref()
        .unwrap();
    assert_eq!(own.id, "practice-octagon");
    assert!(
        own.path
            .ends_with("example-campaign/tracks/practice-octagon.track.toml")
    );
    assert_eq!(
        custom.grids[1].cells[2].track.as_deref(),
        Some("Practice Octagon")
    );
    // Hillside, out of the track-only mod `oag.hillside`.
    let other = custom
        .event("grid0_1_1")
        .unwrap()
        .track_file
        .as_ref()
        .unwrap();
    assert_eq!(other.id, "hillside");
    assert!(other.path.ends_with("hillside/tracks/hillside.track.toml") && other.path.is_file());
    assert_eq!(custom.grids[0].cells[4].track.as_deref(), Some("Hillside"));
    let octagon = "file = \"tracks/practice-octagon.track.toml\"";
    for (to, says) in [
        (
            "file = \"tracks/none.track.toml\"",
            "file \"tracks/none.track.toml\"",
        ),
        (
            "file = \"oag.nowhere:tracks/x.track.toml\"",
            "no mod \"oag.nowhere\" is installed",
        ),
        (
            "file = \"oag.hillside:tracks/none.track.toml\"",
            "(in mod \"oag.hillside\")",
        ),
    ] {
        let error = edited(octagon, to).unwrap_err();
        assert!(error.contains(says), "{to}: {error}");
    }
    let error = edited(
        "mode = \"time_trial\"\nfile = \"tracks/practice",
        "mode = \"zone\"\nfile = \"tracks/practice",
    )
    .unwrap_err();
    assert!(error.contains("no Zone circuit"), "{error}");
}

#[test]
fn absent_laps_take_the_discs_own_count_for_the_mode_and_class() {
    needs_examples!();
    let custom = example();
    let laps = |grid: usize, cell: usize| custom.grids[grid].cells[cell].laps;
    assert_eq!(laps(1, 0), Some(4), "a Flash tournament");
    assert_eq!(laps(0, 2), Some(7), "a speed lap");
    assert_eq!(laps(0, 3), Some(4), "a Flash head to head");
    assert_eq!(laps(1, 1), None, "an elimination has none");
    assert_eq!(laps(0, 0), Some(3), "authored");
}

#[test]
fn a_class_takes_the_titles_own_spelling() {
    needs_examples!();
    let custom = example();
    assert_eq!(custom.grids[0].cells[0].class, "Venom");
    assert_eq!(custom.grids[1].cells[4].class, "Phantom");
}

/// The track-only example mod the campaign races Hillside out of.
fn hillside() -> oag_mod::Mod {
    oag_mod::Mod::open(&examples().join("hillside")).unwrap()
}

/// A scratch copy of the example mod, its campaign file replaced by `campaign`
/// and an extra broken one beside it when `broken`.
fn scratch_mod(name: &str, campaign: &str, broken: bool) -> oag_mod::Mod {
    let source = examples().join("example-campaign");
    let dir = std::env::temp_dir().join(format!("oag-custom-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for sub in ["campaigns", "pilots", "tracks"] {
        std::fs::create_dir_all(dir.join(sub)).unwrap();
    }
    for file in [
        "oag-mod.toml",
        "pilots/rival.toml",
        "tracks/practice-octagon.track.toml",
        "tracks/practice-octagon.glb",
    ] {
        std::fs::copy(source.join(file), dir.join(file)).unwrap();
    }
    std::fs::write(dir.join("campaigns/night-run.toml"), campaign).unwrap();
    if broken {
        std::fs::write(dir.join("campaigns/broken.toml"), "[campaign]\n").unwrap();
    }
    oag_mod::Mod::open(&dir).unwrap()
}

#[test]
fn installed_skips_a_title_the_mod_is_not_for_and_names_a_broken_campaign() {
    needs_examples!();
    let module = scratch_mod("installed", example_text(), true);
    let pulse = Installed::for_title([&module, &hillside()], &[], oag_pulse::TITLE, &|_| true);
    assert_eq!(pulse.campaigns.len(), 1, "{:?}", pulse.failed);
    assert!(matches!(
        pulse.campaigns[0].event("grid0_3_2").unwrap().pilot,
        Some(Pilot::Built(_))
    ));
    assert_eq!(pulse.failed.len(), 1);
    assert!(pulse.failed[0].1.starts_with("campaigns/broken.toml"));
    assert!(pulse.get("oag.example-campaign/night-run").is_some());
    let pure = Installed::for_title([&module, &hillside()], &[], oag_pure::TITLE, &|_| true);
    assert!(pure.campaigns.is_empty() && pure.failed.is_empty());
}

#[test]
fn a_track_file_in_a_zipped_mod_is_refused_by_name() {
    needs_examples!();
    let files: &'static [(&'static str, &'static [u8])] = Box::leak(Box::new([
        (
            "oag-mod.toml",
            read("example-campaign/oag-mod.toml").as_bytes(),
        ),
        ("campaigns/night-run.toml", example_text().as_bytes()),
    ]));
    let module = oag_mod::Mod::from_container(oag_mod::Container::memory("test", files)).unwrap();
    let installed = Installed::for_title([&module, &hillside()], &[], oag_pulse::TITLE, &|_| true);
    assert!(installed.campaigns.is_empty());
    assert!(
        installed.failed[0].1.contains("folder mod"),
        "{:?}",
        installed.failed
    );
}

#[test]
fn a_campaign_on_a_disc_that_is_not_here_is_left_out_by_name() {
    needs_examples!();
    let text =
        example_text().replacen("track = \"03_Track\"", "track = \"wipeout-hd:03_Track\"", 1);
    let module = scratch_mod("cross-disc", &text, false);
    let without = Installed::for_title([&module, &hillside()], &[], oag_pulse::TITLE, &|_| false);
    assert!(without.campaigns.is_empty());
    assert!(
        without.failed[0].1.contains("wipeout-hd"),
        "{:?}",
        without.failed
    );
    let with = Installed::for_title([&module, &hillside()], &[], oag_pulse::TITLE, &|slug| {
        slug == "wipeout-hd"
    });
    assert_eq!(with.campaigns.len(), 1);
}

/// A mod cell can share its name with the disc's own (`grid0_2_1` is on Pulse's
/// first grid too): its medal lands under the namespaced title only, and the
/// disc's medal on that name opens nothing in the mod campaign.
#[test]
fn mod_progress_and_disc_progress_never_meet() {
    needs_examples!();
    use crate::records::{Medal as Stored, Store};
    let custom = example();
    let mut records = Store::default();
    records.record_campaign("Wipeout Pulse", "grid0_2_1", Some(Stored::Gold), None);
    {
        let medal_of = crate::unlock::campaign_medal_of(&records, &custom.records_title);
        assert_eq!(medal_of("grid0_2_1"), None);
        assert_eq!(custom.fixed_lock("grid0_2_2", &medal_of), Some(true));
    }

    records.record_campaign(
        &custom.records_title,
        "grid0_2_1",
        Some(Stored::Bronze),
        None,
    );
    let vanilla = records
        .campaign_medal("Wipeout Pulse", "grid0_2_1")
        .unwrap();
    assert_eq!(vanilla.best_medal, Some(Stored::Gold));
    let medal_of = crate::unlock::campaign_medal_of(&records, &custom.records_title);
    assert_eq!(custom.fixed_lock("grid0_2_2", &medal_of), Some(false));
}

#[test]
fn a_campaign_racing_a_mod_turned_off_is_left_out_saying_so() {
    needs_examples!();
    let module = scratch_mod("turned-off", example_text(), false);
    let off = Installed::for_title([&module], &["oag.hillside"], oag_pulse::TITLE, &|_| true);
    assert!(off.campaigns.is_empty());
    let reason = &off.failed[0].1;
    assert!(
        reason.contains("\"oag.hillside\" is turned off"),
        "the reason names the mod and that it is off, not missing: {reason}"
    );
    let gone = Installed::for_title([&module], &[], oag_pulse::TITLE, &|_| true);
    assert!(
        gone.failed[0]
            .1
            .contains("no mod \"oag.hillside\" is installed"),
        "{:?}",
        gone.failed
    );
}
