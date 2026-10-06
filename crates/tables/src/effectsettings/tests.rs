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
    // "Lighting" alone never parses as a stage number: it stays title-wide.
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
    // Each term truncates toward zero before summing, so 255*0.5 + 255*0.5 is
    // 254: the traced arithmetic, not a rounding bug (see `cross_fade_rgba8`).
    assert_eq!(
        cross_fade_rgba8([100, 0, 255, 10], [0, 100, 255, 20], 0.5),
        [50, 50, 254, 15]
    );
}

#[test]
fn cross_fade_clamps_high_rather_than_wrapping() {
    // An out-of-range weight (not checked on the low end by the traced code
    // either) must clamp, not wrap like a raw `u8` cast; a zero previous channel
    // keeps the negative term from cancelling the overflow.
    assert_eq!(
        cross_fade_rgba8([200, 0, 0, 0], [0, 0, 0, 0], 1.5),
        [255, 0, 0, 0]
    );
}

/// Two stages of HD's file, verbatim (`Start`, `Sub Venom`), copied from the disc
/// (`just psarc cat ... /data/environments/zonemode.effectsettings`):
/// `effectsettings_ground_truth.rs` asserts the same numbers off the image, so a
/// drift here fails there too.
const HD_TWO_STAGES: &str = concat!(
    "\"0 Start.Lighting.Sky reflection colour\"=0 0 0 255\n",
    "\"0 Start.Lighting.Sun colour\"=1.000000 1.000000 1.000000 0.000000\n",
    "\"0 Start.Lighting.Constant Ambient Colour\"=1.000000 1.000000 1.000000 0.000000\n",
    "\"0 Start.Lighting.Fog colour\"=0.000000 0.000000 0.000000 0.000000\n",
    "\"0 Start.Lighting.Fog density\"=0.000000\n",
    "\"0 Start.Sky horizon colour\"=0.000000 0.000000 0.000000\n",
    "\"1 Sub Venom.Lighting.Sky reflection colour\"=255 255 255 255\n",
    "\"1 Sub Venom.Lighting.Sun colour\"=0.000000 0.000000 0.000000 0.000000\n",
    "\"1 Sub Venom.Lighting.Constant Ambient Colour\"=1.500000 1.500000 1.500000 0.000000\n",
    "\"1 Sub Venom.Lighting.Fog colour\"=0.000000 1.305882 1.800000 0.000000\n",
    "\"1 Sub Venom.Lighting.Fog density\"=0.002100\n",
    "\"1 Sub Venom.Sky horizon colour\"=0.000000 0.211765 0.247059\n",
);

#[test]
fn a_stage_palette_reads_the_keys_the_schema_names() {
    let e = EffectSettings::parse(HD_TWO_STAGES).expect("it parses");
    let one = e.stage_palette(1).expect("stage 1 is named");
    assert_eq!(one.fog_colour, Some([0.0, 1.305_882, 1.8]));
    assert_eq!(one.fog_density, Some(0.002_1));
    assert_eq!(one.ambient_colour, Some([1.5, 1.5, 1.5]));
    assert_eq!(one.sky_reflection_colour, Some([255, 255, 255, 255]));
    // Absent on both stages and not substituted: the file exercises part of the schema.
    assert_eq!(one.prelit_power, None);
}

#[test]
fn a_stage_the_file_does_not_name_has_no_palette() {
    let e = EffectSettings::parse(HD_TWO_STAGES).expect("it parses");
    assert_eq!(e.stage_palette(7), None);
    assert_eq!(e.blended_palette(7, 1.0), None);
}

/// The alpha lane every four-number colour authors `0.000000` is dropped, so a
/// three-number key and a four-number one read the same shape.
#[test]
fn a_colour_reads_its_rgb_whether_it_is_written_with_three_numbers_or_four() {
    let e = EffectSettings::parse(HD_TWO_STAGES).expect("it parses");
    assert_eq!(
        e.stage_rgb(1, key::FOG_COLOUR),
        Some([0.0, 1.305_882, 1.8]),
        "four numbers"
    );
    assert_eq!(
        e.stage_rgb(1, key::SKY_HORIZON_COLOUR),
        Some([0.0, 0.211_765, 0.247_059]),
        "three numbers"
    );
}

/// The ends of the fade are each stage's own authored values, untouched.
#[test]
fn the_ends_of_a_cross_fade_are_the_two_stages_themselves() {
    let e = EffectSettings::parse(HD_TWO_STAGES).expect("it parses");
    let zero = e.stage_palette(0).expect("stage 0");
    let one = e.stage_palette(1).expect("stage 1");
    assert_eq!(
        one.cross_fade(zero, 1.0),
        one,
        "weight 1 is the current stage"
    );
    assert_eq!(
        one.cross_fade(zero, 0.0),
        zero,
        "weight 0 is the previous one"
    );
}

/// Halfway between `Start` and `Sub Venom`: float keys fade in floats, the one
/// byte-written key through the recovered byte arithmetic (`(int)(255*0.5) +
/// (int)(0*0.5)` is 127, not 127.5).
#[test]
fn a_half_weight_blend_fades_each_field_in_its_own_domain() {
    let e = EffectSettings::parse(HD_TWO_STAGES).expect("it parses");
    let half = e.blended_palette(1, 0.5).expect("stage 1 is named");
    assert_eq!(half.fog_colour, Some([0.0, 1.305_882 / 2.0, 0.9]));
    assert_eq!(half.fog_density, Some(0.002_1 / 2.0));
    assert_eq!(half.ambient_colour, Some([1.25, 1.25, 1.25]));
    // Alpha `255` on both stages lands on `254`: each term truncates before
    // summing, the original's own rounding.
    assert_eq!(half.sky_reflection_colour, Some([127, 127, 127, 254]));
}

/// **A float colour past `1.0` survives the blend.** Running it through the
/// byte-domain cross-fade would send `1.8` to white; the corpus authors values
/// up to `9.0`, so this is the property that keeps the file's own data intact.
#[test]
fn a_float_colour_over_one_is_faded_rather_than_clipped_to_white() {
    let e = EffectSettings::parse(HD_TWO_STAGES).expect("it parses");
    let full = e.blended_palette(1, 1.0).expect("stage 1 is named");
    assert_eq!(full.fog_colour, Some([0.0, 1.305_882, 1.8]));
}

/// Stage zero pairs with itself (`stage - 1` saturates), so at the ends of the
/// fade it is exactly its own palette: a race sits on stage zero at weight `1.0`
/// and that must change nothing.
#[test]
fn stage_zero_blends_against_itself() {
    let e = EffectSettings::parse(HD_TWO_STAGES).expect("it parses");
    let zero = e.stage_palette(0).expect("stage 0");
    for weight in [0.0, 1.0] {
        assert_eq!(e.blended_palette(0, weight), Some(zero), "weight {weight}");
    }
    // **Not** at an intermediate weight on the byte key: `255` faded against
    // `255` at `0.5` is `254`, which is why the resting weight is `1.0`.
    let half = e.blended_palette(0, 0.5).expect("stage 0");
    assert_eq!(half.sky_reflection_colour, Some([0, 0, 0, 254]));
    assert_eq!(half.fog_colour, zero.fog_colour, "the float keys are exact");
}

/// A field fades only where both stages author it; otherwise the current
/// stage's value stands, not a fade from an invented zero.
#[test]
fn a_key_the_previous_stage_omits_keeps_the_current_stages_value() {
    let text = concat!(
        "\"0 Start.Lighting.Fog density\"=0.000000\n",
        "\"1 Sub Venom.Lighting.Fog density\"=0.002100\n",
        "\"1 Sub Venom.Lighting.Fog colour\"=0.000000 1.305882 1.800000 0.000000\n",
    );
    let e = EffectSettings::parse(text).expect("it parses");
    let half = e.blended_palette(1, 0.5).expect("stage 1 is named");
    assert_eq!(
        half.fog_density,
        Some(0.002_1 / 2.0),
        "both stages author it"
    );
    assert_eq!(
        half.fog_colour,
        Some([0.0, 1.305_882, 1.8]),
        "only stage 1 authors it"
    );
}
