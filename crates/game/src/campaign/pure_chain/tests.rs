use std::collections::BTreeMap;

use super::*;
use crate::campaign::pure_list::Words;
use oag_tables::pure_campaign;

const DEFINITION: &str = r#"<Screen name="Top">
<PI_Track name="Vineta K"><Values type="Race" location="Data\Environments\01_Vineta_K"></Values></PI_Track>
<PI_Track name="Karbonis"><Values type="Classic" location="Data\ClassicsNS\01_Karbonis"></Values>
<Unlock MedalCount="2"></Unlock></PI_Track>
<PI_Track name="Zone 1"><Values type="Zone" location="Data\Zone\01_Zone"></Values></PI_Track>
<PI_Track name="Zone 2"><Values type="Zone" location="Data\Zone\02_Zone"></Values>
<Unlock Track="Zone 1" medal="bronze"></Unlock></PI_Track>
<PI_League name="Alpha"><Entry track="Vineta K"></Entry></PI_League>
<PI_League name="Classic"><Entry track="Karbonis"></Entry><Unlock MedalCount="1"></Unlock></PI_League>
<PI_Tournament name="Alpha"><Entry track="Vineta K"></Entry></PI_Tournament>
<PI_Tournament name="Beta"><Entry track="Vineta K"></Entry>
<Unlock Tournament="Alpha" class="asSelected" medal="Any"></Unlock></PI_Tournament>
</Screen>"#;

const STATS: &str = r#"<Stats>
<RaceTimes Bronze="6" Flash="124" Phantom="122" Rapier="109" Silver="3" Vector="124" Venom="105"/>
<ZoneMedals Bronze="20" Gold="30" Silver="25"/></Stats>"#;

const SKIN: &str = r#"
<Screen name="Main Menu"><Menu name="Mode"><Entry idstring="Single Player"/></Menu></Screen>
<Screen name="Single Player">
  <Menu name="Mode"><Entry idstring="Arcade"/><Entry idstring="Tournament"/><Entry idstring="Time Trial"/><Entry idstring="Zone"/><Entry idstring="Free Play"/></Menu>
  <Redirect>
    <Entry item="Mode" equals="Arcade" goto="Class Selection"/>
    <Entry item="Mode" equals="Tournament" goto="Class Selection"/>
    <Entry item="Mode" equals="Time Trial" goto="Class Selection"/>
    <Entry item="Mode" equals="Zone" goto="Zone Track Selection"/>
    <Entry item="Mode" equals="Free Play" goto="Class Selection"/>
    <Default goto="To Be Done"/>
  </Redirect>
</Screen>"#;

const SELECTION: &str = r#"
<Screen name="Class Selection">
  <Menu name="Class"><Entry idstring="Vector"/><Entry idstring="Venom"/><Entry idstring="Flash"/><Entry idstring="Rapier"/><Entry idstring="Phantom"/></Menu>
  <Redirect name="Single Player Redirect">
    <Entry item="Single Player->Mode" equals="Tournament" goto="Tournament Selection"/>
    <Default goto="League Selection"/>
  </Redirect>
</Screen>
<Screen name="League Selection"><Redirect><Default goto="Track Selection"/></Redirect></Screen>
<Screen name="Tournament Selection"><Redirect><Default goto="Team Selection"/></Redirect></Screen>
<Screen name="Track Selection"><Redirect><Default goto="Team Selection"/></Redirect></Screen>
<Screen name="Zone Track Selection"><Redirect><Default goto="Launch Game"/></Redirect></Screen>
<Screen name="Team Selection"><Redirect><Default goto="Launch Game"/></Redirect></Screen>"#;

const START: &[&str] = &["Vector", "Venom"];

fn chain() -> Chain {
    let figures = pure_campaign::parse_stats(STATS).unwrap();
    let campaign = PureCampaign {
        definition: pure_campaign::parse(DEFINITION).unwrap(),
        stats: ["Vineta K", "Karbonis", "Zone 1", "Zone 2"]
            .into_iter()
            .map(|name| (name.to_string(), figures))
            .collect::<BTreeMap<_, _>>(),
    };
    Chain::new(
        campaign,
        oag_tables::pure_flow::parse(SKIN, SELECTION).unwrap(),
        PureClass::Vector,
    )
}

fn names() -> (Words, impl Fn(&str) -> String) {
    let words = Words {
        gold: "GOLD".into(),
        silver: "SILVER".into(),
        bronze: "BRONZE".into(),
    };
    (words, |name: &str| name.to_uppercase())
}

fn view(chain: &Chain, held: &[(&str, Medal)], unlock_all: bool) -> View {
    let (words, say) = names();
    let medal_of = |name: &str| held.iter().find(|(n, _)| *n == name).map(|(_, m)| *m);
    chain.view(
        &Profile {
            medal_of: &medal_of,
            unlock_all,
            classes_at_start: Some(START),
        },
        &Names {
            name_of: &say,
            words: &words,
        },
    )
}

fn pick(chain: &mut Chain, tag: usize, held: &[(&str, Medal)]) -> Chosen {
    let medal_of = |name: &str| held.iter().find(|(n, _)| *n == name).map(|(_, m)| *m);
    chain.choose(
        tag,
        &Profile {
            medal_of: &medal_of,
            unlock_all: false,
            classes_at_start: Some(START),
        },
    )
}

fn labels(view: &View) -> Vec<&str> {
    view.rows.iter().map(|r| r.label.as_str()).collect()
}

#[test]
fn single_player_lists_the_modes_the_disc_authors() {
    let chain = chain();
    let view = view(&chain, &[], false);
    assert_eq!(view.title, "SINGLE PLAYER");
    assert_eq!(
        labels(&view),
        ["ARCADE", "TOURNAMENT", "TIME TRIAL", "ZONE", "FREE PLAY"]
    );
}

#[test]
fn a_fresh_profile_offers_vector_and_venom_only() {
    let mut chain = chain();
    pick(&mut chain, 0, &[]);
    assert_eq!(labels(&view(&chain, &[], false)), ["VECTOR", "VENOM"]);
    assert_eq!(
        labels(&view(&chain, &[], true)),
        ["VECTOR", "VENOM", "FLASH", "RAPIER", "PHANTOM"],
        "--unlock-all offers every class"
    );
    assert_eq!(
        pick(&mut chain, 2, &[]),
        Chosen::Refused,
        "Flash is not offered, so it cannot be chosen by number either"
    );
}

#[test]
fn a_tournament_medal_opens_the_next_class_and_only_that_one() {
    let mut chain = chain();
    pick(&mut chain, 0, &[]);
    let held = [("tournament/Alpha/Venom", Medal::Bronze)];
    assert_eq!(
        labels(&view(&chain, &held, false)),
        ["VECTOR", "VENOM", "FLASH"]
    );
    let vector = [("tournament/Alpha/Vector", Medal::Gold)];
    assert_eq!(
        labels(&view(&chain, &vector, false)),
        ["VECTOR", "VENOM"],
        "Vector's medal opens Venom, which is open already"
    );
}

#[test]
fn arcade_walks_class_league_circuit_and_starts_the_single_race_event() {
    let mut chain = chain();
    assert_eq!(pick(&mut chain, 0, &[]), Chosen::Advanced);
    assert_eq!(chain.at(), Step::Class);
    assert_eq!(pick(&mut chain, 1, &[]), Chosen::Advanced);
    assert_eq!(chain.at(), Step::League);
    let leagues = view(&chain, &[], false);
    assert_eq!(
        labels(&leagues),
        ["ALPHA"],
        "the league that wants a medal is not offered, as on the original's fresh profile"
    );
    assert_eq!(leagues.context, "ARCADE  -  VENOM");
    assert_eq!(
        pick(&mut chain, 1, &[]),
        Chosen::Refused,
        "nor chosen by number"
    );
    assert_eq!(pick(&mut chain, 0, &[]), Chosen::Advanced);
    assert_eq!(chain.at(), Step::Track);
    let Chosen::Start(event) = pick(&mut chain, 0, &[]) else {
        panic!("the circuit starts the event");
    };
    assert_eq!(event.name, "race/Vineta K/Venom");
}

#[test]
fn a_medal_opens_the_league_and_the_circuit_it_gates() {
    let mut chain = chain();
    pick(&mut chain, 0, &[]);
    pick(&mut chain, 1, &[]);
    let one = [("zone/Zone 1", Medal::Bronze)];
    assert_eq!(labels(&view(&chain, &one, false)), ["ALPHA", "CLASSIC"]);
    assert_eq!(pick(&mut chain, 1, &one), Chosen::Advanced);
    assert_eq!(
        labels(&view(&chain, &one, false)),
        Vec::<&str>::new(),
        "Karbonis wants two medals and one is held"
    );
    let two = [
        ("zone/Zone 1", Medal::Bronze),
        ("tournament/Alpha/Venom", Medal::Bronze),
    ];
    assert_eq!(labels(&view(&chain, &two, false)), ["KARBONIS"]);
}

#[test]
fn a_time_trial_circuit_starts_the_time_trial_and_shows_that_events_medal() {
    let mut chain = chain();
    pick(&mut chain, 2, &[]);
    pick(&mut chain, 1, &[]);
    pick(&mut chain, 0, &[]);
    let held = [
        ("timetrial/Vineta K/Venom", Medal::Silver),
        ("race/Vineta K/Venom", Medal::Gold),
    ];
    let tracks = view(&chain, &held, false);
    assert_eq!(tracks.rows[0].status, Status::Medal(Medal::Silver));
    let Chosen::Start(event) = pick(&mut chain, 0, &held) else {
        panic!("started");
    };
    assert_eq!(event.name, "timetrial/Vineta K/Venom");
}

#[test]
fn a_tournament_skips_the_league_and_lists_only_what_is_open() {
    let mut chain = chain();
    pick(&mut chain, 1, &[]);
    pick(&mut chain, 1, &[]);
    assert_eq!(chain.at(), Step::Tournament);
    assert_eq!(labels(&view(&chain, &[], false)), ["ALPHA"]);
    assert_eq!(pick(&mut chain, 1, &[]), Chosen::Refused);
    let held = [("tournament/Alpha/Venom", Medal::Bronze)];
    let tournaments = view(&chain, &held, false);
    assert_eq!(labels(&tournaments), ["ALPHA", "BETA"]);
    assert_eq!(tournaments.rows[0].status, Status::Medal(Medal::Bronze));
    let Chosen::Start(event) = pick(&mut chain, 1, &held) else {
        panic!("Beta opens on an Alpha medal in the class being played");
    };
    assert_eq!(event.name, "tournament/Beta/Venom");
}

#[test]
fn zone_has_no_class_and_lists_only_the_circuits_it_has_opened() {
    let mut chain = chain();
    assert_eq!(pick(&mut chain, 3, &[]), Chosen::Advanced);
    assert_eq!(chain.at(), Step::ZoneTrack);
    assert_eq!(labels(&view(&chain, &[], false)), ["ZONE 1"]);
    assert_eq!(pick(&mut chain, 1, &[]), Chosen::Refused);
    let bronze = [("zone/Zone 1", Medal::Bronze)];
    assert_eq!(labels(&view(&chain, &bronze, false)), ["ZONE 1", "ZONE 2"]);
    let Chosen::Start(event) = pick(&mut chain, 0, &[]) else {
        panic!("started");
    };
    assert_eq!(event.name, "zone/Zone 1");
}

#[test]
fn free_play_picks_a_circuit_and_starts_no_event() {
    let mut chain = chain();
    pick(&mut chain, 4, &[]);
    pick(&mut chain, 1, &[]);
    pick(&mut chain, 0, &[]);
    let held = [("race/Vineta K/Venom", Medal::Gold)];
    assert_eq!(
        view(&chain, &held, false).rows[0].status,
        Status::Open,
        "no medal is shown for a mode that awards none"
    );
    let Chosen::Start(event) = pick(&mut chain, 0, &[]) else {
        panic!("started");
    };
    assert_eq!(event.name, "freeplay/Vineta K/Venom");
    assert_eq!(event.cell.mode, oag_tables::race_campaign::Mode::Race);
    assert_eq!(
        [1, 2, 3, 8].map(|place| event.cell.evaluate_medal(place)),
        [None; 4],
        "no place earns a medal"
    );
    assert!(pure_campaign::is_free_play(&event.name));
    assert_eq!(
        pure_campaign::free_play_parts(&event.name),
        Some(("Vineta K", PureClass::Venom))
    );
    chain.return_to(&event);
    assert_eq!(chain.at(), Step::Track);
    assert_eq!(chain.mode_name(), "Free Play");
}

#[test]
fn back_steps_to_the_screen_before_and_closes_from_the_first() {
    let mut chain = chain();
    pick(&mut chain, 0, &[]);
    pick(&mut chain, 0, &[]);
    assert_eq!(chain.at(), Step::League);
    assert!(chain.back());
    assert_eq!(chain.at(), Step::Class);
    assert!(chain.back());
    assert_eq!(chain.at(), Step::Mode);
    assert!(!chain.back());
}

#[test]
fn coming_back_from_a_race_lands_on_the_screen_that_picked_it() {
    let mut chain = chain();
    let event = chain
        .campaign()
        .events(PureClass::Venom)
        .into_iter()
        .find(|e| e.name == "timetrial/Vineta K/Venom")
        .unwrap();
    chain.return_to(&event);
    assert_eq!(chain.at(), Step::Track);
    assert_eq!(chain.class(), PureClass::Venom);
    assert_eq!(chain.tag_of(&event), Some(0));
    assert!(chain.back());
    assert_eq!(chain.at(), Step::League);
}

#[test]
fn every_mode_the_flow_names_is_one_this_chain_knows() {
    for mode in chain().modes() {
        assert!(ModeKind::of(mode).is_some(), "{mode}");
    }
}
