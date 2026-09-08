//! Unit tests against invented fixtures, per `docs/architecture/adr/0006-no-copyrighted-content.md`.
//! Shipped values are only ever checked in `crates/formats/tests/race_campaign_ground_truth.rs`.

use super::*;

const FIXTURE: &str = r#"
<PI_Grid name="gridX">
  <Values RequiredPoints="9" Locked="true"></Values>
  <Unlock Grid="GridW"></Unlock>
  <PI_Cell name="gridX_1_1">
    <Values track="99_Track" mode="Race" class="Venom" Weapons="on" damage="on"
            AICount="7" skillEasy="1.1" skill="1.75" skillHard="2.5" laps="3"
            ship="None" ShipChoice="Yes"></Values>
    <Gold Target="1"></Gold>
    <Silver Target="2"></Silver>
    <Bronze Target="3"></Bronze>
  </PI_Cell>
  <PI_Cell name="gridX_2_1">
    <Values mode="Tournament" class="Flash" Weapons="on" damage="on"
            AICount="7" skill="1.8" laps="4" ship="None" ShipChoice="Yes"></Values>
    <TournamentTrack track="99_Track"></TournamentTrack>
    <TournamentTrack track="88_Track"></TournamentTrack>
    <Gold Target="1"></Gold>
    <Silver Target="2"></Silver>
    <Bronze Target="3"></Bronze>
  </PI_Cell>
  <PI_Cell name="gridX_3_1">
    <Values track="88_Track" mode="Elimination" class="Flash" Weapons="on"
            damage="on" AICount="7" skill="1.8" ship="None" ShipChoice="Yes"></Values>
    <Gold Target="10"></Gold>
    <Silver Target="7"></Silver>
    <Bronze Target="5"></Bronze>
  </PI_Cell>
  <PI_Cell name="gridX_4_1">
    <Values track="77_Track" mode="Zone" class="Zone" Weapons="off" damage="on"
            laps="0" ship="None" ShipChoice="Yes"></Values>
    <Gold Target="20"></Gold>
    <Silver Target="17"></Silver>
    <Bronze Target="15"></Bronze>
  </PI_Cell>
</PI_Grid>
"#;

#[test]
fn parses_a_grid_and_its_cells() {
    let grid = parse(FIXTURE).expect("parses");
    assert_eq!(grid.name, "gridX");
    assert_eq!(grid.required_points, 9);
    assert!(grid.locked);
    assert_eq!(grid.unlock_grid.as_deref(), Some("GridW"));
    assert_eq!(grid.group, None);
    assert_eq!(grid.cells.len(), 4);
    assert_eq!(grid.max_points(), 12);
}

#[test]
fn a_race_cell_carries_track_class_and_skill() {
    let grid = parse(FIXTURE).expect("parses");
    let race = &grid.cells[0];
    assert_eq!(race.track.as_deref(), Some("99_Track"));
    assert_eq!(race.mode, Mode::Race);
    assert_eq!(race.mode.ordinal(), 3);
    assert_eq!(race.class, "Venom");
    assert_eq!(race.speed_class(), Some(SpeedClass::Venom));
    assert!(race.weapons);
    assert!(race.damage);
    assert_eq!(race.ai_count, Some(7));
    assert_eq!(race.laps, Some(3));
    assert_eq!((race.gold, race.silver, race.bronze), (1, 2, 3));
    assert_eq!(race.grid_coords(), Some((1, 1)));
    assert_eq!(race.skill_for_difficulty(0), Some(1.1));
    assert_eq!(race.skill_for_difficulty(1), Some(1.75));
    assert_eq!(race.skill_for_difficulty(2), Some(2.5));
}

/// A `Tournament` cell names no `track` at all - its legs come from
/// `<TournamentTrack>` - and its `skillEasy`/`skillHard` are absent, which the
/// original's own parser is documented to default from `skill`.
#[test]
fn a_tournament_cell_has_no_track_and_defaults_its_skill_spread() {
    let grid = parse(FIXTURE).expect("parses");
    let tournament = &grid.cells[1];
    assert_eq!(tournament.track, None);
    assert_eq!(tournament.mode, Mode::Tournament);
    assert_eq!(
        tournament.tournament_tracks,
        vec!["99_Track".to_string(), "88_Track".to_string()]
    );
    assert_eq!(tournament.skill, Some(1.8));
    assert_eq!(tournament.skill_easy, None);
    // The documented default: skillEasy = skill - 1.0, skillHard = skill + 1.0.
    assert_eq!(tournament.skill_for_difficulty(0), Some(1.8 - 1.0));
    assert_eq!(tournament.skill_for_difficulty(2), Some(1.8 + 1.0));
}

/// `Elimination` authors no `laps` attribute whatsoever - not `"0"`, absent.
#[test]
fn elimination_has_no_laps_attribute() {
    let grid = parse(FIXTURE).expect("parses");
    let elim = &grid.cells[2];
    assert_eq!(elim.mode, Mode::Elimination);
    assert_eq!(elim.laps, None);
    assert_eq!((elim.gold, elim.silver, elim.bronze), (10, 7, 5));
}

/// `Zone`'s `class="Zone"` is real text and is not one of the four speed
/// classes - `speed_class()` reads it as `None` rather than erroring.
#[test]
fn zone_class_is_not_a_speed_class() {
    let grid = parse(FIXTURE).expect("parses");
    let zone = &grid.cells[3];
    assert_eq!(zone.class, "Zone");
    assert_eq!(zone.speed_class(), None);
    assert_eq!(zone.laps, Some(0));
    assert_eq!(zone.ai_count, None);
    assert_eq!(zone.skill, None);
    assert_eq!(zone.skill_for_difficulty(0), None);
}

#[test]
fn an_unknown_mode_is_a_hard_error_not_a_default() {
    let bad = FIXTURE.replace(r#"mode="Race""#, r#"mode="Nonsense""#);
    let err = parse(&bad).unwrap_err();
    assert_eq!(
        err,
        Error::UnknownMode {
            name: "Nonsense".to_string()
        }
    );
}

#[test]
fn a_missing_required_attribute_is_an_error_not_a_zero() {
    let bad = FIXTURE.replace(r#"RequiredPoints="9""#, "");
    let err = parse(&bad).unwrap_err();
    assert_eq!(
        err,
        Error::MissingAttribute {
            element: "Values",
            attribute: "RequiredPoints"
        }
    );
}

#[test]
fn a_malformed_boolean_is_an_error_not_a_silent_false() {
    let bad = FIXTURE.replace(r#"Weapons="on""#, r#"Weapons="maybe""#);
    let err = parse(&bad).unwrap_err();
    assert!(matches!(
        err,
        Error::NotANumber {
            attribute: "Weapons",
            ..
        }
    ));
}

#[test]
fn definition_xml_lists_its_load_xml_entries() {
    let expanded = r#"
<Screen name="Top">
  <LoadXML><Values Src="Data\Plugins\grids\grid_00.xml"></Values></LoadXML>
  <LoadXML><Values Src="Data\Plugins\grids\grid_01.xml"></Values></LoadXML>
</Screen>
"#;
    assert_eq!(
        definition_entries(expanded),
        vec![
            r"Data\Plugins\grids\grid_00.xml".to_string(),
            r"Data\Plugins\grids\grid_01.xml".to_string(),
        ]
    );
}

#[test]
fn mode_ordinals_match_the_executables_own_table() {
    assert_eq!(Mode::Race.ordinal(), 3);
    assert_eq!(Mode::Tournament.ordinal(), 4);
    assert_eq!(Mode::TimeTrial.ordinal(), 5);
    assert_eq!(Mode::Zone.ordinal(), 6);
    assert_eq!(Mode::Elimination.ordinal(), 8);
    assert_eq!(Mode::Head2Head.ordinal(), 9);
    assert_eq!(Mode::SpeedLap.ordinal(), 10);
    assert_eq!(Mode::CustomGrid.ordinal(), 11);
    assert_eq!(Mode::AiRace.ordinal(), 12);
    assert_eq!(Mode::from_name("time trial"), Some(Mode::TimeTrial));
    assert_eq!(Mode::from_name("nope"), None);
}
