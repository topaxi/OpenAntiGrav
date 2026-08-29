//! Unit tests for [`super`], on the two stage-prefix spellings the disc uses.

use super::*;

/// HD's own first stage, `zonemode.effectsettings`'s opening lines.
const HD_EXCERPT: &str = concat!(
    "\"Texture U scale\"=1.000000\n",
    "\"0 Start.Lighting.Sun colour\"=1.000000 1.000000 1.000000 0.000000\n",
    "\"0 Start.Lighting.Constant Ambient Colour\"=1.000000 1.000000 1.000000 0.000000\n",
    "\"0 Start.Sky horizon colour\"=0.000000 0.000000 0.000000\n",
    "\"1 Sub Venom.Sky horizon colour\"=0.000000 0.211765 0.247059\n",
);

/// 2048's own first stage, `ZoneMode2048.effectSettings`'s opening lines.
const TWOK48_EXCERPT: &str = concat!(
    "\"Sky Radius\"=4000.000000\n",
    "\"Lighting.Sun direction\"=-0.802630 0.469582 0.367802\n",
    "\"Zone 0 Start.Colour 1.Colour\"=1.000000 1.000000 1.000000\n",
    "\"Zone 0 Start.Edge Colour\"=1.098408 0.000000 0.000000\n",
    "\"Zone 12 Supersonic.Track Paint.Primary Colour\"=0.345098 0.000000 0.000000\n",
);

#[test]
fn hd_style_stages_are_indexed_without_a_zone_prefix() {
    let e = EffectSettings::parse(HD_EXCERPT).expect("it parses");
    assert_eq!(e.stages.len(), 2);
    assert_eq!(e.stages[&0].name, "Start");
    assert_eq!(e.stages[&1].name, "Sub Venom");
}

#[test]
fn twok48_style_stages_strip_the_leading_zone_word() {
    let e = EffectSettings::parse(TWOK48_EXCERPT).expect("it parses");
    assert_eq!(e.stages.len(), 2);
    assert_eq!(e.stages[&0].name, "Start");
    assert_eq!(e.stages[&12].name, "Supersonic");
}

#[test]
fn a_four_number_colour_reads_as_vec4_not_vec3_or_rgba8() {
    let e = EffectSettings::parse(HD_EXCERPT).expect("it parses");
    assert_eq!(
        e.stage_vec4(0, "Lighting.Sun colour"),
        Some([1.0, 1.0, 1.0, 0.0])
    );
    assert_eq!(e.stage_vec3(0, "Lighting.Sun colour"), None);
}

#[test]
fn a_title_wide_key_reads_off_the_table_directly_with_no_stage() {
    let e = EffectSettings::parse(TWOK48_EXCERPT).expect("it parses");
    assert_eq!(e.table.scalar("Sky Radius"), Some(4000.0));
    assert_eq!(
        e.table.vec3("Lighting.Sun direction"),
        Some([-0.802_630, 0.469_582, 0.367_802])
    );
}

#[test]
fn a_value_over_one_is_reported_rather_than_clamped() {
    let e = EffectSettings::parse(TWOK48_EXCERPT).expect("it parses");
    let edge = e.stage_vec3(0, "Edge Colour").expect("Start's edge colour");
    assert_eq!(edge[0], 1.098_408);
}

#[test]
fn two_stages_own_disjoint_keys() {
    let e = EffectSettings::parse(TWOK48_EXCERPT).expect("it parses");
    assert!(e.stage_vec3(0, "Track Paint.Primary Colour").is_none());
    assert!(e.stage_vec3(12, "Colour 1.Colour").is_none());
    assert_eq!(
        e.stage_vec3(12, "Track Paint.Primary Colour"),
        Some([0.345_098, 0.0, 0.0])
    );
}

#[test]
fn stage_accessors_answer_none_for_a_stage_the_file_does_not_name() {
    let e = EffectSettings::parse(HD_EXCERPT).expect("it parses");
    assert_eq!(e.stage_key(9, "Sky horizon colour"), None);
    assert_eq!(e.stage_scalar(9, "anything"), None);
    assert_eq!(e.stage_vec3(9, "anything"), None);
    assert_eq!(e.stage_vec4(9, "anything"), None);
}

#[test]
fn a_key_with_no_space_before_the_dot_is_not_read_as_a_stage() {
    // "Lighting" alone never parses as a stage number, so it stays a
    // title-wide key rather than colliding with a real "0 Start" prefix.
    let e = EffectSettings::parse("\"Lighting.Constant Ambient Colour\"=1.0 1.0 1.0\n")
        .expect("it parses");
    assert!(e.stages.is_empty());
    assert_eq!(
        e.table.vec3("Lighting.Constant Ambient Colour"),
        Some([1.0, 1.0, 1.0])
    );
}

#[test]
fn a_malformed_line_reports_the_same_error_envsettings_does() {
    let err = EffectSettings::parse("\"0 Start.a\"=1.0\nnonsense\n").expect_err("the second line");
    assert_eq!(err.line, 2);
}

#[test]
fn cross_fade_at_full_weight_is_just_the_current_stage() {
    assert_eq!(
        cross_fade_rgba8([10, 20, 30, 40], [200, 200, 200, 200], 1.0),
        [10, 20, 30, 40]
    );
}

#[test]
fn cross_fade_at_zero_weight_is_just_the_previous_stage() {
    assert_eq!(
        cross_fade_rgba8([10, 20, 30, 40], [200, 200, 200, 200], 0.0),
        [200, 200, 200, 200]
    );
}

#[test]
fn cross_fade_at_the_midpoint_averages_each_channel() {
    // Each term truncates toward zero before the two are summed - not the
    // same as truncating the sum - so 255*0.5 + 255*0.5 lands on 254, one
    // short of a rounded average. That is the traced arithmetic, not a
    // rounding bug: see `cross_fade_rgba8`'s own doc comment.
    assert_eq!(
        cross_fade_rgba8([100, 0, 255, 10], [0, 100, 255, 20], 0.5),
        [50, 50, 254, 15]
    );
}

#[test]
fn cross_fade_clamps_high_rather_than_wrapping() {
    // An out-of-range weight (never seen at a real call site, but not
    // checked on the low end by the traced code either) must clamp, not
    // wrap the way a raw `u8` cast would. A zero previous channel keeps
    // the negative `other_weight` term from cancelling the overflow out.
    assert_eq!(
        cross_fade_rgba8([200, 0, 0, 0], [0, 0, 0, 0], 1.5),
        [255, 0, 0, 0]
    );
}
