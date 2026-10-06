//! Compiles the WESL under `shaders/` to the plain WGSL `wgpu` is handed, and
//! validates it with `naga` so a bad shader fails the build.
//!
//! The artifact lands in `$OUT_DIR/mesh.wgsl` and is pulled in with
//! `include_str!`, so nothing past this script knows WESL exists.

fn main() {
    oag_shader_check::link("shaders", "mesh", "package::mesh", &[]);
    println!("cargo::rerun-if-changed=build.rs");
}
