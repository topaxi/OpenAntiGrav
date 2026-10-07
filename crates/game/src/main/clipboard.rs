//! The system clipboard, for pasting a disc key into the chooser.
//!
//! Desktop builds read it through `arboard`. **Android reads nothing**: winit's
//! `NativeActivity` backend has no clipboard and this project carries no JNI
//! shim, so [`AVAILABLE`] is `false` there and the key prompt offers its keypad
//! and, beside the image, a `.dkey` file instead.

/// Whether this build can read a clipboard.
pub(crate) const AVAILABLE: bool = cfg!(not(target_os = "android"));

static CTRL: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Records whether a Control key is down, from the window's modifier events.
pub(crate) fn set_ctrl_held(held: bool) {
    CTRL.store(held, std::sync::atomic::Ordering::Relaxed);
}

/// Whether a Control key is down.
pub(crate) fn ctrl_held() -> bool {
    CTRL.load(std::sync::atomic::Ordering::Relaxed)
}

/// The clipboard's text, or `None` when it holds none or cannot be read.
#[cfg(not(target_os = "android"))]
pub(crate) fn read() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}

/// See the module doc.
#[cfg(target_os = "android")]
pub(crate) fn read() -> Option<String> {
    None
}
