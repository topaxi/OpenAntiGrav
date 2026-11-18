//! `oag-game` as the Android NativeActivity library, built as a `cdylib` by
//! `just apk`. The symbol NativeActivity looks up by name lives here; the body
//! is [`oag_game::app::android_main`]. See docs/tools/android.md.

// Same allocator as `main.rs`: it has to be the process's own to see anything.
#[cfg(feature = "perf-probe")]
#[global_allocator]
static ALLOCATOR: oag_gpu::perfprobe::Counting = oag_gpu::perfprobe::Counting;

#[cfg(target_os = "android")]
#[allow(unsafe_code)] // the C ABI symbol NativeActivity looks up by name
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    oag_game::app::android_main(app);
}
