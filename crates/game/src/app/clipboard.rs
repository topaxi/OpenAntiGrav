//! The system clipboard: pasting a disc key into the chooser, and copying a benchmark result out.
//!
//! Desktop builds read and write it through `arboard`. **Android writes it** through JNI
//! (`oag_mobile::clipboard`) **and reads nothing**: winit's
//! `NativeActivity` backend has no clipboard and this project carries no JNI
//! shim, so [`AVAILABLE`] is `false` there and the key prompt offers its keypad
//! and, beside the image, a `.dkey` file instead. **The web build reads nothing
//! either**: the browser's clipboard is asynchronous and asks permission, and a
//! key file beside the image does not exist there; see docs/tools/web.md.

/// Whether this build can write a clipboard: every target but iOS, where the benchmark's COPY TO
/// CLIPBOARD is left off its screen.
pub(crate) const CAN_WRITE: bool = cfg!(not(target_os = "ios"));

/// Whether this build can read a clipboard.
pub(crate) const AVAILABLE: bool = cfg!(not(any(
    target_os = "android",
    target_os = "ios",
    target_arch = "wasm32"
)));

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
#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
pub(crate) fn read() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}

/// See the module doc.
#[cfg(any(target_os = "android", target_os = "ios", target_arch = "wasm32"))]
pub(crate) fn read() -> Option<String> {
    None
}

/// Puts `text` on the clipboard as plain text, and says whether the system took it.
///
/// The desktop's clipboard object is kept for the life of the process: on Linux the text is served
/// by the process that owns the selection, so a clipboard dropped at once would paste nothing.
#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
pub(crate) fn write(text: &str) -> bool {
    static HELD: std::sync::Mutex<Option<arboard::Clipboard>> = std::sync::Mutex::new(None);
    let mut held = HELD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if held.is_none() {
        *held = arboard::Clipboard::new().ok();
    }
    held.as_mut()
        .is_some_and(|clipboard| clipboard.set_text(text.to_owned()).is_ok())
}

/// See the module doc.
#[cfg(target_os = "android")]
pub(crate) fn write(text: &str) -> bool {
    oag_mobile::clipboard::write(text).is_ok()
}

/// The browser's `navigator.clipboard.writeText`, which needs a secure page and the click that
/// asked for it. Called through `js_sys` because `web-sys`'s clipboard is behind an unstable
/// cfg. The promise is not awaited: a refusal after the call (permission) is not seen here.
#[cfg(target_arch = "wasm32")]
pub(crate) fn write(text: &str) -> bool {
    use wasm_bindgen::JsCast;
    let Some(window) = web_sys::window() else {
        return false;
    };
    let navigator = window.navigator();
    let Ok(clipboard) = js_sys::Reflect::get(&navigator, &"clipboard".into()) else {
        return false;
    };
    if clipboard.is_undefined() || clipboard.is_null() {
        return false;
    }
    let Ok(function) = js_sys::Reflect::get(&clipboard, &"writeText".into()) else {
        return false;
    };
    function
        .dyn_into::<js_sys::Function>()
        .and_then(|function| function.call1(&clipboard, &text.into()))
        .is_ok()
}

/// See the module doc.
#[cfg(target_os = "ios")]
pub(crate) fn write(_text: &str) -> bool {
    false
}

#[cfg(test)]
mod tests {
    /// The benchmark's COPY TO CLIPBOARD is offered on every target but iOS, which has no write.
    #[test]
    fn the_clipboard_can_be_written_on_every_target_but_ios() {
        assert_eq!(super::CAN_WRITE, cfg!(not(target_os = "ios")));
    }
}
