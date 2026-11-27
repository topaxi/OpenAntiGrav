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

/// Unpacks the official mods the APK carries into `dest`, relative to the working directory
/// [`prepare`] set (pass `oag_mod::OFFICIAL`, which `oag_mod::mods_search_path` already
/// lists). Only the first launch of a build writes anything. See [`crate::packaged`].
///
/// Runs on the activity's thread before the event loop and the window exist, so no frame
/// waits on it.
///
/// # Errors
///
/// [`prepare`] was never called, the APK has no mods, or the disk refused a write.
pub fn install_official_mods(
    dest: &std::path::Path,
) -> std::io::Result<crate::packaged::Installed> {
    use std::io::{Error, ErrorKind, Read};

    let app = APP
        .get()
        .ok_or_else(|| Error::other("no AndroidApp: not started by android_main"))?;
    let assets = app.asset_manager();
    crate::packaged::install(
        &mut |name| {
            let name = std::ffi::CString::new(name)
                .map_err(|_| Error::new(ErrorKind::InvalidInput, "asset name has a NUL"))?;
            let mut asset = assets
                .open(&name)
                .ok_or_else(|| Error::new(ErrorKind::NotFound, name.to_string_lossy()))?;
            let mut bytes = Vec::new();
            asset.read_to_end(&mut bytes)?;
            Ok(bytes)
        },
        dest,
    )
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

/// The name of the one-shot launch arguments file in the app's files directory.
pub const LAUNCH_ARGS_FILE: &str = "oag-args.txt";

/// Takes the command line a developer left for this launch from [`LAUNCH_ARGS_FILE`] in the
/// working directory [`prepare`] set, and deletes the file so the next launch from the home
/// screen is a normal one.
///
/// Android gives a NativeActivity no command line and no environment. This is how a
/// measurement run starts a race without touch: `adb shell` writes the file, then
/// `am start` launches. Read in every build, because a measurement needs a release one. The
/// directory is app-private from Android 11; on 8 to 10 an app holding the storage
/// permission could write the file too. One argument a
/// line; blank lines are skipped. A line `OAG_NAME=value` (an upper-case `OAG_` name) sets
/// that environment variable instead, for the A/B switches the desktop reads from the
/// environment (`OAG_SPLIT_FINISH=0`). No file, an empty list. See docs/tools/android.md.
///
/// Call right after [`prepare`], before any thread of ours exists.
#[allow(unsafe_code)] // `set_var`, see below
#[must_use]
pub fn take_launch_args() -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(LAUNCH_ARGS_FILE) else {
        return Vec::new();
    };
    let _ = std::fs::remove_file(LAUNCH_ARGS_FILE);
    let mut args = Vec::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        match line.split_once('=') {
            Some((name, value))
                if name.starts_with("OAG_")
                    && name
                        .bytes()
                        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_') =>
            {
                // SAFETY: called from `android_main` before any thread of ours exists.
                unsafe { std::env::set_var(name, value) };
            }
            _ => args.push(line.to_owned()),
        }
    }
    args
}

/// Asks, each time it is called, the refresh rate in Hz of the display the activity is on.
///
/// The returned reader holds its own reference to the activity, so a thread may keep it; it
/// attaches that thread to the JVM the activity started.
///
/// # Errors
///
/// The activity is not up, or the JVM refused the reference.
pub(crate) fn display_rate()
-> Result<impl FnMut() -> Result<f32, crate::export::Error> + Send + 'static, crate::export::Error>
{
    use crate::export::Error;
    let app = APP
        .get()
        .ok_or(Error::Unsupported("the activity is not up"))?;
    let vm = jni::JavaVM::singleton().map_err(|e| Error::System(e.to_string()))?;
    let activity = app.activity_as_ptr();
    let activity = vm
        .attach_current_thread(|env| own_activity(env, activity))
        .map_err(|e| Error::System(e.to_string()))?;
    Ok(move || {
        // The attachment names the thread: unnamed, ART renames it `Thread-N` in every profile.
        let named = || jni::AttachConfig::default().thread_name(jni::jni_str!("android-refresh"));
        vm.attach_current_thread_with_config(
            named,
            Some(jni::DEFAULT_LOCAL_FRAME_CAPACITY),
            |env| {
                let outcome = display_refresh_rate(env, &activity);
                if env.exception_check() {
                    env.exception_clear();
                }
                outcome.map_err(|e| Error::System(e.to_string()))
            },
        )
    })
}

/// A global reference of our own to the activity, valid on any thread for as long as we hold it.
#[allow(unsafe_code)] // wrapping the activity's raw reference, see below
fn own_activity(
    env: &mut jni::Env,
    activity: *mut std::ffi::c_void,
) -> jni::errors::Result<jni::refs::Global<jni::objects::JObject<'static>>> {
    use jni::objects::JObject;
    use std::mem::ManuallyDrop;

    // SAFETY: the activity's JNI global reference, owned by `android-activity` for as long as
    // the app runs; `ManuallyDrop` keeps this wrapper from deleting a reference it does not own.
    let activity = ManuallyDrop::new(unsafe { JObject::from_raw(env, activity.cast()) });
    env.new_global_ref(&*activity)
}

/// `activity.getWindowManager().getDefaultDisplay().getRefreshRate()`: the activity's own
/// display, on every Android the APK runs on (`Activity.getDisplay` needs API 30).
fn display_refresh_rate(
    env: &mut jni::Env,
    activity: &jni::objects::JObject,
) -> jni::errors::Result<f32> {
    use jni::{jni_sig, jni_str};

    let manager = env
        .call_method(
            activity,
            jni_str!("getWindowManager"),
            jni_sig!("()Landroid/view/WindowManager;"),
            &[],
        )?
        .l()?;
    let display = env
        .call_method(
            &manager,
            jni_str!("getDefaultDisplay"),
            jni_sig!("()Landroid/view/Display;"),
            &[],
        )?
        .l()?;
    env.call_method(&display, jni_str!("getRefreshRate"), jni_sig!("()F"), &[])?
        .f()
}

/// Inserts `bytes` into `MediaStore` as `plan` describes and returns the new item's URI.
///
/// Runs on whichever thread calls it: the JVM the activity started is attached for the call.
/// Needs Android 10 (API 29), where `RELATIVE_PATH` and `IS_PENDING` exist and an app may add
/// its own media with no permission.
pub(crate) fn export(
    plan: &crate::export::Plan,
    bytes: &[u8],
) -> Result<String, crate::export::Error> {
    use crate::export::Error;
    let app = APP
        .get()
        .ok_or(Error::Unsupported("the activity is not up"))?;
    let vm = jni::JavaVM::singleton().map_err(|e| Error::System(e.to_string()))?;
    let activity = app.activity_as_ptr();
    vm.attach_current_thread(|env| {
        let outcome = media_store_insert(env, activity, plan, bytes);
        if env.exception_check() {
            env.exception_describe();
            env.exception_clear();
        }
        outcome
    })
}

/// Puts `text` on the activity's clipboard as plain text.
///
/// Runs on whichever thread calls it: the JVM the activity started is attached for the call.
pub(crate) fn copy(text: &str) -> Result<(), crate::export::Error> {
    use crate::export::Error;
    let app = APP
        .get()
        .ok_or(Error::Unsupported("the activity is not up"))?;
    let vm = jni::JavaVM::singleton().map_err(|e| Error::System(e.to_string()))?;
    let activity = app.activity_as_ptr();
    vm.attach_current_thread(|env| {
        let outcome = clipboard_set(env, activity, text);
        if env.exception_check() {
            env.exception_describe();
            env.exception_clear();
        }
        outcome
    })
}

#[allow(unsafe_code)] // wrapping the activity's raw reference, see below
fn clipboard_set(
    env: &mut jni::Env,
    activity: *mut std::ffi::c_void,
    text: &str,
) -> Result<(), crate::export::Error> {
    use crate::export::Error;
    use jni::objects::{JObject, JValue};
    use jni::{jni_sig, jni_str};
    use std::mem::ManuallyDrop;

    let sys = |e: jni::errors::Error| Error::System(e.to_string());
    // SAFETY: the activity's JNI global reference, owned by `android-activity` for as long as
    // the app runs; `ManuallyDrop` keeps this wrapper from deleting a reference it does not own.
    let activity = ManuallyDrop::new(unsafe { JObject::from_raw(env, activity.cast()) });
    let service = env.new_string("clipboard").map_err(sys)?;
    let manager = env
        .call_method(
            &*activity,
            jni_str!("getSystemService"),
            jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
            &[JValue::from(&service)],
        )
        .and_then(|v| v.l())
        .map_err(sys)?;
    let label = env.new_string("OpenAntiGrav").map_err(sys)?;
    let body = env.new_string(text).map_err(sys)?;
    let clip = env
        .call_static_method(
            jni_str!("android/content/ClipData"),
            jni_str!("newPlainText"),
            jni_sig!(
                "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;"
            ),
            &[JValue::from(&label), JValue::from(&body)],
        )
        .and_then(|v| v.l())
        .map_err(sys)?;
    env.call_method(
        &manager,
        jni_str!("setPrimaryClip"),
        jni_sig!("(Landroid/content/ClipData;)V"),
        &[JValue::from(&clip)],
    )
    .map_err(sys)?;
    Ok(())
}

fn put_string(
    env: &mut jni::Env,
    values: &jni::objects::JObject,
    key: &str,
    value: &str,
) -> Result<(), crate::export::Error> {
    use jni::objects::JValue;
    use jni::{jni_sig, jni_str};
    let key = env.new_string(key)?;
    let value = env.new_string(value)?;
    env.call_method(
        values,
        jni_str!("put"),
        jni_sig!("(Ljava/lang/String;Ljava/lang/String;)V"),
        &[JValue::from(&key), JValue::from(&value)],
    )?;
    Ok(())
}

fn put_int(
    env: &mut jni::Env,
    values: &jni::objects::JObject,
    key: &str,
    value: i32,
) -> Result<(), crate::export::Error> {
    use jni::objects::JValue;
    use jni::{jni_sig, jni_str};
    let key = env.new_string(key)?;
    let boxed = env
        .call_static_method(
            jni_str!("java/lang/Integer"),
            jni_str!("valueOf"),
            jni_sig!("(I)Ljava/lang/Integer;"),
            &[JValue::Int(value)],
        )?
        .l()?;
    env.call_method(
        values,
        jni_str!("put"),
        jni_sig!("(Ljava/lang/String;Ljava/lang/Integer;)V"),
        &[JValue::from(&key), JValue::from(&boxed)],
    )?;
    Ok(())
}

#[allow(unsafe_code)] // wrapping the activity's raw reference, see below
fn media_store_insert(
    env: &mut jni::Env,
    activity: *mut std::ffi::c_void,
    plan: &crate::export::Plan,
    bytes: &[u8],
) -> Result<String, crate::export::Error> {
    use crate::export::{Collection, Error};
    use jni::objects::{JObject, JString, JValue};
    use jni::{jni_sig, jni_str};
    use std::mem::ManuallyDrop;

    let sys = |e: jni::errors::Error| Error::System(e.to_string());
    let sdk = env
        .get_static_field(
            jni_str!("android/os/Build$VERSION"),
            jni_str!("SDK_INT"),
            jni_sig!("I"),
        )
        .and_then(|v| v.i())
        .map_err(sys)?;
    if sdk < 29 {
        return Err(Error::Unsupported("shared storage needs Android 10"));
    }
    // SAFETY: the activity's JNI global reference, owned by `android-activity` for as long as
    // the app runs; `ManuallyDrop` keeps this wrapper from deleting a reference it does not own.
    let activity = ManuallyDrop::new(unsafe { JObject::from_raw(env, activity.cast()) });
    let resolver = env
        .call_method(
            &*activity,
            jni_str!("getContentResolver"),
            jni_sig!("()Landroid/content/ContentResolver;"),
            &[],
        )
        .and_then(|v| v.l())
        .map_err(sys)?;

    let values = env
        .new_object(
            jni_str!("android/content/ContentValues"),
            jni_sig!("()V"),
            &[],
        )
        .map_err(sys)?;
    put_string(env, &values, "_display_name", &plan.display_name)?;
    put_string(env, &values, "mime_type", plan.mime)?;
    put_string(env, &values, "relative_path", &plan.relative_dir)?;
    put_int(env, &values, "is_pending", 1)?;

    let collection = match plan.collection {
        Collection::Images => env.get_static_field(
            jni_str!("android/provider/MediaStore$Images$Media"),
            jni_str!("EXTERNAL_CONTENT_URI"),
            jni_sig!("Landroid/net/Uri;"),
        ),
        Collection::Downloads => env.get_static_field(
            jni_str!("android/provider/MediaStore$Downloads"),
            jni_str!("EXTERNAL_CONTENT_URI"),
            jni_sig!("Landroid/net/Uri;"),
        ),
    }
    .and_then(|v| v.l())
    .map_err(sys)?;

    let uri = env
        .call_method(
            &resolver,
            jni_str!("insert"),
            jni_sig!("(Landroid/net/Uri;Landroid/content/ContentValues;)Landroid/net/Uri;"),
            &[JValue::from(&collection), JValue::from(&values)],
        )
        .and_then(|v| v.l())
        .map_err(sys)?;
    if uri.is_null() {
        return Err(Error::System("MediaStore gave no location".into()));
    }

    let stream = env
        .call_method(
            &resolver,
            jni_str!("openOutputStream"),
            jni_sig!("(Landroid/net/Uri;)Ljava/io/OutputStream;"),
            &[JValue::from(&uri)],
        )
        .and_then(|v| v.l())
        .map_err(sys)?;
    if stream.is_null() {
        return Err(Error::System("no stream to write to".into()));
    }
    let array = env.byte_array_from_slice(bytes).map_err(sys)?;
    let written = env
        .call_method(
            &stream,
            jni_str!("write"),
            jni_sig!("([B)V"),
            &[JValue::from(&array)],
        )
        .map_err(sys);
    let closed = env
        .call_method(&stream, jni_str!("close"), jni_sig!("()V"), &[])
        .map_err(sys);
    written?;
    closed?;

    // Visible to the gallery and the file manager only once it is no longer pending.
    let done = env
        .new_object(
            jni_str!("android/content/ContentValues"),
            jni_sig!("()V"),
            &[],
        )
        .map_err(sys)?;
    put_int(env, &done, "is_pending", 0)?;
    env.call_method(
        &resolver,
        jni_str!("update"),
        jni_sig!("(Landroid/net/Uri;Landroid/content/ContentValues;Ljava/lang/String;[Ljava/lang/String;)I"),
        &[
            JValue::from(&uri),
            JValue::from(&done),
            JValue::from(&JObject::null()),
            JValue::from(&JObject::null()),
        ],
    )
    .map_err(sys)?;

    let text = env
        .call_method(
            &uri,
            jni_str!("toString"),
            jni_sig!("()Ljava/lang/String;"),
            &[],
        )
        .and_then(|v| v.l())
        .map_err(sys)?;
    let text = env.cast_local::<JString>(text).map_err(sys)?;
    Ok(text.to_string())
}
