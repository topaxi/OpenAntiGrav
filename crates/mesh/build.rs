//! Compiles the WESL under `shaders/` to the plain WGSL `wgpu` is handed, and
//! validates it with `naga` so a bad shader fails the build.
//!
//! The artifacts land in `$OUT_DIR/mesh.wgsl` and `$OUT_DIR/mesh_bindless.wgsl`
//! and are pulled in with `include_str!`, so nothing past this script knows WESL
//! exists.

fn main() {
    oag_shader_check::link("shaders", "mesh", "package::mesh", &[]);
    // The same shader with group 1 as binding arrays, for a device with
    // `mesh_render::BINDLESS_FEATURES` - see `shaders/bindings.wesl`.
    oag_shader_check::link("shaders", "mesh_bindless", "package::mesh", &["bindless"]);
    // The ray-query debug picture, for a device with `EXPERIMENTAL_RAY_QUERY` only -
    // see `mesh_render::ray`.
    oag_shader_check::link("shaders", "ray_debug", "package::ray_debug", &[]);
    println!("cargo::rerun-if-changed=build.rs");
}
