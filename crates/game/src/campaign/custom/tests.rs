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
    let custom = edited(
        "medals = { gold = 10, silver = 6, bronze = 3, extra = [{ tier = \"platinum\", target = 14 }] }\n",
        "",
    )
    .unwrap();
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
    let tier = |name: &str| custom.tiers.find(name);
    let none = |_: &str| None;
    assert_eq!(custom.fixed_lock("grid0_2_1", &none), Some(false));
    assert_eq!(custom.fixed_lock("grid0_2_2", &none), Some(true));
    let bronze_first = |name: &str| (name == "grid0_2_1").then_some(tier("bronze").unwrap());
    assert_eq!(custom.fixed_lock("grid0_2_2", &bronze_first), Some(false));
    // The duel wants silver on the gate.
    let bronze_gate = |name: &str| (name == "grid0_2_2").then_some(tier("bronze").unwrap());
    assert_eq!(custom.fixed_lock("grid0_3_2", &bronze_gate), Some(true));
    let gold_gate = |name: &str| (name == "grid0_2_2").then_some(tier("gold").unwrap());
    assert_eq!(custom.fixed_lock("grid0_3_2", &gold_gate), Some(false));
    // No unlock table: the title's neighbour rule decides.
    assert_eq!(custom.fixed_lock("grid0_3_1", &none), None);
    // Two requirements: both must hold.
    let sprint_silver = |name: &str| (name == "grid0_1_1").then_some(tier("silver").unwrap());
    assert_eq!(custom.fixed_lock("grid1_4_1", &sprint_silver), Some(true));
    let both = |name: &str| match name {
        "grid0_1_1" => Some(tier("silver").unwrap()),
        "grid1_3_2" => Some(tier("bronze").unwrap()),
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
    // The Octagon is a Zone event on that file: it compiles, with no laps and
    // the mode's own class word.
    let zone = &custom.grids[1].cells[2];
    assert_eq!(zone.mode, Mode::Zone);
    assert_eq!(zone.class, "Zone");
    assert_eq!(zone.laps, None);
    assert!(custom.event("grid1_2_2").unwrap().track_file.is_some());
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
        let medal_of =
            crate::unlock::campaign_medal_of(&records, &custom.records_title, &custom.tiers);
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
    let medal_of = crate::unlock::campaign_medal_of(&records, &custom.records_title, &custom.tiers);
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

/// `n` race events in one tier, every other one flown by our driver, under the
/// given `[campaign]` lines and with `extra` appended to the third event.
fn ladder_campaign(setting: &str, n: usize, extra: &str) -> Result<Custom, String> {
    let mut text = format!(
        "toml-schema = {{ location = \"https://oag.topaxi.com/mods/schemas/v1/campaign.tosd\", version = \"1.0.0\" }}\n\
         [campaign]\nid = \"ladder\"\nname = \"Ladder\"\n{setting}\n[[grid]]\nid = \"g0\"\n"
    );
    for i in 0..n {
        let ai = if i % 2 == 0 { "original" } else { "ours" };
        let own = if i == 2 { extra } else { "" };
        text.push_str(&format!(
            "[[grid.event]]\nslot = [{}, {}]\nmode = \"race\"\ntrack = \"01_Track\"\nclass = \"venom\"\n\
             unlock = {{ open = true }}\nfield = {{ ai = \"{ai}\"{own} }}\n",
            i % 10,
            i / 10
        ));
    }
    let def = CampaignDef::parse(&text).map_err(|e| e.to_string())?;
    compile(&def, "m", oag_pulse::TITLE, &pilots(), &files)
}

/// The original rung and our level of each event of a `ladder_campaign`, in order.
fn rungs(custom: &Custom) -> Vec<(Option<usize>, Option<oag_ai::Difficulty>)> {
    custom.grids[0]
        .cells
        .iter()
        .map(|cell| {
            let event = custom.event(&cell.name).unwrap();
            (event.level, event.difficulty)
        })
        .collect()
}

#[test]
fn a_flat_campaign_runs_every_cell_at_the_base_on_either_driver() {
    use oag_ai::Difficulty::{Ace, Novice, Skilled};
    let hard = ladder_campaign("difficulty = \"hard\"", 4, "").unwrap();
    assert_eq!(
        rungs(&hard),
        [
            (Some(2), None),
            (None, Some(Ace)),
            (Some(2), None),
            (None, Some(Ace))
        ]
    );
    let easy = ladder_campaign("difficulty = \"easy\"\nprogression = \"flat\"", 2, "").unwrap();
    assert_eq!(rungs(&easy), [(Some(0), None), (None, Some(Novice))]);
    // Named in our driver's words, the base keeps its place on the original's three rungs.
    let skilled = ladder_campaign("difficulty = \"skilled\"", 2, "").unwrap();
    assert_eq!(rungs(&skilled), [(Some(1), None), (None, Some(Skilled))]);
}

#[test]
fn no_setting_leaves_the_screens_rung_alone() {
    let custom = ladder_campaign("", 3, "").unwrap();
    assert_eq!(rungs(&custom), [(None, None); 3]);
}

#[test]
fn a_ramp_never_steps_down_and_reaches_the_top_of_the_ladder() {
    for base in [
        "easy", "medium", "hard", "novice", "skilled", "elite", "ace",
    ] {
        for n in [2, 3, 5, 12] {
            let setting = format!("difficulty = \"{base}\"\nprogression = \"ramp\"");
            let custom = ladder_campaign(&setting, n, "").unwrap();
            let events = rungs(&custom);
            let original: Vec<usize> = events.iter().filter_map(|e| e.0).collect();
            let ours: Vec<u8> = events.iter().filter_map(|e| e.1).map(|d| d as u8).collect();
            assert!(
                original.windows(2).all(|w| w[0] <= w[1]),
                "{base} {n}: {original:?}"
            );
            assert!(
                ours.windows(2).all(|w| w[0] <= w[1]),
                "{base} {n}: {ours:?}"
            );
            // The last cell is at the top of whichever ladder its driver climbs.
            let last = events.last().unwrap();
            assert!(
                *last == (Some(2), None) || *last == (None, Some(oag_ai::Difficulty::Ace)),
                "{base} {n}: the last cell is {last:?}"
            );
        }
    }
}

#[test]
fn a_ramp_with_no_base_starts_at_the_bottom() {
    use oag_ai::Difficulty::{Ace, Skilled};
    let custom = ladder_campaign("progression = \"ramp\"", 4, "").unwrap();
    let events = rungs(&custom);
    assert_eq!(events[0], (Some(0), None));
    assert_eq!(events[1].1, Some(Skilled));
    assert_eq!(events[3].1, Some(Ace));
}

#[test]
fn a_ramp_steps_up_with_the_cell_order_across_tiers() {
    let text = "toml-schema = { location = \"https://oag.topaxi.com/mods/schemas/v1/campaign.tosd\", version = \"1.0.0\" }\n\
        [campaign]\nid = \"t\"\nname = \"T\"\ndifficulty = \"easy\"\nprogression = \"ramp\"\n\
        [[grid]]\nid = \"a\"\n[[grid.event]]\nslot = [0, 0]\nmode = \"race\"\ntrack = \"01_Track\"\nclass = \"venom\"\n\
        [[grid.event]]\nslot = [1, 0]\nmode = \"race\"\ntrack = \"01_Track\"\nclass = \"venom\"\n\
        [[grid]]\nid = \"b\"\n[[grid.event]]\nslot = [0, 0]\nmode = \"race\"\ntrack = \"01_Track\"\nclass = \"venom\"\n";
    let custom = compile(
        &CampaignDef::parse(text).unwrap(),
        "m",
        oag_pulse::TITLE,
        &pilots(),
        &files,
    )
    .unwrap();
    let level = |cell: &str| custom.event(cell).unwrap().level;
    assert_eq!(
        [level("grid0_0_0"), level("grid0_1_0"), level("grid1_0_0")],
        [Some(0), Some(1), Some(2)]
    );
}

#[test]
fn a_cells_own_difficulty_wins_over_the_ramp() {
    // The third event (index 2, flown by the original driver) names `easy`: on a
    // ramp it would be partway up.
    let setting = "difficulty = \"easy\"\nprogression = \"ramp\"";
    let ramped = ladder_campaign(setting, 5, "").unwrap();
    assert_eq!(rungs(&ramped)[2].0, Some(1));
    let own = ladder_campaign(setting, 5, ", difficulty = \"hard\"").unwrap();
    assert_eq!(rungs(&own)[2].0, Some(2));
    assert_eq!(
        rungs(&own)[0].0,
        Some(0),
        "the cells beside it still follow the ramp"
    );
    assert_eq!(rungs(&own)[4].0, ramped.event("grid0_4_0").unwrap().level);
}

#[test]
fn a_difficulty_neither_driver_knows_is_refused_by_name() {
    let err = ladder_campaign("difficulty = \"brutal\"", 2, "").unwrap_err();
    assert!(err.contains("mod m campaign ladder"), "{err}");
    assert!(err.contains("brutal") && err.contains("novice"), "{err}");
}

#[test]
fn the_example_starts_its_opponents_at_medium_and_a_cells_own_rung_wins() {
    needs_examples!();
    let custom = example();
    let cells: Vec<&Cell> = custom.grids.iter().flat_map(|grid| &grid.cells).collect();
    let rung = |cell: &Cell| custom.event(&cell.name).unwrap().level;
    // Warm-up is the first event listed and names no field: the campaign's base, medium.
    assert_eq!(rung(cells[0]), Some(1));
    // Clean Race is the sixth listed, ramped to medium too, and names `hard` itself.
    assert_eq!(custom.event("grid0_4_1").unwrap().level, Some(2));
    // Every other event flown by the original driver follows the ramp: medium or above.
    for cell in cells
        .iter()
        .filter(|cell| custom.event(&cell.name).unwrap().ai == Ai::Original)
    {
        assert!(rung(cell) >= Some(1), "{}: {:?}", cell.name, rung(cell));
    }
}

fn medals_mod() -> oag_mod::Mod {
    let manifest = r#"toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/mod.tosd", version = "1.0.0" }

[mod]
id = "test.hd-medals"
name = "HD Medals"
version = "1.0.0"
authors = ["A"]
description = "x"
titles = ["wipeout-pulse"]
needs-titles = ["wipeout-hd"]

[campaign-medals]
source = { title = "wipeout-hd" }
per-difficulty = true
"#;
    let files: &'static [(&'static str, &'static [u8])] =
        Box::leak(Box::new([("oag-mod.toml", manifest.as_bytes())]));
    oag_mod::Mod::from_container(oag_mod::Container::memory("test", files)).unwrap()
}

#[test]
fn a_medals_mod_is_asked_for_on_its_title_and_names_the_disc_it_lacks() {
    let module = medals_mod();
    let without = Installed::for_title([&module], &[], oag_pulse::TITLE, &|_| false);
    let (id, medals) = without.medals.as_ref().expect("asks for medal art");
    assert_eq!(id, "test.hd-medals");
    assert!(medals.per_difficulty);
    assert_eq!(
        without.needs_discs,
        [("test.hd-medals".to_string(), "Wipeout HD".to_string())]
    );
    let with = Installed::for_title([&module], &[], oag_pulse::TITLE, &|slug| {
        slug == "wipeout-hd"
    });
    assert!(with.medals.is_some() && with.needs_discs.is_empty());
    let pure = Installed::for_title([&module], &[], oag_pure::TITLE, &|_| false);
    assert!(pure.medals.is_none() && pure.needs_discs.is_empty());
}

#[test]
fn the_mods_page_says_which_disc_a_mod_needs() {
    let module = medals_mod();
    let installed = Installed::for_title([&module], &[], oag_pulse::TITLE, &|_| false);
    let mut listing = vec![crate::mods::Listing {
        id: "test.hd-medals".to_string(),
        name: "HD Medals".to_string(),
        version: "1.0.0".to_string(),
        authors: Vec::new(),
        license: None,
        rows: Vec::new(),
        builtin: false,
        enabled: Some(true),
        default_enabled: false,
        texture_pack: false,
        error: None,
        problems: Vec::new(),
        publishes: Vec::new(),
        listens: Vec::new(),
    }];
    crate::mods::note_left_out(
        &mut listing,
        &installed,
        &oag_ui::language::StringTable::default(),
    );
    assert_eq!(
        listing[0].problems,
        [("NEEDS".to_string(), "WIPEOUT HD DISC".to_string())]
    );
}

const PLATINUM_TIERS: &str = r##"
[[medal-tier]]
name = "platinum"
points = 4
colour = "#e5e4e2"
string_id = "PLATINUM_WORD"

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

/// Two timed events on one tier: `a` asks platinum 70 s, gold 80 s, silver 90 s, bronze 100 s,
/// with a harder rung; `b` needs platinum on `a`.
fn platinum_campaign(tiers: &str) -> Result<Custom, String> {
    let text = format!(
        "toml-schema = {{ location = \"https://oag.topaxi.com/mods/schemas/v1/campaign.tosd\", version = \"1.0.0\" }}\n\
         {tiers}\n[campaign]\nid = \"plat\"\nname = \"Plat\"\n\n[[grid]]\nid = \"g0\"\n\n\
         [[grid.event]]\nid = \"a\"\nslot = [1, 1]\nmode = \"time_trial\"\ntrack = \"01_Track\"\nclass = \"venom\"\n\
         unlock = {{ open = true }}\n\
         medals = {{ gold = 80.0, silver = 90.0, bronze = 100.0, extra = [{{ tier = \"platinum\", target = 70.0 }}], \
         hard = {{ gold = 75.0, silver = 85.0, bronze = 95.0, extra = [{{ tier = \"platinum\", target = 65.0 }}] }} }}\n\n\
         [[grid.event]]\nid = \"b\"\nslot = [2, 1]\nmode = \"time_trial\"\ntrack = \"02_Track\"\nclass = \"venom\"\n\
         unlock = {{ requires = [{{ event = \"a\", medal = \"platinum\" }}] }}\n\
         medals = {{ gold = 80.0 }}\n"
    );
    let def = CampaignDef::parse(&text).map_err(|e| e.to_string())?;
    compile(&def, "m", oag_pulse::TITLE, &pilots(), &files)
}

#[test]
fn a_campaign_with_no_tier_list_has_the_titles_own_three() {
    let custom = ladder_campaign("", 3, "").unwrap();
    assert!(custom.tiers.is_standard());
}

#[test]
fn a_declared_tier_list_is_compiled_with_its_points_colours_and_words() {
    let custom = platinum_campaign(PLATINUM_TIERS).unwrap();
    let names: Vec<_> = custom.tiers.iter().map(|(_, t)| t.name.as_str()).collect();
    assert_eq!(names, ["platinum", "gold", "silver", "bronze"]);
    let platinum = custom.tiers.find("platinum").unwrap();
    assert_eq!(custom.tiers.points(platinum), 4);
    assert_eq!(custom.tiers.argb(platinum), 0xffe5_e4e2);
    assert_eq!(
        custom.tiers.get(platinum).unwrap().string_id.as_deref(),
        Some("PLATINUM_WORD")
    );
    let gold = custom
        .tiers
        .get(custom.tiers.find("gold").unwrap())
        .unwrap();
    assert_eq!(
        gold.string_id.as_deref(),
        Some("IG_HUD_GOLD"),
        "the title's word for gold"
    );
}

#[test]
fn a_result_earns_the_best_tier_whose_target_it_meets_on_the_rung_it_was_raced_at() {
    let custom = platinum_campaign(PLATINUM_TIERS).unwrap();
    let targets = &custom.events["grid0_1_1"].tier_targets;
    let tier = |centis, rung| {
        targets
            .evaluate(&Mode::TimeTrial, centis, rung)
            .map(|m| custom.tiers.get(m).unwrap().name.as_str())
    };
    assert_eq!(tier(6900, Difficulty::Medium), Some("platinum"));
    assert_eq!(tier(7500, Difficulty::Medium), Some("gold"));
    assert_eq!(tier(10100, Difficulty::Medium), None);
    assert_eq!(tier(6500, Difficulty::Hard), Some("platinum"));
    assert_eq!(
        tier(6900, Difficulty::Hard),
        Some("gold"),
        "platinum asks 65 s on hard"
    );
    let cell = &custom.grids[0].cells[0];
    assert_eq!(
        (cell.gold, cell.silver, cell.bronze),
        (8000, 9000, 10_000),
        "gold, silver and bronze keep their own targets, whatever the list above them"
    );
}

#[test]
fn a_requirement_names_a_declared_tier_and_a_better_tier_satisfies_a_lesser_one() {
    let custom = platinum_campaign(PLATINUM_TIERS).unwrap();
    let platinum = custom.tiers.find("platinum").unwrap();
    let gold = custom.tiers.find("gold").unwrap();
    let held =
        |tier: Option<Medal>| move |name: &str| (name == "grid0_1_1").then_some(tier).flatten();
    assert_eq!(
        custom.fixed_lock("grid0_2_1", &held(Some(platinum))),
        Some(false)
    );
    assert_eq!(
        custom.fixed_lock("grid0_2_1", &held(Some(gold))),
        Some(true)
    );
    assert_eq!(custom.fixed_lock("grid0_2_1", &held(None)), Some(true));
    let own_gold = platinum_campaign(PLATINUM_TIERS)
        .unwrap()
        .events
        .get("grid0_2_1")
        .unwrap()
        .requires
        .clone();
    assert_eq!(own_gold, [("grid0_1_1".to_string(), Some(platinum))]);
}

#[test]
fn a_tier_the_campaign_does_not_declare_is_refused_by_name() {
    let error = platinum_campaign("").unwrap_err();
    assert!(error.contains("platinum"), "{error}");
}

#[test]
fn a_stored_tier_is_read_back_by_name_against_the_campaigns_list() {
    use crate::records::{Medal as Stored, Store, TierEarned};
    let custom = platinum_campaign(PLATINUM_TIERS).unwrap();
    let order: Vec<&str> = custom.tiers.iter().map(|(_, t)| t.name.as_str()).collect();
    let mut records = Store::default();
    records.record_campaign_tier(
        &custom.records_title,
        "grid0_1_1",
        Some(TierEarned {
            name: "platinum",
            class: Stored::Gold,
            points: 4,
            order: &order,
        }),
        None,
    );
    records.record_campaign(
        &custom.records_title,
        "grid0_2_1",
        Some(Stored::Silver),
        None,
    );
    let medal_of = crate::unlock::campaign_medal_of(&records, &custom.records_title, &custom.tiers);
    assert_eq!(medal_of("grid0_1_1"), custom.tiers.find("platinum"));
    assert_eq!(
        medal_of("grid0_2_1"),
        custom.tiers.find("silver"),
        "a row written as a plain silver is the campaign's silver"
    );
    let standard = oag_tables::race_campaign::MedalTiers::standard();
    let plain = crate::unlock::campaign_medal_of(&records, &custom.records_title, standard);
    assert_eq!(
        plain("grid0_1_1"),
        None,
        "a list without platinum has no such tier"
    );
}

#[test]
fn a_grids_maximum_is_what_its_events_can_award() {
    let custom = platinum_campaign(PLATINUM_TIERS).unwrap();
    // `a` awards platinum (4), `b` only gold (3).
    assert_eq!(custom.max_points(&custom.grids[0]), 7);
    let standard = ladder_campaign("", 3, "").unwrap();
    assert_eq!(
        standard.max_points(&standard.grids[0]),
        0,
        "no medals, no points"
    );
}

#[test]
fn a_launch_keeps_a_tier_by_name_and_names_the_tiers_the_title_has_no_phrase_for() {
    let custom = platinum_campaign(PLATINUM_TIERS).unwrap();
    let launch = Launch {
        records_title: custom.records_title.clone(),
        tiers: custom.tiers.clone(),
        words: None,
        tier_targets: custom.events["grid0_1_1"].tier_targets.clone(),
        ai: Ai::Original,
        level: None,
        pilot: None,
    };
    let mut records = crate::records::Store::default();
    launch.record_result(&mut records, "grid0_1_1", Some("platinum"), None);
    launch.record_result(&mut records, "grid0_1_1", Some("silver"), None);
    let row = records
        .campaign_medal(&custom.records_title, "grid0_1_1")
        .unwrap();
    assert_eq!(
        row.best_tier.as_deref(),
        Some("platinum"),
        "a later silver is no downgrade"
    );
    assert_eq!(
        row.best_medal,
        Some(crate::records::Medal::Gold),
        "the class that stands in"
    );
    assert_eq!(row.best_points, Some(4));
    assert!(launch.own_phrase_tier("platinum").is_some());
    assert!(
        launch.own_phrase_tier("gold").is_none(),
        "the title's own phrase"
    );
}
