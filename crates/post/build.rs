//! Compiles the WESL under `shaders/` to the plain WGSL `wgpu` is handed.
//!
//! Every artifact lands in `$OUT_DIR/<name>.wgsl` and is pulled in with
//! `include_str!`, so nothing past this script knows WESL exists.

use wesl::{CompileOptions, Compiler, ManglerKind, resolver::StandardResolver};

/// `(artifact, module path, features switched on)`. One module may be built
/// more than once, as `prepare_inputs` is for MSAA.
const ARTIFACTS: &[(&str, &str, &[&str])] = &[
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
        let mut options = CompileOptions {
            // The names a pipeline reads (`override` constants, entry points,
            // bindings) must reach wgpu exactly as written.
            mangler: ManglerKind::None,
            ..Default::default()
        };
        for feature in features {
            options.features.set(*feature, true);
        }
        let compiler = Compiler::new_with_resolver(options, StandardResolver::new("shaders"));
        let compiled = compiler
            .compile_module(&module.parse().expect("a module path"))
            .inspect_err(|error| eprintln!("{error}"))
            .unwrap_or_else(|_| panic!("{module} did not compile"));
        compiled.emit_rerun_if_changed();
        compiled.write_artifact(name);
    }
    println!("cargo::rerun-if-changed=build.rs");
}
