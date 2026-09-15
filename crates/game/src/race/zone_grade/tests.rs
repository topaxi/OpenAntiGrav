//! Unit tests for [`super`], on two whole stages of HD's own file.

use super::*;

/// `Start` and `Sub Venom`, verbatim from
/// `/data/environments/zonemode.effectsettings` - the same excerpt
/// `oag_tables::effectsettings`'s own tests blend, and the same numbers
/// `effectsettings_ground_truth.rs` asserts straight off the disc image.
const HD_TWO_STAGES: &str = concat!(
    "\"0 Start.Lighting.Sun colour\"=1.000000 1.000000 1.000000 0.000000\n",
    "\"0 Start.Lighting.Constant Ambient Colour\"=1.000000 1.000000 1.000000 0.000000\n",
    "\"0 Start.Lighting.Fog colour\"=0.000000 0.000000 0.000000 0.000000\n",
    "\"0 Start.Lighting.Fog density\"=0.000000\n",
    "\"1 Sub Venom.Lighting.Sun colour\"=0.000000 0.000000 0.000000 0.000000\n",
    "\"1 Sub Venom.Lighting.Constant Ambient Colour\"=1.500000 1.500000 1.500000 0.000000\n",
    "\"1 Sub Venom.Lighting.Fog colour\"=0.000000 1.305882 1.800000 0.000000\n",
    "\"1 Sub Venom.Lighting.Fog density\"=0.002100\n",
);

fn grade() -> ZoneGrade {
    grade_with(None)
}

fn grade_with(stages: Option<&'static oag_title::ZoneStages>) -> ZoneGrade {
    let table = EffectSettings::parse(HD_TWO_STAGES).expect("it parses");
    ZoneGrade::new(
        "zonemode.effectsettings".to_string(),
        table,
        stages,
        Vec::new(),
        Vec::new(),
    )
    .expect("it names stages")
}

/// The circuit's own rig, as `envsettings_light` would hand one over.
fn circuit_light() -> mesh_render::Light {
    mesh_render::Light::authored(
        [0.0, 1.0, 0.0],
        [0.5, 0.5, 0.5],
        [0.2, 0.2, 0.2],
        [1.0; 3],
        [1.0; 3],
        0.25,
    )
}

/// The circuit's own distance fog, as `envsettings_fog` would hand one over.
fn circuit_fog() -> mesh_render::Fog {
    mesh_render::Fog::authored_exp2([0.1, 0.2, 0.3], 0.004)
}

#[test]
fn a_fresh_grade_rests_on_stage_zero_fully_applied() {
    let grade = grade();
    assert_eq!(grade.blend().current, 0);
    assert_eq!(grade.blend().requested, 0);
    assert_eq!(grade.blend().weight, 1.0);
    assert_eq!(grade.last_stage(), 1);
}

/// A table naming no stage is not a grade at all - there would be nothing to
/// select between.
#[test]
fn a_table_with_no_stages_builds_no_grade() {
    let table = EffectSettings::parse("\"Texture U scale\"=1.000000\n").expect("it parses");
    assert!(ZoneGrade::new("empty".to_string(), table, None, Vec::new(), Vec::new()).is_none());
}

/// **Stage 0 and stage 1 are two different, correctly-sourced fogs**: `Start`
/// authors none and leaves the circuit's own standing, `Sub Venom` authors the
/// cyan the file states, at the density the file states.
#[test]
fn stage_zero_and_stage_one_produce_different_fog() {
    let mut grade = grade();
    let base = Some(circuit_fog());

    let start = grade.fog(base).expect("stage 0 leaves the circuit's fog");
    assert_eq!(
        start.colour,
        circuit_fog().colour,
        "Start authors density 0, so the circuit's own fog stands"
    );

    grade.request_stage(1);
    assert!(grade.commit(), "the request differs, so it commits");
    // The commit zeroes the weight, so the stage is asked for but not yet
    // showing - it reads as `Start` until something raises the weight.
    assert_eq!(grade.blend().weight, 0.0);
    assert_eq!(
        grade.fog(base).expect("still the circuit's fog").colour,
        circuit_fog().colour
    );

    grade.set_weight(1.0);
    let sub_venom = grade.fog(base).expect("stage 1 authors its own fog");
    assert_eq!(sub_venom.colour, [0.0, 1.305_882, 1.8]);
    assert_eq!(sub_venom.density, 0.002_1);
    assert_eq!(
        sub_venom.curve, 1.0,
        "the same exp2 curve .envsettings drives"
    );
    assert_ne!(sub_venom.colour, start.colour);
}

/// Halfway between the two stages, the fog is halfway between the two authored
/// colours - the cross-fade reaching the renderer, not just the table.
#[test]
fn a_half_committed_stage_fogs_halfway_between_the_two() {
    let mut grade = grade();
    grade.request_stage(1);
    grade.commit();
    grade.set_weight(0.5);
    let fog = grade
        .fog(Some(circuit_fog()))
        .expect("density is over zero");
    assert_eq!(fog.colour, [0.0, 1.305_882 / 2.0, 0.9]);
    assert_eq!(fog.density, 0.002_1 / 2.0);
}

/// The stage tints the circuit's rig; it never aims one. There is no `Sun
/// direction` key in this schema, so the direction and the specular weight are
/// the circuit's own on every stage.
#[test]
fn a_stage_recolours_the_rig_and_keeps_its_direction() {
    let mut grade = grade();
    grade.request_stage(1);
    grade.commit();
    grade.set_weight(1.0);
    let base = circuit_light();
    let lit = grade.light(base);
    assert_eq!(lit.direction, base.direction, "the circuit aims the sun");
    assert_eq!(lit.specular_scale, base.specular_scale);
    assert_eq!(lit.sun, [0.0, 0.0, 0.0], "Sub Venom authors a sunless rig");
    assert_eq!(lit.ambient, [1.5, 1.5, 1.5]);
    assert_eq!(lit.enabled, 1.0);
}

/// A circuit with no authored rig is handed back untouched: a tint needs a
/// direction to tint, and this file states none.
#[test]
fn a_stand_in_rig_is_left_alone() {
    let mut grade = grade();
    grade.request_stage(1);
    grade.commit();
    grade.set_weight(1.0);
    let stand_in = mesh_render::Light::stand_in();
    let lit = grade.light(stand_in);
    assert_eq!(lit.enabled, 0.0, "still the stand-in rig");
    assert_eq!(lit.sun, stand_in.sun);
    assert_eq!(lit.ambient, stand_in.ambient);
    assert_eq!(lit.direction, stand_in.direction);
}

/// A request past the end of the ladder clamps to it, the way 2048's own
/// `Zone_UpdateStage` clamps to `0xc` against its thirteen-stage table.
#[test]
fn a_request_past_the_ladder_clamps_to_it() {
    let mut grade = grade();
    grade.request_stage(99);
    assert_eq!(grade.blend().requested, 1);
    assert!(grade.commit());
    assert_eq!(grade.blend().current, 1);
}

/// Committing the stage already showing is a no-op - the gate the traced code
/// draws its transition effect behind.
#[test]
fn committing_the_current_stage_changes_nothing() {
    let mut grade = grade();
    grade.set_weight(0.75);
    assert!(!grade.commit());
    assert_eq!(grade.blend().weight, 0.75, "the weight is left alone");
}

/// A title with no recovered ladder does not move, which is HD/Fury: its own
/// Zone stage source (`craftArray[n]->+0x640`) has no found writer, so the
/// grade must sit still rather than borrow 2048's numbers.
#[test]
fn a_title_with_no_ladder_never_advances() {
    let mut grade = grade();
    assert_eq!(grade.stage_for_zone(40), None);
    assert!(!grade.show_zone(40));
    assert_eq!(grade.blend().current, 0);
}

/// 2048's own table, against this two-stage excerpt: the clamp is the loaded
/// file's last row, exactly as `Zone_UpdateStage` clamps to its own.
#[test]
fn a_recovered_ladder_advances_and_clamps_to_the_loaded_table() {
    let mut grade = grade_with(Some(oag_2048::race::ZONE_STAGES));
    // Zone 0 matches the last record (threshold `0`) and is class 1, so a race
    // opens on stage 1 rather than on `Start`.
    assert_eq!(grade.stage_for_zone(0), Some(1));
    assert!(grade.show_zone(0));
    assert_eq!(grade.blend().current, 1);
    assert_eq!(grade.blend().weight, 1.0);
    // Same class, same stage, no change.
    assert!(!grade.show_zone(1));
    // A far higher class still clamps to the two-stage excerpt's own last row.
    assert!(!grade.show_zone(90));
    assert_eq!(grade.blend().current, 1);
}

/// Without `--zone-stage`, a recovered ladder still steps on a zone-counter
/// advance - the regression `pin_stage` must not cause. Same table and same
/// zone as `a_recovered_ladder_advances_and_clamps_to_the_loaded_table`
/// above, stated again here so this test stands on its own as the "unpinned"
/// half of the pin/no-pin pair below.
#[test]
fn an_unpinned_grade_still_steps_on_a_zone_advance() {
    let mut grade = grade_with(Some(oag_2048::race::ZONE_STAGES));
    assert!(grade.show_zone(0));
    assert_eq!(grade.blend().current, 1);
}

/// The `--zone-stage` override: pinning holds the grade on the requested
/// stage even though a later `show_zone` call carries a zone number the
/// title's own ladder would otherwise step off of - the bug this build fixes,
/// where the ladder overwrote the override from the very first frame.
#[test]
fn a_pinned_stage_survives_a_zone_counter_advance() {
    let mut grade = grade_with(Some(oag_2048::race::ZONE_STAGES));
    grade.pin_stage(0);
    assert_eq!(grade.blend().current, 0);
    assert_eq!(grade.blend().weight, 1.0);
    // Zone 0 maps to stage 1 on this ladder (see the test above) - if the pin
    // did not hold, this call would move the grade off stage 0.
    assert!(!grade.show_zone(0));
    assert_eq!(
        grade.blend().current,
        0,
        "the pin held, the ladder did not move it"
    );
    // A far later zone, deep into the ladder, does not move it either.
    assert!(!grade.show_zone(90));
    assert_eq!(grade.blend().current, 0);
}

/// `pin_stage` clamps the same way [`ZoneGrade::request_stage`] does - a
/// request past the loaded file's last row still lands on a stage the file
/// actually names.
#[test]
fn pin_stage_clamps_to_the_loaded_table() {
    let mut grade = grade();
    grade.pin_stage(99);
    assert_eq!(
        grade.blend().current,
        1,
        "clamped to this excerpt's last stage"
    );
    assert!(!grade.show_zone(0), "still pinned after the clamp");
    assert_eq!(grade.blend().current, 1);
}
