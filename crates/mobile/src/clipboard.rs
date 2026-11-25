//! Putting text on the phone's clipboard, so a benchmark result can be pasted into a forum post.
//!
//! Android only: winit's `NativeActivity` backend has no clipboard, so this asks the activity's
//! `ClipboardManager` through JNI, as [`crate::export`] asks `MediaStore`. The call runs on
//! whichever thread makes it (the JVM the activity started is attached for it). Nothing reads the
//! clipboard here: Android 10 and later hide it from an app that is not in the foreground, and a
//! paste is the player's own keyboard's.

use crate::export::Error;

/// Whether this build can write the clipboard: Android only.
#[must_use]
pub const fn available() -> bool {
    cfg!(target_os = "android")
}

/// Puts `text` on the clipboard as plain text.
///
/// # Errors
///
/// [`Error::Unsupported`] off Android or before the activity is up, [`Error::System`] when the
/// system refuses.
pub fn write(text: &str) -> Result<(), Error> {
    set(text)
}

#[cfg(target_os = "android")]
fn set(text: &str) -> Result<(), Error> {
    crate::android::copy(text)
}

#[cfg(not(target_os = "android"))]
fn set(_text: &str) -> Result<(), Error> {
    Err(Error::Unsupported(
        "the clipboard is written on Android only",
    ))
}
