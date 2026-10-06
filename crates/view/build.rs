//! Validates every `.wgsl` under `src/` with `naga`, so an undeclared name or a
//! type mismatch fails `cargo build` instead of surfacing at race start.

fn main() {
    oag_shader_check::check_dir("src", &[]);
    println!("cargo::rerun-if-changed=build.rs");
}
