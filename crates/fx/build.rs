//! Compiles the WESL under `shaders/` to the plain WGSL `wgpu` is handed, and
//! validates each with `naga` so a bad shader fails the build. Every artifact
//! lands in `$OUT_DIR/<name>.wgsl`, pulled in with `include_str!`.

fn main() {
    oag_shader_check::link_each(
        "shaders",
        &[
            "beam",
            "cloud",
            "distort",
            "exhaust",
            "flash",
            "mist",
            "psys",
            "weapon_quads",
        ],
    );
    oag_shader_check::check_dir("src", &[]);
    println!("cargo::rerun-if-changed=build.rs");
}
