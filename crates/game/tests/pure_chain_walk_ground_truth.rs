//! Wipeout Pure's campaign walked end to end on the real simulation: SINGLE
//! PLAYER answered screen by screen down to an event, flown to the line by the
//! autopilot, judged by its own cell, recorded in the profile's `[[campaign]]`
//! table, and the chain read again.
//!
//! **`#[ignore]`d and never run in CI.** It needs a disc image. `just test-data`.
//!
//! What is *not* here is the window: the screens, the team picker and the launch
//! are the session's (`crates/game/src/main/session/pure_chain.rs`), and a
//! windowed walk covers them. What is here is every part below the window -
//! the chain's own screens and their answers, the cell it hands the race, the
//! race it asks for, the medal law over the real race clock and the real field,
//! the store, and the gates the new medals open.

use oag_game::campaign::launch::plan_cell_for;
use oag_game::campaign::pure_chain::{Chain, Chosen, choose, present};
use oag_game::campaign::pure_list::{self, PureCampaign};
use oag_game::records::{Medal, Store};
use oag_raceplay as race;
use oag_tables::pure_ai_stats::PureClass;
use oag_tables::pure_campaign::{Event, EventKind};
use oag_ui_screens::pure_chain::Status;

const TITLE: &str = "Wipeout Pure";

/// The ladder `Session::launch_campaign_cell` hands `plan_cell_for`.
const LADDER: &[&str] = match oag_pure::TITLE.race.speed_classes {
    Some(classes) => classes.names,
    None => &[],
};

fn campaign(image: &str) -> Option<(String, PureCampaign)> {
    let path = oag_testdata::image(image)?;
    let source = path.display().to_string();
    let mut archives = oag_pure::open(&source).expect("open as Pure");
    let campaign = pure_list::load(&mut archives, oag_pure::TITLE).expect("the campaign");
    Some((source, campaign))
}

/// The chain as the session opens it, on the disc's own screens.
fn chain_of(source: &str) -> Chain {
    let mut archives = oag_pure::open(source).expect("open as Pure");
    let campaign = pure_list::load(&mut archives, oag_pure::TITLE).expect("the campaign");
    let flow = pure_list::load_flow(&mut archives, oag_pure::TITLE).expect("the flow");
    Chain::new(campaign, flow, PureClass::Vector)
}

/// Answers `Single Player` and each screen after it with `answers` (the mode
/// by name, the rest by row number) and returns what the last one started.
fn answer(chain: &mut Chain, store: &Store, mode: &str, rest: &[usize]) -> Chosen {
    let tag = chain
        .modes()
        .iter()
        .position(|m| m == mode)
        .unwrap_or_else(|| panic!("no mode {mode}"));
    let mut chosen = choose(chain, tag, oag_pure::TITLE, store);
    for &tag in rest {
        assert_eq!(chosen, Chosen::Advanced, "{:?}", chain.at());
        chosen = choose(chain, tag, oag_pure::TITLE, store);
    }
    chosen
}

/// The status of the row for `event` on the screen the chain is on, or `None`
/// when the screen does not list it (a fresh profile's does not list what it has
/// not opened).
fn status_on(chain: &Chain, store: &Store, event: &Event) -> Option<Status> {
    let view = present(
        chain,
        &oag_ui::language::StringTable::default(),
        oag_pure::TITLE,
        store,
    );
    let tag = chain.tag_of(event).expect("the event is on this screen");
    view.rows.iter().find(|r| r.tag == tag).map(|r| r.status)
}

fn event_of(campaign: &PureCampaign, class: PureClass, name: &str) -> Event {
    campaign
        .events(class)
        .into_iter()
        .find(|event| event.name == name)
        .unwrap_or_else(|| panic!("no event {name}"))
}

fn convert(medal: oag_tables::race_campaign::Medal) -> Medal {
    match medal {
        oag_tables::race_campaign::Medal::Gold => Medal::Gold,
        oag_tables::race_campaign::Medal::Silver => Medal::Silver,
        oag_tables::race_campaign::Medal::Bronze => Medal::Bronze,
    }
}

/// The race `event`'s cell asks for, leg `leg`, as the session's launch builds it.
fn load_leg(
    source: &str,
    campaign: &PureCampaign,
    event: &Event,
    leg: usize,
    archives_entry: &dyn Fn(&str) -> Option<String>,
) -> race::Race {
    let plan =
        plan_cell_for(&event.cell, "Venom", LADDER, |_, id| archives_entry(id)).expect("it plans");
    let loaded = race::load(&race::Options {
        source: source.to_string(),
        class: plan.class.clone(),
        mode: plan.mode,
        track: Some(plan.leg_entries[leg].clone()),
        laps_override: plan.laps_override,
        difficulty: oag_ai::Difficulty::Ace,
        ..race::Options::default()
    })
    .expect("the race loads");
    let _ = campaign;
    let mut race = race::Race::start(loaded.setup);
    race.set_autopilot(true);
    race.set_autopilot_tuning(oag_ai::Difficulty::Ace.tune(&oag_ai::Tuning::default()));
    race
}

fn fly_to_the_line(race: &mut race::Race, ticks: u32) -> bool {
    let inputs = oag_gameplay::PlayerInputs::none();
    for _ in 0..ticks {
        race.tick(&inputs);
        if race.finished() {
            return true;
        }
    }
    false
}

fn names(campaign: &PureCampaign) -> impl Fn(&str) -> Option<String> + '_ {
    move |id| campaign.track_entry(id)
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_time_trial_flown_to_the_line_is_judged_recorded_and_shown() {
    let Some((source, campaign)) = campaign("data/images/pure-psp-eu.chd") else {
        return;
    };
    let mut chain = chain_of(&source);
    let mut store = Store::default();
    let Chosen::Start(event) = answer(&mut chain, &store, "Time Trial", &[1, 0, 0]) else {
        panic!("SINGLE PLAYER, Time Trial, Venom, the first league and circuit start an event");
    };
    assert_eq!(event.name, "timetrial/Vineta K/Venom");
    assert_eq!(event.kind, EventKind::TimeTrial);
    let entry = names(&campaign);
    let mut race = load_leg(&source, &campaign, &event, 0, &entry);
    assert!(
        fly_to_the_line(&mut race, 60 * 60 * 6),
        "the autopilot gets round a time trial"
    );
    let tick = race
        .player_standing()
        .finish_tick
        .expect("a finished race has a finish tick");
    let centiseconds =
        oag_game::medal_watch::ticks_to_centiseconds(oag_race::race_clock_ticks(tick));
    let medal = event.cell.evaluate_medal(centiseconds);
    println!(
        "time trial Vineta K Venom: {:.2}s ({centiseconds} cs) against {} / {} / {} -> {medal:?}",
        centiseconds as f32 / 100.0,
        event.cell.gold,
        event.cell.silver,
        event.cell.bronze
    );
    store.record_campaign(TITLE, &event.name, medal.map(convert), None);
    let held = oag_game::unlock::campaign_medal_of(&store, TITLE);
    // The race ended: the chain comes back on the screen that picked it.
    let mut chain = chain_of(&source);
    chain.return_to(&event);
    let status = status_on(&chain, &store, &event);
    match medal {
        Some(medal) => {
            assert_eq!(
                status,
                Some(Status::Medal(medal)),
                "the row shows the medal"
            );
            assert_eq!(held(&event.name), Some(medal));
        }
        None => assert_eq!(
            status,
            Some(Status::Open),
            "no medal, and the event stays open"
        ),
    }
}

#[test]
#[ignore = "needs a disc image in data/images/ and flies four races"]
fn a_tournament_flown_by_the_autopilot_earns_a_rank_and_opens_the_next() {
    let Some((source, campaign)) = campaign("data/images/pure-psp-eu.chd") else {
        return;
    };
    let mut chain = chain_of(&source);
    let mut store = Store::default();
    let Chosen::Start(event) = answer(&mut chain, &store, "Tournament", &[1, 0]) else {
        panic!("SINGLE PLAYER, Tournament, Venom, Alpha start an event");
    };
    assert_eq!(event.name, "tournament/Alpha/Venom");
    assert_eq!(event.cell.tournament_tracks.len(), 4);
    let entry = names(&campaign);
    let plan = plan_cell_for(&event.cell, "Venom", LADDER, |_, id| entry(id)).expect("it plans");
    let mut progress = race::tournament::Progress::new(plan.leg_entries.clone());
    for leg in 0..plan.leg_entries.len() {
        let mut race = load_leg(&source, &campaign, &event, leg, &entry);
        assert!(
            fly_to_the_line(&mut race, 60 * 60 * 12),
            "leg {leg} finishes under the autopilot"
        );
        progress.record_leg(race.results().expect("a finished race has results"));
        println!(
            "leg {}: place {}, {} points so far",
            leg + 1,
            race.player_place(),
            progress.points(0)
        );
        progress.advance();
    }
    let rank = i64::from(progress.rank(0));
    let medal = event.cell.evaluate_medal(rank);
    println!("tournament Alpha Venom: rank {rank} -> {medal:?}");
    store.record_campaign(TITLE, &event.name, medal.map(convert), None);
    let status_of = |class: PureClass, name: &str| {
        let event = event_of(&campaign, class, name);
        let mut chain = chain_of(&source);
        chain.return_to(&event);
        status_on(&chain, &store, &event)
    };
    assert_eq!(
        status_of(PureClass::Venom, "tournament/Beta/Venom").is_some(),
        medal.is_some(),
        "Beta opens on any medal in Alpha at this class, and only then"
    );
    assert_eq!(
        status_of(PureClass::Flash, "tournament/Beta/Flash"),
        None,
        "a Venom medal opens nothing for Flash"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_fresh_profile_walks_every_mode_to_an_event_and_is_offered_two_classes() {
    for image in [
        "data/images/pure-psp-usa.chd",
        "data/images/pure-psp-eu.chd",
    ] {
        let Some((source, _)) = campaign(image) else {
            continue;
        };
        let store = Store::default();
        let view_of = |chain: &Chain| {
            present(
                chain,
                &oag_ui::language::StringTable::default(),
                oag_pure::TITLE,
                &store,
            )
        };
        let mut chain = chain_of(&source);
        assert_eq!(view_of(&chain).rows.len(), 5, "{image}: five modes");
        assert_eq!(
            choose(&mut chain, 0, oag_pure::TITLE, &store),
            Chosen::Advanced
        );
        let classes: Vec<String> = view_of(&chain).rows.into_iter().map(|r| r.label).collect();
        assert_eq!(
            classes,
            ["VECTOR", "VENOM"],
            "{image}: a fresh profile's Class Selection"
        );
        for (mode, rest, kind) in [
            ("Arcade", &[0usize, 0, 0][..], EventKind::Race),
            ("Time Trial", &[0, 0, 0][..], EventKind::TimeTrial),
            ("Tournament", &[0, 0][..], EventKind::Tournament),
            ("Zone", &[0][..], EventKind::Zone),
        ] {
            let mut chain = chain_of(&source);
            let Chosen::Start(event) = answer(&mut chain, &store, mode, rest) else {
                panic!("{image}: {mode} did not start an event");
            };
            assert_eq!(event.kind, kind, "{image}: {mode}");
        }
        let mut chain = chain_of(&source);
        let Chosen::Start(event) = answer(&mut chain, &store, "Free Play", &[0, 0, 0]) else {
            panic!("{image}: Free Play did not start a race");
        };
        assert!(
            oag_tables::pure_campaign::is_free_play(&event.name)
                && event.class == Some(PureClass::Vector),
            "{image}: Free Play starts a race that keeps no medal"
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_event_plans_and_its_circuits_are_on_the_disc() {
    for image in [
        "data/images/pure-psp-usa.chd",
        "data/images/pure-psp-eu.chd",
    ] {
        let Some((source, campaign)) = campaign(image) else {
            continue;
        };
        let mut archives = oag_pure::open(&source).expect("open as Pure");
        let def = archives
            .read_name(oag_pure::campaign::DEFINITION_ENTRY)
            .expect("Definition.xml");
        let def = oag_tables::fexml::text(&def).expect("text");
        let zones = oag_raceplay::catalogue::tracks_of_kind(&def, "Zone");
        let ordinary = oag_raceplay::catalogue::tracks(&def);
        let mut planned = 0;
        for class in PureClass::ALL {
            for event in campaign.events(class) {
                let plan = plan_cell_for(&event.cell, "Venom", LADDER, |mode, id| {
                    let list = if mode == oag_race::Mode::Zone {
                        &zones
                    } else {
                        &ordinary
                    };
                    list.iter()
                        .find(|t| t.id == id)
                        .map(oag_raceplay::catalogue::Track::entry_name)
                        .or_else(|| campaign.track_entry(id))
                })
                .unwrap_or_else(|e| panic!("{image}: {} does not plan: {e}", event.name));
                for entry in &plan.leg_entries {
                    assert!(
                        archives.read_name(entry).is_ok(),
                        "{image}: {} races {entry}, which is not on the disc",
                        event.name
                    );
                }
                planned += 1;
            }
        }
        println!("{image}: {planned} events planned");
    }
}
