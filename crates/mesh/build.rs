//! Compiles the WESL under `shaders/` to the plain WGSL `wgpu` is handed.
//!
//! The artifact lands in `$OUT_DIR/mesh.wgsl` and is pulled in with
//! `include_str!`, so nothing past this script knows WESL exists.

use wesl::{CompileOptions, Compiler, ManglerKind, resolver::StandardResolver};

fn main() {
    let options = CompileOptions {
        // The names a pipeline reads (`override` constants, entry points,
        // bindings) must reach wgpu exactly as written.
        mangler: ManglerKind::None,
        ..Default::default()
    };
    let compiler = Compiler::new_with_resolver(options, StandardResolver::new("shaders"));
    let compiled = compiler
        .compile_module(&"package::mesh".parse().expect("a module path"))
        .inspect_err(|error| eprintln!("{error}"))
        .expect("the mesh shader did not compile");
    compiled.emit_rerun_if_changed();
    compiled.write_artifact("mesh");
    println!("cargo::rerun-if-changed=build.rs");
}
