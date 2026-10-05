//! What a preset file is asserted to parse to, and what naga is asserted to
//! reject - none of which needs a device.

use super::*;

const PASSTHROUGH: &str = "\
//! name = \"Nothing\"
//! description = \"The frame, untouched.\"
//!
//! [[param]]
//! name = \"amount\"
//! default = 0.5
//! max = 2.0

fn screen_filter(uv: vec2<f32>, pixel: vec2<f32>) -> vec3<f32> {
    return frame_at(uv) * param_amount();
}
";

#[test]
fn the_header_names_the_preset_and_its_tunables() {
    let preset = Preset::parse("nothing", PASSTHROUGH).expect("a preset");
    assert_eq!(preset.name, "Nothing");
    assert_eq!(preset.description, "The frame, untouched.");
    assert_eq!(preset.params.len(), 1);
    assert_eq!(preset.params[0].name, "amount");
    assert_eq!(preset.params[0].default, 0.5);
    assert_eq!(preset.params[0].min, 0.0);
    assert_eq!(preset.params[0].max, 2.0);
    assert_eq!(preset.params[0].step, 0.01);
    assert!(
        preset.body.starts_with("fn screen_filter"),
        "{:?}",
        preset.body
    );
    assert_eq!(preset.defaults()[0][0], 0.5);
}

#[test]
fn a_file_with_no_header_is_named_after_its_id() {
    let preset = Preset::parse(
        "bare",
        "fn screen_filter(uv: vec2<f32>, pixel: vec2<f32>) -> vec3<f32> { return frame_at(uv); }",
    )
    .expect("a preset");
    assert_eq!(preset.name, "bare");
    assert!(preset.params.is_empty());
    preset.validate().expect("valid");
}

#[test]
fn the_compiled_module_validates_and_carries_the_accessor() {
    let preset = Preset::parse("nothing", PASSTHROUGH).expect("a preset");
    let wgsl = preset.wgsl();
    assert!(wgsl.starts_with(PRELUDE));
    assert!(
        wgsl.contains("fn param_amount() -> f32 { return screen.params[0u][0u]; }"),
        "{wgsl}"
    );
    preset.validate().expect("valid");
}

#[test]
fn the_seventeenth_tunable_is_refused() {
    let mut source = String::new();
    for i in 0..=MAX_PARAMS {
        source.push_str(&format!(
            "//! [[param]]\n//! name = \"p{i}\"\n//! default = 0.0\n"
        ));
    }
    source.push_str(
        "fn screen_filter(uv: vec2<f32>, pixel: vec2<f32>) -> vec3<f32> { return frame_at(uv); }\n",
    );
    let error = Preset::parse("many", &source).expect_err("too many");
    assert!(error.to_string().contains("17 tunables"), "{error:#}");
}

#[test]
fn a_tunable_that_is_not_an_identifier_is_refused() {
    let source = "//! [[param]]\n//! name = \"not ok\"\n//! default = 0.0\nfn screen_filter(uv: vec2<f32>, pixel: vec2<f32>) -> vec3<f32> { return frame_at(uv); }\n";
    let error = Preset::parse("bad", source).expect_err("bad name");
    assert!(
        error.to_string().contains("not a WGSL identifier"),
        "{error:#}"
    );
}

#[test]
fn a_default_outside_its_range_is_refused() {
    let source = "//! [[param]]\n//! name = \"x\"\n//! default = 3.0\n//! max = 1.0\nfn screen_filter(uv: vec2<f32>, pixel: vec2<f32>) -> vec3<f32> { return frame_at(uv); }\n";
    let error = Preset::parse("bad", source).expect_err("out of range");
    assert!(error.to_string().contains("outside"), "{error:#}");
}

#[test]
fn an_unknown_header_key_is_refused() {
    let source = "//! nmae = \"typo\"\nfn screen_filter(uv: vec2<f32>, pixel: vec2<f32>) -> vec3<f32> { return frame_at(uv); }\n";
    let error = Preset::parse("typo", source).expect_err("unknown key");
    assert!(error.to_string().contains("header"), "{error:#}");
}

#[test]
fn a_body_that_does_not_parse_reports_its_line() {
    let source = "//! name = \"Broken\"\n\nfn screen_filter(uv: vec2<f32>, pixel: vec2<f32>) -> vec3<f32> {\n    return frame_at(uv) +;\n}\n";
    let preset = Preset::parse("broken", source).expect("the header is fine");
    let error = preset.validate().expect_err("does not parse");
    let text = error.to_string();
    // naga's report names the file and marks the line with a caret.
    assert!(text.contains("broken"), "{text}");
    assert!(text.contains('^'), "{text}");
}

#[test]
fn a_body_that_defines_no_filter_is_refused() {
    let preset = Preset::parse("empty", "//! name = \"Empty\"\n").expect("the header is fine");
    let error = preset.validate().expect_err("no filter");
    assert!(error.to_string().contains("defines no"), "{error:#}");
}

#[test]
fn a_body_that_returns_the_wrong_type_is_refused() {
    let source = "fn screen_filter(uv: vec2<f32>, pixel: vec2<f32>) -> vec4<f32> { return vec4<f32>(1.0); }\n";
    let preset = Preset::parse("wrong", source).expect("the header is fine");
    preset
        .validate()
        .expect_err("fs_main's call does not type-check");
}

#[test]
fn the_uniform_is_the_size_the_shader_declares() {
    // 6 floats, 3 floats of padding, 4 vec4s: 112 bytes, 16-aligned.
    assert_eq!(size_of::<Uniform>(), 112);
}
