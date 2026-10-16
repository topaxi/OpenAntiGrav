//! Compiles the WESL under `shaders/` to plain WGSL, validated with `naga`, so a bad shader
//! fails the build. The build's commit hash and version suffix come from `oag-net`'s build
//! script (`oag_net::BUILD_VERSION`).

fn main() {
    // The renderer's own shaders are WESL under `shaders/`. The screen-filter
    // presets under `assets/` are bodies compiled after `oag-post`'s prelude,
    // so they are not modules alone and stay plain WGSL.
    oag_shader_check::link_each("shaders", &["backdrop", "scene", "ui", "video"]);
    oag_shader_check::check_dir("src", &[]);
}
