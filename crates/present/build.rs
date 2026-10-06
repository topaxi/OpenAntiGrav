//! Compiles the WESL under `shaders/` to the plain WGSL `wgpu` is handed, and
//! validates it with `naga` so a bad shader fails the build. The artifact lands
//! in `$OUT_DIR/upscale.wgsl`, pulled in with `include_str!`.

fn main() {
    oag_shader_check::link_each("shaders", &["upscale"]);
    oag_shader_check::check_dir("src", &[]);
    println!("cargo::rerun-if-changed=build.rs");
}
