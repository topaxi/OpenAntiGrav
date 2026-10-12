use super::*;

const DEFINITION: &str = r#"<Screen name="Top">
<PI_Track name="Vineta K"><Values type="Race" location="Data\Environments\01_Vineta_K"></Values></PI_Track>
<PI_Track name="Karbonis"><Values type="Classic" location="Data\ClassicsNS\01_Karbonis"></Values>
<Unlock MedalCount="10"></Unlock></PI_Track>
<PI_Track name="Zone 1"><Values type="Zone" location="Data\Zone\01_Zone"></Values></PI_Track>
<PI_Track name="Zone 2"><Values type="Zone" location="Data\Zone\02_Zone"></Values>
<Unlock Track="Zone 1" medal="bronze"></Unlock></PI_Track>
<PI_Team name="Medievil"><Values location="Data\Ships\Medievil"></Values><Unlock MedalCount="70"></Unlock></PI_Team>
<PI_League name="Alpha"><Entry track="Vineta K"></Entry></PI_League>
<PI_League name="Classic"><Entry track="Karbonis"></Entry><Unlock MedalCount="10"></Unlock></PI_League>
<PI_Tournament name="Alpha"><Entry track="Vineta K"></Entry></PI_Tournament>
<PI_Tournament name="Beta"><Entry track="Vineta K"></Entry>
<Unlock Tournament="Alpha" class="asSelected" medal="Any"></Unlock></PI_Tournament>
<PI_Tournament name="Classic"><Entry track="Karbonis"></Entry><Unlock MedalCount="60"></Unlock></PI_Tournament>
</Screen>"#;

const STATS: &str = r#"<Stats>
<RaceTimes Bronze="6" Flash="124" Phantom="122" Rapier="109" Silver="3" Vector="124" Venom="105"/>
<ZoneMedals Bronze="20" Gold="30" Silver="25"/></Stats>"#;

fn stats() -> BTreeMap<String, Stats> {
    let figures = parse_stats(STATS).unwrap();
    ["Vineta K", "Karbonis", "Zone 1", "Zone 2"]
        .into_iter()
        .map(|name| (name.to_string(), figures))
        .collect()
}

#[test]
fn a_league_and_a_tournament_of_one_name_keep_their_own_gate() {
    let definition = parse(DEFINITION).unwrap();
    let gate = |series: &[Series]| {
        series.iter().find(|s| s.name == "Classic").unwrap().unlocks[0].medal_count
    };
    assert_eq!(gate(&definition.leagues), Some(10));
    assert_eq!(gate(&definition.tournaments), Some(60));
}

#[test]
fn medal_casing_and_any_are_matched_whatever_the_file_spells() {
    let definition = parse(DEFINITION).unwrap();
    let zone2 = definition
        .tracks
        .iter()
        .find(|t| t.name == "Zone 2")
        .unwrap();
    assert_eq!(
        zone2.unlocks[0].medal,
        Some(MedalWanted::AtLeast(Medal::Bronze))
    );
    let beta = &definition.tournaments[1];
    assert_eq!(beta.unlocks[0].medal, Some(MedalWanted::Any));
    assert_eq!(beta.unlocks[0].class, Some(ClassRef::AsSelected));
}

#[test]
fn an_unlock_this_reader_does_not_model_is_refused_by_name() {
    let xml = DEFINITION.replace(r#"MedalCount="70""#, r#"Loyalty="5""#);
    let error = parse(&xml).unwrap_err();
    assert!(
        matches!(error, Error::Unmodelled(ref what) if what.contains("loyalty")),
        "{error}"
    );
    let xml = DEFINITION.replace(r#"Track="Zone 1""#, r#"Track="Vineta K""#);
    assert!(matches!(parse(&xml), Err(Error::Unmodelled(_))));
}

#[test]
fn a_tier_or_better_passes_a_better_medal_and_a_class_is_the_one_played() {
    let definition = parse(DEFINITION).unwrap();
    let zone2 = &definition.tracks[3].unlocks;
    let silver = |name: &str| (name == "zone/Zone 1").then_some(Medal::Silver);
    let progress = |class, count| Progress {
        class,
        medal_count: count,
        medal_of: &silver,
    };
    assert!(
        all_met(zone2, &progress("Venom", 0)),
        "silver clears bronze"
    );
    let none = |_: &str| None;
    assert!(!all_met(
        zone2,
        &Progress {
            class: "Venom",
            medal_count: 0,
            medal_of: &none
        }
    ));
    let beta = &definition.tournaments[1].unlocks;
    let alpha_venom = |name: &str| (name == "tournament/Alpha/Venom").then_some(Medal::Bronze);
    let at = |class| Progress {
        class,
        medal_count: 0,
        medal_of: &alpha_venom,
    };
    assert!(all_met(beta, &at("Venom")));
    assert!(
        !all_met(beta, &at("Flash")),
        "a Venom medal does not open Flash"
    );
}

#[test]
fn the_list_is_tournaments_then_races_then_time_trials_then_zone() {
    let definition = parse(DEFINITION).unwrap();
    let list = events(&definition, &stats(), PureClass::Venom);
    let kinds: Vec<EventKind> = list.iter().map(|e| e.kind).collect();
    assert_eq!(
        kinds,
        [
            EventKind::Tournament,
            EventKind::Tournament,
            EventKind::Tournament,
            EventKind::Race,
            EventKind::Race,
            EventKind::TimeTrial,
            EventKind::TimeTrial,
            EventKind::Zone,
            EventKind::Zone,
        ]
    );
    let race = list
        .iter()
        .find(|e| e.name == "race/Karbonis/Venom")
        .unwrap();
    assert_eq!(race.unlocks.len(), 2, "its league's gate and its own");
}

#[test]
fn the_medal_law_is_the_executables_for_each_mode() {
    let definition = parse(DEFINITION).unwrap();
    let list = events(&definition, &stats(), PureClass::Venom);
    let by = |name: &str| list.iter().find(|e| e.name == name).unwrap();
    let race = &by("race/Vineta K/Venom").cell;
    assert_eq!(race.evaluate_medal(1), Some(Medal::Gold));
    assert_eq!(race.evaluate_medal(3), Some(Medal::Bronze));
    assert_eq!(race.evaluate_medal(4), None);
    let trial = &by("timetrial/Vineta K/Venom").cell;
    assert_eq!(
        (trial.gold, trial.silver, trial.bronze),
        (10_500, 10_800, 11_100)
    );
    assert_eq!(trial.evaluate_medal(10_500), Some(Medal::Gold));
    assert_eq!(trial.evaluate_medal(10_501), Some(Medal::Silver));
    assert_eq!(trial.evaluate_medal(11_101), None);
    let zone = &by("zone/Zone 1").cell;
    assert_eq!(zone.evaluate_medal(30), Some(Medal::Gold));
    assert_eq!(zone.evaluate_medal(20), Some(Medal::Bronze));
    assert_eq!(zone.evaluate_medal(19), None);
    let class_time = |class: PureClass| {
        let list = events(&definition, &stats(), class);
        list.iter()
            .find(|e| e.name == format!("timetrial/Vineta K/{}", class.title()))
            .unwrap()
            .cell
            .gold
    };
    assert_eq!(
        class_time(PureClass::Rapier),
        10_900,
        "each class has its own gold time"
    );
}

#[test]
fn a_circuit_without_stats_gets_no_invented_time_or_zone_target() {
    let definition = parse(DEFINITION).unwrap();
    let list = events(&definition, &BTreeMap::new(), PureClass::Venom);
    assert!(
        list.iter()
            .all(|e| e.kind != EventKind::TimeTrial && e.kind != EventKind::Zone)
    );
}
