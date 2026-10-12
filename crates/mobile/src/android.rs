//! Android: the NativeActivity's `AndroidApp`, the app's files directory and
//! the event loop winit builds over them. See docs/tools/android.md.

use std::sync::OnceLock;

use winit::event_loop::{EventLoop, EventLoopBuilder};
use winit::platform::android::EventLoopBuilderExtAndroid;
use winit::platform::android::activity::AndroidApp;

static APP: OnceLock<AndroidApp> = OnceLock::new();

/// Points the working directory and the XDG roots at the app's files directory,
/// then keeps `app` for [`event_loop`].
///
/// There is no command line and no working directory on Android, so the app's
/// external files directory stands in for both: `data/images`, `data/cache`,
/// the settings, records, ghosts and the log land under
/// `/storage/emulated/0/Android/data/<package>/files/` without `oag-source`
/// knowing about Android at all.
///
/// Call first thing in `android_main`, before any thread of ours exists.
#[allow(unsafe_code)] // `set_var`, see below
pub fn prepare(app: AndroidApp) {
    if let Some(files) = app
        .external_data_path()
        .or_else(|| app.internal_data_path())
    {
        for (name, sub) in [
            ("HOME", "home"),
            ("XDG_CONFIG_HOME", "config"),
            ("XDG_DATA_HOME", "data"),
            ("XDG_CACHE_HOME", "cache"),
            ("XDG_STATE_HOME", "state"),
        ] {
            let dir = files.join(sub);
            let _ = std::fs::create_dir_all(&dir);
            // SAFETY: the first thing `android_main` does, before any thread of
            // ours exists to race a read of the environment.
            unsafe { std::env::set_var(name, &dir) };
        }
        let _ = std::fs::create_dir_all(files.join("data").join("images"));
        let _ = std::env::set_current_dir(&files);
    }
    let _ = APP.set(app);
}

/// The window system's event loop, handed the activity [`prepare`] kept.
///
/// # Errors
///
/// [`prepare`] was never called, or winit refused to build the loop.
pub fn event_loop() -> Result<EventLoop<()>, String> {
    let app = APP
        .get()
        .ok_or("no AndroidApp: not started by android_main")?
        .clone();
    EventLoopBuilder::default()
        .with_android_app(app)
        .build()
        .map_err(|why| why.to_string())
}
