//! Compiles the WESL under `shaders/` to plain WGSL, validated with `naga`, so a bad shader
//! fails the build. The build's commit hash and version suffix come from `oag-net`'s build
//! script (`oag_net::BUILD_VERSION`).

fn main() {
    // Windows gives a main thread 1 MiB where Linux and macOS give 8, and the MSVC build overflowed
    // it on its first frame (the reporter's exe ran once its header said 8 MiB). The MinGW build
    // under wine did not, so only MSVC needs asking.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-bins=/STACK:8388608");
    }
    // The renderer's own shaders are WESL under `shaders/`. The screen-filter
    // presets under `assets/` are bodies compiled after `oag-post`'s prelude,
    // so they are not modules alone and stay plain WGSL.
    oag_shader_check::link_each("shaders", &["backdrop", "scene", "ui", "video"]);
    oag_shader_check::check_dir("src", &[]);
}
