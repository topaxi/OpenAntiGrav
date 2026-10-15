//! Compiles the WESL under `shaders/` to the plain WGSL `wgpu` is handed, and
//! validates it with `naga` so a bad shader fails the build.
//!
//! The artifacts land in `$OUT_DIR/mesh.wgsl` and `$OUT_DIR/mesh_gl.wgsl` and is pulled in with
//! `include_str!`, so nothing past this script knows WESL exists.

fn main() {
    oag_shader_check::link("shaders", "mesh", "package::mesh", &[]);
    // The same shader with the `gl` feature on: what wgpu's GL backend can
    // compile. `create_shader_module` picks it by the device's backend.
    oag_shader_check::link("shaders", "mesh_gl", "package::mesh", &["gl"]);
    println!("cargo::rerun-if-changed=build.rs");
}
