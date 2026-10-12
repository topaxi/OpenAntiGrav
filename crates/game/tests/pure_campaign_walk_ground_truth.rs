//! Wipeout Pure's campaign walked end to end on the real simulation: an event
//! from the list, flown to the line by the autopilot, judged by its own cell,
//! recorded in the profile's `[[campaign]]` table, and the list read again.
//!
//! **`#[ignore]`d and never run in CI.** It needs a disc image. `just test-data`.
//!
//! What is *not* here is the window: the screens, the team picker and the launch
//! are the session's (`crates/game/src/main/session/pure_campaign.rs`), and a
//! windowed walk covers them. What is here is every part below the window -
//! the cell the list hands the race, the race it asks for, the medal law over
//! the real race clock and the real field, the store, and the gates the new
//! medals open.

use oag_game::campaign::launch::plan_cell_for;
use oag_game::campaign::pure_list::{self, PureCampaign};
use oag_game::records::{Medal, Store};
use oag_raceplay as race;
use oag_tables::pure_ai_stats::PureClass;
use oag_tables::pure_campaign::{Event, EventKind};
use oag_ui_screens::pure_campaign::Status;

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

fn event(campaign: &PureCampaign, class: PureClass, name: &str) -> Event {
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
    let event = event(&campaign, PureClass::Venom, "timetrial/Vineta K/Venom");
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
    let mut store = Store::default();
    store.record_campaign(TITLE, &event.name, medal.map(convert), None);
    let held = oag_game::unlock::campaign_medal_of(&store, TITLE);
    let (rows, events, _) = pure_list::present(
        &campaign,
        &oag_ui::language::StringTable::default(),
        TITLE,
        PureClass::Venom,
        &store,
    );
    let tag = events.iter().position(|e| e.name == event.name).unwrap();
    let row = rows.iter().find(|r| !r.header && r.tag == tag).unwrap();
    match medal {
        Some(medal) => {
            assert_eq!(row.status, Status::Medal(medal), "the row shows the medal");
            assert_eq!(held(&event.name), Some(medal));
        }
        None => assert_eq!(
            row.status,
            Status::Open,
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
    let event = event(&campaign, PureClass::Venom, "tournament/Alpha/Venom");
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
    let mut store = Store::default();
    store.record_campaign(TITLE, &event.name, medal.map(convert), None);
    let strings = oag_ui::language::StringTable::default();
    let status_of = |class: PureClass, name: &str| {
        let (rows, events, _) = pure_list::present(&campaign, &strings, TITLE, class, &store);
        let tag = events.iter().position(|e| e.name == name).unwrap();
        rows.iter()
            .find(|r| !r.header && r.tag == tag)
            .unwrap()
            .status
    };
    assert_eq!(
        status_of(PureClass::Venom, "tournament/Beta/Venom") != Status::Locked,
        medal.is_some(),
        "Beta opens on any medal in Alpha at this class, and only then"
    );
    assert_eq!(
        status_of(PureClass::Flash, "tournament/Beta/Flash"),
        Status::Locked,
        "a Venom medal opens nothing for Flash"
    );
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
