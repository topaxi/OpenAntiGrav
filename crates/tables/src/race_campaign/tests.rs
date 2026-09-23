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
    assert_eq!(race.mode.ordinal(), Some(3));
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

/// **Changed 2026-09-14**: an unrecognised `mode=` used to be a hard parse
/// error (`Error::UnknownMode`). Wipeout HD's own `"NitroBattle"` and
/// `"Detonator"` forced the same "keep it raw" treatment [`Cell::class`]
/// already gets for `Zone`'s `"Zone"` - see the module docs' HD section and
/// [`Mode::Other`]. No shipped Pulse cell ever authors an unrecognised mode
/// (see `race_campaign_ground_truth.rs`'s own census), so this is a change
/// to the parser's behaviour on data that has never been observed, not a
/// change to what any real Pulse cell reads as.
#[test]
fn an_unknown_mode_is_kept_raw_rather_than_erroring() {
    let bad = FIXTURE.replace(r#"mode="Race""#, r#"mode="Nonsense""#);
    let grid = parse(&bad).expect("parses, with the mode kept raw");
    assert_eq!(grid.cells[0].mode, Mode::Other("Nonsense".to_string()));
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

// `Cell_EvaluateMedal` tests. The fixture's `Race` cell targets 1/2/3, its
// `Elimination` cell targets 10/7/5, its `Zone` cell targets 20/17/15 - see
// `FIXTURE` above. Every exact-tie boundary below (`value == target`) is
// covered directly, since `evaluate_medal`'s `<=`/`>=` settle a tie without
// needing a "chosen, not measured" label.

#[test]
fn a_race_position_hits_each_tier_at_its_exact_target() {
    let grid = parse(FIXTURE).expect("parses");
    let race = &grid.cells[0];
    assert_eq!(race.evaluate_medal(1), Some(Medal::Gold));
    assert_eq!(race.evaluate_medal(2), Some(Medal::Silver));
    assert_eq!(race.evaluate_medal(3), Some(Medal::Bronze));
    assert_eq!(race.evaluate_medal(4), None);
}

#[test]
fn an_elimination_kill_count_hits_each_tier_at_its_exact_target() {
    let grid = parse(FIXTURE).expect("parses");
    let elim = &grid.cells[2];
    assert_eq!(elim.evaluate_medal(10), Some(Medal::Gold));
    assert_eq!(elim.evaluate_medal(7), Some(Medal::Silver));
    assert_eq!(elim.evaluate_medal(5), Some(Medal::Bronze));
    assert_eq!(elim.evaluate_medal(4), None);
}

#[test]
fn a_zone_count_hits_each_tier_at_its_exact_target() {
    let grid = parse(FIXTURE).expect("parses");
    let zone = &grid.cells[3];
    assert_eq!(zone.evaluate_medal(20), Some(Medal::Gold));
    assert_eq!(zone.evaluate_medal(17), Some(Medal::Silver));
    assert_eq!(zone.evaluate_medal(15), Some(Medal::Bronze));
    assert_eq!(zone.evaluate_medal(14), None);
}

/// The guard: an `Elimination` value between the silver and bronze targets
/// only scores bronze because the comparison is flipped (`value >=
/// target`). Dropping the flip would compare `6 <= 10` instead and wrongly
/// award gold - this is the case that would pass a test written from the
/// same wrong "less is better" assumption everywhere else in this file.
#[test]
fn elimination_direction_is_flipped_not_the_default_less_is_better() {
    let grid = parse(FIXTURE).expect("parses");
    let elim = &grid.cells[2];
    assert_eq!(elim.evaluate_medal(6), Some(Medal::Bronze));
}

/// The same guard for `Zone`: 16 zones clears only the bronze floor
/// (`16 >= 15`, but `16 < 17`). Un-flipped, `16 <= 20` would wrongly read as
/// gold.
#[test]
fn zone_direction_is_flipped_not_the_default_less_is_better() {
    let grid = parse(FIXTURE).expect("parses");
    let zone = &grid.cells[3];
    assert_eq!(zone.evaluate_medal(16), Some(Medal::Bronze));
}

/// The mirror guard, for a mode the flip must **not** apply to: a `Race`
/// finishing position of 3 is bronze under the ordinary `<=` direction
/// (`3 <= 3`). If the flip were wrongly extended to `Race`, `3 >= 1` would
/// misread it as gold instead.
#[test]
fn race_direction_is_not_flipped() {
    let grid = parse(FIXTURE).expect("parses");
    let race = &grid.cells[0];
    assert_eq!(race.evaluate_medal(3), Some(Medal::Bronze));
}

#[test]
fn zero_and_the_unset_sentinel_are_no_result_regardless_of_mode() {
    let grid = parse(FIXTURE).expect("parses");
    let race = &grid.cells[0];
    let elim = &grid.cells[2];
    assert_eq!(race.evaluate_medal(0), None);
    assert_eq!(race.evaluate_medal(0xFFFF_FFFF), None);
    assert_eq!(elim.evaluate_medal(0), None);
    assert_eq!(elim.evaluate_medal(0xFFFF_FFFF), None);
}

/// `<= 0`, not only `== 0`: this reimplementation's own deliberately wider
/// guard - see [`Cell::evaluate_medal`]'s own doc for why a negative value
/// is treated the same as "unset" rather than reaching the comparison loop.
#[test]
fn a_negative_value_is_also_no_result() {
    let grid = parse(FIXTURE).expect("parses");
    let race = &grid.cells[0];
    assert_eq!(race.evaluate_medal(-1), None);
}

#[test]
fn medal_points_match_the_measured_table() {
    assert_eq!(Medal::Gold.points(), 3);
    assert_eq!(Medal::Silver.points(), 2);
    assert_eq!(Medal::Bronze.points(), 1);
}

#[test]
fn gold_is_the_smallest_ord_value() {
    assert!(Medal::Gold < Medal::Silver);
    assert!(Medal::Silver < Medal::Bronze);
}

/// A second, small grid - `gridW`, matching `FIXTURE`'s own `<Unlock
/// Grid="GridW"/>` row - so [`grid_points_met`] below has two real `Grid`s
/// to pick between by name rather than a single-grid fixture that could
/// pass by only ever finding `grids[0]`.
const SMALL_GRID_FIXTURE: &str = r#"
<PI_Grid name="gridW">
  <Values RequiredPoints="5" Locked="false"></Values>
  <PI_Cell name="gridW_1_1">
    <Values track="11_Track" mode="Race" class="Venom" Weapons="on" damage="on"
            AICount="7" skill="1.75" laps="3" ship="None" ShipChoice="Yes"></Values>
    <Gold Target="1"></Gold>
    <Silver Target="2"></Silver>
    <Bronze Target="3"></Bronze>
  </PI_Cell>
  <PI_Cell name="gridW_2_1">
    <Values track="12_Track" mode="Race" class="Venom" Weapons="on" damage="on"
            AICount="7" skill="1.75" laps="3" ship="None" ShipChoice="Yes"></Values>
    <Gold Target="1"></Gold>
    <Silver Target="2"></Silver>
    <Bronze Target="3"></Bronze>
  </PI_Cell>
</PI_Grid>
"#;

#[test]
fn points_earned_sums_medal_points_by_cell_name() {
    let grid = parse(SMALL_GRID_FIXTURE).expect("parses");
    let gold_first = |name: &str| (name == "gridW_1_1").then_some(Medal::Gold);
    assert_eq!(grid.points_earned(&gold_first), 3);
    let none = |_: &str| None;
    assert_eq!(grid.points_earned(&none), 0);
    let both_silver = |_: &str| Some(Medal::Silver);
    assert_eq!(grid.points_earned(&both_silver), 4);
}

/// `Unlock_GridPointsMet`'s own case-insensitive name match - `"GridW"`, the
/// mixed case an `<Unlock Grid="...">` row authors, against `gridW`'s own
/// lowercase `name` - and its threshold, against `SMALL_GRID_FIXTURE`'s
/// `RequiredPoints="5"` (max `6`, two Venom `Race` cells).
#[test]
fn grid_points_met_matches_the_named_grid_case_insensitively() {
    let grids = vec![
        parse(FIXTURE).expect("parses"),
        parse(SMALL_GRID_FIXTURE).expect("parses"),
    ];
    let gold_everything = |_: &str| Some(Medal::Gold);
    assert!(grid_points_met(&grids, "GridW", &gold_everything));
    assert!(grid_points_met(&grids, "gridw", &gold_everything));
    let none = |_: &str| None;
    assert!(!grid_points_met(&grids, "GridW", &none));
}

#[test]
fn grid_points_met_is_false_for_an_unknown_grid_name() {
    let grids = vec![parse(FIXTURE).expect("parses")];
    let gold_everything = |_: &str| Some(Medal::Gold);
    assert!(!grid_points_met(&grids, "GridQ", &gold_everything));
}

#[test]
fn grid_points_met_fails_below_the_threshold() {
    let grids = vec![parse(SMALL_GRID_FIXTURE).expect("parses")];
    // Only one of gridW's two cells medalled: 3 points < 5 required.
    let one_gold = |name: &str| (name == "gridW_1_1").then_some(Medal::Gold);
    assert!(!grid_points_met(&grids, "gridW", &one_gold));
}

/// A standalone fixture rather than an addition to `FIXTURE` above, since
/// every other test there indexes `grid.cells` positionally and a fifth cell
/// would shift them all. Mirrors `DATA00.PSARC`'s own shape for an
/// `Elimination` cell: per-difficulty `Easy`/`Medium`/`Hard` targets (dummy
/// `1`/`2`/`3` on every rung, per `grid8_3_2`'s own measured values) plus a
/// `NitroElimNovice`/`Skilled`/`Elite` triple.
const NITRO_FIXTURE: &str = r#"
<PI_Grid name="grid8">
  <Values RequiredPoints="0" Locked="false"></Values>
  <PI_Cell name="grid8_3_2">
    <Values track="88_Track" mode="Elimination" class="Flash" Weapons="on"
            damage="on" AICount="7" skillMedium="1.8" ship="None" ShipChoice="Yes"></Values>
    <EasyGold Target="1"></EasyGold>
    <EasySilver Target="2"></EasySilver>
    <EasyBronze Target="3"></EasyBronze>
    <MediumGold Target="1"></MediumGold>
    <MediumSilver Target="2"></MediumSilver>
    <MediumBronze Target="3"></MediumBronze>
    <HardGold Target="1"></HardGold>
    <HardSilver Target="2"></HardSilver>
    <HardBronze Target="3"></HardBronze>
    <NitroElimNovice Target="200"></NitroElimNovice>
    <NitroElimSkilled Target="200"></NitroElimSkilled>
    <NitroElimElite Target="200"></NitroElimElite>
  </PI_Cell>
</PI_Grid>
"#;

/// [`Cell::nitro_elimination_target_for_difficulty`] reads the triple by
/// rung, `0` novice through `2` elite - the measured shape, see the method's
/// own doc comment for what is and is not established about how the
/// executable turns this one number into a medal.
#[test]
fn nitro_elimination_target_reads_by_rung() {
    let grid = parse(NITRO_FIXTURE).expect("parses");
    let elim = &grid.cells[0];
    assert_eq!(elim.nitro_elimination_targets, Some((200, 200, 200)));
    assert_eq!(elim.nitro_elimination_target_for_difficulty(0), Some(200));
    assert_eq!(elim.nitro_elimination_target_for_difficulty(1), Some(200));
    assert_eq!(elim.nitro_elimination_target_for_difficulty(2), Some(200));
}

/// A cell with no `<NitroElimNovice>` at all - every cell in `FIXTURE` above
/// - reads `None`, not a panic or a default.
#[test]
fn nitro_elimination_target_is_none_without_the_triple() {
    let grid = parse(FIXTURE).expect("parses");
    assert_eq!(grid.cells[0].nitro_elimination_targets, None);
    assert_eq!(
        grid.cells[0].nitro_elimination_target_for_difficulty(1),
        None
    );
}

#[test]
fn mode_ordinals_match_the_executables_own_table() {
    assert_eq!(Mode::Race.ordinal(), Some(3));
    assert_eq!(Mode::Tournament.ordinal(), Some(4));
    assert_eq!(Mode::TimeTrial.ordinal(), Some(5));
    assert_eq!(Mode::Zone.ordinal(), Some(6));
    assert_eq!(Mode::Elimination.ordinal(), Some(8));
    assert_eq!(Mode::Head2Head.ordinal(), Some(9));
    assert_eq!(Mode::SpeedLap.ordinal(), Some(10));
    assert_eq!(Mode::CustomGrid.ordinal(), Some(11));
    assert_eq!(Mode::AiRace.ordinal(), Some(12));
    assert_eq!(Mode::from_name("time trial"), Some(Mode::TimeTrial));
    assert_eq!(Mode::from_name("nope"), None);
}
