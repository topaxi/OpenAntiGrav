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
    assert_eq!(race.skill_for_difficulty(Difficulty::Easy), Some(1.1));
    assert_eq!(race.skill_for_difficulty(Difficulty::Medium), Some(1.75));
    assert_eq!(race.skill_for_difficulty(Difficulty::Hard), Some(2.5));
}

/// A `Tournament` cell names no `track` (its legs are `<TournamentTrack>`) and
/// no `skillEasy`/`skillHard`, which the original's parser defaults from `skill`.
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
    assert_eq!(
        tournament.skill_for_difficulty(Difficulty::Easy),
        Some(1.8 - 1.0)
    );
    assert_eq!(
        tournament.skill_for_difficulty(Difficulty::Hard),
        Some(1.8 + 1.0)
    );
}

/// `Elimination` authors no `laps` attribute: absent, not `"0"`.
#[test]
fn elimination_has_no_laps_attribute() {
    let grid = parse(FIXTURE).expect("parses");
    let elim = &grid.cells[2];
    assert_eq!(elim.mode, Mode::Elimination);
    assert_eq!(elim.laps, None);
    assert_eq!((elim.gold, elim.silver, elim.bronze), (10, 7, 5));
}

/// `Zone`'s `class="Zone"` is real text, not a speed class: `speed_class()`
/// reads `None` rather than erroring.
#[test]
fn zone_class_is_not_a_speed_class() {
    let grid = parse(FIXTURE).expect("parses");
    let zone = &grid.cells[3];
    assert_eq!(zone.class, "Zone");
    assert_eq!(zone.speed_class(), None);
    assert_eq!(zone.laps, Some(0));
    assert_eq!(zone.ai_count, None);
    assert_eq!(zone.skill, None);
    assert_eq!(zone.skill_for_difficulty(Difficulty::Easy), None);
}

/// **Changed 2026-09-14**: an unrecognised `mode=` used to be a hard parse error
/// (`Error::UnknownMode`). HD's `"NitroBattle"` and `"Detonator"` forced the
/// same keep-it-raw treatment [`Cell::class`] gives `"Zone"`; see [`Mode::Other`]
/// and the module docs' HD section. No shipped Pulse cell authors one
/// (`race_campaign_ground_truth.rs`'s census), so no real Pulse read changes.
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

// `Cell_EvaluateMedal` tests. Fixture targets: `Race` 1/2/3, `Elimination`
// 10/7/5, `Zone` 20/17/15 (see `FIXTURE`). Every exact-tie boundary
// (`value == target`) is covered.

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
/// scores bronze only because the comparison is flipped (`value >= target`);
/// without it `6 <= 10` would wrongly award gold.
#[test]
fn elimination_direction_is_flipped_not_the_default_less_is_better() {
    let grid = parse(FIXTURE).expect("parses");
    let elim = &grid.cells[2];
    assert_eq!(elim.evaluate_medal(6), Some(Medal::Bronze));
}

/// The same guard for `Zone`: 16 zones clears only bronze (`16 >= 15`, `16 <
/// 17`); un-flipped, `16 <= 20` would read as gold.
#[test]
fn zone_direction_is_flipped_not_the_default_less_is_better() {
    let grid = parse(FIXTURE).expect("parses");
    let zone = &grid.cells[3];
    assert_eq!(zone.evaluate_medal(16), Some(Medal::Bronze));
}

/// The mirror guard: a `Race` position of 3 is bronze under `<=` (`3 <= 3`);
/// the flip wrongly extended to `Race` would read `3 >= 1` as gold.
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

/// `<= 0`, not only `== 0`: the deliberately wider guard (see
/// [`Cell::evaluate_medal`]) treating a negative as "unset".
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

/// A second small grid (`gridW`, matching `FIXTURE`'s `<Unlock Grid="GridW"/>`)
/// so [`grid_points_met`] must pick between two grids by name, not pass by
/// finding `grids[0]`.
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

/// `Unlock_GridPointsMet`'s case-insensitive name match (`"GridW"` against
/// `gridW`) and its threshold, against `SMALL_GRID_FIXTURE`'s
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

/// A standalone fixture, since other tests index `grid.cells` positionally.
/// Mirrors `DATA00.PSARC`'s `Elimination` cell: per-difficulty targets (dummy
/// `1`/`2`/`3` on every rung, per `grid8_3_2`) plus a
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

/// [`Cell::nitro_elimination_target_for_difficulty`] reads the triple by rung,
/// `0` novice through `2` elite; see the method for what is not established
/// about turning it into a medal.
#[test]
fn nitro_elimination_target_reads_by_rung() {
    let grid = parse(NITRO_FIXTURE).expect("parses");
    let elim = &grid.cells[0];
    assert_eq!(elim.nitro_elimination_targets, Some((200, 200, 200)));
    assert_eq!(
        elim.nitro_elimination_target_for_difficulty(Difficulty::Easy),
        Some(200)
    );
    assert_eq!(
        elim.nitro_elimination_target_for_difficulty(Difficulty::Medium),
        Some(200)
    );
    assert_eq!(
        elim.nitro_elimination_target_for_difficulty(Difficulty::Hard),
        Some(200)
    );
}

/// A cell with no `<NitroElimNovice>` (every `FIXTURE` cell) reads `None`.
#[test]
fn nitro_elimination_target_is_none_without_the_triple() {
    let grid = parse(FIXTURE).expect("parses");
    assert_eq!(grid.cells[0].nitro_elimination_targets, None);
    assert_eq!(
        grid.cells[0].nitro_elimination_target_for_difficulty(Difficulty::Medium),
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
