use super::*;

const DEFINITION: &str = r#"<Screen name="Top">
<PI_Track name="Vineta K"><Values type="Race" location="Data\Environments\01_Vineta_K"></Values></PI_Track>
<PI_Track name="Karbonis"><Values type="Classic" location="Data\ClassicsNS\01_Karbonis"></Values>
<Unlock MedalCount="2"></Unlock></PI_Track>
<PI_Track name="Zone 1"><Values type="Zone" location="Data\Zone\01_Zone"></Values></PI_Track>
<PI_Track name="Zone 2"><Values type="Zone" location="Data\Zone\02_Zone"></Values>
<Unlock Track="Zone 1" medal="bronze"></Unlock></PI_Track>
<PI_League name="Alpha"><Entry track="Vineta K"></Entry></PI_League>
<PI_League name="Classic"><Entry track="Karbonis"></Entry></PI_League>
<PI_Tournament name="Alpha"><Entry track="Vineta K"></Entry></PI_Tournament>
<PI_Tournament name="Beta"><Entry track="Vineta K"></Entry>
<Unlock Tournament="Alpha" class="asSelected" medal="Any"></Unlock></PI_Tournament>
</Screen>"#;

const STATS: &str = r#"<Stats>
<RaceTimes Bronze="6" Flash="124" Phantom="122" Rapier="109" Silver="3" Vector="124" Venom="105"/>
<ZoneMedals Bronze="20" Gold="30" Silver="25"/></Stats>"#;

fn campaign() -> PureCampaign {
    let figures = pure_campaign::parse_stats(STATS).unwrap();
    PureCampaign {
        definition: pure_campaign::parse(DEFINITION).unwrap(),
        stats: ["Vineta K", "Karbonis", "Zone 1", "Zone 2"]
            .into_iter()
            .map(|name| (name.to_string(), figures))
            .collect(),
    }
}

fn words() -> Words {
    Words {
        locked: "LOCKED".into(),
        needs: "NEEDS".into(),
        medals: "MEDALS".into(),
        any_medal: "ANY MEDAL".into(),
        gold: "GOLD".into(),
        silver: "SILVER".into(),
        bronze: "BRONZE".into(),
    }
}

fn rows_with(held: &[(&str, Medal)], class: PureClass, unlock_all: bool) -> (Vec<Row>, Vec<Event>) {
    let words = words();
    let medal_of = |name: &str| held.iter().find(|(n, _)| *n == name).map(|(_, m)| *m);
    rows(
        &campaign(),
        class,
        &medal_of,
        unlock_all,
        &Names {
            name_of: &|name| name.to_uppercase(),
            heading: &|kind| format!("{kind:?}").to_uppercase(),
            words: &words,
        },
    )
}

fn row_of<'a>(rows: &'a [Row], events: &[Event], name: &str) -> &'a Row {
    let tag = events.iter().position(|e| e.name == name).unwrap();
    rows.iter().find(|r| !r.header && r.tag == tag).unwrap()
}

#[test]
fn a_fresh_profile_locks_what_the_disc_gates_and_says_what_opens_it() {
    let (rows, events) = rows_with(&[], PureClass::Venom, false);
    assert_eq!(
        row_of(&rows, &events, "race/Vineta K/Venom").status,
        Status::Open
    );
    let karbonis = row_of(&rows, &events, "race/Karbonis/Venom");
    assert_eq!(karbonis.status, Status::Locked);
    assert_eq!(karbonis.requires, "NEEDS 2 MEDALS");
    let beta = row_of(&rows, &events, "tournament/Beta/Venom");
    assert_eq!(beta.requires, "NEEDS ALPHA ANY MEDAL");
    let zone2 = row_of(&rows, &events, "zone/Zone 2");
    assert_eq!(zone2.requires, "NEEDS ZONE 1 BRONZE");
}

#[test]
fn a_medal_opens_the_gate_it_names_and_shows_on_its_own_row() {
    let held = [
        ("tournament/Alpha/Venom", Medal::Silver),
        ("zone/Zone 1", Medal::Gold),
    ];
    let (rows, events) = rows_with(&held, PureClass::Venom, false);
    assert_eq!(
        row_of(&rows, &events, "tournament/Alpha/Venom").status,
        Status::Medal(Medal::Silver)
    );
    assert_eq!(
        row_of(&rows, &events, "tournament/Beta/Venom").status,
        Status::Open
    );
    assert_eq!(row_of(&rows, &events, "zone/Zone 2").status, Status::Open);
    assert_eq!(
        row_of(&rows, &events, "race/Karbonis/Venom").status,
        Status::Open,
        "two medals are what Karbonis wants, and two are held"
    );
    let (rows, events) = rows_with(&held[..1], PureClass::Venom, false);
    assert_eq!(
        row_of(&rows, &events, "race/Karbonis/Venom").status,
        Status::Locked,
        "one is not enough"
    );
    let (rows, events) = rows_with(&held, PureClass::Flash, false);
    assert_eq!(
        row_of(&rows, &events, "tournament/Beta/Flash").status,
        Status::Locked,
        "a Venom medal does not open Flash's Beta"
    );
    assert_eq!(
        row_of(&rows, &events, "race/Karbonis/Flash").status,
        Status::Open,
        "but the medal count is the profile's, whatever the class"
    );
}

#[test]
fn unlock_all_opens_every_gate() {
    let (rows, _) = rows_with(&[], PureClass::Venom, true);
    assert!(rows.iter().all(|r| r.status != Status::Locked));
}

#[test]
fn the_list_groups_by_mode_under_headings_and_tags_index_the_events() {
    let (rows, events) = rows_with(&[], PureClass::Venom, false);
    let headings: Vec<&str> = rows
        .iter()
        .filter(|r| r.header)
        .map(|r| r.label.as_str())
        .collect();
    assert_eq!(headings, ["TOURNAMENT", "RACE", "TIMETRIAL", "ZONE"]);
    for row in rows.iter().filter(|r| !r.header) {
        assert!(row.tag < events.len());
    }
}

#[test]
fn classic_circuits_resolve_through_the_campaigns_own_locations() {
    let campaign = campaign();
    assert_eq!(
        campaign.track_entry("Karbonis").as_deref(),
        Some(r"Data\ClassicsNS\01_Karbonis\track.vex")
    );
    assert_eq!(
        campaign.track_entry("Zone 1"),
        None,
        "Zone circuits go the Zone way"
    );
}

#[test]
fn a_vector_event_races_in_vector_and_a_zone_event_keeps_the_fallback() {
    use crate::campaign::launch::{plan_cell, plan_cell_for};
    let ladder = oag_pure::TITLE.race.speed_classes.unwrap().names;
    let campaign = campaign();
    let entry = |id: &str| {
        campaign
            .track_entry(id)
            .or_else(|| Some(format!("zone {id}")))
    };
    let by = |class: PureClass, name: &str| {
        campaign
            .events(class)
            .into_iter()
            .find(|e| e.name == name)
            .unwrap()
    };
    let race = by(PureClass::Vector, "race/Vineta K/Vector");
    let plan = plan_cell_for(&race.cell, "Phantom", ladder, |_, id| entry(id)).unwrap();
    assert_eq!(
        (plan.class.as_str(), plan.class_is_fallback),
        ("Vector", false)
    );
    let plan = plan_cell(&race.cell, "Phantom", |_, id| entry(id)).unwrap();
    assert_eq!(
        (plan.class.as_str(), plan.class_is_fallback),
        ("Phantom", true),
        "a title with the four-rung ladder falls back, as it always did"
    );
    let zone = by(PureClass::Vector, "zone/Zone 1");
    let plan = plan_cell_for(&zone.cell, "Phantom", ladder, |_, id| entry(id)).unwrap();
    assert_eq!(
        (plan.class.as_str(), plan.class_is_fallback),
        ("Phantom", true)
    );
    let tournament = by(PureClass::Flash, "tournament/Alpha/Flash");
    let plan = plan_cell_for(&tournament.cell, "Phantom", ladder, |_, id| entry(id)).unwrap();
    assert_eq!(plan.class, "Flash");
}

#[test]
fn a_gated_team_is_locked_until_the_disc_says_otherwise() {
    let xml = DEFINITION.replace(
        "<PI_League name=\"Alpha\">",
        r#"<PI_Team name="Zone"><Values location="Data\Ships\Zone"></Values>
<Unlock Track="Zone 1" medal="Gold"></Unlock></PI_Team>
<PI_Team name="Feisar"><Values location="Data\Ships\Feisar"></Values></PI_Team>
<PI_League name="Alpha">"#,
    );
    let campaign = PureCampaign {
        definition: pure_campaign::parse(&xml).unwrap(),
        stats: BTreeMap::new(),
    };
    let none = |_: &str| None;
    assert_eq!(locked_teams(&campaign, &none, "Venom"), ["Zone"]);
    let bronze = |name: &str| (name == "zone/Zone 1").then_some(Medal::Bronze);
    assert_eq!(
        locked_teams(&campaign, &bronze, "Venom"),
        ["Zone"],
        "bronze is not gold"
    );
    let gold = |name: &str| (name == "zone/Zone 1").then_some(Medal::Gold);
    assert!(locked_teams(&campaign, &gold, "Venom").is_empty());
}
