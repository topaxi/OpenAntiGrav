//! Compiles the WESL under `shaders/` to the plain WGSL `wgpu` is handed.
//!
//! Every artifact lands in `$OUT_DIR/<name>.wgsl` and is pulled in with
//! `include_str!`, so nothing past this script knows WESL exists. Every artifact,
//! and every plain `.wgsl` under `src/`, is validated with `naga` so a bad
//! shader fails the build.

/// `(artifact, module path, features switched on)`. One module may be built
/// more than once, as `prepare_inputs` is for MSAA.
const ARTIFACTS: &[(&str, &str, &[&str])] = &[
    ("fxaa", "package::fxaa", &[]),
    ("fill_depth", "package::fill_depth", &[]),
    ("smaa", "package::smaa", &[]),
    ("fsr1", "package::fsr1", &[]),
    ("motion_blur", "package::motion_blur", &[]),
    ("omega_tonemap", "package::omega_tonemap", &[]),
    ("hd_bloom", "package::hd_bloom", &[]),
    ("ps2_bloom", "package::ps2_bloom", &[]),
    ("bloom", "package::bloom", &[]),
    ("fsr3_clear", "package::fsr3::clear", &[]),
    ("fsr3_prepare_inputs", "package::fsr3::prepare_inputs", &[]),
    (
        "fsr3_prepare_inputs_msaa",
        "package::fsr3::prepare_inputs",
        &["msaa"],
    ),
    ("fsr3_luma_pyramid", "package::fsr3::luma_pyramid", &[]),
    (
        "fsr3_shading_change_pyramid",
        "package::fsr3::shading_change_pyramid",
        &[],
    ),
    ("fsr3_shading_change", "package::fsr3::shading_change", &[]),
    (
        "fsr3_prepare_reactivity",
        "package::fsr3::prepare_reactivity",
        &[],
    ),
    (
        "fsr3_luma_instability",
        "package::fsr3::luma_instability",
        &[],
    ),
    ("fsr3_accumulate", "package::fsr3::accumulate", &[]),
    ("fsr3_rcas", "package::fsr3::rcas", &[]),
];

fn main() {
    for &(name, module, features) in ARTIFACTS {
        oag_shader_check::link("shaders", name, module, features);
    }
    // `screen.wgsl` is the prelude every screen-filter preset is compiled after:
    // it calls the preset's `param_*()` functions, so it is no module alone.
    // `Preset::new` validates it with each preset at runtime, and the presets
    // are covered by `oag-game`'s tests.
    oag_shader_check::check_dir("src", &["screen.wgsl"]);
    println!("cargo::rerun-if-changed=build.rs");
}
