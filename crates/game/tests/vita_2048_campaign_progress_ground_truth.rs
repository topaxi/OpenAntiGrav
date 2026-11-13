//! Validates Wipeout 2048's own career - the unlock graph, the objective
//! law, and the map reading a save - against its real `SP.xml`.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship - `data/extracted/vita/PCSF00007/base`, the
//! decrypted EU package. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! Until 2026-09-21, 2048's own campaign persisted nothing: every event on
//! `newFEshell` was offered regardless of the unlock graph `SP.xml` authors,
//! and a finished event's result went nowhere. What replaced that is
//! `oag_2048::campaign::{unlock_gates, event_objectives, evaluate_tier}`
//! (the measured objective law, `docs/formats/2048-campaign.md`'s "The
//! objective law" section) and `oag_ui::frontend::Frontend::
//! refresh_campaign_progress` (folding a save into the map). Everything here
//! checks those against the disc's own `SP.xml`, and the save half against
//! an in-memory `oag_game::records::Store` built with [`records::parse`] -
//! **never [`records::load`]**, which would read (and on a save, write) this
//! machine's own `~/.config/oag/records.toml`. A test that touched a real
//! player's save would be non-deterministic here and destructive on a real
//! machine; see `oag_game::records::Store::record_campaign`'s own doc for
//! why a `(title, event name)` row is the right key for this.

use std::path::{Path, PathBuf};

use oag_2048::campaign::{EventOutcome, Tier, objective_type};
use oag_game::{boot, records};
use oag_gameplay::input::{Button, Input};
use oag_raceplay as race;
use oag_ui::frontend::{self, EarnedTier, ProgressState};

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

fn options(source: &Path) -> boot::Options {
    boot::Options {
        language: Some("English".to_string()),
        source: source.display().to_string(),
        dlc: Vec::new(),
        leg: frontend::Leg::LogoFmv,
        movie: None,
        cache: std::env::temp_dir().join("oag-2048-campaign-progress-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
        find_texture_packs: None,
    }
}

/// `SP.xml`, parsed straight through `oag_2048::open` the same way
/// `race::load_event` reads it - no boot, no front end, just the document.
fn sp_xml(source: &Path) -> oag_2048::campaign::Document {
    let mut archives = oag_2048::open(&source.display().to_string()).expect("this is 2048");
    let bytes = archives
        .read_name(oag_2048::campaign::SP_XML)
        .expect("SP.xml is in the base package");
    let text = String::from_utf8(bytes).expect("SP.xml is UTF-8");
    oag_2048::campaign::parse(&text)
}

/// A fresh save reads every event with no gate as open, and everything else
/// as locked - the same 16-name census `oag_2048::campaign::unlock_gates`'s
/// own doc comment cites, checked here directly against the file.
///
/// **Filtered to the map's own 115 cell-bearing, non-`E3_*` events**, the
/// same filter `boot::campaign2048::map_events` applies - `unlock_gates`
/// itself reads every instance in the document, `MP_*`/`E3_*` templates
/// included, and every one of those reads as gate-free too (nothing ever
/// names one as a prerequisite), which would inflate this count to 40 if
/// counted unfiltered. See that function's own doc comment.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn a_fresh_save_opens_exactly_the_events_with_no_gate() {
    let Some(source) = source() else { return };
    let doc = sp_xml(&source);
    let gates = oag_2048::campaign::unlock_gates(&doc);
    let events = oag_2048::campaign::events(&doc);
    let has_cell = |name: &str| {
        events.iter().any(|event| {
            event.name == name && !name.starts_with("E3_") && event.x.is_some() && event.y.is_some()
        })
    };

    let open: Vec<&str> = gates
        .iter()
        .filter(|(name, gate)| gate.is_none() && has_cell(name))
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(
        open.len(),
        16,
        "measured on the EU v1.04 package: 2048 - Event 1 plus 15 Ship/Phantom \
         Challenge side events author no incoming edge and no M_PEVENTREQUIRED; got {open:#?}"
    );
    assert!(open.contains(&"2048 - Event 1"));
    // `Frontend::set_campaign` opens the map on the first ungated event that
    // another event requires, in the order `map_events` lists them.
    let first_open = events.iter().map(|event| event.name.as_str()).find(|name| {
        open.contains(name) && gates.iter().any(|(_, gate)| gate.as_deref() == Some(*name))
    });
    assert_eq!(first_open, Some("2048 - Event 1"));
    assert!(
        !open.contains(&"2048 - Event 2"),
        "Event 2 is reached by Event 1's own M_PNEXTEVENT"
    );

    let gate_of = |name: &str| {
        gates
            .iter()
            .find(|(event, _)| event == name)
            .and_then(|(_, gate)| gate.clone())
    };
    assert_eq!(
        gate_of("2048 - Event 2"),
        Some("2048 - Event 1".to_string())
    );
}

/// The map itself agrees with the graph: `refresh_campaign_progress` with an
/// empty store locks every gated event and refuses to launch one, and
/// recording a pass on its own prerequisite opens it.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn passing_2048_event_1_opens_2048_event_2_on_the_map() {
    let Some(source) = source() else { return };
    let loaded = boot::load(&options(&source)).expect("the whole boot");
    let mut fe = loaded.frontend;

    // Reach the campaign map the same way a player's own pad does - see
    // `vita_2048_boot_ground_truth.rs::drive_until` for the identical walk.
    let mut input = Input::new();
    for tick in 0..3600u32 {
        input.begin_frame(if tick.is_multiple_of(2) {
            Button::Cross.bit()
        } else {
            0
        });
        fe.update(1.0 / 60.0, &mut input, None);
        if fe.machine().is(oag_2048::frontend::states::NEW_FE_SHELL) {
            break;
        }
    }
    assert!(fe.machine().is(oag_2048::frontend::states::NEW_FE_SHELL));

    // An empty save: every gated event reads Locked, and Event 1 itself
    // (no gate at all) reads Open.
    let (store, notes) = records::parse("").expect("an empty file is valid TOML");
    assert!(notes.is_empty());
    fe.refresh_campaign_progress(|name| {
        store
            .campaign_medal("wipeout 2048", name)
            .and_then(|row| row.best_medal)
            .map(|medal| match medal {
                records::Medal::Gold => EarnedTier::Elite,
                records::Medal::Silver | records::Medal::Bronze => EarnedTier::Pass,
            })
    });
    assert_eq!(
        fe.campaign_event_state("2048 - Event 1"),
        Some(ProgressState::Open)
    );
    assert_eq!(
        fe.campaign_event_state("2048 - Event 2"),
        Some(ProgressState::Locked)
    );

    // A save with Event 1 passed (bronze): Event 2's own gate opens.
    let mut store = records::Store::default();
    store.record_campaign(
        "wipeout 2048",
        "2048 - Event 1",
        Some(records::Medal::Bronze),
        None,
    );
    fe.refresh_campaign_progress(|name| {
        store
            .campaign_medal("wipeout 2048", name)
            .and_then(|row| row.best_medal)
            .map(|medal| match medal {
                records::Medal::Gold => EarnedTier::Elite,
                records::Medal::Silver | records::Medal::Bronze => EarnedTier::Pass,
            })
    });
    assert_eq!(
        fe.campaign_event_state("2048 - Event 1"),
        Some(ProgressState::Passed)
    );
    assert_eq!(
        fe.campaign_event_state("2048 - Event 2"),
        Some(ProgressState::Open),
        "Event 1's own bronze result should satisfy Event 2's gate"
    );
}

/// `race::load_event` resolves the same event `oag_2048::campaign` measured:
/// `"2048 - Event 1"`'s own pass is `FinishRaceAnyPosition` (boolean, no
/// target) and its elite is `Win` (position, target 1) -
/// `docs/formats/2048-campaign.md`'s own census of the real file.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn load_event_resolves_event_1s_own_pass_and_elite_objectives() {
    let Some(source) = source() else { return };
    let options = race::Options {
        source: source.display().to_string(),
        ..race::Options::default()
    };
    let resolved = race::load_event(&options, "2048 - Event 1").expect("the event loads");
    let progress = resolved
        .campaign_2048_event
        .expect("load_event always fills this");
    assert_eq!(progress.name, "2048 - Event 1");
    let objectives = progress
        .objectives
        .expect("Event 1 authors both M_PASSOBJECTIVE and M_ELITEOBJECTIVE");
    assert_eq!(objectives.pass.objective_type, Some(objective_type::FINISH));
    assert_eq!(objectives.pass.target, None);
    assert_eq!(
        objectives.elite.objective_type,
        Some(objective_type::POSITION)
    );
    assert_eq!(objectives.elite.target, Some(1));

    // And the law reads exactly as `oag_2048::campaign`'s own doc comment
    // says it should: any finish clears the pass bar, only first place
    // clears the elite one.
    assert_eq!(
        oag_2048::campaign::evaluate_tier(
            &objectives,
            &EventOutcome {
                finished: true,
                place: 6,
                ..EventOutcome::default()
            }
        ),
        Some(Tier::Pass)
    );
    assert_eq!(
        oag_2048::campaign::evaluate_tier(
            &objectives,
            &EventOutcome {
                finished: true,
                place: 1,
                ..EventOutcome::default()
            }
        ),
        Some(Tier::Elite)
    );
    assert_eq!(
        oag_2048::campaign::evaluate_tier(&objectives, &EventOutcome::default()),
        None,
        "never finished: not even the pass bar is decidable"
    );
}

/// `race::load_event` wires a decoded weapon set through to
/// `Loaded::setup::allowed_weapons`, gating `race::pads` at runtime - see
/// `crates/raceplay/src/load/campaign.rs` and
/// `docs/formats/2048-campaign.md`'s "The weapon set gate" section.
///
/// Checked against a direct decode off the same document rather than a
/// hard-coded weapon list, so this does not go stale the moment a future
/// patch or region reshapes which `WeaponSetDefinition` `"2048 - Event 6"`
/// points at - only that `load_event` carries through whatever that
/// resolves to today, and that today it resolves to something.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn load_event_wires_2048_event_6s_own_weapon_set_onto_setup() {
    let Some(source) = source() else { return };
    let doc = sp_xml(&source);
    let event = oag_2048::campaign::events(&doc)
        .into_iter()
        .find(|event| event.name == "2048 - Event 6")
        .expect("\"2048 - Event 6\" is not in SP.xml");
    let expected = event
        .weapon_set
        .and_then(|reference| oag_2048::campaign::weapon_set_for(&doc, reference))
        .map(|set| set.allowed_weapons())
        .unwrap_or_default();
    assert!(
        !expected.is_empty(),
        "\"2048 - Event 6\"'s own weapon set decoded to nothing recognised - pick a \
         different pinned event so this test still exercises the gate"
    );

    let options = race::Options {
        source: source.display().to_string(),
        ..race::Options::default()
    };
    let resolved = race::load_event(&options, "2048 - Event 6").expect("\"2048 - Event 6\" loads");
    assert_eq!(resolved.setup.allowed_weapons, expected);
}

/// One finished 2048 event raises the stored XP by what `XPAwards.xml` authors for it: the
/// race's in-race tally (a kill, a perfect zone) plus the completion award of its tier plus
/// `Finished Nth`, in the event's mode column, each counted once. Expected values come from the
/// parsed disc table, through the same `Store::bank_event_xp` the session's finish site calls,
/// and the tally the race itself arms (`Race::arm_xp`).
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn a_finished_event_raises_the_stored_xp_by_the_authored_award() {
    let Some(source) = source() else { return };
    let options = race::Options {
        source: source.display().to_string(),
        ..race::Options::default()
    };
    let progress = race::load_event(&options, "2048 - Event 1")
        .expect("the event loads")
        .campaign_2048_event
        .expect("load_event always fills this");
    let mut archives = oag_2048::open(&source.display().to_string()).expect("this is 2048");
    let (awards, ranks) = oag_2048::campaign::xp_tables(&mut archives).expect("both XP tables");
    let column = oag_tables::xp::Column::Race;
    assert_eq!(progress.xp_column, column);
    assert_eq!(progress.xp_awards.as_ref(), Some(&awards));
    let tally = || oag_raceplay::xp_tally::XpTally::new(awards.clone(), column);

    let (mut store, _) = records::parse("").expect("an empty file parses");
    assert_eq!(store.xp(), 0);

    // Third place clears the pass bar only, after one kill in the race.
    let mut gathered = tally();
    gathered.observe_kills(1);
    let kill = awards.xp(6, column);
    assert!(kill > 0, "the table authors Opponent destroyed");
    let paid = store.bank_event_xp(&mut gathered, None, Some(records::Medal::Bronze), Some(3));
    let pass = awards.xp(oag_tables::xp::award::EVENT_PASS, column);
    let third = awards.xp(oag_tables::xp::award::FINISHED_1ST + 2, column);
    assert!(pass > 0 && third > 0, "the table authors both rows");
    assert_eq!(paid, kill + pass + third);
    assert_eq!(store.xp(), kill + pass + third);
    assert_eq!(
        gathered.total(),
        0,
        "banking empties the tally, so a second call adds nothing"
    );

    // First place is elite over the pass before it: Elite alone, and the stored total accumulates.
    let before = store.xp();
    let first = awards.xp(oag_tables::xp::award::FINISHED_1ST, column);
    let elite = awards.xp(oag_tables::xp::award::EVENT_ELITE, column);
    let paid = store.bank_event_xp(
        &mut tally(),
        Some(records::Medal::Bronze),
        Some(records::Medal::Gold),
        Some(1),
    );
    assert_eq!(paid, elite + first);
    assert_eq!(store.xp(), before + elite + first);

    // An elite as a first pass pays both completion awards.
    let paid = store.bank_event_xp(&mut tally(), None, Some(records::Medal::Gold), Some(1));
    assert_eq!(paid, pass + elite + first);

    // Repeating a result that does not beat the best pays the finishing place only.
    for (prior, now) in [
        (records::Medal::Bronze, records::Medal::Bronze),
        (records::Medal::Gold, records::Medal::Gold),
        (records::Medal::Gold, records::Medal::Bronze),
    ] {
        let paid = store.bank_event_xp(&mut tally(), Some(prior), Some(now), Some(1));
        assert_eq!(paid, first, "{prior:?} then {now:?}");
    }
    assert_eq!(store.xp_rank(&ranks), ranks.rank(store.xp()));
}

/// A whole event raced under the autopilot pays the lap and place awards the disc's table
/// authors, through the edges the race reports: a lap without damage at full health, and the
/// places taken on the way. Fails if the lap or place feeds are dropped from `Race::tick`.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn an_autopiloted_event_pays_the_lap_and_place_awards_the_table_authors() {
    let Some(source) = source() else { return };
    let options = race::Options {
        source: source.display().to_string(),
        ..race::Options::default()
    };
    let loaded = race::load_event(&options, "2048 - Event 1").expect("the event loads");
    let progress = loaded.campaign_2048_event.clone().expect("an event");
    let awards = progress.xp_awards.clone().expect("the table loads");
    let mut run = race::Race::start(loaded.setup);
    run.arm_event_xp(&progress);
    run.set_autopilot(true);
    while !run.finished() && run.sim.world.tick < 30_000 {
        run.tick(&oag_gameplay::PlayerInputs::none());
    }
    assert!(run.finished(), "the event did not finish in 30000 ticks");
    let tally = run.xp_tally().expect("armed");
    let column = progress.xp_column;
    let paid = |index: u32| tally.times_paid(index);
    assert!(
        awards.xp(5, column) > 0 && awards.xp(44, column) > 0 && awards.xp(21, column) > 0,
        "the table authors the lap and place awards"
    );
    // The autopilot scrapes a wall in the first lap and never refills: the laps after it are
    // clean, the scraped one is not, and no lap ends at full health.
    let laps = u64::from(run.sim.world.race[0].laps_target.expect("a lap target"));
    assert!(
        (1..laps).contains(&u64::from(paid(5))),
        "Perfect lap paid {} of {laps} laps",
        paid(5)
    );
    assert_eq!(paid(44), 0, "no lap ended at full health");
    assert!(
        paid(21) + paid(22) + paid(23) >= 1,
        "the player took at least one of the first three places"
    );
}

/// The rank gate reaches the map: the 15 hidden side challenges author the rank they wait for,
/// and the profile's XP against this package's own `XPRanks.xml` decides which one is
/// revealable (`boot::reveals2048::refresh`); a find kept in the save reads as found.
#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn the_xp_total_decides_which_hidden_events_are_revealable() {
    use oag_ui::frontend::Reveal;
    let Some(source) = source() else { return };
    let text = source.display().to_string();
    let mut loaded = boot::load(&options(&source)).expect("the whole boot");
    let hidden: Vec<_> = loaded
        .frontend
        .campaign_events()
        .iter()
        .filter(|event| event.rank_gated)
        .map(|event| (event.name.clone(), event.rank_required))
        .collect();
    assert_eq!(hidden.len(), 15);
    assert!(hidden.iter().all(|(_, rank)| rank.is_some()));
    let feisar = "Feisar - P Ship Challenge";
    assert!(hidden.contains(&(feisar.to_string(), Some(10))));

    let mut archives = oag_2048::open(&text).expect("this is 2048");
    let (_, ranks) = oag_2048::campaign::xp_tables(&mut archives).expect("both XP tables");
    let (mut store, _) = records::parse("").expect("an empty file parses");
    let revealable = |frontend: &mut frontend::Frontend, store: &records::Store| {
        boot::reveals2048::refresh(frontend, &text, store, "wipeout 2048");
        hidden
            .iter()
            .filter(|(name, _)| frontend.campaign_reveal(name) == Some(Reveal::Revealable))
            .count()
    };
    assert_eq!(
        revealable(&mut loaded.frontend, &store),
        0,
        "no XP, nothing to find"
    );

    let rank_10 = ranks.threshold_xp(10, 0);
    store.bank_xp(rank_10 - 1);
    assert_eq!(
        revealable(&mut loaded.frontend, &store),
        0,
        "one XP short of rank 10"
    );
    store.bank_xp(1);
    assert_eq!(
        revealable(&mut loaded.frontend, &store),
        hidden.iter().filter(|(_, rank)| *rank <= Some(10)).count(),
        "rank 10 reveals exactly the challenges needing 10 or less"
    );

    assert!(store.record_found("wipeout 2048", feisar));
    boot::reveals2048::refresh(&mut loaded.frontend, &text, &store, "wipeout 2048");
    assert_eq!(loaded.frontend.campaign_reveal(feisar), Some(Reveal::Found));
}
