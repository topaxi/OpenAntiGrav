use super::*;
use crate::mjolnir::parse;

/// Four instances covering the typed view: a `RACE_A` event chained to a
/// `RACE_B` one (as `"2048 - Event 3"` chains to `"2048 - Event 4"`), a `ZONE`
/// event with no track/speed class, a `TrackDefinition` and a
/// `WeaponSetDefinition`.
const FIXTURE: &str = r#"<mjolnir><instance instanceid="-1143582041" typedefid="-1915183557" name="2048 - Event 3" schema="0" version="0" file="SP.xml"><DATA>
<M_DESCRIPTION name="m_description" type="char" length="64" typedefid="1380284284"><ARRAY value="2048_EVENT_3" typedefid="1380284284"/></M_DESCRIPTION>
<M_TRACKDEF name="m_trackDef" type="TrackDefinition" length="1" typedefid="205052969"><ARRAY value="-1892961298" typedefid="205052969"/></M_TRACKDEF>
<M_SPEEDCLASS name="m_speedClass" type="eClass" length="1" typedefid="-1934651500"><ARRAY value="1" typedefid="-1934651500"/></M_SPEEDCLASS>
<M_NUMOFLAPS name="m_numOfLaps" type="int" length="1" typedefid="351272028"><ARRAY value="2" typedefid="351272028"/></M_NUMOFLAPS>
<M_WEAPONSET name="m_weaponSet" type="WeaponSetDefinition" length="1" typedefid="-966434245"><ARRAY value="-692702972" typedefid="-966434245"/></M_WEAPONSET>
<M_PNEXTEVENT name="m_pNextEvent" type="GameModeBase" length="1" typedefid="366306753"><ARRAY value="-484309551" typedefid="-1353052320"/></M_PNEXTEVENT>
<M_PBRANCHEVENT name="m_pBranchEvent" type="GameModeBase" length="1" typedefid="366306753"><ARRAY value="890052595" typedefid="-1915183557"/></M_PBRANCHEVENT>
<M_PASSOBJECTIVE name="m_passObjective" type="GameModeObjective" length="1" typedefid="380278911"><ARRAY value="13000" typedefid="380278911"/></M_PASSOBJECTIVE>
<M_ELITEOBJECTIVE name="m_eliteObjective" type="GameModeObjective" length="1" typedefid="380278911"><ARRAY value="11000" typedefid="380278911"/></M_ELITEOBJECTIVE>
<M_X name="m_x" type="int" length="1" typedefid="351272028"><ARRAY value="5" typedefid="351272028"/></M_X>
<M_Y name="m_y" type="int" length="1" typedefid="351272028"><ARRAY value="5" typedefid="351272028"/></M_Y>
<M_MAXGHOSTSHIPS name="m_maxGhostShips" type="int" length="1" typedefid="351272028"><ARRAY value="" typedefid="351272028"/></M_MAXGHOSTSHIPS>
</DATA></instance>
<instance instanceid="-484309551" typedefid="-1353052320" name="2048 - Event 4" schema="0" version="0" file="SP.xml"><DATA>
<M_NUMOFLAPS name="m_numOfLaps" type="int" length="1" typedefid="351272028"><ARRAY value="5" typedefid="351272028"/></M_NUMOFLAPS>
</DATA></instance>
<instance instanceid="13000" typedefid="380278911" name="2048 - Event 3 Pass" schema="0" version="0" file="SP.xml"><DATA>
<M_OBJECTIVETYPE name="m_ObjectiveType" type="ObjectiveValue" length="1" typedefid="-66037811"><ARRAY value="2" typedefid="-66037811"/></M_OBJECTIVETYPE>
<M_OBJECTIVETARGET name="m_ObjectiveTarget" type="u32" length="1" typedefid="-1854316044"><ARRAY value="13000" typedefid="-1854316044"/></M_OBJECTIVETARGET>
</DATA></instance>
<instance instanceid="11000" typedefid="380278911" name="2048 - Event 3 Elite" schema="0" version="0" file="SP.xml"><DATA>
<M_OBJECTIVETYPE name="m_ObjectiveType" type="ObjectiveValue" length="1" typedefid="-66037811"><ARRAY value="2" typedefid="-66037811"/></M_OBJECTIVETYPE>
<M_OBJECTIVETARGET name="m_ObjectiveTarget" type="u32" length="1" typedefid="-1854316044"><ARRAY value="11000" typedefid="-1854316044"/></M_OBJECTIVETARGET>
</DATA></instance>
<instance instanceid="1" typedefid="380278911" name="MostDamage" schema="0" version="0" file="SP.xml"><DATA>
<M_OBJECTIVETYPE name="m_ObjectiveType" type="ObjectiveValue" length="1" typedefid="-66037811"><ARRAY value="" typedefid="-66037811"/></M_OBJECTIVETYPE>
<M_OBJECTIVETARGET name="m_ObjectiveTarget" type="u32" length="1" typedefid="-1854316044"><ARRAY value="" typedefid="-1854316044"/></M_OBJECTIVETARGET>
</DATA></instance>
<instance instanceid="777" typedefid="1018671239" name="2049 - Event 3" schema="0" version="0" file="SP.xml"><DATA>
<M_ZONETIMECOUNTER name="m_zoneTimeCounter" type="int" length="1" typedefid="351272028"><ARRAY value="1" typedefid="351272028"/></M_ZONETIMECOUNTER>
</DATA></instance>
<instance instanceid="-405306526" typedefid="205052969" name="Bridge" schema="0" version="0" file="SP.xml"><DATA>
<M_TRACKNAME name="m_trackName" type="char" length="64" typedefid="1380284284"><ARRAY value="bridge" typedefid="1380284284"/></M_TRACKNAME>
<M_DISPLAYNAME name="m_displayName" type="char" length="64" typedefid="1380284284"><ARRAY value="CAPITAL REACH" typedefid="1380284284"/></M_DISPLAYNAME>
</DATA></instance>
<instance instanceid="-1892961298" typedefid="205052969" name="Park" schema="0" version="0" file="SP.xml"><DATA>
<M_TRACKNAME name="m_trackName" type="char" length="64" typedefid="1380284284"><ARRAY value="park" typedefid="1380284284"/></M_TRACKNAME>
<M_DISPLAYNAME name="m_displayName" type="char" length="64" typedefid="1380284284"><ARRAY value="METRO PARK" typedefid="1380284284"/></M_DISPLAYNAME>
</DATA></instance>
<instance instanceid="-692702972" typedefid="-966434245" name="Rockets Only" schema="0" version="0" file="SP.xml"><DATA>
<M_WEAPONAVAILABLEBITS name="m_weaponAvailableBits" type="WeaponType" length="1" typedefid="2139957613"><ARRAY value="1" typedefid="2139957613"/></M_WEAPONAVAILABLEBITS>
</DATA></instance>
</mjolnir>"#;

#[test]
fn events_finds_the_three_event_kinds_and_skips_tracks_and_weapon_sets() {
    let doc = parse(FIXTURE);
    let events = events(&doc);
    assert_eq!(events.len(), 3);
    assert!(events.iter().any(|e| e.name == "2048 - Event 3"));
    assert!(events.iter().any(|e| e.name == "2048 - Event 4"));
    assert!(events.iter().any(|e| e.name == "2049 - Event 3"));
}

#[test]
fn an_event_carries_its_own_kind() {
    let doc = parse(FIXTURE);
    let events = events(&doc);
    let race = events.iter().find(|e| e.name == "2048 - Event 3").unwrap();
    assert_eq!(race.kind, EventKind::Race);
    assert_eq!(race.typedef_id, typedef::RACE_A);

    let zone = events.iter().find(|e| e.name == "2049 - Event 3").unwrap();
    assert_eq!(zone.kind, EventKind::Zone);
    assert_eq!(
        zone.track, None,
        "no M_TRACKDEF authored on the Zone fixture"
    );
    assert_eq!(zone.speed_class, None);
}

#[test]
fn event_3_reads_the_full_field_set_measured_off_the_real_file() {
    let doc = parse(FIXTURE);
    let event = events(&doc)
        .into_iter()
        .find(|e| e.name == "2048 - Event 3")
        .unwrap();

    assert_eq!(event.description.as_deref(), Some("2048_EVENT_3"));
    assert_eq!(event.speed_class, Some(1));
    assert_eq!(event.laps, Some(2));
    assert!(event.has_ghost_capacity);

    let track = track_for(&doc, event.track.unwrap()).unwrap();
    assert_eq!(track.track_name, "park");
    assert_eq!(track.display_name, "METRO PARK");

    let weapon_set = weapon_set_for(&doc, event.weapon_set.unwrap()).unwrap();
    assert_eq!(weapon_set.name, "Rockets Only");
    assert_eq!(weapon_set.available_bits, Some(1));

    let next = event.next_event.unwrap();
    assert_eq!(next.instance_id, -484309551);
    assert_eq!(
        next.typedef_id,
        Some(typedef::RACE_B),
        "the chain crosses race typedefs"
    );

    let branch = event.branch_event.unwrap();
    assert_eq!(branch.instance_id, 890052595);
    assert_eq!(branch.typedef_id, Some(typedef::RACE_A));

    assert_eq!(event.x, Some(5));
    assert_eq!(event.y, Some(5));
}

#[test]
fn laps_zero_is_not_produced_by_this_fixture_but_the_field_still_parses_as_some_zero() {
    // Speed Lap's sentinel (typedef::RACE_A): a synthetic zero, since
    // `Option<u32>` must tell "authored zero" from "absent".
    let xml = r#"<mjolnir><instance instanceid="1" typedefid="-1915183557" name="Bridge Speed Lap - Flash"><DATA>
<M_NUMOFLAPS name="m_numOfLaps" type="int" length="1" typedefid="351272028"><ARRAY value="0" typedefid="351272028"/></M_NUMOFLAPS>
</DATA></instance></mjolnir>"#;
    let doc = parse(xml);
    let event = events(&doc).into_iter().next().unwrap();
    assert_eq!(event.laps, Some(0));
}

#[test]
fn an_events_pass_and_elite_objectives_resolve_to_their_own_type_and_target() {
    let doc = parse(FIXTURE);
    let event = events(&doc)
        .into_iter()
        .find(|e| e.name == "2048 - Event 3")
        .unwrap();

    let pass = objective_for(&doc, event.pass_objective.unwrap()).unwrap();
    assert_eq!(pass.name, "2048 - Event 3 Pass");
    assert_eq!(pass.objective_type, Some(2));
    assert_eq!(pass.target, Some(13000));

    let elite = objective_for(&doc, event.elite_objective.unwrap()).unwrap();
    assert_eq!(elite.name, "2048 - Event 3 Elite");
    assert_eq!(elite.objective_type, Some(2));
    assert_eq!(elite.target, Some(11000));
}

#[test]
fn an_objective_with_no_authored_type_or_target_reads_as_none_not_zero() {
    let doc = parse(FIXTURE);
    let most_damage = doc.instance(1).unwrap();
    let objective = Objective::from_instance(most_damage).unwrap();
    assert_eq!(objective.name, "MostDamage");
    assert_eq!(objective.objective_type, None);
    assert_eq!(objective.target, None);
}

#[test]
fn tracks_and_weapon_sets_are_found_by_typedef_not_by_field_shape() {
    let doc = parse(FIXTURE);
    let tracks = tracks(&doc);
    assert_eq!(tracks.len(), 2);
    assert!(tracks.iter().any(|t| t.track_name == "bridge"));
    assert!(tracks.iter().any(|t| t.track_name == "park"));

    let sets = weapon_sets(&doc);
    assert_eq!(sets.len(), 1);
    assert_eq!(sets[0].name, "Rockets Only");
}

/// [`WeaponSet::allowed_weapons`] against the values `SP.xml`'s 20
/// `WeaponSetDefinition` instances carry; census in
/// `docs/formats/2048-campaign.md`'s "The weapon set gate", `WeaponType`'s
/// declaration in `docs/ghidra/functions/vita-2048-eu-v104/weapon-type-bits.md`.
#[test]
fn allowed_weapons_decodes_every_weapon_type_bit() {
    use crate::weapons::Weapon;

    let of = |bits: i64| WeaponSet {
        instance_id: 0,
        name: String::new(),
        available_bits: Some(bits),
    };

    // A single named bit each, as `"Rockets Only"`/`"Missile Only"`/... author them.
    assert_eq!(of(1).allowed_weapons(), vec![Weapon::Rocket]);
    assert_eq!(of(2).allowed_weapons(), vec![Weapon::Missile]);
    assert_eq!(of(1024).allowed_weapons(), vec![Weapon::LeachBeam]);

    // `"Cannons, Missile, Plasma"` = 162 = 128 + 32 + 2, in `WEAPON_BITS` order.
    assert_eq!(
        of(162).allowed_weapons(),
        vec![Weapon::Missile, Weapon::Cannon, Weapon::Plasma]
    );

    // `"Mines Only"` = 768 = bit 8 (`Bomb`) + bit 9 (`Mine`): `WeaponType`'s order,
    // not the HUD held-weapon id table's.
    assert_eq!(of(768).allowed_weapons(), vec![Weapon::Bomb, Weapon::Mine]);

    // `"DemoWeapons"` = 1023 sets bit 4 (`Shield`) alongside the rest: the case
    // that discriminates against the prior reading, which left bit 4 out.
    assert_eq!(
        of(1023).allowed_weapons(),
        vec![
            Weapon::Rocket,
            Weapon::Missile,
            Weapon::Quake,
            Weapon::Turbo,
            Weapon::Shield,
            Weapon::Cannon,
            Weapon::Autopilot,
            Weapon::Plasma,
            Weapon::Bomb,
            Weapon::Mine,
        ]
    );

    // `"EliminatorWeapons"` = 1959 sets every bit but Turbo (3), Shield (4) and
    // Autopilot (6): Shield stays out when its bit is clear.
    assert_eq!(
        of(1959).allowed_weapons(),
        vec![
            Weapon::Rocket,
            Weapon::Missile,
            Weapon::Quake,
            Weapon::Cannon,
            Weapon::Plasma,
            Weapon::Bomb,
            Weapon::Mine,
            Weapon::LeachBeam,
        ]
    );

    // No bits, and no `available_bits`, decode to nothing, not every weapon.
    assert_eq!(of(0).allowed_weapons(), Vec::<Weapon>::new());
    assert_eq!(
        WeaponSet {
            instance_id: 0,
            name: String::new(),
            available_bits: None,
        }
        .allowed_weapons(),
        Vec::<Weapon>::new()
    );
}

/// `RACE_A`/`RACE_B`/`ELIMINATION`/`ZONE` are `GameMode_SpeedLapRace`/
/// `GameMode_ArcadeRace`/`GameMode_EliminatorRace`/`GameMode_ZoneRace`'s typedef
/// ids: `oag_formats::wad::hash_name` of each class name matches exactly, the
/// convention the five in-file names corroborate. See `typedef::RACE_A`.
#[test]
fn typedef_ids_are_hash_name_of_the_class_they_are() {
    let hash = |name: &str| i64::from(oag_formats::wad::hash_name(name) as i32);

    assert_eq!(hash("GameMode_SpeedLapRace"), typedef::RACE_A);
    assert_eq!(hash("GameMode_ArcadeRace"), typedef::RACE_B);
    assert_eq!(hash("GameMode_EliminatorRace"), typedef::ELIMINATION);
    assert_eq!(hash("GameMode_ZoneRace"), typedef::ZONE);

    // The five in-file typedef names corroborate the same convention.
    assert_eq!(hash("GameModeObjective"), typedef::GAME_MODE_OBJECTIVE);
    assert_eq!(hash("GameModeBase"), typedef::GAME_MODE_BASE);
    assert_eq!(hash("WOShipModelData"), typedef::SHIP_MODEL_DATA);
    assert_eq!(hash("TrackDefinition"), typedef::TRACK_DEFINITION);
    assert_eq!(hash("WeaponSetDefinition"), typedef::WEAPON_SET_DEFINITION);

    // The other two `GameMode_*` names in `eboot.elf` match no `SP.xml` typedef:
    // shipped classes the campaign never instantiates.
    assert!(
        ![
            typedef::RACE_A,
            typedef::RACE_B,
            typedef::ELIMINATION,
            typedef::ZONE
        ]
        .contains(&hash("GameMode_CheckPointRace"))
    );
    assert!(
        ![
            typedef::RACE_A,
            typedef::RACE_B,
            typedef::ELIMINATION,
            typedef::ZONE
        ]
        .contains(&hash("GameMode_ZombieRace"))
    );
}
