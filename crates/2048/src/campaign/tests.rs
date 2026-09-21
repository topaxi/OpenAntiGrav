use super::*;

#[test]
fn eclass_ordinals_round_trip() {
    for class in EClass::ALL {
        assert_eq!(EClass::from_ordinal(class.ordinal()), Some(class));
    }
    assert_eq!(EClass::from_ordinal(5), None);
    assert_eq!(EClass::from_ordinal(-1), None);
}

#[test]
fn only_superphantom_has_no_four_wide_speed_class() {
    assert_eq!(
        EClass::Venom.speed_class(),
        Some(oag_tables::handling::SpeedClass::Venom)
    );
    assert_eq!(
        EClass::Phantom.speed_class(),
        Some(oag_tables::handling::SpeedClass::Phantom)
    );
    assert_eq!(EClass::SuperPhantom.speed_class(), None);
}

fn race_event(kind: EventKind, typedef_id: i64, laps: Option<u32>) -> Event {
    Event {
        instance_id: 1,
        typedef_id,
        kind,
        name: String::new(),
        description: None,
        track: None,
        speed_class: None,
        laps,
        weapon_set: None,
        next_event: None,
        branch_event: None,
        required_event: None,
        pass_objective: None,
        elite_objective: None,
        x: None,
        y: None,
        rank_required: None,
        force_always_unlocked: None,
        has_ghost_capacity: false,
    }
}

#[test]
fn engine_mode_maps_zero_laps_to_speed_lap_and_elimination_to_eliminator() {
    assert_eq!(
        engine_mode(&race_event(EventKind::Race, typedef::RACE_A, Some(0))),
        Some("speed_lap")
    );
    assert_eq!(
        engine_mode(&race_event(EventKind::Race, typedef::RACE_A, Some(3))),
        Some("single_race")
    );
    assert_eq!(
        engine_mode(&race_event(EventKind::Race, typedef::RACE_B, Some(5))),
        Some("single_race")
    );
    assert_eq!(
        engine_mode(&race_event(EventKind::Zone, typedef::ZONE, None)),
        Some("zone")
    );
    assert_eq!(
        engine_mode(&race_event(
            EventKind::Elimination,
            typedef::ELIMINATION,
            Some(1)
        )),
        Some("eliminator")
    );
}

#[test]
fn track_vex_entry_matches_default_tracks_own_spelling() {
    let altima = Track {
        instance_id: 1,
        name: "Altima".to_string(),
        track_name: "altima".to_string(),
        display_name: "ALTIMA".to_string(),
    };
    assert_eq!(
        track_vex_entry(&altima),
        crate::race::DEFAULT_TRACK,
        "must match oag_2048::race::DEFAULT_TRACK's own spelling exactly"
    );
}

/// Three chained events, shaped after the real `SP.xml` census this
/// module's own doc comments cite: a `RACE_B` opener with
/// `FINISH`/`POSITION` objectives and no gate of its own, a `RACE_A`
/// time-based follow-on reached by `M_PNEXTEVENT`, and a `ZONE` side
/// event reached by `M_PEVENTREQUIRED` alone (never chained).
const CAMPAIGN_FIXTURE: &str = r#"<mjolnir>
<instance instanceid="1" typedefid="-1353052320" name="2048 - Event 1"><DATA>
<M_PNEXTEVENT name="m_pNextEvent" type="GameModeBase" length="1" typedefid="366306753"><ARRAY value="2" typedefid="-1915183557"/></M_PNEXTEVENT>
<M_PASSOBJECTIVE name="m_passObjective" type="GameModeObjective" length="1" typedefid="380278911"><ARRAY value="10" typedefid="380278911"/></M_PASSOBJECTIVE>
<M_ELITEOBJECTIVE name="m_eliteObjective" type="GameModeObjective" length="1" typedefid="380278911"><ARRAY value="11" typedefid="380278911"/></M_ELITEOBJECTIVE>
</DATA></instance>
<instance instanceid="2" typedefid="-1915183557" name="2048 - Event 2"><DATA>
<M_PEVENTREQUIRED name="m_pEventRequired" type="GameModeBase" length="1" typedefid="366306753"><ARRAY value="1" typedefid="-1353052320"/></M_PEVENTREQUIRED>
<M_PASSOBJECTIVE name="m_passObjective" type="GameModeObjective" length="1" typedefid="380278911"><ARRAY value="20" typedefid="380278911"/></M_PASSOBJECTIVE>
<M_ELITEOBJECTIVE name="m_eliteObjective" type="GameModeObjective" length="1" typedefid="380278911"><ARRAY value="21" typedefid="380278911"/></M_ELITEOBJECTIVE>
</DATA></instance>
<instance instanceid="3" typedefid="1018671239" name="2049 - Event 1"><DATA>
<M_PEVENTREQUIRED name="m_pEventRequired" type="GameModeBase" length="1" typedefid="366306753"><ARRAY value="1" typedefid="-1353052320"/></M_PEVENTREQUIRED>
<M_PASSOBJECTIVE name="m_passObjective" type="GameModeObjective" length="1" typedefid="380278911"><ARRAY value="30" typedefid="380278911"/></M_PASSOBJECTIVE>
<M_ELITEOBJECTIVE name="m_eliteObjective" type="GameModeObjective" length="1" typedefid="380278911"><ARRAY value="31" typedefid="380278911"/></M_ELITEOBJECTIVE>
</DATA></instance>
<instance instanceid="10" typedefid="380278911" name="FinishRaceAnyPosition"><DATA>
<M_OBJECTIVETYPE name="m_ObjectiveType" type="ObjectiveValue" length="1" typedefid="-66037811"><ARRAY value="1" typedefid="-66037811"/></M_OBJECTIVETYPE>
<M_OBJECTIVETARGET name="m_ObjectiveTarget" type="u32" length="1" typedefid="-1854316044"><ARRAY value="" typedefid="-1854316044"/></M_OBJECTIVETARGET>
</DATA></instance>
<instance instanceid="11" typedefid="380278911" name="Win"><DATA>
<M_OBJECTIVETYPE name="m_ObjectiveType" type="ObjectiveValue" length="1" typedefid="-66037811"><ARRAY value="4" typedefid="-66037811"/></M_OBJECTIVETYPE>
<M_OBJECTIVETARGET name="m_ObjectiveTarget" type="u32" length="1" typedefid="-1854316044"><ARRAY value="1" typedefid="-1854316044"/></M_OBJECTIVETARGET>
</DATA></instance>
<instance instanceid="20" typedefid="380278911" name="2048 - Event 2 Pass"><DATA>
<M_OBJECTIVETYPE name="m_ObjectiveType" type="ObjectiveValue" length="1" typedefid="-66037811"><ARRAY value="2" typedefid="-66037811"/></M_OBJECTIVETYPE>
<M_OBJECTIVETARGET name="m_ObjectiveTarget" type="u32" length="1" typedefid="-1854316044"><ARRAY value="13000" typedefid="-1854316044"/></M_OBJECTIVETARGET>
</DATA></instance>
<instance instanceid="21" typedefid="380278911" name="2048 - Event 2 Elite"><DATA>
<M_OBJECTIVETYPE name="m_ObjectiveType" type="ObjectiveValue" length="1" typedefid="-66037811"><ARRAY value="2" typedefid="-66037811"/></M_OBJECTIVETYPE>
<M_OBJECTIVETARGET name="m_ObjectiveTarget" type="u32" length="1" typedefid="-1854316044"><ARRAY value="11000" typedefid="-1854316044"/></M_OBJECTIVETARGET>
</DATA></instance>
<instance instanceid="30" typedefid="380278911" name="Zone Pass"><DATA>
<M_OBJECTIVETYPE name="m_ObjectiveType" type="ObjectiveValue" length="1" typedefid="-66037811"><ARRAY value="2" typedefid="-66037811"/></M_OBJECTIVETYPE>
<M_OBJECTIVETARGET name="m_ObjectiveTarget" type="u32" length="1" typedefid="-1854316044"><ARRAY value="10" typedefid="-1854316044"/></M_OBJECTIVETARGET>
</DATA></instance>
<instance instanceid="31" typedefid="380278911" name="Zone Elite"><DATA>
<M_OBJECTIVETYPE name="m_ObjectiveType" type="ObjectiveValue" length="1" typedefid="-66037811"><ARRAY value="2" typedefid="-66037811"/></M_OBJECTIVETYPE>
<M_OBJECTIVETARGET name="m_ObjectiveTarget" type="u32" length="1" typedefid="-1854316044"><ARRAY value="20" typedefid="-1854316044"/></M_OBJECTIVETARGET>
</DATA></instance>
</mjolnir>"#;

#[test]
fn finish_and_position_objectives_grade_a_race_event() {
    let doc = parse(CAMPAIGN_FIXTURE);
    let event = events(&doc)
        .into_iter()
        .find(|e| e.name == "2048 - Event 1")
        .unwrap();
    let objectives = event_objectives(&doc, &event).unwrap();

    // Never finished: neither bar is decidable, so no tier at all.
    assert_eq!(evaluate_tier(&objectives, &EventOutcome::default()), None);
    // Finished, but outside the podium: FINISH's own bar has no target,
    // so finishing at all clears it.
    assert_eq!(
        evaluate_tier(
            &objectives,
            &EventOutcome {
                finished: true,
                place: 8,
                ..EventOutcome::default()
            }
        ),
        Some(Tier::Pass)
    );
    // First place clears Win (target 1) too.
    assert_eq!(
        evaluate_tier(
            &objectives,
            &EventOutcome {
                finished: true,
                place: 1,
                ..EventOutcome::default()
            }
        ),
        Some(Tier::Elite)
    );
}

#[test]
fn beat_value_reads_finish_time_on_a_race_and_zone_count_on_a_zone() {
    let doc = parse(CAMPAIGN_FIXTURE);

    let race = events(&doc)
        .into_iter()
        .find(|e| e.name == "2048 - Event 2")
        .unwrap();
    let race_objectives = event_objectives(&doc, &race).unwrap();
    // Slower than both bars (pass 13000cs, elite 11000cs).
    assert_eq!(
        evaluate_tier(
            &race_objectives,
            &EventOutcome {
                finished: true,
                finish_centiseconds: Some(15000),
                ..EventOutcome::default()
            }
        ),
        None
    );
    // Between the two: lower is better for a Race, so this clears Pass
    // but not Elite.
    assert_eq!(
        evaluate_tier(
            &race_objectives,
            &EventOutcome {
                finished: true,
                finish_centiseconds: Some(12000),
                ..EventOutcome::default()
            }
        ),
        Some(Tier::Pass)
    );
    // A Zone medal never applies to a Race event even if the zone field
    // happened to carry a value that would clear it.
    assert_eq!(
        evaluate_tier(
            &race_objectives,
            &EventOutcome {
                zone: 999,
                ..EventOutcome::default()
            }
        ),
        None
    );

    let zone = events(&doc)
        .into_iter()
        .find(|e| e.name == "2049 - Event 1")
        .unwrap();
    let zone_objectives = event_objectives(&doc, &zone).unwrap();
    // Higher is better for Zone, and it is graded with no `finished` at
    // all - the counter only ever grows.
    assert_eq!(
        evaluate_tier(
            &zone_objectives,
            &EventOutcome {
                zone: 5,
                ..EventOutcome::default()
            }
        ),
        None
    );
    assert_eq!(
        evaluate_tier(
            &zone_objectives,
            &EventOutcome {
                zone: 20,
                ..EventOutcome::default()
            }
        ),
        Some(Tier::Elite)
    );
}

#[test]
fn unlock_gates_collapse_a_chain_edge_and_a_required_edge_to_one_name() {
    let doc = parse(CAMPAIGN_FIXTURE);
    let gates = unlock_gates(&doc);

    let gate_of = |name: &str| {
        gates
            .iter()
            .find(|(event, _)| event == name)
            .and_then(|(_, gate)| gate.clone())
    };

    assert_eq!(
        gate_of("2048 - Event 1"),
        None,
        "no incoming edge and no M_PEVENTREQUIRED: open from the start"
    );
    assert_eq!(
        gate_of("2048 - Event 2"),
        Some("2048 - Event 1".to_string()),
        "reached by M_PNEXTEVENT alone"
    );
    assert_eq!(
        gate_of("2049 - Event 1"),
        Some("2048 - Event 1".to_string()),
        "reached by M_PEVENTREQUIRED alone, never chained"
    );
}
