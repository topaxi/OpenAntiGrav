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

/// Omega's prelit combination survives a grade: `Light::authored` clears the
/// flag and the bias, and a graded Omega rig would otherwise fall back to HD's.
#[test]
fn a_graded_omega_rig_keeps_its_prelit_combination() {
    let mut grade = grade();
    grade.request_stage(1);
    grade.commit();
    grade.set_weight(1.0);
    let omega = circuit_light().with_nova_prelit(2.5, 0.2, 2.0);
    let lit = grade.light(omega);
    assert_eq!(lit.nova, 1.0, "the flag survives");
    assert_eq!(lit.prelit_bias, 0.2, "and so does the bias");
    let hd = grade.light(circuit_light());
    assert_eq!(hd.nova, 0.0, "an HD rig stays HD's");
    assert_eq!(hd.prelit_bias, 0.0);
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

/// `Start`, `Sub Venom` and `Venom`, the `Track.Texture Colour` of each and
/// the title-wide UV scale: what [`ZoneGrade::zone_uniform`] needs to switch
/// on, with three stages so HD's ladder has a step to take past the opening
/// one.
const HD_THREE_STAGES: &str = concat!(
    "\"Texture U scale\"=1.000000\n",
    "\"Texture V scale\"=1.000000\n",
    "\"0 Start.Track.Texture Colour\"=9.000000 9.000000 9.000000\n",
    "\"0 Start.Lighting.Fog colour\"=0.000000 0.000000 0.000000 0.000000\n",
    "\"0 Start.Lighting.Fog density\"=0.000000\n",
    "\"1 Sub Venom.Track.Texture Colour\"=4.584567 6.537755 6.917541\n",
    "\"1 Sub Venom.Lighting.Fog colour\"=0.000000 1.305882 1.800000 0.000000\n",
    "\"1 Sub Venom.Lighting.Fog density\"=0.002100\n",
    "\"2 Venom.Track.Texture Colour\"=1.000000 2.000000 3.000000\n",
    "\"2 Venom.Lighting.Fog colour\"=1.000000 0.000000 0.000000 0.000000\n",
    "\"2 Venom.Lighting.Fog density\"=0.004200\n",
);

/// The fixed timestep a race runs at.
const DT: f32 = 1.0 / 60.0;

/// An HD-shaped grade: HD's ladder, HD's transition law and a decoded stage
/// texture in every slot so the Zone term is switched on.
fn hd_grade() -> ZoneGrade {
    let table = EffectSettings::parse(HD_THREE_STAGES).expect("it parses");
    let texel = || {
        Some(Arc::new(ModelTexture {
            label: "stage".into(),
            width: 1,
            height: 1,
            texels: oag_render::mesh::Texels::Rgba8(vec![255, 255, 255, 255]),
            mip_count: None,
        }))
    };
    ZoneGrade::new(
        "zonemode.effectsettings".to_string(),
        table,
        Some(oag_hd::race::ZONE_STAGES),
        vec![texel(), texel(), texel()],
        vec![texel(), texel(), texel()],
    )
    .expect("it names stages")
    .with_transition(Some(oag_hd::race::ZONE_TRANSITION))
}

/// The radius law, against the closed form the two live reads matched:
/// `r_k = 0.1 + 0.5k + 0.05k(k - 1)`, capped at `20000`.
#[test]
fn the_radius_law_matches_its_closed_form_and_caps() {
    let law = oag_hd::race::ZONE_TRANSITION;
    for (k, want) in [
        (0, 0.1),
        (1, 0.6),
        (2, 1.2),
        (10, 9.6),
        (53, 164.4),
        (252, 3288.7),
    ] {
        let got = law.radius_after(k);
        assert!(
            (got - want).abs() < 0.05,
            "k = {k}: radius {got}, want {want}"
        );
    }
    // Frame 628 is the first past the ceiling on the shipped numbers - the
    // iterated original parks at `20001.86` there; the closed form caps and
    // stays.
    assert!((law.radius_after(627) - 19_938.7).abs() < 0.5);
    assert_eq!(law.radius_after(628), 20_000.0);
    assert_eq!(law.radius_after(100_000), 20_000.0);
}

/// The colour weight: `0.01` a frame from zero, clamped at `1.0` on frame
/// 100 - `0.53` at `k = 53`, as read live beside the radius.
#[test]
fn the_weight_ramps_a_hundredth_a_frame_to_one() {
    let law = oag_hd::race::ZONE_TRANSITION;
    assert_eq!(law.weight_after(0), 0.0);
    assert!((law.weight_after(53) - 0.53).abs() < 1e-6);
    assert!((law.weight_after(99) - 0.99).abs() < 1e-6);
    assert_eq!(law.weight_after(100), 1.0);
    assert_eq!(law.weight_after(250), 1.0);
}

/// A race on HD's ladder: the opening stage is shown whole, the first step
/// starts the sphere at the craft, and the frames since are derived from the
/// zone counter and the zone clock rather than counted.
#[test]
fn a_stage_step_starts_the_sphere_and_the_zone_clock_drives_it() {
    let mut grade = hd_grade();
    let craft = [10.0, 20.0, 30.0];

    // Zone 0 opens on Sub Venom, whole: no sphere in flight, the outer stage
    // is the showing one, the radius parked at the cap.
    assert!(grade.show_zone(0));
    grade.follow(0, 3.0, DT, craft);
    let opening = grade.wavefront();
    assert_eq!((grade.blend().current, opening.previous), (1, 1));
    assert_eq!(opening.radius, 20_000.0);
    assert_eq!(grade.blend().weight, 1.0);
    assert_eq!(opening.origin, craft, "centred on the craft even at rest");

    // Zone 2 is Venom. On the tick it steps the sphere is at its first
    // radius, the old stage is outside it, and the palette is all the old
    // stage's.
    assert!(grade.show_zone(2));
    grade.follow(2, 0.0, DT, craft);
    let step = grade.wavefront();
    assert_eq!((grade.blend().current, step.previous), (2, 1));
    assert_eq!(step.frames, 0);
    assert!((step.radius - 0.1).abs() < 1e-6);
    assert_eq!(grade.blend().weight, 0.0);
    assert_eq!(
        grade.palette().fog_colour,
        Some([0.0, 1.305_882, 1.8]),
        "at weight zero the fog is still Sub Venom's"
    );

    // 53 ticks into the zone: the live read's own frame.
    grade.follow(2, 53.0 * DT, DT, craft);
    let live = grade.wavefront();
    assert_eq!(live.frames, 53);
    assert!((live.radius - 164.4).abs() < 0.05);
    assert!((grade.blend().weight - 0.53).abs() < 1e-6);

    // The same inputs again read the same - a paused race stops the clock and
    // the sphere with it, with no catch-up.
    grade.follow(2, 53.0 * DT, DT, craft);
    assert_eq!(grade.wavefront(), live);

    // Zone 3 is Sub Flash on the ladder but this file ends at Venom, so the
    // stage clamps and does not step - and the sphere keeps growing from the
    // zone-2 crossing rather than restarting: 600 ticks in.
    assert!(!grade.show_zone(3));
    grade.follow(3, 0.0, DT, craft);
    let clamped = grade.wavefront();
    assert_eq!(clamped.frames, 600);
    assert!((clamped.radius - 18_270.1).abs() < 0.5);
    assert_eq!(grade.blend().weight, 1.0);

    // And well past the ceiling it parks there.
    grade.follow(4, 5.0, DT, craft);
    assert_eq!(grade.wavefront().radius, 20_000.0);
}

/// The uniform carries both stages and the sphere: the showing stage as the
/// Inner pair, the one being swept out as the Outer, the craft as the origin
/// and the law's radius.
#[test]
fn the_uniform_binds_the_inner_and_outer_pairs_around_the_sphere() {
    let mut grade = hd_grade();
    grade.show_zone(0);
    grade.follow(0, 0.0, DT, [0.0; 3]);
    let settled = grade.zone_uniform();
    assert_eq!(settled.enabled, 1.0);
    assert_eq!(settled.track, settled.track_outer, "nothing in flight");
    assert_eq!(settled.scene, settled.scene_outer);

    grade.show_zone(2);
    grade.follow(2, 10.0 * DT, DT, [1.0, 2.0, 3.0]);
    let sweeping = grade.zone_uniform();
    assert_eq!(sweeping.track.effect[..3], [1.0, 2.0, 3.0], "Venom inside");
    assert_eq!(
        sweeping.track_outer.effect[..3],
        [4.584_567, 6.537_755, 6.917_541],
        "Sub Venom outside"
    );
    assert_eq!(sweeping.origin, [1.0, 2.0, 3.0, 1.0]);
    assert!((sweeping.radius - 9.6).abs() < 1e-4);
}

/// A title with no read transition - 2048 - steps whole: the weight rests at
/// `1.0`, the Outer pair is the Inner and there is no sphere to speak of.
#[test]
fn a_title_with_no_transition_law_steps_whole() {
    let mut grade = grade_with(Some(oag_2048::race::ZONE_STAGES));
    grade.show_zone(0);
    // Zone 2 is 2048's stage 2, clamped to this excerpt's stage 1 - so drive
    // a step with the request/commit pair and watch `follow` leave it whole.
    grade.request_stage(0);
    grade.commit();
    grade.follow(9, 1.0, DT, [5.0; 3]);
    let front = grade.wavefront();
    assert_eq!(front.previous, grade.blend().current);
    assert_eq!(front.radius, 0.0);
    assert_eq!(front.origin, [5.0; 3]);
}

/// The request/commit pair starts the sphere the way the traced commit does,
/// and `show_whole` settles it the way the loader needs for an opening stage.
#[test]
fn a_commit_starts_the_sphere_and_show_whole_settles_it() {
    let mut grade = hd_grade();
    grade.request_stage(1);
    assert!(grade.commit());
    let started = grade.wavefront();
    assert_eq!((started.previous, started.frames), (0, 0));
    assert!((started.radius - 0.1).abs() < 1e-6);
    assert_eq!(grade.blend().weight, 0.0);

    grade.show_whole();
    let whole = grade.wavefront();
    assert_eq!(whole.previous, 1);
    assert_eq!(whole.radius, 20_000.0);
    assert_eq!(grade.blend().weight, 1.0);

    // A pinned stage is shown whole too, and `follow` leaves it so.
    grade.pin_stage(0);
    grade.follow(40, 2.0, DT, [0.0; 3]);
    assert_eq!(grade.wavefront().previous, 0);
    assert_eq!(grade.blend().weight, 1.0);
}
